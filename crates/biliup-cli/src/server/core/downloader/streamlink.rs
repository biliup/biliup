use error_stack::{IntoReport, Result, ResultExt};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use tokio::time::{sleep, Duration};
use url::Url;
use crate::server::errors::{AppError, AppResult};

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

pub struct StreamlinkDownloader {
    platform: Platform,
    url: String,
    headers: HashMap<String, String>,
    output_mode: OutputMode,
    proc: Option<Child>,
}

impl StreamlinkDownloader {
    pub fn new(url: String, platform: Platform) -> Self {
        Self {
            platform,
            url,
            headers: HashMap::new(),
            output_mode: OutputMode::Pipe, // 默认管道模式
            proc: None,
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
    pub async fn start(&mut self) -> AppResult<StreamOutput> {
        let mut cmd = Command::new("streamlink");

        // 基础参数
        cmd.args([
            "--stream-segment-threads", "3",
            "--hls-playlist-reload-attempts", "1",
        ]);

        // 添加HTTP headers
        for (key, value) in &self.headers {
            cmd.args(["--http-header", &format!("{}={}", key, value)]);
        }

        // 平台特定处理
        let platform_args = self.build_platform_args().await?;
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
                    "--player-external-http-port", &port.to_string(),
                    "--player-external-http-interface", "localhost",
                    &self.url,
                    "best",
                ]);

                let mut child = cmd.spawn().change_context(AppError::Unknown)?;

                // 等待HTTP服务器启动
                self.wait_for_startup(&mut child).await?;

                StreamOutput::Http {
                    url: format!("http://localhost:{}", port),
                    process: child,
                }
            }
        };

        Ok(output)
    }

    /// 构建平台特定参数
    async fn build_platform_args(&self) -> AppResult<Vec<String>> {
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
            "uparams", "upsig", "sigparams", "sign",
            "flvsk", "sk", "mid", "site"
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

    /// 等待HTTP服务器启动
    async fn wait_for_startup(&self, child: &mut Child) -> AppResult<()> {
        for _ in 0..5 {
            if child.try_wait().change_context(AppError::Unknown)?.is_some() {
                return Err(AppError::Unknown.into_report());
            }
            sleep(Duration::from_secs(1)).await;
        }
        Ok(())
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
    pub fn get_input_uri(&self) -> String {
        match self {
            StreamOutput::Pipe(_) => "pipe:0".to_string(),
            StreamOutput::Http { url, .. } => url.clone(),
        }
    }

    /// 获取stdout（仅管道模式）
    pub fn take_stdout(&mut self) -> Option<std::process::ChildStdout> {
        match self {
            StreamOutput::Pipe(child) => child.stdout.take(),
            StreamOutput::Http { .. } => None,
        }
    }
}

impl Drop for StreamOutput {
    fn drop(&mut self) {
        let child = match self {
            StreamOutput::Pipe(ref mut c) => c,
            StreamOutput::Http { ref mut process, .. } => process,
        };

        let _ = child.kill(); // 强制终止
        let _ = child.wait(); // 回收资源
    }
}