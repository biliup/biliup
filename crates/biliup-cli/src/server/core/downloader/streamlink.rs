use crate::server::common::throughput::SubprocessProgress;
use crate::server::common::util::redact_process_debug;
use crate::server::core::downloader::{DownloadConfig, DownloadStatus, SegmentEvent, SegmentInfo};
use crate::server::errors::{AppError, AppResult};
use biliup::downloader::util::ByteCounter;
use error_stack::ResultExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{debug, info};
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

        info!(cmd = %redact_process_debug(&cmd), "Starting streamlink download");
        let child = cmd.spawn().change_context(AppError::Unknown)?;
        callback(SegmentEvent::Start {
            next_file_path: PathBuf::from(&part_file),
        });
        let status = spawn_log(
            child,
            &self.process_handle,
            download_config.bytes_written.clone(),
        )
        .await?;

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
    /// `url` 就是插件解析出的流直链时，每次拉流改用 `DownloadConfig::url`：
    /// 重试会重新解析出新直链（斗鱼网宿的 token 连过一次就作废），且经过了 expire=0 探测
    follow_stream_url: bool,
}

impl StreamlinkDownloader {
    pub fn new(url: String, platform: Platform) -> Self {
        Self {
            platform,
            url,
            headers: HashMap::new(),
            output_mode: OutputMode::Pipe, // 默认管道模式
            follow_stream_url: false,
        }
    }

    pub fn following_stream_url(mut self) -> Self {
        self.follow_stream_url = true;
        self
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
        // 快到录制时间范围结束时会被裁短，使录制停在窗口边界
        if let Some(segment_time) = download_config.segment_duration() {
            args.push("--hls-duration".to_string());
            args.push(segment_time);
        }
        args.push("--force".to_string());
        // stderr 不是终端时也按周期打「[download] Written …」进度行，spawn_log 据此算写盘速率
        args.push("--progress".to_string());
        args.push("force".to_string());
        args.push("--output".to_string());
        args.push(output_file.to_string());
        let url = if self.follow_stream_url {
            &download_config.url
        } else {
            &self.url
        };
        args.push(streamlink_cli_url(url));
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
                let cli_url = streamlink_cli_url(&self.url);
                cmd.args([&cli_url, "best", "-O"]);
                cmd.stdout(Stdio::piped());

                let child = cmd.spawn().change_context(AppError::Unknown)?;
                StreamOutput::Pipe(child)
            }
            OutputMode::HttpServer { port } => {
                let cli_url = streamlink_cli_url(&self.url);
                cmd.args([
                    "--player-external-http",
                    "--player-external-http-port",
                    &port.to_string(),
                    "--player-external-http-interface",
                    "localhost",
                    &cli_url,
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


/// Rewrite already-resolved progressive HTTP(S) media URLs so Streamlink's
/// built-in `http` plugin matches them.
///
/// Streamlink treats a bare positional URL as a *plugin* URL. HLS (`.m3u8`) and
/// DASH (`.mpd`) are auto-detected without a prefix, but progressive HTTP/HTTPS
/// streams (e.g. Huya/Douyu `.flv` CDN links) need an explicit `httpstream://`
/// scheme — otherwise Streamlink exits with `No plugin can handle URL`.
///
/// Webpage URLs that Streamlink plugins already handle (Twitch, YouTube, …) and
/// URLs that already carry a protocol prefix are left unchanged. Detection is
/// based on the media extension, not on platform names.
pub(crate) fn streamlink_cli_url(url: &str) -> String {
    if has_streamlink_protocol_prefix(url) {
        return url.to_string();
    }

    let Ok(parsed) = Url::parse(url) else {
        return url.to_string();
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return url.to_string();
    }

    match media_extension(url).as_deref() {
        // Streamlink auto-detects these without an explicit protocol prefix.
        Some("m3u8") | Some("mpd") => url.to_string(),
        Some(ext) if is_progressive_http_ext(ext) => format!("httpstream://{url}"),
        _ => url.to_string(),
    }
}

fn has_streamlink_protocol_prefix(url: &str) -> bool {
    // Streamlink accepts `protocol://URL` (see cli/protocols.html). Match the
    // known built-in schemes plus any `foo://http(s)://...` form so we never
    // double-prefix.
    const KNOWN: &[&str] = &["httpstream://", "hls://", "dash://"];
    let lower = url.to_ascii_lowercase();
    if KNOWN.iter().any(|p| lower.starts_with(p)) {
        return true;
    }
    if let Some(rest) = lower.split_once("://").map(|(_, r)| r) {
        rest.starts_with("http://") || rest.starts_with("https://")
    } else {
        false
    }
}

fn media_extension(url: &str) -> Option<String> {
    let Ok(parsed) = Url::parse(url) else {
        let before_q = url.split(['?', '#']).next().unwrap_or(url);
        return before_q
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase())
            .filter(|ext| !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric()));
    };
    let seg = parsed.path_segments()?.next_back()?;
    let (_, ext) = seg.rsplit_once('.')?;
    let ext = ext.to_ascii_lowercase();
    if ext.is_empty() || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        None
    } else {
        Some(ext)
    }
}

fn is_progressive_http_ext(ext: &str) -> bool {
    // Formats Streamlink's http plugin is meant to consume as a progressive
    // HTTP body. Keep this list of *container/segment* extensions — not page
    // URLs, and not HLS/DASH manifests (handled separately above).
    matches!(
        ext,
        "flv"
            | "ts"
            | "mp4"
            | "m4v"
            | "m4a"
            | "f4v"
            | "f4a"
            | "aac"
            | "mp3"
            | "mkv"
            | "webm"
            | "ogg"
            | "ogv"
            | "opus"
    )
}

/// 等待 streamlink 结束，期间把 stdout / stderr 转成日志；`--progress=force` 的进度行
/// 只解析不打印，把累计写出字节的增量累加到 `bytes_written`（写盘速率的来源）。
/// `--output` 到文件时 streamlink 把控制台输出（含进度）打在 stdout，两条流都解析。
async fn spawn_log(
    mut child: Child,
    process_handle: &RwLock<Option<Child>>,
    bytes_written: ByteCounter,
) -> AppResult<ExitStatus> {
    let progress = Arc::new(std::sync::Mutex::new(SubprocessProgress::default()));
    let log_or_track = {
        let progress = progress.clone();
        move |line: String| {
            if progress
                .lock()
                .unwrap()
                .observe_streamlink(&line, &bytes_written)
            {
                debug!("[streamlink] {line}");
            } else {
                info!("[streamlink] {line}");
            }
        }
    };

    let mut stderr_task = child.stderr.take().map(|stderr| {
        let mut stderr_lines = BufReader::new(stderr).lines();
        let log_or_track = log_or_track.clone();
        tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_lines.next_line().await {
                log_or_track(line);
            }
        })
    });

    let mut stdout_task = child.stdout.take().map(|stdout| {
        let mut stdout_lines = BufReader::new(stdout).lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_lines.next_line().await {
                log_or_track(line);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::common::util::Recorder;
    use crate::server::infrastructure::models::StreamerInfo;

    fn download_config(url: &str) -> DownloadConfig {
        DownloadConfig {
            url: url.to_string(),
            segment_time: None,
            time_range: None,
            file_size: None,
            headers: HashMap::new(),
            recorder: Recorder::new(
                None,
                StreamerInfo::new("t", "u", "title", chrono::Utc::now(), ""),
            ),
            output_dir: ".".into(),
            suffix: "flv".to_string(),
            bytes_written: ByteCounter::new(),
            preview: Default::default(),
        }
    }

    #[test]
    fn stream_url_runtime_follows_each_attempts_url() {
        let first = "https://ws1a.douyucdn.cn/live/a.flv?token=first&expire=300&fcdn=ws&expire=0";
        let fresh = "https://ws1a.douyucdn.cn/live/a.flv?token=fresh&expire=300&fcdn=ws";
        let args = |downloader: StreamlinkDownloader| {
            downloader
                .build_file_args(&download_config(fresh), "out.flv.part")
                .unwrap()
        };

        let following = args(
            StreamlinkDownloader::new(first.to_string(), Platform::Generic).following_stream_url(),
        );
        assert!(following.contains(&streamlink_cli_url(fresh)));
        assert!(!following.iter().any(|arg| arg.contains("token=first")));

        let pinned = args(StreamlinkDownloader::new(
            first.to_string(),
            Platform::Generic,
        ));
        assert!(pinned.contains(&streamlink_cli_url(first)));
    }

    #[test]
    fn progressive_flv_gets_httpstream_prefix() {
        let url = "https://cdn.example/live/abc.flv?wsSecret=sig&wsTime=1";
        assert_eq!(
            streamlink_cli_url(url),
            format!("httpstream://{url}")
        );
    }

    #[test]
    fn progressive_mp4_and_ts_get_httpstream_prefix() {
        assert_eq!(
            streamlink_cli_url("http://cdn.example/a.mp4"),
            "httpstream://http://cdn.example/a.mp4"
        );
        assert_eq!(
            streamlink_cli_url("https://cdn.example/a.ts?token=1"),
            "httpstream://https://cdn.example/a.ts?token=1"
        );
    }

    #[test]
    fn hls_and_dash_manifests_are_left_unchanged() {
        assert_eq!(
            streamlink_cli_url("https://cdn.example/live.m3u8?token=1"),
            "https://cdn.example/live.m3u8?token=1"
        );
        assert_eq!(
            streamlink_cli_url("https://cdn.example/manifest.mpd"),
            "https://cdn.example/manifest.mpd"
        );
    }

    #[test]
    fn existing_protocol_prefix_is_not_doubled() {
        let url = "httpstream://https://cdn.example/live.flv";
        assert_eq!(streamlink_cli_url(url), url);
        let hls = "hls://https://cdn.example/live.m3u8";
        assert_eq!(streamlink_cli_url(hls), hls);
    }

    #[test]
    fn webpage_urls_without_media_extension_are_left_unchanged() {
        assert_eq!(
            streamlink_cli_url("https://www.twitch.tv/example"),
            "https://www.twitch.tv/example"
        );
        assert_eq!(
            streamlink_cli_url("https://www.youtube.com/watch?v=abc"),
            "https://www.youtube.com/watch?v=abc"
        );
    }

    #[test]
    fn build_file_args_uses_httpstream_for_flv() {
        let downloader = StreamlinkDownloader::new(
            "https://cdn.example/live.flv?sig=1".to_string(),
            Platform::Generic,
        );
        let args = downloader
            .build_file_args(
                &DownloadConfig {
                    suffix: "flv".to_string(),
                    ..Default::default()
                },
                "/tmp/out.flv.part",
            )
            .expect("args");
        assert!(
            args.iter().any(|a| a == "httpstream://https://cdn.example/live.flv?sig=1"),
            "expected httpstream URL in args: {args:?}"
        );
        assert!(args.iter().any(|a| a == "best"));
        // 写盘速率靠解析进度行，stderr 不是终端时也要让 streamlink 打出来
        assert!(
            args.windows(2)
                .any(|w| w[0] == "--progress" && w[1] == "force"),
            "expected --progress force in args: {args:?}"
        );
    }
}
