//! 抽音频、静音检测与切块。
//!
//! 每个分段单独交给 ffmpeg 解一遍：`asetpts=N/SR/TB` 把时间戳改成按采样数从 0 数，
//! `silencedetect` 报的静音就是段内时间，不受分段里原始时间戳（stream-gears 的绝对时间戳、
//! fMP4 保留的源站 `tfdt`）影响；段内时间 + `segments.start_ms` = 场次时间。同一趟输出
//! 16 kHz 单声道 FLAC。逐段处理，前后分段编码参数不同也互不影响。
//!
//! 切块：去掉 ≥ 2 s 的静音（两侧各留 0.3 s），剩下的语音区间按顺序合并成不超过 10 分钟
//! （转写模型不给分句时间戳时 60 s）的一块，只在静音处切；一段语音本身超过上限就硬切，
//! 前后重叠 1 s。每块从 FLAC 里按 10 ms 一帧选出自己的语音区间拼起来重新编码（16 kHz
//! 单声道 FLAC 解码很快，不再读录像），块内时间按区间表换算回段内时间。

use crate::tools::low_priority_ffmpeg_command;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::process::Stdio;

pub const SAMPLE_RATE: u32 = 16_000;
/// 切块时选取音频的粒度：每帧 160 个采样
const FRAME_MS: i64 = 10;
const FRAME_SAMPLES: i64 = SAMPLE_RATE as i64 * FRAME_MS / 1000;
/// 能量低于 -35 dB 且持续 2 s 以上算静音。这是能量阈值，不是真正的人声检测：背景音乐、
/// 游戏音效不会被当成静音。
const SILENCE_FILTER: &str = "silencedetect=noise=-35dB:d=2";
/// 语音区间两侧各留一点静音，免得切掉字头字尾
pub const PAD_MS: i64 = 300;
/// 有分句时间戳时每块最多这么长（16 kHz 单声道 FLAC 约 5–10 MB）
pub const MAX_CHUNK_MS: i64 = 10 * 60_000;
/// 转写模型不给分句时间戳时，整块文字只能记在块的起止上，块长缩到这么长
pub const MAX_CHUNK_MS_WITHOUT_SEGMENTS: i64 = 60_000;
/// 一段语音超过块长上限、只能硬切时前后重叠多少
pub const OVERLAP_MS: i64 = 1000;
/// 单块上传的大小上限（OpenAI 文档写的是 25 MB，留出余量）
pub const MAX_CHUNK_BYTES: u64 = 20 * 1024 * 1024;
/// 一块最多拼多少个语音区间，控制 ffmpeg 滤镜表达式的长度（Windows 命令行上限 32K 字符）
const MAX_SPANS_PER_CHUNK: usize = 200;

/// 段内时间区间 `[from_ms, to_ms)`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub from_ms: i64,
    pub to_ms: i64,
}

impl Span {
    pub fn new(from_ms: i64, to_ms: i64) -> Self {
        Span { from_ms, to_ms }
    }

    pub fn len_ms(&self) -> i64 {
        (self.to_ms - self.from_ms).max(0)
    }
}

/// 一个分段抽出来的音频与静音表。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentAudio {
    pub segment_id: i64,
    /// 分段在场次时间轴上的起点
    pub start_ms: i64,
    /// 抽出来的音频时长
    pub duration_ms: i64,
    pub silences: Vec<Span>,
}

impl SegmentAudio {
    pub fn speech(&self) -> Vec<Span> {
        speech_spans(&self.silences, self.duration_ms, PAD_MS)
    }

    pub fn speech_ms(&self) -> i64 {
        self.speech().iter().map(Span::len_ms).sum()
    }
}

/// 要送转写的一块：同一分段里的若干语音区间，按顺序拼成一个音频文件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    /// `<分段 id>-<序号>`，断点续传按它认已经转写过的块
    pub key: String,
    pub segment_id: i64,
    pub segment_start_ms: i64,
    pub spans: Vec<Span>,
}

impl Chunk {
    pub fn speech_ms(&self) -> i64 {
        self.spans.iter().map(Span::len_ms).sum()
    }
}

#[derive(Debug)]
pub enum AudioError {
    /// 分段里没有音频流
    NoAudio,
    Ffmpeg(String),
    Io(std::io::Error),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::NoAudio => f.write_str("没有音频"),
            AudioError::Ffmpeg(detail) => write!(f, "ffmpeg 出错：{detail}"),
            AudioError::Io(error) => write!(f, "读写文件出错：{error}"),
        }
    }
}

impl From<std::io::Error> for AudioError {
    fn from(error: std::io::Error) -> Self {
        AudioError::Io(error)
    }
}

/// 抽一个分段的音频到 `out`（16 kHz 单声道 FLAC），同时拿到静音表。
///
/// `fallback_ms`：FLAC 头里读不出时长时（写到不能回写的地方）用的时长，一般是分段在时间轴上的长度。
pub async fn extract(
    segment: &Path,
    out: &Path,
    fallback_ms: i64,
) -> Result<(i64, Vec<Span>), AudioError> {
    let part = out.with_extension("flac.part");
    let filter = format!("asetpts=N/SR/TB,aresample={SAMPLE_RATE},{SILENCE_FILTER}");
    let output = low_priority_ffmpeg_command()
        .args(["-nostdin", "-hide_banner", "-nostats", "-i"])
        .arg(segment)
        .args(["-vn", "-sn", "-dn", "-map", "0:a:0", "-af", &filter])
        .args(["-ac", "1", "-c:a", "flac", "-f", "flac", "-y"])
        .arg(&part)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .output()
        .await?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let _ = tokio::fs::remove_file(&part).await;
        if stderr.contains("matches no streams") {
            return Err(AudioError::NoAudio);
        }
        return Err(AudioError::Ffmpeg(stderr_tail(&stderr)));
    }
    let duration_ms = flac_duration_ms(&read_head(&part).await?)
        .unwrap_or_else(|| fallback_ms.max(last_silence_end(&stderr)));
    if duration_ms <= 0 {
        let _ = tokio::fs::remove_file(&part).await;
        return Err(AudioError::NoAudio);
    }
    tokio::fs::rename(&part, out).await?;
    Ok((duration_ms, parse_silences(&stderr, duration_ms)))
}

/// 从抽好的 FLAC 里选出 `spans` 拼成一块，写到 `out`，返回文件大小。
pub async fn encode_chunk(source: &Path, spans: &[Span], out: &Path) -> Result<u64, AudioError> {
    let output = low_priority_ffmpeg_command()
        .args([
            "-nostdin",
            "-hide_banner",
            "-nostats",
            "-loglevel",
            "error",
            "-i",
        ])
        .arg(source)
        .args(["-af", &select_filter(spans)])
        .args(["-ac", "1", "-ar", &SAMPLE_RATE.to_string()])
        .args(["-c:a", "flac", "-f", "flac", "-y"])
        .arg(out)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .output()
        .await?;
    if !output.status.success() {
        let _ = tokio::fs::remove_file(out).await;
        return Err(AudioError::Ffmpeg(stderr_tail(&String::from_utf8_lossy(
            &output.stderr,
        ))));
    }
    Ok(tokio::fs::metadata(out).await?.len())
}

/// 按 10 ms 一帧选出区间：先把音频切成每帧 160 个采样，再按帧号选，拼出来的时长与区间表
/// 逐帧对得上，块内时间换算回段内时间不会随区间数累积误差。
fn select_filter(spans: &[Span]) -> String {
    let terms: Vec<String> = spans
        .iter()
        .map(|span| {
            let first = span.from_ms / FRAME_MS;
            let last = (span.to_ms / FRAME_MS - 1).max(first);
            format!("between(n,{first},{last})")
        })
        .collect();
    format!(
        "asetnsamples=n={FRAME_SAMPLES}:p=0,aselect='{}',asetpts=N/SR/TB",
        terms.join("+")
    )
}

async fn read_head(path: &Path) -> std::io::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut head = vec![0u8; 42];
    let mut read = 0;
    while read < head.len() {
        let n = file.read(&mut head[read..]).await?;
        if n == 0 {
            break;
        }
        read += n;
    }
    head.truncate(read);
    Ok(head)
}

/// FLAC 头（STREAMINFO）里的总采样数 / 采样率。
pub fn flac_duration_ms(head: &[u8]) -> Option<i64> {
    if head.len() < 42 || &head[..4] != b"fLaC" || head[4] & 0x7f != 0 {
        return None;
    }
    let info = &head[8..42];
    let rate =
        (u64::from(info[10]) << 12) | (u64::from(info[11]) << 4) | (u64::from(info[12]) >> 4);
    let total = (u64::from(info[13] & 0x0f) << 32)
        | (u64::from(info[14]) << 24)
        | (u64::from(info[15]) << 16)
        | (u64::from(info[16]) << 8)
        | u64::from(info[17]);
    (rate > 0 && total > 0).then(|| (total * 1000 / rate) as i64)
}

/// 解析 `silencedetect` 的输出。最后一段静音一直到结尾、ffmpeg 没报 `silence_end` 时补到 `duration_ms`。
pub fn parse_silences(stderr: &str, duration_ms: i64) -> Vec<Span> {
    let mut silences = Vec::new();
    let mut start = None;
    for line in stderr.lines() {
        if let Some(at) = number_after(line, "silence_start:") {
            start = Some(at);
        } else if let Some(end) = number_after(line, "silence_end:") {
            let from = start.take().unwrap_or(0);
            silences.push(Span::new(
                from.clamp(0, duration_ms),
                end.clamp(0, duration_ms),
            ));
        }
    }
    if let Some(from) = start {
        silences.push(Span::new(from.clamp(0, duration_ms), duration_ms));
    }
    silences.retain(|span| span.len_ms() > 0);
    silences
}

fn last_silence_end(stderr: &str) -> i64 {
    stderr
        .lines()
        .filter_map(|line| number_after(line, "silence_end:"))
        .max()
        .unwrap_or(0)
}

/// `…silence_start: 12.0233` → 12023（毫秒）。
fn number_after(line: &str, label: &str) -> Option<i64> {
    let rest = &line[line.find(label)? + label.len()..];
    let value: f64 = rest.split_whitespace().next()?.parse().ok()?;
    Some((value * 1000.0).round() as i64)
}

fn stderr_tail(stderr: &str) -> String {
    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let tail = lines[lines.len().saturating_sub(3)..].join(" / ");
    let chars: Vec<char> = tail.chars().collect();
    if chars.len() > 300 {
        chars[chars.len() - 300..].iter().collect()
    } else {
        tail
    }
}

/// 静音以外的部分，两侧各留 `pad_ms`，对齐到 10 ms 一帧。
pub fn speech_spans(silences: &[Span], duration_ms: i64, pad_ms: i64) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let mut from = 0;
    let mut push = |raw_from: i64, raw_to: i64| {
        if raw_to <= raw_from {
            return;
        }
        let span = Span::new(
            floor_frame((raw_from - pad_ms).max(0)),
            ceil_frame((raw_to + pad_ms).min(duration_ms)),
        );
        match spans.last_mut() {
            Some(last) if span.from_ms <= last.to_ms => last.to_ms = last.to_ms.max(span.to_ms),
            _ => spans.push(span),
        }
    };
    for silence in silences {
        push(from, silence.from_ms);
        from = from.max(silence.to_ms);
    }
    push(from, duration_ms);
    spans
}

fn floor_frame(ms: i64) -> i64 {
    ms / FRAME_MS * FRAME_MS
}

fn ceil_frame(ms: i64) -> i64 {
    (ms + FRAME_MS - 1) / FRAME_MS * FRAME_MS
}

/// 把一个分段的语音区间按顺序装进不超过 `max_ms` 的块里。
pub fn plan_chunks(audio: &SegmentAudio, max_ms: i64) -> Vec<Chunk> {
    let max_ms = max_ms.max(OVERLAP_MS * 2);
    let mut chunks = Vec::new();
    let mut current: Vec<Span> = Vec::new();
    let mut current_ms = 0;
    let mut close = |current: &mut Vec<Span>, current_ms: &mut i64| {
        if current.is_empty() {
            return;
        }
        chunks.push(Chunk {
            key: format!("{}-{}", audio.segment_id, chunks.len()),
            segment_id: audio.segment_id,
            segment_start_ms: audio.start_ms,
            spans: std::mem::take(current),
        });
        *current_ms = 0;
    };
    for span in audio.speech() {
        for piece in split_long(span, max_ms) {
            if !current.is_empty()
                && (current_ms + piece.len_ms() > max_ms || current.len() >= MAX_SPANS_PER_CHUNK)
            {
                close(&mut current, &mut current_ms);
            }
            current_ms += piece.len_ms();
            current.push(piece);
        }
    }
    close(&mut current, &mut current_ms);
    chunks
}

/// 超过 `max_ms` 的一段语音硬切成几块，前后重叠 [`OVERLAP_MS`]。
fn split_long(span: Span, max_ms: i64) -> Vec<Span> {
    if span.len_ms() <= max_ms {
        return vec![span];
    }
    let mut pieces = Vec::new();
    let mut from = span.from_ms;
    loop {
        let to = (from + max_ms).min(span.to_ms);
        pieces.push(Span::new(from, to));
        if to >= span.to_ms {
            break;
        }
        from = floor_frame(to - OVERLAP_MS);
    }
    pieces
}

/// 把一块分成两半（上传太大时）：多个区间按个数对半分，只有一个区间就从中间切开。
pub fn halve(spans: &[Span]) -> Option<(Vec<Span>, Vec<Span>)> {
    match spans {
        [] => None,
        [only] => {
            if only.len_ms() < FRAME_MS * 2 {
                return None;
            }
            let middle = floor_frame(only.from_ms + only.len_ms() / 2);
            Some((
                vec![Span::new(only.from_ms, middle)],
                vec![Span::new(middle, only.to_ms)],
            ))
        }
        _ => {
            let (left, right) = spans.split_at(spans.len() / 2);
            Some((left.to_vec(), right.to_vec()))
        }
    }
}

/// 块内时间（拼起来之后的音频里的毫秒数）换算回段内时间。
pub fn to_segment_ms(spans: &[Span], chunk_ms: i64) -> i64 {
    let mut offset = 0;
    for span in spans {
        if chunk_ms < offset + span.len_ms() {
            return span.from_ms + (chunk_ms - offset).max(0);
        }
        offset += span.len_ms();
    }
    spans.last().map_or(chunk_ms, |span| span.to_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silences_are_parsed_and_an_open_one_runs_to_the_end() {
        let stderr = "\
[silencedetect @ 0x1] silence_start: 12.0233
[silencedetect @ 0x1] silence_end: 20.0233 | silence_duration: 8
size= 1kB time=00:00:30.00
[silencedetect @ 0x1] silence_start: 52.5";
        assert_eq!(
            parse_silences(stderr, 60_000),
            vec![Span::new(12_023, 20_023), Span::new(52_500, 60_000)]
        );
        assert_eq!(last_silence_end(stderr), 20_023);
    }

    #[test]
    fn speech_is_what_is_left_padded_and_frame_aligned() {
        let silences = [
            Span::new(0, 5_000),
            Span::new(12_023, 20_023),
            Span::new(52_500, 60_000),
        ];
        assert_eq!(
            speech_spans(&silences, 60_000, PAD_MS),
            vec![Span::new(4_700, 12_330), Span::new(19_720, 52_800)]
        );
        // 没有静音：整段都是语音
        assert_eq!(
            speech_spans(&[], 61_234, PAD_MS),
            vec![Span::new(0, 61_240)]
        );
    }

    #[test]
    fn chunks_only_cut_at_silences_and_long_speech_overlaps() {
        let audio = SegmentAudio {
            segment_id: 7,
            start_ms: 100_000,
            duration_ms: 1_500_000,
            silences: vec![
                Span::new(240_000, 250_000),
                Span::new(480_000, 490_000),
                Span::new(730_000, 740_000),
            ],
        };
        let chunks = plan_chunks(&audio, MAX_CHUNK_MS);
        // 语音：[0, 240.3)、[249.7, 480.3)、[489.7, 730.3)、[739.7, 1500)
        assert_eq!(chunks[0].spans.len(), 2, "前两段合起来不到 10 分钟");
        assert_eq!(chunks[0].key, "7-0");
        assert!(chunks.iter().all(|c| c.speech_ms() <= MAX_CHUNK_MS));
        let last_speech = chunks
            .iter()
            .flat_map(|c| c.spans.iter())
            .filter(|s| s.from_ms >= 739_000)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(last_speech.len(), 2, "12 分钟的一段语音硬切成两块");
        assert_eq!(last_speech[0].to_ms - last_speech[1].from_ms, OVERLAP_MS);
        assert_eq!(last_speech[1].to_ms, 1_500_000);
        let total: i64 = chunks.iter().map(Chunk::speech_ms).sum();
        assert_eq!(total, audio.speech_ms() + OVERLAP_MS);
    }

    #[test]
    fn chunk_time_maps_back_across_the_skipped_silences() {
        let spans = [Span::new(1_000, 3_000), Span::new(10_000, 11_000)];
        assert_eq!(to_segment_ms(&spans, 0), 1_000);
        assert_eq!(to_segment_ms(&spans, 1_999), 2_999);
        assert_eq!(to_segment_ms(&spans, 2_000), 10_000);
        assert_eq!(to_segment_ms(&spans, 2_500), 10_500);
        assert_eq!(to_segment_ms(&spans, 9_999), 11_000, "越界取最后一段的末尾");
        assert_eq!(
            select_filter(&spans),
            "asetnsamples=n=160:p=0,aselect='between(n,100,299)+between(n,1000,1099)',asetpts=N/SR/TB"
        );
    }

    #[test]
    fn halving_splits_by_spans_or_in_the_middle() {
        let spans = [
            Span::new(0, 1_000),
            Span::new(2_000, 3_000),
            Span::new(4_000, 5_000),
        ];
        let (left, right) = halve(&spans).unwrap();
        assert_eq!((left.len(), right.len()), (1, 2));
        let (left, right) = halve(&[Span::new(0, 10_010)]).unwrap();
        assert_eq!(left, vec![Span::new(0, 5_000)]);
        assert_eq!(right, vec![Span::new(5_000, 10_010)]);
        assert_eq!(halve(&[Span::new(0, 10)]), None);
    }

    #[test]
    fn flac_header_gives_the_duration() {
        let mut head = vec![0u8; 42];
        head[..4].copy_from_slice(b"fLaC");
        // STREAMINFO：16000 Hz、单声道、16 bit、48000 个采样
        let info = &mut head[8..42];
        info[10] = (16_000u32 >> 12) as u8;
        info[11] = (16_000u32 >> 4) as u8;
        info[12] = ((16_000u32 & 0x0f) << 4) as u8 | 0b0000_0001;
        info[13] = 0xf0;
        info[16] = (48_000u32 >> 8) as u8;
        info[17] = 48_000u32 as u8;
        assert_eq!(flac_duration_ms(&head), Some(3_000));
        assert_eq!(flac_duration_ms(b"RIFF...."), None);
    }
}
