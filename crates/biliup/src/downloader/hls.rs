use crate::downloader::error::{Error, Result};
use crate::downloader::index_tap::FileTap;
use crate::downloader::preview::{ChunkKind, PreviewSink};
use crate::downloader::util::{LifecycleFile, Segmentable};
use bytes::Bytes;
use m3u8_rs::{MasterPlaylist, MediaPlaylist, MediaSegment, Playlist, VariantStream};

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

/// 上一片之后跳过了至少一个媒体序号。`previous_last_segment` 为 `None` 表示还没写过切片。
///
/// 媒体序号可以从 0 开始（没写 `#EXT-X-MEDIA-SEQUENCE` 时就是 0），所以「没写过」不能用 0 表示。
fn has_sequence_gap(previous_last_segment: Option<u64>, current_segment: u64) -> bool {
    previous_last_segment.is_some_and(|previous| current_segment > previous.saturating_add(1))
}

/// `#EXTINF` 给出的分片时长，保留小数部分（直播分片常是 1.968、5.96 之类）；
/// 负数、NaN 等畸形值按 0 计，而不是 panic。
fn segment_duration(segment: &MediaSegment) -> Duration {
    Duration::try_from_secs_f64(f64::from(segment.duration)).unwrap_or_default()
}

/// 从主播放列表里挑带宽最高、可播放的变体；一个变体都没有时返回 `None`。
fn select_variant(pl: &MasterPlaylist) -> Option<&VariantStream> {
    // Pick the highest-bandwidth playable variant. The first variant is not
    // necessarily the best quality (e.g. Twitch orders transcodes ahead of the
    // source), so prefer the highest-bandwidth stream that carries a resolution.
    // Skip I-frame (trick-play) streams, which are not full playable renditions.
    // Fall back to the highest-bandwidth non-I-frame variant, then the first one.
    pl.variants
        .iter()
        .filter(|v| !v.is_i_frame && v.resolution.is_some())
        .max_by_key(|v| v.bandwidth)
        .or_else(|| {
            pl.variants
                .iter()
                .filter(|v| !v.is_i_frame)
                .max_by_key(|v| v.bandwidth)
        })
        .or_else(|| pl.variants.first())
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
            let best = select_variant(&pl).ok_or_else(|| {
                Error::Custom("Master playlist has no variant stream to record".to_string())
            })?;
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
    let mut previous_last_segment: Option<u64> = None;
    let mut last_playlist_load = Instant::now();
    loop {
        if pl.segments.is_empty() {
            debug!("Segments array is empty - waiting for playlist update");
        }
        let mut seq = pl.media_sequence;
        for segment in &pl.segments {
            if previous_last_segment.is_none_or(|previous| seq > previous) {
                // 媒体序号跳号：漏掉的切片补不回来，已写的内容与下一片接不上，在这里切段，
                // 让每个文件内部的时间戳和弹幕保持连续
                let gap = has_sequence_gap(previous_last_segment, seq);
                if gap {
                    warn!(
                        previous = ?previous_last_segment,
                        current = seq,
                        "HLS segment sequence gap detected; starting a new file"
                    );
                }
                if segment.discontinuity {
                    warn!("#EXT-X-DISCONTINUITY");
                }
                debug!("Yield segment");
                // 刚切过段的空文件不再切，免得发布空分段
                if (gap || segment.discontinuity) && ts_file.pos > 0 {
                    ts_file.create_new()?;
                    splitting.reset();
                }
                let length = download_to_file(
                    media_url.join(&segment.uri)?,
                    client,
                    &mut ts_file,
                    preview.as_mut(),
                )
                .await?;
                splitting.increase_size(length);
                splitting.increase_time(segment_duration(segment));
                if splitting.needed() {
                    ts_file.create_new()?;
                    splitting.reset();
                }
                previous_last_segment = Some(seq);
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
    out: &mut TsFile<'_>,
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
        out.write_chunk(&chunk)?;
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
    /// 当前分段已写的字节数。
    pos: u64,
    index: Option<FileTap>,
}

impl<'a> TsFile<'a> {
    pub fn new(mut file: LifecycleFile<'a>) -> std::io::Result<Self> {
        let path = file.create()?;
        let buf_writer = Self::create(path)?;
        let index = file.index.as_ref().map(|tap| tap.open(&file.path));
        Ok(Self {
            buf_writer,
            file,
            finished: false,
            pos: 0,
            index,
        })
    }

    /// 结束当前分段并开始下一个。当前分段 flush 失败时返回错误，不再开新文件。
    pub fn create_new(&mut self) -> std::io::Result<()> {
        self.finish()?;
        let path = self.file.create()?;
        self.buf_writer = Self::create(path)?;
        self.finished = false;
        self.pos = 0;
        self.index = self
            .file
            .index
            .as_ref()
            .map(|tap| tap.open(&self.file.path));
        Ok(())
    }

    /// 把一块分片字节追加进当前分段。
    pub fn write_chunk(&mut self, chunk: &Bytes) -> std::io::Result<()> {
        self.buf_writer.write_all(chunk)?;
        self.file.bytes_written.add(chunk.len() as u64);
        if let Some(index) = &self.index {
            index.bytes(self.pos, chunk);
        }
        self.pos += chunk.len() as u64;
        Ok(())
    }

    /// flush 并检查错误 → 去掉 `.part` → 触发钩子，见 [`LifecycleFile::finish`]。
    fn finish(&mut self) -> std::io::Result<()> {
        self.finished = true;
        // 先于改名钩子发出：录制器收到分段关闭时，索引任务队列里已有这个文件的全部事件
        if let Some(index) = self.index.take() {
            index.closed(self.pos);
        }
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
    use super::{
        has_sequence_gap, parse_media_playlist, playlist_poll_interval, playlist_should_refresh,
    };
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
    fn sequence_gap_is_detected_without_treating_initial_sequence_as_a_gap() {
        assert!(!has_sequence_gap(None, 100));
        assert!(!has_sequence_gap(Some(100), 101));
        assert!(has_sequence_gap(Some(100), 102));
        assert!(!has_sequence_gap(Some(u64::MAX), u64::MAX));
        // 序号 0 是真实写过的切片，不是「还没写过」
        assert!(!has_sequence_gap(Some(0), 1));
        assert!(has_sequence_gap(Some(0), 2));
    }

    #[test]
    fn segment_duration_keeps_fractions_and_ignores_malformed_values() {
        let segment = |duration: f32| m3u8_rs::MediaSegment {
            duration,
            ..Default::default()
        };
        assert_eq!(
            super::segment_duration(&segment(1.5)),
            Duration::from_millis(1500)
        );
        assert_eq!(super::segment_duration(&segment(-1.0)), Duration::ZERO);
        assert_eq!(super::segment_duration(&segment(f32::NAN)), Duration::ZERO);
        assert_eq!(
            super::segment_duration(&segment(f32::INFINITY)),
            Duration::ZERO
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

    /// 起一个本地 HTTP 服务，按路径返回固定内容，返回它的 base URL。
    async fn serve(routes: Vec<(&'static str, Vec<u8>)>) -> String {
        let mut app = axum::Router::new();
        for (path, body) in routes {
            app = app.route(
                path,
                axum::routing::get(move || {
                    let body = body.clone();
                    async move { body }
                }),
            );
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}")
    }

    /// 一个内容可辨认的 TS 分片（一个 188 字节的包，第二个字节是序号）。
    fn ts_segment(n: u8) -> Vec<u8> {
        let mut packet = vec![0x47; 188];
        packet[1] = n;
        packet
    }

    /// 录 `base/index.m3u8`，每个分段文件在钩子里改名留下；返回 `download` 的结果与各分段内容。
    async fn record(
        routes: Vec<(&'static str, Vec<u8>)>,
        splitting: crate::downloader::util::Segmentable,
    ) -> (crate::downloader::error::Result<()>, Vec<Vec<u8>>) {
        use crate::client::StatelessClient;
        use crate::downloader::util::LifecycleFile;
        use std::sync::{Arc, Mutex};

        let base = serve(routes).await;
        let dir = tempfile::tempdir().unwrap();
        let kept: Arc<Mutex<Vec<std::path::PathBuf>>> = Arc::default();
        let file = LifecycleFile::with_hook(dir.path().join("rec").to_str().unwrap(), "ts", {
            let kept = kept.clone();
            let dir = dir.path().to_path_buf();
            move |name: &str| {
                let mut kept = kept.lock().unwrap();
                let path = dir.join(format!("seg-{}.ts", kept.len()));
                std::fs::rename(name, &path).unwrap();
                kept.push(path);
            }
        });
        let client = StatelessClient::default();
        let url = format!("{base}/index.m3u8");
        let result = super::download(&url, &client, file, splitting, None).await;
        let segments = kept
            .lock()
            .unwrap()
            .iter()
            .map(|path| std::fs::read(path).unwrap())
            .collect();
        (result, segments)
    }

    /// 没写 `#EXT-X-MEDIA-SEQUENCE` 的播放列表媒体序号从 0 起：序号为 0 的第一个分片也要录下来。
    #[tokio::test]
    async fn the_first_segment_of_a_playlist_starting_at_sequence_zero_is_recorded() {
        let playlist = b"#EXTM3U\n\
            #EXT-X-TARGETDURATION:2\n\
            #EXTINF:2.0,\n\
            0.ts\n\
            #EXTINF:2.0,\n\
            1.ts\n\
            #EXT-X-ENDLIST\n";
        let (result, segments) = record(
            vec![
                ("/index.m3u8", playlist.to_vec()),
                ("/0.ts", ts_segment(0)),
                ("/1.ts", ts_segment(1)),
            ],
            crate::downloader::util::Segmentable::default(),
        )
        .await;
        result.unwrap();
        assert_eq!(segments, vec![[ts_segment(0), ts_segment(1)].concat()]);
    }

    /// 按时长分段时 `#EXTINF` 的小数部分也要计入：4 个 1.5 秒的分片、每 3 秒一段，
    /// 应当两片一段，而不是把每片当成 1 秒、三片才切一次。
    #[tokio::test]
    async fn fractional_segment_durations_count_toward_time_splitting() {
        let playlist = b"#EXTM3U\n\
            #EXT-X-TARGETDURATION:2\n\
            #EXT-X-MEDIA-SEQUENCE:10\n\
            #EXTINF:1.5,\n\
            10.ts\n\
            #EXTINF:1.5,\n\
            11.ts\n\
            #EXTINF:1.5,\n\
            12.ts\n\
            #EXTINF:1.5,\n\
            13.ts\n\
            #EXT-X-ENDLIST\n";
        let (result, segments) = record(
            vec![
                ("/index.m3u8", playlist.to_vec()),
                ("/10.ts", ts_segment(10)),
                ("/11.ts", ts_segment(11)),
                ("/12.ts", ts_segment(12)),
                ("/13.ts", ts_segment(13)),
            ],
            crate::downloader::util::Segmentable::new(Some(Duration::from_secs(3)), None),
        )
        .await;
        result.unwrap();
        assert!(segments.len() >= 2, "{} segments", segments.len());
        assert_eq!(segments[0], [ts_segment(10), ts_segment(11)].concat());
        assert_eq!(segments[1], [ts_segment(12), ts_segment(13)].concat());
    }

    /// 只有 `#EXT-X-MEDIA` 没有任何 `#EXT-X-STREAM-INF` 的主播放列表：返回错误，而不是越界 panic。
    #[tokio::test]
    async fn a_master_playlist_without_variants_is_an_error() {
        let playlist = b"#EXTM3U\n\
            #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aac\",NAME=\"audio\",URI=\"audio.m3u8\"\n";
        let (result, _) = record(
            vec![("/index.m3u8", playlist.to_vec())],
            crate::downloader::util::Segmentable::default(),
        )
        .await;
        assert!(result.is_err());
    }

    #[test]
    fn parse_media_playlist_returns_error_for_invalid_content() {
        let error = parse_media_playlist(b"not a media playlist")
            .expect_err("invalid media playlist should return an error");

        assert!(error.to_string().contains("Unable to parse media playlist"));
    }
}
