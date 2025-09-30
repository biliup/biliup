use crate::server::errors::{AppError, AppResult};
use error_stack::{ResultExt, bail};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::info;

/// 钩子步骤枚举：支持多种操作格式
/// 既支持 key-value 形式（如 {run: "..."}），也支持纯字符串（如 "rm"）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookStep {
    /// 执行命令格式：{run: "command"}
    Run { run: String },
    /// 移动文件格式：{mv: "target_dir"}
    Move { mv: String },
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
                self.execute_command(run, video_paths).await?;
            }
            HookStep::Move { mv } => {
                // 移动文件到指定目录
                self.move_file(video_paths, mv).await?;
            }
            HookStep::Remove(cmd) if cmd == "rm" => {
                // 删除文件
                self.remove_file(video_paths).await?;
            }
            HookStep::Remove(cmd) => {
                // 未知命令，返回错误
                bail!(AppError::Custom(format!("Unknown command: {}", cmd)));
            }
        }
        Ok(())
    }

    /// 执行自定义命令，将视频路径作为标准输入传入
    ///
    /// # 参数
    /// * `cmd` - 要执行的命令字符串
    /// * `video_paths` - 视频文件路径列表
    async fn execute_command(&self, cmd: &str, video_paths: &[&Path]) -> AppResult<()> {
        println!("Executing: {}", cmd);

        // 解析命令和参数
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            bail!(AppError::Custom("Empty command".into()));
        }

        // 启动子进程，配置标准输入管道
        let mut process = Command::new(parts[0])
            .args(&parts[1..])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .change_context(AppError::Unknown)?;

        // 将视频文件路径写入标准输入
        if let Some(mut stdin) = process.stdin.take() {
            let paths_str = video_paths
                .iter()
                .map(|p| p.to_string_lossy())
                .reduce(|acc, e| Cow::from(acc.to_string() + "\n" + &*e))
                .expect("At least one video path should exist");
            stdin
                .write_all(paths_str.as_bytes())
                .await
                .change_context(AppError::Unknown)?;
        }

        // 等待进程完成并检查退出状态
        let status = process.wait().await.change_context(AppError::Unknown)?;
        if !status.success() {
            bail!(AppError::Custom(
                format!("Command failed with status: {}", status).into()
            ));
        }

        Ok(())
    }

    /// 移动文件到指定目录
    ///
    /// # 参数
    /// * `video_paths` - 视频文件路径列表
    /// * `target_dir` - 目标目录路径
    async fn move_file(&self, video_paths: &[&Path], target_dir: &str) -> AppResult<()> {
        let target_path = Path::new(target_dir);

        // 创建目标目录（如果不存在）
        if !target_path.exists() {
            fs::create_dir_all(target_path)
                .await
                .change_context(AppError::Unknown)?;
        }

        // 移动每个视频文件到目标目录
        for video_path in video_paths {
            let file_name = video_path
                .file_name()
                .ok_or(AppError::Custom("Invalid file name".to_string()))?;
            let destination = target_path.join(file_name);

            info!(
                "Moving {} to {}",
                video_path.display(),
                destination.display()
            );
            fs::rename(video_path, destination)
                .await
                .change_context(AppError::Unknown)?;

            let buf = video_path.with_extension("xml");
            if buf.exists() {
                let file_name = buf
                    .file_name()
                    .ok_or(AppError::Custom("Invalid file name".to_string()))?;
                let destination = target_path.join(file_name);
                fs::rename(&buf, &destination)
                    .await
                    .change_context(AppError::Unknown)?;
                info!(
                    "移动弹幕文件: Moving {} to {}",
                    buf.display(),
                    destination.display()
                );
            }
        }
        Ok(())
    }

    /// 删除指定的视频文件
    ///
    /// # 参数
    /// * `video_paths` - 要删除的视频文件路径列表
    async fn remove_file(&self, video_paths: &[&Path]) -> AppResult<()> {
        // 逐个删除视频文件
        for video_path in video_paths {
            info!("删除 - Removing: {}", video_path.display());
            fs::remove_file(video_path)
                .await
                .change_context(AppError::Unknown)?;
            let buf = video_path.with_extension("xml");
            if buf.exists() {
                fs::remove_file(&buf)
                    .await
                    .change_context(AppError::Unknown)?;
                info!("删除弹幕文件 - Removing: {}", buf.display());
            }
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
