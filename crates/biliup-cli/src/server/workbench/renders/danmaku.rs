//! Strict XML validation and complete-segment ASS generation. Never clips or
//! re-lays out comments at a clip's in-point.
//!
//! The layout itself is DanmakuFactory's, statically linked
//! (`danmaku-factory`) and run by [`convert_in_worker`] in a child process of
//! this program, so it can be cancelled, timed out and survive a crash.
use super::engine::{AssPreview, run_process};
use super::model::{DanmakuSettings, RenderSource};
use crate::danmaku_worker::{self, PROTOCOL, Request, Response};
use danmaku_factory::{ConvertError, ConvertOptions, ConvertStats, ErrorKind};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const MAX_XML_BYTES: u64 = 128 * 1024 * 1024;
const MAX_COMMENTS: usize = 1_000_000;
const CONVERT_TIMEOUT: Duration = Duration::from_secs(120);
const SELF_TEST_TIMEOUT: Duration = Duration::from_secs(15);
/// Comments further than 250 hours from the segment start can never be on
/// screen. Dropping them keeps every time the core sees within its `int`
/// milliseconds (see csrc/df_ffi.h); real recordings never get close.
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
                        if result.comments.len() > MAX_COMMENTS {
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

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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
    work: &Path,
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
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| format!("读取弹幕 XML 失败：{e}"))?;
    let parsed = parse_xml(&bytes)?;
    let (xml_origin, estimated) = timing_origin(&parsed);
    preview.estimated_timing = estimated;
    preview.comment_count = parsed.comments.len();
    if parsed.comments.is_empty() || settings.opacity == 0. {
        return Ok(preview);
    }
    let correction = match (xml_origin, source.source_origin_ms) {
        (Some(xml), Some(video)) => xml - video,
        _ => {
            preview.estimated_timing = true;
            0
        }
    };
    let offset = settings.offset_ms
        + settings
            .segment_offsets
            .get(&source.segment_id)
            .copied()
            .unwrap_or(0);
    let comments: Vec<(&XmlComment, i64)> = parsed
        .comments
        .iter()
        .map(|comment| (comment, comment.elapsed_ms + correction + offset))
        .filter(|(_, at)| at.abs() <= MAX_COMMENT_OFFSET_MS)
        .collect();
    if comments.is_empty() {
        return Ok(preview);
    }
    let origin = comments.iter().map(|(_, at)| *at).min().unwrap_or(0).min(0);
    preview.time_origin_ms = origin;
    let mut normalized = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><i>\n");
    for (comment, at) in comments {
        normalized.push_str(&format!(
            "<d p=\"{:.3},{},{},{},0,0,0,0\">{}</d>\n",
            (at - origin) as f64 / 1000.,
            comment.mode,
            comment.size,
            comment.color,
            escape(&comment.text)
        ));
    }
    normalized.push_str("</i>\n");
    let expected = normalized.matches("<d p=").count();
    tokio::fs::write(work.join(XML_FILE), normalized)
        .await
        .map_err(|e| format!("创建弹幕转换文件失败：{e}"))?;
    let stats = convert_in_worker(work, &factory_options(settings, width, height), cancel).await?;
    if stats.danmaku_count as usize != expected {
        tracing::warn!(
            expected,
            read = stats.danmaku_count,
            "DanmakuFactory read a different number of danmaku than were written"
        );
    }
    let bytes = tokio::fs::read(work.join(ASS_FILE))
        .await
        .map_err(|e| format!("弹幕转换没有生成 ASS：{e}"))?;
    preview.ass = decode_ass(&bytes);
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
    Ok(preview)
}

/// The DanmakuFactory options biliup has always used: those of the former
/// `DanmakuFactory --resolution WxH --fontname .. --fontsize N
/// --font-size-strict --opacity N --outline N --shadow 0 --scrolltime S
/// --fixtime S --displayarea A --density D`, everything else at the CLI's
/// defaults. Values go through f32 exactly like the CLI's atof()+cast.
pub(super) fn factory_options(settings: &DanmakuSettings, width: u32, height: u32) -> ConvertOptions {
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

/// Converts `danmaku.xml` in `work` to `danmaku.ass` in a worker process.
/// Cancelling, a timeout or dropping the future kills and reaps the worker.
pub(super) async fn convert_in_worker(
    work: &Path,
    options: &ConvertOptions,
    cancel: &CancellationToken,
) -> Result<ConvertStats, String> {
    let request = Request {
        protocol: PROTOCOL,
        input: XML_FILE.into(),
        output: ASS_FILE.into(),
        result: RESULT_FILE.into(),
        options: options.clone(),
        #[cfg(test)]
        test_hook: None,
    };
    run_worker(work, &request, cancel, CONVERT_TIMEOUT).await
}

async fn run_worker(
    work: &Path,
    request: &Request,
    cancel: &CancellationToken,
    timeout: Duration,
) -> Result<ConvertStats, String> {
    let json = serde_json::to_vec(request).map_err(|e| format!("创建弹幕转换请求失败：{e}"))?;
    tokio::fs::write(work.join(REQUEST_FILE), json)
        .await
        .map_err(|e| format!("创建弹幕转换请求失败：{e}"))?;
    for stale in [&request.result, &request.output] {
        let _ = tokio::fs::remove_file(work.join(stale)).await;
    }
    let mut command = danmaku_worker::command(work, Path::new(REQUEST_FILE))?;
    command.stdin(Stdio::null());
    let outcome = run_process(command, cancel, timeout, |_, _| {}).await;
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    match (outcome, danmaku_worker::read_response(work, &request.result)) {
        (Ok(_), Some(Ok(Response::Converted { stats, .. }))) => Ok(stats),
        (_, Some(Ok(Response::Failed { error, .. }))) => Err(describe(&error)),
        (_, Some(Err(error))) => Err(error),
        (Err(error), _) => Err(format!("弹幕转换失败：{error}")),
        (Ok(_), _) => Err("弹幕转换进程没有返回结果".into()),
    }
}

fn describe(error: &ConvertError) -> String {
    const ENOSPC: i32 = 28;
    match error.kind {
        ErrorKind::OutputWrite if error.os_error == Some(ENOSPC) => "磁盘空间不足，弹幕转换失败".into(),
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

/// The core writes a BOM on Windows and copies texts byte by byte, cutting a
/// comment over 1023 bytes in the middle of a character; one such comment
/// must not fail a whole export. Invalid sequences are dropped.
pub(super) fn decode_ass(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut text = String::with_capacity(bytes.len());
    for chunk in bytes.utf8_chunks() {
        text.push_str(chunk.valid());
    }
    text
}

const SELF_TEST_XML: &str =
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><i>\n<d p=\"0.000,1,25,16777215,0,0,0,0\">biliup 弹幕</d>\n</i>\n";

/// Converts a one-comment XML in a worker, which proves that this
/// installation can start the worker and that the linked core works.
pub async fn check_converter(cancel: &CancellationToken) -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|e| format!("无法创建弹幕转换检查目录：{e}"))?;
    tokio::fs::write(temp.path().join(XML_FILE), SELF_TEST_XML)
        .await
        .map_err(|e| format!("无法创建弹幕转换检查文件：{e}"))?;
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
    let stats = run_worker(temp.path(), &request, cancel, SELF_TEST_TIMEOUT).await?;
    let ass = tokio::fs::read(temp.path().join(ASS_FILE))
        .await
        .map(|bytes| decode_ass(&bytes))
        .map_err(|e| format!("弹幕转换检查没有生成 ASS：{e}"))?;
    if stats.danmaku_count != 1 || !ass.contains("biliup\\h弹幕") {
        return Err("弹幕转换检查的结果不正确".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "danmaku_tests.rs"]
mod tests;
