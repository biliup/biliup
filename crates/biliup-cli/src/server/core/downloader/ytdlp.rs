use crate::server::common::util::{FileValidator, redact_process_debug};
use crate::server::core::downloader::{
    DownloadConfig as RuntimeDownloadConfig, DownloadStatus, SegmentEvent, SegmentInfo,
};
use crate::server::errors::{AppError, AppResult};
use crate::tools;
use error_stack::{ResultExt, bail};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{fs, time::timeout};
use tracing::{info, warn};

#[derive(Clone, Debug)]
pub enum Backend {
    YtDlp,
    YtArchive,
}

#[derive(Clone, Debug)]
pub struct DownloadConfig {
    // URLs
    pub webpage_url: String,
    pub download_url: Option<String>,

    // 输出命名
    pub filename: String,
    pub suffix: String,
    pub working_dir: PathBuf,

    // 模式
    pub backend: Backend,
    pub is_live: bool,

    // 封面下载
    pub use_live_cover: bool,
    pub cover_url: Option<String>,

    // 认证与网络
    pub cookies_file: Option<PathBuf>,
    pub proxy: Option<String>,

    // yt-dlp 参数
    pub prefer_vcodec: Option<String>,
    pub prefer_acodec: Option<String>,
    pub max_filesize: Option<String>,
    pub max_height: Option<u32>,
    pub download_archive: Option<PathBuf>,
    pub two_stream_merge: bool,

    // ytarchive 参数
    pub yta_threads: u8,

    // 可执行文件名
    pub ytdlp_bin: String,
    pub ytarchive_bin: String,

    // 附加自定义参数
    pub extra_ytdlp_args: Vec<String>,
    pub extra_yta_args: Vec<String>,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            webpage_url: String::new(),
            download_url: None,
            filename: "output".into(),
            suffix: "mp4".into(),
            working_dir: PathBuf::from("."),
            backend: Backend::YtDlp,
            is_live: false,
            use_live_cover: false,
            cover_url: None,
            cookies_file: None,
            proxy: None,
            prefer_vcodec: None,
            prefer_acodec: None,
            max_filesize: None,
            max_height: None,
            download_archive: Some(PathBuf::from("archive.txt")),
            two_stream_merge: true,
            yta_threads: 3,
            ytdlp_bin: "yt-dlp".into(),
            ytarchive_bin: "ytarchive".into(),
            extra_ytdlp_args: vec![],
            extra_yta_args: vec![],
        }
    }
}

pub struct YouTubeDownloader {
    cfg: DownloadConfig,
    stopped: Arc<AtomicBool>,
}

impl YouTubeDownloader {
    pub fn new(cfg: DownloadConfig) -> Self {
        Self {
            cfg,
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn download<'a>(
        &self,
        mut callback: Box<dyn FnMut(SegmentEvent) + Send + Sync + 'a>,
        runtime_cfg: RuntimeDownloadConfig,
    ) -> AppResult<DownloadStatus> {
        self.stopped.store(false, Ordering::Relaxed);
        let mut cfg = self.cfg.clone();
        cfg.suffix = runtime_cfg.suffix;
        cfg.filename = runtime_cfg.recorder.generate_filename_in(
            &runtime_cfg.output_dir,
            &["mp4", "flv", "ts", "mkv", "webm"],
        );
        cfg.working_dir = runtime_cfg.output_dir;

        let downloader = Self {
            cfg,
            stopped: self.stopped.clone(),
        };
        let paths = downloader.run().await?;
        if downloader.stopped.load(Ordering::Relaxed) {
            return Ok(DownloadStatus::StreamEnded);
        }
        for (index, path) in paths.into_iter().enumerate() {
            callback(SegmentEvent::Segment(SegmentInfo::new(
                path, None, None, index,
            )));
        }
        Ok(DownloadStatus::StreamEnded)
    }

    async fn run(&self) -> AppResult<Vec<PathBuf>> {
        let cover_handle = self
            .cfg
            .use_live_cover
            .then(|| self.spawn_cover_download())
            .flatten();
        let paths = match self.cfg.backend {
            Backend::YtArchive => self.run_ytarchive().await?,
            Backend::YtDlp => self.run_ytdlp().await?,
        };
        if let Some(handle) = cover_handle {
            match timeout(Duration::from_secs(20), handle).await {
                Ok(Ok(Ok(()))) => info!("封面已下载"),
                other => warn!(?other, "封面下载未完成"),
            }
        }
        Ok(paths)
    }

    pub async fn stop(&self) -> AppResult<()> {
        self.stopped.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn run_ytdlp(&self) -> AppResult<Vec<PathBuf>> {
        fs::create_dir_all(&self.cfg.working_dir)
            .await
            .change_context(AppError::Unknown)?;
        // Download and merge in the output directory. Only after_move reports
        // completed media; partial files and sidecars are never discovered by scanning.
        let manifest = tempfile::NamedTempFile::new()
            .change_context(AppError::Unknown)?
            .into_temp_path();
        // 构造格式串
        let format_str = if self.cfg.two_stream_merge {
            let mut s = String::from("bestvideo");
            if let Some(v) = &self.cfg.prefer_vcodec {
                s.push_str(&format!("[vcodec~='^({})']", v));
            }
            if !self.cfg.is_live
                && let Some(f) = &self.cfg.max_filesize
            {
                s.push_str(&format!("[filesize<{}]", f));
            }
            if let Some(h) = self.cfg.max_height {
                s.push_str(&format!("[height<={}]", h));
            }
            s.push_str("+bestaudio");
            if let Some(a) = &self.cfg.prefer_acodec {
                s.push_str(&format!("[acodec~='^({})']", a));
            }
            s
        } else {
            "best".to_string()
        };

        let mut cmd = tools::command(&self.cfg.ytdlp_bin);
        cmd.arg("-o")
            .arg(format!("{}.%(ext)s", self.cfg.filename.replace('%', "%%")))
            .arg("--paths")
            .arg(format!("home:{}", self.cfg.working_dir.display()))
            .arg("--format")
            .arg(format_str);
        if let Some(cookie) = &self.cfg.cookies_file {
            cmd.arg("--cookies").arg(cookie);
        }
        if let Some(proxy) = &self.cfg.proxy {
            cmd.arg("--proxy").arg(proxy);
        }
        if let Some(archive) = &self.cfg.download_archive {
            // 直播也传 --download-archive：yt-dlp 在直播下载完成后同样会写入 archive，
            // 避免直播结束后同一视频被当作回放重复下载（对齐 youtube.py:312, 331-334）
            cmd.arg("--download-archive").arg(archive);
        }

        // 自定义附加参数
        for a in &self.cfg.extra_ytdlp_args {
            cmd.arg(a);
        }

        cmd.arg("--print-to-file")
            .arg("after_move:%(filepath)j")
            .arg(manifest.to_string_lossy().replace('%', "%%"));

        let url = self
            .cfg
            .download_url
            .as_ref()
            .unwrap_or(&self.cfg.webpage_url);
        cmd.arg(url);

        cmd.kill_on_drop(true);

        if self.stopped.load(Ordering::Relaxed) {
            return Ok(Vec::new());
        }

        info!("运行: {}", redact_process_debug(&cmd));
        let output = cmd.output().await.change_context(AppError::Custom(format!(
            "运行 {} 失败，请确认已安装并在 PATH 中",
            &self.cfg.ytdlp_bin
        )))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);

        // yt-dlp 用退出码 101 表示“已达到 --max-downloads 上限，主动停止”。
        // 这不是失败：该下的文件已经下载完并合并好了，只是不再继续下一个。
        // 若当成失败，会漏掉已写入 archive 的完成产物，下一轮又被 archive 跳过。
        let max_downloads_abort = output.status.code() == Some(101);

        if !output.status.success() && !max_downloads_abort {
            if combined.contains("ffmpeg is not installed")
                || combined.contains("ffmpeg not found")
                || combined.contains("ffprobe not found")
            {
                bail!(AppError::Custom(String::from(
                    "ffmpeg 未安装或不可用，无法合并音视频"
                )));
            } else if combined.contains("Requested format is not available") {
                bail!(AppError::Custom(String::from(
                    "无法获取到流，请检查 vcodec/acodec/height/filesize 等筛选设置"
                )));
            } else {
                bail!(AppError::Custom(format!("yt-dlp 执行失败:\n{}", combined)));
            }
        }

        if max_downloads_abort {
            info!("yt-dlp 已达 --max-downloads 上限（exit 101），接收已完成的下载产物");
        }

        let reported = fs::read_to_string(&manifest)
            .await
            .change_context(AppError::Unknown)?;
        let paths = completed_paths(&reported)?;
        if paths.is_empty() {
            info!("yt-dlp 未报告完成的视频（可能命中 archive），跳过上传");
        }
        Ok(paths)
    }

    async fn run_ytarchive(&self) -> AppResult<Vec<PathBuf>> {
        fs::create_dir_all(&self.cfg.working_dir)
            .await
            .change_context(AppError::Unknown)?;
        let working_dir =
            std::fs::canonicalize(&self.cfg.working_dir).change_context(AppError::Unknown)?;
        let suffix = if self
            .cfg
            .suffix
            .trim_start_matches('.')
            .eq_ignore_ascii_case("mkv")
        {
            "mkv"
        } else {
            "mp4"
        };
        let base = working_dir.join(&self.cfg.filename);
        let output_path = working_dir.join(format!("{}.{}", self.cfg.filename, suffix));
        if output_path.exists() {
            bail!(AppError::Custom(format!(
                "ytarchive 输出已存在，拒绝覆盖: {}",
                output_path.display()
            )));
        }
        let mut cmd = tools::command(&self.cfg.ytarchive_bin);
        cmd.arg("--threads")
            .arg(self.cfg.yta_threads.to_string())
            // ytarchive appends its own extension; --output is a basename.
            .arg("--output")
            .arg(base.to_string_lossy().replace('%', "%%"))
            .arg("--add-metadata");
        if suffix == "mkv" {
            cmd.arg("--mkv");
        }
        if let Some(cookie) = &self.cfg.cookies_file {
            cmd.arg("--cookies").arg(cookie);
        }
        if let Some(proxy) = &self.cfg.proxy {
            cmd.arg("--proxy").arg(proxy);
        }
        cmd.args(&self.cfg.extra_yta_args)
            .arg(&self.cfg.webpage_url)
            .arg("best")
            .kill_on_drop(true);
        if self.stopped.load(Ordering::Relaxed) {
            return Ok(Vec::new());
        }
        info!("运行: {}", redact_process_debug(&cmd));
        let output = cmd.output().await.change_context(AppError::Unknown)?;
        if !output.status.success() {
            bail!(AppError::Custom(format!(
                "ytarchive 执行失败: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        FileValidator::validate_completed_video(&output_path)?;
        Ok(vec![output_path])
    }

    fn spawn_cover_download(&self) -> Option<tokio::task::JoinHandle<AppResult<()>>> {
        let url = self.cfg.cover_url.clone()?;

        let filename = self.cfg.filename.clone();
        let working_dir = self.cfg.working_dir.clone();

        let handle = tokio::spawn(async move {
            fs::create_dir_all(&working_dir).await.ok();

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .change_context(AppError::Unknown)?;

            let resp = client
                .get(&url)
                .send()
                .await
                .change_context(AppError::Unknown)?;
            if !resp.status().is_success() {
                bail!(AppError::Custom(format!(
                    "封面请求失败: HTTP {}",
                    resp.status()
                )));
            }

            let content_type = resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("image/jpeg")
                .to_string();

            let ext = if content_type.contains("png") {
                "png"
            } else if content_type.contains("webp") {
                "webp"
            } else {
                "jpg"
            };

            let bytes = resp.bytes().await.change_context(AppError::Unknown)?;
            let out = working_dir.join(format!("{}.{}", filename, ext));
            fs::write(&out, &bytes)
                .await
                .change_context(AppError::Custom(format!("写入封面失败: {}", out.display())))?;
            Ok(())
        });

        Some(handle)
    }
}

// JSON lines preserve Unicode, spaces and embedded newlines in paths.
fn completed_paths(reported: &str) -> AppResult<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for line in reported.lines().filter(|line| !line.trim().is_empty()) {
        let path: PathBuf = serde_json::from_str(line).change_context(AppError::Unknown)?;
        FileValidator::validate_completed_video(&path)?;
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn download_exit_never_deletes_leftover_media() {
        for exit_code in [1, 0] {
            let dir = tempfile::tempdir().unwrap();
            let script = dir.path().join(if cfg!(windows) {
                "yt-dlp.cmd"
            } else {
                "yt-dlp"
            });
            #[cfg(windows)]
            let source = format!(
                "@echo off\r\n:next\r\nif \"%~1\"==\"\" goto done\r\nset \"arg=%~1\"\r\nif \"%arg:~0,5%\"==\"temp:\" exit /b 2\r\nif \"%arg:~0,5%\"==\"home:\" set \"output=%arg:~5%\"\r\nshift\r\ngoto next\r\n:done\r\necho partial>\"%output%\\unfinished.mp4.part\"\r\nexit /b {exit_code}\r\n",
            );
            #[cfg(not(windows))]
            let source = format!(
                "#!/bin/sh\nfor arg do\n case \"$arg\" in\n temp:*) exit 2;;\n home:*) output=${{arg#home:}};;\n esac\ndone\nprintf partial > \"$output/unfinished.mp4.part\"\nexit {exit_code}\n",
            );
            fs::write(&script, source).await.unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
                    .await
                    .unwrap();
            }
            let downloader = YouTubeDownloader::new(DownloadConfig {
                ytdlp_bin: script.to_string_lossy().into_owned(),
                webpage_url: "https://example.invalid/video".into(),
                working_dir: dir.path().join("output"),
                download_archive: None,
                ..Default::default()
            });
            let result = downloader.run_ytdlp().await;
            assert_eq!(result.is_err(), exit_code != 0);
            assert!(
                dir.path().join("output/unfinished.mp4.part").is_file(),
                "exit {exit_code} removed unfinished media"
            );
        }
    }

    #[tokio::test]
    async fn only_reported_outputs_are_published_regardless_of_configured_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let actual = dir.path().join("video with spaces.webm");
        let stale = dir.path().join("video with spaces.mp4");
        fs::write(&actual, b"webm").await.unwrap();
        fs::write(&stale, b"old mp4").await.unwrap();
        fs::write(dir.path().join("video with spaces.info.json"), b"{}")
            .await
            .unwrap();
        let report = serde_json::to_string(&actual).unwrap();
        assert_eq!(
            completed_paths(&format!("{report}\n{report}\n")).unwrap(),
            [actual]
        );
        assert!(
            completed_paths("").unwrap().is_empty(),
            "archive hits must not reuse old files"
        );
    }

    #[tokio::test]
    async fn missing_partial_and_non_media_reports_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "missing.mp4",
            "video.mp4.part",
            "video.info.json",
            "empty.mp4",
        ] {
            let path = dir.path().join(name);
            if name != "missing.mp4" {
                fs::write(
                    &path,
                    if name == "empty.mp4" {
                        b"".as_slice()
                    } else {
                        b"fixture"
                    },
                )
                .await
                .unwrap();
            }
            assert!(completed_paths(&serde_json::to_string(&path).unwrap()).is_err());
        }
    }
}
