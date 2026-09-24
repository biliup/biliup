//! MPEG-TS：逐个 188 字节包读包头，找 PAT → PMT → 视频 PID；视频 PES 起始包带随机访问标志，
//! 或 PES 里第一个图像 NAL 是 IDR（H.264 类型 5 / H.265 类型 16–21），就是一个关键帧。

use super::{KeyframeIndex, Source, read_at};
use ::ts::{PatRef, PesHeaderRef, PmtRef, StreamType, TsPacketRef};
use bytes::Bytes;
use std::io::{self, SeekFrom};

const PACKET: u64 = 188;
const PID_PAT: u16 = 0x0000;
const PID_NULL: u16 = 0x1FFF;
const CODEC_H264: u8 = 0x1B;
const CODEC_H265: u8 = 0x24;
/// 判定一个 PES 是不是 IDR 最多看这么多字节的 ES 数据。
const MAX_SNIFF: usize = 64 * 1024;
const PTS_WRAP: i64 = 1 << 33;

struct PendingPes {
    offset: u64,
    pts: Option<i64>,
    random_access: bool,
    nal: NalSniff,
}

/// 在 PES 的 ES 数据里就地找第一个图像 NAL：逐包喂入，只记起始码跨包的半截状态，不攒数据。
#[derive(Default)]
struct NalSniff {
    zeros: u8,
    at_header: bool,
    seen: usize,
    verdict: Option<bool>,
}

impl NalSniff {
    fn feed(&mut self, data: &[u8], codec: u8) {
        if self.verdict.is_some() || self.seen >= MAX_SNIFF {
            return;
        }
        for &b in data {
            if self.at_header {
                self.at_header = false;
                if let Some(key) = picture_nal(b, codec) {
                    self.verdict = Some(key);
                    return;
                }
            }
            if b == 0 {
                self.zeros = self.zeros.saturating_add(1);
            } else {
                self.at_header = b == 1 && self.zeros >= 2;
                self.zeros = 0;
            }
        }
        self.seen += data.len();
    }
}

/// 从 `index.scanned_upto` 扫到文件末尾最后一个完整的包。
pub(super) fn scan(
    reader: &mut impl Source,
    file_len: u64,
    index: &mut KeyframeIndex,
) -> io::Result<()> {
    let mut offset = index.scanned_upto;
    if offset == 0 {
        match find_sync(reader, file_len)? {
            Some(first) => offset = first,
            None => return Ok(()),
        }
    }
    reader.seek(SeekFrom::Start(offset))?;
    let mut pending: Option<PendingPes> = None;
    while offset + PACKET <= file_len {
        let packet = match TsPacketRef::parse(reader.read_bytes(PACKET as usize)?) {
            Ok(packet) => packet,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("TS packet at offset {offset}: {e}"),
                ));
            }
        };
        let pid = packet.pid;
        let pmt_pid = index.track.aux as u16;
        let video_pid = index.track.id as u16;
        if pid == PID_PAT {
            if let Some(program) = packet
                .psi_payload()
                .and_then(|p| PatRef::parse(p).ok())
                .and_then(|pat| pat.programs().find(|p| p.program_number != 0))
            {
                index.track.aux = program.pmt_pid as u32;
            }
        } else if pmt_pid != 0 && pid == pmt_pid {
            if let Some(pmt) = packet.psi_payload().and_then(|p| PmtRef::parse(p).ok())
                && let Some(stream) = pmt
                    .streams()
                    .flatten()
                    .find(|s| matches!(s.stream_type, StreamType::H264 | StreamType::H265))
            {
                index.track.id = stream.elementary_pid as u32;
                index.track.codec = if stream.stream_type == StreamType::H265 {
                    CODEC_H265
                } else {
                    CODEC_H264
                };
            }
        } else if video_pid != 0 && pid == video_pid {
            if packet.payload_unit_start_indicator {
                if let Some(done) = pending.take() {
                    resolve(index, done, false);
                }
                index.finalize_header(offset);
                let pes = packet.payload().and_then(|p| PesHeaderRef::parse(p).ok());
                let mut nal = NalSniff::default();
                if let Some(h) = &pes {
                    nal.feed(&h.payload(), index.track.codec);
                }
                pending = Some(PendingPes {
                    offset,
                    pts: pes.as_ref().and_then(|h| h.pts.or(h.dts)).map(|v| v as i64),
                    random_access: packet.has_random_access_indicator(),
                    nal,
                });
            } else if let Some(p) = pending.as_mut()
                && let Some(payload) = packet.payload()
            {
                p.nal.feed(&payload, index.track.codec);
            }
            if let Some(key) = pending.as_ref().and_then(is_idr)
                && let Some(done) = pending.take()
            {
                resolve(index, done, key);
            }
        } else if packet.payload_unit_start_indicator && pid != PID_NULL && pid > 0x1F {
            index.finalize_header(offset);
        }
        offset += PACKET;
        index.scanned_upto = pending.as_ref().map_or(offset, |p| p.offset);
    }
    Ok(())
}

/// 判定完的 PES：是关键帧就记下，并用它的 PTS 推进时长。
fn resolve(index: &mut KeyframeIndex, pes: PendingPes, key: bool) {
    let Some(raw) = pes.pts else {
        return;
    };
    let raw = unwrap_pts(index.base_ts, raw);
    if key {
        index.push_keyframe(raw, pes.offset);
    }
    index.observe_media(raw);
}

/// PTS 是 33 位的，跨过回绕点时接着往上数。
fn unwrap_pts(base: Option<i64>, raw: i64) -> i64 {
    match base {
        Some(base) if raw < base - PTS_WRAP / 2 => raw + PTS_WRAP,
        _ => raw,
    }
}

/// 看 PES 里第一个图像 NAL：IDR → `Some(true)`，非 IDR → `Some(false)`，还没看到 → `None`。
fn is_idr(pes: &PendingPes) -> Option<bool> {
    if pes.random_access {
        return Some(true);
    }
    pes.nal
        .verdict
        .or((pes.nal.seen >= MAX_SNIFF).then_some(false))
}

/// 起始码后的 NAL 头字节：图像 NAL 给出是不是 IDR，其它（SPS/PPS/SEI/AUD…）返回 `None`。
fn picture_nal(header: u8, codec: u8) -> Option<bool> {
    if codec == CODEC_H265 {
        match (header >> 1) & 0x3F {
            16..=21 => Some(true),
            0..=9 => Some(false),
            _ => None,
        }
    } else {
        match header & 0x1F {
            5 => Some(true),
            1 => Some(false),
            _ => None,
        }
    }
}

/// 第一个同步字节的位置：`0x47` 且 188 字节后还是 `0x47`（或已到文件末尾）。
fn find_sync(reader: &mut impl Source, file_len: u64) -> io::Result<Option<u64>> {
    let window = file_len.min(PACKET * 8) as usize;
    let mut buf = vec![0u8; window];
    read_at(reader, 0, &mut buf)?;
    for i in 0..(PACKET as usize).min(window) {
        if buf[i] == 0x47 && (i + PACKET as usize >= window || buf[i + PACKET as usize] == 0x47) {
            return Ok(Some(i as u64));
        }
    }
    Ok(None)
}

/// `offset` 处是不是视频 PID 上的一个 PES 起始包。
pub(super) fn is_unit_start_at(
    reader: &mut impl Source,
    offset: u64,
    file_len: u64,
    video_pid: u32,
) -> bool {
    if offset + PACKET > file_len {
        return false;
    }
    let mut buf = [0u8; PACKET as usize];
    if read_at(reader, offset, &mut buf).is_err() {
        return false;
    }
    matches!(TsPacketRef::parse(Bytes::copy_from_slice(&buf)),
        Ok(p) if p.payload_unit_start_indicator && p.pid as u32 == video_pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 整段 ES 攒齐后一次性找第一个图像 NAL（逐包就地判定必须与它一致）。
    fn whole(es: &[u8], codec: u8) -> Option<bool> {
        let mut i = 0;
        while i + 3 < es.len() {
            if es[i] == 0 && es[i + 1] == 0 && es[i + 2] == 1 {
                if let Some(key) = picture_nal(es[i + 3], codec) {
                    return Some(key);
                }
                i += 3;
            } else {
                i += 1;
            }
        }
        None
    }

    #[test]
    fn nal_sniff_across_packet_splits_matches_the_whole_buffer() {
        let cases: [(&[u8], u8); 9] = [
            (&[0, 0, 0, 1, 0x09, 0xF0, 0, 0, 1, 0x65, 0x88], CODEC_H264),
            (
                &[0, 0, 1, 0x67, 0x42, 0, 0, 1, 0x68, 0, 0, 1, 0x41],
                CODEC_H264,
            ),
            (&[0, 0, 1, 0x06, 0, 0, 0, 0, 1, 0x25], CODEC_H264),
            (&[0, 0, 1, 0, 0, 1, 0x65], CODEC_H264),
            (&[0xAB, 0, 0, 2, 0, 1, 0x65, 0, 0, 1], CODEC_H264),
            (&[0, 0, 1, 0x06, 0, 0], CODEC_H264),
            (
                &[0, 0, 0, 1, 35 << 1, 1, 0x50, 0, 0, 1, 19 << 1, 1],
                CODEC_H265,
            ),
            (&[0, 0, 1, 32 << 1, 1, 0, 0, 1, 1 << 1, 1], CODEC_H265),
            (&[0, 0, 1, 39 << 1, 1, 0, 0, 1, 21 << 1], CODEC_H265),
        ];
        for (es, codec) in cases {
            let want = whole(es, codec);
            for a in 0..=es.len() {
                for b in a..=es.len() {
                    let mut nal = NalSniff::default();
                    for part in [&es[..a], &es[a..b], &es[b..]] {
                        nal.feed(part, codec);
                    }
                    assert_eq!(nal.verdict, want, "{es:02x?} split at {a}/{b}");
                }
            }
        }
    }

    #[test]
    fn nal_sniff_gives_up_after_max_sniff_bytes() {
        let mut nal = NalSniff::default();
        let filler = [0xAB; 184];
        while nal.seen < MAX_SNIFF {
            nal.feed(&filler, CODEC_H264);
        }
        nal.feed(&[0, 0, 1, 0x65], CODEC_H264);
        let pes = PendingPes {
            offset: 0,
            pts: None,
            random_access: false,
            nal,
        };
        assert_eq!(is_idr(&pes), Some(false));
    }
}
