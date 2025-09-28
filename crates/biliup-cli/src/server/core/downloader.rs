/// FFmpeg下载器实现
pub mod ffmpeg_downloader;
/// Stream-gears下载器实现
pub mod stream_gears;

use crate::server::common::util::Recorder;
use crate::server::errors::AppResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 下载器配置
/// 包含下载过程中需要的各种参数和设置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadConfig {
    /// 分段时长 (格式: "HH:MM:SS")
    pub segment_time: Option<String>,

    /// 分段文件大小限制 (字节)
    pub file_size: Option<u64>,

    /// HTTP请求头
    pub headers: HashMap<String, String>,

    /// 录制器实例
    pub recorder: Recorder,

    /// 输出目录路径
    pub output_dir: PathBuf,
}

impl DownloadConfig {
    /// 生成输出文件名
    ///
    /// # 返回
    /// 返回完整的输出文件路径
    fn generate_output_filename(&self) -> PathBuf {
        self.output_dir.join(self.recorder.generate_path())
    }
}

/// 下载器类型枚举
/// 定义支持的各种下载器类型
#[derive(Debug, Clone, Deserialize, Serialize)]
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
}

#[derive(Debug, Clone)]
pub struct SegmentInfo {
    /// 分段文件路径
    pub prev_file_path: PathBuf,
    pub next_file_path: Option<PathBuf>,
    /// 分段序号
    pub segment_index: usize,
    // /// 分段开始时间戳
    // start_time: std::time::SystemTime,
    // /// 分段结束时间戳
    // end_time: std::time::SystemTime,
}

impl SegmentInfo {
    pub fn new(
        prev_file_path: PathBuf,
        next_file_path: Option<PathBuf>,
        segment_index: usize,
    ) -> Self {
        Self {
            prev_file_path,
            next_file_path,
            segment_index,
        }
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

/// 下载器基础trait
/// 定义所有下载器必须实现的基本接口
#[async_trait]
pub trait Downloader: Send + Sync {
    /// 开始下载
    ///
    /// # 参数
    /// * `callback` - 分段完成时的回调函数
    ///
    /// # 返回
    /// 返回下载状态
    async fn download(
        &self,
        callback: Box<dyn Fn(SegmentEvent) + Send + Sync + 'static>,
    ) -> AppResult<DownloadStatus>;

    /// 停止下载
    async fn stop(&self) -> AppResult<()>;

    /// 滚动保存（用于弹幕等）
    ///
    /// # 参数
    /// * `file_name` - 文件名
    fn rolling(&self, file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
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
