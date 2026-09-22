//! 录制写盘速率：各下载器在写出点往 [`ByteCounter`] 里累加，这里按固定间隔采样，
//! 用滑动窗口内首尾两次采样的差值算出 bytes/s，供 `/v1/streamers` 透出。
//!
//! 采样与计算都在录制热路径之外进行；写盘路径上只有一次原子加法。

use biliup::downloader::util::ByteCounter;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

/// 滑动窗口长度。httpflv 只在关键帧才把缓存的 tag 批量落盘，一个 GOP 可达数秒，
/// 窗口必须明显长于 GOP，否则速率会在 0 与峰值之间抖动。
pub const WINDOW: Duration = Duration::from_secs(10);
/// 采样间隔
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
/// 最新一次采样超过这个时长没有更新（采样任务已停）时不再报告速率
const STALE_AFTER: Duration = Duration::from_secs(3);

/// 单路录制的写盘速率表。
#[derive(Debug, Default)]
pub struct RateMeter {
    counter: ByteCounter,
    samples: Mutex<VecDeque<(Instant, u64)>>,
}

impl RateMeter {
    pub fn new() -> Self {
        Self::default()
    }

    /// 交给下载器累加的计数器句柄。
    pub fn counter(&self) -> ByteCounter {
        self.counter.clone()
    }

    /// 已写盘总字节数。
    pub fn total(&self) -> u64 {
        self.counter.total()
    }

    /// 记录一次采样并丢掉窗口之外的旧样本。
    pub fn sample(&self) {
        self.sample_at(Instant::now());
    }

    fn sample_at(&self, now: Instant) {
        let total = self.counter.total();
        let mut samples = self.samples.lock().unwrap();
        samples.push_back((now, total));
        while let Some(&(t, _)) = samples.front() {
            if now.duration_since(t) > WINDOW && samples.len() > 1 {
                samples.pop_front();
            } else {
                break;
            }
        }
    }

    /// 窗口内的平均写盘速率（字节/秒）。
    ///
    /// 采样不足两次、或采样已停止更新时返回 `None`，界面据此显示「—」而不是一个过期数字。
    pub fn bytes_per_sec(&self) -> Option<u64> {
        self.bytes_per_sec_at(Instant::now())
    }

    fn bytes_per_sec_at(&self, now: Instant) -> Option<u64> {
        let samples = self.samples.lock().unwrap();
        let &(newest_at, newest) = samples.back()?;
        if now.duration_since(newest_at) > STALE_AFTER {
            return None;
        }
        let &(oldest_at, oldest) = samples.front()?;
        let elapsed = newest_at.duration_since(oldest_at);
        if elapsed < SAMPLE_INTERVAL {
            return None;
        }
        let rate = newest.saturating_sub(oldest) as f64 / elapsed.as_secs_f64();
        Some(rate.round() as u64)
    }
}

/// 后台按 [`SAMPLE_INTERVAL`] 采样，直到句柄被 drop（随之 abort）。
pub struct Sampler(JoinHandle<()>);

impl Sampler {
    pub fn spawn(meter: Arc<RateMeter>) -> Self {
        Self(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(SAMPLE_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                meter.sample();
            }
        }))
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// 子进程落盘的下载器（ffmpeg / streamlink）不经过本进程写盘，
/// 改为旁路观察输出文件长度：每秒读一次 `.part` 的大小，把增量累加到计数器。
/// 只读元数据，不碰子进程也不碰文件内容；文件尚未创建时静默等待。
/// 返回的句柄 drop 时停止观察。
pub struct FileSizeProbe(JoinHandle<()>);

impl FileSizeProbe {
    pub fn spawn(path: impl Into<PathBuf>, counter: ByteCounter) -> Self {
        let path = path.into();
        Self(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(SAMPLE_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut last_len = 0u64;
            loop {
                ticker.tick().await;
                last_len = probe_once(&path, last_len, &counter).await;
            }
        }))
    }
}

impl Drop for FileSizeProbe {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// 读一次文件长度，把相对上次的增量累加到计数器，返回本次长度供下次差分。
/// 文件不存在或长度回退（重建）时不累加，只把基准归零。
async fn probe_once(path: &Path, last_len: u64, counter: &ByteCounter) -> u64 {
    match tokio::fs::metadata(path).await {
        Ok(meta) => {
            let len = meta.len();
            if len > last_len {
                counter.add(len - last_len);
            }
            len
        }
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_two_samples_at_least_one_interval_apart() {
        let meter = RateMeter::new();
        let t0 = Instant::now();
        assert_eq!(meter.bytes_per_sec_at(t0), None);

        meter.counter().add(1000);
        meter.sample_at(t0);
        assert_eq!(meter.bytes_per_sec_at(t0), None, "只有一个样本算不出速率");

        meter.counter().add(2000);
        meter.sample_at(t0 + Duration::from_secs(2));
        assert_eq!(
            meter.bytes_per_sec_at(t0 + Duration::from_secs(2)),
            Some(1000)
        );
    }

    #[test]
    fn rate_is_averaged_over_the_window_and_old_samples_fall_off() {
        let meter = RateMeter::new();
        let t0 = Instant::now();
        // 每秒写 100 字节，持续 30 秒
        for i in 0..=30u64 {
            meter.sample_at(t0 + Duration::from_secs(i));
            meter.counter().add(100);
        }
        let now = t0 + Duration::from_secs(30);
        assert_eq!(meter.bytes_per_sec_at(now), Some(100));
        assert!(
            meter.samples.lock().unwrap().len() <= (WINDOW.as_secs() as usize) + 1,
            "窗口之外的样本应被丢掉"
        );

        // 停写超过一个窗口后窗口内没有增量，速率归零而不是沿用旧值
        for i in 31..=41u64 {
            meter.sample_at(t0 + Duration::from_secs(i));
        }
        assert_eq!(
            meter.bytes_per_sec_at(t0 + Duration::from_secs(41)),
            Some(0)
        );
    }

    #[test]
    fn a_stale_meter_reports_nothing() {
        let meter = RateMeter::new();
        let t0 = Instant::now();
        meter.sample_at(t0);
        meter.counter().add(500);
        meter.sample_at(t0 + Duration::from_secs(1));
        assert_eq!(
            meter.bytes_per_sec_at(t0 + Duration::from_secs(2)),
            Some(500)
        );
        assert_eq!(
            meter.bytes_per_sec_at(t0 + Duration::from_secs(10)),
            None,
            "采样任务停了之后不能继续报旧速率"
        );
    }

    #[tokio::test]
    async fn file_probe_accumulates_growth_and_tolerates_missing_or_rewritten_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.flv.part");
        let counter = ByteCounter::new();

        // 文件还不存在：不累加，基准为 0
        assert_eq!(probe_once(&path, 0, &counter).await, 0);
        assert_eq!(counter.total(), 0);

        std::fs::write(&path, vec![0u8; 100]).unwrap();
        let len = probe_once(&path, 0, &counter).await;
        assert_eq!((len, counter.total()), (100, 100));

        std::fs::write(&path, vec![0u8; 250]).unwrap();
        let len = probe_once(&path, len, &counter).await;
        assert_eq!((len, counter.total()), (250, 250));

        // 文件被重建变短：不倒扣，只重置基准
        std::fs::write(&path, vec![0u8; 30]).unwrap();
        let len = probe_once(&path, len, &counter).await;
        assert_eq!((len, counter.total()), (30, 250));
    }
}
