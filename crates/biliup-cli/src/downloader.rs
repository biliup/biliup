use crate::server::common::construct_headers;
use crate::server::errors::{AppError, AppResult};
use biliup::client::StatelessClient;
use biliup::downloader::flv_parser::{
    CodecId, SoundFormat, TagData, aac_audio_packet_header, avc_video_packet_header, header,
    script_data, tag_data, tag_header,
};
use biliup::downloader::flv_writer;
use biliup::downloader::flv_writer::{FlvTag, TagDataHeader};
use biliup::downloader::httpflv::{self, Connection, map_parse_err};
use biliup::downloader::live::{
    DownloaderHint, LiveCredentials, LiveOptions, LiveRequest, LiveStatus, builtin_plugins,
};
use biliup::downloader::util::{LifecycleFile, Segmentable};
use biliup::downloader::{hls, live};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use error_stack::{Report, ResultExt};
use reqwest::header::{ACCEPT_ENCODING, HeaderValue};
use std::io::{BufReader, BufWriter, ErrorKind, Read};
use std::path::PathBuf;
use tracing::{error, info, warn};

pub async fn download(
    url: &str,
    output: String,
    split_size: Option<u64>,
    split_time: Option<humantime::Duration>,
) -> AppResult<()> {
    let segmentable = Segmentable::new(split_time.map(|t| t.into()), split_size);
    let client = reqwest::Client::new();
    let request = LiveRequest {
        client,
        url: url.to_string(),
        name: url.to_string(),
        options: LiveOptions::default(),
        credentials: LiveCredentials::default(),
    };

    let Some(plugin) = builtin_plugins()
        .into_iter()
        .find(|plugin| plugin.matches(url))
    else {
        warn!("not find extractor for {url}");
        return Ok(());
    };

    let status = plugin
        .check_stream(request)
        .await
        .map_err(|err| Report::new(AppError::Custom(err.to_string())))?;
    let LiveStatus::Live { stream } = status else {
        warn!("stream is offline: {url}");
        return Ok(());
    };

    match stream.downloader_hint {
        DownloaderHint::StreamGears | DownloaderHint::Ffmpeg => {
            download_stream(
                &stream.raw_stream_url,
                &stream.stream_headers,
                &stream.title,
                &output,
                segmentable,
            )
            .await
        }
        DownloaderHint::Streamlink | DownloaderHint::YtDlp => {
            Err(Report::new(AppError::Custom(format!(
                "biliup download 不支持 {} 的 {:?} 运行时下载，请使用服务端录制链路",
                stream.platform, stream.downloader_hint
            ))))
        }
    }
}

async fn download_stream(
    url: &str,
    headers: &std::collections::HashMap<String, String>,
    title: &str,
    output: &str,
    segmentable: Segmentable,
) -> AppResult<()> {
    let output = output.replace("{title}", title);
    let headers = construct_headers(headers).map_err(|err| Report::new(AppError::Custom(err)))?;
    let mut client = StatelessClient::new(headers.clone(), None);
    client.headers = headers;

    match live::media_ext_from_url(url).as_deref() {
        Some("m3u8" | "ts") => {
            let file = LifecycleFile::new(&output, "ts");
            hls::download(url, &client, file, segmentable)
                .await
                .change_context_lazy(|| AppError::Unknown)
        }
        _ => {
            client
                .headers
                .append(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate"));
            let response = client
                .retryable(url)
                .await
                .change_context_lazy(|| AppError::Unknown)?;
            let mut connection = Connection::new(response);
            connection
                .read_frame(9)
                .await
                .change_context_lazy(|| AppError::Unknown)?;
            let file = LifecycleFile::new(&output, "flv");
            httpflv::download(connection, file, segmentable).await;
            Ok(())
        }
    }
}

pub fn generate_json(mut file_name: PathBuf) -> AppResult<()> {
    // let args: Vec<String> = env::args().collect();
    // let file_name = &args[1];
    let flv_file = std::fs::File::open(&file_name).change_context_lazy(|| AppError::Unknown)?;
    let buf_reader = BufReader::new(flv_file);
    let mut reader = Reader::new(buf_reader);

    let mut script_tag_count = 0;
    let mut audio_tag_count = 0;
    let mut video_tag_count = 0;
    let mut tag_count = 0;
    let _err_count = 0;
    let flv_header = reader
        .read_frame(9)
        .change_context_lazy(|| AppError::Unknown)?;
    // file_name.parent().and_then(|p|p + file_name.file_name()+".json");
    // Vec::clear()
    let (_, header) = map_parse_err(header(&flv_header), "flv header")
        .change_context_lazy(|| AppError::Unknown)?;
    let mut os_string = file_name.extension().unwrap_or_default().to_owned();
    os_string.push(".json");
    file_name.set_extension(os_string);
    // file_name.extend(".json");
    // file_name.with_extension()
    let file = std::fs::File::options()
        .create_new(true)
        .write(true)
        .open(&file_name)
        .change_context_lazy(|| {
            AppError::Custom(format!(
                "file name: {}",
                file_name.canonicalize().unwrap().display()
            ))
        })?;
    let mut writer = BufWriter::new(file);
    flv_writer::to_json(&mut writer, &header).change_context_lazy(|| AppError::Unknown)?;
    loop {
        let _previous_tag_size = reader
            .read_frame(4)
            .change_context_lazy(|| AppError::Unknown)?;

        let t_header = reader
            .read_frame(11)
            .change_context_lazy(|| AppError::Unknown)?;
        if t_header.is_empty() {
            break;
        }
        let tag_header = match map_parse_err(tag_header(&t_header), "tag header") {
            Ok((_, tag_header)) => tag_header,
            Err(e) => {
                error!("{e}");
                break;
            }
        };
        tag_count += 1;
        let bytes = reader
            .read_frame(tag_header.data_size as usize)
            .change_context_lazy(|| AppError::Unknown)?;
        let (i, flv_tag_data) = match map_parse_err(
            tag_data(tag_header.tag_type, tag_header.data_size as usize)(&bytes),
            "tag data",
        ) {
            Ok((i, flv_tag_data)) => (i, flv_tag_data),
            Err(e) => {
                error!("{e}");
                break;
            }
        };

        let flv_tag = match flv_tag_data {
            TagData::Audio(audio_data) => {
                audio_tag_count += 1;

                let packet_type = if audio_data.sound_format == SoundFormat::AAC {
                    let (_, packet_header) =
                        aac_audio_packet_header(audio_data.sound_data).unwrap();
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
                video_tag_count += 1;

                let (packet_type, composition_time) = if CodecId::H264 == video_data.codec_id {
                    let (_, avc_video_header) =
                        avc_video_packet_header(video_data.video_data).unwrap();
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
                script_tag_count += 1;

                let (_, tag_data) = script_data(i).unwrap();

                FlvTag {
                    header: tag_header,
                    data: TagDataHeader::Script(tag_data),
                }
            }
        };
        flv_writer::to_json(&mut writer, &flv_tag).change_context_lazy(|| AppError::Unknown)?;
    }
    info!("tag count: {tag_count}");
    info!("script tag count: {script_tag_count}");
    info!("audio tag count: {audio_tag_count}");
    info!("video tag count: {video_tag_count}");
    Ok(())
}

pub struct Reader<T> {
    read: T,
    buffer: BytesMut,
}

impl<T: Read> Reader<T> {
    fn new(read: T) -> Reader<T> {
        Reader {
            read,
            buffer: BytesMut::with_capacity(8 * 1024),
        }
    }

    fn read_frame(&mut self, chunk_size: usize) -> std::io::Result<Bytes> {
        let mut buf = [0u8; 8 * 1024];
        loop {
            if chunk_size <= self.buffer.len() {
                let bytes = Bytes::copy_from_slice(&self.buffer[..chunk_size]);
                self.buffer.advance(chunk_size);
                return Ok(bytes);
            }
            // BytesMut::with_capacity(0).deref_mut()
            // tokio::fs::File::open("").read()
            // self.read_buf.
            let n = match self.read.read(&mut buf) {
                Ok(n) => n,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            if n == 0 {
                return Ok(self.buffer.split().freeze());
            }
            self.buffer.put_slice(&buf[..n]);
        }
    }
}
