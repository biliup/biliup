//! 分窗与提示词：30 分钟一窗、前后重叠 2 分钟，每窗一次 chat。时间一律写成场次时间 `H:MM:SS`
//! （模型对它比对毫秒数稳），回复里的时间按同样的写法解析回来。

use super::danmaku::Density;
use super::files::Line;
use super::thumbs::Shot;

pub const WINDOW_MS: i64 = 30 * 60_000;
pub const OVERLAP_MS: i64 = 2 * 60_000;
/// 超上下文时窗口减半，最短减到这么长。
pub const MIN_WINDOW_MS: i64 = 5 * 60_000;
/// 每窗最多要几个候选。
pub const MAX_PER_WINDOW: usize = 10;
/// 每张图按这么多 token 估（OpenAI 低清图 85；按图块计价的模型更多，实际用量以服务返回的为准）。
pub const IMAGE_TOKENS: i64 = 170;
/// 每窗的回复按这么多 token 估；请求里 `max_tokens` 另给上限。
pub const EXPECTED_OUTPUT_TOKENS: i64 = 1_000;
pub const MAX_OUTPUT_TOKENS: u32 = 2_000;
/// system 与 user 两条消息的格式开销。
const MESSAGE_OVERHEAD_TOKENS: i64 = 12;

/// 分析的一窗（场次时间）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub from_ms: i64,
    pub to_ms: i64,
}

impl Window {
    pub fn contains(&self, from_ms: i64, to_ms: i64) -> bool {
        self.from_ms <= from_ms && to_ms <= self.to_ms
    }

    /// 对半分成两窗（中间仍重叠 `OVERLAP_MS`）；已经不能再短时返回 `None`。
    pub fn halves(&self) -> Option<(Window, Window)> {
        let length = self.to_ms - self.from_ms;
        if length / 2 < MIN_WINDOW_MS {
            return None;
        }
        let mid = self.from_ms + length / 2;
        Some((
            Window {
                from_ms: self.from_ms,
                to_ms: (mid + OVERLAP_MS / 2).min(self.to_ms),
            },
            Window {
                from_ms: (mid - OVERLAP_MS / 2).max(self.from_ms),
                to_ms: self.to_ms,
            },
        ))
    }
}

/// `[0, end_ms)` 切成 30 分钟一窗、相邻重叠 2 分钟。
pub fn windows(end_ms: i64) -> Vec<Window> {
    let mut out = Vec::new();
    if end_ms <= 0 {
        return out;
    }
    let mut from_ms = 0;
    loop {
        let to_ms = (from_ms + WINDOW_MS).min(end_ms);
        out.push(Window { from_ms, to_ms });
        if to_ms >= end_ms {
            break;
        }
        from_ms = to_ms - OVERLAP_MS;
    }
    out
}

/// 场次毫秒 → `H:MM:SS`（向下取整到秒）。
pub fn clock(ms: i64) -> String {
    let secs = ms.max(0) / 1000;
    format!("{}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}

/// `H:MM:SS`、`MM:SS`、`H:MM:SS.s` 或秒数 → 场次毫秒。分、秒不能到 60。
pub fn parse_clock(text: &str) -> Option<i64> {
    let text = text.trim();
    if let Ok(secs) = text.parse::<f64>() {
        return (secs.is_finite() && secs >= 0.0).then(|| (secs * 1000.0).round() as i64);
    }
    let parts: Vec<&str> = text.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let seconds: f64 = parts.last()?.parse().ok()?;
    if !seconds.is_finite() || !(0.0..60.0).contains(&seconds) {
        return None;
    }
    let mut whole: Vec<i64> = Vec::new();
    for part in &parts[..parts.len() - 1] {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        whole.push(part.parse().ok()?);
    }
    let (hours, minutes) = match whole.as_slice() {
        [m] => (0, *m),
        [h, m] => (*h, *m),
        _ => return None,
    };
    if minutes >= 60 && parts.len() == 3 {
        return None;
    }
    Some((hours * 3600 + minutes * 60) * 1000 + (seconds * 1000.0).round() as i64)
}

/// 场次信息，放在提示词开头。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionInfo {
    pub title: String,
    pub streamer: String,
    pub platform: String,
}

/// 生成一窗提示词要用的全部输入。
#[derive(bon::Builder, Debug, Clone)]
pub struct PromptInput<'a> {
    pub session: &'a SessionInfo,
    pub window: Window,
    pub lines: &'a [Line],
    pub density: Option<&'a Density>,
    #[builder(default)]
    pub shots: &'a [Shot],
    pub min_clip_secs: u64,
    pub max_clip_secs: u64,
}

/// 一窗的提示词。`images` 是随消息附上的缩图（场次时间），顺序就是「图1、图2……」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub system: String,
    pub user: String,
    pub images: Vec<i64>,
}

impl Prompt {
    /// 输入 token 的估算：汉字等非 ASCII 字符每个按 1 个，ASCII 每 4 个按 1 个，图每张 [`IMAGE_TOKENS`]。
    pub fn estimate_tokens(&self) -> i64 {
        estimate_text_tokens(&self.system)
            + estimate_text_tokens(&self.user)
            + self.images.len() as i64 * IMAGE_TOKENS
            + MESSAGE_OVERHEAD_TOKENS
    }

    /// 去掉图片的同一窗（服务拒收图片时用）。
    pub fn without_images(&self) -> Prompt {
        let user = self
            .user
            .lines()
            .filter(|line| !line.starts_with("[画面]"))
            .collect::<Vec<_>>()
            .join("\n");
        Prompt {
            system: self.system.clone(),
            user,
            images: Vec::new(),
        }
    }
}

pub fn estimate_text_tokens(text: &str) -> i64 {
    let (ascii, other) = text.chars().fold((0i64, 0i64), |(a, o), c| {
        if c.is_ascii() { (a + 1, o) } else { (a, o + 1) }
    });
    other + (ascii + 3) / 4
}

/// 固定的 system 消息：角色、输出格式、不许编时间、理由要引原文。
pub fn system_message() -> String {
    format!(
        "你是直播切片助手：根据给出的弹幕密度、弹幕高峰、语音转写和画面截图，找出适合单独剪成短视频的片段。\n\
         只输出一个 JSON 对象，不要任何解释或 Markdown，格式：\n\
         {{\"candidates\": [{{\"start\": \"H:MM:SS\", \"end\": \"H:MM:SS\", \"title\": \"不超过 30 字的标题\", \
         \"reason\": \"为什么值得剪\", \"confidence\": 0.8, \"tags\": [\"高能\"], \"evidence\": [\"danmaku\", \"asr\", \"image:2\"]}}]}}\n\
         规则：\n\
         - start、end 是场次时间，写成 H:MM:SS，只能落在「本窗」范围里；不得编造材料里没有出现的时间；\n\
         - reason 要引用弹幕、语音或画面里的具体内容；\n\
         - evidence 列出依据：danmaku 表示弹幕高峰，asr 表示语音，image:N 表示第 N 张图；\n\
         - confidence 是 0 到 1 的小数，表示你有多大把握这段值得剪；\n\
         - 最多 {MAX_PER_WINDOW} 段；没有合适的片段就返回 {{\"candidates\": []}}。"
    )
}

/// 生成一窗的提示词。
pub fn build(input: &PromptInput) -> Prompt {
    let window = input.window;
    let mut user = Vec::new();
    user.push(format!(
        "场次：{}｜主播：{}｜平台：{}｜本窗：{}–{}",
        one_line(&input.session.title),
        one_line(&input.session.streamer),
        one_line(&input.session.platform),
        clock(window.from_ms),
        clock(window.to_ms)
    ));
    user.push(format!(
        "要求：找出适合单独剪成短视频的片段（高能操作、搞笑、名场面、情绪爆发、与观众的精彩互动、才艺高潮）；\
         每段 {} 秒到 {} 秒；最多 {MAX_PER_WINDOW} 段；没有就返回空数组；只能用下面给出的时间范围。",
        input.min_clip_secs, input.max_clip_secs
    ));
    match input.density {
        Some(density) => {
            let range = density.buckets_in(window.from_ms, window.to_ms);
            let mut base: Vec<f64> = density.baseline[range.clone()].to_vec();
            base.sort_by(f64::total_cmp);
            let typical = base.get(base.len() / 2).copied().unwrap_or(0.0);
            let counts: Vec<String> = density.counts[range.clone()]
                .iter()
                .map(u32::to_string)
                .collect();
            user.push(format!(
                "[弹幕密度] 每 10 秒条数（基线约 {}）：{} {}",
                typical.round() as i64,
                clock(density.bucket_start(range.start)),
                counts.join(" ")
            ));
            for peak in density.peaks_in(window.from_ms, window.to_ms) {
                let samples: Vec<String> = peak
                    .samples
                    .iter()
                    .map(|s| format!("「{}」×{}", one_line(&s.text), s.count))
                    .collect();
                user.push(format!(
                    "[弹幕高峰] {}–{} 共 {} 条（基线 {:.0} 倍）：{}",
                    clock(peak.from_ms),
                    clock(peak.to_ms),
                    peak.count,
                    peak.ratio,
                    samples.join(" ")
                ));
            }
            let mut paid: Vec<_> = density
                .paid
                .iter()
                .filter(|b| b.from_ms >= window.from_ms && b.from_ms < window.to_ms)
                .collect();
            if !paid.is_empty() {
                paid.sort_by(|a, b| b.count.cmp(&a.count).then(a.from_ms.cmp(&b.from_ms)));
                paid.truncate(10);
                paid.sort_by_key(|b| b.from_ms);
                let listed: Vec<String> = paid
                    .iter()
                    .map(|b| format!("{} {} 次", clock(b.from_ms), b.count))
                    .collect();
                user.push(format!(
                    "[礼物与醒目留言] 最集中的时段：{}",
                    listed.join("，")
                ));
            }
        }
        None => user.push("[弹幕] 本场无弹幕记录".into()),
    }
    let mut spoken = 0;
    for line in input
        .lines
        .iter()
        .filter(|l| l.to_ms > window.from_ms && l.from_ms < window.to_ms)
    {
        spoken += 1;
        user.push(format!(
            "[语音] {}–{} {}",
            clock(line.from_ms),
            clock(line.to_ms),
            one_line(&line.text)
        ));
    }
    if spoken == 0 {
        user.push("[语音] 本窗没有转写到语音".into());
    }
    let images: Vec<i64> = input.shots.iter().map(|s| s.t_ms).collect();
    if !images.is_empty() {
        let listed: Vec<String> = images
            .iter()
            .enumerate()
            .map(|(i, t)| format!("图{} = {}", i + 1, clock(*t)))
            .collect();
        user.push(format!("[画面] {}（随后附图）", listed.join("，")));
    }
    Prompt {
        system: system_message(),
        user: user.join("\n"),
        images,
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests;
