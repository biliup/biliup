//! FLV：逐 tag 读 11 字节 tag 头和 body 开头几个字节判定关键帧，body 其余部分跳过。

use super::{KeyframeIndex, Source, read_at};
use ::flv::FlvTag;
use ::flv::framing::{PREV_TAG_SIZE_FIELD_SIZE, TAG_HEADER_SIZE, parse_tag_header_bytes};
use ::flv::tag::FlvTagType;
use bytes::Bytes;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};

const FILE_HEADER_SIZE: u64 = 9;
/// 判定关键帧 / 序列头要看的 body 字节数（Enhanced-FLV 的 ModEx 头也够用）。
const CLASSIFY_BYTES: usize = 32;
/// onMetaData 超过这个大小就不读（mesio 按 3.5 h 预留的也远小于它）。
const MAX_SCRIPT_TAG: u32 = 16 * 1024 * 1024;

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

/// 已完成的 mesio FLV：用文件头 `onMetaData.keyframes` 预填索引，续扫只需从最后一个
/// 关键帧扫到文件末尾（顺带补上索引被截断时漏掉的尾部）。没有可用的元数据时什么都不做。
///
/// mesio 的索引对间隔不到 1.9 s 的关键帧只记第一个，这些关键帧不会出现在结果里。
pub(super) fn seed_from_metadata(
    reader: &mut BufReader<File>,
    file_len: u64,
    index: &mut KeyframeIndex,
) -> io::Result<()> {
    let Some(first) = first_tag_offset(reader, file_len)? else {
        return Ok(());
    };
    let mut head = [0u8; TAG_HEADER_SIZE];
    if first + TAG_HEADER_SIZE as u64 > file_len {
        return Ok(());
    }
    read_at(reader, first, &mut head)?;
    let header = parse_tag_header_bytes(head)?;
    if header.tag_type != FlvTagType::ScriptData
        || header.data_size > MAX_SCRIPT_TAG
        || first + TAG_HEADER_SIZE as u64 + header.data_size as u64 > file_len
    {
        return Ok(());
    }
    let mut body = vec![0u8; header.data_size as usize];
    reader.read_exact(&mut body)?;
    let script = FlvTag::new(0, 0, FlvTagType::ScriptData, false, Bytes::from(body));
    let Ok(script) = script.decode_script() else {
        return Ok(());
    };
    if script.name != "onMetaData" {
        return Ok(());
    }
    let Some(properties) = script.data.first().and_then(|v| v.as_object_properties()) else {
        return Ok(());
    };
    let Some(keyframes) = properties
        .iter()
        .find(|(k, _)| k == "keyframes")
        .and_then(|(_, v)| v.as_object_properties())
    else {
        return Ok(());
    };
    let numbers = |name: &str| -> Vec<f64> {
        keyframes
            .iter()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_number()).collect())
            .unwrap_or_default()
    };
    let times = numbers("times");
    let positions = numbers("filepositions");
    let pairs: Vec<(i64, u64)> = times
        .iter()
        .zip(&positions)
        .map(|(t, p)| ((t * 1000.0).round() as i64, *p as u64))
        .take_while(|(_, p)| *p > first && *p < file_len)
        .collect();
    let Some(&(_, first_keyframe)) = pairs.first() else {
        return Ok(());
    };
    if !is_keyframe_at(reader, first_keyframe, file_len) {
        return Ok(());
    }

    // 头区：从第一个 tag 扫到第一个媒体 tag
    let mut offset = first;
    reader.seek(SeekFrom::Start(offset))?;
    while let Some(tag) = read_tag(reader, offset, file_len)? {
        if tag.is_media() && !tag.class.sequence_header {
            break;
        }
        offset = tag.end(offset);
    }
    index.finalize_header(offset);
    index.source = Source::Metadata;
    for (raw, position) in pairs {
        index.push_keyframe(raw, position);
    }
    super::rewind_to_last_keyframe(index);
    Ok(())
}

/// `offset` 处是不是一个完整的视频关键帧 tag。
pub(super) fn is_keyframe_at(reader: &mut BufReader<File>, offset: u64, file_len: u64) -> bool {
    if reader.seek(SeekFrom::Start(offset)).is_err() {
        return false;
    }
    matches!(read_tag(reader, offset, file_len), Ok(Some(tag)) if tag.is_keyframe())
}
