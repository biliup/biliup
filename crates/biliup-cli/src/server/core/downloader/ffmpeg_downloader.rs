use crate::server::core::downloader;
use crate::server::core::downloader::{
    DownloadConfig, DownloadStatus, Downloader, DownloaderType, SegmentEvent, SegmentInfo,
};
use crate::server::errors::{AppError, AppResult};
use async_trait::async_trait;
use error_stack::{ResultExt, bail};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::info;

/// FFmpeg下载器实现
/// 使用FFmpeg进行直播流下载，支持内部和外部分段
pub struct FfmpegDownloader {
    /// 下载配置
    config: DownloadConfig,
    /// 流URL
    url: String,
    /// 下载状态
    status: Arc<RwLock<DownloadStatus>>,
    /// 进程句柄
    process_handle: Arc<RwLock<Option<tokio::process::Child>>>,

    /// 额外的FFmpeg参数
    pub extra_args: Vec<String>,

    /// 下载器类型
    pub downloader_type: DownloaderType,
}

impl FfmpegDownloader {
    /// 创建新的FFmpeg下载器实例
    ///
    /// # 参数
    /// * `url` - 流URL
    /// * `config` - 下载配置
    /// * `extra_args` - 额外的FFmpeg参数
    /// * `downloader_type` - 下载器类型
    pub fn new(
        url: &str,
        config: DownloadConfig,
        extra_args: Vec<String>,
        downloader_type: DownloaderType,
    ) -> Self {
        Self {
            config,
            url: url.to_string(),
            status: Arc::new(RwLock::new(DownloadStatus::Downloading)),
            process_handle: Arc::new(RwLock::new(None)),
            extra_args,
            downloader_type,
        }
    }

    /// 构建内部分段模式的FFmpeg命令参数
    /// 使用FFmpeg的segment muxer进行自动分段
    fn build_ffmpeg_args_internal_segment(&self) -> Vec<String> {
        let mut args = Vec::new();

        // 内部分段使用info级别日志以获取分段信息
        args.extend(["-loglevel".to_string(), "info".to_string()]);

        // 添加通用输入参数
        self.append_common_input_args(&mut args);

        // 内部分段特定的输出参数
        // -f segment: 使用segment muxer进行自动分段
        args.extend(["-f".to_string(), "segment".to_string()]);
        args.extend([
            "-segment_format".to_string(),
            self.config.recorder.suffix.to_string(),
        ]);
        // -segment_list pipe:1: 将分段文件名输出到stdout
        // 这样我们可以实时获取新生成的分段文件
        args.extend(["-segment_list".to_string(), "pipe:1".to_string()]);
        args.extend(["-map".to_string(), "0".to_string()]);

        // -segment_list_type flat: 输出格式为纯文件名列表
        args.extend(["-segment_list_type".to_string(), "flat".to_string()]);

        // -reset_timestamps 1: 每个分段重置时间戳从0开始
        // 确保每个分段文件可以独立播放
        args.extend(["-reset_timestamps".to_string(), "1".to_string()]);
        // %Y-%m-%dT%H_%M_%S 是 strftime 的时间占位符（需要配合 -strftime 1）
        // %d 是序号占位符（printf 风格，默认模式）
        // segment 复用器不能同时用这两种
        args.extend(["-strftime".to_string(), "1".to_string()]);

        // -segment_time: 分段时长（秒）
        if let Some(segment_time) = &self.config.segment_time {
            let seconds = downloader::parse_duration(segment_time);
            args.extend(["-segment_time".to_string(), seconds.to_string()]);
        }

        // 添加通用输出参数
        self.append_common_output_args(&mut args, "segment");

        args
    }

    /// 构建外部分段模式的FFmpeg命令参数
    /// 通过外部控制进行分段，每次录制固定时长或大小
    fn build_ffmpeg_args_external_segment(&self) -> Vec<String> {
        let mut args = Vec::new();

        // 外部分段使用quiet减少日志
        args.extend(["-loglevel".to_string(), "quiet".to_string()]);

        // 添加通用输入参数
        self.append_common_input_args(&mut args);

        // 外部分段特定的输出参数
        // -to: 限制录制时长
        if let Some(segment_time) = &self.config.segment_time {
            args.extend(["-to".to_string(), segment_time.clone()]);
        }

        // -fs: 限制文件大小（字节）
        if let Some(file_size) = self.config.file_size {
            args.extend(["-fs".to_string(), file_size.to_string()]);
        }

        // 添加通用输出参数
        self.append_common_output_args(&mut args, &self.config.recorder.suffix);

        args
    }

    /// 添加通用的输入参数
    /// 包括覆盖文件、HTTP头、超时设置等
    fn append_common_input_args(&self, args: &mut Vec<String>) {
        args.push("-y".to_string()); // 覆盖已存在文件

        // HTTP headers
        // -headers: 设置HTTP请求头，格式为"Key: Value\r\n"
        // 用于传递User-Agent、Cookie等信息
        if !self.config.headers.is_empty() {
            let headers_str = self
                .config
                .headers
                .iter()
                .map(|(k, v)| format!("{}: {}\r\n", k, v))
                .collect::<String>();
            args.extend(["-headers".to_string(), headers_str]);
        }

        // -rw_timeout: 读写超时时间（微秒）
        // 防止网络卡顿导致无限等待
        args.extend(["-rw_timeout".to_string(), "20000000".to_string()]);

        // 对于m3u8流的特殊处理
        if self.url.contains(".m3u8") {
            // -max_reload: HLS播放列表最大重载次数
            // 对于直播流需要设置较大值以持续获取新片段
            args.extend(["-max_reload".to_string(), "1000".to_string()]);
        }

        // 输入URL
        args.extend(["-i".to_string(), self.url.clone()]);
    }

    /// 添加通用的输出参数
    /// 包括编码设置、格式特定参数等
    fn append_common_output_args(&self, args: &mut Vec<String>, format: &str) {
        // -c copy: 直接复制编码，不重新编码
        // 减少CPU使用，保持原始质量
        args.extend(["-c".to_string(), "copy".to_string()]);

        // 格式特定参数
        match format {
            "mp4" => {
                // -bsf:a aac_adtstoasc: 音频比特流过滤器
                // 将ADTS格式的AAC转换为MP4容器所需的格式
                args.extend(["-bsf:a".to_string(), "aac_adtstoasc".to_string()]);

                // -movflags +faststart: 优化MP4用于流媒体播放
                // 将moov atom移到文件开头，允许边下载边播放
                args.extend(["-movflags".to_string(), "+faststart".to_string()]);

                args.extend(["-f".to_string(), "mp4".to_string()]);
            }
            "ts" => {
                args.extend(["-f".to_string(), "mpegts".to_string()]);
            }
            "mkv" => {
                args.extend(["-f".to_string(), "matroska".to_string()]);
            }
            "flv" => {
                args.extend(["-f".to_string(), "flv".to_string()]);
            }
            _ => {}
        }

        // 添加额外参数
        args.extend(self.extra_args.clone());
    }

    /// 执行外部分段下载
    /// 每次录制一个完整的分段文件
    async fn download_external(
        &self,
        callback: Box<dyn Fn(SegmentEvent) + Send + Sync + 'static>,
    ) -> AppResult<DownloadStatus> {
        let args = self.build_ffmpeg_args_external_segment();
        let output_file = self.config.generate_output_filename();

        let mut cmd = Command::new("ffmpeg");
        cmd.args(&args)
            .arg(format!("{}.part", output_file.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child = cmd.spawn().change_context(AppError::Unknown)?;

        // 保存进程句柄
        {
            let mut handle = self.process_handle.write().await;
            *handle = Some(child);
        }

        // 等待进程结束
        let status = {
            let mut handle = self.process_handle.write().await;
            if let Some(mut child) = handle.take() {
                child.wait().await.change_context(AppError::Unknown)?
            } else {
                bail!(AppError::Custom("Process handle not found".to_string()));
            }
        };
        // let (tx, rx) = bounded(16);
        // 分段回调

        // 根据退出码判断状态
        match status.code() {
            Some(0) => {
                // 正常退出，重命名文件
                let part_file = format!("{}.part", output_file.display());
                tokio::fs::rename(&part_file, &output_file)
                    .await
                    .change_context(AppError::Unknown)?;

                // 触发分段回调

                // segment_callback(SegmentEvent {
                //     file_path: output_file,
                //     segment_index: 0,
                //     start_time: std::time::SystemTime::now(),
                //     end_time: std::time::SystemTime::now(),
                // });

                Ok(DownloadStatus::SegmentCompleted)
            }
            Some(255) => Ok(DownloadStatus::StreamEnded),
            err => Ok(DownloadStatus::Error(format!("FFmpeg error: {err:?}"))),
        }
    }

    /// 执行内部分段下载
    /// 使用FFmpeg的segment muxer自动分段
    async fn download_internal(
        &self,
        callback: Box<dyn Fn(SegmentEvent) + Send + Sync + 'static>,
    ) -> AppResult<DownloadStatus> {
        let args = self.build_ffmpeg_args_internal_segment();

        let mut cmd = Command::new("ffmpeg");
        cmd.args(&args)
            .arg(format!(
                "{}.{}.part",
                self.config.recorder.filename_template(),
                self.config.recorder.suffix
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        info!("FFmpeg cmd: {:?}", cmd);
        let mut child = cmd.spawn().change_context(AppError::Unknown)?;

        // 获取stdout用于读取分段文件名
        let stdout = child
            .stdout
            .take()
            .ok_or(AppError::Custom("Failed to capture stdout".to_string()))?;

        let stderr = child.stderr.take().ok_or(AppError::Custom(
            "failed to capture stderr pipe".to_string(),
        ))?;

        // 保存进程句柄
        {
            let mut handle = self.process_handle.write().await;
            *handle = Some(child);
        }

        let mut stderr_lines = BufReader::new(stderr).lines();
        // 将 stderr 打印到当前进程的 stderr
        let stderr_task = tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_lines.next_line().await {
                info!("[ffmpeg] {line}");
            }
        });

        // 异步读取stdout
        let mut reader = BufReader::new(stdout).lines();
        let mut segment_index = 0;
        let mut prev_file_path: Option<PathBuf> = None;

        while let Some(line) = reader.next_line().await.change_context(AppError::Unknown)? {
            // 解析文件名
            let file_path = PathBuf::from(line.trim());

            // 等待文件写入完成
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // 触发分段回调

            // 重命名文件
            let no_ext = file_path.with_extension("");
            tokio::fs::rename(&file_path, &no_ext)
                .await
                .change_context(AppError::Unknown)?;
            info!("renamed file: from {file_path:?} to {no_ext:?}");

            callback(SegmentEvent::Segment(SegmentInfo {
                prev_file_path: no_ext,
                next_file_path: None,
                segment_index,
                // start_time: std::time::SystemTime::now(),
                // end_time: std::time::SystemTime::now(),
            }));

            segment_index += 1;
            prev_file_path = Some(file_path);
        }
        // 确保读任务结束（忽略它们的返回错误以避免因提前关闭管道导致的 join 错）
        let _ = stderr_task.await;

        // 等待进程结束
        let status = {
            let mut handle = self.process_handle.write().await;
            if let Some(mut child) = handle.take() {
                child.wait().await.change_context(AppError::Unknown)?
            } else {
                bail!(AppError::Custom("Process handle not found".to_string()));
            }
        };

        if let Some(file_path) = prev_file_path {
            // 重命名文件
            let no_ext = file_path.with_extension("");
            tokio::fs::rename(&file_path, &no_ext)
                .await
                .change_context(AppError::Unknown)?;
            callback(SegmentEvent::Segment(SegmentInfo {
                prev_file_path: no_ext,
                next_file_path: None,
                segment_index,
                // start_time: std::time::SystemTime::now(),
                // end_time: std::time::SystemTime::now(),
            }));
        }

        // 根据退出码判断状态
        match status.code() {
            Some(0) => {
                // 正常退出
                Ok(DownloadStatus::SegmentCompleted)
            }
            Some(255) => Ok(DownloadStatus::StreamEnded),
            err => Ok(DownloadStatus::Error(format!("FFmpeg error: {err:?}"))),
        }
    }
}

#[async_trait]
impl Downloader for FfmpegDownloader {
    async fn download(
        &self,
        callback: Box<dyn Fn(SegmentEvent) + Send + Sync + 'static>,
    ) -> AppResult<DownloadStatus> {
        match self.downloader_type {
            DownloaderType::FfmpegExternal => self
                .download_external(callback)
                .await
                .change_context(AppError::Unknown),
            DownloaderType::FfmpegInternal => self
                .download_internal(callback)
                .await
                .change_context(AppError::Unknown),
            _ => bail!(AppError::Custom("Unsupported downloader type".to_string())),
        }
    }

    async fn stop(&self) -> AppResult<()> {
        let mut handle = self.process_handle.write().await;
        if let Some(child) = &mut *handle {
            child.kill().await.change_context(AppError::Unknown)?;
            Ok(())
        } else {
            Err(AppError::Custom("Process handle not found".to_string()).into())
        }
    }

    // async fn get_status(&self) -> DownloadStatus {
    //     self.status.read().await.clone()
    // }
}
