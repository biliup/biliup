//! 录制时间范围（`time_range`）的解析与判定。
//!
//! 存储格式是一个含两个 ISO 8601 时刻的 JSON 数组字符串，例如
//! `["2025-03-26T16:00:00.000Z","2025-03-27T15:59:59.000Z"]`。
//!
//! 前端时间选择器拿到的是用户本地时刻，写库前经 `Date.toISOString()` 转成 UTC
//! （`app/ui/TemplateModal.tsx`），所以两端存的都是 **UTC 时刻**，日期部分只是
//! 选择当天的残留、没有意义。判定时统一取 UTC 当日秒数比较，正好抵消前端写入时
//! 的时区偏移，与 Python 版 `check_timerange()` 用 `datetime.now(timezone.utc)`
//! 比较的行为一致。若改用本地时间判定，UTC+8 用户配置的 16:00–20:00 会被当成
//! 08:00–12:00，整整错开 8 小时。

use chrono::{DateTime, NaiveDateTime, Timelike, Utc};
use tracing::debug;

const SECONDS_PER_DAY: u32 = 24 * 60 * 60;

/// 一个录制窗口，端点为 UTC 当日秒数的闭区间。
/// `start > end` 表示跨午夜区间（如 23:00 → 04:00）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    start: u32,
    end: u32,
}

impl TimeRange {
    /// 解析存储的 `time_range` 字符串，任何形式的非法输入都返回 `None`。
    pub fn parse(raw: &str) -> Option<Self> {
        let bounds: Vec<String> = serde_json::from_str(raw)
            .inspect_err(|e| debug!(raw = raw, error = %e, "time_range 不是字符串数组，忽略"))
            .ok()?;
        let [start, end] = <[String; 2]>::try_from(bounds)
            .inspect_err(|_| debug!(raw = raw, "time_range 需要恰好两个时刻，忽略"))
            .ok()?;
        Some(Self {
            start: utc_seconds_from_midnight(&start)?,
            end: utc_seconds_from_midnight(&end)?,
        })
    }

    /// `now`（UTC 当日秒数）是否落在窗口内。
    fn contains(&self, now: u32) -> bool {
        if self.start <= self.end {
            // 普通区间，如 16:00 → 20:00
            self.start <= now && now <= self.end
        } else {
            // 跨午夜区间，如 23:00 → 04:00
            now >= self.start || now <= self.end
        }
    }

    /// 距窗口结束还有多少秒；不在窗口内返回 0。
    ///
    /// 跨午夜窗口在午夜之前的那段里 `end < now`，必须跨到次日的结束时刻才对。
    /// 但「已经错过窗口」同样满足 `end < now`，不先判定在不在窗口内就会算出
    /// 「还剩将近 24 小时」——正好是这个功能要防的那种失控录制。
    fn seconds_until_end(&self, now: u32) -> u32 {
        if !self.contains(now) {
            return 0;
        }
        if self.end >= now {
            self.end - now
        } else {
            SECONDS_PER_DAY - now + self.end
        }
    }
}

/// 取 ISO 8601 时刻换算到 UTC 后的当日秒数。
///
/// 前端写入的一定带 `Z`；手写配置若带其他偏移量（如 `+08:00`）同样先换算到 UTC，
/// 完全不带时区信息时按 UTC 解释。
fn utc_seconds_from_midnight(raw: &str) -> Option<u32> {
    let time = match DateTime::parse_from_rfc3339(raw) {
        Ok(dt) => dt.with_timezone(&Utc).time(),
        Err(_) => raw
            .parse::<NaiveDateTime>()
            .inspect_err(|e| debug!(raw = raw, error = %e, "time_range 时刻无法解析，忽略"))
            .ok()?
            .time(),
    };
    Some(time.num_seconds_from_midnight())
}

fn now_seconds_from_midnight() -> u32 {
    Utc::now().time().num_seconds_from_midnight()
}

/// 当前是否处于配置的录制时间范围内。
///
/// 未配置或配置无法解析时一律放行：坏配置只应让时间范围失效，不该让主播完全录不到。
/// 与 Python 版 `check_timerange()` 出错即 `return True` 的取舍一致。
pub fn is_within(time_range: Option<&str>) -> bool {
    is_within_at(time_range, now_seconds_from_midnight())
}

fn is_within_at(time_range: Option<&str>, now: u32) -> bool {
    match time_range.and_then(TimeRange::parse) {
        Some(range) => range.contains(now),
        None => true,
    }
}

/// 距录制时间范围结束还剩多久（`"HH:MM:SS"`）；未配置或配置非法时为 `None`。
///
/// 给「自己不会退出」的录制方式收尾用（如 ffmpeg 内部分段由 segment muxer 持续切片），
/// 需要一个总时长上限把进程截停在窗口边界。
pub fn remaining_until_end(time_range: Option<&str>) -> Option<String> {
    remaining_until_end_at(time_range, now_seconds_from_midnight())
}

fn remaining_until_end_at(time_range: Option<&str>, now: u32) -> Option<String> {
    let range = time_range.and_then(TimeRange::parse)?;
    Some(format_hms(range.seconds_until_end(now)))
}

/// 计算本次录制块允许的最长时长（`"HH:MM:SS"`）。
///
/// 在 `segment_time` 的基础上按窗口结束时刻裁剪：快到结束时间时缩短本段，
/// 让录制正好停在窗口边界而不是冲出去一整段。等价于 Python 版 `get_duration()`，
/// 但额外覆盖了「配了时间范围却没配分段时长」的情况——Python 那种情形下
/// `-to` 不会下发，录制会一直冲到直播结束。
pub fn clamp_segment_time(segment_time: Option<&str>, time_range: Option<&str>) -> Option<String> {
    clamp_segment_time_at(segment_time, time_range, now_seconds_from_midnight())
}

fn clamp_segment_time_at(
    segment_time: Option<&str>,
    time_range: Option<&str>,
    now: u32,
) -> Option<String> {
    let Some(range) = time_range.and_then(TimeRange::parse) else {
        return segment_time.map(str::to_owned);
    };
    let remaining = range.seconds_until_end(now);

    match segment_time {
        // 分段时长无法解析时原样下发，交给下游工具自己报错
        Some(configured) => match parse_hms(configured) {
            Some(seconds) if seconds > remaining => Some(format_hms(remaining)),
            Some(_) | None => Some(configured.to_owned()),
        },
        None => Some(format_hms(remaining)),
    }
}

/// 解析 `"HH:MM:SS"` 为秒数。
fn parse_hms(raw: &str) -> Option<u32> {
    let [hours, minutes, seconds] =
        <[&str; 3]>::try_from(raw.split(':').collect::<Vec<_>>()).ok()?;
    Some(
        hours.parse::<u32>().ok()? * 3600
            + minutes.parse::<u32>().ok()? * 60
            + seconds.parse::<u32>().ok()?,
    )
}

fn format_hms(total: u32) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 把 "HH:MM:SS" 写成当日秒数，让用例读起来贴近配置里的时刻
    fn at(hms: &str) -> u32 {
        parse_hms(hms).expect("测试时刻格式应合法")
    }

    /// 前端 `Date.toISOString()` 写出的形态
    fn range(start: &str, end: &str) -> String {
        format!(r#"["2025-03-26T{start}.000Z","2025-03-27T{end}.000Z"]"#)
    }

    #[test]
    fn missing_time_range_does_not_restrict_recording() {
        assert!(is_within_at(None, at("03:00:00")));
        assert_eq!(
            clamp_segment_time_at(Some("01:00:00"), None, at("03:00:00")),
            Some("01:00:00".to_string())
        );
    }

    #[test]
    fn malformed_time_range_falls_open_instead_of_blocking() {
        // 坏配置只让时间范围失效，不能把主播整个录制掐掉
        for raw in [
            "",
            "null",
            "not json",
            "[]",
            r#"["2025-03-26T16:00:00.000Z"]"#,
            r#"["a","b"]"#,
            r#"["2025-03-26T16:00:00.000Z","2025-03-26T16:00:00.000Z","x"]"#,
        ] {
            assert!(is_within_at(Some(raw), at("03:00:00")), "raw={raw}");
            assert_eq!(
                clamp_segment_time_at(Some("01:00:00"), Some(raw), at("03:00:00")),
                Some("01:00:00".to_string()),
                "raw={raw}"
            );
        }
    }

    #[test]
    fn normal_interval_admits_only_inside_the_window() {
        let tr = range("16:00:00", "20:00:00");
        assert!(!is_within_at(Some(&tr), at("15:59:59")));
        assert!(is_within_at(Some(&tr), at("16:00:00"))); // 起点闭区间
        assert!(is_within_at(Some(&tr), at("18:00:00")));
        assert!(is_within_at(Some(&tr), at("20:00:00"))); // 终点闭区间
        assert!(!is_within_at(Some(&tr), at("20:00:01")));
    }

    #[test]
    fn cross_midnight_interval_admits_both_sides_of_midnight() {
        let tr = range("23:00:00", "04:00:00");
        assert!(is_within_at(Some(&tr), at("23:30:00")));
        assert!(is_within_at(Some(&tr), at("00:00:00")));
        assert!(is_within_at(Some(&tr), at("03:59:59")));
        assert!(!is_within_at(Some(&tr), at("04:00:01")));
        assert!(!is_within_at(Some(&tr), at("12:00:00")));
        assert!(!is_within_at(Some(&tr), at("22:59:59")));
    }

    /// 前端存的是 UTC，判定也必须按 UTC，否则 UTC+8 用户会整体错开 8 小时
    #[test]
    fn offset_bearing_instants_are_converted_to_utc() {
        // 16:00+08:00 == 08:00Z
        let tr = r#"["2025-03-26T16:00:00+08:00","2025-03-26T20:00:00+08:00"]"#;
        assert!(is_within_at(Some(tr), at("09:00:00")));
        assert!(!is_within_at(Some(tr), at("17:00:00")));
    }

    #[test]
    fn naive_instants_are_read_as_utc() {
        let tr = r#"["2025-03-26T16:00:00","2025-03-26T20:00:00"]"#;
        assert!(is_within_at(Some(tr), at("17:00:00")));
        assert!(!is_within_at(Some(tr), at("09:00:00")));
    }

    #[test]
    fn segment_time_is_untouched_while_the_window_end_is_far() {
        let tr = range("16:00:00", "20:00:00");
        assert_eq!(
            clamp_segment_time_at(Some("01:00:00"), Some(&tr), at("16:30:00")),
            Some("01:00:00".to_string())
        );
    }

    #[test]
    fn segment_time_is_shortened_near_the_window_end() {
        let tr = range("16:00:00", "20:00:00");
        // 19:40 距结束还有 20 分钟，小于 1 小时的分段时长
        assert_eq!(
            clamp_segment_time_at(Some("01:00:00"), Some(&tr), at("19:40:00")),
            Some("00:20:00".to_string())
        );
    }

    #[test]
    fn segment_time_matching_the_remaining_time_is_not_shortened() {
        let tr = range("16:00:00", "20:00:00");
        assert_eq!(
            clamp_segment_time_at(Some("01:00:00"), Some(&tr), at("19:00:00")),
            Some("01:00:00".to_string())
        );
    }

    #[test]
    fn missing_segment_time_still_ends_at_the_window_end() {
        // Python 版这种情况不下发 -to，会一直录到直播结束；这里补上
        let tr = range("16:00:00", "20:00:00");
        assert_eq!(
            clamp_segment_time_at(None, Some(&tr), at("19:40:00")),
            Some("00:20:00".to_string())
        );
    }

    #[test]
    fn remaining_time_of_a_cross_midnight_window_wraps_past_zero() {
        let tr = range("23:00:00", "04:00:00");
        // 23:30 距次日 04:00 还有 4 小时 30 分
        assert_eq!(
            clamp_segment_time_at(Some("10:00:00"), Some(&tr), at("23:30:00")),
            Some("04:30:00".to_string())
        );
    }

    #[test]
    fn unparsable_segment_time_is_passed_through() {
        let tr = range("16:00:00", "20:00:00");
        for configured in ["", "abc", "1:2", "01:00:00:00", "-1:00:00"] {
            assert_eq!(
                clamp_segment_time_at(Some(configured), Some(&tr), at("19:40:00")),
                Some(configured.to_string()),
                "configured={configured}"
            );
        }
    }

    #[test]
    fn sitting_exactly_on_the_window_end_leaves_zero_remaining() {
        let tr = range("16:00:00", "20:00:00");
        assert_eq!(
            clamp_segment_time_at(Some("01:00:00"), Some(&tr), at("20:00:00")),
            Some("00:00:00".to_string())
        );
    }

    /// 窗口外 `end < now` 与跨午夜窗口的形态相同，若不先判定在不在窗口内，
    /// 会算出「还剩近 24 小时」，反而放行一整天的录制
    #[test]
    fn outside_the_window_nothing_remains_instead_of_wrapping_to_tomorrow() {
        let tr = range("16:00:00", "20:00:00");
        assert_eq!(
            clamp_segment_time_at(Some("01:00:00"), Some(&tr), at("20:00:01")),
            Some("00:00:00".to_string())
        );
        assert_eq!(
            remaining_until_end_at(Some(&tr), at("06:00:00")),
            Some("00:00:00".to_string())
        );
    }
}
