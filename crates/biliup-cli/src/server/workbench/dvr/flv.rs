//! FLV：按 tag 转发，改写 tag 时间戳；script tag（`onMetaData`）一律丢掉。
//!
//! 分段文件里的 `onMetaData` 写的是这个文件的时长和关键帧位置，对跨段、从中间起播的流都不成立，
//! 而 mpegts.js 不依赖它（直播流本来就常常没有）。

use super::{Clock, TimeMap, Tracker};
use bytes::{BufMut, BytesMut};
use std::io;

const FILE_HEADER_SIZE: usize = 9;
pub(in crate::server::workbench) const TAG_HEADER_SIZE: usize = 11;
pub(in crate::server::workbench) const PREV_TAG_SIZE: usize = 4;
pub(in crate::server::workbench) const TAG_AUDIO: u8 = 8;
pub(in crate::server::workbench) const TAG_VIDEO: u8 = 9;
pub(in crate::server::workbench) const TAG_SCRIPT: u8 = 18;

pub(in crate::server::workbench) struct TagRef<'a> {
    pub tag_type: u8,
    pub timestamp: u32,
    /// tag 头 + body + PreviousTagSize
    pub raw: &'a [u8],
}

impl TagRef<'_> {
    pub(in crate::server::workbench) fn body(&self) -> &[u8] {
        &self.raw[TAG_HEADER_SIZE..self.raw.len() - PREV_TAG_SIZE]
    }
}

/// `data` 开头的一个完整 tag；不完整返回 `Ok(None)`。
pub(in crate::server::workbench) fn next_tag(data: &[u8]) -> io::Result<Option<TagRef<'_>>> {
    if data.len() < TAG_HEADER_SIZE {
        return Ok(None);
    }
    let tag_type = data[0] & 0x1F;
    if !matches!(tag_type, TAG_AUDIO | TAG_VIDEO | TAG_SCRIPT) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("FLV tag type {tag_type}"),
        ));
    }
    let size = u32::from_be_bytes([0, data[1], data[2], data[3]]) as usize;
    let total = TAG_HEADER_SIZE + size + PREV_TAG_SIZE;
    if data.len() < total {
        return Ok(None);
    }
    let timestamp = u32::from_be_bytes([data[7], data[4], data[5], data[6]]);
    Ok(Some(TagRef {
        tag_type,
        timestamp,
        raw: &data[..total],
    }))
}

pub(in crate::server::workbench) fn put_tag(out: &mut BytesMut, tag: &TagRef<'_>, timestamp: u32) {
    out.put_u8(tag.raw[0]);
    out.put_slice(&tag.raw[1..4]);
    out.put_slice(&timestamp.to_be_bytes()[1..4]);
    out.put_u8((timestamp >> 24) as u8);
    out.put_slice(&tag.raw[8..]);
}

/// 分段文件的头区（`[0, header_len)`：文件头、`onMetaData`、序列头）里能决定解码器配置的部分。
pub(in crate::server::workbench) struct Header<'a> {
    pub flags: u8,
    /// 头区里的音视频 tag，也就是序列头。
    pub sequence_headers: Vec<TagRef<'a>>,
}

impl<'a> Header<'a> {
    pub(in crate::server::workbench) fn parse(region: &'a [u8]) -> io::Result<Self> {
        if region.len() < FILE_HEADER_SIZE + PREV_TAG_SIZE || &region[..3] != b"FLV" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not an FLV file",
            ));
        }
        let flags = region[4];
        let data_offset = u32::from_be_bytes(region[5..9].try_into().unwrap()) as usize;
        let mut rest = region
            .get(data_offset + PREV_TAG_SIZE..)
            .unwrap_or_default();
        let mut sequence_headers = Vec::new();
        while let Some(tag) = next_tag(rest)? {
            rest = &rest[tag.raw.len()..];
            if tag.tag_type != TAG_SCRIPT {
                sequence_headers.push(tag);
            }
        }
        Ok(Self {
            flags,
            sequence_headers,
        })
    }

    /// 两个分段能不能接成一条流：音视频有无与序列头都相同。
    pub(in crate::server::workbench) fn fingerprint(&self) -> Vec<u8> {
        let mut out = vec![self.flags & 0x05];
        for tag in &self.sequence_headers {
            out.push(tag.tag_type);
            out.extend_from_slice(&(tag.body().len() as u32).to_be_bytes());
            out.extend_from_slice(tag.body());
        }
        out
    }

    /// 响应开头：FLV 文件头 + 序列头（时间戳记为起播关键帧的时刻）。
    pub(super) fn stream_header(&self, start_ts: u32) -> BytesMut {
        let mut out = BytesMut::new();
        out.put_slice(b"FLV\x01");
        out.put_u8(self.flags);
        out.put_u32(FILE_HEADER_SIZE as u32);
        out.put_u32(0);
        for tag in &self.sequence_headers {
            put_tag(&mut out, tag, start_ts);
        }
        out
    }
}

pub(super) fn out_ts(map: &TimeMap, raw: u32) -> u32 {
    map.out_units(raw as i64 - map.base)
        .clamp(0, u32::MAX as i64) as u32
}

/// 从 `input` 开头（tag 边界）起转发完整的 tag，改写时间戳后追加到 `out`，`out` 攒到 `limit`
/// 就停。返回消耗的字节数，末尾不完整的 tag 留给下一轮。
pub(super) fn process(
    input: &[u8],
    out: &mut BytesMut,
    limit: usize,
    map: &TimeMap,
    clock: &mut Clock,
    video: &mut Tracker,
) -> io::Result<usize> {
    let mut consumed = 0;
    while out.len() < limit {
        let Some(tag) = next_tag(&input[consumed..])? else {
            break;
        };
        consumed += tag.raw.len();
        if tag.tag_type == TAG_SCRIPT {
            continue;
        }
        let ts = out_ts(map, tag.timestamp);
        clock.observe(ts as i64);
        if tag.tag_type == TAG_VIDEO {
            video.observe(ts as i64);
        }
        put_tag(out, &tag, ts);
    }
    Ok(consumed)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub fn tag(tag_type: u8, timestamp: u32, body: &[u8]) -> Vec<u8> {
        let mut out = vec![tag_type];
        out.extend_from_slice(&(body.len() as u32).to_be_bytes()[1..]);
        out.extend_from_slice(&timestamp.to_be_bytes()[1..]);
        out.push((timestamp >> 24) as u8);
        out.extend_from_slice(&[0, 0, 0]);
        out.extend_from_slice(body);
        out.extend_from_slice(&((TAG_HEADER_SIZE + body.len()) as u32).to_be_bytes());
        out
    }

    pub fn file_header() -> Vec<u8> {
        let mut out = b"FLV\x01\x05\x00\x00\x00\x09".to_vec();
        out.extend_from_slice(&[0, 0, 0, 0]);
        out
    }

    #[test]
    fn rewrites_timestamps_drops_script_and_keeps_partial_tail() {
        let mut data = tag(TAG_SCRIPT, 0, b"meta");
        data.extend(tag(TAG_VIDEO, 17_000, &[0x17, 1, 0, 0, 0]));
        data.extend(tag(TAG_AUDIO, 17_010, &[0xAF, 1]));
        let whole = data.len();
        data.extend(&tag(TAG_VIDEO, 17_033, &[0x27, 1, 0, 0, 0])[..7]);
        let map = TimeMap {
            base: 17_000,
            timescale: 1000,
            origin_ms: 61_000,
        };
        let mut out = BytesMut::new();
        let mut clock = Clock::default();
        let mut video = Tracker::default();
        let used = process(&data, &mut out, usize::MAX, &map, &mut clock, &mut video).unwrap();
        assert_eq!(used, whole);
        let first = next_tag(&out).unwrap().unwrap();
        assert_eq!((first.tag_type, first.timestamp), (TAG_VIDEO, 61_000));
        let second = next_tag(&out[first.raw.len()..]).unwrap().unwrap();
        assert_eq!((second.tag_type, second.timestamp), (TAG_AUDIO, 61_010));
        assert_eq!(out.len(), first.raw.len() + second.raw.len());
        assert_eq!(video.last, Some(61_000));
    }

    #[test]
    fn header_keeps_sequence_headers_and_fingerprints_them() {
        let mut region = file_header();
        region.extend(tag(TAG_SCRIPT, 0, b"onMetaData"));
        region.extend(tag(TAG_VIDEO, 0, &[0x17, 0, 0, 0, 0, 1, 2, 3]));
        region.extend(tag(TAG_AUDIO, 0, &[0xAF, 0, 0x12, 0x10]));
        let header = Header::parse(&region).unwrap();
        assert_eq!(header.sequence_headers.len(), 2);
        let out = header.stream_header(5_000);
        assert_eq!(&out[..13], &file_header()[..]);
        let first = next_tag(&out[13..]).unwrap().unwrap();
        assert_eq!((first.tag_type, first.timestamp), (TAG_VIDEO, 5_000));

        let mut other = file_header();
        other.extend(tag(TAG_VIDEO, 0, &[0x17, 0, 0, 0, 0, 1, 2, 4]));
        other.extend(tag(TAG_AUDIO, 0, &[0xAF, 0, 0x12, 0x10]));
        assert_ne!(
            Header::parse(&other).unwrap().fingerprint(),
            header.fingerprint()
        );
        let mut same = file_header();
        same.extend(tag(TAG_SCRIPT, 0, b"different metadata"));
        same.extend(tag(TAG_VIDEO, 99, &[0x17, 0, 0, 0, 0, 1, 2, 3]));
        same.extend(tag(TAG_AUDIO, 99, &[0xAF, 0, 0x12, 0x10]));
        assert_eq!(
            Header::parse(&same).unwrap().fingerprint(),
            header.fingerprint()
        );
    }
}
