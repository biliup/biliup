use crate::downloader::flv_parser::{
    AACPacketType, AVCPacketType, CodecId, FrameType, SoundFormat, TagData, TagHeader,
    aac_audio_packet_header, avc_video_packet_header, script_data, tag_data, tag_header,
};
use crate::downloader::flv_writer::{FlvFile, FlvTag, TagDataHeader};
use crate::downloader::util::{LifecycleFile, Segmentable};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use nom::{Err, IResult};
use reqwest::Response;

use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

pub async fn download(connection: Connection, file: LifecycleFile<'_>, segment: Segmentable) {
    let file_name = file.file_name.clone();
    match parse_flv(connection, file, segment).await {
        Ok(_) => {
            info!("Done... {}", file_name);
        }
        Err(e) => {
            warn!("{e}")
        }
    }
}

pub(crate) async fn parse_flv(
    mut connection: Connection,
    file: LifecycleFile<'_>,
    mut segment: Segmentable,
) -> crate::downloader::error::Result<()> {
    let mut flv_tags_cache: Vec<(TagHeader, Bytes, Bytes)> = Vec::new();
    // println!("parse_flv Segment: {:?}", segment);
    let _previous_tag_size = connection.read_frame(4).await?;

    let mut out = FlvFile::new(file)?;
    segment.set_size_position(9 + 4);
    // let mut downloaded_size = 9 + 4;
    let mut on_meta_data = None;
    let mut aac_sequence_header = None;
    let mut h264_sequence_header: Option<(TagHeader, Bytes, Bytes)> = None;
    let mut prev_timestamp = 0;
    let mut create_new = false;
    loop {
        let tag_header_bytes = connection.read_frame(11).await?;
        if tag_header_bytes.is_empty() {
            // let mut rdr = Cursor::new(tag_header_bytes);
            // println!("{}", rdr.read_u32::<BigEndian>().unwrap());
            break;
        }

        let (_, tag_header) = map_parse_err(tag_header(&tag_header_bytes), "tag header")?;
        // write_tag_header(&mut out, &tag_header)?;

        let bytes = connection.read_frame(tag_header.data_size as usize).await?;
        let previous_tag_size = connection.read_frame(4).await?;
        // out.write(&bytes)?;
        let (i, flv_tag_data) = map_parse_err(
            tag_data(tag_header.tag_type, tag_header.data_size as usize)(&bytes),
            "tag data",
        )?;
        let flv_tag = match flv_tag_data {
            TagData::Audio(audio_data) => {
                let packet_type = if audio_data.sound_format == SoundFormat::AAC {
                    let (_, packet_header) = aac_audio_packet_header(audio_data.sound_data)
                        .expect("Error in parsing aac audio packet header.");
                    if packet_header.packet_type == AACPacketType::SequenceHeader {
                        if aac_sequence_header.is_some() {
                            warn!("Unexpected aac sequence header tag. {tag_header:?}");
                            // panic!("Unexpected aac_sequence_header tag.");
                            // create_new = true;
                        }
                        aac_sequence_header =
                            Some((tag_header, bytes.clone(), previous_tag_size.clone()))
                    }
                    Some(packet_header.packet_type)
                } else {
                    None
                };

                FlvTag {
                    header: tag_header,
                    data: TagDataHeader::Audio {
                        sound_format: audio_data.sound_format,
                        sound_rate: audio_data.sound_rate,
                        sound_size: audio_data.sound_size,
                        sound_type: audio_data.sound_type,
                        packet_type,
                    },
                }
            }
            TagData::Video(video_data) => {
                let (packet_type, composition_time) = if CodecId::H264 == video_data.codec_id {
                    let (_, avc_video_header) = avc_video_packet_header(video_data.video_data)
                        .expect("Error in parsing avc video packet header.");
                    if avc_video_header.packet_type == AVCPacketType::SequenceHeader {
                        if let Some((_, binary_data, _)) = &h264_sequence_header {
                            warn!("Unexpected h264 sequence header tag. {tag_header:?}");
                            if bytes != binary_data {
                                create_new = true;
                                warn!("Different h264 sequence header tag. {tag_header:?}");
                            }
                        }
                        h264_sequence_header =
                            Some((tag_header, bytes.clone(), previous_tag_size.clone()))
                    }
                    (
                        Some(avc_video_header.packet_type),
                        Some(avc_video_header.composition_time),
                    )
                } else {
                    (None, None)
                };

                FlvTag {
                    header: tag_header,
                    data: TagDataHeader::Video {
                        frame_type: video_data.frame_type,
                        codec_id: video_data.codec_id,
                        packet_type,
                        composition_time,
                    },
                }
            }
            TagData::Script => {
                let (_, tag_data) = script_data(i).expect("Error in parsing script tag.");
                if on_meta_data.is_some() {
                    warn!("Unexpected script tag. {tag_header:?}");
                }
                on_meta_data = Some((tag_header, bytes.clone(), previous_tag_size.clone()));

                FlvTag {
                    header: tag_header,
                    data: TagDataHeader::Script(tag_data),
                }
            }
        };
        match &flv_tag {
            FlvTag {
                data:
                    TagDataHeader::Video {
                        frame_type: FrameType::Key,
                        ..
                    },
                ..
            } => {
                let timestamp = flv_tag.header.timestamp as u64;
                if prev_timestamp == 0 && timestamp != 0 {
                    segment.set_start_time(Duration::from_millis(timestamp));
                }
                segment.set_time_position(Duration::from_millis(timestamp));
                for (tag_header, flv_tag_data, previous_tag_size_bytes) in &flv_tags_cache {
                    if tag_header.timestamp < prev_timestamp {
                        warn!(
                            "Non-monotonous DTS in output stream; previous: {prev_timestamp}, current: {};",
                            tag_header.timestamp
                        );
                    }
                    out.write_tag(tag_header, flv_tag_data, previous_tag_size_bytes)?;
                    segment.increase_size((11 + tag_header.data_size + 4) as u64);
                    // downloaded_size += (11 + tag_header.data_size + 4) as u64;
                    prev_timestamp = tag_header.timestamp
                    // println!("{downloaded_size}");
                }
                flv_tags_cache.clear();

                if segment.needed() || create_new {
                    segment.set_start_time(Duration::from_millis(timestamp));
                    segment.set_size_position(9 + 4);

                    // 开启新分段时补齐已捕获的头部标签。这些头部并非所有直播流都具备
                    // （例如纯视频流没有 AAC 序列头，部分流缺少 onMetaData 脚本标签），
                    // 缺失时跳过并告警，而不是像原先那样直接 expect/panic 中断录制。
                    // onMetaData
                    if let Some((meta_header, meta_bytes, previous_meta_tag_size)) =
                        on_meta_data.as_ref()
                    {
                        flv_tags_cache.push((
                            *meta_header,
                            meta_bytes.clone(),
                            previous_meta_tag_size.clone(),
                        ));
                    } else {
                        warn!("onMetaData not found before segmenting; new segment will omit it.");
                    }
                    // AACSequenceHeader
                    if let Some((aac_header, aac_bytes, aac_prev_tag_size)) =
                        aac_sequence_header.as_ref()
                    {
                        flv_tags_cache.push((
                            *aac_header,
                            aac_bytes.clone(),
                            aac_prev_tag_size.clone(),
                        ));
                    }
                    if !create_new {
                        // H264SequenceHeader
                        if let Some(h264_header) = h264_sequence_header.as_ref() {
                            flv_tags_cache.push(h264_header.clone());
                        } else {
                            warn!(
                                "h264_sequence_header not found before segmenting; new segment may be unplayable."
                            );
                        }
                    }
                    info!("{} splitting.{segment:?}", out.file.file_name);
                    out.create_new()?;
                    create_new = false;
                }
                flv_tags_cache.push((tag_header, bytes.clone(), previous_tag_size.clone()));
            }
            _ => {
                flv_tags_cache.push((tag_header, bytes.clone(), previous_tag_size.clone()));
            }
        }
    }
    Ok(())
}

pub fn map_parse_err<'a, T>(
    i_result: IResult<&'a [u8], T>,
    msg: &str,
) -> core::result::Result<(&'a [u8], T), crate::downloader::error::Error> {
    match i_result {
        Ok((i, res)) => Ok((i, res)),
        Err(nom::Err::Incomplete(needed)) => Err(crate::downloader::error::Error::NomIncomplete(
            msg.to_string(),
            needed,
        )),
        Err(Err::Error(e)) => Err(crate::downloader::error::Error::Custom(format!(
            "parse {msg} err: {e:?}"
        ))),
        Err(Err::Failure(f)) => Err(crate::downloader::error::Error::Custom(format!(
            "{msg} Failure: {f:?}"
        ))),
    }
}

pub struct Connection {
    resp: Response,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(resp: Response) -> Connection {
        Connection {
            resp,
            buffer: BytesMut::with_capacity(8 * 1024),
        }
    }

    pub async fn read_frame(
        &mut self,
        chunk_size: usize,
    ) -> crate::downloader::error::Result<Bytes> {
        // let mut buf = [0u8; 8 * 1024];
        loop {
            if chunk_size <= self.buffer.len() {
                let bytes = Bytes::copy_from_slice(&self.buffer[..chunk_size]);
                self.buffer.advance(chunk_size);
                return Ok(bytes);
            }
            // BytesMut::with_capacity(0).deref_mut()
            // tokio::fs::File::open("").read()
            // self.resp.chunk()
            match timeout(Duration::from_secs(30), self.resp.chunk()).await? {
                Ok(Some(chunk)) => {
                    // let n = chunk.len();
                    // println!("Chunk: {:?}", chunk);
                    self.buffer.put(chunk);
                    // self.buffer.put_slice(&buf[..n]);
                }
                _ => {
                    return Ok(self.buffer.split().freeze());
                }
            }
            // let n = match self.resp.read(&mut buf).await {
            //     Ok(n) => n,
            //     Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            //     Err(e) => return Err(e),
            // };

            // if n == 0 {
            //     return Ok(self.buffer.split().freeze());
            // }
            // self.buffer.put_slice(&buf[..n]);
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::{Buf, BufMut, BytesMut};

    #[test]
    fn byte_it_works() -> Result<(), Box<dyn std::error::Error>> {
        let mut bb = bytes::BytesMut::with_capacity(10);
        println!("chunk {:?}", bb.chunk());
        println!("capacity {}", bb.capacity());
        bb.put(&b"hello"[..]);
        println!("chunk {:?}", bb.chunk());
        println!("remaining {}", bb.remaining());
        bb.advance(5);
        println!("capacity {}", bb.capacity());
        println!("chunk {:?}", bb.chunk());
        println!("remaining {}", bb.remaining());
        bb.put(&b"hello"[..]);
        bb.put(&b"hello"[..]);
        println!("chunk {:?}", bb.chunk());
        println!("capacity {}", bb.capacity());
        println!("remaining {}", bb.remaining());

        let mut buf = BytesMut::with_capacity(11);
        buf.put(&b"hello world"[..]);

        let other = buf.split();
        // buf.advance_mut()

        assert!(buf.is_empty());
        assert_eq!(0, buf.capacity());
        assert_eq!(11, other.capacity());
        assert_eq!(other, b"hello world"[..]);

        Ok(())
    }

    #[test]
    fn it_works() -> Result<(), Box<dyn std::error::Error>> {
        // download(
        //     "test.flv")?;
        Ok(())
    }

    /// 回归测试：纯视频流（没有任何音频标签）在首次分段时不应 panic。
    ///
    /// 该流只包含一个 onMetaData 脚本标签和一个 H264 序列头关键帧，`aac_sequence_header`
    /// 全程为 `None`。修复前，分段重建逻辑会对 `aac_sequence_header` 执行
    /// `expect("aac_sequence_header does not exist")` 而 panic，导致纯视频直播录制中断。
    #[tokio::test]
    async fn pure_video_stream_segments_without_panic() -> Result<(), Box<dyn std::error::Error>> {
        use crate::downloader::util::{LifecycleFile, Segmentable};

        let mut data: Vec<u8> = Vec::new();
        // parse_flv 起始会先读取 4 字节（上一个 tag 的大小），这里给占位。
        data.extend_from_slice(&[0, 0, 0, 0]);

        // Script 标签（onMetaData），其值为 Null（0x05）。
        // 结构：0x02(字符串) + u16 长度(10) + "onMetaData" + 0x05(Null)
        let script_body: [u8; 14] = [
            0x02, 0x00, 0x0A, b'o', b'n', b'M', b'e', b't', b'a', b'D', b'a', b't', b'a', 0x05,
        ];
        // tag_header: type=18(script), data_size=14, timestamp=0, stream_id=0
        data.extend_from_slice(&[
            0x12, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        data.extend_from_slice(&script_body);
        data.extend_from_slice(&[0, 0, 0, 0]); // previous_tag_size

        // Video 标签：关键帧 + H264 序列头（无音频）。
        // body[0]=0x17 → frame_type=Key(1), codec_id=H264(7)
        // 其后 4 字节：avc packet_type=0(SequenceHeader) + composition_time(i24)=0
        let video_body: [u8; 5] = [0x17, 0x00, 0x00, 0x00, 0x00];
        // tag_header: type=9(video), data_size=5, timestamp=0, stream_id=0
        data.extend_from_slice(&[
            0x09, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        data.extend_from_slice(&video_body);
        data.extend_from_slice(&[0, 0, 0, 0]); // previous_tag_size

        let http_resp = http::Response::builder().status(200).body(data)?;
        let resp = reqwest::Response::from(http_resp);
        let connection = super::Connection::new(resp);

        let dir = tempfile::tempdir()?;
        let file_stem = dir.path().join("pure_video_seg");
        let file = LifecycleFile::new(file_stem.to_str().unwrap(), "flv");

        // expected_size 设得极小，确保首个关键帧即触发分段，进入头部重建路径。
        let segment = Segmentable::new(None, Some(1));

        // 修复前：此调用会 panic（aac_sequence_header does not exist）。
        super::parse_flv(connection, file, segment).await?;
        Ok(())
    }
}
