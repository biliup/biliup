//! 场次键与场次对齐（ha-pair 方案 §2）。
//!
//! 主机在线时由主机在开播时生成 `{房间 id}:{开播毫秒}` 推给备机，备机对同一房间、时间对得上的场次采用它；
//! 主机离线时备机自己生成 `standby:{房间 id}:{开播毫秒}`，主机回来后按房间 + 时间重叠对齐。
//! 两台机器的时钟可以差几分钟：「时间对得上」用 `live_merge_minutes`（默认 10 分钟）做容差。

pub const STANDBY_PREFIX: &str = "standby:";

pub fn primary_key(room: i64, started_at: i64) -> String {
    format!("{room}:{started_at}")
}

pub fn standby_key(room: i64, started_at: i64) -> String {
    format!("{STANDBY_PREFIX}{room}:{started_at}")
}

pub fn is_standby_key(key: &str) -> bool {
    key.starts_with(STANDBY_PREFIX)
}

/// 键里的房间与开播时刻
pub fn parse(key: &str) -> Option<(i64, i64)> {
    let rest = key.strip_prefix(STANDBY_PREFIX).unwrap_or(key);
    let (room, started_at) = rest.split_once(':')?;
    Some((room.parse().ok()?, started_at.parse().ok()?))
}

/// 一段录制在时间轴上的范围；`end` 为空表示还在录（或主机中途离线、不知道何时结束）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: i64,
    pub end: Option<i64>,
}

impl Span {
    pub fn new(start: i64, end: Option<i64>) -> Self {
        Span { start, end }
    }
}

/// 判断时间重叠时容忍的两台机器时钟差
pub const SKEW_MS: i64 = 2 * 60 * 1000;

/// 同一房间的两段录制是不是同一场：开播时刻相差不超过 `window_ms`，或两段真的重叠（容差 [`SKEW_MS`]）。
/// 重叠只容忍时钟差、不用合并窗口：下播后几分钟又开播的下一场不能算进上一场。
/// 没有结束时刻的一段按录到 `now` 算
pub fn same_session(a: Span, b: Span, window_ms: i64, now: i64) -> bool {
    if (a.start - b.start).abs() <= window_ms {
        return true;
    }
    let a_end = a.end.unwrap_or(now);
    let b_end = b.end.unwrap_or(now);
    a.start < b_end.saturating_add(SKEW_MS) && b.start < a_end.saturating_add(SKEW_MS)
}

/// 在 `candidates`（键, 范围）里找开播时刻与 `span` 相差不超过 `window_ms`、最接近的那个（备机采用主机的场次键）
pub fn best_match<'a>(
    span: Span,
    candidates: impl IntoIterator<Item = (&'a str, Span)>,
    window_ms: i64,
) -> Option<&'a str> {
    candidates
        .into_iter()
        .filter(|(_, candidate)| (candidate.start - span.start).abs() <= window_ms)
        .min_by_key(|(_, candidate)| (candidate.start - span.start).abs())
        .map(|(key, _)| key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;
    const NOW: i64 = 1000 * MIN;

    #[test]
    fn keys_carry_room_and_start() {
        assert_eq!(primary_key(3, 1000), "3:1000");
        assert_eq!(standby_key(3, 1000), "standby:3:1000");
        assert_eq!(parse("3:1000"), Some((3, 1000)));
        assert_eq!(parse("standby:3:1000"), Some((3, 1000)));
        assert!(is_standby_key("standby:3:1000"));
        assert!(!is_standby_key("3:1000"));
        assert_eq!(parse("nope"), None);
        assert_eq!(parse("3:x"), None);
    }

    #[test]
    fn starts_within_the_merge_window_are_the_same_session() {
        let window = 10 * MIN;
        let primary = Span::new(100 * MIN, Some(160 * MIN));
        // 备机晚 3 分钟开录（检测间隔、时钟差）
        assert!(same_session(
            primary,
            Span::new(103 * MIN, Some(161 * MIN)),
            window,
            NOW
        ));
        // 备机时钟快 9 分钟
        assert!(same_session(
            primary,
            Span::new(91 * MIN, Some(150 * MIN)),
            window,
            NOW
        ));
        // 模式 2 接手：备机在主机录到一半时才开录，主机那段没有结束时刻
        assert!(same_session(
            Span::new(100 * MIN, None),
            Span::new(140 * MIN, Some(200 * MIN)),
            window,
            NOW
        ));
        // 下一场：主机那场早就结束了
        assert!(!same_session(
            primary,
            Span::new(200 * MIN, Some(260 * MIN)),
            window,
            NOW
        ));
        // 下播 5 分钟后又开播：开播时刻差得远，也没有真的重叠，是下一场
        assert!(!same_session(
            primary,
            Span::new(165 * MIN, Some(200 * MIN)),
            window,
            NOW
        ));
        // 还在录的一段按录到此刻算，不会和以后才开播的场次对上
        assert!(!same_session(
            Span::new(250 * MIN, None),
            Span::new(300 * MIN, None),
            window,
            260 * MIN
        ));
    }

    #[test]
    fn the_closest_start_within_the_window_is_adopted() {
        let window = 10 * MIN;
        let candidates = [
            ("3:1", Span::new(0, Some(30 * MIN))),
            ("3:2", Span::new(35 * MIN, Some(90 * MIN))),
            ("3:3", Span::new(300 * MIN, None)),
        ];
        assert_eq!(
            best_match(Span::new(36 * MIN, None), candidates, window),
            Some("3:2")
        );
        assert_eq!(
            best_match(Span::new(2 * MIN, Some(20 * MIN)), candidates, window),
            Some("3:1")
        );
        // 重叠但开播时刻差得远（接手时主机那场早开始了）：不采用它的键
        assert_eq!(
            best_match(Span::new(60 * MIN, None), candidates, window),
            None
        );
    }
}
