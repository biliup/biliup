use crate::downloader::error::{Error, Result};
use crate::downloader::index_tap::FileTap;
use crate::downloader::preview::{ChunkKind, PreviewFormat, PreviewHub, PreviewSink};
use crate::downloader::util::{LifecycleFile, Segmentable};
use bytes::Bytes;
use m3u8_rs::{
    KeyMethod, Map, MasterPlaylist, MediaPlaylist, MediaSegment, Playlist, VariantStream,
};

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

/// HLS 的录制容器。
///
/// 带 `#EXT-X-MAP` 的播放列表是 fMP4：init segment（ftyp + moov）之后跟着 moof/mdat 分片，
/// 拼起来就是可播的分片 MP4；其余按 MPEG-TS 处理（每个分片自带 PAT/PMT）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HlsContainer {
    Ts,
    Fmp4,
}

impl HlsContainer {
    /// 落盘扩展名：TS 存 `.ts`，fMP4 存 `.mp4`（与 mesio 一致，后续分段校验与投稿都按 mp4 处理）。
    pub fn extension(self) -> &'static str {
        match self {
            HlsContainer::Ts => "ts",
            HlsContainer::Fmp4 => "mp4",
        }
    }

    /// TS 分片给浏览器里的 mpegts.js；fMP4 分片直接交给 MediaSource。
    fn preview_format(self) -> PreviewFormat {
        match self {
            HlsContainer::Ts => PreviewFormat::MpegTs,
            HlsContainer::Fmp4 => PreviewFormat::Fmp4,
        }
    }
}

/// 播放列表里的初始化分片：`#EXT-X-MAP` 可以挂在任意分片上，取第一个。
fn map_of(playlist: &MediaPlaylist) -> Option<&Map> {
    playlist
        .segments
        .iter()
        .find_map(|segment| segment.map.as_ref())
}

/// 初始化分片的身份（URI 加字节范围）。变了就说明解码配置换了：要重下、并另开一个文件。
fn map_identity(map: &Map) -> String {
    match &map.byte_range {
        Some(range) => format!("{}#{}@{}", map.uri, range.length, range.offset.unwrap_or(0)),
        None => map.uri.clone(),
    }
}

/// 加密的播放列表解不开（`#EXT-X-KEY` 的 METHOD 不是 NONE）：明确报错，别写出打不开的文件。
fn refuse_encrypted(playlist: &MediaPlaylist) -> Result<()> {
    let key = playlist
        .segments
        .iter()
        .find_map(|segment| segment.key.as_ref())
        .filter(|key| key.method != KeyMethod::None);
    match key {
        None => Ok(()),
        Some(key) => Err(Error::Custom(format!(
            "HLS 播放列表要求解密（#EXT-X-KEY METHOD={:?}），stream-gears 不支持加密流，请改用 mesio 或 ffmpeg",
            key.method
        ))),
    }
}

/// 下载好的初始化分片：fMP4 的解码配置，等价于 FLV 那边每个文件开头的视频序列头。
struct InitSegment {
    identity: String,
    bytes: Bytes,
}

impl InitSegment {
    /// 下载 `#EXT-X-MAP` 指向的初始化分片；带 `BYTERANGE` 时按字节范围截取。
    async fn load(client: &StatelessClient, media_url: &Url, map: &Map) -> Result<Self> {
        let url = media_url.join(&map.uri)?;
        let bytes = client.retryable(url.as_str()).await?.bytes().await?;
        let bytes = match &map.byte_range {
            Some(range) => {
                let start = range.offset.unwrap_or(0) as usize;
                let end = start.saturating_add(range.length as usize);
                if end > bytes.len() {
                    return Err(Error::Custom(format!(
                        "初始化分片 {url} 的 BYTERANGE {start}..{end} 超出资源长度 {}",
                        bytes.len()
                    )));
                }
                bytes.slice(start..end)
            }
            None => bytes,
        };
        info!("init segment: {url} ({} bytes)", bytes.len());
        Ok(Self {
            identity: map_identity(map),
            bytes,
        })
    }
}

/// 把初始化分片写进当前文件：每个文件都要自带解码配置，缺了它整个文件都放不了。
/// 只在文件要写第一个分片之前调用，免得切段正好落在流末尾时留下一个只有解码配置的空分段。
fn begin_file(ts_file: &mut TsFile<'_>, init: Option<&InitSegment>) -> std::io::Result<()> {
    match init {
        Some(init) => ts_file.write_chunk(&init.bytes),
        None => Ok(()),
    }
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

/// 轮询 m3u8 并按容器把分片追加进分段文件：带 `#EXT-X-MAP` 的 fMP4 存 `.mp4`，其余存 `.ts`。
///
/// fMP4 的初始化分片写在每个文件开头（`#EXT-X-MAP` 变了就另开文件）：它和 FLV 那边的视频序列头
/// 是一回事，缺了它整个文件都放不了。
///
/// `preview` 为这一路录制的预览 hub：容器由播放列表决定，写入端在这里按容器 attach，
/// 每个分片的字节在落盘的同时旁路一份给它；`None` 则不旁路。
pub async fn download(
    url: &str,
    client: &StatelessClient,
    mut file: LifecycleFile<'_>,
    mut splitting: Segmentable,
    preview: Option<&PreviewHub>,
) -> Result<()> {
    info!("Downloading {}...", url);
    let resp = client.retryable(url).await?;
    info!("{}", resp.status());
    // let mut resp = resp.bytes_stream();
    let bytes = resp.bytes().await?;

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
    // 容器由播放列表决定：有 #EXT-X-MAP 的是 fMP4，先下初始化分片，再定扩展名与预览格式。
    let mut init = match map_of(&pl) {
        Some(map) => Some(InitSegment::load(client, &media_url, map).await?),
        None => None,
    };
    let mut container = if init.is_some() {
        HlsContainer::Fmp4
    } else {
        HlsContainer::Ts
    };
    refuse_encrypted(&pl)?;
    file.extension = container.extension();
    let mut preview = preview.map(|hub| hub.attach(container.preview_format()));
    let mut ts_file = TsFile::new(file)?;
    let mut needs_init = init.is_some();
    if let Some(init) = &init
        && let Some(sink) = preview.as_mut()
    {
        // init segment 进快照：新订阅者也从这里起播
        sink.push(ChunkKind::Header, init.bytes.clone());
    }
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
                    needs_init = init.is_some();
                    splitting.reset();
                }
                if needs_init {
                    begin_file(&mut ts_file, init.as_ref())?;
                    needs_init = false;
                }
                let length = download_to_file(
                    media_url.join(&segment.uri)?,
                    client,
                    &mut ts_file,
                    container,
                    preview.as_mut(),
                )
                .await?;
                splitting.increase_size(length);
                splitting.increase_time(segment_duration(segment));
                if splitting.needed() {
                    ts_file.create_new()?;
                    needs_init = init.is_some();
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

        // 刷新后的三种变化：加密、初始化分片变化、容器切换。都在写下一个分片之前处理。
        refuse_encrypted(&pl)?;
        let next_map = map_of(&pl);
        if next_map.map(map_identity) != init.as_ref().map(|init| init.identity.clone()) {
            let next_container = if next_map.is_some() {
                HlsContainer::Fmp4
            } else {
                HlsContainer::Ts
            };
            if next_container != container {
                warn!(
                    from = ?container,
                    to = ?next_container,
                    "HLS 容器变了，按新容器另开文件继续录"
                );
                container = next_container;
                // 扩展名只影响下一次 create()：当前文件的名字已经定下了
                ts_file.file.extension = container.extension();
                if ts_file.pos == 0 {
                    warn!("当前分段还是空的，扩展名从下一个分段开始生效");
                }
                if let Some(sink) = preview.as_mut() {
                    // 预览格式在 attach 时就定死了，中途换容器只能停掉预览；录像不受影响
                    sink.mark_unavailable("HLS 中途换了容器，预览已停止；录像不受影响");
                }
            }
            if ts_file.pos > 0 {
                ts_file.create_new()?;
                splitting.reset();
            }
            init = match next_map {
                Some(map) => {
                    let loaded = InitSegment::load(client, &media_url, map).await?;
                    if let Some(sink) = preview.as_mut() {
                        // 换了初始化分片：订阅者的旧 GOP 不能再和新配置拼在一起，由写入端处理
                        sink.push(ChunkKind::Header, loaded.bytes.clone());
                    }
                    needs_init = true;
                    Some(loaded)
                }
                None => None,
            };
        }
    }
    info!("Done...");
    Ok(())
}

/// 分片是不是收全了：响应声明的长度（没有声明时说不准，按收全算）读满才算。
///
/// TS 按切片追加，读到一半断掉时前面的内容仍可播放；fMP4 不能这样 —— 半个分片会让整个文件的
/// 结尾解不出来，只能整片丢掉。
fn segment_is_complete(received: u64, expected: Option<u64>) -> bool {
    expected.is_none_or(|expected| received >= expected)
}

/// 下载一个分片：TS 边收边落盘，fMP4 收齐一整片再落盘。
///
/// 返回写进文件的字节数；fMP4 分片没收全时返回 0（整片丢弃）。
async fn download_to_file(
    url: Url,
    client: &StatelessClient,
    out: &mut TsFile<'_>,
    container: HlsContainer,
    mut preview: Option<&mut PreviewSink>,
) -> Result<u64> {
    debug!("url: {url}");
    let mut response = client.retryable(url.as_str()).await?;
    let expected = response.content_length();
    let mut length: u64 = 0;
    // 分片起点即预览的关键帧边界（HLS 分片自带 PAT/PMT、从关键帧开始），
    // 新订阅者从最近一个分片的开头起播
    let mut segment_start = true;
    // fMP4 的分片先攒齐：落盘之后再补齐会把整个文件的结尾弄坏
    let mut pending = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if container == HlsContainer::Fmp4 {
            pending.extend_from_slice(&chunk);
            continue;
        }
        length += chunk.len() as u64;
        out.write_chunk(&chunk)?;
        if let Some(sink) = preview.as_deref_mut() {
            if segment_start && chunk.first() != Some(&0x47) {
                // 播放列表按 TS 录，这个分片却不是 TS：没有初始化分片，预览起不来
                sink.mark_unavailable(
                    "HLS 分片不是 MPEG-TS，stream-gears 暂不支持预览此格式，可改用 mesio",
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
    if container == HlsContainer::Fmp4 {
        let received = pending.len() as u64;
        if !segment_is_complete(received, expected) {
            warn!(
                "fMP4 分片只收到 {received} 字节，少于声明的 {expected:?}，整片丢弃（半个分片会让文件解不出来）"
            );
            return Ok(0);
        }
        let bytes = Bytes::from(pending);
        length = bytes.len() as u64;
        out.write_chunk(&bytes)?;
        if let Some(sink) = preview.as_deref_mut() {
            sink.push_fmp4_segment(bytes);
        }
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

/// 分段没正常收尾就被丢弃（断流、出错、录制被停止）时照常收尾：flush、去掉 `.part`、触发钩子。
///
/// **未完成的录像一律保留，不删除**：TS 按切片追加，截断后前面的内容仍可播放，删掉就等于丢掉这一段录像。
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

    /// 分片收全了才写：没读满声明的长度就整片丢掉（半个分片会让整个文件解不出来）。
    #[test]
    fn a_short_fmp4_segment_is_dropped() {
        assert!(super::segment_is_complete(100, None));
        assert!(super::segment_is_complete(100, Some(100)));
        assert!(!super::segment_is_complete(99, Some(100)));
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
    /// 播放列表随每次请求变化（模拟直播刷新）：按顺序给，只剩最后一个时一直给它。
    async fn serve_rotating_playlists(
        playlists: Vec<Vec<u8>>,
        routes: Vec<(&'static str, Vec<u8>)>,
    ) -> String {
        use std::sync::{Arc, Mutex};

        let state = Arc::new(Mutex::new(playlists));
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
        app = app.route(
            "/index.m3u8",
            axum::routing::get(move || {
                let state = state.clone();
                async move {
                    let mut playlists = state.lock().unwrap();
                    if playlists.len() > 1 {
                        playlists.remove(0)
                    } else {
                        playlists.first().cloned().unwrap_or_default()
                    }
                }
            }),
        );
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

    /// 录 `base/index.m3u8`，每个分段文件在钩子里读下来（连同真实扩展名）并接一个预览 hub；
    /// 返回 `download` 的结果、各分段的（扩展名, 内容）与预览状态。
    async fn record_files(
        routes: Vec<(&'static str, Vec<u8>)>,
        splitting: crate::downloader::util::Segmentable,
    ) -> (
        crate::downloader::error::Result<()>,
        Vec<(String, Vec<u8>)>,
        crate::downloader::preview::PreviewStatus,
    ) {
        let base = serve(routes).await;
        record_files_from(&format!("{base}/index.m3u8"), splitting).await
    }

    /// 播放列表由调用方自己起服务（内容随刷新变化）时用这个入口。
    async fn record_files_from(
        playlist_url: &str,
        splitting: crate::downloader::util::Segmentable,
    ) -> (
        crate::downloader::error::Result<()>,
        Vec<(String, Vec<u8>)>,
        crate::downloader::preview::PreviewStatus,
    ) {
        use crate::client::StatelessClient;
        use crate::downloader::preview::PreviewHub;
        use crate::downloader::util::LifecycleFile;
        use std::sync::{Arc, Mutex};

        let dir = tempfile::tempdir().unwrap();
        let kept: Arc<Mutex<Vec<(String, Vec<u8>)>>> = Arc::default();
        let file = LifecycleFile::with_hook(dir.path().join("rec").to_str().unwrap(), "ts", {
            let kept = kept.clone();
            move |name: &str| {
                let mut kept = kept.lock().unwrap();
                // 扩展名由容器决定（调用方给的 "ts" 只是解析播放列表之前的默认值）
                let extension = std::path::Path::new(name)
                    .extension()
                    .map(|ext| ext.to_string_lossy().into_owned())
                    .unwrap_or_default();
                // 钩子跑在 flush 之后：这里读到的就是最终内容，趁临时目录还在读下来
                kept.push((extension, std::fs::read(name).unwrap()));
            }
        });
        let client = StatelessClient::default();
        let hub = PreviewHub::new(4);
        let result = super::download(playlist_url, &client, file, splitting, Some(&hub)).await;
        let status = hub.status();
        let kept = kept.lock().unwrap().clone();
        (result, kept, status)
    }

    /// 同上，只关心 `download` 的结果与各分段内容。
    async fn record(
        routes: Vec<(&'static str, Vec<u8>)>,
        splitting: crate::downloader::util::Segmentable,
    ) -> (crate::downloader::error::Result<()>, Vec<Vec<u8>>) {
        let (result, files, _) = record_files(routes, splitting).await;
        (result, files.into_iter().map(|(_, bytes)| bytes).collect())
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
    /// 一个结构合法的 MP4 box（size + 类型 + 内容）。
    fn mp4_box(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        out
    }

    /// 最小的 fMP4 初始化分片（ftyp + moov/mvex/trex），`marker` 用来区分不同的 init。
    fn fmp4_init(marker: u8) -> Vec<u8> {
        let mut init = mp4_box(b"ftyp", b"iso5");
        init.extend_from_slice(&mp4_box(
            b"moov",
            &mp4_box(b"mvex", &mp4_box(b"trex", &[marker; 8])),
        ));
        init
    }

    /// 一个 fMP4 分片（moof + mdat）。
    fn fmp4_fragment(marker: u8) -> Vec<u8> {
        let mut fragment = mp4_box(b"moof", &[marker; 8]);
        fragment.extend_from_slice(&mp4_box(b"mdat", &[marker; 8]));
        fragment
    }

    /// fMP4 播放列表：init segment 写在文件开头，分片按顺序跟上，产物是 `.mp4`，
    /// 预览按 fMP4 容器接入。
    #[tokio::test]
    async fn an_fmp4_playlist_is_recorded_as_an_mp4_starting_with_its_init_segment() {
        let playlist = b"#EXTM3U\n\
            #EXT-X-TARGETDURATION:2\n\
            #EXT-X-MAP:URI=\"init.mp4\"\n\
            #EXTINF:2.0,\n\
            0.m4s\n\
            #EXTINF:2.0,\n\
            1.m4s\n\
            #EXT-X-ENDLIST\n";
        let init = fmp4_init(0xa1);
        let first = fmp4_fragment(0xb1);
        let second = fmp4_fragment(0xb2);
        let (result, files, status) = record_files(
            vec![
                ("/index.m3u8", playlist.to_vec()),
                ("/init.mp4", init.clone()),
                ("/0.m4s", first.clone()),
                ("/1.m4s", second.clone()),
            ],
            crate::downloader::util::Segmentable::default(),
        )
        .await;
        result.unwrap();
        assert_eq!(files.len(), 1, "{:?}", files.len());
        assert_eq!(files[0].0, "mp4");
        assert_eq!(
            files[0].1,
            [init, first, second].concat(),
            "init segment 必须在文件开头，分片按播放列表顺序跟上"
        );
        assert_eq!(
            status.format,
            Some(crate::downloader::preview::PreviewFormat::Fmp4),
            "fMP4 的预览按 video/mp4 接入"
        );
    }

    /// TS 播放列表不受影响：仍然存 `.ts`，预览仍按 MPEG-TS 接入。
    #[tokio::test]
    async fn a_ts_playlist_still_records_ts() {
        let playlist = b"#EXTM3U\n\
            #EXT-X-TARGETDURATION:2\n\
            #EXTINF:2.0,\n\
            0.ts\n\
            #EXT-X-ENDLIST\n";
        let (result, files, status) = record_files(
            vec![("/index.m3u8", playlist.to_vec()), ("/0.ts", ts_segment(0))],
            crate::downloader::util::Segmentable::default(),
        )
        .await;
        result.unwrap();
        assert_eq!(files[0].0, "ts");
        assert_eq!(files[0].1, ts_segment(0));
        assert_eq!(
            status.format,
            Some(crate::downloader::preview::PreviewFormat::MpegTs)
        );
    }

    /// 分段轮转后每个文件都要自带 init segment：没有它新文件放不了。
    #[tokio::test]
    async fn every_fmp4_segment_file_starts_with_the_init_segment() {
        let playlist = b"#EXTM3U\n\
            #EXT-X-TARGETDURATION:2\n\
            #EXT-X-MEDIA-SEQUENCE:10\n\
            #EXT-X-MAP:URI=\"init.mp4\"\n\
            #EXTINF:1.5,\n\
            10.m4s\n\
            #EXTINF:1.5,\n\
            11.m4s\n\
            #EXTINF:1.5,\n\
            12.m4s\n\
            #EXTINF:1.5,\n\
            13.m4s\n\
            #EXT-X-ENDLIST\n";
        let init = fmp4_init(0xc1);
        let mut routes: Vec<(&'static str, Vec<u8>)> = vec![
            ("/index.m3u8", playlist.to_vec()),
            ("/init.mp4", init.clone()),
        ];
        for (index, name) in ["/10.m4s", "/11.m4s", "/12.m4s", "/13.m4s"]
            .iter()
            .enumerate()
        {
            routes.push((name, fmp4_fragment(0xd0 + index as u8)));
        }
        let (result, files, _) = record_files(
            routes,
            crate::downloader::util::Segmentable::new(Some(Duration::from_secs(3)), None),
        )
        .await;
        result.unwrap();
        assert!(files.len() >= 2, "{} files", files.len());
        for (index, (extension, bytes)) in files.iter().enumerate() {
            assert_eq!(extension, "mp4", "第 {index} 个分段的扩展名");
            if bytes.is_empty() {
                // 切段正好落在流末尾时会留下一个空分段（TS 也是这样），它不该只有一段解码配置
                assert_eq!(index, files.len() - 1, "只有最后那个分段可以是空的");
                continue;
            }
            assert!(
                bytes.starts_with(&init),
                "第 {index} 个分段要以 init segment 开头"
            );
        }
    }

    /// `#EXT-X-MAP` 变了（编码器重启、换了初始化分片）：另开文件，新文件带新的 init。
    #[tokio::test]
    async fn a_changed_init_segment_starts_a_new_file() {
        let first_playlist = b"#EXTM3U\n\
            #EXT-X-TARGETDURATION:1\n\
            #EXT-X-MAP:URI=\"init-a.mp4\"\n\
            #EXTINF:1.0,\n\
            0.m4s\n";
        let second_playlist = b"#EXTM3U\n\
            #EXT-X-TARGETDURATION:1\n\
            #EXT-X-MEDIA-SEQUENCE:1\n\
            #EXT-X-MAP:URI=\"init-b.mp4\"\n\
            #EXTINF:1.0,\n\
            1.m4s\n\
            #EXT-X-ENDLIST\n";
        let init_a = fmp4_init(0xe1);
        let init_b = fmp4_init(0xe2);
        let first = fmp4_fragment(0xf1);
        let second = fmp4_fragment(0xf2);
        let base = serve_rotating_playlists(
            vec![first_playlist.to_vec(), second_playlist.to_vec()],
            vec![
                ("/init-a.mp4", init_a.clone()),
                ("/init-b.mp4", init_b.clone()),
                ("/0.m4s", first.clone()),
                ("/1.m4s", second.clone()),
            ],
        )
        .await;
        let (result, files, _) = record_files_from(
            &format!("{base}/index.m3u8"),
            crate::downloader::util::Segmentable::default(),
        )
        .await;
        result.unwrap();
        assert_eq!(files.len(), 2, "换了 init 要另开文件");
        assert_eq!(files[0].1, [init_a, first].concat(), "第一个文件用旧 init");
        assert_eq!(files[1].1, [init_b, second].concat(), "新文件带新 init");
    }

    /// `#EXT-X-MAP` 带 `BYTERANGE` 时只取那段字节（init 常常是资源里的一段）。
    #[tokio::test]
    async fn the_init_segment_byterange_is_a_slice_of_the_resource() {
        let init = fmp4_init(0x11);
        // 资源里 init 前面还有一段无关字节
        let mut resource = vec![0xff; 4];
        resource.extend_from_slice(&init);
        let playlist = format!(
            "#EXTM3U\n#EXT-X-TARGETDURATION:2\n#EXT-X-MAP:URI=\"init.mp4\",BYTERANGE=\"{}@{}\"\n#EXTINF:2.0,\n0.m4s\n#EXT-X-ENDLIST\n",
            init.len(),
            4
        );
        let fragment = fmp4_fragment(0x12);
        let (result, files, _) = record_files(
            vec![
                ("/index.m3u8", playlist.into_bytes()),
                ("/init.mp4", resource),
                ("/0.m4s", fragment.clone()),
            ],
            crate::downloader::util::Segmentable::default(),
        )
        .await;
        result.unwrap();
        assert_eq!(
            files[0].1,
            [init, fragment].concat(),
            "只取 BYTERANGE 那一段，不含资源里前面的字节"
        );
    }

    /// 加密的播放列表解不开：明确报错，一个字节都不写。
    #[tokio::test]
    async fn an_encrypted_playlist_is_an_error() {
        let playlist = b"#EXTM3U\n\
            #EXT-X-TARGETDURATION:2\n\
            #EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n\
            #EXTINF:2.0,\n\
            0.ts\n\
            #EXT-X-ENDLIST\n";
        let (result, files, _) = record_files(
            vec![("/index.m3u8", playlist.to_vec()), ("/0.ts", ts_segment(0))],
            crate::downloader::util::Segmentable::default(),
        )
        .await;
        let error = result.expect_err("encrypted playlists cannot be recorded");
        assert!(error.to_string().contains("AES128"), "{error}");
        assert!(files.is_empty(), "解不开的流不该落盘");
    }
}
