use crate::downloader::error::{Error, Result};
use crate::downloader::preview::{ChunkKind, PreviewSink};
use crate::downloader::util::{ByteCounter, LifecycleFile, Segmentable};
use m3u8_rs::{MediaPlaylist, Playlist};

use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use url::Url;

use crate::client::StatelessClient;

const MIN_PLAYLIST_POLL_INTERVAL: Duration = Duration::from_secs(1);

fn parse_media_playlist(bytes: &[u8]) -> Result<MediaPlaylist> {
    m3u8_rs::parse_media_playlist(bytes)
        .map(|(_, playlist)| playlist)
        .map_err(|error| Error::Custom(format!("Unable to parse media playlist content: {error}")))
}

fn playlist_poll_interval(playlist: &MediaPlaylist) -> Duration {
    Duration::from_secs(playlist.target_duration).max(MIN_PLAYLIST_POLL_INTERVAL)
}

fn playlist_should_refresh(playlist: &MediaPlaylist) -> bool {
    !playlist.end_list
}

/// 轮询 m3u8 并把分片追加进同一个 `.ts` 文件。
///
/// `preview` 为直播预览的写入端：每个分片的字节在落盘的同时旁路一份给它，`None` 则不旁路。
pub async fn download(
    url: &str,
    client: &StatelessClient,
    file: LifecycleFile<'_>,
    mut splitting: Segmentable,
    mut preview: Option<PreviewSink>,
) -> Result<()> {
    info!("Downloading {}...", url);
    let resp = client.retryable(url).await?;
    info!("{}", resp.status());
    // let mut resp = resp.bytes_stream();
    let bytes = resp.bytes().await?;
    let mut ts_file = TsFile::new(file)?;

    let mut media_url = Url::parse(url)?;
    let mut pl = match m3u8_rs::parse_playlist(&bytes) {
        Ok((_i, Playlist::MasterPlaylist(pl))) => {
            info!("Master playlist:\n{:#?}", pl);
            // Pick the highest-bandwidth playable variant. The first variant is not
            // necessarily the best quality (e.g. Twitch orders transcodes ahead of the
            // source), so prefer the highest-bandwidth stream that carries a resolution.
            // Skip I-frame (trick-play) streams, which are not full playable renditions.
            // Fall back to the highest-bandwidth non-I-frame variant, then the first one.
            let best = pl
                .variants
                .iter()
                .filter(|v| !v.is_i_frame && v.resolution.is_some())
                .max_by_key(|v| v.bandwidth)
                .or_else(|| {
                    pl.variants
                        .iter()
                        .filter(|v| !v.is_i_frame)
                        .max_by_key(|v| v.bandwidth)
                })
                .unwrap_or(&pl.variants[0]);
            info!(
                "Selected variant: bandwidth={}, resolution={:?}, video={:?}",
                best.bandwidth, best.resolution, best.video
            );
            media_url = media_url.join(&best.uri)?;
            info!("media url: {media_url}");
            let resp = client.retryable(media_url.as_str()).await?;
            let bs = resp.bytes().await?;
            parse_media_playlist(&bs)?
        }
        Ok((_i, Playlist::MediaPlaylist(pl))) => {
            info!("Media playlist:\n{:#?}", pl);
            info!("index {}", pl.media_sequence);
            pl
        }
        Err(e) => return Err(Error::Custom(format!("Parsing playlist error: {e}"))),
    };
    let mut previous_last_segment = 0;
    let mut last_playlist_load = Instant::now();
    loop {
        if pl.segments.is_empty() {
            debug!("Segments array is empty - waiting for playlist update");
        }
        let mut seq = pl.media_sequence;
        for segment in &pl.segments {
            if seq > previous_last_segment {
                if (previous_last_segment > 0) && (seq > (previous_last_segment + 1)) {
                    warn!("SEGMENT INFO SKIPPED");
                }
                debug!("Yield segment");
                if segment.discontinuity {
                    warn!("#EXT-X-DISCONTINUITY");
                    ts_file.create_new()?;
                    // splitting = Segment::from_seg(splitting);
                    splitting.reset();
                }
                let length = download_to_file(
                    media_url.join(&segment.uri)?,
                    client,
                    &mut ts_file.buf_writer,
                    &ts_file.file.bytes_written,
                    preview.as_mut(),
                )
                .await?;
                splitting.increase_size(length);
                splitting.increase_time(Duration::from_secs(segment.duration as u64));
                if splitting.needed() {
                    ts_file.create_new()?;
                    splitting.reset();
                }
                previous_last_segment = seq;
            }
            seq += 1;
        }

        if !playlist_should_refresh(&pl) {
            info!("#EXT-X-ENDLIST received - stream finished");
            break;
        }

        let poll_interval = playlist_poll_interval(&pl);
        let refresh_delay = poll_interval.saturating_sub(last_playlist_load.elapsed());
        if !refresh_delay.is_zero() {
            debug!("Waiting {refresh_delay:?} before refreshing media playlist");
            tokio::time::sleep(refresh_delay).await;
        }

        let resp = client.retryable(media_url.as_str()).await?;
        let bs = resp.bytes().await?;
        pl = parse_media_playlist(&bs)?;
        last_playlist_load = Instant::now();
    }
    info!("Done...");
    Ok(())
}

async fn download_to_file(
    url: Url,
    client: &StatelessClient,
    out: &mut impl Write,
    bytes_written: &ByteCounter,
    mut preview: Option<&mut PreviewSink>,
) -> Result<u64> {
    debug!("url: {url}");
    let mut response = client.retryable(url.as_str()).await?;
    let mut length: u64 = 0;
    // 分片起点即预览的关键帧边界（HLS 分片自带 PAT/PMT、从关键帧开始），
    // 新订阅者从最近一个分片的开头起播
    let mut segment_start = true;
    while let Some(chunk) = response.chunk().await? {
        length += chunk.len() as u64;
        out.write_all(&chunk)?;
        bytes_written.add(chunk.len() as u64);
        if let Some(sink) = preview.as_deref_mut() {
            if segment_start && chunk.first() != Some(&0x47) {
                // 不是 TS 同步字节：多半是 fMP4（m4s）分片。这条路径没有下载 #EXT-X-MAP 的
                // 初始化分片（录制文件同样如此），没有 init segment 就播不了，明确标为不可预览
                sink.mark_unavailable(
                    "HLS 分片不是 MPEG-TS（可能是 fMP4），stream-gears 暂不支持预览此格式，可改用 mesio",
                );
            }
            if segment_start {
                // 分片起点：嗅探首个视频 PES 是否从 IDR 起，决定要不要作为新 GOP 的起点
                sink.push_ts_segment_start(chunk);
            } else {
                sink.push(ChunkKind::Media, chunk);
            }
        }
        segment_start = false;
    }
    // let mut out = File::options()
    //     .append(true)
    //     .open(format!("{file_name}.ts"))?;
    // let length = response.copy_to(out)?;
    Ok(length)
}

pub struct TsFile<'a> {
    pub buf_writer: BufWriter<File>,
    pub file: LifecycleFile<'a>,
    /// 当前分段已交给 [`LifecycleFile::finish`]，`Drop` 不再重复改名、触发钩子。
    finished: bool,
}

impl<'a> TsFile<'a> {
    pub fn new(mut file: LifecycleFile<'a>) -> std::io::Result<Self> {
        let path = file.create()?;
        Ok(Self {
            buf_writer: Self::create(path)?,
            file,
            finished: false,
        })
    }

    /// 结束当前分段并开始下一个。当前分段 flush 失败时返回错误，不再开新文件。
    pub fn create_new(&mut self) -> std::io::Result<()> {
        self.finish()?;
        let path = self.file.create()?;
        self.buf_writer = Self::create(path)?;
        self.finished = false;
        Ok(())
    }

    /// flush 并检查错误 → 去掉 `.part` → 触发钩子，见 [`LifecycleFile::finish`]。
    fn finish(&mut self) -> std::io::Result<()> {
        self.finished = true;
        self.file.finish(&mut self.buf_writer)
    }

    fn create<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<BufWriter<File>> {
        let path = path.as_ref();
        let out = match File::create(path) {
            Ok(o) => o,
            Err(e) => {
                return Err(std::io::Error::new(
                    e.kind(),
                    format!("Unable to create file {}", path.display()),
                ));
            }
        };
        info!("create file {}", path.display());
        Ok(BufWriter::new(out))
    }
}

impl Drop for TsFile<'_> {
    fn drop(&mut self) {
        if !self.finished
            && let Err(e) = self.finish()
        {
            error!("{e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_media_playlist, playlist_poll_interval, playlist_should_refresh};
    use m3u8_rs::MediaPlaylist;
    use reqwest::Url;
    use std::time::Duration;

    #[test]
    fn test_url() -> Result<(), Box<dyn std::error::Error>> {
        let url = Url::parse("h://host.path/to/remote/resource.m3u8")?;
        let scheme = url.scheme();
        let new_url = url.join("http://path.host/remote/resource.ts")?;
        println!("{url}, {scheme}");
        println!("{new_url}, {scheme}");
        Ok(())
    }

    #[test]
    fn it_works() -> Result<(), Box<dyn std::error::Error>> {
        // download(
        //     "test.ts")?;
        Ok(())
    }

    #[test]
    fn playlist_poll_interval_uses_target_duration() {
        let playlist = MediaPlaylist {
            target_duration: 6,
            ..MediaPlaylist::default()
        };

        assert_eq!(playlist_poll_interval(&playlist), Duration::from_secs(6));
    }

    #[test]
    fn playlist_poll_interval_has_one_second_minimum() {
        assert_eq!(
            playlist_poll_interval(&MediaPlaylist::default()),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn parse_media_playlist_preserves_end_list() {
        let playlist = parse_media_playlist(
            b"#EXTM3U\n\
              #EXT-X-TARGETDURATION:6\n\
              #EXT-X-MEDIA-SEQUENCE:7\n\
              #EXTINF:6.0,\n\
              7.ts\n\
              #EXT-X-ENDLIST\n",
        )
        .expect("valid media playlist should parse");

        assert!(playlist.end_list);
        assert!(!playlist_should_refresh(&playlist));
        assert_eq!(playlist.segments.len(), 1);
    }

    /// 分段钩子触发时 `BufWriter` 里的数据已经写进文件：钩子看到的大小就是最终大小。
    #[test]
    fn the_segment_hook_sees_the_flushed_file() -> Result<(), Box<dyn std::error::Error>> {
        use super::TsFile;
        use crate::downloader::util::LifecycleFile;
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        let dir = tempfile::tempdir()?;
        let seen: Arc<Mutex<Vec<u64>>> = Arc::default();
        let file = LifecycleFile::with_hook(dir.path().join("rec").to_str().unwrap(), "ts", {
            let seen = seen.clone();
            let dir = dir.path().to_path_buf();
            move |name: &str| {
                let mut seen = seen.lock().unwrap();
                seen.push(std::fs::metadata(name).unwrap().len());
                std::fs::rename(name, dir.join(format!("seg-{}.ts", seen.len()))).unwrap();
            }
        });
        let mut ts = TsFile::new(file)?;
        ts.buf_writer.write_all(&[0x47; 188 * 3])?;
        ts.create_new()?;
        ts.buf_writer.write_all(&[0x47; 188])?;
        drop(ts);

        assert_eq!(*seen.lock().unwrap(), vec![188 * 3, 188]);
        assert_eq!(std::fs::metadata(dir.path().join("seg-2.ts"))?.len(), 188);
        Ok(())
    }

    #[test]
    fn parse_media_playlist_returns_error_for_invalid_content() {
        let error = parse_media_playlist(b"not a media playlist")
            .expect_err("invalid media playlist should return an error");

        assert!(error.to_string().contains("Unable to parse media playlist"));
    }
}
