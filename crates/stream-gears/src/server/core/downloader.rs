pub mod ffmpeg_downloader;
pub mod stream_gears;

use crate::server::core::download_manager::UploaderMessage;
use crate::server::infrastructure::context::Worker;
use async_channel::Sender;
use async_trait::async_trait;
use ffmpeg_downloader::FfmpegDownloader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;

/// 下载器配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadConfig {
    /// 录制后保存文件格式 (mp4, ts, mkv, flv)
    pub format: String,

    /// 分段时长 (格式: "HH:MM:SS")
    pub segment_time: Option<String>,

    /// 分段文件大小限制 (字节)
    pub file_size: Option<u64>,

    /// HTTP请求头
    pub headers: HashMap<String, String>,

    /// 额外的FFmpeg参数
    pub extra_args: Vec<String>,

    /// 下载器类型
    pub downloader_type: DownloaderType,

    /// 输出文件名前缀
    pub filename_prefix: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DownloaderType {
    Ytarchive,
    #[serde(rename = "sync-downloader")]
    SyncDownloader,
    /// 使用stream-gears
    #[serde(rename = "stream-gears")]
    StreamGears,
    Ffmpeg,
    /// FFmpeg外部分段
    FfmpegExternal,
    /// FFmpeg内部分段
    FfmpegInternal,
    /// Streamlink
    Streamlink,
    /// yt-dlp
    YtDlp,
}

/// 分段事件
#[derive(Debug, Clone)]
pub struct SegmentEvent {
    /// 分段文件路径
    pub file_path: PathBuf,
    /// 分段序号
    pub segment_index: usize,
    /// 分段开始时间戳
    pub start_time: std::time::SystemTime,
    /// 分段结束时间戳
    pub end_time: std::time::SystemTime,
}

/// 下载状态
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
#[async_trait]
pub trait Downloader: Send + Sync {
    /// 开始下载
    async fn download(
        &self,
        sender: Sender<UploaderMessage>,
        worker: Arc<Worker>,
    ) -> Result<DownloadStatus, Box<dyn std::error::Error>>;

    /// 停止下载
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>>;

    /// 获取当前下载状态
    async fn get_status(&self) -> DownloadStatus;
}

/// 解析时长字符串 "HH:MM:SS" 为秒数
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
