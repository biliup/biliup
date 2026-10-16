//! Byte-oriented text operations used by the DanmakuFactory conversion path.
//!
//! The input and ASS buffers historically have a 1023-byte payload limit.
//! Keeping bytes also preserves inputs truncated in the middle of UTF-8.

use crate::model::MAX_TEXT_LENGTH;

pub(crate) fn nul_prefix(text: &[u8]) -> &[u8] {
    &text[..text
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(text.len())]
}

pub(crate) fn is_blank(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\t' | b'\r')
}

pub fn trim(text: &[u8]) -> &[u8] {
    let text = nul_prefix(text);
    let start = text
        .iter()
        .position(|&byte| !is_blank(byte))
        .unwrap_or(text.len());
    let end = text
        .iter()
        .rposition(|&byte| !is_blank(byte))
        .map_or(start, |at| at + 1);
    &text[start..end]
}

pub fn dequote(text: &[u8]) -> &[u8] {
    let mut text = trim(text);
    if matches!(text.first(), Some(b'\'' | b'"')) {
        text = &text[1..];
    }
    if matches!(text.last(), Some(b'\'' | b'"')) {
        text = &text[..text.len() - 1];
    }
    text
}

/// The old delimiter helper drops the last field if it has no delimiter.
/// Bounds and cursor movement are retained without ever indexing past EOF.
pub struct ByteCursor<'a> {
    text: &'a [u8],
    at: usize,
}

impl<'a> ByteCursor<'a> {
    pub fn new(text: &'a [u8]) -> Self {
        Self {
            text: nul_prefix(text),
            at: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.at >= self.text.len()
    }

    pub fn remaining(&self) -> &'a [u8] {
        &self.text[self.at..]
    }

    pub fn left_part(&mut self, delimiter: u8, capacity: usize) -> &'a [u8] {
        let start = self.at;
        let limit = start
            .saturating_add(capacity.saturating_sub(1))
            .min(self.text.len());
        while self.at < limit && self.text[self.at] != delimiter {
            self.at += 1;
        }
        let part = if self.at == self.text.len() {
            &self.text[self.at..self.at]
        } else {
            &self.text[start..self.at]
        };
        if self.at < self.text.len() {
            self.at += 1;
        }
        part
    }

    pub fn word(&mut self, capacity: usize) -> &'a [u8] {
        self.next_word(capacity, b' ', true)
    }

    pub fn next_word(&mut self, capacity: usize, end_before: u8, move_cursor: bool) -> &'a [u8] {
        if capacity < 2 {
            return &self.text[self.at..self.at];
        }
        let mut at = self.at;
        while at < self.text.len() && is_blank(self.text[at]) {
            at += 1;
        }
        let start = at;
        let limit = start
            .saturating_add(capacity.saturating_sub(1))
            .min(self.text.len());
        while at < limit {
            at += 1;
            if at < self.text.len() && (self.text[at] == end_before || is_blank(self.text[at])) {
                break;
            }
        }
        if move_cursor {
            self.at = at;
        }
        &self.text[start..at]
    }
}

/// XML entities decoded by the upstream reader; numeric entities other than
/// `&#34;` are deliberately left for the ASS text conversion stage.
pub fn xml_unescape(text: &[u8]) -> Vec<u8> {
    const ENTITIES: &[(&[u8], u8)] = &[
        (b"&amp;", b'&'),
        (b"&apos;", b'\''),
        (b"&gt;", b'>'),
        (b"&lt;", b'<'),
        (b"&quot;", b'"'),
        (b"&#34;", b'"'),
    ];
    let text = nul_prefix(text);
    let mut output = Vec::with_capacity(text.len());
    let mut at = 0;
    while at < text.len() {
        if text[at] == b'&' {
            if let Some(&(entity, value)) = ENTITIES
                .iter()
                .find(|(entity, _)| text[at..].starts_with(entity))
            {
                output.push(value);
                at += entity.len();
                continue;
            }
        }
        output.push(text[at]);
        at += 1;
    }
    output
}

/// Escapes source text in one pass, including the historical second XML
/// unescape. Replacement bytes are not processed again in this pass.
pub fn ass_escape(text: &[u8]) -> Vec<u8> {
    const ESCAPES: &[(&[u8], &[u8])] = &[
        (b"\\n", b"\\\xE2\x80\x8Bn"),
        (b"\\N", b"\\\xE2\x80\x8BN"),
        (b"\\h", b"\\\xE2\x80\x8Bh"),
        (b"{", b"\\{"),
        (b"}", b"\\}"),
        (b" ", b"\\h"),
        (b"\n", b"\\N"),
        (b"&lt;", b"<"),
        (b"&gt;", b">"),
        (b"&amp;", b"&"),
        (b"&apos;", b"'"),
        (b"&quot;", b"\""),
    ];
    let text = nul_prefix(text);
    let mut output = Vec::with_capacity(text.len().min(MAX_TEXT_LENGTH - 1));
    let mut at = 0;
    while at < text.len() && output.len() < MAX_TEXT_LENGTH - 1 {
        if let Some(&(source, replacement)) = ESCAPES
            .iter()
            .find(|(source, _)| text[at..].starts_with(source))
        {
            append_bounded(&mut output, replacement);
            at += source.len();
        } else if text[at..].starts_with(b"&#") {
            let mut cursor = ByteCursor::new(&text[at + 2..]);
            let digits = cursor.left_part(b';', 8);
            let codepoint = parse_i32(digits) as u32;
            if let Some(character) =
                char::from_u32(codepoint).filter(|&character| character != '\0')
            {
                let mut buffer = [0; 4];
                append_bounded(&mut output, character.encode_utf8(&mut buffer).as_bytes());
            }
            // A missing ';' historically removes only the prefix and one
            // following byte. A bare trailing '&#' removes exactly two.
            at = at.saturating_add(2 + digits.len() + usize::from(at + 2 < text.len()));
        } else {
            output.push(text[at]);
            at += 1;
        }
    }
    output
}

/// Reverses the upstream ASS substitutions, including its XML-escaped output.
pub fn ass_unescape(text: &[u8], capacity: usize) -> Vec<u8> {
    const ESCAPES: &[(&[u8], &[u8])] = &[
        (b"\\\xE2\x80\x8Bn", b"\\n"),
        (b"\\\xE2\x80\x8BN", b"\\N"),
        (b"\\\xE2\x80\x8Bh", b"\\h"),
        (b"\\{", b"{"),
        (b"\\}", b"}"),
        (b"\\h", b" "),
        (b"\\N", b"\n"),
        (b"<", b"&lt;"),
        (b">", b"&gt;"),
        (b"&", b"&amp;"),
        (b"'", b"&apos;"),
        (b"\"", b"&quot;"),
    ];
    let text = nul_prefix(text);
    let limit = capacity.saturating_sub(1);
    let mut output = Vec::with_capacity(text.len().min(limit));
    let mut at = 0;
    while at < text.len() && output.len() < limit {
        if let Some(&(source, replacement)) = ESCAPES
            .iter()
            .find(|(source, _)| text[at..].starts_with(source))
        {
            let count = replacement.len().min(limit - output.len());
            output.extend_from_slice(&replacement[..count]);
            at += source.len();
        } else {
            output.push(text[at]);
            at += 1;
        }
    }
    output
}

fn append_bounded(output: &mut Vec<u8>, text: &[u8]) {
    let available = (MAX_TEXT_LENGTH - 1).saturating_sub(output.len());
    output.extend_from_slice(&text[..text.len().min(available)]);
}

/// Upstream's UTF-8 detector accepts structural 2- through 6-byte sequences,
/// and counts all valid sequences equally, including combining characters.
pub fn is_utf8(text: &[u8]) -> bool {
    let text = nul_prefix(text);
    let mut at = 0;
    while at < text.len() {
        let first = text[at];
        let extra = match first {
            0..=0x7F => 0,
            0xC0..=0xDF => 1,
            0xE0..=0xEF => 2,
            0xF0..=0xF7 => 3,
            0xF8..=0xFB => 4,
            0xFC..=0xFD => 5,
            _ => return false,
        };
        if at + extra >= text.len()
            || text[at + 1..at + extra + 1]
                .iter()
                .any(|byte| byte & 0xC0 != 0x80)
        {
            return false;
        }
        at += extra + 1;
    }
    true
}

pub fn get_str_hei(_text: &[u8], font_size_set: i32, font_size_in_file: i16) -> i32 {
    font_size_set + i32::from(font_size_in_file) - 25
}

pub fn get_str_len(text: &[u8], font_size_set: i32, font_size_in_file: i16) -> i32 {
    let text = nul_prefix(text);
    let count = if is_utf8(text) {
        text.iter()
            .filter(|&&byte| byte >= 0xC0 || byte < 0x80)
            .count()
    } else {
        text.len()
    };
    (count as f64 * f64::from(get_str_hei(text, font_size_set, font_size_in_file)) / 1.2) as i32
}

fn strip_fraction_zeroes(mut text: String) -> String {
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

/// `%lf` first rounds to six decimal places; requested accuracy then truncates.
pub fn float_to_str(value: f64, accuracy: usize) -> String {
    if value.is_nan() {
        return if value.is_sign_negative() {
            "-nan"
        } else {
            "nan"
        }
        .into();
    }
    let mut text = format!("{value:.6}");
    if let Some(dot) = text.find('.') {
        text.truncate(
            dot.saturating_add(1)
                .saturating_add(accuracy)
                .min(text.len()),
        );
    }
    strip_fraction_zeroes(text)
}

pub fn int_time_to_str(milliseconds: i32, accuracy: usize) -> String {
    let seconds = milliseconds / 1000;
    let fraction = milliseconds - seconds * 1000;
    if fraction == 0 {
        return seconds.to_string();
    }
    let mut text = format!("{seconds}.{fraction:03}");
    let dot = text.find('.').unwrap();
    text.truncate(
        dot.saturating_add(1)
            .saturating_add(accuracy)
            .min(text.len()),
    );
    strip_fraction_zeroes(text)
}

/// The C writer uses printf's default `%g` precision (six significant digits).
pub fn general_float(value: f64) -> String {
    if value.is_nan() {
        return "nan".into();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf"
        } else {
            "inf"
        }
        .into();
    }
    let scientific = format!("{value:.5e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap();
    let exponent: i32 = exponent.parse().unwrap();
    if !(-4..6).contains(&exponent) {
        format!("{}e{exponent:+03}", strip_fraction_zeroes(mantissa.into()))
    } else {
        strip_fraction_zeroes(format!(
            "{value:.precision$}",
            precision = (5 - exponent) as usize
        ))
    }
}

pub fn to_hex_color(mut color: i32) -> String {
    let mut output = [0; 6];
    for channel in 0..3 {
        for nibble in (0..2).rev() {
            let value = color % 16;
            output[channel * 2 + nibble] = if value < 10 {
                (value + i32::from(b'0')) as u8
            } else {
                (value - 10 + i32::from(b'A')) as u8
            };
            color /= 16;
        }
    }
    String::from_utf8(output.to_vec()).unwrap()
}

pub fn to_hex_opacity(mut opacity: i32) -> String {
    let mut output = [0; 2];
    for nibble in (0..2).rev() {
        let value = opacity % 16;
        output[nibble] = if value < 10 {
            (value + i32::from(b'0')) as u8
        } else {
            (value - 10 + i32::from(b'A')) as u8
        };
        opacity /= 16;
    }
    String::from_utf8(output.to_vec()).unwrap()
}

pub fn to_dec_color(text: &[u8]) -> i32 {
    let mut color = 0_i32;
    for channel in (0..3).rev() {
        for nibble in 0..2 {
            if let Some(value) = text
                .get(channel * 2 + nibble)
                .and_then(|&byte| hex_digit(byte))
            {
                color = color * 16 + i32::from(value);
            }
        }
    }
    color
}

/// Removes the `danmakuFactory_ext_subNN_` namespace from imported styles.
pub fn de_style_name_prefix(text: &[u8]) -> Option<Vec<u8>> {
    let text = nul_prefix(text);
    if text.len() < 22 {
        return None;
    }
    let mut at = 21;
    while at < text.len() && text[at - 1] != b'_' {
        at += 1;
    }
    Some(text[at..].to_vec())
}

/// Parses ASS hours, minutes, seconds and up to three fractional digits.
pub fn time_to_int(text: &[u8]) -> i32 {
    let text = nul_prefix(text);
    let mut at = 0;
    let mut part = || {
        let mut value = 0_i32;
        while at < text.len() && !matches!(text[at], b':' | b'.') {
            if text[at] != b' ' {
                value = value
                    .wrapping_mul(10)
                    .wrapping_add(i32::from(text[at]) - i32::from(b'0'));
            }
            at += 1;
        }
        at = at.saturating_add(1);
        value
    };
    let hours = part();
    let minutes = part();
    let seconds = part();
    let mut time = hours
        .wrapping_mul(3600)
        .wrapping_add(minutes.wrapping_mul(60))
        .wrapping_add(seconds);
    let mut digits = 0;
    while at < text.len() && !matches!(text[at], b':' | b'.') {
        let byte = text[at];
        at += 1;
        if byte == b' ' {
            continue;
        }
        digits += 1;
        if digits > 3 {
            if byte >= b'5' {
                time = time.wrapping_add(1);
            }
            break;
        }
        time = time
            .wrapping_mul(10)
            .wrapping_add(i32::from(byte) - i32::from(b'0'));
    }
    for _ in digits..3 {
        time = time.wrapping_mul(10);
    }
    time
}

pub fn to_lower(text: &[u8]) -> Vec<u8> {
    nul_prefix(text)
        .iter()
        .map(u8::to_ascii_lowercase)
        .collect()
}

pub fn last_chr(text: &[u8], byte: u8) -> i32 {
    let text = nul_prefix(text);
    if text.len() > MAX_TEXT_LENGTH {
        return -2;
    }
    if byte == 0 {
        return text.len() as i32;
    }
    text.iter()
        .rposition(|&value| value == byte)
        .map_or(-1, |at| at as i32)
}

pub fn is_designated_char(byte: u8, designated: &[u8]) -> bool {
    nul_prefix(designated).contains(&byte)
}

pub fn is_designated_str(text: &[u8], designated: &[u8]) -> bool {
    let text = nul_prefix(text);
    let designated = nul_prefix(designated);
    // Preserve the original pair-counting behavior for duplicate designated
    // bytes, rather than changing this into a set-membership operation.
    text.iter()
        .map(|value| {
            designated
                .iter()
                .filter(|candidate| *candidate == value)
                .count()
        })
        .sum::<usize>()
        == text.len()
}

pub fn str_safe_copy(source: &[u8], capacity: usize) -> Vec<u8> {
    let source = nul_prefix(source);
    source[..source.len().min(capacity.saturating_sub(1))].to_vec()
}

/// Returns the upstream counter, which includes the byte that hit capacity.
pub fn str_safe_cat(destination: &mut Vec<u8>, capacity: usize, source: &[u8]) -> usize {
    if capacity == 0 {
        return 0;
    }
    destination.truncate(nul_prefix(destination).len());
    if destination.len() >= capacity {
        destination.truncate(capacity - 1);
        return capacity;
    }
    let source = nul_prefix(source);
    let available = capacity - 1 - destination.len();
    let count = source.len().min(available);
    destination.extend_from_slice(&source[..count]);
    destination.len() + usize::from(source.len() > available)
}

pub fn match_count(text: &[u8], pattern: &[u8]) -> i32 {
    let text = nul_prefix(text);
    let pattern = nul_prefix(pattern);
    if pattern.is_empty() {
        return -1;
    }
    let mut count = 0;
    let mut at = 0;
    while at < text.len() {
        if text[at..].starts_with(pattern) {
            count += 1;
            at += pattern.len();
        } else {
            at += 1;
        }
    }
    count
}

/// The old replacement helper's capacity is the payload limit, unlike the
/// copy and ASS helpers whose capacities include a terminating NUL.
pub fn str_replace(
    text: &[u8],
    pattern: &[u8],
    replacement: &[u8],
    capacity: usize,
) -> Option<Vec<u8>> {
    let text = nul_prefix(text);
    let pattern = nul_prefix(pattern);
    let replacement = nul_prefix(replacement);
    if pattern.is_empty() {
        return None;
    }
    let mut output = Vec::with_capacity(text.len().min(capacity));
    let mut at = 0;
    while at < text.len() && output.len() < capacity {
        if text[at..].starts_with(pattern) {
            let count = replacement.len().min(capacity - output.len());
            output.extend_from_slice(&replacement[..count]);
            at += pattern.len();
        } else {
            output.push(text[at]);
            at += 1;
        }
    }
    Some(output)
}

pub fn is_start_with(text: &[u8], prefix: &[u8]) -> bool {
    nul_prefix(text).starts_with(nul_prefix(prefix))
}

pub fn filename_get_format(path: &[u8], capacity: usize) -> Vec<u8> {
    let path = nul_prefix(path);
    match path.iter().rposition(|&byte| byte == b'.') {
        Some(at) => str_safe_copy(&path[at + 1..], capacity),
        None => Vec::new(),
    }
}

pub fn filename_get_path(path: &[u8], capacity: usize) -> Vec<u8> {
    let path = nul_prefix(path);
    match path
        .iter()
        .rposition(|&byte| matches!(byte, b'/' | b'\\'))
        .filter(|&at| at > 0)
    {
        Some(at) => str_safe_copy(&path[..at + 1], capacity),
        None => Vec::new(),
    }
}

pub fn str_to_bool(text: &[u8]) -> bool {
    trim(&to_lower(text)) != b"false"
}

pub fn bool_to_str(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub fn unicode_to_utf8(codepoint: u32) -> Vec<u8> {
    let Some(character) = char::from_u32(codepoint) else {
        return Vec::new();
    };
    if character == '\0' {
        return Vec::new();
    }
    let mut buffer = [0; 4];
    character.encode_utf8(&mut buffer).as_bytes().to_vec()
}

fn numeric_start(text: &[u8]) -> (&[u8], bool) {
    let text = nul_prefix(text);
    let start = text
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(text.len());
    let mut text = &text[start..];
    let negative = text.first() == Some(&b'-');
    if matches!(text.first(), Some(b'+' | b'-')) {
        text = &text[1..];
    }
    (text, negative)
}

pub(crate) fn parse_i32(text: &[u8]) -> i32 {
    let (text, negative) = numeric_start(text);
    let mut value = 0u64;
    for &byte in text.iter().take_while(|byte| byte.is_ascii_digit()) {
        value = value
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'));
    }
    let signed = if negative {
        (value.min(i64::MAX as u64 + 1) as i64).wrapping_neg()
    } else {
        value.min(i64::MAX as u64) as i64
    };
    signed as i32
}

pub(crate) fn parse_u64(text: &[u8]) -> u64 {
    let (text, negative) = numeric_start(text);
    let mut value = 0u64;
    let mut overflow = false;
    for &byte in text.iter().take_while(|byte| byte.is_ascii_digit()) {
        match value
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(byte - b'0')))
        {
            Some(next) => value = next,
            None => {
                value = u64::MAX;
                overflow = true;
            }
        }
    }
    if negative && !overflow {
        value.wrapping_neg()
    } else {
        value
    }
}

/// Prefix parsing matches atof rather than requiring the entire attribute to
/// be a Rust numeric literal. Hexadecimal floating-point input is accepted too.
pub(crate) fn parse_f64(text: &[u8]) -> f64 {
    let (text, negative) = numeric_start(text);
    let sign = if negative { -1.0 } else { 1.0 };
    if text
        .get(..3)
        .is_some_and(|text| text.eq_ignore_ascii_case(b"nan"))
    {
        return f64::NAN;
    }
    if text
        .get(..3)
        .is_some_and(|text| text.eq_ignore_ascii_case(b"inf"))
    {
        return sign * f64::INFINITY;
    }
    if text.starts_with(b"0x") || text.starts_with(b"0X") {
        let mut at = 2;
        let mut value = 0.0;
        let mut digits = 0;
        while let Some(digit) = text.get(at).and_then(|&byte| hex_digit(byte)) {
            value = value * 16.0 + f64::from(digit);
            at += 1;
            digits += 1;
        }
        if text.get(at) == Some(&b'.') {
            at += 1;
            let mut place = 1.0 / 16.0;
            while let Some(digit) = text.get(at).and_then(|&byte| hex_digit(byte)) {
                value += f64::from(digit) * place;
                place /= 16.0;
                at += 1;
                digits += 1;
            }
        }
        if digits == 0 {
            return 0.0;
        }
        if matches!(text.get(at), Some(b'p' | b'P')) {
            let exponent = parse_i32(&text[at + 1..]);
            value *= 2.0_f64.powi(exponent);
        }
        return sign * value;
    }
    let mut at = 0;
    let mut digits = 0;
    while text.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
        digits += 1;
    }
    if text.get(at) == Some(&b'.') {
        at += 1;
        while text.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return 0.0;
    }
    if matches!(text.get(at), Some(b'e' | b'E')) {
        let mut end = at + 1;
        if matches!(text.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let start = end;
        while text.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if end > start {
            at = end;
        }
    }
    sign * std::str::from_utf8(&text[..at])
        .unwrap()
        .parse::<f64>()
        .unwrap_or(0.0)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_and_ass_escaping_remain_separate_stages() {
        let decoded = xml_unescape(b"&amp;amp; &#32; &#34; &#x41; &unknown;");
        assert_eq!(decoded, b"&amp; &#32; \" &#x41; &unknown;");
        assert_eq!(ass_escape(&decoded), b"&\\h \\h\"\\h\\h&unknown;");
        assert_eq!(
            ass_escape(b"\\n \\N \\h {x}\n"),
            b"\\\xE2\x80\x8Bn\\h\\\xE2\x80\x8BN\\h\\\xE2\x80\x8Bh\\h\\{x\\}\\N"
        );
    }

    #[test]
    fn numeric_entity_boundaries_do_not_read_past_the_source() {
        assert_eq!(ass_escape(b"x&#"), b"x");
        assert_eq!(
            ass_escape(b"&#55357; &#1114112; &#128514;"),
            "\\h\\h😂".as_bytes()
        );
        assert_eq!(ass_escape(b"&#123"), b"23");
        assert_eq!(ass_escape(b"&#0;end"), b"end");
    }

    #[test]
    fn byte_truncation_and_legacy_width_are_preserved() {
        assert_eq!(ass_escape(&vec![b' '; 600]).len(), 1023);
        assert_eq!(get_str_len("a中😂e\u{301}".as_bytes(), 38, 25), 158);
        assert_eq!(get_str_len(&[0xF8, 0x88, 0x80, 0x80, 0x80], 38, 25), 31);
        assert_eq!(get_str_len(&[0xE4, 0xB8], 38, 25), 63);
        assert_eq!(get_str_len(b"before\0after", 38, 25), 190);
    }

    #[test]
    fn number_formatting_uses_upstream_precision_and_color_order() {
        assert_eq!(float_to_str(1.2399999, 2), "1.24");
        assert_eq!(float_to_str(1.239, 2), "1.23");
        assert_eq!(int_time_to_str(1234, 2), "1.23");
        assert_eq!(general_float(0.00009999999), "0.0001");
        assert_eq!(general_float(999999.9), "1e+06");
        assert_eq!(general_float(0.00001), "1e-05");
        assert_eq!(to_hex_color(0x123456), "563412");
        assert_eq!(to_hex_opacity(255), "FF");
    }

    #[test]
    fn numeric_prefixes_and_cursor_limits_match_c_helpers() {
        assert_eq!(parse_f64(b"  -1.2e2junk"), -120.0);
        assert_eq!(parse_f64(b"0x1.8p2junk"), 6.0);
        assert_eq!(parse_f64(b"2e+not-an-exponent"), 2.0);
        assert_eq!(parse_i32(b" +123abc"), 123);
        assert_eq!(parse_u64(b"-1"), u64::MAX);
        let mut cursor = ByteCursor::new(b"field,tail");
        assert_eq!(cursor.left_part(b',', 1024), b"field");
        assert_eq!(cursor.left_part(b',', 1024), b"");
        let mut cursor = ByteCursor::new(b"abcdef,tail");
        assert_eq!(cursor.left_part(b',', 4), b"abc");
        assert_eq!(cursor.left_part(b',', 4), b"ef");
    }

    #[test]
    fn imported_ass_text_colors_styles_and_fractional_times_are_decoded() {
        assert_eq!(
            ass_unescape(b"\\h\\N\\{x\\}<>&\"'", 1024),
            b" \n{x}&lt;&gt;&amp;&quot;&apos;"
        );
        assert_eq!(ass_unescape(b"\\\xE2\x80\x8BN", 1024), b"\\N");
        assert_eq!(ass_unescape(b"<", 3), b"&l");
        assert!(ass_unescape(b"<", 0).is_empty());
        assert_eq!(to_dec_color(b"563412"), 0x123456);
        assert_eq!(
            de_style_name_prefix(b"danmakuFactory_ext_sub00_Title"),
            Some(b"Title".to_vec())
        );
        assert_eq!(de_style_name_prefix(b"ordinary"), None);
        assert_eq!(time_to_int(b"1:02:03.4567"), 3_723_457);
        assert_eq!(time_to_int(b"0:00:00.9995"), 1000);
        assert_eq!(time_to_int(b"0:00:01.6"), 1600);
        assert_eq!(time_to_int(b"0:00:01"), 1000);
        assert_eq!(time_to_int(b""), 0);
    }

    #[test]
    fn core_string_helpers_preserve_capacity_and_filename_conventions() {
        assert_eq!(trim(b" \r\ntext\t "), b"text");
        assert_eq!(dequote(b" 'text'\t"), b"text");
        assert_eq!(to_lower(b"AbC\0ignored"), b"abc");
        assert_eq!(last_chr(b"a.b.c", b'.'), 3);
        assert_eq!(last_chr(b"abc", 0), 3);
        assert!(is_designated_char(b'\n', b" \n\r"));
        assert!(is_designated_str(b" \n", b" \n\r"));
        assert!(!is_designated_str(b"aa", b"aa"));
        assert_eq!(str_safe_copy(b"abcd", 3), b"ab");
        let mut destination = b"a".to_vec();
        assert_eq!(str_safe_cat(&mut destination, 4, b"bcd"), 4);
        assert_eq!(destination, b"abc");
        assert_eq!(match_count(b"aaaaa", b"aa"), 2);
        assert_eq!(match_count(b"aaa", b""), -1);
        assert_eq!(str_replace(b"aaa", b"aa", b"xy", 10), Some(b"xya".to_vec()));
        assert_eq!(str_replace(b"a", b"a", b"xyz", 2), Some(b"xy".to_vec()));
        assert!(is_start_with(b"abc", b"ab"));
        assert_eq!(filename_get_format(b"dir/file.ass", 4), b"ass");
        assert_eq!(filename_get_path(b"dir\\file.ass", 1024), b"dir\\");
        assert_eq!(filename_get_path(b"/file.ass", 1024), b"");
        assert!(!str_to_bool(b" FALSE \r\n"));
        assert!(str_to_bool(b"0"));
        assert_eq!(bool_to_str(true), "true");
        assert_eq!(unicode_to_utf8(0x1F602), "😂".as_bytes());
        assert!(unicode_to_utf8(0xD800).is_empty());
        let mut cursor = ByteCursor::new(b" first,second");
        assert_eq!(cursor.next_word(1024, b',', false), b"first");
        assert_eq!(cursor.next_word(1024, b',', true), b"first");
    }
}
