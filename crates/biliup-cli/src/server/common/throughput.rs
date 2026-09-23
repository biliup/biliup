//! 录制写盘速率：进程内写盘的下载器在写出点往 [`ByteCounter`] 里累加，子进程落盘的
//! 下载器（ffmpeg / streamlink）由 [`SubprocessProgress`] 解析其 stderr 进度行累加；
//! [`RateMeter`] 按固定间隔采样，用滑动窗口内首尾两次采样的差值算出 bytes/s，
//! 供 `/v1/streamers` 透出。
//!
//! 采样与计算都在录制热路径之外进行；写盘路径上只有一次原子加法。

use biliup::downloader::util::ByteCounter;
use std::collections::VecDeque;
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

    pub(crate) fn sample_at(&self, now: Instant) {
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

/// 子进程落盘的下载器（ffmpeg / streamlink）不经过本进程写盘，改为解析它们打到 stderr 的
/// 进度行：两者都会周期性报告「本次进程累计写出多少字节」，这里把增量累加到计数器。
/// 不读盘、不新开任务，就在既有的 stderr 读取协程里顺带解析；解析不出来只是没有速率，
/// 不影响录制。
#[derive(Debug, Default)]
pub struct SubprocessProgress {
    /// 当前子进程最近一次报告的累计字节数
    last_total: u64,
}

/// ffmpeg `-progress` 输出的字段名（`stream_N_M_q` 另行按前后缀匹配）
const FFMPEG_PROGRESS_KEYS: &[&str] = &[
    "frame",
    "fps",
    "bitrate",
    "total_size",
    "out_time_us",
    "out_time_ms",
    "out_time",
    "dup_frames",
    "drop_frames",
    "speed",
    "progress",
];

impl SubprocessProgress {
    /// 记录一次累计值：只加增量；回退（换了输出文件）时只重置基准，不倒扣。
    pub fn advance_to(&mut self, total: u64, counter: &ByteCounter) {
        if total > self.last_total {
            counter.add(total - self.last_total);
        }
        self.last_total = total;
    }

    /// ffmpeg `-progress pipe:2` 的 `key=value` 行，其中 `total_size=N` 是累计写出字节。
    /// 返回 true 表示这一行是进度行（调用方不必再当普通日志打印）。
    pub fn observe_ffmpeg(&mut self, line: &str, counter: &ByteCounter) -> bool {
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        let key = key.trim();
        let is_progress_key = FFMPEG_PROGRESS_KEYS.contains(&key)
            || (key.starts_with("stream_") && key.ends_with("_q"));
        if !is_progress_key {
            return false;
        }
        if key == "total_size"
            && let Ok(total) = value.trim().parse::<u64>()
        {
            self.advance_to(total, counter);
        }
        true
    }

    /// streamlink `--progress=force` 的 `[download] Written 12.34 MiB to <file> (5s @ 2.50 MiB/s)`。
    /// 返回 true 表示这一行是进度行。
    pub fn observe_streamlink(&mut self, line: &str, counter: &ByteCounter) -> bool {
        let Some(total) = streamlink_written_bytes(line) else {
            return false;
        };
        self.advance_to(total, counter);
        true
    }
}

/// 从 streamlink 进度行里取出累计写出字节数（`Written 12.34 MiB`，单位为 bytes / KiB / MiB / GiB / TiB）
pub fn streamlink_written_bytes(line: &str) -> Option<u64> {
    let rest = line.split_once("[download]")?.1.trim_start();
    let rest = rest.strip_prefix("Written")?.trim_start();
    let mut parts = rest.splitn(3, ' ');
    let amount: f64 = parts.next()?.parse().ok()?;
    let unit = parts.next()?;
    let scale: f64 = match unit {
        "bytes" | "B" => 1.0,
        "KiB" | "KB" => 1024.0,
        "MiB" | "MB" => 1024.0 * 1024.0,
        "GiB" | "GB" => 1024.0 * 1024.0 * 1024.0,
        "TiB" | "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((amount * scale).round() as u64)
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

    #[test]
    fn ffmpeg_progress_lines_feed_total_size_and_are_recognised() {
        let counter = ByteCounter::new();
        let mut progress = SubprocessProgress::default();
        // 实测 ffmpeg 6.1 `-progress pipe:2` 的一个进度块
        let block = [
            "frame=0",
            "fps=0.00",
            "stream_0_0_q=0.0",
            "bitrate=N/A",
            "total_size=N/A",
            "out_time_us=N/A",
            "out_time_ms=N/A",
            "out_time=N/A",
            "dup_frames=0",
            "drop_frames=0",
            "speed=N/A",
            "progress=continue",
        ];
        for line in block {
            assert!(
                progress.observe_ffmpeg(line, &counter),
                "{line} 应被识别为进度行"
            );
        }
        assert_eq!(counter.total(), 0, "N/A 不计");

        assert!(progress.observe_ffmpeg("total_size=63269", &counter));
        assert!(progress.observe_ffmpeg("total_size=100000", &counter));
        assert_eq!(counter.total(), 100_000);
        // 新进程从 0 重新计：不倒扣，只重置基准
        assert!(progress.observe_ffmpeg("total_size=5000", &counter));
        assert_eq!(counter.total(), 100_000);
        assert!(progress.observe_ffmpeg("total_size=7000", &counter));
        assert_eq!(counter.total(), 102_000);

        // 普通日志行不是进度行
        for line in [
            "ffmpeg version 6.1.1 Copyright (c) 2000-2023",
            "[flv @ 0x55] Failed to update header with correct duration.",
            "Input #0, flv, from 'https://x/y.flv?a=b':",
            "",
        ] {
            assert!(!progress.observe_ffmpeg(line, &counter), "{line:?}");
        }
    }

    #[test]
    fn streamlink_progress_lines_are_parsed_in_binary_units() {
        assert_eq!(
            streamlink_written_bytes("[download] Written 61.78 KiB to /tmp/x.flv.part (0s)"),
            Some(63_263)
        );
        assert_eq!(
            streamlink_written_bytes(
                "[download] Written 488.00 KiB to /tmp/x.flv.part (4s @ 243.92 KiB/s)"
            ),
            Some(499_712)
        );
        assert_eq!(
            streamlink_written_bytes("[download] Written 1.50 MiB to a b c.ts (9s @ 1.2 MiB/s)"),
            Some(1_572_864)
        );
        assert_eq!(
            streamlink_written_bytes("[download] Written 2.00 GiB to x (1h)"),
            Some(2 * 1024 * 1024 * 1024)
        );
        assert_eq!(
            streamlink_written_bytes("[download] Written 512 bytes to x (0s)"),
            Some(512)
        );
        for line in [
            "[cli][info] Found matching plugin http for URL httpstream://...",
            "[cli][info] Writing output to",
            "[download] Written weird MiB to x",
            "",
        ] {
            assert_eq!(streamlink_written_bytes(line), None, "{line:?}");
        }

        let counter = ByteCounter::new();
        let mut progress = SubprocessProgress::default();
        assert!(progress.observe_streamlink(
            "[download] Written 240.00 KiB to /tmp/x.flv.part (2s)",
            &counter
        ));
        assert!(progress.observe_streamlink(
            "[download] Written 488.00 KiB to /tmp/x.flv.part (4s @ 243.92 KiB/s)",
            &counter
        ));
        assert_eq!(counter.total(), 499_712);
        assert!(!progress.observe_streamlink("[cli][info] Stream ended", &counter));
    }
}
