//! MPEG-TS：文件开头是第一段头区里的 PAT / PMT，之后按 188 字节包复制：每段从关键帧那个视频 PES
//! 的起始包开始，改写 PES 的 PTS / DTS 和适配域里的 PCR，各 PID 的连续计数器重新往下数；
//! 每段里各 PID 第一个 PES 起始包之前的半截 PES 丢掉，空包（PID 0x1FFF）丢掉。

use super::super::plan::Piece;
use super::{Cut, VideoClock};
use crate::server::workbench::dvr::ts::{
    PACKET, Program, SYNC, WRAP, delta, payload_start, pid_of, pusi, read_ts, write_ts,
};
use std::collections::{HashMap, HashSet};
use std::io;

const NULL_PID: u16 = 0x1FFF;
const UNITS_PER_MS: i64 = 90;

#[derive(Debug, Default)]
pub(super) struct TsCut {
    program: Program,
    /// 每个 PID 下一个要写的连续计数器。
    next_cc: HashMap<u16, u8>,
    /// 这一段里已经见过 PES 起始包的 PID。
    started: HashSet<u16>,
    /// 当前 PES 被丢掉的 PID（早于起始关键帧的音频）。
    dropping: HashSet<u16>,
    origin_ms: i64,
    raw_origin: Option<i64>,
    video: VideoClock,
    last_pts: HashMap<u16, i64>,
}

impl TsCut {
    fn out_units(&self, raw: i64) -> i64 {
        delta(raw, self.raw_origin.unwrap_or(raw)) + self.origin_ms * UNITS_PER_MS
    }

    fn renumber(&mut self, p: &mut [u8]) {
        let pid = pid_of(p);
        let has_payload = (p[3] >> 4) & 0x1 != 0;
        let next = self.next_cc.entry(pid).or_insert(0);
        let cc = if has_payload {
            let cc = *next;
            *next = (cc + 1) & 0x0F;
            cc
        } else {
            next.wrapping_sub(1) & 0x0F
        };
        p[3] = (p[3] & 0xF0) | cc;
    }

    fn rewrite_pcr(&self, p: &mut [u8]) {
        let afc = (p[3] >> 4) & 0x3;
        if afc & 0x2 == 0 || p[4] < 7 || p[5] & 0x10 == 0 || self.raw_origin.is_none() {
            return;
        }
        let pcr = &mut p[6..12];
        let base = (pcr[0] as i64) << 25
            | (pcr[1] as i64) << 17
            | (pcr[2] as i64) << 9
            | (pcr[3] as i64) << 1
            | (pcr[4] >> 7) as i64;
        let out = self.out_units(base).max(0) % WRAP;
        pcr[0] = (out >> 25) as u8;
        pcr[1] = (out >> 17) as u8;
        pcr[2] = (out >> 9) as u8;
        pcr[3] = (out >> 1) as u8;
        pcr[4] = (pcr[4] & 0x7F) | (((out & 1) as u8) << 7);
    }

    /// 处理一个 PES 起始包；返回 `false` 表示这个 PES 要丢掉。
    fn rewrite_pes_start(&mut self, p: &mut [u8], pid: u16) -> bool {
        let Some(start) = payload_start(p) else {
            return true;
        };
        let pes = &mut p[start..];
        if pes.len() < 14 || pes[..3] != [0, 0, 1] || (pes[7] >> 6) & 0x2 == 0 {
            return true;
        }
        let has_dts = pes[7] >> 6 == 0x3 && pes.len() >= 19;
        let pts = read_ts(&pes[9..14]);
        let dts = if has_dts { read_ts(&pes[14..19]) } else { pts };
        let is_video = self.program.video().is_some_and(|(v, _)| v == pid);
        if self.raw_origin.is_none() {
            if !is_video {
                return false;
            }
            self.raw_origin = Some(dts);
        }
        let out_pts = self.out_units(pts);
        let out_dts = self.out_units(dts);
        if is_video {
            self.video.observe(out_dts.max(0) / UNITS_PER_MS);
        } else if out_pts < 0 || self.last_pts.get(&pid).is_some_and(|last| out_pts < *last) {
            return false;
        } else {
            self.last_pts.insert(pid, out_pts);
        }
        write_ts(&mut pes[9..14], out_pts.max(0));
        if has_dts {
            write_ts(&mut pes[14..19], out_dts.max(0));
        }
        true
    }
}

impl Cut for TsCut {
    fn header(&mut self, region: &[u8], out: &mut Vec<u8>) -> io::Result<()> {
        self.program = Program::parse(region);
        if self.program.video().is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "TS 分段里没有找到 H.264 / H.265 视频流",
            ));
        }
        for p in region.as_chunks::<PACKET>().0 {
            if p[0] != SYNC || pid_of(p) == NULL_PID || self.program.is_pes(pid_of(p)) {
                continue;
            }
            let mut packet = *p;
            self.renumber(&mut packet);
            out.extend_from_slice(&packet);
        }
        Ok(())
    }

    fn begin(&mut self, origin_ms: i64, _piece: &Piece) {
        self.origin_ms = origin_ms;
        self.raw_origin = None;
        self.started.clear();
        self.dropping.clear();
    }

    fn process(&mut self, input: &[u8], out: &mut Vec<u8>) -> io::Result<usize> {
        let mut consumed = 0;
        let mut packet = [0u8; PACKET];
        while input.len() - consumed >= PACKET {
            packet.copy_from_slice(&input[consumed..consumed + PACKET]);
            consumed += PACKET;
            if packet[0] != SYNC {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "TS packet lost sync",
                ));
            }
            let pid = pid_of(&packet);
            if pid == NULL_PID {
                continue;
            }
            if self.program.is_pes(pid) {
                if pusi(&packet) {
                    self.started.insert(pid);
                    if self.rewrite_pes_start(&mut packet, pid) {
                        self.dropping.remove(&pid);
                    } else {
                        self.dropping.insert(pid);
                    }
                }
                if !self.started.contains(&pid) || self.dropping.contains(&pid) {
                    continue;
                }
            }
            self.rewrite_pcr(&mut packet);
            self.renumber(&mut packet);
            out.extend_from_slice(&packet);
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
