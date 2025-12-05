use crate::server::errors::{AppError, AppResult};
use error_stack::ResultExt;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, ChildStdout, Command};
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{error, info};
use url::Url;

#[derive(Debug, Clone)]
pub enum Platform {
    Bilibili,
    Twitch { disable_ads: bool },
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

    /// 停止下载
    pub(crate) async fn stop(&self) -> AppResult<()> {
        // 仅发出取消信号并更新状态
        // 如果底层下载函数不支持取消，这里不能真正中断正在进行的下载

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

    /// 启动streamlink进程
    pub fn start(&mut self) -> AppResult<StreamOutput> {
        let mut cmd = Command::new("streamlink");

        // 基础参数
        cmd.args([
            "--stream-segment-threads",
            "3",
            "--hls-playlist-reload-attempts",
            "1",
        ]);

        // 添加HTTP headers
        for (key, value) in &self.headers {
            cmd.args(["--http-header", &format!("{}={}", key, value)]);
        }

        // 平台特定处理
        let platform_args = self.build_platform_args()?;
        for arg in platform_args {
            cmd.arg(arg);
        }

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
            Platform::Twitch { disable_ads } => {
                if *disable_ads {
                    args.push("--twitch-disable-ads".to_string());
                }

                // 添加认证token（如果存在）
                if let Some(token) = Self::get_twitch_auth_token() {
                    args.push(format!("--twitch-api-header=Authorization=OAuth {}", token));
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
        let query = url.query().unwrap_or("");

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
        self.wait_for_startup().await;
        match self {
            StreamOutput::Pipe(_) => "pipe:0".to_string(),
            StreamOutput::Http { url, .. } => url.clone(),
        }
    }

    /// 等待HTTP服务器启动
    async fn wait_for_startup(&mut self) {
        let process = match self {
            StreamOutput::Pipe(process) => process,
            StreamOutput::Http { url, process } => process,
        };
        match tokio::time::timeout(Duration::from_secs(5), process.wait()).await {
            Ok(code) => {
                info!("StreamOutput Exited with code {:?}", code);
            }
            Err(e) => {
                error!(e=?e, "Timed out waiting for stream output");
            }
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
