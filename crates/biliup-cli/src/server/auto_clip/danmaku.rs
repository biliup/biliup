//! 弹幕密度与高峰：从分段的弹幕 XML 算出 10 秒一桶的条数、前后 10 分钟的滑动中位数基线，
//! 以及条数明显高于基线的高峰（附出现最多的弹幕样例）。
//!
//! - `<d p="秒,类型,字号,颜色,Unix秒,…">`：第一项是距这个 XML 创建的秒数。分段切换时 XML 跟着滚动，
//!   创建时刻约等于分段开头，所以场次时间 = 分段 `start_ms` + 秒数 × 1000，误差 1 秒左右；
//! - 分段没记 `danmaku_path`（旧数据、滚动失败）但旁边有同名 `.xml` 时，按第五项 Unix 秒减场次
//!   `started_at` 对齐；
//! - `<s>`（礼物、醒目留言、上舰，只在开了 `*_danmaku_detail` 时写）只有 Unix 秒，用同一个文件里
//!   `<d>` 的两种时间算出的偏移换算，另记一条「付费」曲线；
//! - 写到一半的 XML（进程被杀、还在录）读到哪算到哪。

use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::store::{self, SegmentRow};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 一桶多长（毫秒）。
pub const BUCKET_MS: i64 = 10_000;
/// 基线取前后各多少桶（10 分钟）的中位数。
const BASELINE_HALF: usize = 60;
/// 高峰：条数 ≥ max(基线 × 3, 基线 + 10)。
const PEAK_RATIO: f64 = 3.0;
const PEAK_MIN_EXTRA: f64 = 10.0;
/// 每个高峰留几条样例、每条最多几个字。
const SAMPLES_PER_PEAK: usize = 10;
const SAMPLE_CHARS: usize = 20;

/// 一条弹幕，时间已换算成场次时间。
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    pub at_ms: i64,
    pub text: String,
}

/// 一条礼物 / 醒目留言 / 上舰，时间已换算成场次时间。
#[derive(Debug, Clone, PartialEq)]
pub struct Paid {
    pub at_ms: i64,
    pub price: f64,
}

/// 高峰里出现最多的一条弹幕。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub text: String,
    pub count: u32,
}

/// 条数明显高于基线的一段（连续的高峰桶合并后前后各扩一桶）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Peak {
    pub from_ms: i64,
    pub to_ms: i64,
    /// 这一段里的弹幕条数
    pub count: u32,
    /// 这一段的平均基线（每桶）
    pub baseline: f64,
    /// 条数是基线的几倍（基线为 0 时按 1 算）
    pub ratio: f64,
    pub samples: Vec<Sample>,
}

impl Peak {
    pub fn center_ms(&self) -> i64 {
        (self.from_ms + self.to_ms) / 2
    }

    pub fn overlaps(&self, from_ms: i64, to_ms: i64) -> bool {
        self.from_ms < to_ms && from_ms < self.to_ms
    }
}

/// 付费事件按桶汇总（只列有事件的桶）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaidBucket {
    pub from_ms: i64,
    pub count: u32,
    /// 平台给的原始金额相加（单位随平台）
    pub price: f64,
}

/// 一场的弹幕密度。`counts[i]` 是场次时间 `[i × bucket_ms, (i + 1) × bucket_ms)` 里的条数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Density {
    pub bucket_ms: i64,
    pub counts: Vec<u32>,
    pub baseline: Vec<f64>,
    pub peaks: Vec<Peak>,
    pub paid: Vec<PaidBucket>,
    pub total: u64,
}

impl Density {
    /// 桶 `index` 的起点（场次时间）。
    pub fn bucket_start(&self, index: usize) -> i64 {
        index as i64 * self.bucket_ms
    }

    /// `[from_ms, to_ms)` 覆盖的桶下标。
    pub fn buckets_in(&self, from_ms: i64, to_ms: i64) -> std::ops::Range<usize> {
        let first = (from_ms.max(0) / self.bucket_ms) as usize;
        let last = ((to_ms.max(0) + self.bucket_ms - 1) / self.bucket_ms) as usize;
        first.min(self.counts.len())..last.min(self.counts.len())
    }

    pub fn peaks_in(&self, from_ms: i64, to_ms: i64) -> impl Iterator<Item = &Peak> {
        self.peaks
            .iter()
            .filter(move |peak| peak.overlaps(from_ms, to_ms))
    }
}

/// 一个 XML 文件里读到的原始内容。
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Parsed {
    /// `(距文件创建的秒数, Unix 秒, 内容)`
    pub comments: Vec<(f64, Option<i64>, String)>,
    /// `(Unix 秒, 金额)`
    pub paid: Vec<(i64, f64)>,
}

impl Parsed {
    /// `<d>` 的两种时间给出的「文件创建时刻」（Unix 毫秒，取中位数）；没有带 Unix 秒的弹幕时为 `None`。
    fn created_at_ms(&self) -> Option<i64> {
        let mut offsets: Vec<i64> = self
            .comments
            .iter()
            .filter_map(|(secs, unix, _)| unix.map(|u| u * 1000 - (secs * 1000.0).round() as i64))
            .collect();
        if offsets.is_empty() {
            return None;
        }
        offsets.sort_unstable();
        Some(offsets[offsets.len() / 2])
    }
}

/// 解析一个弹幕 XML。坏掉的部分（写到一半、非法字符）之前读到的照样返回。
pub fn parse_xml(bytes: &[u8]) -> Parsed {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut parsed = Parsed::default();
    let mut buf = Vec::new();
    // 正在读的 `<d>` 的时间，或 `<s>` 的时间与金额；读到文字时配对
    let mut open: Option<Open> = None;
    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            Event::Start(start) => open = opened(&start),
            Event::Text(text) => match open.take() {
                Some(Open::Comment(secs, unix)) => {
                    let content = text
                        .unescape()
                        .map(|t| t.into_owned())
                        .unwrap_or_else(|_| String::from_utf8_lossy(&text).into_owned());
                    parsed.comments.push((secs, unix, content));
                }
                Some(Open::Paid(unix, price)) => parsed.paid.push((unix, price)),
                None => {}
            },
            Event::End(_) => {
                if let Some(Open::Paid(unix, price)) = open.take() {
                    parsed.paid.push((unix, price));
                }
            }
            Event::Empty(start) => {
                if let Some(Open::Paid(unix, price)) = opened(&start) {
                    parsed.paid.push((unix, price));
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    parsed
}

enum Open {
    Comment(f64, Option<i64>),
    Paid(i64, f64),
}

fn attribute(start: &BytesStart, name: &[u8]) -> Option<String> {
    start
        .attributes()
        .flatten()
        .find(|attr| attr.key.as_ref() == name)
        .and_then(|attr| attr.unescape_value().ok().map(|v| v.into_owned()))
}

fn opened(start: &BytesStart) -> Option<Open> {
    match start.name().as_ref() {
        b"d" => {
            let p = attribute(start, b"p")?;
            let mut fields = p.split(',');
            let secs: f64 = fields.next()?.trim().parse().ok()?;
            if !secs.is_finite() || secs < 0.0 {
                return None;
            }
            let unix = fields.nth(3).and_then(|v| v.trim().parse::<i64>().ok());
            Some(Open::Comment(secs, unix))
        }
        b"s" => {
            let unix = attribute(start, b"timestamp")?.trim().parse().ok()?;
            let price = attribute(start, b"price")
                .and_then(|v| v.trim().parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v >= 0.0)
                .unwrap_or(0.0);
            Some(Open::Paid(unix, price))
        }
        _ => None,
    }
}

/// 一个 XML 文件怎么换算到场次时间。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    /// 随分段滚动的 XML：场次时间 = 分段起点 + 秒数
    Segment { start_ms: i64 },
    /// 没挂在分段上：场次时间 = Unix 毫秒 − 场次 `started_at`
    Wallclock { started_at: i64 },
}

/// 把一个文件里的内容换算成场次时间。
pub fn align(parsed: &Parsed, alignment: Alignment) -> (Vec<Comment>, Vec<Paid>) {
    let comments: Vec<Comment> = parsed
        .comments
        .iter()
        .filter_map(|(secs, unix, text)| {
            let at_ms = match alignment {
                Alignment::Segment { start_ms } => start_ms + (secs * 1000.0).round() as i64,
                Alignment::Wallclock { started_at } => unix.map(|u| u * 1000 - started_at)?,
            };
            Some(Comment {
                at_ms,
                text: text.clone(),
            })
        })
        .collect();
    // `<s>` 只有 Unix 秒：按同一文件的「创建时刻」换到文件内秒数，再按弹幕的规则对齐
    let offset = match alignment {
        Alignment::Segment { start_ms } => parsed.created_at_ms().map(|created| start_ms - created),
        Alignment::Wallclock { started_at } => Some(-started_at),
    };
    let paid = match offset {
        Some(offset) => parsed
            .paid
            .iter()
            .map(|(unix, price)| Paid {
                at_ms: unix * 1000 + offset,
                price: *price,
            })
            .collect(),
        None => Vec::new(),
    };
    (comments, paid)
}

/// 由弹幕和付费事件算密度、基线与高峰。`end_ms` 是场次时间轴的终点，之外的弹幕丢掉。
pub fn density(comments: &[Comment], paid: &[Paid], end_ms: i64) -> Density {
    let end_ms = end_ms.max(0);
    let buckets = ((end_ms + BUCKET_MS - 1) / BUCKET_MS).max(1) as usize;
    let index = |at_ms: i64| (at_ms >= 0 && at_ms < end_ms).then_some((at_ms / BUCKET_MS) as usize);
    let mut counts = vec![0u32; buckets];
    let mut total = 0u64;
    for comment in comments {
        if let Some(i) = index(comment.at_ms) {
            counts[i] += 1;
            total += 1;
        }
    }
    let baseline = rolling_median(&counts, BASELINE_HALF);
    let peaks = peaks(&counts, &baseline, comments);
    let mut paid_buckets: Vec<Option<PaidBucket>> = vec![None; buckets];
    for event in paid {
        if let Some(i) = index(event.at_ms) {
            let bucket = paid_buckets[i].get_or_insert(PaidBucket {
                from_ms: i as i64 * BUCKET_MS,
                count: 0,
                price: 0.0,
            });
            bucket.count += 1;
            bucket.price += event.price;
        }
    }
    Density {
        bucket_ms: BUCKET_MS,
        counts,
        baseline,
        peaks,
        paid: paid_buckets.into_iter().flatten().collect(),
        total,
    }
}

fn rolling_median(counts: &[u32], half: usize) -> Vec<f64> {
    let mut window = Vec::with_capacity(half * 2 + 1);
    (0..counts.len())
        .map(|i| {
            let from = i.saturating_sub(half);
            let to = (i + half + 1).min(counts.len());
            window.clear();
            window.extend_from_slice(&counts[from..to]);
            window.sort_unstable();
            let mid = window.len() / 2;
            if window.len() % 2 == 1 {
                window[mid] as f64
            } else {
                (window[mid - 1] as f64 + window[mid] as f64) / 2.0
            }
        })
        .collect()
}

fn is_peak(count: u32, baseline: f64) -> bool {
    count as f64 >= (baseline * PEAK_RATIO).max(baseline + PEAK_MIN_EXTRA)
}

fn peaks(counts: &[u32], baseline: &[f64], comments: &[Comment]) -> Vec<Peak> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < counts.len() {
        if !is_peak(counts[i], baseline[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < counts.len() && is_peak(counts[i], baseline[i]) {
            i += 1;
        }
        let from = start.saturating_sub(1);
        let to = (i + 1).min(counts.len());
        match ranges.last_mut() {
            // 前后各扩一桶后连上了就并成一个
            Some(last) if last.1 >= from => last.1 = to,
            _ => ranges.push((from, to)),
        }
    }
    ranges
        .into_iter()
        .map(|(from, to)| {
            let count: u32 = counts[from..to].iter().sum();
            let base = baseline[from..to].iter().sum::<f64>() / (to - from) as f64;
            let from_ms = from as i64 * BUCKET_MS;
            let to_ms = to as i64 * BUCKET_MS;
            Peak {
                from_ms,
                to_ms,
                count,
                baseline: base,
                ratio: count as f64 / (base.max(1.0) * (to - from) as f64),
                samples: samples(comments, from_ms, to_ms),
            }
        })
        .collect()
}

/// `[from_ms, to_ms)` 里出现最多的弹幕（去掉首尾空白后按原文计次，同样多的按先出现的排前）。
fn samples(comments: &[Comment], from_ms: i64, to_ms: i64) -> Vec<Sample> {
    let mut seen: HashMap<&str, (u32, usize)> = HashMap::new();
    for (order, comment) in comments
        .iter()
        .filter(|c| c.at_ms >= from_ms && c.at_ms < to_ms)
        .enumerate()
    {
        let text = comment.text.trim();
        if text.is_empty() {
            continue;
        }
        seen.entry(text).or_insert((0, order)).0 += 1;
    }
    let mut ranked: Vec<(&str, u32, usize)> = seen
        .into_iter()
        .map(|(text, (count, order))| (text, count, order))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));
    ranked
        .into_iter()
        .take(SAMPLES_PER_PEAK)
        .map(|(text, count, _)| Sample {
            text: text.chars().take(SAMPLE_CHARS).collect(),
            count,
        })
        .collect()
}

/// 分段的弹幕文件与对齐方式：记了 `danmaku_path` 按分段起点，否则找同名 `.xml` 按 Unix 秒。
fn danmaku_file(segment: &SegmentRow, started_at: Option<i64>) -> Option<(PathBuf, Alignment)> {
    if let Some(path) = &segment.danmaku_path {
        return Some((
            PathBuf::from(path),
            Alignment::Segment {
                start_ms: segment.start_ms,
            },
        ));
    }
    let video = Path::new(&segment.path);
    let name = video.to_string_lossy();
    let sibling = PathBuf::from(name.strip_suffix(".part").unwrap_or(&name)).with_extension("xml");
    Some((
        sibling,
        Alignment::Wallclock {
            started_at: started_at?,
        },
    ))
}

/// 场次的弹幕密度。这一场一个弹幕文件都读不到时返回 `None`（没开弹幕录制或文件已删）。
pub async fn session_density(
    pool: &ConnectionPool,
    session_id: i64,
) -> sqlx::Result<Option<Density>> {
    let Some(session) = store::session(pool, session_id).await? else {
        return Ok(None);
    };
    let segments = store::session_segments(pool, session_id).await?;
    let end_ms = segments
        .iter()
        .map(|s| s.end_ms.unwrap_or(s.start_ms))
        .max()
        .unwrap_or(0);
    let files: Vec<(PathBuf, Alignment)> = segments
        .iter()
        .filter_map(|segment| danmaku_file(segment, session.started_at))
        .collect();
    let density = tokio::task::spawn_blocking(move || {
        let mut comments = Vec::new();
        let mut paid = Vec::new();
        let mut found = false;
        for (path, alignment) in files {
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            found = true;
            let (c, p) = align(&parse_xml(&bytes), alignment);
            comments.extend(c);
            paid.extend(p);
        }
        comments.sort_by_key(|c| c.at_ms);
        found.then(|| density(&comments, &paid, end_ms))
    })
    .await
    .map_err(|e| sqlx::Error::Io(std::io::Error::other(e)))?;
    Ok(density)
}

#[cfg(test)]
mod tests;
