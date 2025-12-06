//! 账号级上传互斥锁
//!
//! 用于防止多个进程同时使用同一账号上传时触发限流。
//! 当某个进程因限流进入等待状态时，会创建锁文件。
//! 其他进程检测到锁存在时会立即退出，避免继续触发限流。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

/// 上传锁，用于账号级互斥
pub struct UploadLock {
    lock_path: PathBuf,
    acquired: bool,
}

impl UploadLock {
    /// 创建一个新的上传锁（基于账号凭证）
    ///
    /// # Arguments
    /// * `credential_id` - 账号标识符（如 cookie hash 或 uid）
    pub fn new(credential_id: &str) -> io::Result<Self> {
        let lock_dir = Self::get_lock_dir()?;
        fs::create_dir_all(&lock_dir)?;

        let lock_path = lock_dir.join(format!("biliup_upload_{}.lock", credential_id));

        Ok(Self {
            lock_path,
            acquired: false,
        })
    }

    /// 获取锁目录路径
    fn get_lock_dir() -> io::Result<PathBuf> {
        let base_dir = dirs::data_local_dir()
            .or_else(|| Some(std::env::temp_dir()))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "无法获取数据目录"))?;

        Ok(base_dir.join("biliup").join("locks"))
    }

    /// 尝试获取锁
    ///
    /// # Returns
    /// * `Ok(true)` - 成功获取锁
    /// * `Ok(false)` - 锁已被其他进程持有
    /// * `Err(_)` - 发生错误
    pub fn try_acquire(&mut self) -> io::Result<bool> {
        // 检查锁文件是否存在
        if self.lock_path.exists() {
            // 检查锁是否过期（超过30分钟认为是僵尸锁）
            if let Ok(metadata) = fs::metadata(&self.lock_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                        if elapsed > Duration::from_secs(30 * 60) {
                            warn!("检测到过期锁文件，自动清理: {:?}", self.lock_path);
                            let _ = fs::remove_file(&self.lock_path);
                        } else {
                            info!("检测到其他进程正在使用该账号上传（锁文件: {:?}）", self.lock_path);
                            return Ok(false);
                        }
                    }
                }
            }
        }

        // 尝试创建锁文件
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.lock_path)
        {
            Ok(_) => {
                info!("成功获取上传锁: {:?}", self.lock_path);
                self.acquired = true;
                Ok(true)
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// 释放锁
    pub fn release(&mut self) -> io::Result<()> {
        if self.acquired && self.lock_path.exists() {
            fs::remove_file(&self.lock_path)?;
            info!("释放上传锁: {:?}", self.lock_path);
            self.acquired = false;
        }
        Ok(())
    }

    /// 检查锁是否存在（不尝试获取）
    pub fn is_locked(&self) -> bool {
        self.lock_path.exists()
    }
}

impl Drop for UploadLock {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_acquire_and_release() {
        let mut lock1 = UploadLock::new("test_account").unwrap();

        // 第一个锁应该能获取成功
        assert!(lock1.try_acquire().unwrap());

        // 第二个锁应该获取失败
        let mut lock2 = UploadLock::new("test_account").unwrap();
        assert!(!lock2.try_acquire().unwrap());

        // 释放第一个锁
        lock1.release().unwrap();

        // 现在第二个锁应该能获取成功
        assert!(lock2.try_acquire().unwrap());

        lock2.release().unwrap();
    }
}
