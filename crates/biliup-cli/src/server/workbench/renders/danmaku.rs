//! Strict XML validation and complete-segment ASS generation. Never clips or
//! re-lays out comments at a clip's in-point.
use super::engine::{AssPreview, run_process};
use super::model::{DanmakuSettings, RenderSource};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::path::Path;
use std::process::Stdio;
use tokio_util::sync::CancellationToken;

const MAX_XML_BYTES: u64 = 128 * 1024 * 1024;
const MAX_COMMENTS: usize = 1_000_000;

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
        .collect();
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
    let input = work.join("danmaku.xml");
    let output = work.join("danmaku.ass");
    tokio::fs::write(&input, normalized)
        .await
        .map_err(|e| format!("创建弹幕转换文件失败：{e}"))?;
    let factory = crate::tools::danmaku_factory()?;
    let factory =
        std::fs::canonicalize(&factory).map_err(|e| format!("打开 DanmakuFactory 失败：{e}"))?;
    let mut command = crate::tools::command(factory);
    // Work in an isolated directory: Factory's implicit config lookup must never
    // inherit another user's settings or write next to their source XML.
    command
        .current_dir(work)
        .args([
            "-i",
            "danmaku.xml",
            "-o",
            "danmaku.ass",
            "--ignore-warnings",
            "--force",
        ])
        .args(["--resolution", &format!("{width}x{height}")])
        .args([
            "--fontname",
            &settings.font,
            "--fontsize",
            &settings.font_size.round().to_string(),
            "--font-size-strict",
        ])
        .args([
            "--opacity",
            &((settings.opacity * 255.).round().max(1.) as u32).to_string(),
        ])
        .args([
            "--outline",
            &settings.outline.min(4.).to_string(),
            "--shadow",
            "0",
        ])
        .args([
            "--scrolltime",
            &settings.scroll_seconds.to_string(),
            "--fixtime",
            &settings.scroll_seconds.to_string(),
        ])
        .args([
            "--displayarea",
            &settings.display_area.to_string(),
            "--density",
            &settings.density.to_string(),
        ])
        .stdin(Stdio::null());
    let _ = run_process(
        command,
        cancel,
        std::time::Duration::from_secs(120),
        |_, _| {},
    )
    .await?;
    preview.ass = tokio::fs::read_to_string(&output)
        .await
        .map_err(|e| format!("DanmakuFactory 没有生成 ASS：{e}"))?;
    if !preview.ass.contains("[Events]") || !preview.ass.contains("PlayResX:") {
        return Err("DanmakuFactory 生成的 ASS 无效".into());
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

#[cfg(test)]
#[path = "danmaku_tests.rs"]
mod tests;
