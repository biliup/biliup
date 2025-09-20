use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

// 钩子步骤：既支持 key-value 形式（如 {run: "..."}），也支持纯字符串（如 "rm"）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookStep {
    // 处理 {run = "..."} 格式
    Run { run: String },
    // 处理 {mv = "..."} 格式
    Move { mv: String },
    // 处理 "rm" 字符串格式
    Remove(String),
}

impl HookStep {
    /// 执行后处理操作
    pub async fn execute(&self, video_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            HookStep::Run { run } => {
                self.execute_command(run, video_path).await?;
            }
            HookStep::Move { mv } => {
                self.move_file(video_path, mv).await?;
            }
            HookStep::Remove(cmd) if cmd == "rm" => {
                self.remove_file(video_path).await?;
            }
            HookStep::Remove(cmd) => {
                return Err(format!("Unknown command: {}", cmd).into());
            }
        }
        Ok(())
    }

    /// 执行命令，将视频路径作为标准输入传入
    async fn execute_command(
        &self,
        cmd: &str,
        video_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Executing: {}", cmd);

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".into());
        }

        let mut process = Command::new(parts[0])
            .args(&parts[1..])
            .stdin(std::process::Stdio::piped())
            .spawn()?;

        // 将视频文件路径写入标准输入
        if let Some(mut stdin) = process.stdin.take() {
            stdin
                .write_all(video_path.to_string_lossy().as_bytes())
                .await?;
        }

        let status = process.wait().await?;
        if !status.success() {
            return Err(format!("Command failed with status: {}", status).into());
        }

        Ok(())
    }

    /// 移动文件到指定目录
    async fn move_file(
        &self,
        video_path: &Path,
        target_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let target_path = Path::new(target_dir);

        // 创建目标目录（如果不存在）
        if !target_path.exists() {
            fs::create_dir_all(target_path).await?;
        }

        let file_name = video_path.file_name().ok_or("Invalid file name")?;

        let destination = target_path.join(file_name);

        println!(
            "Moving {} to {}",
            video_path.display(),
            destination.display()
        );
        fs::rename(video_path, destination).await?;

        Ok(())
    }

    /// 删除文件
    async fn remove_file(&self, video_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        println!("Removing: {}", video_path.display());
        fs::remove_file(video_path).await?;
        Ok(())
    }
}

/// 处理所有后处理器
pub async  fn process_video(video_path: &Path, processors: &[HookStep]) -> Result<(), Box<dyn std::error::Error>> {
    for processor in processors {
        processor.execute(video_path).await?;
    }
    Ok(())
}
