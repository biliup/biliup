use crate::server::core::downloader::{DownloadConfig, DownloadStatus, SegmentEvent, SegmentInfo};
use crate::server::errors::{AppError, AppResult};
use error_stack::ResultExt;
use std::collections::HashMap;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::info;
use url::Url;

#[derive(Debug, Clone)]
pub enum Platform {
    Bilibili,
    Twitch {
        disable_ads: bool,
        auth_token: Option<String>,
    },
    Niconico {
        email: Option<String>,
        password: Option<String>,
        user_session: Option<String>,
        purge_credentials: Option<String>,
    },
    Generic,
}

#[derive(Debug, Clone)]
pub enum OutputMode {
    /// 管道模式：streamlink输出到stdout，由父进程读取
    Pipe,
    /// HTTP服务器模式：streamlink启动本地HTTP服务器
    HttpServer { port: u16 },
}

pub struct Streamlink {
    streamlink_downloader: StreamlinkDownloader,
    /// 进程句柄
    process_handle: Arc<RwLock<Option<Child>>>,
}

impl Streamlink {
    pub fn new(streamlink_downloader: StreamlinkDownloader) -> Streamlink {
        Self {
            streamlink_downloader,
            process_handle: Arc::new(RwLock::new(None)),
        }
    }

    pub(crate) async fn download<'a>(
        &self,
        mut callback: Box<dyn FnMut(SegmentEvent) + Send + Sync + 'a>,
        download_config: DownloadConfig,
    ) -> AppResult<DownloadStatus> {
        let output_file = download_config.generate_output_filename(&download_config.suffix);
        let part_file = format!("{}.part", output_file.display());
        let args = self
            .streamlink_downloader
            .build_file_args(&download_config, &part_file)?;

        let mut cmd = Command::new("streamlink");
        cmd.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        info!(cmd = ?cmd, "Starting streamlink download");
        let child = cmd.spawn().change_context(AppError::Unknown)?;
        let status = spawn_log(child, &self.process_handle).await?;

        if tokio::fs::try_exists(&part_file)
            .await
            .change_context(AppError::Unknown)?
        {
            tokio::fs::rename(&part_file, &output_file)
                .await
                .change_context(AppError::Custom(String::from("退出时，重命名文件")))?;
            callback(SegmentEvent::Segment(SegmentInfo::new(
                output_file,
                None,
                None,
                0,
            )));
        }

        match status.code() {
            Some(0) => Ok(DownloadStatus::SegmentCompleted),
            Some(130) | Some(143) | Some(255) => Ok(DownloadStatus::StreamEnded),
            err => Ok(DownloadStatus::Error(format!("Streamlink error: {err:?}"))),
        }
    }

    /// 停止下载
    pub(crate) async fn stop(&self) -> AppResult<()> {
        let mut handle = self.process_handle.write().await;
        if let Some(child) = &mut *handle {
            child.kill().await.change_context(AppError::Unknown)?;
        }
        Ok(())
    }
}

pub struct StreamlinkDownloader {
    platform: Platform,
    url: String,
    headers: HashMap<String, String>,
    output_mode: OutputMode,
}

impl StreamlinkDownloader {
    pub fn new(url: String, platform: Platform) -> Self {
        Self {
            platform,
            url,
            headers: HashMap::new(),
            output_mode: OutputMode::Pipe, // 默认管道模式
        }
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_output_mode(mut self, mode: OutputMode) -> Self {
        self.output_mode = mode;
        self
    }

    fn build_base_args(&self) -> AppResult<Vec<String>> {
        let mut args = vec![
            "--stream-segment-threads".to_string(),
            "3".to_string(),
            "--hls-playlist-reload-attempts".to_string(),
            "1".to_string(),
        ];

        for (key, value) in &self.headers {
            args.push("--http-header".to_string());
            args.push(format!("{}={}", key, value));
        }

        args.extend(self.build_platform_args()?);
        Ok(args)
    }

    fn build_file_args(
        &self,
        download_config: &DownloadConfig,
        output_file: &str,
    ) -> AppResult<Vec<String>> {
        let mut args = self.build_base_args()?;
        for (key, value) in &download_config.headers {
            args.push("--http-header".to_string());
            args.push(format!("{}={}", key, value));
        }
        if let Some(segment_time) = &download_config.segment_time {
            args.push("--hls-duration".to_string());
            args.push(segment_time.clone());
        }
        args.push("--force".to_string());
        args.push("--output".to_string());
        args.push(output_file.to_string());
        args.push(self.url.clone());
        args.push("best".to_string());
        Ok(args)
    }

    /// 启动streamlink进程
    pub fn start(&mut self) -> AppResult<StreamOutput> {
        let mut cmd = Command::new("streamlink");

        cmd.args(self.build_base_args()?);

        // 配置输出模式
        let output = match &self.output_mode {
            OutputMode::Pipe => {
                cmd.args([&self.url, "best", "-O"]);
                cmd.stdout(Stdio::piped());

                let child = cmd.spawn().change_context(AppError::Unknown)?;
                StreamOutput::Pipe(child)
            }
            OutputMode::HttpServer { port } => {
                cmd.args([
                    "--player-external-http",
                    "--player-external-http-port",
                    &port.to_string(),
                    "--player-external-http-interface",
                    "localhost",
                    &self.url,
                    "best",
                ]);

                let child = cmd.spawn().change_context(AppError::Unknown)?;

                StreamOutput::Http {
                    url: format!("http://localhost:{}", port),
                    process: child,
                }
            }
        };

        Ok(output)
    }

    /// 构建平台特定参数
    fn build_platform_args(&self) -> AppResult<Vec<String>> {
        let mut args = Vec::new();

        match &self.platform {
            Platform::Bilibili => {
                // Bilibili需要保留特定URL参数，否则segment请求会404
                args.extend(self.parse_bilibili_params()?);
            }
            Platform::Twitch {
                disable_ads,
                auth_token,
            } => {
                if *disable_ads {
                    args.push("--twitch-disable-ads".to_string());
                }

                let token = auth_token.clone().or_else(Self::get_twitch_auth_token);
                if let Some(token) = token {
                    args.push(format!("--twitch-api-header=Authorization=OAuth {}", token));
                }
            }
            Platform::Niconico {
                email,
                password,
                user_session,
                purge_credentials,
            } => {
                if let Some(email) = email.as_deref().filter(|value| !value.is_empty()) {
                    args.push("--niconico-email".to_string());
                    args.push(email.to_string());
                }
                if let Some(password) = password.as_deref().filter(|value| !value.is_empty()) {
                    args.push("--niconico-password".to_string());
                    args.push(password.to_string());
                }
                if let Some(user_session) =
                    user_session.as_deref().filter(|value| !value.is_empty())
                {
                    args.push("--niconico-user-session".to_string());
                    args.push(user_session.to_string());
                }
                if let Some(purge_credentials) = purge_credentials
                    .as_deref()
                    .filter(|value| !value.is_empty())
                {
                    args.push("--niconico-purge-credentials".to_string());
                    args.push(purge_credentials.to_string());
                }
            }
            Platform::Generic => {}
        }

        Ok(args)
    }

    /// 解析Bilibili URL参数（白名单过滤）
    fn parse_bilibili_params(&self) -> AppResult<Vec<String>> {
        let mut params = Vec::new();

        let url = Url::parse(&self.url).change_context(AppError::Unknown)?;
        // 白名单参数
        let mut whitelist = vec![
            "uparams",
            "upsig",
            "sigparams",
            "sign",
            "flvsk",
            "sk",
            "mid",
            "site",
        ];

        // 动态扩展白名单
        let query_pairs: HashMap<_, _> = url.query_pairs().collect();

        if let Some(sigparams) = query_pairs.get("sigparams") {
            whitelist.extend(sigparams.split(',').map(|s| s.trim()));
        }
        if let Some(uparams) = query_pairs.get("uparams") {
            whitelist.extend(uparams.split(',').map(|s| s.trim()));
        }

        // 过滤参数
        for (key, value) in url.query_pairs() {
            if whitelist.contains(&key.as_ref()) {
                params.push("--http-query-param".to_string());
                params.push(format!("{}={}", key, value));
            }
        }

        Ok(params)
    }

    fn get_twitch_auth_token() -> Option<String> {
        // 从配置文件或环境变量读取
        std::env::var("TWITCH_AUTH_TOKEN").ok()
    }
}

/// Streamlink输出类型
pub enum StreamOutput {
    /// 管道输出（直接读取stdout）
    Pipe(Child),
    /// HTTP服务器输出
    Http { url: String, process: Child },
}

impl StreamOutput {
    /// 获取可读的输入源（用于FFmpeg等）
    pub async fn get_input_uri(&mut self) -> String {
        match self {
            StreamOutput::Pipe(_) => "pipe:0".to_string(),
            StreamOutput::Http { url, .. } => url.clone(),
        }
    }

    /// 获取stdout（仅管道模式）
    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        match self {
            StreamOutput::Pipe(child) => child.stdout.take(),
            StreamOutput::Http { .. } => None,
        }
    }

    pub async fn stop(&mut self) {
        info!("准备停止stream terminated");
        let child = match self {
            StreamOutput::Pipe(c) => c,
            StreamOutput::Http { process, .. } => process,
        };

        let _ = child.kill().await; // 强制终止
        let _ = child.wait().await; // 回收资源
        info!("成功stream terminated");
    }
}

async fn spawn_log(
    mut child: Child,
    process_handle: &RwLock<Option<Child>>,
) -> AppResult<ExitStatus> {
    let mut stderr_task = child.stderr.take().map(|stderr| {
        let mut stderr_lines = BufReader::new(stderr).lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_lines.next_line().await {
                info!("[streamlink] {line}");
            }
        })
    });

    let mut stdout_task = child.stdout.take().map(|stdout| {
        let mut stdout_lines = BufReader::new(stdout).lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_lines.next_line().await {
                info!("[streamlink] {line}");
            }
        })
    });

    {
        let mut handle = process_handle.write().await;
        *handle = Some(child);
    }

    let status = loop {
        {
            let mut handle = process_handle.write().await;
            let Some(child) = handle.as_mut() else {
                return Err(AppError::Custom("Process handle not found".to_string()).into());
            };
            if let Some(status) = child.try_wait().change_context(AppError::Unknown)? {
                *handle = None;
                break status;
            }
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    };

    if let Some(task) = stderr_task.take() {
        let _ = task.await;
    }
    if let Some(task) = stdout_task.take() {
        let _ = task.await;
    }

    Ok(status)
}
