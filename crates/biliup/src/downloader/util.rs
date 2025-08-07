use chrono::{DateTime, Local};
use std::fs;
use std::path::{Path, PathBuf};

use std::time::Duration;
use tracing::{error, info};

use super::extractor::CallbackFn;

#[derive(Debug)]
pub enum Segment {
    Time(Duration, Duration),
    Size(u64, u64),
    Never,
}

#[derive(Debug)]
pub struct Segmentable {
    time: Time,
    size: Size,
}

#[derive(Debug)]
struct Time {
    expected: Option<Duration>,
    start: Duration,
    current: Duration,
}

#[derive(Debug)]
struct Size {
    expected: Option<u64>,
    current: u64,
}

impl Segmentable {
    pub fn new(expected_time: Option<Duration>, expected_size: Option<u64>) -> Self {
        Self {
            time: Time {
                expected: expected_time,
                start: Duration::ZERO,
                current: Duration::ZERO,
            },
            size: Size {
                expected: expected_size,
                current: 0,
            },
        }
    }

    /// 检查是否需要分割 - 只要时间或大小任一条件满足就返回 true
    pub fn needed(&self) -> bool {
        // 检查时间条件
        let time_exceeded = if let Some(expected_time) = self.time.expected {
            (self.time.current - self.time.start) >= expected_time
        } else {
            false
        };

        // 检查大小条件
        let size_exceeded = if let Some(expected_size) = self.size.expected {
            self.size.current >= expected_size
        } else {
            false
        };

        // 任一条件满足就需要分割
        let result = time_exceeded || size_exceeded;

        // 添加调试信息
        if result {
            if time_exceeded && size_exceeded {
                tracing::debug!(
                    "Segmentation needed: Both time ({:?} >= {:?}) and size ({} >= {}) conditions met",
                    self.time.current - self.time.start,
                    self.time.expected.unwrap(),
                    self.size.current,
                    self.size.expected.unwrap()
                );
            } else if time_exceeded {
                tracing::debug!(
                    "Segmentation needed: Time condition met ({:?} >= {:?})",
                    self.time.current - self.time.start,
                    self.time.expected.unwrap()
                );
            } else if size_exceeded {
                tracing::debug!(
                    "Segmentation needed: Size condition met ({} >= {})",
                    self.size.current,
                    self.size.expected.unwrap()
                );
            }
        }

        result
    }

    /// 检查单独的时间条件
    pub fn time_needed(&self) -> bool {
        if let Some(expected_time) = self.time.expected {
            (self.time.current - self.time.start) >= expected_time
        } else {
            false
        }
    }

    /// 检查单独的大小条件
    pub fn size_needed(&self) -> bool {
        if let Some(expected_size) = self.size.expected {
            self.size.current >= expected_size
        } else {
            false
        }
    }

    /// 获取分割原因的描述
    pub fn get_segment_reason(&self) -> String {
        let time_exceeded = self.time_needed();
        let size_exceeded = self.size_needed();

        match (time_exceeded, size_exceeded) {
            (true, true) => "Time and size limits reached".to_string(),
            (true, false) => "Time limit reached".to_string(),
            (false, true) => "Size limit reached".to_string(),
            (false, false) => "No segmentation needed".to_string(),
        }
    }

    pub fn increase_time(&mut self, number: Duration) {
        self.time.current += number
    }

    pub fn set_time_position(&mut self, number: Duration) {
        self.time.current = number
    }

    pub fn set_start_time(&mut self, number: Duration) {
        self.time.start = number
    }

    pub fn increase_size(&mut self, number: u64) {
        self.size.current += number
    }

    pub fn set_size_position(&mut self, number: u64) {
        self.size.current = number
    }

    /// 重置计数器，通常在创建新分割后调用
    pub fn reset(&mut self) {
        self.size.current = 0;
        self.time.start = self.time.current; // 保持当前时间位置，但重置起始点
    }

    /// 完全重置所有状态
    pub fn full_reset(&mut self) {
        self.size.current = 0;
        self.time.current = Duration::ZERO;
        self.time.start = Duration::ZERO;
    }

    /// 获取当前状态信息
    pub fn get_status(&self) -> String {
        let time_info = if let Some(expected) = self.time.expected {
            format!(
                "Time: {:.1}s/{:.1}s ({:.1}%)",
                (self.time.current - self.time.start).as_secs_f64(),
                expected.as_secs_f64(),
                ((self.time.current - self.time.start).as_secs_f64() / expected.as_secs_f64() * 100.0).min(100.0)
            )
        } else {
            "Time: No limit".to_string()
        };

        let size_info = if let Some(expected) = self.size.expected {
            format!(
                "Size: {}/{} bytes ({:.1}%)",
                self.size.current,
                expected,
                (self.size.current as f64 / expected as f64 * 100.0).min(100.0)
            )
        } else {
            "Size: No limit".to_string()
        };

        format!("{}, {}", time_info, size_info)
    }
}

impl Default for Segmentable {
    fn default() -> Self {
        Segmentable {
            time: Time {
                expected: None,
                start: Duration::ZERO,
                current: Duration::ZERO,
            },
            size: Size {
                expected: None,
                current: 0,
            },
        }
    }
}

pub struct LifecycleFile {
    pub fmt_file_name: String,
    pub file_name: String,
    pub path: PathBuf,
    pub hook: CallbackFn,
    pub extension: &'static str,
}

impl LifecycleFile {
    pub fn new(fmt_file_name: &str, extension: &'static str, hook: Option<CallbackFn>) -> Self {
        let hook: Box<dyn Fn(&str) + Send> = match hook {
            Some(hook) => hook,
            _ => Box::new(|_| {}),
        };
        Self {
            fmt_file_name: fmt_file_name.to_string(),
            file_name: "".to_string(),
            path: Default::default(),
            hook,
            extension,
        }
    }

    pub fn create(&mut self) -> Result<&Path, std::io::Error> {
        self.file_name = format!(
            "{}.{}",
            format_filename(&self.fmt_file_name),
            self.extension
        );
        self.path = PathBuf::from(&self.file_name);
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?
        }
        // path.set_extension(&self.extension);
        self.path.set_extension(format!("{}.part", self.extension));
        info!("Save to {}", self.path.display());
        Ok(self.path.as_path())
    }

    pub fn rename(&self) {
        match fs::rename(&self.path, &self.file_name) {
            Ok(_) => (self.hook)(&self.file_name),
            Err(e) => {
                error!("drop {} {e}", self.path.display())
            }
        }
    }
}

pub fn format_filename(file_name: &str) -> String {
    let local: DateTime<Local> = Local::now();
    // let time_str = local.format("%Y-%m-%dT%H_%M_%S");
    let time_str = local.format(file_name);
    // format!("{file_name}{time_str}")
    time_str.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::path::{Path, PathBuf};

    #[test]
    fn it_works() -> Result<()> {
        let mut p = PathBuf::from("/feel/the");

        p.set_extension("force");
        assert_eq!(Path::new("/feel/the.force"), p.as_path());

        p.set_extension("");
        assert_eq!(Path::new("/feel/the"), p.as_path());

        Ok(())
    }

    #[test]
    fn test_segmentation_logic() -> Result<()> {
        // 测试时间分割
        let mut seg = Segmentable::new(Some(Duration::from_secs(10)), None);
        assert!(!seg.needed());

        seg.increase_time(Duration::from_secs(15));
        assert!(seg.needed());
        assert!(seg.time_needed());
        assert!(!seg.size_needed());

        // 测试大小分割
        let mut seg = Segmentable::new(None, Some(1024));
        assert!(!seg.needed());

        seg.increase_size(2048);
        assert!(seg.needed());
        assert!(!seg.time_needed());
        assert!(seg.size_needed());

        // 测试双重条件
        let mut seg = Segmentable::new(Some(Duration::from_secs(10)), Some(1024));
        assert!(!seg.needed());

        // 只满足时间条件
        seg.increase_time(Duration::from_secs(15));
        assert!(seg.needed());

        // 重置并只满足大小条件
        seg.full_reset();
        seg.increase_size(2048);
        assert!(seg.needed());

        // 同时满足两个条件
        seg.increase_time(Duration::from_secs(15));
        assert!(seg.needed());
        assert!(seg.time_needed());
        assert!(seg.size_needed());

        Ok(())
    }
}