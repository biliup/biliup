//! FLV：文件头 + 新的 `onMetaData` + 序列头，之后逐 tag 复制并改写时间戳；源文件里的 script tag
//! 一律丢掉（它们写的是分段自己的时长和关键帧位置）。
//!
//! `onMetaData` 与 flv-fix 的脚本填充器写的字段一致（`duration`、`filesize`、`keyframes.times` /
//! `filepositions` 等）：关键帧个数事先从索引里就知道，先写同样大小的占位，写完产物后回填。

use super::super::plan::Piece;
use super::{Cut, VideoClock};
use crate::server::workbench::dvr::flv::{
    Header, TAG_AUDIO, TAG_HEADER_SIZE, TAG_SCRIPT, TAG_VIDEO, next_tag, put_tag,
};
use bytes::{BufMut, BytesMut};
use std::io;

const FILE_HEADER_SIZE: usize = 9;

#[derive(Debug, Default)]
struct Codecs {
    has_video: bool,
    has_audio: bool,
    video: Option<u8>,
    audio: Option<u8>,
}

#[derive(Debug)]
pub(super) struct FlvCut {
    /// 要写 `onMetaData` 时，产物里关键帧的个数。
    keyframes: Option<usize>,
    codecs: Codecs,
    meta_offset: u64,
    written: u64,
    /// 已写出的关键帧：`(产物时间 ms, 产物里的偏移)`。
    index: Vec<(i64, u64)>,
    /// 这一段里关键帧的源偏移（升序）与下一个要找的位置。
    piece_keyframes: Vec<u64>,
    next_keyframe: usize,
    src: u64,
    origin_ms: i64,
    raw_origin: Option<i64>,
    video: VideoClock,
    last_audio: Option<i64>,
    last_ms: i64,
}

impl FlvCut {
    pub(super) fn new(keyframes: Option<usize>) -> Self {
        Self {
            keyframes,
            codecs: Codecs::default(),
            meta_offset: 0,
            written: 0,
            index: Vec::new(),
            piece_keyframes: Vec::new(),
            next_keyframe: 0,
            src: 0,
            origin_ms: 0,
            raw_origin: None,
            video: VideoClock::default(),
            last_audio: None,
            last_ms: 0,
        }
    }

    fn metadata(&self, total_len: u64, duration_ms: i64) -> Vec<u8> {
        let count = self.keyframes.unwrap_or(0);
        let mut times: Vec<f64> = self.index.iter().map(|k| k.0 as f64 / 1000.0).collect();
        let mut positions: Vec<f64> = self.index.iter().map(|k| k.1 as f64).collect();
        times.resize(count, times.last().copied().unwrap_or(0.0));
        positions.resize(count, positions.last().copied().unwrap_or(0.0));
        times.truncate(count);
        positions.truncate(count);
        let mut entries: Vec<(&str, Amf)> = vec![
            ("duration", Amf::Number(duration_ms as f64 / 1000.0)),
            ("filesize", Amf::Number(total_len as f64)),
        ];
        if let Some(id) = self.codecs.video {
            entries.push(("videocodecid", Amf::Number(id as f64)));
        }
        if let Some(id) = self.codecs.audio {
            entries.push(("audiocodecid", Amf::Number(id as f64)));
        }
        entries.extend([
            ("lasttimestamp", Amf::Number(self.last_ms as f64 / 1000.0)),
            (
                "lastkeyframetimestamp",
                Amf::Number(times.last().copied().unwrap_or(0.0)),
            ),
            (
                "lastkeyframelocation",
                Amf::Number(positions.last().copied().unwrap_or(0.0)),
            ),
            ("hasVideo", Amf::Bool(self.codecs.has_video)),
            ("hasAudio", Amf::Bool(self.codecs.has_audio)),
            ("hasMetadata", Amf::Bool(true)),
            ("hasKeyframes", Amf::Bool(count > 0)),
            ("canSeekToEnd", Amf::Bool(true)),
            ("metadatacreator", Amf::String("biliup")),
            (
                "keyframes",
                Amf::Object(vec![
                    ("times", Amf::Array(times)),
                    ("filepositions", Amf::Array(positions)),
                ]),
            ),
        ]);
        let mut body = Vec::new();
        Amf::String("onMetaData").encode(&mut body);
        body.push(0x08);
        body.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for (key, value) in &entries {
            amf_key(&mut body, key);
            value.encode(&mut body);
        }
        body.extend_from_slice(&[0, 0, 9]);
        let mut out = vec![TAG_SCRIPT];
        out.extend_from_slice(&(body.len() as u32).to_be_bytes()[1..]);
        out.extend_from_slice(&[0; 7]);
        out.extend_from_slice(&body);
        out.extend_from_slice(&((TAG_HEADER_SIZE + body.len()) as u32).to_be_bytes());
        out
    }
}

enum Amf<'a> {
    Number(f64),
    Bool(bool),
    String(&'a str),
    Object(Vec<(&'a str, Amf<'a>)>),
    Array(Vec<f64>),
}

fn amf_key(out: &mut Vec<u8>, key: &str) {
    out.extend_from_slice(&(key.len() as u16).to_be_bytes());
    out.extend_from_slice(key.as_bytes());
}

impl Amf<'_> {
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Amf::Number(v) => {
                out.push(0x00);
                out.extend_from_slice(&v.to_be_bytes());
            }
            Amf::Bool(v) => {
                out.push(0x01);
                out.push(u8::from(*v));
            }
            Amf::String(s) => {
                out.push(0x02);
                amf_key(out, s);
            }
            Amf::Object(entries) => {
                out.push(0x03);
                for (key, value) in entries {
                    amf_key(out, key);
                    value.encode(out);
                }
                out.extend_from_slice(&[0, 0, 9]);
            }
            Amf::Array(values) => {
                out.push(0x0A);
                out.extend_from_slice(&(values.len() as u32).to_be_bytes());
                for v in values {
                    out.push(0x00);
                    out.extend_from_slice(&v.to_be_bytes());
                }
            }
        }
    }
}

fn put(out: &mut Vec<u8>, written: &mut u64, bytes: &[u8]) {
    out.extend_from_slice(bytes);
    *written += bytes.len() as u64;
}

impl Cut for FlvCut {
    fn header(&mut self, region: &[u8], out: &mut Vec<u8>) -> io::Result<()> {
        let header = Header::parse(region)?;
        let mut head = BytesMut::new();
        head.put_slice(b"FLV\x01");
        head.put_u8(header.flags);
        head.put_u32(FILE_HEADER_SIZE as u32);
        head.put_u32(0);
        put(out, &mut self.written, &head);
        self.codecs.has_video = header.flags & 0x01 != 0;
        self.codecs.has_audio = header.flags & 0x04 != 0;
        for tag in &header.sequence_headers {
            let first = tag.body().first().copied().unwrap_or(0);
            match tag.tag_type {
                TAG_VIDEO => {
                    self.codecs.has_video = true;
                    // Enhanced-FLV（最高位置 1）没有 codec id，写不了这个字段
                    if first & 0x80 == 0 {
                        self.codecs.video = Some(first & 0x0F);
                    }
                }
                TAG_AUDIO => {
                    self.codecs.has_audio = true;
                    self.codecs.audio = Some(first >> 4);
                }
                _ => {}
            }
        }
        if self.keyframes.is_some() {
            self.meta_offset = self.written;
            let placeholder = self.metadata(0, 0);
            put(out, &mut self.written, &placeholder);
        }
        let mut tags = BytesMut::new();
        for tag in &header.sequence_headers {
            put_tag(&mut tags, tag, 0);
        }
        put(out, &mut self.written, &tags);
        Ok(())
    }

    fn begin(&mut self, origin_ms: i64, piece: &Piece) {
        self.origin_ms = origin_ms;
        self.src = piece.from;
        self.raw_origin = None;
        self.piece_keyframes = piece.keyframes.iter().map(|k| k.0).collect();
        self.next_keyframe = 0;
    }

    fn process(&mut self, input: &[u8], out: &mut Vec<u8>) -> io::Result<usize> {
        let mut consumed = 0;
        let mut tag_buf = BytesMut::new();
        while let Some(tag) = next_tag(&input[consumed..])? {
            let src = self.src;
            let len = tag.raw.len();
            consumed += len;
            self.src += len as u64;
            if tag.tag_type == TAG_SCRIPT {
                continue;
            }
            let raw = tag.timestamp as i64;
            let origin = *self.raw_origin.get_or_insert(raw);
            let ts = raw - origin + self.origin_ms;
            if tag.tag_type == TAG_AUDIO {
                if ts < 0 || self.last_audio.is_some_and(|last| ts < last) {
                    continue;
                }
                self.last_audio = Some(ts);
            }
            let ts = ts.max(0);
            while self
                .piece_keyframes
                .get(self.next_keyframe)
                .is_some_and(|k| *k < src)
            {
                self.next_keyframe += 1;
            }
            if self.piece_keyframes.get(self.next_keyframe) == Some(&src) {
                self.index.push((ts, self.written));
                self.next_keyframe += 1;
            }
            if tag.tag_type == TAG_VIDEO {
                self.video.observe(ts);
            }
            self.last_ms = self.last_ms.max(ts);
            tag_buf.clear();
            put_tag(&mut tag_buf, &tag, ts.min(u32::MAX as i64) as u32);
            debug_assert_eq!(tag_buf.len(), len);
            put(out, &mut self.written, &tag_buf);
        }
        Ok(consumed)
    }

    fn video(&self) -> VideoClock {
        self.video
    }

    fn finish(&mut self, total_len: u64, duration_ms: i64) -> Option<(u64, Vec<u8>)> {
        self.keyframes?;
        Some((self.meta_offset, self.metadata(total_len, duration_ms)))
    }
}
