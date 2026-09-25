//! 快速剪：按 [`Plan`] 从各分段读出 `[from, to)`，改写时间戳后接成一个文件，不转码。
//!
//! - 文件开头用第一段的头区：FLV 文件头 + 新写的 `onMetaData`（时长、关键帧表）+ 序列头；TS 的
//!   PAT / PMT；fMP4 的 `ftyp` + `moov`；
//! - 每段从关键帧起：产物时间 = 原始时间戳 − 起始关键帧的时间戳（FLV 为 DTS，TS 为视频 PES 的 DTS，
//!   fMP4 为视频轨 `tfdt`）+ 这一段在产物上的起点；第一段从 0 开始，之后各段紧接上一段（断流缺口
//!   不留空），并保证视频时间戳严格递增；
//! - 早于起始关键帧的音频丢掉，时间戳不会是负数。
//!
//! 写入端是同步的纯函数（输入字节 → 输出字节），读文件、写产物、报进度在 [`write`] 里。

mod flv;
mod fmp4;
mod ts;

use super::plan::{Piece, Plan};
use crate::server::workbench::dvr;
use crate::server::workbench::index::Container;
use std::io::{self, SeekFrom};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};

const READ_CHUNK: usize = 1024 * 1024;
const DEFAULT_FRAME_MS: i64 = 33;

/// 视频轨在产物上最后的时间与帧间隔，接段时保证时间戳严格递增。
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct VideoClock {
    last: Option<i64>,
    frame: Option<i64>,
}

impl VideoClock {
    pub(super) fn observe(&mut self, ms: i64) {
        if let Some(last) = self.last {
            let step = ms - last;
            if (1..=100).contains(&step) {
                self.frame = Some(step);
            }
            self.last = Some(last.max(ms));
        } else {
            self.last = Some(ms);
        }
    }

    /// 视频轨结束的时刻：最后一帧的时间 + 一帧。
    pub(super) fn end(&self) -> Option<i64> {
        self.last
            .map(|last| last + self.frame.unwrap_or(DEFAULT_FRAME_MS))
    }
}

/// 一种容器的写入端。
pub(super) trait Cut {
    /// 产物开头（按第一段的头区）。
    fn header(&mut self, region: &[u8], out: &mut Vec<u8>) -> io::Result<()>;
    /// 开始读一段：`origin_ms` 是起始关键帧（`piece.from` 处）在产物上的时刻。
    fn begin(&mut self, origin_ms: i64, piece: &Piece);
    /// 从 `input` 开头起处理完整的单元，输出追加到 `out`，返回消耗的字节数。
    fn process(&mut self, input: &[u8], out: &mut Vec<u8>) -> io::Result<usize>;
    fn video(&self) -> VideoClock;
    /// 写完之后要回填到产物 `offset` 处的内容（FLV 的 `onMetaData`）。
    fn finish(&mut self, total_len: u64, duration_ms: i64) -> Option<(u64, Vec<u8>)>;
}

fn writer(plan: &Plan, metadata: bool) -> Box<dyn Cut + Send> {
    match plan.container {
        Container::Flv => {
            let keyframes = plan.pieces.iter().map(|p| p.keyframes.len()).sum();
            Box::new(flv::FlvCut::new(metadata.then_some(keyframes)))
        }
        Container::Ts => Box::new(ts::TsCut::default()),
        Container::Fmp4 => Box::new(fmp4::Fmp4Cut::default()),
    }
}

pub fn extension(container: Container) -> &'static str {
    match container {
        Container::Flv => "flv",
        Container::Ts => "ts",
        Container::Fmp4 => "mp4",
    }
}

/// ffmpeg `-f` 的输入格式名。
pub fn ffmpeg_format(container: Container) -> &'static str {
    match container {
        Container::Flv => "flv",
        Container::Ts => "mpegts",
        Container::Fmp4 => "mp4",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Remuxed {
    pub bytes: u64,
    /// 产物的媒体时长（毫秒）。
    pub duration_ms: i64,
}

/// 按 `plan` 把各段接起来写进 `out`；`progress` 收到已读的源字节数。
/// 写到管道（精确剪喂给 ffmpeg）时 `metadata = false`，不写要回填的 FLV `onMetaData`。
async fn write<W: AsyncWrite + Unpin>(
    plan: &Plan,
    out: &mut W,
    metadata: bool,
    progress: &mut (dyn FnMut(u64) + Send),
) -> io::Result<(Remuxed, Option<(u64, Vec<u8>)>)> {
    let mut cut = writer(plan, metadata);
    let mut written = 0u64;
    let mut read_total = 0u64;
    let mut origin_ms = 0i64;
    let mut shift_ms = 0i64;
    let mut buf = Vec::with_capacity(READ_CHUNK);
    let mut output = Vec::with_capacity(READ_CHUNK + 64 * 1024);
    for (i, piece) in plan.pieces.iter().enumerate() {
        let mut file = File::open(&piece.path).await?;
        if i == 0 {
            let region = dvr::read_at(&mut file, 0, piece.header_len as usize).await?;
            cut.header(&region, &mut output)?;
        }
        if let Some(end) = cut.video().end()
            && end > origin_ms + shift_ms
        {
            shift_ms = end - origin_ms;
        }
        cut.begin(origin_ms + shift_ms, piece);
        file.seek(SeekFrom::Start(piece.from)).await?;
        let mut left = piece.to - piece.from;
        buf.clear();
        loop {
            if left > 0 {
                let want = (left as usize).min(READ_CHUNK);
                let start = buf.len();
                buf.resize(start + want, 0);
                let n = file.read(&mut buf[start..]).await?;
                buf.truncate(start + n);
                if n == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!("{} 比关键帧索引记的短", piece.path.display()),
                    ));
                }
                left -= n as u64;
                read_total += n as u64;
            }
            let used = cut.process(&buf, &mut output)?;
            buf.drain(..used);
            if !output.is_empty() {
                out.write_all(&output).await?;
                written += output.len() as u64;
                output.clear();
            }
            progress(read_total);
            if left == 0 {
                break;
            }
        }
        origin_ms += piece.duration_ms;
    }
    let duration_ms = cut.video().end().unwrap_or(plan.duration_ms());
    let patch = cut.finish(written, duration_ms);
    out.flush().await?;
    Ok((
        Remuxed {
            bytes: written,
            duration_ms,
        },
        patch,
    ))
}

/// 写到文件 `path`（先写完再回填 FLV 的 `onMetaData`）。
pub async fn to_file(
    plan: &Plan,
    path: &Path,
    progress: &mut (dyn FnMut(u64) + Send),
) -> io::Result<Remuxed> {
    let mut file = File::create(path).await?;
    let (done, patch) = write(plan, &mut file, true, progress).await?;
    if let Some((offset, bytes)) = patch {
        file.seek(SeekFrom::Start(offset)).await?;
        file.write_all(&bytes).await?;
    }
    file.sync_all().await?;
    Ok(done)
}

/// 写到管道（精确剪的 ffmpeg 标准输入）。
pub async fn to_pipe<W: AsyncWrite + Unpin>(
    plan: &Plan,
    out: &mut W,
    progress: &mut (dyn FnMut(u64) + Send),
) -> io::Result<Remuxed> {
    Ok(write(plan, out, false, progress).await?.0)
}

#[cfg(test)]
mod tests;
