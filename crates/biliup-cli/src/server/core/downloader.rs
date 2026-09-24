pub mod cover_downloader;
/// FFmpeg下载器实现
pub mod ffmpeg_downloader;
/// mesio（rust-srec）进程内下载 + 修复管线
pub mod mesio;
/// Stream-gears下载器实现
pub mod stream_gears;
pub mod streamlink;
/// 边录边传（零落盘流式上传）
pub mod sync_downloader;
pub mod ws_expire;
pub mod ytdlp;

use crate::server::common::timerange;
use crate::server::common::util::Recorder;
use crate::server::core::downloader::ffmpeg_downloader::FfmpegDownloader;
use crate::server::core::downloader::mesio::Mesio;
use crate::server::core::downloader::stream_gears::StreamGears;
use crate::server::core::downloader::streamlink::Streamlink;
use crate::server::core::downloader::sync_downloader::SyncDownloader;
use crate::server::core::downloader::ytdlp::YouTubeDownloader;
use crate::server::errors::{AppError, AppResult};
use async_trait::async_trait;
use biliup::downloader::preview::PreviewHub;
use biliup::downloader::util::ByteCounter;
use danmaku_client::{DanmakuRecorder, RecorderConfig, RecorderHandle};
use error_stack::Report;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::warn;

/// 下载器配置
/// 包含下载过程中需要的各种参数和设置
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DownloadConfig {
    /// 流 URL
    pub(crate) url: String,
    /// 分段时长 (格式: "HH:MM:SS")
    pub segment_time: Option<String>,

    /// 录制时间范围，两个 ISO 8601 时刻的 JSON 数组字符串。
    /// 见 [`crate::server::common::timerange`]。
    pub time_range: Option<String>,

    /// 分段文件大小限制 (字节)
    pub file_size: Option<u64>,

    /// HTTP请求头
    pub headers: HashMap<String, String>,

    /// 录制器实例
    pub recorder: Recorder,

    /// 输出目录路径
    pub output_dir: PathBuf,

    pub suffix: String,

    /// 本次录制任务的写盘字节累计，由各下载器在写出点累加，供界面显示实时速率。
    /// 只是一个原子计数器的句柄：不参与序列化，也不影响任何下载控制流。
    #[serde(skip)]
    pub bytes_written: ByteCounter,

    /// 本次录制任务的直播预览 hub。进程内写盘的下载器（stream-gears / mesio）开始拉流时
    /// 从它 `attach` 一个写入端，在写盘点旁边旁路媒体字节；其它下载器不碰它。
    /// 与 `bytes_written` 一样只是句柄，不参与序列化，不影响任何下载控制流。
    #[serde(skip)]
    pub preview: PreviewHub,
}

impl DownloadConfig {
    /// 生成输出文件名
    ///
    /// # 返回
    /// 返回完整的输出文件路径
    fn generate_output_filename(&self, suffix: &str) -> PathBuf {
        self.output_dir.join(self.recorder.generate_path(suffix))
    }

    /// 本次录制块允许的最长时长（`"HH:MM:SS"`）。
    ///
    /// 即 `segment_time` 按录制时间范围的结束时刻裁剪后的结果：快到窗口结束时缩短本段，
    /// 让录制停在窗口边界。各下载器都以「本段录多久」的语义使用它，
    /// 因此必须用这个方法而不是直接读 [`Self::segment_time`]。
    pub fn segment_duration(&self) -> Option<String> {
        timerange::clamp_segment_time(self.segment_time.as_deref(), self.time_range.as_deref())
    }

    /// 距录制时间范围结束还剩多久（`"HH:MM:SS"`）；未配置录制时间范围时为 `None`。
    pub fn time_range_remaining(&self) -> Option<String> {
        timerange::remaining_until_end(self.time_range.as_deref())
    }
}

/// 下载器类型枚举
/// 定义支持的各种下载器类型
#[derive(PartialEq, Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DownloaderType {
    /// Ytarchive下载器
    Ytarchive,
    /// 同步下载器
    #[serde(rename = "sync-downloader")]
    SyncDownloader,
    /// 使用stream-gears
    #[serde(rename = "stream-gears")]
    StreamGears,
    /// FFmpeg下载器
    Ffmpeg,
    /// FFmpeg外部分段
    FfmpegExternal,
    /// FFmpeg内部分段
    FfmpegInternal,
    /// Streamlink下载器
    Streamlink,
    /// yt-dlp下载器
    YtDlp,
    /// mesio（rust-srec）：进程内 FLV/HLS 下载与修复管线
    Mesio,
}

/// 实际的下载器枚举（包含实例）
pub enum DownloaderRuntime {
    Ffmpeg(FfmpegDownloader),
    StreamGears(StreamGears),
    StreamLink(Streamlink),
    YtDlp(YouTubeDownloader),
    Sync(SyncDownloader),
    Mesio(Mesio),
}

impl DownloaderRuntime {
    /// 从配置创建
    pub fn from_type(downloader_type: DownloaderType) -> Self {
        match downloader_type {
            // `ffmpeg-internal` 的 segment muxer 实现从未接通过，与 `ffmpeg` / `ffmpeg-external`
            // 一样按外部分段跑，至少不再静默换成 stream-gears
            DownloaderType::Ffmpeg
            | DownloaderType::FfmpegExternal
            | DownloaderType::FfmpegInternal => Self::Ffmpeg(FfmpegDownloader::new(
                Vec::new(),
                DownloaderType::FfmpegExternal,
            )),
            DownloaderType::SyncDownloader => Self::Sync(SyncDownloader::new()),
            DownloaderType::Mesio => Self::Mesio(Mesio::new()),
            DownloaderType::StreamGears => Self::StreamGears(StreamGears::new(None)),
            // 这三种要用到直播流里的参数，由 `core::live::downloader_runtime` 构造，走不到这里
            DownloaderType::Streamlink | DownloaderType::YtDlp | DownloaderType::Ytarchive => {
                warn!(
                    ?downloader_type,
                    "this downloader needs the live stream to be built, using stream-gears"
                );
                Self::StreamGears(StreamGears::new(None))
            }
        }
    }

    pub async fn download<'a>(
        &self,
        callback: Box<dyn FnMut(SegmentEvent) + Send + Sync + 'a>,
        download_config: DownloadConfig,
    ) -> AppResult<DownloadStatus> {
        match self {
            Self::Ffmpeg(d) => d.download(callback, download_config).await,
            Self::StreamGears(d) => d.download(callback, download_config).await,
            DownloaderRuntime::StreamLink(d) => d.download(callback, download_config).await,
            Self::YtDlp(d) => d.download(callback, download_config).await,
            Self::Mesio(d) => d.download(callback, download_config).await,
            Self::Sync(_) => Err(AppError::Custom(
                "sync-downloader 应走边录边传专用流程，而不是落盘分段回调".into(),
            )
            .into()),
        }
    }

    pub async fn stop(&self) -> AppResult<()> {
        match self {
            Self::Ffmpeg(d) => d.stop().await,
            Self::StreamGears(d) => d.stop().await,
            DownloaderRuntime::StreamLink(d) => d.stop().await,
            Self::YtDlp(d) => d.stop().await,
            Self::Sync(d) => d.stop().await,
            Self::Mesio(d) => d.stop().await,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SegmentInfo {
    /// 分段文件路径
    pub prev_file_path: PathBuf,
    pub danmaku_file_path: Option<PathBuf>,
    pub next_file_path: Option<PathBuf>,
    /// 分段序号
    pub segment_index: usize,
    /// 分段时长（秒）。下载器报告了才有，目前只有 mesio
    pub duration_secs: Option<f64>,
    /// 分段文件的字节数。下载器报告了才有，目前只有 mesio
    pub size_bytes: Option<u64>,
    // /// 分段开始时间戳
    // start_time: std::time::SystemTime,
    // /// 分段结束时间戳
    // end_time: std::time::SystemTime,
}

impl SegmentInfo {
    pub fn new(
        prev_file_path: PathBuf,
        danmaku_file_path: Option<PathBuf>,
        next_file_path: Option<PathBuf>,
        segment_index: usize,
    ) -> Self {
        Self {
            prev_file_path,
            danmaku_file_path,
            next_file_path,
            segment_index,
            duration_secs: None,
            size_bytes: None,
        }
    }

    /// 附上下载器报告的分段时长与字节数。
    pub fn with_stats(mut self, duration_secs: f64, size_bytes: u64) -> Self {
        self.duration_secs = Some(duration_secs);
        self.size_bytes = Some(size_bytes);
        self
    }
}

/// 分段事件
/// 当下载器完成一个分段时触发的事件
#[derive(Debug, Clone)]
pub enum SegmentEvent {
    Start {
        /// 分段文件路径
        next_file_path: PathBuf,
    },
    Segment(SegmentInfo),
}

/// 下载状态
/// 表示下载器当前的状态
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    /// 正在下载
    Downloading,
    /// 正常分段（外部分段触发）
    SegmentCompleted,
    /// 直播流结束
    StreamEnded,
    /// 错误
    Error(String),
}

#[async_trait]
// 弹幕客户端 (需要根据实际情况实现)
pub trait DanmakuClient {
    /// Starts danmaku recording and manages lifecycle
    async fn download(&self) -> AppResult<()>;

    async fn stop(&self) -> AppResult<()>;
    /// 滚动保存（用于弹幕等）
    ///
    /// # 参数
    /// * `file_name` - 文件名
    ///
    /// 返回 true 表示本次滚动保存产生了可交给后处理的弹幕文件。
    fn rolling(&self, _file_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
}

pub struct RustDanmakuClient {
    config: RecorderConfig,
    handle: Mutex<Option<RecorderHandle>>,
}

impl RustDanmakuClient {
    pub fn new(config: RecorderConfig) -> Self {
        Self {
            config,
            handle: Mutex::new(None),
        }
    }
}

#[async_trait]
impl DanmakuClient for RustDanmakuClient {
    async fn download(&self) -> AppResult<()> {
        let mut handle = self.handle.lock().unwrap();
        if handle.is_some() {
            return Ok(());
        }

        let recorder = DanmakuRecorder::new(self.config.clone())
            .map_err(|e| Report::new(AppError::Custom(e.to_string())))?;
        *handle = Some(recorder.start());
        Ok(())
    }

    async fn stop(&self) -> AppResult<()> {
        let handle = self.handle.lock().unwrap().take();
        if let Some(handle) = handle {
            handle
                .stop()
                .await
                .map_err(|e| Report::new(AppError::Custom(e.to_string())))?;
        }
        Ok(())
    }

    fn rolling(&self, file_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let handle = self
            .handle
            .lock()
            .map_err(|_| "danmaku handle lock poisoned")?
            .clone();
        if let Some(handle) = handle {
            return tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(handle.rolling(Some(PathBuf::from(file_name))))
            })
            .map_err(Into::into);
        }
        Ok(false)
    }
}

/// 解析时长字符串 "HH:MM:SS" 为秒数
///
/// # 参数
/// * `duration` - 时长字符串，格式为"HH:MM:SS"
///
/// # 返回
/// 返回总秒数
fn parse_duration(duration: &str) -> u64 {
    let parts: Vec<&str> = duration.split(':').collect();
    if parts.len() == 3 {
        let hours: u64 = parts[0].parse().unwrap_or(0);
        let minutes: u64 = parts[1].parse().unwrap_or(0);
        let seconds: u64 = parts[2].parse().unwrap_or(0);
        hours * 3600 + minutes * 60 + seconds
    } else {
        0
    }
}

// 使用示例
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let config = DownloadConfig {
//         format: "mp4".to_string(),
//         segment_time: Some("01:00:00".to_string()),
//         file_size: Some(2 * 1024 * 1024 * 1024), // 2GB
//         headers: HashMap::from([("User-Agent".to_string(), "Mozilla/5.0".to_string())]),
//         extra_args: vec![],
//         downloader_type: DownloaderType::FfmpegInternal,
//         filename_prefix: "stream".to_string(),
//     };
//
//     // 分段回调
//     let segment_callback = Arc::new(|event: SegmentEvent| {
//         println!("New segment: {:?}", event.file_path);
//         // 这里可以触发上传等后续处理
//     });
//
//     let downloader = FfmpegDownloader::new(
//         "http://example.com/stream.m3u8".to_string(),
//         config,
//         PathBuf::from("./downloads"),
//         Some(segment_callback),
//     );
//
//     // 检查流
//     // if downloader.check_stream().await? {
//     //     // 开始下载
//     //     let status = downloader.download().await?;
//     //     println!("Download completed with status: {:?}", status);
//     // }
//
//     Ok(())
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_downloader_is_not_silently_mapped_to_stream_gears() {
        let runtime = DownloaderRuntime::from_type(DownloaderType::SyncDownloader);
        assert!(
            matches!(runtime, DownloaderRuntime::Sync(_)),
            "选择 sync-downloader 必须走边录边传，不能再落到 stream-gears 落盘"
        );
        let gears = DownloaderRuntime::from_type(DownloaderType::StreamGears);
        assert!(matches!(gears, DownloaderRuntime::StreamGears(_)));
    }

    #[test]
    fn ffmpeg_variants_are_not_silently_mapped_to_stream_gears() {
        for configured in ["\"ffmpeg\"", "\"ffmpeg-external\"", "\"ffmpeg-internal\""] {
            let downloader_type: DownloaderType = serde_json::from_str(configured).unwrap();
            match DownloaderRuntime::from_type(downloader_type) {
                DownloaderRuntime::Ffmpeg(ffmpeg) => {
                    assert_eq!(ffmpeg.downloader_type, DownloaderType::FfmpegExternal)
                }
                _ => panic!("{configured} must run ffmpeg"),
            }
        }
    }

    #[test]
    fn mesio_maps_to_its_own_runtime_and_kebab_case_name() {
        let runtime = DownloaderRuntime::from_type(DownloaderType::Mesio);
        assert!(matches!(runtime, DownloaderRuntime::Mesio(_)));
        assert_eq!(
            serde_json::to_string(&DownloaderType::Mesio).unwrap(),
            "\"mesio\""
        );
        assert_eq!(
            serde_json::from_str::<DownloaderType>("\"mesio\"").unwrap(),
            DownloaderType::Mesio
        );
    }
}
