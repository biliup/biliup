//! Owned ASS input and legacy XML output, ported from DanmakuFactory.
//!
//! Byte strings retain the upstream parser's bounded fields and non-UTF-8
//! inputs. Dropping these values replaces the C allocation/free protocol.

use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;

use crate::model::{BOTTOM, CoreError, Danmaku, EPS, L2R, R2L, SPECIAL, SpecialPart, TOP};
use crate::string::{
    ByteCursor, ass_unescape, de_style_name_prefix, float_to_str, int_time_to_str, nul_prefix,
    parse_f64, parse_i32, time_to_int, to_dec_color, trim,
};
use crate::{
    DF_ERR_INPUT_OPEN, DF_ERR_INPUT_READ, DF_ERR_NO_DANMAKU, DF_ERR_OUTPUT_CREATE,
    DF_ERR_OUTPUT_WRITE, DF_ERR_UNSUPPORTED_INPUT,
};

const ASS_MAX_LINE_LEN: usize = 4096;
const SUBTITLE_PREFIX: &[u8] = b"danmakuFactory_ext_sub";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssFile {
    pub title: Vec<u8>,
    pub script_type: Vec<u8>,
    pub collisions: Vec<u8>,
    pub play_res_x: i32,
    pub play_res_y: i32,
    pub timer: f32,
    pub styles: Vec<AssStyle>,
    pub events: Vec<AssEvent>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssStyle {
    pub name: Vec<u8>,
    pub font_name: Vec<u8>,
    pub font_size: i32,
    pub primary_colour: Vec<u8>,
    pub secondary_colour: Vec<u8>,
    pub outline_colour: Vec<u8>,
    pub back_colour: Vec<u8>,
    pub bold: i32,
    pub italic: i32,
    pub underline: i32,
    pub strikeout: i32,
    pub scale_x: i32,
    pub scale_y: i32,
    pub spacing: f32,
    pub angle: f32,
    pub border_style: i32,
    pub outline: f32,
    pub shadow: f32,
    pub alignment: i32,
    pub margin_l: i32,
    pub margin_r: i32,
    pub margin_v: i32,
    pub encoding: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssEvent {
    pub event: Vec<u8>,
    pub layer: i32,
    pub start: i32,
    pub end: i32,
    pub style: Vec<u8>,
    pub name: Vec<u8>,
    pub margin_l: i32,
    pub margin_r: i32,
    pub margin_v: i32,
    pub effect: Vec<u8>,
    pub text: Vec<u8>,
}

fn bounded(text: &[u8], capacity: usize) -> Vec<u8> {
    let text = nul_prefix(text);
    text[..text.len().min(capacity.saturating_sub(1))].to_vec()
}

fn field(cursor: &mut ByteCursor<'_>, capacity: usize) -> Vec<u8> {
    bounded(trim(cursor.left_part(b',', 128)), capacity)
}

fn integer(cursor: &mut ByteCursor<'_>) -> i32 {
    parse_i32(cursor.left_part(b',', 128))
}

fn decimal(cursor: &mut ByteCursor<'_>) -> f32 {
    parse_f64(cursor.left_part(b',', 128)) as f32
}

/// Reads all implemented ASS tags; unsupported headers and sections are ignored.
pub fn read_ass_file(path: &Path) -> Result<AssFile, CoreError> {
    let mut file = File::open(path)
        .map_err(|error| CoreError::io(DF_ERR_INPUT_OPEN, 1, &error, "failed to open ASS input"))?;
    let mut text = Vec::new();
    file.read_to_end(&mut text)
        .map_err(|error| CoreError::io(DF_ERR_INPUT_READ, 1, &error, "failed to read ASS input"))?;
    #[cfg(windows)]
    let text = {
        // Match the CRT's text-mode translation before fixed-size line reads.
        let mut translated = Vec::with_capacity(text.len());
        let mut bytes = text.into_iter().peekable();
        while let Some(byte) = bytes.next() {
            if byte == 0x1A {
                break;
            }
            if byte == b'\r' && bytes.peek() == Some(&b'\n') {
                continue;
            }
            translated.push(byte);
        }
        translated
    };
    Ok(parse_ass(&text))
}

/// Parses the original fixed-order V4+ fields without requiring UTF-8.
pub fn parse_ass(text: &[u8]) -> AssFile {
    let mut output = AssFile::default();
    let mut at = 0;
    // fgets processes an overlong line as multiple 4095-byte chunks.
    while at < text.len() {
        let limit = text.len().min(at + ASS_MAX_LINE_LEN - 1);
        let end = text[at..limit]
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or(limit, |length| at + length + 1);
        let line = nul_prefix(&text[at..end]);
        let line = line.strip_suffix(b"\n").unwrap_or(line);
        at = end;
        let Some(colon) = line.iter().position(|&byte| byte == b':') else {
            continue;
        };
        // Upstream's mark buffer permits 23 bytes. Truncated tags cannot match.
        if colon > 23 {
            continue;
        }
        let mark = trim(&line[..colon]);
        let mut lower_buffer = [0; 24];
        lower_buffer[..mark.len()].copy_from_slice(mark);
        lower_buffer[..mark.len()].make_ascii_lowercase();
        let lower = &lower_buffer[..mark.len()];
        let remainder = &line[colon + 1..];
        // The upstream reader does not read Comment tags. Conversion of an
        // explicitly supplied AssEvent still accepts comments below.
        if [
            b"dialogue".as_slice(),
            b"command",
            b"picture",
            b"sound",
            b"movie",
        ]
        .contains(&lower)
        {
            let mut cursor = ByteCursor::new(remainder);
            let layer = integer(&mut cursor);
            let start = time_to_int(cursor.left_part(b',', 128));
            let finish = time_to_int(cursor.left_part(b',', 128));
            let style = field(&mut cursor, 64);
            let name = field(&mut cursor, 64);
            let margin_l = integer(&mut cursor);
            let margin_r = integer(&mut cursor);
            let margin_v = integer(&mut cursor);
            let effect = field(&mut cursor, 128);
            // The final text retains commas and uses the cursor's actual
            // bounded-field position rather than scanning the fields twice.
            output.events.push(AssEvent {
                event: bounded(mark, 16),
                layer,
                start,
                end: finish,
                style,
                name,
                margin_l,
                margin_r,
                margin_v,
                effect,
                text: cursor.remaining().to_vec(),
            });
        } else if lower == b"style" {
            let mut cursor = ByteCursor::new(remainder);
            let name = field(&mut cursor, 64);
            let font_name = field(&mut cursor, 128);
            let font_size = integer(&mut cursor);
            let primary_colour = field(&mut cursor, 16);
            let secondary_colour = field(&mut cursor, 16);
            let outline_colour = field(&mut cursor, 16);
            let back_colour = field(&mut cursor, 16);
            let bold = integer(&mut cursor);
            let italic = integer(&mut cursor);
            let underline = integer(&mut cursor);
            let strikeout = integer(&mut cursor);
            let scale_x = integer(&mut cursor);
            let scale_y = integer(&mut cursor);
            let spacing = decimal(&mut cursor);
            let angle = decimal(&mut cursor);
            let border_style = integer(&mut cursor);
            let outline = decimal(&mut cursor);
            let shadow = decimal(&mut cursor);
            let alignment = integer(&mut cursor);
            let margin_l = integer(&mut cursor);
            let margin_r = integer(&mut cursor);
            let margin_v = integer(&mut cursor);
            let encoding = parse_i32(cursor.remaining());
            output.styles.push(AssStyle {
                name,
                font_name,
                font_size,
                primary_colour,
                secondary_colour,
                outline_colour,
                back_colour,
                bold,
                italic,
                underline,
                strikeout,
                scale_x,
                scale_y,
                spacing,
                angle,
                border_style,
                outline,
                shadow,
                alignment,
                margin_l,
                margin_r,
                margin_v,
                encoding,
            });
        } else {
            match lower {
                b"title" => output.title = bounded(trim(remainder), 128),
                b"scripttype" => output.script_type = bounded(trim(remainder), 16),
                b"collisions" => output.collisions = bounded(trim(remainder), 16),
                b"playresx" => output.play_res_x = parse_i32(remainder),
                b"playresy" => output.play_res_y = parse_i32(remainder),
                b"timer" => output.timer = parse_f64(remainder) as f32,
                _ => {}
            }
        }
    }
    output
}

/// Converts the parsed ASS events, optionally appending to an existing pool.
/// The returned owned file contains the separately named external subtitles.
pub fn ass_file_to_danmaku(
    input: &AssFile,
    pool: &mut Vec<Danmaku>,
    append: bool,
    time_shift: f32,
) -> Result<AssFile, CoreError> {
    if !time_shift.is_finite() {
        return Err(CoreError::new(
            DF_ERR_UNSUPPORTED_INPUT,
            0,
            0,
            "non-finite ASS time shift",
        ));
    }
    if !append {
        pool.clear();
    }
    let mut subtitles = AssFile {
        script_type: b"v4.00+".to_vec(),
        collisions: b"Normal".to_vec(),
        timer: 100.0,
        ..Default::default()
    };
    for style in &input.styles {
        if style.name.starts_with(SUBTITLE_PREFIX) {
            let mut style = style.clone();
            style.name = de_style_name_prefix(&style.name).unwrap_or(style.name);
            subtitles.styles.push(style);
        }
    }
    for event in &input.events {
        if event.style.starts_with(SUBTITLE_PREFIX) {
            let mut subtitle = event.clone();
            subtitle.event.make_ascii_lowercase();
            subtitle.name.make_ascii_lowercase();
            subtitle.style = de_style_name_prefix(&subtitle.style).unwrap_or(subtitle.style);
            subtitles.events.push(subtitle);
        }
        if event.style.starts_with(b"danmakuFactory_stat")
            || event.style.starts_with(b"MSG")
            || !(event.event.eq_ignore_ascii_case(b"dialogue")
                || event.event.eq_ignore_ascii_case(b"comment"))
            || event.text.is_empty()
            || event.end < event.start
        {
            continue;
        }
        let Some(style) = input.styles.iter().find(|style| style.name == event.style) else {
            continue;
        };
        let (text, code) = split_codes(&event.text);
        let mut start_x = 0;
        let mut start_y = 0;
        let mut end_x = 0;
        let mut end_y = 0;
        let mut pause_time = 0;
        let mut move_time = 0;
        if let Some(args) = parenthesized(&code, b"\\pos", b",-.0123456789") {
            let mut cursor = ByteCursor::new(&args);
            start_x = parse_i32(cursor.left_part(b',', 128));
            start_y = parse_i32(args.split_once_byte(b',').map_or(&[][..], |(_, rest)| rest));
            end_x = start_x;
            end_y = start_y;
        }
        if let Some(args) = parenthesized(&code, b"\\move", b",-.0123456789") {
            let mut cursor = ByteCursor::new(&args);
            start_x = integer(&mut cursor);
            start_y = integer(&mut cursor);
            end_x = integer(&mut cursor);
            let parts = args.split(|&byte| byte == b',').collect::<Vec<_>>();
            end_y = parts.get(3).map_or(0, |part| parse_i32(part));
            if parts.len() > 4 {
                pause_time = parts.get(4).map_or(0, |part| parse_i32(part));
                move_time = parts
                    .get(5)
                    .map_or(0, |part| parse_i32(part))
                    .saturating_sub(pause_time);
            }
        }
        let font_size = parameter(&code, b"\\fs", Some(b"0123456789"), 127)
            .map_or(0, |value| parse_i32(&value));
        let font_name = parameter(&code, b"\\fn", None, 127).unwrap_or_default();
        let font_color =
            parameter(&code, b"\\c", Some(b"0123456789abcdefABCDEF"), 6).map(|value| {
                let mut padded = vec![b'0'; 6 - value.len()];
                padded.extend(value);
                to_dec_color(&padded)
            });
        let fr_y = parameter(&code, b"\\fry", Some(b"-0123456789"), 127)
            .map_or(0, |value| parse_i32(&value));
        let fr_z = parameter(&code, b"\\frz", Some(b"-0123456789"), 127)
            .map_or(0, |value| parse_i32(&value));
        let mut fade_start = 0;
        let mut fade_end = 0;
        if let Some(args) = parenthesized(&code, b"\\fade", b",-0123456789") {
            let mut cursor = ByteCursor::new(&args);
            fade_start = integer(&mut cursor);
            cursor.left_part(b',', 128);
            fade_end = integer(&mut cursor);
        }
        let type_ = match event.style.as_slice() {
            b"R2L" => R2L,
            b"L2R" => L2R,
            b"TOP" => TOP,
            b"BTM" => BOTTOM,
            b"SP" => SPECIAL,
            _ if fade_start == 0
                && fade_end == 0
                && fr_y == 0
                && fr_z == 0
                && find_bytes(&code, b"\\alpha").is_none()
                && font_name.is_empty() =>
            {
                if start_x == end_x
                    && start_y == end_y
                    && (i64::from(start_x) - i64::from(input.play_res_x / 2)).abs() < 2
                {
                    if start_y < input.play_res_y / 2 {
                        TOP
                    } else {
                        BOTTOM
                    }
                } else if start_y == end_y && start_x > input.play_res_x && end_x < 0 {
                    R2L
                } else if start_y == end_y && end_x > input.play_res_x && start_x < 0 {
                    L2R
                } else {
                    SPECIAL
                }
            }
            _ => SPECIAL,
        };
        let mut seconds = event.start as f32 / 1000.0 + time_shift;
        if seconds < EPS as f32 {
            seconds = 0.0;
        }
        let special = (type_ == SPECIAL).then(|| {
            Box::new(SpecialPart {
                exist_time: event.end.saturating_sub(event.start),
                move_time,
                pause_time,
                start_x: (start_x as f32 / input.play_res_x as f32).abs(),
                start_y: (start_y as f32 / input.play_res_y as f32).abs(),
                end_x: (end_x as f32 / input.play_res_x as f32).abs(),
                end_y: (end_y as f32 / input.play_res_y as f32).abs(),
                fade_start,
                fade_end,
                fr_y,
                fr_z,
                font_name,
            })
        });
        pool.push(Danmaku {
            time: (seconds * 1000.0 + 0.5) as i32,
            type_,
            font_size: if font_size == 0 {
                25
            } else {
                font_size.wrapping_sub(style.font_size).wrapping_add(25) as i16
            },
            color: font_color.unwrap_or_else(|| {
                if style.primary_colour.len() < 10 {
                    0xFFFFFF
                } else {
                    to_dec_color(&style.primary_colour[4..])
                }
            }),
            text: ass_unescape(&text, ASS_MAX_LINE_LEN),
            text_is_null: false,
            special,
            user: None,
            gift: None,
        });
    }
    Ok(subtitles)
}

pub fn read_ass(
    path: &Path,
    pool: &mut Vec<Danmaku>,
    append: bool,
    time_shift: f32,
) -> Result<AssFile, CoreError> {
    ass_file_to_danmaku(&read_ass_file(path)?, pool, append, time_shift)
}

/// Explicitly releases allocated tables; normal Rust ownership does this on drop.
pub fn free_ass_file(file: &mut AssFile) {
    file.styles = Vec::new();
    file.events = Vec::new();
}

/// Writes each full 23-field style with the original number formatting.
pub fn write_ass_styles_part<W: Write>(writer: &mut W, styles: &[AssStyle]) -> io::Result<()> {
    for style in styles {
        writer.write_all(b"\nStyle: ")?;
        write_field(writer, &style.name)?;
        write_field(writer, &style.font_name)?;
        write!(writer, "{},", style.font_size)?;
        for colour in [
            &style.primary_colour,
            &style.secondary_colour,
            &style.outline_colour,
            &style.back_colour,
        ] {
            write_field(writer, colour)?;
        }
        write!(
            writer,
            "{},{},{},{},{:.2},{:.2},{:.2},{:.2},{},{},{},{},{},{},{},{}",
            style.bold,
            style.italic,
            style.underline,
            style.strikeout,
            style.scale_x as f32,
            style.scale_y as f32,
            style.spacing,
            style.angle,
            style.border_style,
            style.outline as i32,
            style.shadow as i32,
            style.alignment,
            style.margin_l,
            style.margin_r,
            style.margin_v,
            style.encoding
        )?;
    }
    Ok(())
}

fn write_field<W: Write>(writer: &mut W, field: &[u8]) -> io::Result<()> {
    writer.write_all(nul_prefix(field))?;
    writer.write_all(b",")
}

/// Original writeXml output: normal and special nodes, including blocked nodes.
/// Message and user metadata were never emitted by this upstream API.
pub fn write_xml(path: &Path, pool: &[Danmaku]) -> Result<usize, CoreError> {
    if pool.is_empty() {
        return Err(CoreError::new(
            DF_ERR_NO_DANMAKU,
            1,
            0,
            "empty danmaku pool",
        ));
    }
    validate_xml_pool(pool)?;
    let file = File::create(path).map_err(|error| {
        CoreError::io(
            DF_ERR_OUTPUT_CREATE,
            2,
            &error,
            "failed to create XML output",
        )
    })?;
    let mut writer = crate::ass::TextOutput(BufWriter::new(file));
    let result = write_xml_to(&mut writer, pool).and_then(|count| {
        crate::ass::finish_file(writer)?;
        Ok(count)
    });
    result.map_err(|error| {
        CoreError::io(DF_ERR_OUTPUT_WRITE, 3, &error, "failed to write XML output")
    })
}

fn validate_xml_pool(pool: &[Danmaku]) -> Result<(), CoreError> {
    if pool
        .iter()
        .any(|node| node.is_special() && node.special.is_none())
    {
        return Err(CoreError::new(
            DF_ERR_UNSUPPORTED_INPUT,
            0,
            0,
            "special danmaku missing data",
        ));
    }
    Ok(())
}

pub fn write_xml_to<W: Write>(writer: &mut W, pool: &[Danmaku]) -> io::Result<usize> {
    if pool.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "empty danmaku pool",
        ));
    }
    validate_xml_pool(pool)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.detail))?;
    writer.write_all(b"<?xml version=\"1.0\"?>\n<i>\n    <chatserver></chatserver>\n    <chatid></chatid>\n    <mission></mission>\n    <maxlimit></maxlimit>\n    <state></state>\n    <real_name></real_name>\n    <source></source>")?;
    let mut count = 0;
    for node in pool {
        let type_ = match node.type_.unsigned_abs() as i16 {
            R2L => 1,
            L2R => 6,
            TOP => 5,
            BOTTOM => 4,
            SPECIAL => 7,
            _ => continue,
        };
        write!(
            writer,
            "\n    <d p=\"{},{},{},{},0,0,NULL,0\">",
            int_time_to_str(node.time, 3),
            type_,
            node.font_size,
            node.color
        )?;
        if let Some(special) = node.special.as_ref().filter(|_| node.is_special()) {
            write!(
                writer,
                "[{},{},\"{}-{}\",{},\"",
                float_to_str(f64::from(special.start_x), 2),
                float_to_str(f64::from(special.start_y), 2),
                float_to_str(1.0 - f64::from(special.fade_start) / 255.0, 2),
                float_to_str(1.0 - f64::from(special.fade_end) / 255.0, 2),
                int_time_to_str(special.exist_time, 1)
            )?;
            writer.write_all(nul_prefix(&node.text))?;
            write!(
                writer,
                "\",{},{},{},{},{},{},true,\"",
                special.fr_z,
                special.fr_y,
                float_to_str(f64::from(special.end_x), 2),
                float_to_str(f64::from(special.end_y), 2),
                special.move_time,
                special.pause_time
            )?;
            writer.write_all(nul_prefix(&special.font_name))?;
            writer.write_all(b"\",1]")?;
        } else {
            writer.write_all(nul_prefix(&node.text))?;
        }
        writer.write_all(b"</d>")?;
        count += 1;
    }
    writer.write_all(b"\n</i>")?;
    Ok(count)
}

fn find_bytes(text: &[u8], needle: &[u8]) -> Option<usize> {
    text.windows(needle.len()).position(|bytes| bytes == needle)
}

fn split_codes(text: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut output = Vec::new();
    let mut code = Vec::new();
    let mut at = 0;
    while at < text.len() {
        if text[at] == b'{' {
            if let Some(length) = text[at..].iter().position(|&byte| byte == b'}') {
                code.extend_from_slice(&text[at + 1..at + length]);
                at += length + 1;
                continue;
            }
        }
        output.push(text[at]);
        at += 1;
    }
    (output, code)
}

fn parameter(code: &[u8], name: &[u8], allowed: Option<&[u8]>, capacity: usize) -> Option<Vec<u8>> {
    let start = find_bytes(code, name)? + name.len();
    let mut output = Vec::new();
    for &byte in code[start..].iter().take_while(|&&byte| byte != b'\\') {
        if allowed.is_none_or(|allowed| allowed.contains(&byte)) {
            if output.len() == capacity {
                break;
            }
            output.push(byte);
        }
    }
    Some(output)
}

fn parenthesized(code: &[u8], name: &[u8], allowed: &[u8]) -> Option<Vec<u8>> {
    let mut at = find_bytes(code, name)? + name.len();
    while at < code.len() && !matches!(code[at], b'(' | b'\\') {
        at += 1;
    }
    if code.get(at) != Some(&b'(') {
        return None;
    }
    Some(
        code[at + 1..]
            .iter()
            .copied()
            .take_while(|&byte| !matches!(byte, b')' | b'\\'))
            .filter(|byte| allowed.contains(byte))
            .take(127)
            .collect(),
    )
}

trait SplitOnceByte {
    fn split_once_byte(&self, byte: u8) -> Option<(&[u8], &[u8])>;
}
impl SplitOnceByte for [u8] {
    fn split_once_byte(&self, byte: u8) -> Option<(&[u8], &[u8])> {
        let at = self.iter().position(|&item| item == byte)?;
        Some((&self[..at], &self[at + 1..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TemporaryFile(std::path::PathBuf);

    impl TemporaryFile {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(std::env::temp_dir().join(format!(
                "danmaku-formats-{}-{nonce}-{}.xml",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )))
        }
    }

    impl Drop for TemporaryFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn source(style: &str, text: &str) -> AssFile {
        parse_ass(format!("Title: test\nScriptType: v4.00+\nCollisions: Normal\nPlayResX: 1920\nPlayResY: 1080\nTimer: 100\nStyle: {style},Arial,38,&H00FFFFFF,&H00FFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,1,1,8,0,0,0,1\nDialogue: 0,0:00:01.25,0:00:06.25,{style},USER,0000,0000,0000,,{text}\n").as_bytes())
    }

    #[test]
    fn read_full_ass_fields_and_commas_in_text() {
        let file = source("R2L", "{\\move(1921,40,-100,40)}hello, world");
        assert_eq!(file.title, b"test");
        assert_eq!(file.timer, 100.0);
        assert_eq!(file.styles.len(), 1);
        assert_eq!(file.styles[0].font_size, 38);
        assert_eq!(file.styles[0].encoding, 1);
        assert_eq!(file.events[0].start, 1250);
        assert_eq!(
            file.events[0].text,
            b"{\\move(1921,40,-100,40)}hello, world"
        );
        let mut pool = Vec::new();
        ass_file_to_danmaku(&file, &mut pool, false, -0.25).unwrap();
        assert_eq!(pool[0].time, 1000);
        assert_eq!(pool[0].type_, R2L);
        assert_eq!(pool[0].text, b"hello, world");
    }

    #[test]
    fn styles_write_roundtrip_preserves_all_supported_fields() {
        let mut style = source("Default", "x").styles.remove(0);
        style.bold = -1;
        style.italic = -1;
        style.underline = -1;
        style.strikeout = -1;
        style.scale_x = 125;
        style.scale_y = 90;
        style.spacing = 2.25;
        style.angle = -15.5;
        style.border_style = 3;
        style.outline = 2.75;
        style.shadow = 1.75;
        style.alignment = 9;
        style.margin_l = 12;
        style.margin_r = 34;
        style.margin_v = 56;
        style.encoding = 134;
        let mut bytes = Vec::new();
        write_ass_styles_part(&mut bytes, &[style.clone()]).unwrap();
        let mut expected = style;
        expected.outline = 2.0;
        expected.shadow = 1.0;
        assert_eq!(parse_ass(&bytes).styles, [expected]);
    }

    #[test]
    fn detect_geometry_and_special_attributes_without_style_names() {
        let file = source(
            "Default",
            "{\\move(100,200,400,500,100,1000)\\fs48\\fnFont\\c&H563412&\\fade(10,20,30,0,0,0,0)\\fry-15\\frz20}a\\hb\\Nc",
        );
        let mut pool = Vec::new();
        ass_file_to_danmaku(&file, &mut pool, false, 0.0).unwrap();
        let node = &pool[0];
        assert_eq!(node.type_, SPECIAL);
        assert_eq!(node.font_size, 35);
        assert_eq!(node.color, 0x123456);
        assert_eq!(node.text, b"a b\nc");
        let special = node.special.as_ref().unwrap();
        assert_eq!(special.exist_time, 5000);
        assert_eq!((special.move_time, special.pause_time), (900, 100));
        assert_eq!(
            (
                special.fade_start,
                special.fade_end,
                special.fr_y,
                special.fr_z
            ),
            (10, 30, -15, 20)
        );
        assert_eq!(special.font_name, b"Font");
        for (text, kind) in [
            ("{\\pos(960,50)}fixed", TOP),
            ("{\\pos(960,1000)}fixed", BOTTOM),
            ("{\\move(2000,30,-100,30)}roll", R2L),
            ("{\\move(-100,30,2000,30)}roll", L2R),
        ] {
            ass_file_to_danmaku(&source("Default", text), &mut pool, false, 0.0).unwrap();
            assert_eq!(pool[0].type_, kind);
        }
    }

    #[test]
    fn comments_and_external_subtitles_are_owned_and_releasable() {
        let mut file = source(
            "danmakuFactory_ext_sub00_Default",
            "{\\pos(960,50)}subtitle",
        );
        file.events[0].event = b"Comment".to_vec();
        let mut pool = vec![Danmaku::default()];
        let mut subtitles = ass_file_to_danmaku(&file, &mut pool, true, 0.0).unwrap();
        assert_eq!(pool.len(), 2);
        assert_eq!(subtitles.styles[0].name, b"Default");
        assert_eq!(subtitles.events[0].style, b"Default");
        assert_eq!(subtitles.events[0].name, b"user");
        file.events[0].text.clear();
        assert!(!subtitles.events[0].text.is_empty());
        free_ass_file(&mut subtitles);
        free_ass_file(&mut subtitles);
        assert!(subtitles.styles.is_empty() && subtitles.events.is_empty());
        assert!(
            parse_ass(b"Comment: 0,0:00:00.00,0:00:01.00,X,,0,0,0,,text")
                .events
                .is_empty()
        );
    }

    #[test]
    fn malformed_and_oversized_tags_are_memory_safe() {
        let mut file = source("SP", "{\\fnVery long font name}text{unterminated");
        file.play_res_x = 0;
        file.play_res_y = 0;
        file.events[0].text = [b"{\\fn".as_slice(), &vec![b'X'; 9000], b"}x{unterminated"].concat();
        let mut pool = Vec::new();
        ass_file_to_danmaku(&file, &mut pool, false, 0.0).unwrap();
        assert_eq!(pool[0].special.as_ref().unwrap().font_name.len(), 127);
        assert_eq!(pool[0].text, b"x{unterminated");
        assert_eq!(time_to_int(b"1:02:03.4567"), 3723457);
        assert_eq!(time_to_int(b""), 0);
        assert_eq!(time_to_int(b"0:00:00.9999"), 1000);
        assert!(parse_ass(&vec![b'x'; 20000]).events.is_empty());
    }

    #[test]
    fn xml_writer_supports_blocked_and_special_nodes_and_propagates_errors() {
        let normal = Danmaku {
            time: 1234,
            type_: -TOP,
            font_size: 25,
            color: 0xFFFFFF,
            text: b"hello".to_vec(),
            ..Default::default()
        };
        let special = Danmaku {
            type_: SPECIAL,
            text: b"special".to_vec(),
            special: Some(Box::new(SpecialPart {
                exist_time: 5000,
                move_time: 800,
                pause_time: 200,
                start_x: 0.25,
                start_y: 0.5,
                end_x: 0.75,
                end_y: 0.5,
                fade_start: 0,
                fade_end: 255,
                font_name: b"Arial".to_vec(),
                ..Default::default()
            })),
            ..normal.clone()
        };
        let mut bytes = Vec::new();
        assert_eq!(write_xml_to(&mut bytes, &[normal, special]).unwrap(), 2);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("<d p=\"1.234,5,25,16777215,0,0,NULL,0\">hello</d>"));
        assert!(
            text.contains("[0.25,0.5,\"1-0\",5,\"special\",0,0,0.75,0.5,800,200,true,\"Arial\",1]")
        );
        struct Fail;
        impl Write for Fail {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::WriteZero.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        assert!(write_xml_to(&mut Fail, &[Danmaku::default()]).is_err());
        assert!(
            write_xml_to(
                &mut Vec::new(),
                &[Danmaku {
                    type_: SPECIAL,
                    ..Default::default()
                }]
            )
            .is_err()
        );
    }

    #[test]
    fn xml_normal_and_special_roundtrip_restores_format_fields() {
        let normal = Danmaku {
            time: 1234,
            type_: R2L,
            font_size: 30,
            color: 0x123456,
            text: "normal 汉字".as_bytes().to_vec(),
            ..Default::default()
        };
        let special = Danmaku {
            type_: SPECIAL,
            text: b"special".to_vec(),
            special: Some(Box::new(SpecialPart {
                exist_time: 5000,
                move_time: 800,
                pause_time: 200,
                start_x: 0.25,
                start_y: 0.5,
                end_x: 0.75,
                end_y: 0.5,
                fade_start: 0,
                fade_end: 255,
                fr_z: 20,
                fr_y: -10,
                font_name: b"Arial".to_vec(),
                ..Default::default()
            })),
            ..normal.clone()
        };
        let pool = [normal, special];
        let file = TemporaryFile::new();
        assert_eq!(write_xml(&file.0, &pool).unwrap(), 2);
        assert_eq!(crate::xml::read_xml(&file.0).unwrap(), pool);
    }

    #[test]
    fn failed_ass_read_does_not_modify_existing_pool() {
        let file = TemporaryFile::new();
        let original = vec![Danmaku {
            time: 42,
            text: b"existing".to_vec(),
            ..Default::default()
        }];
        let mut pool = original.clone();
        assert_eq!(
            read_ass(&file.0, &mut pool, false, 0.0).unwrap_err().status,
            DF_ERR_INPUT_OPEN
        );
        assert_eq!(pool, original);
    }
}
