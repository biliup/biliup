use crate::downloader::index_tap::IndexTap;
use chrono::{DateTime, Local};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use std::time::Duration;
use tokio::sync::Notify;
use tracing::{error, info};

pub type CallbackFn<'a> = Box<dyn FnMut(&str) + Send + Sync + 'a>;

/// 已写盘字节的原子累计，供录制线程之外（如 Web 接口）读取实时速率。
///
/// 写盘路径上只做一次 `fetch_add` 和一次原子读：不 await、不会失败，
/// 因此不改变录制的任何控制流。有人通过 [`ByteCounter::watch`] 等新数据时才顺带
/// 唤醒它们（这时才会碰 [`Notify`] 内部的锁），没人等时不加锁。
/// `Clone` 得到的是同一计数器的另一个句柄。
#[derive(Debug, Clone, Default)]
pub struct ByteCounter(Arc<CounterInner>);

#[derive(Debug, Default)]
struct CounterInner {
    total: AtomicU64,
    watchers: AtomicUsize,
    grown: Notify,
}

impl ByteCounter {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn add(&self, bytes: u64) {
        // 与 `ByteWatch::grown_since` 的「先登记再读总数」配对，两边都用 SeqCst，
        // 保证等待方要么读到新总数、要么被这里唤醒
        self.0.total.fetch_add(bytes, Ordering::SeqCst);
        if self.0.watchers.load(Ordering::SeqCst) > 0 {
            self.0.grown.notify_waiters();
        }
    }

    pub fn total(&self) -> u64 {
        self.0.total.load(Ordering::Relaxed)
    }

    /// 订阅「又写了新数据」的通知（DVR 回看跟随正在写的分段时用）。
    pub fn watch(&self) -> ByteWatch {
        self.0.watchers.fetch_add(1, Ordering::SeqCst);
        ByteWatch(self.0.clone())
    }
}

/// [`ByteCounter::watch`] 的句柄，drop 时退订。
#[derive(Debug)]
pub struct ByteWatch(Arc<CounterInner>);

impl ByteWatch {
    /// 等到累计字节数超过 `seen`，返回新的累计值；已经超过就立即返回。
    pub async fn grown_since(&self, seen: u64) -> u64 {
        loop {
            let notified = self.0.grown.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let total = self.0.total.load(Ordering::SeqCst);
            if total > seen {
                return total;
            }
            notified.await;
        }
    }

    pub fn total(&self) -> u64 {
        self.0.total.load(Ordering::Relaxed)
    }
}

impl Drop for ByteWatch {
    fn drop(&mut self) {
        self.0.watchers.fetch_sub(1, Ordering::SeqCst);
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
    /// 每个分段文件 [`Self::create`] 时调用，参数是带 `.part` 的临时路径。
    pub start_hook: Option<CallbackFn<'a>>,
    pub extension: &'static str,
    /// 写入这一系列分段文件的字节累计（跨分段，不随 `create_new` 归零）。
    pub bytes_written: ByteCounter,
    /// 关键帧索引旁路：写盘处把每个分段写了什么交给索引任务，见 [`IndexTap`]。
    pub index: Option<IndexTap>,
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
            start_hook: None,
            extension,
            bytes_written: ByteCounter::new(),
            index: None,
        }
    }

    /// 开始写每个分段文件时回调（参数为 `.part` 临时路径）。
    pub fn with_start_hook<F>(mut self, hook: F) -> Self
    where
        F: FnMut(&str) + Send + Sync + 'a,
    {
        self.start_hook = Some(Box::new(hook));
        self
    }

    /// 让写盘字节累计到调用方持有的计数器上（例如按录制任务汇总速率）。
    pub fn with_counter(mut self, counter: ByteCounter) -> Self {
        self.bytes_written = counter;
        self
    }

    /// 边写边建关键帧索引，见 [`IndexTap`]。
    pub fn with_index_tap(mut self, index: Option<IndexTap>) -> Self {
        self.index = index;
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
        if let Some(hook) = self.start_hook.as_mut() {
            hook(&self.path.to_string_lossy());
        }
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

    /// 结束当前分段：先把 `writer` 缓冲的数据写进文件并检查错误，再去掉 `.part` 后缀、触发钩子，
    /// 钩子（上传、`FileValidator`）看到的文件大小即最终大小。
    ///
    /// flush 失败（如盘满）时，已经写进文件的部分照常改名交给钩子，不留下没人处理的 `.part`；
    /// 错误返回给调用方。
    pub fn finish(&mut self, writer: &mut impl Write) -> std::io::Result<()> {
        let flushed = writer.flush().map_err(|e| {
            std::io::Error::new(e.kind(), format!("flush {}: {e}", self.path.display()))
        });
        self.rename();
        flushed
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

    #[tokio::test]
    async fn byte_watch_wakes_on_growth_and_unsubscribes_on_drop() {
        let counter = ByteCounter::new();
        counter.add(3);
        let watch = counter.watch();
        assert_eq!(watch.grown_since(0).await, 3);

        let writer = counter.clone();
        let waiter = tokio::spawn(async move { watch.grown_since(3).await });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());
        writer.add(4);
        let total = tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("add 之后应当被唤醒")
            .unwrap();
        assert_eq!(total, 7);
        assert_eq!(counter.0.watchers.load(Ordering::SeqCst), 0);

        let watch = counter.watch();
        let waiter = tokio::spawn(async move { watch.grown_since(10).await });
        writer.add(2);
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished(), "还没超过给定的门槛");
        writer.add(2);
        let total = tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(total, 11);
    }

    /// flush 失败时不再静默：错误返回给调用方，已写入的部分照常改名交给钩子。
    #[test]
    fn finish_reports_a_failed_flush_and_still_hands_the_file_over() {
        struct FullDisk;
        impl std::io::Write for FullDisk {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::StorageFull.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::ErrorKind::StorageFull.into())
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let hooked: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
        let mut file = LifecycleFile::with_hook(dir.path().join("rec").to_str().unwrap(), "flv", {
            let hooked = hooked.clone();
            move |name: &str| hooked.lock().unwrap().push(name.to_string())
        });
        let part = file.create().unwrap().to_path_buf();
        fs::write(&part, b"FLV").unwrap();

        let err = file.finish(&mut FullDisk).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::StorageFull);
        assert!(err.to_string().contains("rec.flv.part"), "{err}");
        assert!(!part.exists());
        assert_eq!(*hooked.lock().unwrap(), vec![file.file_name.clone()]);
        assert_eq!(fs::read(&file.file_name).unwrap(), b"FLV");
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
