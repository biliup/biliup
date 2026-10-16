//! Strict XML validation and complete-segment ASS generation. Never clips or
//! re-lays out comments at a clip's in-point.
//!
//! The native Rust layout (`danmaku-factory`) runs in a supervised child
//! process, retaining cancellation, timeout and crash isolation.
pub use super::danmaku_directory::ManagedRoot;
use super::engine::AssPreview;
use super::model::{DanmakuSettings, RenderSource};
use crate::danmaku_worker::{PROTOCOL, Request};
use danmaku_factory::{ConvertError, ConvertOptions, ConvertStats, ErrorKind};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[path = "danmaku_process.rs"]
mod process;

#[derive(Debug)]
struct WorkerOutput {
    stats: ConvertStats,
    ass: Vec<u8>,
}

use danmaku_factory::limits::{
    BoundedWriter, MAX_COMMENTS, MAX_XML_BYTES, NORMALIZED_FOOTER, NORMALIZED_HEADER,
    normalized_xml_budget,
};
const CONVERT_TIMEOUT: Duration = Duration::from_secs(120);
const SELF_TEST_TIMEOUT: Duration = Duration::from_secs(15);
/// Comments further than 250 hours from the segment start can never be on
/// screen. Dropping them keeps every time the core sees within its `int`
/// millisecond compatibility range; real recordings never get close.
const MAX_COMMENT_OFFSET_MS: i64 = 250 * 3_600_000;
/// File names inside the conversion's private work directory.
const XML_FILE: &str = "danmaku.xml";
const ASS_FILE: &str = "danmaku.ass";
const REQUEST_FILE: &str = "danmaku-request.json";
const RESULT_FILE: &str = "danmaku-result.json";

#[derive(Debug, Clone, PartialEq)]
pub struct XmlComment {
    pub elapsed_ms: i64,
    pub unix_ms: Option<i64>,
    pub mode: u32,
    pub size: u32,
    pub color: u32,
    pub text: String,
}

#[derive(Debug, Default)]
pub struct ParsedXml {
    pub recording_start_time_ms: Option<i64>,
    pub comments: Vec<XmlComment>,
}

fn attributes(start: &BytesStart<'_>) -> Result<Vec<(Vec<u8>, String)>, String> {
    start
        .attributes()
        .map(|attribute| {
            let attribute = attribute.map_err(|e| format!("弹幕 XML 属性损坏：{e}"))?;
            let value = attribute
                .unescape_value()
                .map_err(|e| format!("弹幕 XML 属性损坏：{e}"))?;
            Ok((attribute.key.as_ref().to_vec(), value.into_owned()))
        })
        .collect()
}

fn comment(attrs: &[(Vec<u8>, String)]) -> Result<XmlComment, String> {
    let p = attrs
        .iter()
        .find(|(key, _)| key == b"p")
        .map(|(_, value)| value)
        .ok_or("弹幕 XML 的 d 元素缺少 p 属性")?;
    let fields: Vec<&str> = p.split(',').collect();
    if fields.len() < 4 {
        return Err("弹幕 XML 的 p 属性字段不完整".into());
    }
    let seconds = fields[0]
        .trim()
        .parse::<f64>()
        .map_err(|_| "弹幕 XML 的时间无效")?;
    if !seconds.is_finite() || !(0.0..=1_000_000_000.0).contains(&seconds) {
        return Err("弹幕 XML 的时间无效".into());
    }
    let mode = fields[1]
        .trim()
        .parse::<u32>()
        .map_err(|_| "弹幕 XML 的类型无效")?;
    let size = fields[2]
        .trim()
        .parse::<u32>()
        .map_err(|_| "弹幕 XML 的字号无效")?;
    let color = fields[3]
        .trim()
        .parse::<u32>()
        .map_err(|_| "弹幕 XML 的颜色无效")?;
    if !(1..=9).contains(&mode) || !(1..=1000).contains(&size) || color > 0xffffff {
        return Err("弹幕 XML 的类型、字号或颜色无效".into());
    }
    let unix_ms = fields
        .get(4)
        .filter(|v| !v.trim().is_empty())
        .map(|v| {
            let unix = v
                .trim()
                .parse::<i64>()
                .map_err(|_| "弹幕 XML 的 Unix 时间无效")?;
            unix.checked_mul(1000)
                .filter(|n| *n >= 0)
                .ok_or("弹幕 XML 的 Unix 时间超出范围")
        })
        .transpose()?;
    Ok(XmlComment {
        elapsed_ms: (seconds * 1000.).round() as i64,
        unix_ms,
        mode,
        size,
        color,
        text: String::new(),
    })
}

/// A closed `<i>` document is required, including for an empty recording. Broken
/// or half-written XML fails instead of silently creating an incomplete export.
pub fn parse_xml(bytes: &[u8]) -> Result<ParsedXml, String> {
    parse_xml_with_limit(bytes, MAX_XML_BYTES, MAX_COMMENTS)
}

fn parse_xml_with_limit(
    bytes: &[u8],
    byte_limit: u64,
    comment_limit: usize,
) -> Result<ParsedXml, String> {
    if bytes.len() as u64 > byte_limit {
        return Err("弹幕 XML 超过处理上限".into());
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut result = ParsedXml::default();
    let mut depth = 0usize;
    let mut root_seen = false;
    let mut root_closed = false;
    let mut pending: Option<XmlComment> = None;
    let mut origin_text = None::<String>;
    let set_origin = |result: &mut ParsedXml, value: &str| -> Result<(), String> {
        let origin = value
            .trim()
            .parse::<i64>()
            .map_err(|_| "弹幕 XML 的录制起点无效")?;
        if origin < 0
            || result
                .recording_start_time_ms
                .is_some_and(|old| old != origin)
        {
            return Err("弹幕 XML 的录制起点无效或重复".into());
        }
        result.recording_start_time_ms = Some(origin);
        Ok(())
    };
    loop {
        match reader
            .read_event()
            .map_err(|e| format!("弹幕 XML 已损坏或未写完：{e}"))?
        {
            Event::Start(start) => {
                let attrs = attributes(&start)?;
                if depth == 0 {
                    if root_seen || start.name().as_ref() != b"i" {
                        return Err("弹幕 XML 必须只有一个 i 根元素".into());
                    }
                    root_seen = true;
                    if let Some((_, value)) = attrs
                        .iter()
                        .find(|(key, _)| key == b"recording_start_time_ms")
                    {
                        set_origin(&mut result, value)?;
                    }
                } else if depth == 1 && start.name().as_ref() == b"d" {
                    pending = Some(comment(&attrs)?);
                } else if depth == 1 && start.name().as_ref() == b"recording_start_time_ms" {
                    origin_text = Some(String::new());
                } else if pending.is_some() || origin_text.is_some() {
                    return Err("弹幕 XML 文本元素不能包含子元素".into());
                }
                depth += 1;
            }
            Event::Empty(start) => {
                let attrs = attributes(&start)?;
                if depth == 0 {
                    if root_seen || start.name().as_ref() != b"i" {
                        return Err("弹幕 XML 必须只有一个 i 根元素".into());
                    }
                    root_seen = true;
                    root_closed = true;
                    if let Some((_, value)) = attrs
                        .iter()
                        .find(|(key, _)| key == b"recording_start_time_ms")
                    {
                        set_origin(&mut result, value)?;
                    }
                } else if depth == 1 && start.name().as_ref() == b"d" {
                    let _ = comment(&attrs)?;
                } else if depth == 1 && start.name().as_ref() == b"recording_start_time_ms" {
                    return Err("弹幕 XML 的录制起点为空".into());
                } else if pending.is_some() || origin_text.is_some() {
                    return Err("弹幕 XML 文本元素不能包含子元素".into());
                }
            }
            Event::Text(text) => {
                let text = text
                    .unescape()
                    .map_err(|e| format!("弹幕 XML 转义损坏：{e}"))?;
                if let Some(pending) = &mut pending {
                    pending.text.push_str(&text);
                } else if let Some(origin) = &mut origin_text {
                    origin.push_str(&text);
                } else if depth == 0 && !text.trim().is_empty() {
                    return Err("弹幕 XML 根元素外存在内容".into());
                }
            }
            Event::CData(text) => {
                let text = std::str::from_utf8(&text).map_err(|_| "弹幕 XML 不是 UTF-8")?;
                if let Some(pending) = &mut pending {
                    pending.text.push_str(text);
                } else if let Some(origin) = &mut origin_text {
                    origin.push_str(text);
                } else if depth == 0 {
                    return Err("弹幕 XML 根元素外存在内容".into());
                }
            }
            Event::End(end) => {
                depth = depth.checked_sub(1).ok_or("弹幕 XML 的结束元素无效")?;
                if depth == 1 && end.name().as_ref() == b"d" {
                    let pending = pending.take().ok_or("弹幕 XML 的 d 元素无效")?;
                    if !pending.text.trim().is_empty() {
                        result.comments.push(pending);
                        if result.comments.len() > comment_limit {
                            return Err("弹幕数量超过处理上限".into());
                        }
                    }
                }
                if depth == 1 && end.name().as_ref() == b"recording_start_time_ms" {
                    set_origin(&mut result, &origin_text.take().unwrap_or_default())?;
                }
                if depth == 0 {
                    root_closed = true;
                }
            }
            Event::DocType(_) => return Err("弹幕 XML 不允许 DTD".into()),
            Event::Eof => break,
            _ => {}
        }
    }
    if !root_seen || !root_closed || depth != 0 {
        return Err("弹幕 XML 已损坏或未写完".into());
    }
    Ok(result)
}

pub fn timing_origin(parsed: &ParsedXml) -> (Option<i64>, bool) {
    if let Some(origin) = parsed.recording_start_time_ms {
        return (Some(origin), false);
    }
    let mut estimates: Vec<i64> = parsed
        .comments
        .iter()
        .filter_map(|comment| comment.unix_ms.map(|unix| unix - comment.elapsed_ms))
        .collect();
    estimates.sort_unstable();
    (estimates.get(estimates.len() / 2).copied(), true)
}

fn corrected_time(elapsed: i64, correction: i64, offset: i64) -> Result<i64, String> {
    elapsed
        .checked_add(correction)
        .and_then(|time| time.checked_add(offset))
        .ok_or_else(|| "弹幕时间偏移超出范围".into())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// One serialization pass over logical, already XML-decoded text. The Gate 0
/// bound is tested through this function and `parse_xml`, the production path.
fn normalize_xml(comments: &[(&XmlComment, i64)], origin: i64) -> Result<String, String> {
    let budget =
        normalized_xml_budget(MAX_XML_BYTES, MAX_COMMENTS as u64).ok_or("弹幕归一化预算溢出")?;
    normalize_xml_with_limit(comments, origin, budget)
}

fn normalize_xml_with_limit(
    comments: &[(&XmlComment, i64)],
    origin: i64,
    budget: u64,
) -> Result<String, String> {
    let mut normalized = BoundedWriter::new(Vec::new(), budget);
    normalized
        .write_all(NORMALIZED_HEADER.as_bytes())
        .map_err(|e| e.to_string())?;
    for (comment, at) in comments {
        writeln!(
            normalized,
            "<d p=\"{:.3},{},{},{},0,0,0,0\">{}</d>",
            (at - origin) as f64 / 1000.,
            comment.mode,
            comment.size,
            comment.color,
            escape(&comment.text)
        )
        .map_err(|e| e.to_string())?;
    }
    normalized
        .write_all(NORMALIZED_FOOTER.as_bytes())
        .map_err(|e| e.to_string())?;
    String::from_utf8(normalized.into_inner()).map_err(|e| e.to_string())
}

fn is_emoji_base(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1f000..=0x1faff | 0x2600..=0x27bf | 0x2300..=0x23ff
    )
}

fn is_emoji_extender(ch: char) -> bool {
    matches!(
        ch as u32,
        0x200d | 0xfe0e..=0xfe0f | 0x1f3fb..=0x1f3ff | 0xe0020..=0xe007f
    )
}

/// Prefer the bundled monochrome emoji face for emoji runs while leaving the
/// generated event timing, layout, colors, and ordinary CJK text untouched.
fn add_emoji_font_fallback(ass: &str) -> String {
    let mut output = String::with_capacity(ass.len() + 128);
    for (line_number, line) in ass.lines().enumerate() {
        if line_number > 0 {
            output.push('\n');
        }
        let Some(text_start) = line
            .strip_prefix("Dialogue:")
            .and_then(|_| line.match_indices(',').nth(8).map(|(at, _)| at + 1))
        else {
            output.push_str(line);
            continue;
        };
        output.push_str(&line[..text_start]);
        let text = &line[text_start..];
        let mut chars = text.char_indices().peekable();
        while let Some((_, ch)) = chars.next() {
            if !is_emoji_base(ch) {
                output.push(ch);
                continue;
            }
            output.push_str("{\\fnNoto Emoji}");
            output.push(ch);
            while let Some(&(_, next)) = chars.peek() {
                if !is_emoji_base(next) && !is_emoji_extender(next) {
                    break;
                }
                chars.next();
                output.push(next);
            }
            output.push_str("{\\fnNoto Sans CJK SC}");
        }
    }
    output
}

pub fn empty_ass(width: u32, height: u32) -> String {
    format!(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: {width}\nPlayResY: {height}\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Noto Sans CJK SC,38,&H00FFFFFF,&H00FFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,7,0,0,0,1\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n"
    )
}

/// Generates the same ASS used by browser preview and FFmpeg. Offsets are applied
/// before layout; a negative earliest event shifts the *clock*, never truncates
/// the event or restarts its motion at a cut boundary.
pub async fn generate(
    source: &RenderSource,
    settings: &DanmakuSettings,
    width: u32,
    height: u32,
    work: &Arc<ManagedRoot>,
    cancel: &CancellationToken,
) -> Result<AssPreview, String> {
    let mut preview = AssPreview {
        ass: empty_ass(width, height),
        width,
        height,
        time_origin_ms: 0,
        estimated_timing: false,
        comment_count: 0,
    };
    if !settings.enabled {
        return Ok(preview);
    }
    if settings.font.contains(',') {
        return Err("弹幕字体名称不能包含逗号".into());
    }
    let path = source
        .danmaku_path
        .as_ref()
        .ok_or("没有录制弹幕 XML，无法生成弹幕版；可以关闭弹幕后导出")?;
    let size = tokio::fs::metadata(path)
        .await
        .map_err(|e| format!("读取弹幕 XML 失败：{e}"))?
        .len();
    if size > MAX_XML_BYTES {
        return Err("弹幕 XML 超过 128 MB".into());
    }
    use tokio::io::AsyncReadExt;
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|e| format!("读取弹幕 XML 失败：{e}"))?;
    let mut bytes = Vec::new();
    file.take(MAX_XML_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| format!("读取弹幕 XML 失败：{e}"))?;
    let parsed = parse_xml(&bytes)?;
    drop(bytes);
    let (xml_origin, estimated) = timing_origin(&parsed);
    preview.estimated_timing = estimated;
    preview.comment_count = parsed.comments.len();
    if parsed.comments.is_empty() || settings.opacity == 0. {
        return Ok(preview);
    }
    let correction = match (xml_origin, source.source_origin_ms) {
        (Some(xml), Some(video)) => xml.checked_sub(video).ok_or("弹幕录制起点差值超出范围")?,
        _ => {
            preview.estimated_timing = true;
            0
        }
    };
    let offset = settings
        .offset_ms
        .checked_add(
            settings
                .segment_offsets
                .get(&source.segment_id)
                .copied()
                .unwrap_or(0),
        )
        .ok_or("弹幕时间偏移超出范围")?;
    let mut comments = Vec::new();
    for comment in &parsed.comments {
        let at = corrected_time(comment.elapsed_ms, correction, offset)?;
        if at.unsigned_abs() <= MAX_COMMENT_OFFSET_MS as u64 {
            comments.push((comment, at));
        }
    }
    if comments.is_empty() {
        return Ok(preview);
    }
    let origin = comments.iter().map(|(_, at)| *at).min().unwrap_or(0).min(0);
    preview.time_origin_ms = origin;
    let normalized = normalize_xml(&comments, origin)?;
    let expected = normalized.matches("<d p=").count();
    drop(comments);
    drop(parsed);
    let output = convert_in_worker(
        work,
        normalized.into_bytes(),
        &factory_options(settings, width, height),
        cancel,
    )
    .await?;
    let stats = output.stats;
    if stats.danmaku_count as usize != expected {
        tracing::warn!(
            expected,
            read = stats.danmaku_count,
            "DanmakuFactory read a different number of danmaku than were written"
        );
    }
    preview.ass = decode_ass(&output.ass);
    if !preview.ass.contains("[Events]") || !preview.ass.contains("PlayResX:") {
        return Err("弹幕转换生成的 ASS 无效".into());
    }
    // Factory accepts outline <= 4, while our public style allows wider borders.
    // Change style values after layout without touching cue times or motion.
    preview.ass = preview
        .ass
        .lines()
        .map(|line| {
            if let Some(style) = line.strip_prefix("Style: ") {
                let mut values: Vec<String> = style.split(',').map(str::to_owned).collect();
                if values.len() >= 18 && matches!(values[0].as_str(), "R2L" | "L2R" | "TOP" | "BTM")
                {
                    values[2] = settings.font_size.to_string();
                    values[16] = settings.outline.to_string();
                    return format!("Style: {}", values.join(","));
                }
            }
            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n");
    preview.ass = add_emoji_font_fallback(&preview.ass);
    Ok(preview)
}

/// The DanmakuFactory options biliup has always used: those of the former
/// `DanmakuFactory --resolution WxH --fontname .. --fontsize N
/// --font-size-strict --opacity N --outline N --shadow 0 --scrolltime S
/// --fixtime S --displayarea A --density D`, everything else at the CLI's
/// defaults. Values go through f32 exactly like the CLI's atof()+cast.
pub(super) fn factory_options(
    settings: &DanmakuSettings,
    width: u32,
    height: u32,
) -> ConvertOptions {
    ConvertOptions {
        width,
        height,
        font_name: settings.font.clone(),
        font_size: settings.font_size.round() as i32,
        font_size_strict: true,
        opacity: (settings.opacity * 255.).round().clamp(1., 255.) as u8,
        outline: settings.outline.min(4.) as f32,
        shadow: 0.,
        scroll_time: settings.scroll_seconds as f32,
        fix_time: settings.scroll_seconds as f32,
        display_area: settings.display_area as f32,
        density: settings.density,
        ..ConvertOptions::default()
    }
}

/// Each worker owns an exclusive attempt directory under a shared root owner.
/// The detached supervisor reaps the child before reclaiming its files.
async fn convert_in_worker(
    work: &Arc<ManagedRoot>,
    xml: Vec<u8>,
    options: &ConvertOptions,
    cancel: &CancellationToken,
) -> Result<WorkerOutput, String> {
    let request = Request {
        protocol: PROTOCOL,
        input: XML_FILE.into(),
        output: ASS_FILE.into(),
        result: RESULT_FILE.into(),
        options: options.clone(),
        #[cfg(test)]
        test_hook: None,
    };
    run_worker(work, xml, &request, cancel, CONVERT_TIMEOUT).await
}

async fn run_worker(
    work: &Arc<ManagedRoot>,
    xml: Vec<u8>,
    request: &Request,
    cancel: &CancellationToken,
    timeout: Duration,
) -> Result<WorkerOutput, String> {
    process::run(work, xml, request, cancel, timeout).await
}

fn describe(error: &ConvertError) -> String {
    const ENOSPC: i32 = 28;
    match error.kind {
        ErrorKind::OutputWrite if error.os_error == Some(ENOSPC) => {
            "磁盘空间不足，弹幕转换失败".into()
        }
        ErrorKind::OutputWrite | ErrorKind::OutputCreate | ErrorKind::OutputExists => {
            format!("写入弹幕 ASS 失败：{error}")
        }
        ErrorKind::InputOpen | ErrorKind::InputRead => format!("读取弹幕转换文件失败：{error}"),
        ErrorKind::InvalidArgument => format!("弹幕转换参数无效：{error}"),
        ErrorKind::NoDanmaku => "弹幕转换没有读到弹幕".into(),
        ErrorKind::UnsupportedInput => format!("弹幕 XML 含有无法转换的弹幕：{error}"),
        ErrorKind::OutOfMemory => "弹幕转换内存不足".into(),
        ErrorKind::Locale => "弹幕转换进程的数字格式（LC_NUMERIC）不受支持".into(),
        ErrorKind::Internal => format!("弹幕转换内部错误：{error}"),
    }
}

/// Accept legacy ASS artifacts with a BOM or truncated UTF-8 while new native
/// conversion output retains complete, validated UTF-8.
pub(super) fn decode_ass(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut text = String::with_capacity(bytes.len());
    for chunk in bytes.utf8_chunks() {
        text.push_str(chunk.valid());
    }
    text
}

const SELF_TEST_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><i>\n<d p=\"0.000,1,25,16777215,0,0,0,0\">biliup 弹幕</d>\n</i>\n";

/// Converts a one-comment XML in a worker, which proves that this
/// installation can start the worker and that the linked core works.
pub async fn check_converter(cancel: &CancellationToken) -> Result<(), String> {
    let temp = Arc::new(
        ManagedRoot::new(
            crate::danmaku_worker::managed_preview_root()
                .map_err(|e| format!("无法创建弹幕转换检查目录：{e}"))?,
        )
        .map_err(|e| format!("无法登记弹幕转换检查目录：{e}"))?,
    );
    let request = Request {
        protocol: PROTOCOL,
        input: XML_FILE.into(),
        output: ASS_FILE.into(),
        result: RESULT_FILE.into(),
        options: ConvertOptions {
            width: 64,
            height: 36,
            font_size: 16,
            ..ConvertOptions::default()
        },
        #[cfg(test)]
        test_hook: None,
    };
    let output = run_worker(
        &temp,
        SELF_TEST_XML.as_bytes().to_vec(),
        &request,
        cancel,
        SELF_TEST_TIMEOUT,
    )
    .await?;
    let ass = decode_ass(&output.ass);
    if output.stats.danmaku_count != 1 || !ass.contains("biliup\\h弹幕") {
        return Err("弹幕转换检查的结果不正确".into());
    }
    ManagedRoot::finish(temp, Ok(()))
}

#[cfg(test)]
#[path = "danmaku_tests.rs"]
mod tests;
