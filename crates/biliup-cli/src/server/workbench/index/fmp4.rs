//! 分片 MP4：逐个顶层 box 读 box 头；`moov` 里找视频轨（`hdlr` = `vide`）的 track_ID、
//! `mdhd` timescale 和 `trex` 默认值，`moof` 里读该轨的 `tfdt` 与 `trun` 首个 sample 的
//! flags / 合成时间偏移。首个 sample 是同步 sample 的 `moof` 就是一个关键帧，`mdat` 整个跳过。
//!
//! rust-srec 的 `mp4` crate 只做 init 段的编解码识别，box 遍历是 crate 私有的，读不到
//! `moof` / `tfdt` / `trun`，所以这里自己写了最小的 box 解析。

use super::{KeyframeIndex, Source, read_at};
use bytes::Bytes;
use std::io::{self, SeekFrom};

/// `moov` / `moof` 超过这个大小就当文件损坏。
const MAX_META_BOX: u64 = 64 * 1024 * 1024;
const SAMPLE_IS_NON_SYNC: u32 = 0x0001_0000;

struct BoxHeader {
    kind: [u8; 4],
    size: u64,
    header: u64,
}

fn read_box_header(
    reader: &mut impl Source,
    offset: u64,
    file_len: u64,
) -> io::Result<Option<BoxHeader>> {
    if offset + 8 > file_len {
        return Ok(None);
    }
    let mut head = [0u8; 8];
    reader.read_exact(&mut head)?;
    let size32 = u32::from_be_bytes(head[..4].try_into().unwrap()) as u64;
    let kind: [u8; 4] = head[4..].try_into().unwrap();
    let (size, header) = match size32 {
        // 延伸到文件末尾：只有写完的最后一个 box 才会这样，正在写时当作不完整
        0 => return Ok(None),
        1 => {
            if offset + 16 > file_len {
                return Ok(None);
            }
            let mut large = [0u8; 8];
            reader.read_exact(&mut large)?;
            (u64::from_be_bytes(large), 16)
        }
        n => (n, 8),
    };
    if size < header {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("MP4 box at offset {offset} has size {size}"),
        ));
    }
    if offset + size > file_len {
        return Ok(None);
    }
    Ok(Some(BoxHeader { kind, size, header }))
}

/// 在一段已读入内存的 box 内容里按顺序遍历子 box：`(类型, 内容)`。
fn children(mut data: &[u8]) -> impl Iterator<Item = ([u8; 4], &[u8])> {
    std::iter::from_fn(move || {
        if data.len() < 8 {
            return None;
        }
        let size32 = u32::from_be_bytes(data[..4].try_into().unwrap()) as usize;
        let kind: [u8; 4] = data[4..8].try_into().unwrap();
        let (size, header) = match size32 {
            0 => (data.len(), 8),
            1 if data.len() >= 16 => (
                u64::from_be_bytes(data[8..16].try_into().unwrap()) as usize,
                16,
            ),
            n => (n, 8),
        };
        if size < header || size > data.len() {
            return None;
        }
        let body = &data[header..size];
        data = &data[size..];
        Some((kind, body))
    })
}

fn be_u32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn be_u64(data: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(data.get(at..at + 8)?.try_into().ok()?))
}

#[derive(Default)]
struct VideoTrack {
    id: u32,
    timescale: u32,
    default_duration: u32,
    default_flags: u32,
}

/// 从 `moov` 内容里找视频轨。没有 `mvex`（不是分片 MP4）返回 `Unsupported`。
fn parse_moov(moov: &[u8]) -> io::Result<Option<VideoTrack>> {
    let mut video: Option<VideoTrack> = None;
    let mut fragmented = false;
    let mut trex: Vec<(u32, u32, u32)> = Vec::new();
    for (kind, body) in children(moov) {
        match &kind {
            b"trak" => {
                let mut id = 0;
                let mut timescale = 0;
                let mut is_video = false;
                for (kind, body) in children(body) {
                    match &kind {
                        b"tkhd" => {
                            id = if body.first() == Some(&1) {
                                be_u32(body, 20)
                            } else {
                                be_u32(body, 12)
                            }
                            .unwrap_or(0);
                        }
                        b"mdia" => {
                            for (kind, body) in children(body) {
                                match &kind {
                                    b"mdhd" => {
                                        timescale = if body.first() == Some(&1) {
                                            be_u32(body, 20)
                                        } else {
                                            be_u32(body, 12)
                                        }
                                        .unwrap_or(0);
                                    }
                                    b"hdlr" => is_video = body.get(8..12) == Some(b"vide"),
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if is_video && video.is_none() {
                    video = Some(VideoTrack {
                        id,
                        timescale,
                        ..Default::default()
                    });
                }
            }
            b"mvex" => {
                fragmented = true;
                for (kind, body) in children(body) {
                    if &kind == b"trex"
                        && let (Some(id), Some(duration), Some(flags)) =
                            (be_u32(body, 4), be_u32(body, 12), be_u32(body, 20))
                    {
                        trex.push((id, duration, flags));
                    }
                }
            }
            _ => {}
        }
    }
    if !fragmented {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "不是分片 MP4（moov 里没有 mvex），不建关键帧索引",
        ));
    }
    if let Some(video) = video.as_mut()
        && let Some(&(_, duration, flags)) = trex.iter().find(|(id, _, _)| *id == video.id)
    {
        video.default_duration = duration;
        video.default_flags = flags;
    }
    Ok(video)
}

struct Fragment {
    decode_time: u64,
    first_cto: i64,
    first_is_sync: bool,
    duration: u64,
}

/// 从 `moof` 内容里取视频轨那个 `traf` 的时间与首帧信息。
fn parse_moof(moof: &[u8], track: &VideoTrack) -> Option<Fragment> {
    for (kind, traf) in children(moof) {
        if &kind != b"traf" {
            continue;
        }
        let mut track_id = 0;
        let mut default_duration = track.default_duration;
        let mut default_flags = track.default_flags;
        let mut decode_time = None;
        let mut first_flags = None;
        let mut first_cto = 0i64;
        let mut duration = 0u64;
        for (kind, body) in children(traf) {
            match &kind {
                b"tfhd" => {
                    let flags = be_u32(body, 0)? & 0x00FF_FFFF;
                    track_id = be_u32(body, 4)?;
                    let mut at = 8;
                    if flags & 0x01 != 0 {
                        at += 8;
                    }
                    if flags & 0x02 != 0 {
                        at += 4;
                    }
                    if flags & 0x08 != 0 {
                        default_duration = be_u32(body, at)?;
                        at += 4;
                    }
                    if flags & 0x10 != 0 {
                        at += 4;
                    }
                    if flags & 0x20 != 0 {
                        default_flags = be_u32(body, at)?;
                    }
                }
                b"tfdt" => {
                    decode_time = if body.first() == Some(&1) {
                        be_u64(body, 4)
                    } else {
                        be_u32(body, 4).map(u64::from)
                    };
                }
                b"trun" => {
                    let version = *body.first()?;
                    let flags = be_u32(body, 0)? & 0x00FF_FFFF;
                    let count = be_u32(body, 4)?;
                    let mut at = 8;
                    if flags & 0x01 != 0 {
                        at += 4;
                    }
                    let run_first_flags = if flags & 0x04 != 0 {
                        let v = be_u32(body, at)?;
                        at += 4;
                        Some(v)
                    } else {
                        None
                    };
                    for i in 0..count {
                        let mut sample_duration = default_duration;
                        let mut sample_flags = None;
                        let mut cto = 0i64;
                        if flags & 0x100 != 0 {
                            sample_duration = be_u32(body, at)?;
                            at += 4;
                        }
                        if flags & 0x200 != 0 {
                            at += 4;
                        }
                        if flags & 0x400 != 0 {
                            sample_flags = Some(be_u32(body, at)?);
                            at += 4;
                        }
                        if flags & 0x800 != 0 {
                            let raw = be_u32(body, at)?;
                            cto = if version == 0 {
                                raw as i64
                            } else {
                                raw as i32 as i64
                            };
                            at += 4;
                        }
                        if i == 0 && first_flags.is_none() {
                            first_flags =
                                Some(run_first_flags.or(sample_flags).unwrap_or(default_flags));
                            first_cto = cto;
                        }
                        duration += sample_duration as u64;
                    }
                }
                _ => {}
            }
        }
        if track_id != track.id {
            continue;
        }
        return Some(Fragment {
            decode_time: decode_time?,
            first_cto,
            first_is_sync: first_flags.is_some_and(|f| f & SAMPLE_IS_NON_SYNC == 0),
            duration,
        });
    }
    None
}

fn read_body(reader: &mut impl Source, header: &BoxHeader) -> io::Result<Bytes> {
    let len = header.size - header.header;
    if len > MAX_META_BOX {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("MP4 box {:?} too large: {len}", header.kind),
        ));
    }
    reader.read_bytes(len as usize)
}

/// 从 `index.scanned_upto` 扫到文件末尾最后一个完整的顶层 box。
pub(super) fn scan(
    reader: &mut impl Source,
    file_len: u64,
    index: &mut KeyframeIndex,
) -> io::Result<()> {
    let mut offset = index.scanned_upto;
    let mut track = VideoTrack {
        id: index.track.id,
        timescale: index.timescale,
        default_duration: index.track.aux,
        default_flags: index.track.default_flags,
    };
    reader.seek(SeekFrom::Start(offset))?;
    while let Some(header) = read_box_header(reader, offset, file_len)? {
        let mut consumed = false;
        match &header.kind {
            b"moov" => {
                consumed = true;
                let body = read_body(reader, &header)?;
                if let Some(video) = parse_moov(&body)? {
                    track = video;
                    index.track.id = track.id;
                    index.track.aux = track.default_duration;
                    index.track.default_flags = track.default_flags;
                    index.timescale = track.timescale;
                }
            }
            b"moof" => {
                index.finalize_header(offset);
                consumed = true;
                let body = read_body(reader, &header)?;
                if track.id != 0
                    && track.timescale != 0
                    && let Some(fragment) = parse_moof(&body, &track)
                {
                    let start = fragment.decode_time as i64 + fragment.first_cto;
                    if fragment.first_is_sync {
                        index.push_keyframe(start, offset);
                    }
                    index.observe_media(fragment.decode_time as i64 + fragment.duration as i64);
                }
            }
            _ => {}
        }
        if !consumed {
            reader.skip((header.size - header.header) as i64)?;
        }
        offset += header.size;
        index.scanned_upto = offset;
    }
    Ok(())
}

/// `offset` 处是不是一个完整的 `moof`。
pub(super) fn is_moof_at(reader: &mut impl Source, offset: u64, file_len: u64) -> bool {
    let mut head = [0u8; 8];
    if offset + 8 > file_len || read_at(reader, offset, &mut head).is_err() {
        return false;
    }
    &head[4..] == b"moof"
}
