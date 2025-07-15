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

pub fn complete_tag(input: &[u8]) -> IResult<&[u8], Tag> {
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

pub fn aac_audio_packet(input: &[u8], size: usize) -> IResult<&[u8], AACAudioPacket> {
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

pub fn audio_data(input: &[u8], size: usize) -> IResult<&[u8], AudioData> {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AVCPacketType {
    SequenceHeader,
    NALU,
    EndOfSequence,
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

pub fn avc_video_packet(input: &[u8], size: usize) -> IResult<&[u8], AVCVideoPacket> {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoData<'a> {
    pub frame_type: FrameType,
    pub codec_id: CodecId,
    pub video_data: &'a [u8],
}

pub fn video_data(input: &[u8], size: usize) -> IResult<&[u8], VideoData> {
    if input.len() < size {
        return Err(Err::Incomplete(Needed::new(size)));
    }

    if size < 1 {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let take_bits = pair(take(4usize), take(4usize));
    bits::<_, _, Error<_>, _, _>(take_bits)(input).and_then(|(_, (frame_type, codec_id))| {
        let frame_type = match frame_type {
            1 => FrameType::Key,
            2 => FrameType::Inter,
            3 => FrameType::DisposableInter,
            4 => FrameType::Generated,
            5 => FrameType::Command,
            _ => return Err(Err::Error(Error::new(input, ErrorKind::Alt))),
        };
        let codec_id = match codec_id {
            1 => CodecId::JPEG,
            2 => CodecId::SORENSON_H263,
            3 => CodecId::SCREEN,
            4 => CodecId::VP6,
            5 => CodecId::VP6A,
            6 => CodecId::SCREEN2,
            7 => CodecId::H264,
            8 => CodecId::H263,
            9 => CodecId::MPEG4Part2,
            _ => return Err(Err::Error(Error::new(input, ErrorKind::Alt))),
        };

        Ok((
            &input[size..],
            VideoData {
                frame_type,
                codec_id,
                video_data: &input[1..size],
            },
        ))
    })
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

    let take_bits = pair(take(4usize), take(4usize));
    map_res(
        bits::<_, _, Error<_>, _, _>(take_bits),
        |(frame_type, codec_id)| {
            let frame_type = match frame_type {
                1 => FrameType::Key,
                2 => FrameType::Inter,
                3 => FrameType::DisposableInter,
                4 => FrameType::Generated,
                5 => FrameType::Command,
                _ => {
                    return Err(Err::<nom::error::Error<&[u8]>>::Error(Error::new(
                        input,
                        ErrorKind::Alt,
                    )));
                }
            };
            let codec_id = match codec_id {
                1 => CodecId::JPEG,
                2 => CodecId::SORENSON_H263,
                3 => CodecId::SCREEN,
                4 => CodecId::VP6,
                5 => CodecId::VP6A,
                6 => CodecId::SCREEN2,
                7 => CodecId::H264,
                8 => CodecId::H263,
                9 => CodecId::MPEG4Part2,
                _ => return Err(Err::Error(Error::new(input, ErrorKind::Alt))),
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

pub fn script_data(input: &[u8]) -> IResult<&[u8], ScriptData> {
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

pub fn script_data_value(input: &[u8]) -> IResult<&[u8], ScriptDataValue> {
    be_u8(input).and_then(|v| match v {
        (i, 0) => map(be_f64, ScriptDataValue::Number).parse(i),
        (i, 1) => map(be_u8, |n| ScriptDataValue::Boolean(n != 0)).parse(i),
        (i, 2) => map(script_data_string, ScriptDataValue::String).parse(i),
        (i, 3) => map(script_data_objects, ScriptDataValue::Object).parse(i),
        (i, 4) => map(script_data_string, ScriptDataValue::MovieClip).parse(i),
        (i, 5) => Ok((i, ScriptDataValue::Null)), // to remove
        (i, 6) => Ok((i, ScriptDataValue::Undefined)), // to remove
        (i, 7) => map(be_u16, ScriptDataValue::Reference).parse(i),
        (i, 8) => map(script_data_ecma_array, ScriptDataValue::ECMAArray).parse(i),
        (i, 10) => map(script_data_strict_array, ScriptDataValue::StrictArray).parse(i),
        (i, 11) => map(script_data_date, ScriptDataValue::Date).parse(i),
        (i, 12) => map(script_data_long_string, ScriptDataValue::LongString).parse(i),
        _ => Err(Err::Error(Error::new(input, ErrorKind::Alt))),
    })
}

pub fn script_data_objects(input: &[u8]) -> IResult<&[u8], Vec<ScriptDataObject>> {
    terminated(many0(script_data_object), script_data_object_end).parse(input)
}

pub fn script_data_object(input: &[u8]) -> IResult<&[u8], ScriptDataObject> {
    map(
        pair(script_data_string, script_data_value),
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

pub fn script_data_ecma_array(input: &[u8]) -> IResult<&[u8], Vec<ScriptDataObject>> {
    map(pair(be_u32, script_data_objects), |(_, data_objects)| {
        data_objects
    })
    .parse(input)
}

pub fn script_data_strict_array(input: &[u8]) -> IResult<&[u8], Vec<ScriptDataValue>> {
    flat_map(be_u32, |o| many_m_n(1, o as usize, script_data_value)).parse(input)
}
