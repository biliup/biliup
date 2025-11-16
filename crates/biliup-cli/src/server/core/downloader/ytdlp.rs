use crate::server::errors::{AppError, AppResult};
use error_stack::{ResultExt, bail};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{fs, process::Command, time::timeout};
use tracing::{debug, info, warn};

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

    // 目录策略
    pub cache_dir: Option<PathBuf>,
    pub temp_root: PathBuf,
    pub use_temp_dir_for_ytdlp: bool,

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
            cache_dir: None,
            temp_root: PathBuf::from("./cache/temp/youtube"),
            use_temp_dir_for_ytdlp: true,
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
}

impl YouTubeDownloader {
    pub fn new(cfg: DownloadConfig) -> Self {
        Self { cfg }
    }

    pub async fn download(&self) -> AppResult<()> {
        // 1) 可选并发封面
        let cover_handle = if self.cfg.use_live_cover {
            self.spawn_cover_download()
        } else {
            None
        };

        // 2) 执行下载
        match self.cfg.backend {
            Backend::YtArchive => self.run_ytarchive().await?,
            Backend::YtDlp => self.run_ytdlp().await?,
        }

        // 3) 等待封面（限时 20s）
        if let Some(handle) = cover_handle {
            match timeout(Duration::from_secs(20), handle).await {
                Ok(Ok(Ok(()))) => info!("封面已下载"),
                Ok(Ok(Err(e))) => warn!("封面下载失败: {e:#}"),
                Ok(Err(e)) => warn!("封面下载任务异常: {e:#}"),
                Err(_) => warn!("封面下载超时，继续执行"),
            }
        }

        Ok(())
    }

    async fn run_ytdlp(&self) -> AppResult<()> {
        // 选择输出目录（临时目录 -> 搬运 -> 清理）
        let download_dir = if self.cfg.use_temp_dir_for_ytdlp {
            self.cfg.temp_root.join(&self.cfg.filename)
        } else {
            self.cfg.working_dir.clone()
        };
        fs::create_dir_all(&download_dir)
            .await
            .change_context(AppError::Custom(format!(
                "创建 yt-dlp 下载目录失败: {}",
                download_dir.display()
            )))?;

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

        let mut cmd = Command::new(&self.cfg.ytdlp_bin);
        cmd.arg("--outtmpl")
            .arg(format!(
                "{}/{}.%(ext)s",
                download_dir.display(),
                self.cfg.filename
            ))
            .arg("--break-on-reject")
            .arg("--format")
            .arg(format_str);

        if let Some(cookie) = &self.cfg.cookies_file {
            cmd.arg("--cookies").arg(cookie);
        }
        if let Some(proxy) = &self.cfg.proxy {
            cmd.arg("--proxy").arg(proxy);
        }
        if !self.cfg.is_live
            && let Some(archive) = &self.cfg.download_archive
        {
            cmd.arg("--download-archive").arg(archive);
        }

        // 自定义附加参数
        for a in &self.cfg.extra_ytdlp_args {
            cmd.arg(a);
        }

        let url = self
            .cfg
            .download_url
            .as_ref()
            .unwrap_or(&self.cfg.webpage_url);
        cmd.arg(url);

        cmd.kill_on_drop(true);

        info!("运行: {:?}", cmd);
        let output = cmd.output().await.change_context(AppError::Custom(format!(
            "运行 {} 失败，请确认已安装并在 PATH 中",
            &self.cfg.ytdlp_bin
        )))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);

        if !output.status.success() {
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

        // 下载成功，必要时搬运文件到工作目录
        if self.cfg.use_temp_dir_for_ytdlp && download_dir != self.cfg.working_dir {
            self.move_dir_contents(&download_dir, &self.cfg.working_dir)
                .await
                .change_context(AppError::Custom(format!(
                    "移动下载结果失败: {} -> {}",
                    download_dir.display(),
                    self.cfg.working_dir.display()
                )))?;
        }

        // 清理临时目录
        if self.cfg.use_temp_dir_for_ytdlp
            && let Err(e) = fs::remove_dir_all(&download_dir).await
        {
            warn!(
                "清理残留文件失败，请手动删除: {}，原因: {e}",
                download_dir.display()
            );
        }

        Ok(())
    }

    async fn run_ytarchive(&self) -> AppResult<()> {
        // ytarchive 工作目录（作为临时缓存）
        let cache_dir = self
            .cfg
            .cache_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(format!("./cache/{}", self.cfg.filename)));
        fs::create_dir_all(&cache_dir)
            .await
            .change_context(AppError::Custom(format!(
                "创建缓存目录失败: {}",
                cache_dir.display()
            )))?;

        // 在缓存目录中执行 ytarchive
        let mut cmd = Command::new(&self.cfg.ytarchive_bin);
        cmd.current_dir(&cache_dir)
            .arg(&self.cfg.webpage_url)
            .arg("best")
            .arg("--threads")
            .arg(self.cfg.yta_threads.to_string())
            .arg("--output")
            .arg(format!("{}.{}", self.cfg.filename, self.cfg.suffix));

        if let Some(cookie) = &self.cfg.cookies_file {
            cmd.arg("--cookies").arg(cookie);
        }
        if let Some(proxy) = &self.cfg.proxy {
            cmd.arg("--proxy").arg(proxy);
        }
        cmd.arg("--add-metadata");

        // 自定义附加参数
        for a in &self.cfg.extra_yta_args {
            cmd.arg(a);
        }

        cmd.kill_on_drop(true);
        info!("运行: (cwd: {}) {:?}", cache_dir.display(), cmd);

        let output = cmd.output().await.change_context(AppError::Custom(format!(
            "运行 {} 失败，请确认已安装并在 PATH 中",
            &self.cfg.ytarchive_bin
        )))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);

        if !output.status.success() {
            if combined.contains("ffmpeg is not installed") || combined.contains("ffmpeg not found")
            {
                bail!(AppError::Custom(String::from(
                    "ffmpeg 未安装，ytarchive 无法合并流"
                )));
            } else {
                bail!(AppError::Custom(format!(
                    "ytarchive 执行失败:\n{}",
                    combined
                )));
            }
        }

        // 搬运到工作目录
        self.move_dir_contents(&cache_dir, &self.cfg.working_dir)
            .await
            .change_context(AppError::Custom(format!(
                "移动下载结果失败: {} -> {}",
                cache_dir.display(),
                self.cfg.working_dir.display()
            )))?;

        // 清理缓存目录
        if let Err(e) = fs::remove_dir_all(&cache_dir).await {
            warn!(
                "清理残留文件失败，请手动删除: {}，原因: {e}",
                cache_dir.display()
            );
        }

        Ok(())
    }

    async fn move_dir_contents(&self, from: &Path, to: &Path) -> AppResult<()> {
        fs::create_dir_all(to)
            .await
            .change_context(AppError::Custom(format!(
                "创建目标目录失败: {}",
                to.display()
            )))?;

        let mut entries = fs::read_dir(from).await.change_context(AppError::Unknown)?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .change_context(AppError::Unknown)?
        {
            let p = entry.path();
            let metadata = fs::metadata(&p).await.change_context(AppError::Unknown)?;
            if metadata.is_file() {
                let target = to.join(
                    p.file_name()
                        .ok_or_else(|| AppError::Custom(format!("非法文件名: {}", p.display())))?,
                );
                if let Err(_e) = fs::rename(&p, &target).await {
                    // 跨设备移动失败 -> 复制再删除
                    fs::copy(&p, &target)
                        .await
                        .change_context(AppError::Custom(format!(
                            "复制文件失败: {} -> {}",
                            p.display(),
                            target.display()
                        )))?;
                    fs::remove_file(&p)
                        .await
                        .change_context(AppError::Custom(format!(
                            "删除源文件失败: {}",
                            p.display()
                        )))?;
                    debug!("跨设备移动: {} -> {}", p.display(), target.display());
                }
            }
        }
        Ok(())
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
