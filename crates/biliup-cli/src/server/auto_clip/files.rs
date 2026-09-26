//! 一场的分析数据：`<auto_clip 目录>/<场次 id>/` 下的文件，不进 SQLite（连接池只有 2 个连接，
//! 转写文本这类大块数据放盘上）。
//!
//! - `audio/<分段 id>.flac` + `.json`：抽出来的音频与静音表，转写完就删；
//! - `silence.json`：各分段的静音表，预估用量时用；
//! - `chunks.json`：切块结果，第一次转写前定下，续跑时照它走，块的边界不会因为改了设置而变；
//! - `transcript.jsonl`：每行一句 `{chunk, from_ms, to_ms, text}`，时间是场次时间；
//! - `asr_done.txt`：转写完的块，每行一个 key。一块的句子全部写进 `transcript.jsonl` 之后才记在这里，
//!   续跑时没记上的块的句子先删掉再重转，不会重复。

use super::audio::{Chunk, SegmentAudio};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

/// 转写结果里的一句，时间是场次时间。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub chunk: String,
    pub from_ms: i64,
    pub to_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SessionFiles {
    dir: PathBuf,
}

impl SessionFiles {
    pub fn new(root: &Path, session_id: i64) -> Self {
        SessionFiles {
            dir: root.join(session_id.to_string()),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn audio_dir(&self) -> PathBuf {
        self.dir.join("audio")
    }

    pub fn segment_audio(&self, segment_id: i64) -> PathBuf {
        self.audio_dir().join(format!("{segment_id}.flac"))
    }

    fn segment_meta(&self, segment_id: i64) -> PathBuf {
        self.audio_dir().join(format!("{segment_id}.json"))
    }

    pub fn chunk_audio(&self, name: &str) -> PathBuf {
        self.audio_dir().join(format!("chunk-{name}.flac"))
    }

    fn silence(&self) -> PathBuf {
        self.dir.join("silence.json")
    }

    fn plan(&self) -> PathBuf {
        self.dir.join("chunks.json")
    }

    pub fn transcript(&self) -> PathBuf {
        self.dir.join("transcript.jsonl")
    }

    fn done(&self) -> PathBuf {
        self.dir.join("asr_done.txt")
    }

    /// 抽好的分段音频：音频和静音表都在才算。
    pub async fn load_segment(&self, segment_id: i64) -> Option<SegmentAudio> {
        if !tokio::fs::try_exists(self.segment_audio(segment_id))
            .await
            .unwrap_or(false)
        {
            return None;
        }
        read_json(&self.segment_meta(segment_id)).await
    }

    pub async fn save_segment(&self, audio: &SegmentAudio) -> io::Result<()> {
        write_json(&self.segment_meta(audio.segment_id), audio).await
    }

    pub async fn remove_segment_audio(&self, segment_id: i64) {
        let _ = tokio::fs::remove_file(self.segment_audio(segment_id)).await;
        let _ = tokio::fs::remove_file(self.segment_meta(segment_id)).await;
    }

    pub async fn remove_audio(&self) {
        let _ = tokio::fs::remove_dir_all(self.audio_dir()).await;
    }

    pub async fn load_silence(&self) -> Option<Vec<SegmentAudio>> {
        read_json(&self.silence()).await
    }

    pub async fn save_silence(&self, audio: &[SegmentAudio]) -> io::Result<()> {
        write_json(&self.silence(), &audio).await
    }

    pub async fn load_plan(&self) -> Option<Vec<Chunk>> {
        read_json(&self.plan()).await
    }

    pub async fn save_plan(&self, chunks: &[Chunk]) -> io::Result<()> {
        write_json(&self.plan(), &chunks).await
    }

    /// 新任务换上重新算的切块：旧切块里边界完全相同、已转写完的块保留，其余的块改成没转写
    /// （它们的句子由 [`Self::prune_transcript`] 删掉）。
    pub async fn replace_plan(&self, plan: &[Chunk]) -> io::Result<()> {
        let old = self.load_plan().await.unwrap_or_default();
        let done = self.done_chunks().await;
        let body: String = plan
            .iter()
            .filter(|chunk| done.contains(&chunk.key) && old.contains(chunk))
            .map(|chunk| format!("{}\n", chunk.key))
            .collect();
        tokio::fs::create_dir_all(&self.dir).await?;
        write_atomic(&self.done(), body.as_bytes()).await?;
        self.save_plan(plan).await
    }

    pub async fn done_chunks(&self) -> HashSet<String> {
        tokio::fs::read_to_string(self.done())
            .await
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// 删掉没记完成的块留下的句子（上次在写完一块和记完成之间被打断）。
    pub async fn prune_transcript(&self, done: &HashSet<String>) -> io::Result<()> {
        let path = self.transcript();
        let Ok(text) = tokio::fs::read_to_string(&path).await else {
            return Ok(());
        };
        let kept: Vec<&str> = text
            .lines()
            .filter(|line| {
                serde_json::from_str::<Line>(line).is_ok_and(|line| done.contains(&line.chunk))
            })
            .collect();
        if kept.len() == text.lines().count() {
            return Ok(());
        }
        let mut body = kept.join("\n");
        if !body.is_empty() {
            body.push('\n');
        }
        write_atomic(&path, body.as_bytes()).await
    }

    /// 一块转写完：先追加它的句子，再记完成。
    pub async fn commit_chunk(&self, key: &str, lines: &[Line]) -> io::Result<()> {
        tokio::fs::create_dir_all(&self.dir).await?;
        let mut body = String::new();
        for line in lines {
            body.push_str(&serde_json::to_string(line).map_err(io::Error::other)?);
            body.push('\n');
        }
        append(&self.transcript(), body.as_bytes()).await?;
        append(&self.done(), format!("{key}\n").as_bytes()).await
    }

    /// 不沿用已有转写时：清掉转写结果和切块，下次从头来。
    pub async fn clear_transcript(&self) {
        for path in [self.transcript(), self.done(), self.plan()] {
            let _ = tokio::fs::remove_file(path).await;
        }
    }

    pub async fn lines(&self) -> Vec<Line> {
        tokio::fs::read_to_string(self.transcript())
            .await
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }
}

async fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let text = tokio::fs::read_to_string(path).await.ok()?;
    serde_json::from_str(&text).ok()
}

async fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> io::Result<()> {
    let body = serde_json::to_vec(value).map_err(io::Error::other)?;
    write_atomic(path, &body).await
}

async fn write_atomic(path: &Path, body: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let part = path.with_extension("part");
    tokio::fs::write(&part, body).await?;
    tokio::fs::rename(&part, path).await
}

async fn append(path: &Path, body: &[u8]) -> io::Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(body).await?;
    file.flush().await?;
    file.sync_data().await
}
