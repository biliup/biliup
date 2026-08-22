//! 录制准入策略：集中回答「现在该不该录这个主播」。
//!
//! Python 版把这些条件放在 `should_record()` 里（`biliup/engine/download.py`）。
//! Rust 重写时这个概念整体消失了，于是 `time_range` 和 `excluded_keywords`
//! 虽然一路存到了数据库和界面，运行时却没有任何代码读它们（issue #1654）。
//! 这个模块把「录制前提」重新变成一个有名字、有测试、可枚举的东西：
//! 新增条件时只要加一个 [`Rejection`] 分支和一行判定，三个调用点会一起生效。
//!
//! 判定分两级，依据是**这个条件需要哪些信息**，而这决定了它最早能在什么时候求值：
//!
//! * [`reject_before_probe`] —— 只看主播配置和时钟，开播探测之前就能判定，
//!   因此可以在不占用下载槽位、不发任何请求的情况下把房间筛掉。
//! * [`reject_before_record`] —— 还要看直播流信息（房间标题），只能在
//!   `check_stream` 拿到流之后判定；它是前者的超集。
//!
//! Python 版把两者混在一个探测之后的 `should_record()` 里，所以不在录制时间范围内
//! 的主播照样要发一次网络请求；这里按信息依赖分级，避免了那次无谓的请求。

use std::fmt;

use serde_json::Value;

use crate::server::common::timerange;
use crate::server::infrastructure::models::live_streamer::LiveStreamer;

/// 不予录制的原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    /// 当前不在配置的录制时间范围内
    OutOfTimeRange,
    /// 房间标题命中了排除关键词
    ExcludedKeyword(String),
}

impl Rejection {
    /// `/v1/streamers` 返回给前端的状态字符串。
    ///
    /// `OutOfSchedule` 沿用 Python 版 `biliup/web/__init__.py` 的取值，
    /// 前端 `app/(app)/streamers/page.tsx` 一直保留着对应的「非录播时间」标签。
    pub fn status(&self) -> &'static str {
        match self {
            Rejection::OutOfTimeRange => "OutOfSchedule",
            Rejection::ExcludedKeyword(_) => "TitleExcluded",
        }
    }
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejection::OutOfTimeRange => f.write_str("不在录制时间范围内"),
            Rejection::ExcludedKeyword(keyword) => {
                write!(f, "房间标题命中排除关键词 {keyword:?}")
            }
        }
    }
}

/// 只依赖主播配置和时钟的条件，开播探测之前即可判定。
///
/// 监控循环在占用下载槽位、发出任何请求之前调用它。
pub fn reject_before_probe(streamer: &LiveStreamer) -> Option<Rejection> {
    if !timerange::is_within(streamer.time_range.as_deref()) {
        return Some(Rejection::OutOfTimeRange);
    }
    None
}

/// 需要直播流信息才能判定的条件，是 [`reject_before_probe`] 的超集。
///
/// 在 `check_stream` 取得流之后、真正开录之前调用，续录前每轮也要重新判定
/// ——房间标题和当前时间都可能在录制过程中变化。
pub fn reject_before_record(streamer: &LiveStreamer, title: &str) -> Option<Rejection> {
    if let Some(rejection) = reject_before_probe(streamer) {
        return Some(rejection);
    }
    if let Some(keyword) = matched_excluded_keyword(streamer, title) {
        return Some(Rejection::ExcludedKeyword(keyword));
    }
    None
}

/// 房间标题命中的第一个排除关键词。
///
/// 与 Python 版一致地做大小写敏感的子串匹配并去掉关键词首尾空白；
/// 但会跳过空关键词——Python 版 `"" in title` 恒真，一个空字符串就会让该主播完全录不到。
fn matched_excluded_keyword(streamer: &LiveStreamer, title: &str) -> Option<String> {
    if title.is_empty() {
        return None;
    }
    streamer
        .excluded_keywords
        .as_ref()
        .and_then(Value::as_array)?
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|keyword| !keyword.is_empty())
        .find(|keyword| title.contains(*keyword))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, SecondsFormat, Utc};

    /// 以当前时刻为基准造窗口，形态与前端 `Date.toISOString()` 写出的一致
    fn window(starts_in: i64, ends_in: i64) -> String {
        let now = Utc::now();
        let iso = |offset: i64| {
            (now + Duration::seconds(offset)).to_rfc3339_opts(SecondsFormat::Millis, true)
        };
        format!(r#"["{}","{}"]"#, iso(starts_in), iso(ends_in))
    }

    fn streamer(time_range: Option<String>, excluded_keywords: Option<Value>) -> LiveStreamer {
        LiveStreamer {
            id: 1,
            url: "https://www.douyu.com/5146843".to_string(),
            remark: "test".to_string(),
            filename_prefix: None,
            time_range,
            upload_streamers_id: None,
            format: None,
            override_cfg: None,
            preprocessor: None,
            segment_processor: None,
            downloaded_processor: None,
            postprocessor: None,
            opt_args: None,
            excluded_keywords,
        }
    }

    fn keywords(values: &[&str]) -> Option<Value> {
        Some(Value::Array(
            values
                .iter()
                .map(|k| Value::String((*k).to_string()))
                .collect(),
        ))
    }

    #[test]
    fn a_streamer_without_any_condition_is_always_recorded() {
        let streamer = streamer(None, None);
        assert_eq!(reject_before_probe(&streamer), None);
        assert_eq!(reject_before_record(&streamer, "随便什么标题"), None);
    }

    #[test]
    fn an_open_window_and_a_clean_title_are_recorded() {
        let streamer = streamer(Some(window(-60, 3600)), keywords(&["录像", "重播"]));
        assert_eq!(reject_before_probe(&streamer), None);
        assert_eq!(reject_before_record(&streamer, "今天来点好活"), None);
    }

    #[test]
    fn a_closed_window_is_rejected_before_the_probe() {
        // 关键性质：这个条件在探测之前就能判定，才能省掉下载槽位和网络请求
        let streamer = streamer(Some(window(3600, 7200)), None);
        assert_eq!(
            reject_before_probe(&streamer),
            Some(Rejection::OutOfTimeRange)
        );
    }

    #[test]
    fn an_excluded_keyword_is_only_visible_after_the_probe() {
        // 标题类条件探测前无从判断，探测后才拒绝
        let streamer = streamer(None, keywords(&["录像"]));
        assert_eq!(reject_before_probe(&streamer), None);
        assert_eq!(
            reject_before_record(&streamer, "昨日录像回放"),
            Some(Rejection::ExcludedKeyword("录像".to_string()))
        );
    }

    #[test]
    fn the_post_probe_check_still_enforces_the_pre_probe_conditions() {
        // reject_before_record 必须是 reject_before_probe 的超集，
        // 否则新增一个「探测前条件」时会在开录那一步漏掉
        let streamer = streamer(Some(window(3600, 7200)), None);
        assert_eq!(
            reject_before_record(&streamer, "无关标题"),
            Some(Rejection::OutOfTimeRange)
        );
    }

    #[test]
    fn keywords_are_matched_as_trimmed_substrings() {
        let streamer = streamer(None, keywords(&["  重播  "]));
        assert_eq!(
            reject_before_record(&streamer, "这是重播内容"),
            Some(Rejection::ExcludedKeyword("重播".to_string()))
        );
    }

    #[test]
    fn an_empty_keyword_does_not_block_everything() {
        // Python 版 `"" in title` 恒真，一个空关键词会让主播彻底录不到
        let streamer = streamer(None, keywords(&["", "   "]));
        assert_eq!(reject_before_record(&streamer, "正常直播"), None);
    }

    #[test]
    fn an_empty_title_skips_the_keyword_check() {
        let streamer = streamer(None, keywords(&["录像"]));
        assert_eq!(reject_before_record(&streamer, ""), None);
    }

    #[test]
    fn malformed_keywords_fall_open_instead_of_blocking() {
        for raw in [
            Value::Null,
            Value::String("录像".to_string()),
            Value::Number(1.into()),
            Value::Array(vec![Value::Number(1.into())]),
        ] {
            let streamer = streamer(None, Some(raw.clone()));
            assert_eq!(
                reject_before_record(&streamer, "昨日录像回放"),
                None,
                "raw={raw}"
            );
        }
    }

    #[test]
    fn keyword_matching_is_case_sensitive_like_python() {
        let streamer = streamer(None, keywords(&["Replay"]));
        assert_eq!(reject_before_record(&streamer, "replay time"), None);
        assert_eq!(
            reject_before_record(&streamer, "Replay time"),
            Some(Rejection::ExcludedKeyword("Replay".to_string()))
        );
    }

    #[test]
    fn rejections_carry_the_status_string_the_frontend_expects() {
        assert_eq!(Rejection::OutOfTimeRange.status(), "OutOfSchedule");
        assert_eq!(
            Rejection::ExcludedKeyword("x".to_string()).status(),
            "TitleExcluded"
        );
    }
}
