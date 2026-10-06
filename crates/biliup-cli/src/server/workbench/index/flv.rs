//! FLV：逐 tag 读 11 字节 tag 头和 body 开头几个字节判定关键帧，body 其余部分跳过
//!（标了关键帧的 H.264 / H.265 tag 再看一眼各 NALU 的类型）。
//!
//! 边录边建时同一套判定（[`classify`]）在写盘处对内存里的 body 就地做，索引任务只收结论。

use super::{KeyframeIndex, Source, read_at};
use ::flv::framing::{PREV_TAG_SIZE_FIELD_SIZE, TAG_HEADER_SIZE, parse_tag_header_bytes};
use ::flv::tag::FlvTagType;
use ::flv::{CodecKind, FlvTag, TagClass};
use biliup::downloader::index_tap::FlvTagKind;
use bytes::Bytes;
use std::io::{self, SeekFrom};

const FILE_HEADER_SIZE: u64 = 9;
/// 判定关键帧 / 序列头要看的 body 字节数（Enhanced-FLV 的 ModEx 头也够用）。
const CLASSIFY_BYTES: usize = 32;
/// 一个关键帧 tag 里最多看这么多个 NALU（SEI、AUD、参数集之后就该是 slice 了）。
const MAX_NALUS_PER_TAG: usize = 64;

struct Tag {
    timestamp_ms: u32,
    data_size: u32,
    kind: FlvTagKind,
}

/// 从 `offset` 起、body 长 `data_size` 的 tag 之后的偏移。
pub(super) fn tag_end(offset: u64, data_size: u32) -> u64 {
    offset + (TAG_HEADER_SIZE + PREV_TAG_SIZE_FIELD_SIZE) as u64 + data_size as u64
}

/// 写盘处就地判定一个 tag（`tag_type` 为 tag 头第一个字节，`body` 完整）。
/// 与扫盘时 [`read_tag`] 看同样的字节、得出同样的结论；`body` 只做引用计数切片，不复制。
pub fn classify(tag_type: u8, body: &Bytes) -> FlvTagKind {
    let filtered = tag_type & 0x20 != 0;
    let tag_type = FlvTagType::from(tag_type & 0x1F);
    let peek = body.slice(..body.len().min(CLASSIFY_BYTES));
    let class = FlvTag::new(0, 0, tag_type, filtered, peek).classification();
    let keyframe = match nalu_data_start(tag_type, &class, body) {
        Some(start) => random_access_nalu(start, body.len(), class.codec, |pos| {
            Ok(body[pos..pos + 5].try_into().unwrap())
        })
        .unwrap_or(false),
        None => tag_type == FlvTagType::Video && class.keyframe_media,
    };
    kind(tag_type, &class, keyframe)
}

fn kind(tag_type: FlvTagType, class: &TagClass, keyframe: bool) -> FlvTagKind {
    FlvTagKind {
        media: matches!(tag_type, FlvTagType::Audio | FlvTagType::Video),
        sequence_header: class.sequence_header,
        keyframe,
    }
}

/// 把 `offset` 处的一个 tag 记进索引（扫盘与边录边建共用）。
pub(super) fn record(index: &mut KeyframeIndex, offset: u64, timestamp_ms: u32, kind: FlvTagKind) {
    if kind.media && !kind.sequence_header {
        index.finalize_header(offset);
        if kind.keyframe {
            index.push_keyframe(timestamp_ms as i64, offset);
        }
        index.observe_media(timestamp_ms as i64);
    }
}

/// 读 `offset` 处的 tag 头和 body 开头；tag 不完整（还在写 / 被截断）返回 `None`。
fn read_tag(reader: &mut impl Source, offset: u64, file_len: u64) -> io::Result<Option<Tag>> {
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
    if tag_end(offset, header.data_size) > file_len {
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
    let mut consumed = peek;
    let keyframe = match nalu_data_start(header.tag_type, &class, &body[..peek]) {
        Some(start) => random_access_nalu(start, header.data_size as usize, class.codec, |pos| {
            reader.skip(pos as i64 - consumed as i64)?;
            let mut head = [0u8; 5];
            reader.read_exact(&mut head)?;
            consumed = pos + 5;
            Ok(head)
        })?,
        None => header.tag_type == FlvTagType::Video && class.keyframe_media,
    };
    reader
        .seek_relative((header.data_size as usize - consumed + PREV_TAG_SIZE_FIELD_SIZE) as i64)?;
    Ok(Some(Tag {
        timestamp_ms: header.timestamp_ms,
        data_size: header.data_size,
        kind: kind(header.tag_type, &class, keyframe),
    }))
}

/// 标了关键帧的 H.264 / H.265 视频 tag 里第一个 NALU 长度字段在 body 中的位置；其他 tag，
/// 或是 ModEx / Multitrack 这类不展开的封装时返回 `None`（沿用 tag 头的关键帧标记）。
fn nalu_data_start(tag_type: FlvTagType, class: &TagClass, body: &[u8]) -> Option<usize> {
    if tag_type != FlvTagType::Video
        || !class.keyframe_media
        || !matches!(class.codec, Some(CodecKind::Avc | CodecKind::Hevc))
    {
        return None;
    }
    let first = *body.first()?;
    if !class.enhanced {
        return Some(5);
    }
    match first & 0x0F {
        1 => Some(8),
        3 => Some(5),
        _ => None,
    }
}

/// tag 头标了关键帧，但有的源（虎牙）把非 IDR 的 I 帧也标成关键帧。它不是随机访问点：之后的帧
/// 可以参考它之前的画面，ffmpeg / ffprobe 也不把它当关键帧，从那里起流拷贝切出来的开头不保证干净。
/// 逐个看 NALU 类型：H.264 要有 IDR（5），H.265 要有 IRAP（16–23）。
/// 按 4 字节 NALU 长度走；长度对不上（不是 4 字节长度的流）时沿用 tag 头的标记。
/// `head_at(pos)` 取 body 里 `pos` 起的 5 个字节（NALU 长度 + NALU 头）。
fn random_access_nalu(
    start: usize,
    data_size: usize,
    codec: Option<CodecKind>,
    mut head_at: impl FnMut(usize) -> io::Result<[u8; 5]>,
) -> io::Result<bool> {
    let mut pos = start;
    for _ in 0..MAX_NALUS_PER_TAG {
        if pos + 5 > data_size {
            return Ok(false);
        }
        let head = head_at(pos)?;
        let len = u32::from_be_bytes(head[..4].try_into().unwrap()) as usize;
        if len == 0 || pos + 4 + len > data_size {
            return Ok(true);
        }
        let random_access = match codec {
            Some(CodecKind::Hevc) => (16..=23).contains(&((head[4] >> 1) & 0x3F)),
            _ => head[4] & 0x1F == 5,
        };
        if random_access {
            return Ok(true);
        }
        pos += 4 + len;
    }
    Ok(true)
}

/// 第一个 tag 的偏移（文件头 + PreviousTagSize0）。
fn first_tag_offset(reader: &mut impl Source, file_len: u64) -> io::Result<Option<u64>> {
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
    reader: &mut impl Source,
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
        record(index, offset, tag.timestamp_ms, tag.kind);
        offset = tag_end(offset, tag.data_size);
        index.scanned_upto = offset;
    }
    Ok(())
}

/// `offset` 处是不是一个完整的视频关键帧 tag。
pub(super) fn is_keyframe_at(reader: &mut impl Source, offset: u64, file_len: u64) -> bool {
    if reader.seek(SeekFrom::Start(offset)).is_err() {
        return false;
    }
    matches!(read_tag(reader, offset, file_len), Ok(Some(tag)) if tag.kind.keyframe)
}
