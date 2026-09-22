use chrono::{DateTime, Local};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use std::time::Duration;
use tracing::{error, info};

pub type CallbackFn<'a> = Box<dyn FnMut(&str) + Send + Sync + 'a>;

/// 已写盘字节的原子累计，供录制线程之外（如 Web 接口）读取实时速率。
///
/// 写盘路径上只做一次 `fetch_add`：不加锁、不 await、不会失败，
/// 因此不改变录制的任何控制流。`Clone` 得到的是同一计数器的另一个句柄。
#[derive(Debug, Clone, Default)]
pub struct ByteCounter(Arc<AtomicU64>);

impl ByteCounter {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn add(&self, bytes: u64) {
        self.0.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn total(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Debug)]
pub enum Segment {
    Time(Duration, Duration),
    Size(u64, u64),
    Never,
}

#[derive(Debug, Clone)]
pub struct Segmentable {
    time: Time,
    size: Size,
}

#[derive(Debug, Clone)]
struct Time {
    expected: Option<Duration>,
    start: Duration,
    current: Duration,
}

#[derive(Debug, Clone)]
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
        let time_exceeded = self.time_needed();
        let size_exceeded = self.size_needed();
        let result = time_exceeded || size_exceeded;

        // 添加调试信息
        if result {
            self.log_segmentation_reason(time_exceeded, size_exceeded);
        }

        result
    }

    fn elapsed_time(&self) -> Duration {
        self.time.current.saturating_sub(self.time.start)
    }

    /// 检查单独的时间条件
    pub fn time_needed(&self) -> bool {
        if let Some(expected_time) = self.time.expected {
            self.elapsed_time() >= expected_time
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

    /// 记录分割原因的调试信息
    fn log_segmentation_reason(&self, time_exceeded: bool, size_exceeded: bool) {
        match (time_exceeded, size_exceeded) {
            (true, true) => {
                tracing::info!(
                    "Segmentation needed: Both time ({:?} >= {:?}) and size ({} >= {}) conditions met",
                    self.elapsed_time(),
                    self.time.expected.unwrap(),
                    self.size.current,
                    self.size.expected.unwrap()
                );
            }
            (true, false) => {
                tracing::info!(
                    "Segmentation needed: Time condition met ({:?} >= {:?})",
                    self.elapsed_time(),
                    self.time.expected.unwrap()
                );
            }
            (false, true) => {
                tracing::info!(
                    "Segmentation needed: Size condition met ({} >= {})",
                    self.size.current,
                    self.size.expected.unwrap()
                );
            }
            (false, false) => {} // 不应该到达这里，因为只有在需要分割时才调用
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

    /// 格式化进度信息的通用方法
    fn format_progress<T>(
        label: &str,
        current: T,
        expected: Option<T>,
        unit: &str,
        format_fn: impl Fn(T) -> String,
    ) -> String
    where
        T: Copy + Into<f64>,
    {
        if let Some(expected_val) = expected {
            let current_f64 = current.into();
            let expected_f64 = expected_val.into();
            let percentage = (current_f64 / expected_f64 * 100.0).min(100.0);
            format!(
                "{}: {}/{} {} ({:.1}%)",
                label,
                format_fn(current),
                format_fn(expected_val),
                unit,
                percentage
            )
        } else {
            format!("{}: No limit", label)
        }
    }

    /// 获取当前状态信息
    pub fn get_status(&self) -> String {
        let time_info = Self::format_progress(
            "Time",
            self.elapsed_time().as_secs_f64(),
            self.time.expected.map(|d| d.as_secs_f64()),
            "s",
            |t| format!("{:.1}", t),
        );

        let size_info = Self::format_progress(
            "Size",
            self.size.current as f64,
            self.size.expected.map(|s| s as f64),
            "bytes",
            |s| format!("{}", s as u64),
        );

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

pub struct LifecycleFile<'a> {
    pub fmt_file_name: String,
    pub file_name: String,
    pub path: PathBuf,
    pub hook: CallbackFn<'a>,
    pub extension: &'static str,
    /// 写入这一系列分段文件的字节累计（跨分段，不随 `create_new` 归零）。
    pub bytes_written: ByteCounter,
}

impl<'a> LifecycleFile<'a> {
    pub fn new(fmt_file_name: &str, extension: &'static str) -> Self {
        Self::with_hook(fmt_file_name, extension, |_| {})
    }

    pub fn with_hook<F>(fmt_file_name: &str, extension: &'static str, hook: F) -> Self
    where
        F: FnMut(&str) + Send + Sync + 'a,
    {
        Self {
            fmt_file_name: fmt_file_name.to_string(),
            file_name: "".to_string(),
            path: Default::default(),
            hook: Box::new(hook),
            extension,
            bytes_written: ByteCounter::new(),
        }
    }

    /// 让写盘字节累计到调用方持有的计数器上（例如按录制任务汇总速率）。
    pub fn with_counter(mut self, counter: ByteCounter) -> Self {
        self.bytes_written = counter;
        self
    }

    pub fn create(&mut self) -> Result<&Path, std::io::Error> {
        // 构建最终文件名
        self.file_name = format!(
            "{}.{}",
            format_filename(&self.fmt_file_name),
            self.extension
        );

        // 构建临时文件路径（带 .part 后缀）
        self.path = PathBuf::from(&self.file_name);
        self.path.set_extension(format!("{}.part", self.extension));

        // 确保父目录存在
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        info!("Save to {}", self.path.display());
        Ok(self.path.as_path())
    }

    pub fn rename(&mut self) {
        // 去掉 .part 后缀
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
    use std::path::{Path, PathBuf};

    #[test]
    fn it_works() -> Result<(), Box<dyn std::error::Error>> {
        let mut p = PathBuf::from("/feel/the");

        p.set_extension("force");
        assert_eq!(Path::new("/feel/the.force"), p.as_path());

        p.set_extension("");
        assert_eq!(Path::new("/feel/the"), p.as_path());

        Ok(())
    }

    #[test]
    fn byte_counter_clones_share_one_total() {
        let counter = ByteCounter::new();
        let handle = counter.clone();
        counter.add(10);
        handle.add(5);
        assert_eq!(counter.total(), 15);
        assert_eq!(handle.total(), 15);

        let file = LifecycleFile::new("x", "flv").with_counter(counter.clone());
        file.bytes_written.add(1);
        assert_eq!(counter.total(), 16);
    }

    #[test]
    fn test_segmentation_logic() -> Result<(), Box<dyn std::error::Error>> {
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
