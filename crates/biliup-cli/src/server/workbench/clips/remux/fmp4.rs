//! 分片 MP4：文件开头原样复制第一段的 `ftyp` + `moov`，之后从起始关键帧的 `moof` 起逐个复制
//! `moof` / `mdat`：`mfhd` 的序号重新往下数，各轨 `tfdt` 减去起始关键帧的解码时间（按各轨的
//! timescale 换算）再加上这一段在产物上的起点；`tfhd` 写了绝对 `base_data_offset` 的按产物里的
//! 位置改写。`styp` / `sidx` / `prft` 等其余顶层 box 丢掉。

use super::super::plan::Piece;
use super::{Cut, VideoClock};
use std::collections::HashMap;
use std::io;

fn be_u32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn be_u64(data: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(data.get(at..at + 8)?.try_into().ok()?))
}

/// 一个 box 在所给字节里的位置：`(类型, 起点, 头长, 总长)`。
fn boxes(data: &[u8]) -> Vec<([u8; 4], usize, usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 8 <= data.len() {
        let size32 = be_u32(data, at).unwrap_or(0) as usize;
        let kind: [u8; 4] = data[at + 4..at + 8].try_into().unwrap();
        let (size, header) = match size32 {
            0 => (data.len() - at, 8),
            1 => match be_u64(data, at + 8) {
                Some(size) => (size as usize, 16),
                None => break,
            },
            n => (n, 8),
        };
        if size < header || at + size > data.len() {
            break;
        }
        out.push((kind, at, header, size));
        at += size;
    }
    out
}

#[derive(Debug, Clone, Copy, Default)]
struct Track {
    timescale: u32,
    default_duration: u32,
}

/// `moov` 里各轨的 timescale 与 `trex` 默认 sample 时长，以及视频轨的 track_ID。
fn parse_moov(moov: &[u8]) -> (HashMap<u32, Track>, Option<u32>) {
    let mut tracks: HashMap<u32, Track> = HashMap::new();
    let mut video = None;
    for (kind, at, header, size) in boxes(moov) {
        let body = &moov[at + header..at + size];
        match &kind {
            b"trak" => {
                let (mut id, mut timescale, mut is_video) = (0, 0, false);
                for (kind, at, header, size) in boxes(body) {
                    let inner = &body[at + header..at + size];
                    match &kind {
                        b"tkhd" => {
                            let pos = if inner.first() == Some(&1) { 20 } else { 12 };
                            id = be_u32(inner, pos).unwrap_or(0);
                        }
                        b"mdia" => {
                            for (kind, at, header, size) in boxes(inner) {
                                let leaf = &inner[at + header..at + size];
                                match &kind {
                                    b"mdhd" => {
                                        let pos = if leaf.first() == Some(&1) { 20 } else { 12 };
                                        timescale = be_u32(leaf, pos).unwrap_or(0);
                                    }
                                    b"hdlr" => is_video = leaf.get(8..12) == Some(b"vide"),
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                tracks.entry(id).or_default().timescale = timescale;
                if is_video && video.is_none() {
                    video = Some(id);
                }
            }
            b"mvex" => {
                for (kind, at, header, size) in boxes(body) {
                    let inner = &body[at + header..at + size];
                    if &kind == b"trex"
                        && let (Some(id), Some(duration)) = (be_u32(inner, 4), be_u32(inner, 12))
                    {
                        tracks.entry(id).or_default().default_duration = duration;
                    }
                }
            }
            _ => {}
        }
    }
    (tracks, video)
}

/// `traf` 里要改的位置。
struct Traf {
    track_id: u32,
    /// `tfhd` 里 `base_data_offset` 在 moof 内的偏移。
    base_offset_at: Option<usize>,
    /// `tfdt` 内容在 moof 内的偏移与版本。
    tfdt_at: Option<(usize, u8)>,
    samples: u64,
    duration: u64,
}

fn parse_traf(
    moof: &[u8],
    traf_body: usize,
    traf_end: usize,
    tracks: &HashMap<u32, Track>,
) -> Traf {
    let data = &moof[traf_body..traf_end];
    let mut traf = Traf {
        track_id: 0,
        base_offset_at: None,
        tfdt_at: None,
        samples: 0,
        duration: 0,
    };
    let mut default_duration = 0u32;
    for (kind, at, header, size) in boxes(data) {
        let body = &data[at + header..at + size];
        let body_at = traf_body + at + header;
        match &kind {
            b"tfhd" => {
                let flags = be_u32(body, 0).unwrap_or(0) & 0x00FF_FFFF;
                traf.track_id = be_u32(body, 4).unwrap_or(0);
                default_duration = tracks.get(&traf.track_id).map_or(0, |t| t.default_duration);
                let mut pos = 8;
                if flags & 0x01 != 0 {
                    traf.base_offset_at = Some(body_at + pos);
                    pos += 8;
                }
                if flags & 0x02 != 0 {
                    pos += 4;
                }
                if flags & 0x08 != 0 {
                    default_duration = be_u32(body, pos).unwrap_or(default_duration);
                }
            }
            b"tfdt" => {
                traf.tfdt_at = Some((body_at + 4, body.first().copied().unwrap_or(0)));
            }
            b"trun" => {
                let flags = be_u32(body, 0).unwrap_or(0) & 0x00FF_FFFF;
                let count = be_u32(body, 4).unwrap_or(0);
                let mut pos = 8;
                if flags & 0x01 != 0 {
                    pos += 4;
                }
                if flags & 0x04 != 0 {
                    pos += 4;
                }
                let per_sample = [0x100, 0x200, 0x400, 0x800]
                    .iter()
                    .filter(|f| flags & **f != 0)
                    .count()
                    * 4;
                for i in 0..count as usize {
                    let duration = if flags & 0x100 != 0 {
                        be_u32(body, pos + i * per_sample).unwrap_or(default_duration)
                    } else {
                        default_duration
                    };
                    traf.duration += duration as u64;
                }
                traf.samples += count as u64;
            }
            _ => {}
        }
    }
    traf
}

fn read_tfdt(moof: &[u8], at: usize, version: u8) -> Option<i64> {
    if version == 1 {
        be_u64(moof, at).map(|v| v as i64)
    } else {
        be_u32(moof, at).map(i64::from)
    }
}

fn write_tfdt(moof: &mut [u8], at: usize, version: u8, value: i64) {
    let value = value.max(0);
    if version == 1 {
        moof[at..at + 8].copy_from_slice(&(value as u64).to_be_bytes());
    } else {
        moof[at..at + 4].copy_from_slice(&(value.min(u32::MAX as i64) as u32).to_be_bytes());
    }
}

fn rescale(value: i64, from: u32, to: u32) -> i64 {
    if from == 0 {
        return 0;
    }
    (value as i128 * to as i128 / from as i128) as i64
}

#[derive(Debug, Default)]
pub(super) struct Fmp4Cut {
    tracks: HashMap<u32, Track>,
    video_id: Option<u32>,
    sequence: u32,
    written: u64,
    src: u64,
    origin_ms: i64,
    /// 这一段起始关键帧的视频解码时间（视频轨单位）。
    raw_origin: Option<i64>,
    video: VideoClock,
}

impl Fmp4Cut {
    fn timescale(&self, track: u32) -> u32 {
        self.tracks.get(&track).map_or(0, |t| t.timescale)
    }

    fn rewrite_moof(&mut self, moof: &mut [u8], src_at: u64) {
        let video_id = self.video_id.unwrap_or(0);
        let video_scale = self.timescale(video_id);
        let mut trafs = Vec::new();
        let moof_header = if be_u32(moof, 0) == Some(1) { 16 } else { 8 };
        for (kind, at, header, size) in boxes(&moof[moof_header..]) {
            let at = at + moof_header;
            match &kind {
                b"mfhd" if size >= header + 8 => {
                    self.sequence = self.sequence.wrapping_add(1);
                    let pos = at + header + 4;
                    moof[pos..pos + 4].copy_from_slice(&self.sequence.to_be_bytes());
                }
                b"traf" => trafs.push(parse_traf(moof, at + header, at + size, &self.tracks)),
                _ => {}
            }
        }
        if self.raw_origin.is_none() {
            let first = trafs
                .iter()
                .find(|t| t.track_id == video_id)
                .or(trafs.first());
            if let Some(traf) = first
                && let Some((at, version)) = traf.tfdt_at
                && let Some(raw) = read_tfdt(moof, at, version)
            {
                let scale = self.timescale(traf.track_id);
                self.raw_origin = Some(rescale(raw, scale, video_scale.max(1)));
            }
        }
        let origin = self.raw_origin.unwrap_or(0);
        for traf in &trafs {
            if let Some(at) = traf.base_offset_at
                && let Some(base) = be_u64(moof, at)
            {
                let out = (base as i64 - src_at as i64 + self.written as i64).max(0) as u64;
                moof[at..at + 8].copy_from_slice(&out.to_be_bytes());
            }
            let Some((at, version)) = traf.tfdt_at else {
                continue;
            };
            let Some(raw) = read_tfdt(moof, at, version) else {
                continue;
            };
            let scale = self.timescale(traf.track_id);
            let out = raw - rescale(origin, video_scale.max(1), scale)
                + rescale(self.origin_ms, 1000, scale);
            write_tfdt(moof, at, version, out);
            if traf.track_id == video_id && scale > 0 && traf.samples > 0 {
                let end_ms = rescale(out.max(0) + traf.duration as i64, scale, 1000);
                let frame = rescale((traf.duration / traf.samples) as i64, scale, 1000).max(1);
                self.video.last = Some(end_ms - frame);
                self.video.frame = Some(frame);
            }
        }
    }
}

impl Cut for Fmp4Cut {
    fn header(&mut self, region: &[u8], out: &mut Vec<u8>) -> io::Result<()> {
        for (kind, at, header, size) in boxes(region) {
            match &kind {
                b"ftyp" => out.extend_from_slice(&region[at..at + size]),
                b"moov" => {
                    let (tracks, video) = parse_moov(&region[at + header..at + size]);
                    self.tracks = tracks;
                    self.video_id = video;
                    out.extend_from_slice(&region[at..at + size]);
                }
                _ => {}
            }
        }
        if self.video_id.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "分片 MP4 的 moov 里没有视频轨",
            ));
        }
        self.written = out.len() as u64;
        Ok(())
    }

    fn begin(&mut self, origin_ms: i64, piece: &Piece) {
        self.origin_ms = origin_ms;
        self.src = piece.from;
        self.raw_origin = None;
    }

    fn process(&mut self, input: &[u8], out: &mut Vec<u8>) -> io::Result<usize> {
        let mut consumed = 0;
        loop {
            let rest = &input[consumed..];
            if rest.len() < 8 {
                break;
            }
            let size32 = be_u32(rest, 0).unwrap_or(0) as u64;
            let size = match size32 {
                1 => match be_u64(rest, 8) {
                    Some(size) => size,
                    None => break,
                },
                0 => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "分片 MP4 里有延伸到文件末尾的 box",
                    ));
                }
                n => n,
            };
            if size < 8 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("MP4 box at offset {} has size {size}", self.src),
                ));
            }
            if (rest.len() as u64) < size {
                break;
            }
            let size = size as usize;
            let kind: [u8; 4] = rest[4..8].try_into().unwrap();
            match &kind {
                b"moof" => {
                    let mut moof = rest[..size].to_vec();
                    self.rewrite_moof(&mut moof, self.src);
                    out.extend_from_slice(&moof);
                    self.written += size as u64;
                }
                b"mdat" => {
                    out.extend_from_slice(&rest[..size]);
                    self.written += size as u64;
                }
                _ => {}
            }
            consumed += size;
            self.src += size as u64;
        }
        Ok(consumed)
    }

    fn video(&self) -> VideoClock {
        self.video
    }

    fn finish(&mut self, _total_len: u64, _duration_ms: i64) -> Option<(u64, Vec<u8>)> {
        None
    }
}
