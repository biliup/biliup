// source: https://github.com/rust-av/flavors/blob/master/src/parser.rs
use nom::bits::bits;
use nom::bits::streaming::take;
use nom::bytes::streaming::tag;
use nom::combinator::{flat_map, map, map_res};
use nom::error::{Error, ErrorKind};
use nom::multi::{length_data, many_m_n, many0};
use nom::number::streaming::{be_f64, be_i16, be_i24, be_u8, be_u16, be_u24, be_u32};
use nom::sequence::{pair, terminated};
use nom::{Err, IResult, Needed, Parser};
use serde::Serialize;
use std::str::from_utf8;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Header {
    pub version: u8,
    pub audio: bool,
    pub video: bool,
    pub offset: u32,
}

pub fn header(input: &[u8]) -> IResult<&[u8], Header> {
    map(
        (tag("FLV"), be_u8, be_u8, be_u32),
        |(_, version, flags, offset)| Header {
            version,
            audio: flags & 4 == 4,
            video: flags & 1 == 1,
            offset,
        },
    )
    .parse(input)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TagType {
    Audio = 8,
    Video = 9,
    Script = 18,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TagHeader {
    pub tag_type: TagType,
    pub data_size: u32,
    pub timestamp: u32,
    pub stream_id: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TagData<'a> {
    Audio(AudioData<'a>),
    Video(VideoData<'a>),
    Script,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Tag<'a> {
    pub header: TagHeader,
    pub data: TagData<'a>,
}

fn tag_type(input: &[u8]) -> IResult<&[u8], TagType> {
    map_res(be_u8, |tag_type| {
        Ok(match tag_type {
            8 => TagType::Audio,
            9 => TagType::Video,
            18 => TagType::Script,
            _ => {
                return Err(Err::Error::<nom::error::Error<&[u8]>>(Error::new(
                    input,
                    ErrorKind::Alt,
                )));
            }
        })
    })
    .parse(input)
}

pub fn tag_header(input: &[u8]) -> IResult<&[u8], TagHeader> {
    map(
        (tag_type, be_u24, be_u24, be_u8, be_u24),
        |(tag_type, data_size, timestamp, timestamp_extended, stream_id)| TagHeader {
            tag_type,
            data_size,
            timestamp: (u32::from(timestamp_extended) << 24) + timestamp,
            stream_id,
        },
    )
    .parse(input)
}

pub fn complete_tag(input: &[u8]) -> IResult<&[u8], Tag<'_>> {
    flat_map(pair(tag_type, be_u24), |(tag_type, data_size)| {
        map(
            (
                be_u24,
                be_u8,
                be_u24,
                tag_data(tag_type, data_size as usize),
            ),
            move |(timestamp, timestamp_extended, stream_id, data)| Tag {
                header: TagHeader {
                    tag_type,
                    data_size,
                    timestamp: (u32::from(timestamp_extended) << 24) + timestamp,
                    stream_id,
                },
                data,
            },
        )
    })
    .parse(input)
}

pub fn tag_data(tag_type: TagType, size: usize) -> impl Fn(&[u8]) -> IResult<&[u8], TagData> {
    move |input| match tag_type {
        TagType::Video => map(|i| video_data(i, size), TagData::Video).parse(input),
        TagType::Audio => map(|i| audio_data(i, size), TagData::Audio).parse(input),
        TagType::Script => Ok((input, TagData::Script)),
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SoundFormat {
    PCM_NE, // native endianness...
    ADPCM,
    MP3,
    PCM_LE,
    NELLYMOSER_16KHZ_MONO,
    NELLYMOSER_8KHZ_MONO,
    NELLYMOSER,
    PCM_ALAW,
    PCM_ULAW,
    AAC,
    SPEEX,
    MP3_8KHZ,
    DEVICE_SPECIFIC,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SoundRate {
    _5_5KHZ,
    _11KHZ,
    _22KHZ,
    _44KHZ,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SoundSize {
    Snd8bit,
    Snd16bit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SoundType {
    SndMono,
    SndStereo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AACPacketType {
    SequenceHeader,
    Raw,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AACAudioPacketHeader {
    pub packet_type: AACPacketType,
}

pub fn aac_audio_packet_header(input: &[u8]) -> IResult<&[u8], AACAudioPacketHeader> {
    map_res(be_u8, |packet_type| {
        Ok(AACAudioPacketHeader {
            packet_type: match packet_type {
                0 => AACPacketType::SequenceHeader,
                1 => AACPacketType::Raw,
                _ => {
                    return Err(Err::<nom::error::Error<&[u8]>>::Error(Error::new(
                        input,
                        ErrorKind::Alt,
                    )));
                }
            },
        })
    })
    .parse(input)
}

#[derive(Debug, PartialEq, Eq)]
pub struct AACAudioPacket<'a> {
    pub packet_type: AACPacketType,
    pub aac_data: &'a [u8],
}

pub fn aac_audio_packet(input: &[u8], size: usize) -> IResult<&[u8], AACAudioPacket<'_>> {
    if input.len() < size {
        return Err(Err::Incomplete(Needed::new(size)));
    }

    if size < 1 {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    be_u8(input).and_then(|(_, packet_type)| {
        Ok((
            &input[size..],
            AACAudioPacket {
                packet_type: match packet_type {
                    0 => AACPacketType::SequenceHeader,
                    1 => AACPacketType::Raw,
                    _ => {
                        return Err(Err::Error::<nom::error::Error<&[u8]>>(Error::new(
                            input,
                            ErrorKind::Alt,
                        )));
                    }
                },
                aac_data: &input[1..size],
            },
        ))
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioData<'a> {
    pub sound_format: SoundFormat,
    pub sound_rate: SoundRate,
    pub sound_size: SoundSize,
    pub sound_type: SoundType,
    pub sound_data: &'a [u8],
}

pub fn audio_data(input: &[u8], size: usize) -> IResult<&[u8], AudioData<'_>> {
    if input.len() < size {
        return Err(Err::Incomplete(Needed::new(size)));
    }

    if size < 1 {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let take_bits = (take(4usize), take(2usize), take(1usize), take(1usize));
    bits::<_, _, Error<_>, _, _>(take_bits)(input).and_then(
        |(_, (sformat, srate, ssize, stype))| {
            let sformat = match sformat {
                0 => SoundFormat::PCM_NE,
                1 => SoundFormat::ADPCM,
                2 => SoundFormat::MP3,
                3 => SoundFormat::PCM_LE,
                4 => SoundFormat::NELLYMOSER_16KHZ_MONO,
                5 => SoundFormat::NELLYMOSER_8KHZ_MONO,
                6 => SoundFormat::NELLYMOSER,
                7 => SoundFormat::PCM_ALAW,
                8 => SoundFormat::PCM_ULAW,
                10 => SoundFormat::AAC,
                11 => SoundFormat::SPEEX,
                14 => SoundFormat::MP3_8KHZ,
                15 => SoundFormat::DEVICE_SPECIFIC,
                _ => {
                    return Err(Err::Error::<nom::error::Error<&[u8]>>(Error::new(
                        input,
                        ErrorKind::Alt,
                    )));
                }
            };
            let srate = match srate {
                0 => SoundRate::_5_5KHZ,
                1 => SoundRate::_11KHZ,
                2 => SoundRate::_22KHZ,
                3 => SoundRate::_44KHZ,
                _ => {
                    return Err(Err::<nom::error::Error<&[u8]>>::Error(Error::new(
                        input,
                        ErrorKind::Alt,
                    )));
                }
            };
            let ssize = match ssize {
                0 => SoundSize::Snd8bit,
                1 => SoundSize::Snd16bit,
                _ => return Err(Err::Error(Error::new(input, ErrorKind::Alt))),
            };
            let stype = match stype {
                0 => SoundType::SndMono,
                1 => SoundType::SndStereo,
                _ => return Err(Err::Error(Error::new(input, ErrorKind::Alt))),
            };

            Ok((
                &input[size..],
                AudioData {
                    sound_format: sformat,
                    sound_rate: srate,
                    sound_size: ssize,
                    sound_type: stype,
                    sound_data: &input[1..size],
                },
            ))
        },
    )
}

#[derive(Debug, PartialEq, Eq)]
pub struct AudioDataHeader {
    pub sound_format: SoundFormat,
    pub sound_rate: SoundRate,
    pub sound_size: SoundSize,
    pub sound_type: SoundType,
}

pub fn audio_data_header(input: &[u8]) -> IResult<&[u8], AudioDataHeader> {
    if input.is_empty() {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let take_bits = (take(4usize), take(2usize), take(1usize), take(1usize));
    map_res(
        bits::<_, _, Error<_>, _, _>(take_bits),
        |(sformat, srate, ssize, stype)| {
            let sformat = match sformat {
                0 => SoundFormat::PCM_NE,
                1 => SoundFormat::ADPCM,
                2 => SoundFormat::MP3,
                3 => SoundFormat::PCM_LE,
                4 => SoundFormat::NELLYMOSER_16KHZ_MONO,
                5 => SoundFormat::NELLYMOSER_8KHZ_MONO,
                6 => SoundFormat::NELLYMOSER,
                7 => SoundFormat::PCM_ALAW,
                8 => SoundFormat::PCM_ULAW,
                10 => SoundFormat::AAC,
                11 => SoundFormat::SPEEX,
                14 => SoundFormat::MP3_8KHZ,
                15 => SoundFormat::DEVICE_SPECIFIC,
                _ => {
                    return Err(Err::<nom::error::Error<&[u8]>>::Error(Error::new(
                        input,
                        ErrorKind::Alt,
                    )));
                }
            };
            let srate = match srate {
                0 => SoundRate::_5_5KHZ,
                1 => SoundRate::_11KHZ,
                2 => SoundRate::_22KHZ,
                3 => SoundRate::_44KHZ,
                _ => return Err(Err::Error(Error::new(input, ErrorKind::Alt))),
            };
            let ssize = match ssize {
                0 => SoundSize::Snd8bit,
                1 => SoundSize::Snd16bit,
                _ => return Err(Err::Error(Error::new(input, ErrorKind::Alt))),
            };
            let stype = match stype {
                0 => SoundType::SndMono,
                1 => SoundType::SndStereo,
                _ => return Err(Err::Error(Error::new(input, ErrorKind::Alt))),
            };

            Ok(AudioDataHeader {
                sound_format: sformat,
                sound_rate: srate,
                sound_size: ssize,
                sound_type: stype,
            })
        },
    )
    .parse(input)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FrameType {
    /// Enhanced-FLV（E-RTMP）的 0：tag 头没标帧类型；老式 FLV 里 0 不合法。
    Unknown,
    Key,
    Inter,
    DisposableInter,
    Generated,
    Command,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CodecId {
    JPEG,
    SORENSON_H263,
    SCREEN,
    VP6,
    VP6A,
    SCREEN2,
    H264,
    // Not in FLV standard
    H263,
    MPEG4Part2, // MPEG-4 Part 2
    /// 国内平台在 FLV 里给 HEVC 用的非标准 codec id（12），载荷与 AVC 同构。
    HEVC,
    /// 国内平台在 FLV 里给 AV1 用的非标准 codec id（13），载荷与 AVC 同构。
    AV1,
}

/// 视频载荷的 packet type。
///
/// 老式 FLV 的视频载荷按 AVC 的方式分帧（0 序列头 / 1 NALU / 2 序列结束），国内平台的 HEVC、
/// AV1 沿用同一编号；Enhanced-FLV（E-RTMP）另有 3（CodedFramesX，没有 composition time）
/// 与 4/5（metadata / MPEG-2 TS 序列头，不承载画面）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AVCPacketType {
    SequenceHeader,
    NALU,
    EndOfSequence,
    /// Enhanced-FLV 的 packet type 3：后面没有 3 字节 composition time。
    CodedFramesX,
    /// Enhanced-FLV 的 packet type 4：元数据，不是媒体字节。
    Metadata,
    /// Enhanced-FLV 的 packet type 5：后面的载荷是一段 MPEG-2 TS。
    Mpeg2TsSequenceStart,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AVCVideoPacketHeader {
    pub packet_type: AVCPacketType,
    pub composition_time: i32,
}

fn packet_type(input: &[u8]) -> IResult<&[u8], AVCPacketType> {
    map_res(be_u8, |packet_type| {
        Ok(match packet_type {
            0 => AVCPacketType::SequenceHeader,
            1 => AVCPacketType::NALU,
            2 => AVCPacketType::EndOfSequence,
            _ => {
                return Err(Err::<nom::error::Error<&[u8]>>::Error(Error::new(
                    input,
                    ErrorKind::Alt,
                )));
            }
        })
    })
    .parse(input)
}

pub fn avc_video_packet_header(input: &[u8]) -> IResult<&[u8], AVCVideoPacketHeader> {
    map(
        pair(packet_type, be_i24),
        |(packet_type, composition_time)| AVCVideoPacketHeader {
            packet_type,
            composition_time,
        },
    )
    .parse(input)
}

#[derive(Debug, PartialEq, Eq)]
pub struct AVCVideoPacket<'a> {
    pub packet_type: AVCPacketType,
    pub composition_time: i32,
    pub avc_data: &'a [u8],
}

pub fn avc_video_packet(input: &[u8], size: usize) -> IResult<&[u8], AVCVideoPacket<'_>> {
    if input.len() < size {
        return Err(Err::Incomplete(Needed::new(size)));
    }

    if size < 4 {
        return Err(Err::Incomplete(Needed::new(4)));
    }
    pair(packet_type, be_i24)
        .parse(input)
        .map(|(_, (packet_type, composition_time))| {
            (
                &input[size..],
                AVCVideoPacket {
                    packet_type,
                    composition_time,
                    avc_data: &input[4..size],
                },
            )
        })
}

/// 视频 tag 头的形状：老式 FLV 是 `frame_type(4) | codec id(4)`，Enhanced-FLV（E-RTMP）
/// 在最上面加一位 `IsExHeader`，帧类型只剩 3 位，后面跟四字节 FourCC。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoFraming {
    Legacy,
    Enhanced,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoData<'a> {
    pub frame_type: FrameType,
    pub codec_id: CodecId,
    /// tag 头（老式 1 字节，Enhanced-FLV 5 字节）之后的字节。老式与 Enhanced 的 CodedFrames
    /// 里先出现 3 字节 composition time，Enhanced 的 CodedFramesX 与序列头没有这一段。
    pub video_data: &'a [u8],
    pub framing: VideoFraming,
    /// Enhanced-FLV 的 FourCC；老式 FLV tag 为 `None`。
    pub fourcc: Option<[u8; 4]>,
    /// 视频载荷的 packet type；没有分帧字段的编码（JPEG / VP6 等）为 `None`。
    pub packet_type: Option<AVCPacketType>,
    /// composition time（毫秒）；载荷里没有这个字段时为 0。
    pub composition_time: i32,
}

impl VideoData<'_> {
    /// Enhanced-FLV 的 FourCC 对应的名字（只认能录的这几种编码）。
    pub fn fourcc_name(&self) -> Option<&'static str> {
        match self.fourcc? {
            [b'h', b'v', b'c', b'1'] => Some("hvc1"),
            [b'h', b'e', b'v', b'1'] => Some("hev1"),
            [b'a', b'v', b'0', b'1'] => Some("av01"),
            [b'a', b'v', b'c', b'1'] => Some("avc1"),
            _ => None,
        }
    }

    /// 这个 tag 是不是视频序列头（H.264 / HEVC / AV1 的编码配置记录）。
    pub fn is_sequence_header(&self) -> bool {
        self.packet_type == Some(AVCPacketType::SequenceHeader)
    }

    /// 能不能从这个 tag 开始解码。
    pub fn is_keyframe(&self) -> bool {
        self.frame_type == FrameType::Key
    }
}

fn legacy_frame_type(value: u8) -> Option<FrameType> {
    Some(match value {
        1 => FrameType::Key,
        2 => FrameType::Inter,
        3 => FrameType::DisposableInter,
        4 => FrameType::Generated,
        5 => FrameType::Command,
        _ => return None,
    })
}

/// Enhanced-FLV 的 frame type 是 3 位：0 是合法取值（tag 头没标帧类型），6/7 未定义。
fn enhanced_frame_type(value: u8) -> Option<FrameType> {
    match value {
        0 => Some(FrameType::Unknown),
        value => legacy_frame_type(value),
    }
}

fn legacy_codec_id(value: u8) -> Option<CodecId> {
    Some(match value {
        1 => CodecId::JPEG,
        2 => CodecId::SORENSON_H263,
        3 => CodecId::SCREEN,
        4 => CodecId::VP6,
        5 => CodecId::VP6A,
        6 => CodecId::SCREEN2,
        7 => CodecId::H264,
        8 => CodecId::H263,
        9 => CodecId::MPEG4Part2,
        // 国内平台给 HEVC / AV1 用的非标准 codec id
        12 => CodecId::HEVC,
        13 => CodecId::AV1,
        _ => return None,
    })
}

fn codec_id_of_fourcc(fourcc: &[u8]) -> Option<CodecId> {
    Some(match fourcc {
        b"avc1" => CodecId::H264,
        b"hvc1" | b"hev1" => CodecId::HEVC,
        b"av01" => CodecId::AV1,
        _ => return None,
    })
}

/// 老式 FLV 里 HEVC / AV1 的载荷与 AVC 同构，用同一套分帧字段。
fn avc_family(codec_id: CodecId) -> bool {
    matches!(codec_id, CodecId::H264 | CodecId::HEVC | CodecId::AV1)
}

/// Enhanced-FLV（E-RTMP）的 packet type：0/1/2 与老式一致，另有 3、4、5。
fn enhanced_packet_type(value: u8) -> Option<AVCPacketType> {
    Some(match value {
        0 => AVCPacketType::SequenceHeader,
        1 => AVCPacketType::NALU,
        2 => AVCPacketType::EndOfSequence,
        3 => AVCPacketType::CodedFramesX,
        4 => AVCPacketType::Metadata,
        5 => AVCPacketType::Mpeg2TsSequenceStart,
        _ => return None,
    })
}

pub fn video_data(input: &[u8], size: usize) -> IResult<&[u8], VideoData<'_>> {
    if input.len() < size {
        return Err(Err::Incomplete(Needed::new(size)));
    }

    if size < 1 {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    // IsExHeader：Enhanced-FLV 用四字节 FourCC 标编码，帧类型只剩 3 位。
    if input[0] & 0x80 != 0 {
        return enhanced_video_data(input, size);
    }

    let take_bits = pair(take(4usize), take(4usize));
    bits::<_, _, Error<_>, _, _>(take_bits)(input).and_then(|(_, (frame_type, codec_id))| {
        let frame_type = match legacy_frame_type(frame_type) {
            Some(frame_type) => frame_type,
            None => return Err(Err::Error(Error::new(input, ErrorKind::Alt))),
        };
        let codec_id = match legacy_codec_id(codec_id) {
            Some(codec_id) => codec_id,
            None => return Err(Err::Error(Error::new(input, ErrorKind::Alt))),
        };
        let video_data = &input[1..size];
        let (packet_type, composition_time) = if avc_family(codec_id) {
            match avc_video_packet_header(video_data) {
                Ok((_, header)) => (Some(header.packet_type), header.composition_time),
                Err(e) => return Err(e),
            }
        } else {
            (None, 0)
        };

        Ok((
            &input[size..],
            VideoData {
                frame_type,
                codec_id,
                video_data,
                framing: VideoFraming::Legacy,
                fourcc: None,
                packet_type,
                composition_time,
            },
        ))
    })
}

/// Enhanced-FLV（E-RTMP）的视频 tag 头：`IsExHeader|frame_type(3)|packet_type(4)` + FourCC。
fn enhanced_video_data(input: &[u8], size: usize) -> IResult<&[u8], VideoData<'_>> {
    if size < 5 {
        return Err(Err::Incomplete(Needed::new(5)));
    }
    let first = input[0];
    let fourcc: [u8; 4] = input[1..5].try_into().unwrap();
    let (Some(frame_type), Some(codec_id), Some(packet_type)) = (
        enhanced_frame_type((first >> 4) & 0x07),
        codec_id_of_fourcc(&fourcc),
        enhanced_packet_type(first & 0x0f),
    ) else {
        return Err(Err::Error(Error::new(input, ErrorKind::Alt)));
    };
    // CodedFrames 后面是 3 字节 composition time；CodedFramesX 与序列头没有这一段。
    let (payload_start, composition_time) = match packet_type {
        AVCPacketType::NALU => {
            if size < 8 {
                return Err(Err::Incomplete(Needed::new(8)));
            }
            match be_i24(&input[5..size]) {
                Ok((_, composition_time)) => (8, composition_time),
                Err(e) => return Err(e),
            }
        }
        _ => (5, 0),
    };

    Ok((
        &input[size..],
        VideoData {
            frame_type,
            codec_id,
            video_data: &input[payload_start..size],
            framing: VideoFraming::Enhanced,
            fourcc: Some(fourcc),
            packet_type: Some(packet_type),
            composition_time,
        },
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoDataHeader {
    pub frame_type: FrameType,
    pub codec_id: CodecId,
}

pub fn video_data_header(input: &[u8]) -> IResult<&[u8], VideoDataHeader> {
    if input.is_empty() {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    // Enhanced-FLV：帧类型 3 位，编码看 FourCC。
    if input[0] & 0x80 != 0 {
        if input.len() < 5 {
            return Err(Err::Incomplete(Needed::new(5)));
        }
        let first = input[0];
        let fourcc: [u8; 4] = input[1..5].try_into().unwrap();
        let (Some(frame_type), Some(codec_id)) = (
            enhanced_frame_type((first >> 4) & 0x07),
            codec_id_of_fourcc(&fourcc),
        ) else {
            return Err(Err::<nom::error::Error<&[u8]>>::Error(Error::new(
                input,
                ErrorKind::Alt,
            )));
        };
        return Ok((
            &input[5..],
            VideoDataHeader {
                frame_type,
                codec_id,
            },
        ));
    }

    let take_bits = pair(take(4usize), take(4usize));
    map_res(
        bits::<_, _, Error<_>, _, _>(take_bits),
        |(frame_type, codec_id)| {
            let (Some(frame_type), Some(codec_id)) =
                (legacy_frame_type(frame_type), legacy_codec_id(codec_id))
            else {
                return Err(Err::<nom::error::Error<&[u8]>>::Error(Error::new(
                    input,
                    ErrorKind::Alt,
                )));
            };

            Ok(VideoDataHeader {
                frame_type,
                codec_id,
            })
        },
    )
    .parse(input)
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ScriptData<'a> {
    pub name: &'a str,
    pub arguments: ScriptDataValue<'a>,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum ScriptDataValue<'a> {
    Number(f64),
    Boolean(bool),
    String(&'a str),
    Object(Vec<ScriptDataObject<'a>>),
    MovieClip(&'a str),
    Null,
    Undefined,
    Reference(u16),
    ECMAArray(Vec<ScriptDataObject<'a>>),
    StrictArray(Vec<ScriptDataValue<'a>>),
    Date(ScriptDataDate),
    LongString(&'a str),
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ScriptDataObject<'a> {
    pub name: &'a str,
    pub data: ScriptDataValue<'a>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScriptDataDate {
    pub date_time: f64,
    pub local_date_time_offset: i16, // SI16
}

#[allow(non_upper_case_globals)]
static script_data_name_tag: &[u8] = &[2];

pub fn script_data(input: &[u8]) -> IResult<&[u8], ScriptData<'_>> {
    // Must start with a string, i.e. 2
    map(
        (
            tag(script_data_name_tag),
            script_data_string,
            script_data_value,
        ),
        |(_, name, arguments)| ScriptData { name, arguments },
    )
    .parse(input)
}

/// AMF0 容器（Object / ECMA 数组 / 严格数组）的最大嵌套层数。真实的 onMetaData 只有两三层；
/// 解析是递归的，不设上限时一个每层只占 3 字节的畸形脚本 tag 就能把栈打爆、abort 整个进程。
const MAX_SCRIPT_DATA_DEPTH: usize = 64;

pub fn script_data_value(input: &[u8]) -> IResult<&[u8], ScriptDataValue<'_>> {
    script_data_value_at(input, 0)
}

fn script_data_value_at(input: &[u8], depth: usize) -> IResult<&[u8], ScriptDataValue<'_>> {
    if depth > MAX_SCRIPT_DATA_DEPTH {
        return Err(Err::Failure(Error::new(input, ErrorKind::TooLarge)));
    }
    be_u8(input).and_then(|v| match v {
        (i, 0) => map(be_f64, ScriptDataValue::Number).parse(i),
        (i, 1) => map(be_u8, |n| ScriptDataValue::Boolean(n != 0)).parse(i),
        (i, 2) => map(script_data_string, ScriptDataValue::String).parse(i),
        (i, 3) => script_data_objects_at(i, depth + 1)
            .map(|(i, objects)| (i, ScriptDataValue::Object(objects))),
        (i, 4) => map(script_data_string, ScriptDataValue::MovieClip).parse(i),
        (i, 5) => Ok((i, ScriptDataValue::Null)), // to remove
        (i, 6) => Ok((i, ScriptDataValue::Undefined)), // to remove
        (i, 7) => map(be_u16, ScriptDataValue::Reference).parse(i),
        (i, 8) => script_data_ecma_array_at(i, depth + 1)
            .map(|(i, objects)| (i, ScriptDataValue::ECMAArray(objects))),
        (i, 10) => script_data_strict_array_at(i, depth + 1)
            .map(|(i, values)| (i, ScriptDataValue::StrictArray(values))),
        (i, 11) => map(script_data_date, ScriptDataValue::Date).parse(i),
        (i, 12) => map(script_data_long_string, ScriptDataValue::LongString).parse(i),
        _ => Err(Err::Error(Error::new(input, ErrorKind::Alt))),
    })
}

pub fn script_data_objects(input: &[u8]) -> IResult<&[u8], Vec<ScriptDataObject<'_>>> {
    script_data_objects_at(input, 0)
}

fn script_data_objects_at(input: &[u8], depth: usize) -> IResult<&[u8], Vec<ScriptDataObject<'_>>> {
    terminated(
        many0(|i| script_data_object_at(i, depth)),
        script_data_object_end,
    )
    .parse(input)
}

pub fn script_data_object(input: &[u8]) -> IResult<&[u8], ScriptDataObject<'_>> {
    script_data_object_at(input, 0)
}

fn script_data_object_at(input: &[u8], depth: usize) -> IResult<&[u8], ScriptDataObject<'_>> {
    map(
        pair(script_data_string, |i| script_data_value_at(i, depth)),
        |(name, data)| ScriptDataObject { name, data },
    )
    .parse(input)
}

#[allow(non_upper_case_globals)]
static script_data_object_end_terminator: &[u8] = &[0, 0, 9];

pub fn script_data_object_end(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag(script_data_object_end_terminator)(input)
}

pub fn script_data_string(input: &[u8]) -> IResult<&[u8], &str> {
    map_res(length_data(be_u16), from_utf8).parse(input)
}

pub fn script_data_long_string(input: &[u8]) -> IResult<&[u8], &str> {
    map_res(length_data(be_u32), from_utf8).parse(input)
}

pub fn script_data_date(input: &[u8]) -> IResult<&[u8], ScriptDataDate> {
    map(
        pair(be_f64, be_i16),
        |(date_time, local_date_time_offset)| ScriptDataDate {
            date_time,
            local_date_time_offset,
        },
    )
    .parse(input)
}

pub fn script_data_ecma_array(input: &[u8]) -> IResult<&[u8], Vec<ScriptDataObject<'_>>> {
    script_data_ecma_array_at(input, 0)
}

fn script_data_ecma_array_at(
    input: &[u8],
    depth: usize,
) -> IResult<&[u8], Vec<ScriptDataObject<'_>>> {
    let (input, _) = be_u32(input)?;
    script_data_objects_at(input, depth)
}

pub fn script_data_strict_array(input: &[u8]) -> IResult<&[u8], Vec<ScriptDataValue<'_>>> {
    script_data_strict_array_at(input, 0)
}

fn script_data_strict_array_at(
    input: &[u8],
    depth: usize,
) -> IResult<&[u8], Vec<ScriptDataValue<'_>>> {
    let (input, count) = be_u32(input)?;
    many_m_n(1, count as usize, |i| script_data_value_at(i, depth)).parse(input)
}

#[cfg(test)]
mod script_data_tests {
    use super::{ScriptDataValue, script_data};

    /// 嵌套极深的 AMF0（每层只要 3 字节：空键名 + Object 标记）不能把解析栈打爆：
    /// 栈溢出会直接 abort 整个进程（catch_unwind 也接不住），解析器必须返回错误。
    #[test]
    fn deeply_nested_script_data_is_an_error_not_a_stack_overflow() {
        let levels = 200_000;
        let mut tag = vec![0x02, 0x00, 0x0A];
        tag.extend_from_slice(b"onMetaData");
        tag.push(0x03);
        for _ in 0..levels {
            tag.extend_from_slice(&[0x00, 0x00, 0x03]);
        }
        for _ in 0..=levels {
            tag.extend_from_slice(&[0x00, 0x00, 0x09]);
        }
        // tokio 工作线程默认 2 MiB 栈，与录制时解析所在的线程一致
        let parsed = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || script_data(&tag).is_err())
            .unwrap()
            .join()
            .unwrap();
        assert!(parsed, "nesting this deep must be rejected");
    }

    /// 真实的 onMetaData 只有两三层嵌套（ECMA 数组 → keyframes 对象 → 数字严格数组），照常解析。
    #[test]
    fn ordinary_nested_script_data_still_parses() {
        let mut tag = vec![0x02, 0x00, 0x0A];
        tag.extend_from_slice(b"onMetaData");
        // ECMA array(1 项): "keyframes" → Object { "times": StrictArray[Number 1.5] }
        tag.extend_from_slice(&[0x08, 0x00, 0x00, 0x00, 0x01]);
        tag.extend_from_slice(&[0x00, 0x09]);
        tag.extend_from_slice(b"keyframes");
        tag.push(0x03);
        tag.extend_from_slice(&[0x00, 0x05]);
        tag.extend_from_slice(b"times");
        tag.extend_from_slice(&[0x0a, 0x00, 0x00, 0x00, 0x01, 0x00]);
        tag.extend_from_slice(&1.5f64.to_be_bytes());
        tag.extend_from_slice(&[0x00, 0x00, 0x09]); // keyframes 对象结束
        tag.extend_from_slice(&[0x00, 0x00, 0x09]); // ECMA 数组结束
        let (rest, parsed) = script_data(&tag).unwrap();
        assert!(rest.is_empty());
        assert_eq!(parsed.name, "onMetaData");
        let ScriptDataValue::ECMAArray(items) = parsed.arguments else {
            panic!("expected an ECMA array: {:?}", parsed.arguments);
        };
        assert_eq!(items[0].name, "keyframes");
        let ScriptDataValue::Object(keyframes) = &items[0].data else {
            panic!("expected an object");
        };
        assert_eq!(
            keyframes[0].data,
            ScriptDataValue::StrictArray(vec![ScriptDataValue::Number(1.5)])
        );
    }
}

#[cfg(test)]
mod video_data_tests {
    use super::*;

    fn assert_incomplete<T: std::fmt::Debug>(result: IResult<&[u8], T>) {
        assert!(matches!(result, Err(Err::Incomplete(_))), "{result:?}");
    }

    /// 老式 FLV 的 HEVC（codec id 12）与 AV1（13）：载荷与 AVC 同构，帧类型与 composition time
    /// 照常解析。
    #[test]
    fn legacy_hevc_and_av1_headers_are_parsed_like_avc() {
        for (codec_id, expected) in [(12u8, CodecId::HEVC), (13u8, CodecId::AV1)] {
            // 关键帧 + 序列头：0x1c（codec 12）/ 0x1d（codec 13）
            let body = [0x10 | codec_id, 0x00, 0x00, 0x00, 0x00, 0x01, 0x64];
            let (rest, parsed) = video_data(&body, body.len()).unwrap();
            assert!(rest.is_empty());
            assert_eq!(parsed.frame_type, FrameType::Key);
            assert!(parsed.is_keyframe());
            assert_eq!(parsed.codec_id, expected);
            assert_eq!(parsed.framing, VideoFraming::Legacy);
            assert_eq!(parsed.fourcc, None);
            assert_eq!(parsed.packet_type, Some(AVCPacketType::SequenceHeader));
            assert!(parsed.is_sequence_header());
            assert_eq!(parsed.composition_time, 0);
            assert_eq!(parsed.video_data, &body[1..]);

            // 普通帧 + NALU，composition time 按 i24 解析
            let body = [0x20 | codec_id, 0x01, 0x00, 0x00, 0x1e, 0xaa];
            let (_, parsed) = video_data(&body, body.len()).unwrap();
            assert_eq!(parsed.frame_type, FrameType::Inter);
            assert!(!parsed.is_sequence_header());
            assert_eq!(parsed.packet_type, Some(AVCPacketType::NALU));
            assert_eq!(parsed.composition_time, 30);
        }
    }

    /// Enhanced-FLV：帧类型 3 位、编码看 FourCC，packet type 决定 composition time 在不在。
    #[test]
    fn enhanced_headers_carry_the_fourcc_and_packet_type() {
        // 序列头：0x90 = IsExHeader | Key | SequenceStart，载荷是编码配置记录
        for (fourcc, expected) in [
            (b"hvc1", CodecId::HEVC),
            (b"hev1", CodecId::HEVC),
            (b"av01", CodecId::AV1),
            (b"avc1", CodecId::H264),
        ] {
            let mut body = vec![0x90];
            body.extend_from_slice(fourcc);
            body.extend_from_slice(&[0x01, 0x64]);
            let (rest, parsed) = video_data(&body, body.len()).unwrap();
            assert!(rest.is_empty());
            assert_eq!(parsed.codec_id, expected);
            assert_eq!(parsed.framing, VideoFraming::Enhanced);
            assert_eq!(parsed.fourcc, Some(*fourcc));
            assert_eq!(
                parsed.fourcc_name(),
                Some(std::str::from_utf8(fourcc).unwrap())
            );
            assert_eq!(parsed.packet_type, Some(AVCPacketType::SequenceHeader));
            assert!(parsed.is_sequence_header());
            assert_eq!(parsed.video_data, [0x01, 0x64]);
        }

        // CodedFrames：0x91 = Key | packet type 1，后面 3 字节 composition time
        let body = [0x91, b'h', b'v', b'c', b'1', 0x00, 0x00, 0x28, 0xaa, 0xbb];
        let (_, parsed) = video_data(&body, body.len()).unwrap();
        assert_eq!(parsed.frame_type, FrameType::Key);
        assert_eq!(parsed.codec_id, CodecId::HEVC);
        assert_eq!(parsed.fourcc_name(), Some("hvc1"));
        assert_eq!(parsed.packet_type, Some(AVCPacketType::NALU));
        assert_eq!(parsed.composition_time, 40);
        assert_eq!(parsed.video_data, [0xaa, 0xbb]);

        // CodedFramesX：0x93，没有 composition time
        let body = [0x93, b'a', b'v', b'0', b'1', 0xaa];
        let (_, parsed) = video_data(&body, body.len()).unwrap();
        assert_eq!(parsed.packet_type, Some(AVCPacketType::CodedFramesX));
        assert_eq!(parsed.composition_time, 0);
        assert_eq!(parsed.video_data, [0xaa]);

        // 帧类型 0 在 Enhanced-FLV 里合法：tag 头没标帧类型
        let body = [0x80, b'h', b'v', b'c', b'1', 0x01];
        let (_, parsed) = video_data(&body, body.len()).unwrap();
        assert_eq!(parsed.frame_type, FrameType::Unknown);
        assert!(!parsed.is_keyframe());

        // metadata / MPEG-2 TS 序列头不承载画面，但仍然如实解析
        for (first, expected) in [
            (0x94u8, AVCPacketType::Metadata),
            (0x95, AVCPacketType::Mpeg2TsSequenceStart),
        ] {
            let body = [first, b'h', b'v', b'c', b'1', 0xaa];
            let (_, parsed) = video_data(&body, body.len()).unwrap();
            assert_eq!(parsed.packet_type, Some(expected));
        }
    }

    /// 认不出的编码与未定义的 packet type 都是解析错误，不能被当成「个别残缺的 tag」跳过。
    #[test]
    fn unknown_enhanced_codecs_are_errors() {
        for body in [
            vec![0x91, b'v', b'p', b'0', b'9', 0, 0, 0],
            vec![0x91, b'x', b'x', b'x', b'x', 0, 0, 0],
            // packet type 6 未定义
            vec![0x96, b'h', b'v', b'c', b'1', 0, 0, 0],
            // Enhanced 的帧类型 6/7 未定义（0xe1 / 0xf1 = IsExHeader | 6|7 | CodedFrames）
            vec![0xe1, b'h', b'v', b'c', b'1', 0, 0, 0],
            vec![0xf1, b'h', b'v', b'c', b'1', 0, 0, 0],
        ] {
            assert!(video_data(&body, body.len()).is_err(), "{body:?}");
        }
        // 老式 FLV 里没定义的 codec id 照旧是错误
        assert!(video_data(&[0x1a, 0x00, 0x00, 0x00, 0x00], 5).is_err());
        assert!(video_data(&[0x1f, 0x00, 0x00, 0x00, 0x00], 5).is_err());
    }

    /// 头没读全时返回 Incomplete，而不是把短 body 当成一个完整的 tag。
    #[test]
    fn short_bodies_are_incomplete() {
        assert_incomplete(video_data(&[], 0));
        assert_incomplete(video_data(&[0x91, b'h'], 2));
        assert_incomplete(video_data(&[0x91, b'h', b'v', b'c', b'1'], 7));
        // CodedFrames 少了 composition time
        assert_incomplete(video_data(&[0x91, b'h', b'v', b'c', b'1', 0x00], 6));
    }

    /// `video_data_header` 认同一套帧类型与编码，两种 tag 头形状都要。
    #[test]
    fn video_data_header_understands_both_framings() {
        let (rest, header) = video_data_header(&[0x1c, 0x00, 0x01, 0x02, 0x03]).unwrap();
        assert_eq!(rest, [0x00, 0x01, 0x02, 0x03]);
        assert_eq!(header.frame_type, FrameType::Key);
        assert_eq!(header.codec_id, CodecId::HEVC);

        let (rest, header) = video_data_header(&[0x91, b'a', b'v', b'0', b'1', 0xaa]).unwrap();
        assert_eq!(rest, [0xaa]);
        assert_eq!(header.frame_type, FrameType::Key);
        assert_eq!(header.codec_id, CodecId::AV1);

        assert!(video_data_header(&[0x91, b'v', b'p', b'0', b'9']).is_err());
        assert_incomplete(video_data_header(&[0x91, b'h']));
    }
}
