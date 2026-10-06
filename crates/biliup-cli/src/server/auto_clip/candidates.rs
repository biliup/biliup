//! 模型回复 → 候选：解析 JSON、逐条校验、吸附关键帧、各窗合并去重、按置信度截断。
//!
//! 逐条丢掉：时间解析不了、超出本窗或录像、出点不在入点之后、时长不在 `[min_clip_secs, max_clip_secs]`；
//! 区间里既没有转写句子、也没有弹幕高峰、也没引用本窗的图（挡住凭空编的时间）；入点或出点落在断流缺口
//! 或读不到的分段上。入点吸附到之前（含）最近的关键帧，出点吸附到之后（含）最近的关键帧或分段末尾，
//! 与快速剪的落刀规则一致。各窗合并后区间重叠率（IoU）超过 0.5 的只留置信度高的。
//! `confidence` 是模型自评，没给时记为空、排在最后，不包装成准确率。

use super::danmaku::Density;
use super::files::Line;
use super::prompt::{Window, parse_clock};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::index::Container;
use crate::server::workbench::session_keyframes;
use crate::server::workbench::store::{self, SegmentRow, SegmentState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

pub const MAX_TITLE_CHARS: usize = 80;
pub const MAX_REASON_CHARS: usize = 500;
const MAX_TAGS: usize = 8;
/// 重叠率超过它算重复。
const DUPLICATE_IOU: f64 = 0.5;

/// 模型给的一条，时间已换成场次毫秒，还没校验。
#[derive(Debug, Clone, PartialEq)]
pub struct Proposed {
    pub start_ms: i64,
    pub end_ms: i64,
    pub title: String,
    pub reason: String,
    pub confidence: Option<f64>,
    pub tags: Vec<String>,
    /// `image:N` 里的 N（从 1 数）
    pub images: Vec<usize>,
}

/// 候选有哪些依据（服务端核对过的）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// 区间里的转写句子数
    #[serde(default)]
    pub asr_lines: u32,
    /// 与区间重叠的弹幕高峰里的弹幕条数
    #[serde(default)]
    pub danmaku: u32,
    /// 引用到的本窗缩图（场次时间）
    #[serde(default)]
    pub images: Vec<i64>,
}

impl Evidence {
    pub fn is_empty(&self) -> bool {
        self.asr_lines == 0 && self.danmaku == 0 && self.images.is_empty()
    }
}

/// 校验、吸附之后的候选。
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub in_ms: i64,
    pub out_ms: i64,
    pub title: String,
    pub reason: String,
    pub confidence: Option<f64>,
    pub tags: Vec<String>,
    pub evidence: Evidence,
}

/// 为什么丢掉一条。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rejection {
    /// 缺时间或时间写法不对
    BadTime,
    /// 超出本窗或录像
    OutOfRange,
    /// 出点不在入点之后，或时长不在允许范围
    BadLength,
    /// 区间里没有任何材料
    NoEvidence,
    /// 落在断流缺口或读不到的分段上
    Unreadable,
}

impl Rejection {
    pub fn describe(self) -> &'static str {
        match self {
            Rejection::BadTime => "时间写法不对",
            Rejection::OutOfRange => "超出本窗或录像",
            Rejection::BadLength => "时长不合要求",
            Rejection::NoEvidence => "区间里找不到对应的语音、弹幕或画面",
            Rejection::Unreadable => "落在断流缺口或已清理的录像上",
        }
    }
}

/// 回复整体不对。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyError {
    /// 回复里找不到 JSON
    NotJson,
    /// 有 JSON 但没有 `candidates` 数组
    NoCandidates,
}

/// 解析回复：整段是 JSON 或包着一个 `{…}` 块，里面有 `candidates` 数组（也认直接给数组）。
/// 缺时间、时间写法不对的条目记在 `rejected` 里。
pub fn parse(content: &str, rejected: &mut Vec<Rejection>) -> Result<Vec<Proposed>, ReplyError> {
    let value = super::model::extract_json(content)
        .or_else(|| extract_array(content))
        .ok_or(ReplyError::NotJson)?;
    let items = match &value {
        Value::Array(items) => items,
        Value::Object(map) => match map.get("candidates") {
            Some(Value::Array(items)) => items,
            _ => return Err(ReplyError::NoCandidates),
        },
        _ => return Err(ReplyError::NoCandidates),
    };
    let mut out = Vec::new();
    for item in items {
        match proposed(item) {
            Some(p) => out.push(p),
            None => rejected.push(Rejection::BadTime),
        }
    }
    Ok(out)
}

fn extract_array(content: &str) -> Option<Value> {
    let start = content.find('[')?;
    let end = content.rfind(']')?;
    (end > start)
        .then(|| serde_json::from_str::<Value>(&content[start..=end]).ok())
        .flatten()
        .filter(Value::is_array)
}

fn text_of(value: Option<&Value>, max: usize) -> String {
    let text = match value {
        Some(Value::String(s)) => s.as_str(),
        _ => "",
    };
    let single: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    single.trim().chars().take(max).collect()
}

fn time_of(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::String(s) => parse_clock(s),
        Value::Number(n) => n
            .as_f64()
            .filter(|v| v.is_finite() && *v >= 0.0)
            .map(|v| (v * 1000.0).round() as i64),
        _ => None,
    }
}

/// 置信度：0–1 的数；给成百分数（1–100）的换算；其它（缺失、负数、字符串写不成数）为空。
fn confidence_of(value: Option<&Value>) -> Option<f64> {
    let raw = match value? {
        Value::Number(n) => n.as_f64()?,
        Value::String(s) => s.trim().trim_end_matches('%').trim().parse().ok()?,
        _ => return None,
    };
    if !raw.is_finite() || raw < 0.0 {
        return None;
    }
    let scaled = if raw > 1.0 { raw / 100.0 } else { raw };
    (scaled <= 1.0).then_some(scaled)
}

fn proposed(item: &Value) -> Option<Proposed> {
    let start_ms = time_of(item.get("start"))?;
    let end_ms = time_of(item.get("end"))?;
    let strings = |key: &str| -> Vec<String> {
        item.get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let images = strings("evidence")
        .iter()
        .filter_map(|e| e.strip_prefix("image:").or_else(|| e.strip_prefix("图")))
        .filter_map(|n| n.trim().parse::<usize>().ok())
        .collect();
    let mut tags: Vec<String> = strings("tags")
        .into_iter()
        .map(|t| t.chars().take(20).collect())
        .collect();
    tags.truncate(MAX_TAGS);
    Some(Proposed {
        start_ms,
        end_ms,
        title: text_of(item.get("title"), MAX_TITLE_CHARS),
        reason: text_of(item.get("reason"), MAX_REASON_CHARS),
        confidence: confidence_of(item.get("confidence")),
        tags,
        images,
    })
}

/// 核对一窗里的一条要用的材料。
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    pub window: Window,
    /// 录像在场次时间轴上的终点
    pub end_ms: i64,
    pub min_ms: i64,
    pub max_ms: i64,
    pub lines: &'a [Line],
    pub density: Option<&'a Density>,
    /// 这一窗附上的缩图（场次时间），下标 + 1 就是「图 N」
    pub images: &'a [i64],
}

/// 不查库的那部分校验：时间范围、时长、证据。
pub fn check(proposed: &Proposed, cx: &Context) -> Result<Evidence, Rejection> {
    let (start, end) = (proposed.start_ms, proposed.end_ms);
    if start < 0 || !cx.window.contains(start, end) || end > cx.end_ms {
        return Err(Rejection::OutOfRange);
    }
    let length = end - start;
    if length <= 0 || length < cx.min_ms || length > cx.max_ms {
        return Err(Rejection::BadLength);
    }
    let asr_lines = cx
        .lines
        .iter()
        .filter(|l| l.to_ms > start && l.from_ms < end)
        .count() as u32;
    let danmaku = cx
        .density
        .map(|d| d.peaks_in(start, end).map(|p| p.count).sum())
        .unwrap_or(0);
    let mut images: Vec<i64> = proposed
        .images
        .iter()
        .filter_map(|n| n.checked_sub(1).and_then(|i| cx.images.get(i)).copied())
        .collect();
    images.sort_unstable();
    images.dedup();
    let evidence = Evidence {
        asr_lines,
        danmaku,
        images,
    };
    if evidence.is_empty() {
        return Err(Rejection::NoEvidence);
    }
    Ok(evidence)
}

/// 场次里能落刀的分段与它们的关键帧，吸附用。
pub struct Snapper {
    segments: Vec<SegmentRow>,
}

impl Snapper {
    pub async fn load(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<Self> {
        let segments = store::session_segments(pool, session_id)
            .await?
            .into_iter()
            .filter(|s| {
                matches!(
                    s.state,
                    SegmentState::Finished | SegmentState::PendingDelete
                ) && s.end_ms.is_some()
                    && Container::from_path(Path::new(&s.path)).is_some()
            })
            .collect();
        Ok(Snapper { segments })
    }

    fn covering(&self, t_ms: i64, at_end: bool) -> Option<&SegmentRow> {
        self.segments.iter().find(|s| {
            let end = s.end_ms.unwrap_or(s.start_ms);
            if at_end {
                s.start_ms < t_ms && t_ms <= end
            } else {
                s.start_ms <= t_ms && t_ms < end
            }
        })
    }

    /// 入点吸附到之前（含）最近的关键帧（同一分段里），出点吸附到之后（含）最近的关键帧或分段末尾。
    pub async fn snap(
        &self,
        pool: &ConnectionPool,
        session_id: i64,
        in_ms: i64,
        out_ms: i64,
    ) -> Option<(i64, i64)> {
        let first = self.covering(in_ms, false)?;
        let last = self.covering(out_ms, true)?;
        let keyframes = session_keyframes(pool, session_id, first.start_ms, out_ms)
            .await
            .ok()?;
        let snapped_in = keyframes
            .iter()
            .filter(|k| k.segment_id == first.id && k.t_ms <= in_ms)
            .map(|k| k.t_ms)
            .max()?;
        let last_end = last.end_ms.unwrap_or(last.start_ms);
        let after = session_keyframes(pool, session_id, out_ms, last_end)
            .await
            .ok()?;
        let snapped_out = after
            .iter()
            .filter(|k| k.segment_id == last.id && k.t_ms >= out_ms)
            .map(|k| k.t_ms)
            .min()
            .unwrap_or(last_end)
            .min(last_end);
        (snapped_out > snapped_in).then_some((snapped_in, snapped_out))
    }
}

fn iou(a: (i64, i64), b: (i64, i64)) -> f64 {
    let overlap = (a.1.min(b.1) - a.0.max(b.0)).max(0) as f64;
    let union = (a.1.max(b.1) - a.0.min(b.0)).max(1) as f64;
    overlap / union
}

pub fn overlaps_too_much(a: (i64, i64), b: (i64, i64)) -> bool {
    iou(a, b) > DUPLICATE_IOU
}

/// 各窗合并：重叠率超过 0.5 的只留置信度高的（同样高的留先出现的），再按置信度截到 `max`，
/// 最后按入点排序。
pub fn merge(mut all: Vec<Candidate>, max: usize) -> Vec<Candidate> {
    let rank = |c: &Candidate| c.confidence.unwrap_or(-1.0);
    // 稳定排序：同样的置信度保持原来的先后
    all.sort_by(|a, b| rank(b).total_cmp(&rank(a)));
    let mut kept: Vec<Candidate> = Vec::new();
    for candidate in all {
        let range = (candidate.in_ms, candidate.out_ms);
        if kept
            .iter()
            .all(|k| !overlaps_too_much((k.in_ms, k.out_ms), range))
        {
            kept.push(candidate);
        }
    }
    kept.truncate(max);
    kept.sort_by_key(|c| (c.in_ms, c.out_ms));
    kept
}

#[cfg(test)]
mod tests;
