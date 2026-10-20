//! Strict XML validation and complete-segment ASS generation. Never clips or
//! re-lays out comments at a clip's in-point.
//!
//! The native Rust layout (`danmaku-factory`) is linked into this program, so
//! a conversion is an in-process call: no converter program is started and
//! nothing is negotiated over a command line or a result file. Each conversion
//! runs under a bounded lifetime. Preview writes to its response buffer; exports
//! stream to an owned ASS file and hand it to libass only after conversion succeeds.
use super::engine::AssPreview;
use super::model::{DanmakuSettings, RenderSource};
use danmaku_factory::{ConvertError, ConvertOptions, ErrorKind};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::io::{self, BufRead, BufReader, BufWriter, Cursor, Read, Write};
use std::path::Path;
use std::time::Duration;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

use danmaku_factory::limits::{MAX_COMMENTS, MAX_XML_BYTES, NORMALIZED_FOOTER, NORMALIZED_HEADER};
const CONVERT_TIMEOUT: Duration = Duration::from_secs(120);
const SELF_TEST_TIMEOUT: Duration = Duration::from_secs(15);
/// Comments further than 250 hours from the segment start can never be on
/// screen. Dropping them keeps every time the core sees within its `int`
/// millisecond compatibility range; real recordings never get close.
const MAX_COMMENT_OFFSET_MS: i64 = 250 * 3_600_000;

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
#[cfg(test)]
pub fn parse_xml(bytes: &[u8]) -> Result<ParsedXml, String> {
    parse_xml_with_limit(bytes, MAX_XML_BYTES, MAX_COMMENTS)
}

#[cfg(test)]
fn parse_xml_with_limit(
    bytes: &[u8],
    byte_limit: u64,
    comment_limit: usize,
) -> Result<ParsedXml, String> {
    if bytes.len() as u64 > byte_limit {
        return Err("弹幕 XML 超过处理上限".into());
    }
    parse_xml_reader(Cursor::new(bytes), byte_limit, comment_limit)
}

fn parse_xml_reader(
    input: impl BufRead,
    byte_limit: u64,
    comment_limit: usize,
) -> Result<ParsedXml, String> {
    let mut reader = Reader::from_reader(input);
    let mut buffer = Vec::new();
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
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|e| format!("弹幕 XML 已损坏或未写完：{e}"))?;
        if reader.buffer_position() > byte_limit {
            return Err("弹幕 XML 超过处理上限".into());
        }
        match event {
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
        buffer.clear();
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

#[cfg(test)]
fn normalize_xml(comments: &[(&XmlComment, i64)], origin: i64) -> Result<String, String> {
    let budget = danmaku_factory::limits::normalized_xml_budget(MAX_XML_BYTES, MAX_COMMENTS as u64)
        .ok_or("弹幕归一化预算溢出")?;
    normalize_xml_with_limit(comments, origin, budget)
}

#[cfg(test)]
fn normalize_xml_with_limit(
    comments: &[(&XmlComment, i64)],
    origin: i64,
    budget: u64,
) -> Result<String, String> {
    let mut xml = NormalizedXml {
        comments: comments
            .iter()
            .map(|(comment, at)| {
                let mut comment = (*comment).clone();
                comment.elapsed_ms = *at;
                comment
            })
            .collect::<Vec<_>>()
            .into_iter(),
        current: Cursor::new(NORMALIZED_HEADER.as_bytes().to_vec()),
        correction: 0,
        offset: 0,
        origin,
        footer_written: false,
        control: ConversionControl::new(&CancellationToken::new(), CONVERT_TIMEOUT),
    };
    let mut output = danmaku_factory::limits::BoundedWriter::new(Vec::new(), budget);
    io::copy(&mut xml, &mut output).map_err(|e| e.to_string())?;
    String::from_utf8(output.into_inner()).map_err(|e| e.to_string())
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

/// Timing information shared by the preview response and file export.
#[derive(Debug)]
pub struct AssMetadata {
    pub time_origin_ms: i64,
    pub estimated_timing: bool,
    pub comment_count: usize,
}

#[derive(Clone)]
pub(super) struct ConversionControl {
    cancel: CancellationToken,
    deadline: Instant,
}

impl ConversionControl {
    pub(super) fn new(cancel: &CancellationToken, timeout: Duration) -> Self {
        Self {
            cancel: cancel.clone(),
            deadline: Instant::now() + timeout,
        }
    }

    pub(super) fn check(&self) -> io::Result<()> {
        if self.cancel.is_cancelled() {
            Err(io::Error::new(io::ErrorKind::Interrupted, "合成已取消"))
        } else if Instant::now() >= self.deadline {
            Err(io::Error::new(io::ErrorKind::TimedOut, "弹幕转换超时"))
        } else {
            Ok(())
        }
    }
}

struct CheckedReader<R> {
    input: R,
    control: ConversionControl,
}

impl<R: Read> Read for CheckedReader<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.control.check()?;
        self.input.read(bytes)
    }
}

/// Serializes one decoded comment at a time. The complete normalized XML is
/// never allocated or written to disk; consumed comment strings are released.
struct NormalizedXml {
    comments: std::vec::IntoIter<XmlComment>,
    current: Cursor<Vec<u8>>,
    correction: i64,
    offset: i64,
    origin: i64,
    footer_written: bool,
    control: ConversionControl,
}

impl Read for NormalizedXml {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        loop {
            self.control.check()?;
            let count = self.current.read(bytes)?;
            if count != 0 {
                return Ok(count);
            }
            if let Some(comment) = self.comments.next() {
                let at = corrected_time(comment.elapsed_ms, self.correction, self.offset)
                    .map_err(io::Error::other)?;
                if at.unsigned_abs() > MAX_COMMENT_OFFSET_MS as u64 {
                    continue;
                }
                self.current = Cursor::new(
                    format!(
                        "<d p=\"{:.3},{},{},{},0,0,0,0\">{}</d>\n",
                        (at - self.origin) as f64 / 1000.,
                        comment.mode,
                        comment.size,
                        comment.color,
                        escape(&comment.text),
                    )
                    .into_bytes(),
                );
            } else if !self.footer_written {
                self.footer_written = true;
                self.current = Cursor::new(NORMALIZED_FOOTER.as_bytes().to_vec());
            } else {
                return Ok(0);
            }
        }
    }
}

/// Applies public style values and emoji fallback with a single-line buffer.
/// Native ASS event timing and collision placement pass through unchanged.
struct StyledAssWriter<W> {
    output: W,
    line: Vec<u8>,
    first: bool,
    settings: DanmakuSettings,
    control: ConversionControl,
}

impl<W: Write> StyledAssWriter<W> {
    fn line(&mut self) -> io::Result<()> {
        self.control.check()?;
        let text = std::str::from_utf8(&self.line).map_err(io::Error::other)?;
        let text = text.trim_end_matches('\r');
        let text = if self.first {
            text.trim_start_matches('\u{feff}')
        } else {
            text
        };
        let mut styled = None;
        if let Some(style) = text.strip_prefix("Style: ") {
            let mut values: Vec<_> = style.split(',').map(str::to_owned).collect();
            if values.len() >= 18 && matches!(values[0].as_str(), "R2L" | "L2R" | "TOP" | "BTM") {
                values[2] = self.settings.font_size.to_string();
                values[16] = self.settings.outline.to_string();
                styled = Some(format!("Style: {}", values.join(",")));
            }
        }
        if !self.first {
            self.output.write_all(b"\n")?;
        }
        self.first = false;
        self.output
            .write_all(add_emoji_font_fallback(styled.as_deref().unwrap_or(text)).as_bytes())?;
        self.line.clear();
        Ok(())
    }
}

impl<W: Write> Write for StyledAssWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.control.check()?;
        for part in bytes.split_inclusive(|&byte| byte == b'\n') {
            if let Some(body) = part.strip_suffix(b"\n") {
                self.line.extend_from_slice(body);
                self.line()?;
            } else {
                self.line.extend_from_slice(part);
            }
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.line.is_empty() {
            self.line()?;
        }
        self.output.flush()
    }
}

struct PreparedConversion {
    metadata: AssMetadata,
    xml: Option<NormalizedXml>,
}

fn prepare_conversion(
    source: &RenderSource,
    settings: &DanmakuSettings,
    control: &ConversionControl,
) -> Result<PreparedConversion, String> {
    control.check().map_err(|e| e.to_string())?;
    let mut metadata = AssMetadata {
        time_origin_ms: 0,
        estimated_timing: false,
        comment_count: 0,
    };
    if !settings.enabled {
        return Ok(PreparedConversion {
            metadata,
            xml: None,
        });
    }
    if settings.font.contains(',') {
        return Err("弹幕字体名称不能包含逗号".into());
    }
    let path = source
        .danmaku_path
        .as_ref()
        .ok_or("没有录制弹幕 XML，无法生成弹幕版；可以关闭弹幕后导出")?;
    let file = std::fs::File::open(path).map_err(|e| format!("读取弹幕 XML 失败：{e}"))?;
    let input = CheckedReader {
        input: file.take(MAX_XML_BYTES + 1),
        control: control.clone(),
    };
    let parsed = parse_xml_reader(BufReader::new(input), MAX_XML_BYTES, MAX_COMMENTS)?;
    let (xml_origin, estimated) = timing_origin(&parsed);
    metadata.estimated_timing = estimated;
    metadata.comment_count = parsed.comments.len();
    if parsed.comments.is_empty() || settings.opacity == 0. {
        return Ok(PreparedConversion {
            metadata,
            xml: None,
        });
    }
    let correction = match (xml_origin, source.source_origin_ms) {
        (Some(xml), Some(video)) => xml.checked_sub(video).ok_or("弹幕录制起点差值超出范围")?,
        _ => {
            metadata.estimated_timing = true;
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
    let mut origin = 0;
    let mut eligible = false;
    for comment in &parsed.comments {
        let at = corrected_time(comment.elapsed_ms, correction, offset)?;
        if at.unsigned_abs() <= MAX_COMMENT_OFFSET_MS as u64 {
            eligible = true;
            origin = origin.min(at);
        }
    }
    metadata.time_origin_ms = origin;
    let xml = eligible.then(|| NormalizedXml {
        comments: parsed.comments.into_iter(),
        current: Cursor::new(NORMALIZED_HEADER.as_bytes().to_vec()),
        correction,
        offset,
        origin,
        footer_written: false,
        control: control.clone(),
    });
    Ok(PreparedConversion { metadata, xml })
}

fn write_prepared(
    prepared: PreparedConversion,
    settings: &DanmakuSettings,
    width: u32,
    height: u32,
    mut output: impl Write,
    control: &ConversionControl,
) -> Result<AssMetadata, String> {
    if let Some(xml) = prepared.xml {
        let mut writer = StyledAssWriter {
            output: &mut output,
            line: Vec::new(),
            first: true,
            settings: settings.clone(),
            control: control.clone(),
        };
        danmaku_factory::convert_normalized_xml_to_writer(
            xml,
            &mut writer,
            &factory_options(settings, width, height),
        )
        .map_err(|e| describe(&e))?;
        writer
            .flush()
            .map_err(|e| format!("写入弹幕 ASS 失败：{e}"))?;
    } else {
        control.check().map_err(|e| e.to_string())?;
        output
            .write_all(empty_ass(width, height).as_bytes())
            .map_err(|e| e.to_string())?;
    }
    Ok(prepared.metadata)
}

/// Only browser preview collects the final ASS text, as required by its API.
pub async fn generate(
    source: &RenderSource,
    settings: &DanmakuSettings,
    width: u32,
    height: u32,
    cancel: &CancellationToken,
) -> Result<AssPreview, String> {
    let (source, settings) = (source.clone(), settings.clone());
    let control = ConversionControl::new(cancel, CONVERT_TIMEOUT);
    supervised(CONVERT_TIMEOUT, cancel, move || {
        let prepared = prepare_conversion(&source, &settings, &control)?;
        let mut output = Vec::new();
        let metadata = write_prepared(prepared, &settings, width, height, &mut output, &control)?;
        Ok(AssPreview {
            ass: String::from_utf8(output).map_err(|e| e.to_string())?,
            width,
            height,
            time_origin_ms: metadata.time_origin_ms,
            estimated_timing: metadata.estimated_timing,
            comment_count: metadata.comment_count,
        })
    })
    .await
}

/// libass opens a seekable filename. Stream its input into one owned `.ass`
/// file beside the final video; no conversion directory or normalized XML file.
/// The blocking converter owns the handle until it stops, including cancellation.
pub(super) async fn generate_for_render(
    source: &RenderSource,
    settings: &DanmakuSettings,
    width: u32,
    height: u32,
    parent: &Path,
    cancel: &CancellationToken,
) -> Result<(tempfile::NamedTempFile, AssMetadata), String> {
    let (source, settings, parent) = (source.clone(), settings.clone(), parent.to_owned());
    let control = ConversionControl::new(cancel, CONVERT_TIMEOUT);
    supervised(CONVERT_TIMEOUT, cancel, move || {
        let mut file = tempfile::Builder::new()
            .prefix(".biliup-ass-")
            .suffix(".ass")
            .tempfile_in(parent)
            .map_err(|e| format!("创建 ASS 文件失败：{e}"))?;
        let prepared = prepare_conversion(&source, &settings, &control)?;
        let mut writer = BufWriter::new(file.as_file_mut());
        let metadata = write_prepared(prepared, &settings, width, height, &mut writer, &control)?;
        writer
            .flush()
            .map_err(|e| format!("写入 ASS 文件失败：{e}"))?;
        drop(writer);
        control.check().map_err(|e| e.to_string())?;
        Ok((file, metadata))
    })
    .await
}

#[cfg(test)]
pub(super) fn publish_ass<T>(
    output: &Path,
    control: &ConversionControl,
    write: impl FnOnce(&mut BufWriter<&mut std::fs::File>) -> Result<T, String>,
) -> Result<T, String> {
    control.check().map_err(|e| e.to_string())?;
    if std::fs::symlink_metadata(output).is_ok() {
        return Err("弹幕 ASS 文件已经存在，拒绝覆盖".into());
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    #[cfg(windows)]
    let parent = parent
        .canonicalize()
        .map_err(|e| format!("解析弹幕 ASS 目录失败：{e}"))?;
    #[cfg(windows)]
    let destination = parent.join(output.file_name().ok_or("弹幕 ASS 文件名无效")?);
    #[cfg(windows)]
    let output = destination.as_path();
    let prefix = format!(
        ".{}-",
        output
            .file_name()
            .ok_or("弹幕 ASS 文件名无效")?
            .to_string_lossy()
    );
    let mut part = tempfile::Builder::new()
        .prefix(&prefix)
        .suffix(".part")
        .tempfile_in(parent)
        .map_err(|e| format!("创建弹幕 ASS .part 文件失败：{e}"))?;
    let result = (|| {
        let mut writer = BufWriter::new(part.as_file_mut());
        let result = write(&mut writer)?;
        writer
            .flush()
            .map_err(|e| format!("写入弹幕 ASS 失败：{e}"))?;
        drop(writer);
        control.check().map_err(|e| e.to_string())?;
        part.as_file()
            .sync_all()
            .map_err(|e| format!("保存弹幕 ASS 失败：{e}"))?;
        control.check().map_err(|e| e.to_string())?;
        Ok(result)
    })();
    match result {
        Ok(value) => part
            .persist_noclobber(output)
            .map(|_| value)
            .map_err(|e| format!("发布弹幕 ASS 失败：{}", e.error)),
        Err(primary) => match part.close() {
            Ok(()) => Err(primary),
            Err(error) => Err(format!("{primary}; cleanup .part: {error}")),
        },
    }
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

/// Runs one blocking operation under a bounded lifetime.
///
/// Cancellation answers the caller immediately. Reader/writer checkpoints and
/// the publication boundary stop cooperative work; a sorting/layout pass may
/// finish on its blocking thread first. The output handle stays on that
/// thread until it stops; cancelled or expired work cannot publish a final file.
async fn supervised<T, F>(timeout: Duration, cancel: &CancellationToken, op: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    // Dropping the handle of a still-running blocking task never cancels it,
    // which is exactly the lifetime this function promises.
    let mut task = tokio::task::spawn_blocking(op);
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err("合成已取消".into()),
        _ = tokio::time::sleep(timeout) => Err(format!(
            "弹幕转换失败：转换超过 {} 秒，任务已超时；可重试",
            timeout.as_secs()
        )),
        joined = &mut task => match joined {
            Ok(result) => result,
            Err(error) => Err(format!("弹幕转换任务失败：{error}")),
        },
    }
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
        ErrorKind::Locale => "弹幕转换的数字格式（LC_NUMERIC）不受支持".into(),
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

/// Check the linked core using streams, without creating any files.
pub async fn check_converter(cancel: &CancellationToken) -> Result<(), String> {
    let control = ConversionControl::new(cancel, SELF_TEST_TIMEOUT);
    supervised(SELF_TEST_TIMEOUT, cancel, move || {
        let options = ConvertOptions {
            width: 64,
            height: 36,
            font_size: 16,
            ..Default::default()
        };
        let mut output = Vec::new();
        let stats = danmaku_factory::convert_normalized_xml_to_writer(
            CheckedReader {
                input: SELF_TEST_XML.as_bytes(),
                control,
            },
            &mut output,
            &options,
        )
        .map_err(|e| describe(&e))?;
        if stats.danmaku_count != 1 || !decode_ass(&output).contains("biliup\\h弹幕") {
            return Err("弹幕转换检查的结果不正确".into());
        }
        Ok(())
    })
    .await
}

#[cfg(test)]
#[path = "danmaku_tests.rs"]
mod tests;
