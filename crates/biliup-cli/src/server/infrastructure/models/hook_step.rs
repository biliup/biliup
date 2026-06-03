use crate::server::errors::{AppError, AppResult};
use error_stack::{ResultExt, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::process::Command;
use tracing::{error, info};

/// 钩子步骤枚举：支持多种操作格式
/// 既支持 key-value 形式（如 {run: "..."}），也支持纯字符串（如 "rm"）
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookStep {
    /// 执行命令格式：{run: "command"}
    Run { run: String },
    /// 移动文件格式：{mv: "target_dir"}
    Move { mv: String },
    /// ts→mp4 无重编码 remux 格式：{remux: "mp4"}
    /// 解决 B 站对 .ts 直传时常见的 "时间戳跳变" 转码失败。
    /// 用 ffmpeg `-c copy -fflags +genpts+igndts -bsf:a aac_adtstoasc -movflags +faststart`，
    /// 重新生成 PTS。仅当输入扩展名为 .ts/.m2ts 时生效；其它格式跳过。
    /// 路径列表会被原地替换为新生成的 .mp4，原文件保留（postprocessor 中再 "rm"）。
    Remux { remux: String },
    /// 删除文件格式："rm"
    Remove(String),
}

impl HookStep {
    /// 执行钩子步骤操作
    ///
    /// # 参数
    /// * `video_paths` - 视频文件路径列表
    ///
    /// # 返回
    /// 执行成功返回Ok(())，失败返回错误信息
    pub async fn execute(&self, video_paths: &[&Path]) -> AppResult<()> {
        match self {
            HookStep::Run { run } => {
                // 执行自定义命令
                let paths: Vec<String> = video_paths
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                if paths.is_empty() {
                    return Err(AppError::Unknown.into());
                }
                let paths_str = paths.join("\n");
                self.execute_command(run, paths_str.as_bytes()).await?;
            }
            HookStep::Move { mv } => {
                // 移动文件到指定目录
                self.move_file(video_paths, mv).await?;
            }
            HookStep::Remux { remux } => {
                bail!(AppError::Custom(format!(
                    "remux step needs the path-mutating execute_paths API; \
                     called via the legacy execute path with target={}",
                    remux
                )));
            }
            HookStep::Remove(cmd) if cmd == "rm" => {
                // 删除文件
                HookStep::remove_file(video_paths).await?;
            }
            HookStep::Remove(cmd) => {
                // 未知命令，返回错误
                bail!(AppError::Custom(format!("Unknown command: {}", cmd)));
            }
        }
        Ok(())
    }

    /// Path-mutating variant: lets steps swap entries in the working set
    /// (e.g. Remux replaces .ts with the newly-created .mp4).
    pub async fn execute_paths(&self, video_paths: &mut Vec<PathBuf>) -> AppResult<()> {
        match self {
            HookStep::Remux { remux } => {
                let target = remux.to_lowercase();
                if target != "mp4" {
                    bail!(AppError::Custom(format!(
                        "remux: only target=\"mp4\" supported, got {}",
                        remux
                    )));
                }
                for p in video_paths.iter_mut() {
                    let ext = p
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_lowercase())
                        .unwrap_or_default();
                    if ext != "ts" && ext != "m2ts" {
                        info!("remux: skipping non-ts file {}", p.display());
                        continue;
                    }
                    let mp4 = p.with_extension("mp4");
                    if mp4.exists()
                        && tokio::fs::metadata(&mp4)
                            .await
                            .map(|m| m.len() > 0)
                            .unwrap_or(false)
                    {
                        info!("remux: reusing existing {}", mp4.display());
                        *p = mp4;
                        continue;
                    }
                    Self::ffmpeg_remux_to_mp4(p, &mp4).await?;
                    *p = mp4;
                }
            }
            other => {
                // For non-path-mutating steps, fall back to the read-only API.
                let refs: Vec<&Path> = video_paths.iter().map(|p| p.as_path()).collect();
                other.execute(&refs).await?;
            }
        }
        Ok(())
    }

    async fn ffmpeg_remux_to_mp4(src: &Path, dst: &Path) -> AppResult<()> {
        info!("remux ts→mp4: {} → {}", src.display(), dst.display());
        let started = std::time::Instant::now();
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "warning",
                "-y",
                "-fflags",
                "+genpts+igndts",
                "-i",
            ])
            .arg(src)
            .args([
                "-c",
                "copy",
                "-bsf:a",
                "aac_adtstoasc",
                "-movflags",
                "+faststart",
                "-avoid_negative_ts",
                "make_zero",
            ])
            .arg(dst)
            .kill_on_drop(true)
            .status()
            .await
            .change_context(AppError::Custom("failed to spawn ffmpeg".into()))?;
        if !status.success() {
            // Clean up partial output so a retry restarts cleanly.
            let _ = tokio::fs::remove_file(dst).await;
            bail!(AppError::Custom(format!(
                "ffmpeg remux failed (status {:?}) for {}",
                status,
                src.display()
            )));
        }
        let meta = tokio::fs::metadata(dst).await.change_context_lazy(|| {
            AppError::Custom(format!("remux output missing: {}", dst.display()))
        })?;
        if meta.len() == 0 {
            let _ = tokio::fs::remove_file(dst).await;
            bail!(AppError::Custom(format!(
                "ffmpeg produced empty mp4: {}",
                dst.display()
            )));
        }
        info!(
            "remux done {} ({:.1} MiB) in {:.1}s",
            dst.display(),
            meta.len() as f64 / 1048576.0,
            started.elapsed().as_secs_f64()
        );
        // Suppress unused-import warning for Duration on platforms that don't
        // need it (kept for future timeout work).
        let _ = Duration::from_secs(0);
        Ok(())
    }

    /// 执行钩子步骤操作
    ///
    /// # 参数
    /// * `video_paths` - 视频文件路径列表
    ///
    /// # 返回
    /// 执行成功返回Ok(())，失败返回错误信息
    pub async fn execute_with(&self, src: &[u8]) -> AppResult<()> {
        match self {
            HookStep::Run { run } => {
                self.execute_command(run, src).await?;
            }
            cmd => {
                // 未知命令，返回错误
                bail!(AppError::Custom(format!("不支持的命令: {:?}", cmd)));
            }
        }
        Ok(())
    }

    /// 执行自定义命令，将视频路径作为标准输入传入
    ///
    /// # 参数
    /// * `cmd` - 要执行的命令字符串
    /// * `video_paths` - 视频文件路径列表
    async fn execute_command(&self, cmd: &str, src: &[u8]) -> AppResult<()> {
        // 1. 跨平台 Shell 处理 (对应 shell=True)
        // Windows 使用 "cmd /C"，Unix/Mac 使用 "sh -c"
        let (shell, flag) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };
        // 执行自定义命令
        // 解析命令和参数
        // 启动子进程，配置标准输入管道
        let mut process = Command::new(shell)
            .arg(flag)
            .arg(cmd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .change_context(AppError::Unknown)?;

        // 将自定义输入写入标准输入
        if let Some(mut stdin) = process.stdin.take() {
            stdin
                .write_all(src)
                .await
                .change_context(AppError::Unknown)?;
        }

        let stdout = process
            .stdout
            .take()
            .ok_or(AppError::Custom("Failed to capture stdout".to_string()))?;
        let stderr = process
            .stderr
            .take()
            .ok_or(AppError::Custom("Failed to capture stderr".to_string()))?;

        // 子命令输出可能是 GBK、UTF-8 或任意字节流。这里不能按文本解码，
        // 而是原样转发到控制台并追加到 Web 可查看的日志文件。
        let stdout_task = tokio::spawn(Self::tee_command_output(
            stdout,
            tokio::io::stdout(),
            "download.log",
        ));
        let stderr_task = tokio::spawn(Self::tee_command_output(
            stderr,
            tokio::io::stderr(),
            "download.log",
        ));

        let status = process.wait().await.change_context(AppError::Unknown)?;
        stdout_task.await.change_context(AppError::Unknown)??;
        stderr_task.await.change_context(AppError::Unknown)??;

        if !status.success() {
            bail!(AppError::Custom(format!(
                "Command failed with status: {}",
                status
            )));
        }

        Ok(())
    }

    async fn tee_command_output<R, W>(
        mut reader: R,
        mut terminal: W,
        log_file: &'static str,
    ) -> AppResult<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .await
            .change_context(AppError::Unknown)?;
        let mut buf = [0; 8192];
        loop {
            let n = reader
                .read(&mut buf)
                .await
                .change_context(AppError::Unknown)?;
            if n == 0 {
                break;
            }
            terminal
                .write_all(&buf[..n])
                .await
                .change_context(AppError::Unknown)?;
            log.write_all(&buf[..n])
                .await
                .change_context(AppError::Unknown)?;
        }
        terminal.flush().await.change_context(AppError::Unknown)?;
        log.flush().await.change_context(AppError::Unknown)?;
        Ok(())
    }

    /// 移动文件到指定目录
    ///
    /// # 参数
    /// * `video_paths` - 视频文件路径列表
    /// * `target_dir` - 目标目录路径
    async fn move_file(&self, video_paths: &[&Path], target_dir: &str) -> AppResult<()> {
        let target_path = Path::new(target_dir);

        if !target_path.exists() {
            fs::create_dir_all(target_path)
                .await
                .change_context(AppError::Unknown)?;
        }

        for video_path in video_paths {
            self.move_single_file(video_path, target_path).await?;
        }
        Ok(())
    }

    /// 移动单个文件，支持跨文件系统
    async fn move_single_file(&self, source: &Path, target_dir: &Path) -> AppResult<()> {
        let file_name = source
            .file_name()
            .ok_or(AppError::Custom("Invalid file name".to_string()))?;
        let destination = target_dir.join(file_name);

        info!("Moving {} to {}", source.display(), destination.display());

        // 先尝试 rename（快速，仅同文件系统）
        match fs::rename(source, &destination).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // 检查是否是跨文件系统错误
                if HookStep::is_cross_device_error(&e) {
                    info!("检测到跨文件系统操作，使用复制+删除模式");
                    // 复制文件
                    fs::copy(source, &destination)
                        .await
                        .change_context(AppError::Unknown)?;

                    // 删除原文件
                    fs::remove_file(source)
                        .await
                        .change_context(AppError::Unknown)?;

                    Ok(())
                } else {
                    Err(e).change_context(AppError::Unknown)?
                }
            }
        }
    }

    /// 判断是否是跨设备/文件系统错误
    fn is_cross_device_error(error: &std::io::Error) -> bool {
        #[cfg(unix)]
        {
            error.raw_os_error() == Some(libc::EXDEV)
        }
        #[cfg(windows)]
        {
            error.raw_os_error() == Some(17) // ERROR_NOT_SAME_DEVICE
        }
        #[cfg(not(any(unix, windows)))]
        {
            error.kind() == std::io::ErrorKind::Other
        }
    }

    /// 删除指定的视频文件
    ///
    /// # 参数
    /// * `video_paths` - 要删除的视频文件路径列表
    pub(crate) async fn remove_file(video_paths: &[&Path]) -> AppResult<()> {
        // 逐个删除视频文件
        for video_path in video_paths {
            info!("删除 - Removing: {}", video_path.display());
            fs::remove_file(video_path)
                .await
                .change_context(AppError::Unknown)?;
        }
        Ok(())
    }
}

/// 处理所有后处理器步骤
/// 处理视频文件的主函数
/// 按顺序执行所有处理器步骤
///
/// # 参数
/// * `video_path` - 视频文件路径列表
/// * `processors` - 处理器步骤列表
pub async fn process_video(video_path: &[&Path], processors: &[HookStep]) -> AppResult<()> {
    info!("Starting video processing...");

    // 依次执行每个处理器步骤
    for processor in processors {
        processor.execute(video_path).await?;
    }

    info!("Video processing completed");
    Ok(())
}

/// Path-mutating processor pipeline. After each step the path list reflects any
/// in-place transforms (e.g. Remux replaces .ts with .mp4). Callers can read the
/// final paths to find the actual files to upload / postprocess.
pub async fn process_video_paths(
    video_paths: &mut Vec<PathBuf>,
    processors: &[HookStep],
) -> AppResult<()> {
    info!("Starting video processing (path-mutating)...");
    for processor in processors {
        processor.execute_paths(video_paths).await?;
    }
    info!("Video processing completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_yaml_variants() {
        let yaml = r#"
- run: "echo hi"
- mv: "/dst"
- remux: "mp4"
- "rm"
"#;
        let steps: Vec<HookStep> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(steps.len(), 4);
        assert!(matches!(steps[0], HookStep::Run { .. }));
        assert!(matches!(steps[1], HookStep::Move { .. }));
        assert!(matches!(steps[2], HookStep::Remux { .. }));
        assert!(matches!(steps[3], HookStep::Remove(_)));
    }

    #[test]
    fn deserialize_json_variants() {
        let json = r#"[
            {"run": "echo hi"},
            {"mv": "/dst"},
            {"remux": "mp4"},
            "rm"
        ]"#;
        let steps: Vec<HookStep> = serde_json::from_str(json).unwrap();
        assert_eq!(steps.len(), 4);
        assert!(matches!(steps[2], HookStep::Remux { .. }));
    }

    #[tokio::test]
    async fn remux_skips_non_ts() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("foo.mp4");
        std::fs::write(&p, b"not a real mp4 but we shouldn't touch it").unwrap();
        let mut paths = vec![p.clone()];
        let step = HookStep::Remux {
            remux: "mp4".into(),
        };
        step.execute_paths(&mut paths).await.unwrap();
        // Already .mp4 → unchanged.
        assert_eq!(paths[0], p);
    }

    #[tokio::test]
    async fn move_uses_explicit_paths_without_inferred_xml() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");
        std::fs::create_dir(&src).unwrap();
        std::fs::create_dir(&dst).unwrap();
        let video = src.join("segment.mp4");
        let danmaku = src.join("segment.xml");
        std::fs::write(&video, b"video").unwrap();
        std::fs::write(&danmaku, b"danmaku").unwrap();
        let paths = vec![video.as_path(), danmaku.as_path()];
        let step = HookStep::Move {
            mv: dst.display().to_string(),
        };

        step.execute(&paths).await.unwrap();

        assert!(!video.exists());
        assert!(!danmaku.exists());
        assert_eq!(std::fs::read(dst.join("segment.mp4")).unwrap(), b"video");
        assert_eq!(std::fs::read(dst.join("segment.xml")).unwrap(), b"danmaku");
    }

    #[tokio::test]
    async fn remove_uses_explicit_paths_without_inferred_xml() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("segment.mp4");
        let danmaku = dir.path().join("segment.xml");
        std::fs::write(&video, b"video").unwrap();
        std::fs::write(&danmaku, b"danmaku").unwrap();
        let paths = vec![video.as_path(), danmaku.as_path()];
        let step = HookStep::Remove("rm".into());

        step.execute(&paths).await.unwrap();

        assert!(!video.exists());
        assert!(!danmaku.exists());
    }

    /// 复现 pipeline_upload_videos 的 fault-tolerance 触发条件：
    /// segment_processor 在 ffmpeg 退非 0 时返回 Err。生产事故就是这一步因磁盘满
    /// 而失败，调用方必须能从 Err 恢复（log + continue 而不是 ? 早退）。
    #[tokio::test]
    async fn remux_fails_when_ffmpeg_cannot_read_input() {
        let dir = tempfile::tempdir().unwrap();
        let bogus = dir.path().join("does_not_exist.ts");
        let mut paths = vec![bogus.clone()];
        let step = HookStep::Remux {
            remux: "mp4".into(),
        };
        let result = step.execute_paths(&mut paths).await;
        assert!(
            result.is_err(),
            "ffmpeg 读不到输入应返回 Err，调用方才能在 pipeline 里做 log+continue"
        );
        // 失败时不要把破损的 mp4 留在磁盘上（idempotent retry 前提）
        assert!(!bogus.with_extension("mp4").exists());
    }
}

pub async fn process(input: &[u8], processors: &Option<Vec<HookStep>>) {
    if let Some(hooks) = processors {
        // 依次执行每个处理器步骤
        for processor in hooks {
            info!(processor=?processor, "Starting processing...");
            if let Err(e) = processor.execute_with(input).await {
                error!(error=?e, "自定义处理执行出错");
            }
            info!(processor=?processor, "processing completed");
        }
    }
}
