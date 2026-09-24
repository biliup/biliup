//! FLV：逐 tag 读 11 字节 tag 头和 body 开头几个字节判定关键帧，body 其余部分跳过。

use super::{KeyframeIndex, read_at};
use ::flv::FlvTag;
use ::flv::framing::{PREV_TAG_SIZE_FIELD_SIZE, TAG_HEADER_SIZE, parse_tag_header_bytes};
use ::flv::tag::FlvTagType;
use bytes::Bytes;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};

const FILE_HEADER_SIZE: u64 = 9;
/// 判定关键帧 / 序列头要看的 body 字节数（Enhanced-FLV 的 ModEx 头也够用）。
const CLASSIFY_BYTES: usize = 32;

struct Tag {
    tag_type: FlvTagType,
    timestamp_ms: u32,
    data_size: u32,
    class: ::flv::TagClass,
}

impl Tag {
    fn is_media(&self) -> bool {
        matches!(self.tag_type, FlvTagType::Audio | FlvTagType::Video)
    }

    fn is_keyframe(&self) -> bool {
        self.tag_type == FlvTagType::Video && self.class.keyframe_media
    }

    fn end(&self, offset: u64) -> u64 {
        offset + (TAG_HEADER_SIZE + PREV_TAG_SIZE_FIELD_SIZE) as u64 + self.data_size as u64
    }
}

/// 读 `offset` 处的 tag 头和 body 开头；tag 不完整（还在写 / 被截断）返回 `None`。
fn read_tag(reader: &mut BufReader<File>, offset: u64, file_len: u64) -> io::Result<Option<Tag>> {
    if offset + (TAG_HEADER_SIZE + PREV_TAG_SIZE_FIELD_SIZE) as u64 > file_len {
        return Ok(None);
    }
    let mut head = [0u8; TAG_HEADER_SIZE];
    reader.read_exact(&mut head)?;
    let header = parse_tag_header_bytes(head)?;
    if matches!(header.tag_type, FlvTagType::Unknown(_)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("FLV tag type {:?} at offset {offset}", header.tag_type),
        ));
    }
    let tag = Tag {
        tag_type: header.tag_type,
        timestamp_ms: header.timestamp_ms,
        data_size: header.data_size,
        class: Default::default(),
    };
    if tag.end(offset) > file_len {
        return Ok(None);
    }
    let peek = (header.data_size as usize).min(CLASSIFY_BYTES);
    let mut body = [0u8; CLASSIFY_BYTES];
    reader.read_exact(&mut body[..peek])?;
    let class = FlvTag::new(
        header.timestamp_ms,
        header.stream_id,
        header.tag_type,
        header.is_filtered,
        Bytes::copy_from_slice(&body[..peek]),
    )
    .classification();
    reader.seek_relative((header.data_size as usize - peek + PREV_TAG_SIZE_FIELD_SIZE) as i64)?;
    Ok(Some(Tag { class, ..tag }))
}

/// 第一个 tag 的偏移（文件头 + PreviousTagSize0）。
fn first_tag_offset(reader: &mut BufReader<File>, file_len: u64) -> io::Result<Option<u64>> {
    if file_len < FILE_HEADER_SIZE + PREV_TAG_SIZE_FIELD_SIZE as u64 {
        return Ok(None);
    }
    let mut head = [0u8; FILE_HEADER_SIZE as usize];
    read_at(reader, 0, &mut head)?;
    if &head[..3] != b"FLV" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not an FLV file",
        ));
    }
    let data_offset = u32::from_be_bytes(head[5..9].try_into().unwrap()) as u64;
    Ok(Some(data_offset + PREV_TAG_SIZE_FIELD_SIZE as u64))
}

/// 从 `index.scanned_upto` 扫到文件末尾最后一个完整的 tag。
pub(super) fn scan(
    reader: &mut BufReader<File>,
    file_len: u64,
    index: &mut KeyframeIndex,
) -> io::Result<()> {
    let mut offset = index.scanned_upto;
    if offset == 0 {
        let Some(first) = first_tag_offset(reader, file_len)? else {
            return Ok(());
        };
        offset = first;
        index.scanned_upto = first;
    }
    reader.seek(SeekFrom::Start(offset))?;
    while let Some(tag) = read_tag(reader, offset, file_len)? {
        if tag.is_media() && !tag.class.sequence_header {
            index.finalize_header(offset);
            if tag.is_keyframe() {
                index.push_keyframe(tag.timestamp_ms as i64, offset);
            }
            index.observe_media(tag.timestamp_ms as i64);
        }
        offset = tag.end(offset);
        index.scanned_upto = offset;
    }
    Ok(())
}

/// `offset` 处是不是一个完整的视频关键帧 tag。
pub(super) fn is_keyframe_at(reader: &mut BufReader<File>, offset: u64, file_len: u64) -> bool {
    if reader.seek(SeekFrom::Start(offset)).is_err() {
        return false;
    }
    matches!(read_tag(reader, offset, file_len), Ok(Some(tag)) if tag.is_keyframe())
}
