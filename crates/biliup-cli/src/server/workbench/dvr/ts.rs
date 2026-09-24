//! MPEG-TS：按 188 字节包转发，改写 PES 的 PTS / DTS 和适配域里的 PCR；
//! 接上下一个分段时把各 PID 的连续计数器接着上一段往下数。

use super::{Clock, TimeMap, Tracker};
use bytes::{BufMut, BytesMut};
use std::collections::HashMap;
use std::io;

pub(super) const PACKET: usize = 188;
const SYNC: u8 = 0x47;
const PID_PAT: u16 = 0;
const WRAP: i64 = 1 << 33;
const STREAM_H264: u8 = 0x1B;
const STREAM_H265: u8 = 0x24;
/// 从关键帧起最多看这么多字节找参数集。
const PARAM_SNIFF: usize = 256 * 1024;

fn pid_of(p: &[u8]) -> u16 {
    (((p[1] & 0x1F) as u16) << 8) | p[2] as u16
}

fn pusi(p: &[u8]) -> bool {
    p[1] & 0x40 != 0
}

/// 包里负载的起点；没有负载返回 `None`。
fn payload_start(p: &[u8]) -> Option<usize> {
    let afc = (p[3] >> 4) & 0x3;
    let start = match afc {
        1 => 4,
        3 => 5 + p[4] as usize,
        _ => return None,
    };
    (start < PACKET).then_some(start)
}

fn psi_section(p: &[u8]) -> Option<&[u8]> {
    let start = payload_start(p)?;
    let pointer = *p.get(start)? as usize;
    let section = p.get(start + 1 + pointer..)?;
    let len = (((*section.get(1)? & 0x0F) as usize) << 8) | *section.get(2)? as usize;
    section.get(..3 + len)
}

/// 头区里的 PAT / PMT：节目的 PMT PID 与各路基本流。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Program {
    pub pmt_pid: u16,
    /// `(elementary_pid, stream_type)`，按 PMT 里的顺序。
    pub streams: Vec<(u16, u8)>,
}

impl Program {
    pub(super) fn parse(region: &[u8]) -> Self {
        let mut program = Program::default();
        for p in region
            .as_chunks::<PACKET>()
            .0
            .iter()
            .filter(|p| p[0] == SYNC)
        {
            let pid = pid_of(p);
            if !pusi(p) {
                continue;
            }
            let Some(section) = psi_section(p) else {
                continue;
            };
            if pid == PID_PAT && section.first() == Some(&0x00) && section.len() >= 12 {
                let entries = &section[8..section.len() - 4];
                if let Some(e) = entries
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .find(|e| u16::from_be_bytes([e[0], e[1]]) != 0)
                {
                    program.pmt_pid = u16::from_be_bytes([e[2] & 0x1F, e[3]]);
                }
            } else if program.pmt_pid != 0
                && pid == program.pmt_pid
                && section.first() == Some(&0x02)
                && section.len() >= 16
            {
                let info_len = (((section[10] & 0x0F) as usize) << 8) | section[11] as usize;
                let mut rest = section
                    .get(12 + info_len..section.len() - 4)
                    .unwrap_or_default();
                program.streams.clear();
                while rest.len() >= 5 {
                    let es_pid = u16::from_be_bytes([rest[1] & 0x1F, rest[2]]);
                    let es_info = (((rest[3] & 0x0F) as usize) << 8) | rest[4] as usize;
                    program.streams.push((es_pid, rest[0]));
                    rest = rest.get(5 + es_info..).unwrap_or_default();
                }
            }
        }
        program
    }

    pub(super) fn video(&self) -> Option<(u16, u8)> {
        self.streams
            .iter()
            .copied()
            .find(|(_, t)| matches!(*t, STREAM_H264 | STREAM_H265))
    }

    fn is_pes(&self, pid: u16) -> bool {
        self.streams.iter().any(|(p, _)| *p == pid)
    }
}

/// 关键帧那个视频 PES 里的参数集（H.264 SPS / PPS，H.265 VPS / SPS / PPS），`data` 从关键帧的 PES
/// 起始包开始。换分辨率、换编码参数时它们会变。
pub(super) fn parameter_sets(data: &[u8], program: &Program) -> Vec<u8> {
    let Some((video_pid, stream_type)) = program.video() else {
        return Vec::new();
    };
    let mut es = Vec::new();
    let mut started = false;
    for p in data
        .as_chunks::<PACKET>()
        .0
        .iter()
        .take(PARAM_SNIFF / PACKET)
    {
        if p[0] != SYNC || pid_of(p) != video_pid {
            continue;
        }
        let Some(start) = payload_start(p) else {
            continue;
        };
        if pusi(p) {
            if started {
                break;
            }
            started = true;
            let pes = &p[start..];
            if pes.len() < 9 || pes[..3] != [0, 0, 1] {
                return Vec::new();
            }
            es.extend_from_slice(pes.get(9 + pes[8] as usize..).unwrap_or_default());
        } else if started {
            es.extend_from_slice(&p[start..]);
        }
    }
    let mut out = Vec::new();
    let mut i = 0;
    let starts: Vec<usize> = std::iter::from_fn(|| {
        while i + 3 <= es.len() {
            if es[i] == 0 && es[i + 1] == 0 && es[i + 2] == 1 {
                i += 3;
                return Some(i);
            }
            i += 1;
        }
        None
    })
    .collect();
    for (n, &s) in starts.iter().enumerate() {
        let end = starts.get(n + 1).map_or(es.len(), |next| next - 3);
        let nal = &es[s..end];
        let Some(&head) = nal.first() else { continue };
        let keep = if stream_type == STREAM_H265 {
            matches!((head >> 1) & 0x3F, 32..=34)
        } else {
            matches!(head & 0x1F, 7 | 8)
        };
        if keep {
            // 下一个起始码前的 0 可能是 4 字节起始码的一部分
            let trimmed = nal.len() - nal.iter().rev().take_while(|b| **b == 0).count();
            out.extend_from_slice(&(trimmed as u32).to_be_bytes());
            out.extend_from_slice(&nal[..trimmed]);
        }
    }
    out
}

fn read_ts(b: &[u8]) -> i64 {
    (((b[0] >> 1) & 0x07) as i64) << 30
        | (b[1] as i64) << 22
        | ((b[2] >> 1) as i64) << 15
        | (b[3] as i64) << 7
        | (b[4] >> 1) as i64
}

fn write_ts(b: &mut [u8], v: i64) {
    let v = v.rem_euclid(WRAP);
    b[0] = (b[0] & 0xF1) | (((v >> 30) & 0x07) as u8) << 1;
    b[1] = (v >> 22) as u8;
    b[2] = (((v >> 15) & 0x7F) as u8) << 1 | 1;
    b[3] = (v >> 7) as u8;
    b[4] = ((v & 0x7F) as u8) << 1 | 1;
}

/// 33 位时间戳相对 `base` 的差，跨过回绕点也按最近的方向算。
fn delta(raw: i64, base: i64) -> i64 {
    let d = (raw - base).rem_euclid(WRAP);
    if d >= WRAP / 2 { d - WRAP } else { d }
}

/// 一条 TS 输出流的状态：各 PID 的连续计数器偏移。
#[derive(Default)]
pub(super) struct Ts {
    pub program: Program,
    /// 已发出的每个 PID 下一个应有的连续计数器。
    next_cc: HashMap<u16, u8>,
    /// 本段各 PID 的计数器偏移（接段时按第一个包算出）。
    cc_shift: HashMap<u16, u8>,
}

impl Ts {
    pub(super) fn new(program: Program) -> Self {
        Self {
            program,
            ..Default::default()
        }
    }

    /// 换到下一个分段：计数器偏移按新段各 PID 的第一个包重新算。
    pub(super) fn start_segment(&mut self) {
        self.cc_shift.clear();
    }

    fn rewrite_packet(
        &mut self,
        p: &mut [u8],
        map: &TimeMap,
        clock: &mut Clock,
        video: &mut Tracker,
    ) {
        let pid = pid_of(p);
        let afc = (p[3] >> 4) & 0x3;
        if afc & 0x1 != 0 {
            let cc = p[3] & 0x0F;
            let shift = *self.cc_shift.entry(pid).or_insert_with(|| {
                self.next_cc
                    .get(&pid)
                    .map_or(0, |next| next.wrapping_sub(cc) & 0x0F)
            });
            let out = (cc + shift) & 0x0F;
            p[3] = (p[3] & 0xF0) | out;
            self.next_cc.insert(pid, (out + 1) & 0x0F);
        }
        if afc & 0x2 != 0 && p[4] >= 7 && p[5] & 0x10 != 0 {
            let pcr = &mut p[6..12];
            let base = (pcr[0] as i64) << 25
                | (pcr[1] as i64) << 17
                | (pcr[2] as i64) << 9
                | (pcr[3] as i64) << 1
                | (pcr[4] >> 7) as i64;
            let out = map.out_units(delta(base, map.base)).rem_euclid(WRAP);
            pcr[0] = (out >> 25) as u8;
            pcr[1] = (out >> 17) as u8;
            pcr[2] = (out >> 9) as u8;
            pcr[3] = (out >> 1) as u8;
            pcr[4] = (pcr[4] & 0x7F) | (((out & 1) as u8) << 7);
        }
        if !pusi(p) || !self.program.is_pes(pid) {
            return;
        }
        let Some(start) = payload_start(p) else {
            return;
        };
        let pes = &mut p[start..];
        if pes.len() < 14 || pes[..3] != [0, 0, 1] {
            return;
        }
        let flags = pes[7] >> 6;
        if flags & 0x2 == 0 {
            return;
        }
        let pts = map.out_units(delta(read_ts(&pes[9..14]), map.base));
        write_ts(&mut pes[9..14], pts);
        if flags == 0x3 && pes.len() >= 19 {
            let dts = map.out_units(delta(read_ts(&pes[14..19]), map.base));
            write_ts(&mut pes[14..19], dts);
        }
        let ms = map.ms_of(pts);
        clock.observe(ms);
        if self.program.video().is_some_and(|(v, _)| v == pid) {
            video.observe(ms);
        }
    }

    /// 从 `input` 开头（包边界）起转发完整的包，改写后追加到 `out`，`out` 攒到 `limit` 就停。
    pub(super) fn process(
        &mut self,
        input: &[u8],
        out: &mut BytesMut,
        limit: usize,
        map: &TimeMap,
        clock: &mut Clock,
        video: &mut Tracker,
    ) -> io::Result<usize> {
        let mut consumed = 0;
        let mut packet = [0u8; PACKET];
        while out.len() < limit && input.len() - consumed >= PACKET {
            packet.copy_from_slice(&input[consumed..consumed + PACKET]);
            if packet[0] != SYNC {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "TS packet lost sync",
                ));
            }
            consumed += PACKET;
            self.rewrite_packet(&mut packet, map, clock, video);
            out.put_slice(&packet);
        }
        Ok(consumed)
    }

    /// 响应开头：头区的 PAT / PMT 原样发出（计数器从这里开始记）。
    pub(super) fn stream_header(&mut self, region: &[u8], map: &TimeMap) -> BytesMut {
        let mut out = BytesMut::new();
        let mut clock = Clock::default();
        let mut video = Tracker::default();
        let usable = region.len() / PACKET * PACKET;
        let _ = self.process(
            &region[..usable],
            &mut out,
            usize::MAX,
            map,
            &mut clock,
            &mut video,
        );
        out
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    fn packet(pid: u16, pusi: bool, cc: u8, payload: &[u8]) -> Vec<u8> {
        let mut p = vec![
            SYNC,
            (u8::from(pusi) << 6) | (pid >> 8) as u8,
            pid as u8,
            0x10 | cc,
        ];
        p.extend_from_slice(payload);
        p.resize(PACKET, 0xFF);
        p
    }

    fn section(table: &[u8]) -> Vec<u8> {
        let mut s = vec![0u8];
        s.extend_from_slice(table);
        s
    }

    pub fn pat_pmt(video_pid: u16) -> Vec<u8> {
        // PAT：program 1 → PMT PID 0x1000
        let pat = section(&[
            0x00, 0xB0, 0x0D, 0, 1, 0xC1, 0, 0, 0, 1, 0xF0, 0x00, 0, 0, 0, 0,
        ]);
        let pmt = section(&[
            0x02,
            0xB0,
            0x17,
            0,
            1,
            0xC1,
            0,
            0,
            0xE0 | (video_pid >> 8) as u8,
            video_pid as u8,
            0xF0,
            0,
            STREAM_H264,
            0xE0 | (video_pid >> 8) as u8,
            video_pid as u8,
            0xF0,
            0,
            0x0F,
            0xE1,
            0x01,
            0xF0,
            0,
            0,
            0,
            0,
            0,
        ]);
        let mut out = packet(0, true, 0, &pat);
        out.extend(packet(0x1000, true, 0, &pmt));
        out
    }

    fn pes(pts: i64, dts: Option<i64>, es: &[u8]) -> Vec<u8> {
        let mut p = vec![0, 0, 1, 0xE0, 0, 0, 0x80];
        let mut stamps = [0u8; 10];
        let flags = if let Some(dts) = dts {
            stamps[0] = 0x31;
            write_ts(&mut stamps[..5], pts);
            stamps[5] = 0x11;
            write_ts(&mut stamps[5..], dts);
            p.extend_from_slice(&[0xC0, 10]);
            10
        } else {
            stamps[0] = 0x21;
            write_ts(&mut stamps[..5], pts);
            p.extend_from_slice(&[0x80, 5]);
            5
        };
        p.extend_from_slice(&stamps[..flags]);
        p.extend_from_slice(es);
        p
    }

    #[test]
    fn parses_program_and_parameter_sets() {
        let region = pat_pmt(0x100);
        let program = Program::parse(&region);
        assert_eq!(program.pmt_pid, 0x1000);
        assert_eq!(program.streams, vec![(0x100, STREAM_H264), (0x101, 0x0F)]);
        let es = [
            0, 0, 0, 1, 0x09, 0xF0, 0, 0, 0, 1, 0x67, 0x64, 0, 0x1F, 0, 0, 1, 0x68, 0xEE, 0, 0, 1,
            0x65, 0x88,
        ];
        let data = packet(0x100, true, 0, &pes(90_000, None, &es));
        let sets = parameter_sets(&data, &program);
        assert_eq!(
            sets,
            [0, 0, 0, 4, 0x67, 0x64, 0, 0x1F, 0, 0, 0, 2, 0x68, 0xEE]
        );
    }

    #[test]
    fn rewrites_pts_dts_across_wrap_and_splices_continuity_counters() {
        let program = Program::parse(&pat_pmt(0x100));
        let mut ts = Ts::new(program);
        let base = WRAP - 90_000; // 源时间戳 1 秒后回绕
        let map = TimeMap {
            base,
            timescale: 90_000,
            origin_ms: 2_000,
        };
        let mut input = packet(
            0x100,
            true,
            5,
            &pes(base, Some(base - 3_000), &[0, 0, 1, 0x65]),
        );
        input.extend(packet(0x100, false, 6, &[]));
        input.extend(packet(0x100, true, 7, &pes(90_000, None, &[0, 0, 1, 0x41])));
        let mut out = BytesMut::new();
        let (mut clock, mut video) = (Clock::default(), Tracker::default());
        let used = ts
            .process(&input, &mut out, usize::MAX, &map, &mut clock, &mut video)
            .unwrap();
        assert_eq!(used, input.len());
        let first = &out[..PACKET];
        assert_eq!(read_ts(&first[4 + 9..4 + 14]), 180_000);
        assert_eq!(read_ts(&first[4 + 14..4 + 19]), 177_000);
        // 回绕后的 90_000 = base 之后 2 秒
        assert_eq!(read_ts(&out[2 * PACKET + 13..2 * PACKET + 18]), 360_000);
        assert_eq!(video.last, Some(4_000));

        // 下一段的计数器从 3 开始，接着上一段的 8 往下数
        ts.start_segment();
        let next = packet(0x100, true, 3, &pes(0, None, &[0, 0, 1, 0x65]));
        let mut out = BytesMut::new();
        ts.process(&next, &mut out, usize::MAX, &map, &mut clock, &mut video)
            .unwrap();
        assert_eq!(out[3] & 0x0F, 8);
    }
}
