//! ASS escaping receives logical text, never XML source or entity syntax.

use std::io::{self, Write};

/// Stream unchanged runs and escapes without allocating a second full body.
pub(crate) fn write_ordinary(mut writer: impl Write, text: &str) -> io::Result<()> {
    let mut chars = text.char_indices().peekable();
    let mut start = 0;
    while let Some((at, ch)) = chars.next() {
        let replacement = match ch {
            '{' => Some("\\{"),
            '}' => Some("\\}"),
            ' ' => Some("\\h"),
            '\n' => Some("\\N"),
            _ => None,
        };
        if ch == '\\' && matches!(chars.peek(), Some((_, 'n' | 'N' | 'h'))) {
            writer.write_all(&text.as_bytes()[start..at])?;
            writer.write_all("\\\u{200b}".as_bytes())?;
            let (_, next) = chars.next().expect("peeked character");
            writer.write_all(&[next as u8])?;
            start = at + 2;
        } else if let Some(replacement) = replacement {
            writer.write_all(&text.as_bytes()[start..at])?;
            writer.write_all(replacement.as_bytes())?;
            start = at + ch.len_utf8();
        }
    }
    writer.write_all(&text.as_bytes()[start..])
}

#[cfg(test)]
pub(crate) fn ordinary(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' if matches!(chars.peek(), Some('n' | 'N' | 'h')) => {
                escaped.push_str("\\\u{200b}");
                escaped.push(chars.next().expect("peeked character"));
            }
            '{' => escaped.push_str("\\{"),
            '}' => escaped.push_str("\\}"),
            ' ' => escaped.push_str("\\h"),
            '\n' => escaped.push_str("\\N"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entities_are_literal_and_ass_escapes_remain_text() {
        assert_eq!(
            ordinary("&lt; &#55357; {x} \\N \\h\n"),
            "&lt;\\h&#55357;\\h\\{x\\}\\h\\\u{200b}N\\h\\\u{200b}h\\N"
        );
        let text = "字😀e\u{301}".repeat(2048);
        assert_eq!(ordinary(&text), text);
        for text in [text.as_str(), "&lt; {中} \\N \\h\n", "abc\\", "\\\\N", ""] {
            let mut output = Vec::new();
            write_ordinary(&mut output, text).unwrap();
            assert_eq!(output, ordinary(text).into_bytes());
        }
    }
}
