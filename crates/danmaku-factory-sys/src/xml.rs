//! Streaming reader for the recorder XML formats accepted by DanmakuFactory.
//!
//! Recorder files can be read while still being written. The legacy reader
//! therefore scans complete opening tags without requiring a closed root.

use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use crate::model::{
    BOTTOM, CoreError, Danmaku, EPS, GiftPart, L2R, MAX_TEXT_LENGTH, MSG_GIFT, MSG_GUARD,
    MSG_SUPER_CHAT, R2L, SPECIAL, SpecialPart, TOP, UserPart,
};
use crate::string::{
    ByteCursor, dequote, nul_prefix, parse_f64, parse_i32, parse_u64, trim, xml_unescape,
};
use crate::{DF_ERR_INPUT_OPEN, DF_ERR_INPUT_READ, DF_ERR_NO_DANMAKU, DF_ERR_UNSUPPORTED_INPUT};

const LABEL_LENGTH: usize = 10240;
const USER_NAME_LENGTH: usize = 128;
const GIFT_NAME_LENGTH: usize = 64;
const KEY_LENGTH: usize = 128;
const VALUE_LENGTH: usize = 512;

struct Scanner<R> {
    reader: R,
    text_mode: bool,
    ended: bool,
}

impl<R: BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            text_mode: cfg!(windows),
            ended: false,
        }
    }

    fn raw_byte(&mut self) -> io::Result<Option<u8>> {
        let byte = self.reader.fill_buf()?.first().copied();
        if byte.is_some() {
            self.reader.consume(1);
        }
        Ok(byte)
    }

    // CRT text mode changes CRLF and treats SUB as EOF on Windows.
    fn byte(&mut self) -> io::Result<Option<u8>> {
        if self.ended {
            return Ok(None);
        }
        let Some(byte) = self.raw_byte()? else {
            return Ok(None);
        };
        if self.text_mode && byte == 0x1A {
            self.ended = true;
            return Ok(None);
        }
        if self.text_mode && byte == b'\r' && self.reader.fill_buf()?.first() == Some(&b'\n') {
            self.reader.consume(1);
            return Ok(Some(b'\n'));
        }
        Ok(Some(byte))
    }

    fn skip_to(&mut self, delimiter: u8) -> io::Result<bool> {
        if self.text_mode {
            while let Some(byte) = self.byte()? {
                if byte == delimiter {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        loop {
            let buffer = self.reader.fill_buf()?;
            if buffer.is_empty() {
                return Ok(false);
            }
            let found = buffer.iter().position(|&byte| byte == delimiter);
            let consumed = found.map_or(buffer.len(), |at| at + 1);
            self.reader.consume(consumed);
            if found.is_some() {
                return Ok(true);
            }
        }
    }

    fn label(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut output = Vec::with_capacity(128);
        if self.text_mode {
            while let Some(byte) = self.byte()? {
                if byte == b'>' {
                    return Ok(Some(output));
                }
                if output.len() < LABEL_LENGTH - 1 {
                    output.push(byte);
                }
            }
            return Ok(None);
        }
        loop {
            let buffer = self.reader.fill_buf()?;
            if buffer.is_empty() {
                return Ok(None);
            }
            let found = buffer.iter().position(|&byte| byte == b'>');
            let count = found.unwrap_or(buffer.len());
            let available = (LABEL_LENGTH - 1).saturating_sub(output.len());
            output.extend_from_slice(&buffer[..count.min(available)]);
            self.reader.consume(count + usize::from(found.is_some()));
            if found.is_some() {
                return Ok(Some(output));
            }
        }
    }

    fn body(&mut self) -> io::Result<Vec<u8>> {
        let mut output = Vec::with_capacity(128);
        if self.text_mode {
            while let Some(byte) = self.byte()? {
                // fgetc occurs before the length check, so one more byte is
                // consumed even when a long payload has filled its buffer.
                if byte == b'<' || output.len() == MAX_TEXT_LENGTH - 1 {
                    break;
                }
                output.push(byte);
            }
        } else {
            loop {
                let buffer = self.reader.fill_buf()?;
                if buffer.is_empty() {
                    break;
                }
                let available = (MAX_TEXT_LENGTH - 1) - output.len();
                let examined = buffer.len().min(available + 1);
                if let Some(at) = buffer[..examined].iter().position(|&byte| byte == b'<') {
                    output.extend_from_slice(&buffer[..at]);
                    self.reader.consume(at + 1);
                    break;
                }
                let count = buffer.len().min(available);
                output.extend_from_slice(&buffer[..count]);
                if buffer.len() > available {
                    self.reader.consume(available + 1);
                    break;
                }
                self.reader.consume(count);
            }
        }
        output.truncate(nul_prefix(&output).len());
        Ok(output)
    }
}

impl<R: BufRead + Seek> Scanner<R> {
    fn is_bililive_recorder(&mut self) -> io::Result<bool> {
        let mut remaining = 1024;
        let mut found = false;
        // findSubstr used fgets, checking each bounded line independently.
        // Its byte budget was based on strlen, which stops at embedded NUL.
        while remaining > 1 {
            let mut line = Vec::with_capacity(remaining - 1);
            while line.len() < remaining - 1 {
                let Some(byte) = self.byte()? else {
                    break;
                };
                line.push(byte);
                if byte == b'\n' {
                    break;
                }
            }
            if line.is_empty() {
                break;
            }
            let visible = nul_prefix(&line);
            if visible
                .windows(b"<BililiveRecorder".len())
                .any(|part| part == b"<BililiveRecorder")
            {
                found = true;
                break;
            }
            remaining -= visible.len();
        }
        self.reader.seek(SeekFrom::Start(0))?;
        self.ended = false;
        Ok(found)
    }
}

pub fn read_xml(path: &Path) -> Result<Vec<Danmaku>, CoreError> {
    let mut output = Vec::new();
    read_xml_into(path, &mut output, false, 0.0)?;
    Ok(output)
}

/// Reads or appends recorder XML, applying the offset before millisecond
/// rounding. An input-open failure leaves the existing pool untouched.
pub fn read_xml_into(
    path: &Path,
    pool: &mut Vec<Danmaku>,
    append: bool,
    time_shift: f32,
) -> Result<(), CoreError> {
    let file = File::open(path).map_err(|error| {
        CoreError::io(DF_ERR_INPUT_OPEN, 1, &error, "cannot open the input file")
    })?;
    let mut scanner = Scanner::new(BufReader::new(file));
    let recorder = match scanner.is_bililive_recorder() {
        Ok(recorder) => recorder,
        Err(error) => {
            pool.clear();
            return Err(read_error(error));
        }
    };
    if !append {
        pool.clear();
    }
    match read(&mut scanner, recorder, time_shift, !pool.is_empty()) {
        Ok(items) => {
            if pool.is_empty() {
                *pool = items;
            } else {
                pool.reserve_exact(items.len());
                pool.extend(items);
            }
            Ok(())
        }
        Err(error) => {
            pool.clear();
            Err(error)
        }
    }
}

fn read_error(error: io::Error) -> CoreError {
    CoreError::io(DF_ERR_INPUT_READ, 2, &error, "cannot read the input file")
}

fn unsupported(detail: impl Into<String>) -> CoreError {
    CoreError::new(DF_ERR_UNSUPPORTED_INPUT, 0, 0, detail)
}

fn milliseconds(seconds: f64) -> Result<i32, CoreError> {
    let value = seconds * 1000.0 + 0.5;
    if !value.is_finite() || value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
        return Err(unsupported(
            "danmaku duration is outside the supported millisecond range",
        ));
    }
    Ok(value as i32)
}

fn start_milliseconds(time: f32) -> Result<i32, CoreError> {
    if !time.is_finite() {
        return Err(unsupported("danmaku time is not finite"));
    }
    let time = if f64::from(time) < EPS { 0.0 } else { time };
    let value = time * 1000.0_f32 + 0.5_f32;
    // A float at 2^31 already overflows C's int conversion. f64 comparison
    // avoids rounding i32::MAX up to that same f32 value.
    if !value.is_finite() || f64::from(value) > f64::from(i32::MAX) {
        return Err(unsupported(
            "danmaku time is outside the supported millisecond range",
        ));
    }
    Ok(value as i32)
}

fn read<R: BufRead>(
    scanner: &mut Scanner<R>,
    bililive_recorder: bool,
    time_shift: f32,
    allow_empty: bool,
) -> Result<Vec<Danmaku>, CoreError> {
    let mut output = Vec::new();
    // Font, color and time carry forward when omitted, as in the C reader.
    // Zero initialises the old reader's otherwise indeterminate first time.
    let mut time = 0.0_f32;
    let mut font_size = 0_i16;
    let mut color = 0_i32;
    while scanner.skip_to(b'<').map_err(read_error)? {
        let Some(label) = scanner.label().map_err(read_error)? else {
            break;
        };
        let mut label = ByteCursor::new(&label);
        let message_type = match label.word(MAX_TEXT_LENGTH) {
            b"d" => 0,
            b"gift" => MSG_GIFT,
            b"sc" => MSG_SUPER_CHAT,
            b"guard" => MSG_GUARD,
            _ => continue,
        };
        let has_gift = message_type != 0;
        let mut has_user = false;
        let mut user = UserPart {
            level: -1,
            medal_level: -1,
            ..Default::default()
        };
        let mut gift = GiftPart {
            price: -1,
            count: -1,
            ..Default::default()
        };
        let mut type_ = 0_i16;
        while !label.is_empty() {
            let key = trim(label.left_part(b'=', MAX_TEXT_LENGTH));
            match key {
                b"p" => {
                    time = parse_f64(dequote(label.left_part(b',', MAX_TEXT_LENGTH))) as f32;
                    type_ = parse_i32(dequote(label.left_part(b',', MAX_TEXT_LENGTH))) as i16;
                    font_size = parse_i32(dequote(label.left_part(b',', MAX_TEXT_LENGTH))) as i16;
                    color = parse_i32(dequote(label.left_part(b',', MAX_TEXT_LENGTH)));
                    label.left_part(b'"', MAX_TEXT_LENGTH);
                }
                b"ts" => {
                    time = parse_f64(dequote(label.word(MAX_TEXT_LENGTH))) as f32;
                }
                b"user" => {
                    label.left_part(b'"', MAX_TEXT_LENGTH);
                    user.name = label.left_part(b'"', USER_NAME_LENGTH).to_vec();
                    has_user = true;
                }
                b"uid" => {
                    user.uid = parse_u64(dequote(label.word(MAX_TEXT_LENGTH)));
                }
                b"giftname" => {
                    gift.name = dequote(label.word(GIFT_NAME_LENGTH)).to_vec();
                }
                b"giftcount" | b"count" => {
                    gift.count = parse_i32(dequote(label.word(MAX_TEXT_LENGTH)));
                }
                b"price" => {
                    gift.price = parse_i32(dequote(label.word(MAX_TEXT_LENGTH)));
                    if bililive_recorder && message_type == MSG_SUPER_CHAT {
                        gift.price = gift.price.wrapping_mul(1000);
                    }
                }
                b"time" => {
                    gift.duration = milliseconds(parse_f64(dequote(label.word(MAX_TEXT_LENGTH))))?;
                }
                b"level" => {
                    gift.price = match parse_i32(dequote(label.word(MAX_TEXT_LENGTH))) {
                        1 => 19998000,
                        2 => 1998000,
                        3 => 198000,
                        _ => 18000,
                    };
                    gift.duration = gift.price;
                }
                b"raw" if has_gift => {
                    label.left_part(b'"', LABEL_LENGTH);
                    let raw = xml_unescape(label.left_part(b'"', LABEL_LENGTH));
                    read_raw(&raw, &mut user, &mut gift)?;
                }
                b"raw" => {
                    // The legacy non-gift branch does not advance here; the
                    // next iteration consumes its value as an unknown key.
                }
                b"cointype" => {
                    if dequote(label.word(GIFT_NAME_LENGTH)) == "银瓜子".as_bytes() {
                        gift.price = 0;
                    }
                }
                _ => {
                    label.word(MAX_TEXT_LENGTH);
                }
            }
        }
        if message_type == MSG_GIFT && gift.duration == 0 {
            gift.duration = 5000;
        }
        let text = if message_type == 0 || message_type == MSG_SUPER_CHAT {
            xml_unescape(&scanner.body().map_err(read_error)?)
        } else {
            Vec::new()
        };
        let type_ = if message_type != 0 {
            message_type
        } else {
            match type_ {
                1 => R2L,
                6 => L2R,
                5 => TOP,
                4 => BOTTOM,
                7 => SPECIAL,
                other => other,
            }
        };
        time += time_shift;
        if f64::from(time) < EPS {
            time = 0.0;
        }
        let mut item = Danmaku {
            time: start_milliseconds(time)?,
            type_,
            font_size,
            color,
            text,
            text_is_null: message_type == MSG_GIFT || message_type == MSG_GUARD,
            user: has_user.then(|| Box::new(user)),
            gift: has_gift.then(|| Box::new(gift)),
            special: None,
        };
        if type_ == SPECIAL {
            item.special = Some(Box::new(read_special(&mut item.text)?));
        } else if type_ == -SPECIAL {
            return Err(unsupported(format!(
                "danmaku {} is a special danmaku without data",
                output.len() + 1
            )));
        }
        output.push(item);
    }
    if output.is_empty() && !allow_empty {
        return Err(CoreError::new(
            DF_ERR_NO_DANMAKU,
            8,
            0,
            "the input contains no danmaku",
        ));
    }
    Ok(output)
}

fn read_raw(raw: &[u8], user: &mut UserPart, gift: &mut GiftPart) -> Result<(), CoreError> {
    let mut raw = ByteCursor::new(raw);
    raw.left_part(b'{', LABEL_LENGTH);
    let mut coin_type = Vec::new();
    while !raw.is_empty() {
        let key = dequote(raw.left_part(b':', KEY_LENGTH));
        let value = dequote(raw.left_part(b',', VALUE_LENGTH));
        match key {
            b"gift_name" => {
                gift.name = value[..value.len().min(GIFT_NAME_LENGTH - 1)].to_vec();
            }
            b"coin_type" => {
                coin_type = value[..value.len().min(GIFT_NAME_LENGTH - 1)].to_vec();
            }
            b"uid" if user.uid == 0 => {
                user.uid = parse_u64(value);
            }
            b"price" if gift.price == -1 => {
                gift.price = parse_i32(value);
            }
            b"combo_stay_time" if gift.duration == 0 => {
                gift.duration = milliseconds(parse_f64(value))?;
            }
            _ => {}
        }
    }
    if coin_type == b"silver" {
        gift.price = 0;
    }
    Ok(())
}

fn read_special(text: &mut Vec<u8>) -> Result<SpecialPart, CoreError> {
    let mut cursor = ByteCursor::new(text);
    cursor.left_part(b'[', MAX_TEXT_LENGTH);
    let start_x = parse_f64(dequote(cursor.left_part(b',', MAX_TEXT_LENGTH))) as f32;
    let start_y = parse_f64(dequote(cursor.left_part(b',', MAX_TEXT_LENGTH))) as f32;
    let fade_start = (1.0 - parse_f64(dequote(cursor.left_part(b'-', MAX_TEXT_LENGTH)))) * 255.0;
    let fade_end = (1.0 - parse_f64(dequote(cursor.left_part(b',', MAX_TEXT_LENGTH)))) * 255.0;
    if [f64::from(start_x), f64::from(start_y), fade_start, fade_end]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(unsupported(
            "special danmaku coordinates or opacity are not finite",
        ));
    }
    let exist_time = milliseconds(parse_f64(dequote(cursor.left_part(b',', MAX_TEXT_LENGTH))))?;
    cursor.left_part(b'"', MAX_TEXT_LENGTH);
    let original_text = cursor.left_part(b'"', MAX_TEXT_LENGTH);
    let mut body = Vec::with_capacity(original_text.len());
    let mut at = 0;
    while at < original_text.len() {
        if original_text[at..].starts_with(b"/n") {
            body.push(b'\n');
            at += 2;
        } else {
            body.push(original_text[at]);
            at += 1;
        }
    }
    cursor.left_part(b',', MAX_TEXT_LENGTH);
    let fr_z = parse_i32(dequote(cursor.left_part(b',', MAX_TEXT_LENGTH)));
    let fr_y = parse_i32(dequote(cursor.left_part(b',', MAX_TEXT_LENGTH)));
    let end_x = parse_f64(dequote(cursor.left_part(b',', MAX_TEXT_LENGTH))) as f32;
    let end_y = parse_f64(dequote(cursor.left_part(b',', MAX_TEXT_LENGTH))) as f32;
    let move_time = parse_i32(dequote(cursor.left_part(b',', MAX_TEXT_LENGTH)));
    let pause_time = parse_i32(dequote(cursor.left_part(b',', MAX_TEXT_LENGTH)));
    cursor.left_part(b'"', MAX_TEXT_LENGTH);
    let font_name = dequote(cursor.left_part(b'"', MAX_TEXT_LENGTH)).to_vec();
    if font_name.len() >= 128 {
        return Err(unsupported("special danmaku font name is too long"));
    }
    if !end_x.is_finite() || !end_y.is_finite() {
        return Err(unsupported("special danmaku coordinates are not finite"));
    }
    *text = body;
    Ok(SpecialPart {
        start_x,
        start_y,
        end_x,
        end_y,
        fade_start: fade_start as i32,
        fade_end: fade_end as i32,
        exist_time,
        fr_z,
        fr_y,
        move_time,
        pause_time,
        font_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn parse(xml: &[u8]) -> Result<Vec<Danmaku>, CoreError> {
        // Tiny buffers force every kind of tag/entity across a read boundary.
        let mut scanner = Scanner::new(BufReader::with_capacity(7, Cursor::new(xml)));
        let recorder = scanner.is_bililive_recorder().unwrap();
        read(&mut scanner, recorder, 0.0, false)
    }

    #[test]
    fn recorder_files_do_not_require_a_closed_root_or_last_element() {
        let items =
            parse(b"<i><d p=\"0.0005,1,25,16777215,0\">a&amp;\n</d><d p=\"2,6,30,123,0\">last")
                .unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(
            (items[0].time, items[0].font_size, items[0].color),
            (1, 25, 16777215)
        );
        assert_eq!(items[0].text, b"a&\n");
        assert_eq!(items[1].type_, L2R);
        assert_eq!(items[1].text, b"last");
        assert_eq!(
            parse(b"<d p=\"1,1,25,0,0\" incomplete").unwrap_err().status,
            DF_ERR_NO_DANMAKU
        );
    }

    #[test]
    fn input_and_tag_limits_are_byte_limits_with_bounded_reads() {
        let mut input = b"<i><unknown value=\"".to_vec();
        input.extend(std::iter::repeat_n(b'x', LABEL_LENGTH * 2));
        input.extend_from_slice(b"\"><d p=\"1,1,25,0,0\">");
        input.extend_from_slice(&"中".repeat(342).into_bytes());
        input.extend_from_slice(b"</d><d p=\"2,5,25,0,0\">next</d>");
        let items = parse(&input).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text.len(), 1023);
        assert_eq!(items[1].type_, TOP);
        assert_eq!(items[1].text, b"next");
        let mut short = b"<d p=\"1,1,25,0,0\">".to_vec();
        short.extend_from_slice(&vec![b'a'; 1022]);
        short.extend_from_slice("中".as_bytes());
        let items = parse(&short).unwrap();
        assert_eq!(items[0].text.len(), 1023);
        assert_eq!(items[0].text.last(), Some(&0xE4));
    }

    #[test]
    fn messages_preserve_recorder_price_and_raw_metadata_rules() {
        let items = parse(br#"<BililiveRecorder><sc ts="1.5" user="A B" uid="7" price="30" time="60">hello</sc>
          <gift ts="2" user="G" raw="{&quot;gift_name&quot;:&quot;Rose&quot;,&quot;coin_type&quot;:&quot;silver&quot;,&quot;uid&quot;:9,&quot;price&quot;:100,&quot;combo_stay_time&quot;:2.5,}"></gift>
          <guard ts="3" user="C" level="3"></guard>"#).unwrap();
        assert_eq!(
            items.iter().map(|item| item.type_).collect::<Vec<_>>(),
            [MSG_SUPER_CHAT, MSG_GIFT, MSG_GUARD]
        );
        assert_eq!(items[0].text, b"hello");
        assert_eq!(items[0].user.as_ref().unwrap().name, b"A B");
        assert_eq!(items[0].gift.as_ref().unwrap().price, 30000);
        assert_eq!(items[0].gift.as_ref().unwrap().duration, 60000);
        let gift = items[1].gift.as_ref().unwrap();
        assert_eq!((gift.price, gift.duration), (0, 2500));
        assert_eq!(gift.name, b"Rose");
        assert_eq!(items[1].user.as_ref().unwrap().uid, 9);
        assert_eq!(items[2].gift.as_ref().unwrap().price, 198000);
    }

    #[test]
    fn special_payloads_keep_legacy_text_and_motion_fields() {
        let items = parse(br#"<d p="1,7,25,16777215,0">[0.1,0.2,"0.5-1",7,"a/n{b}",10,20,0.7,0.8,500,30,true,"Example Font",1]</d>"#).unwrap();
        let item = &items[0];
        assert_eq!(item.type_, SPECIAL);
        assert_eq!(item.text, b"a\n{b}");
        let special = item.special.as_ref().unwrap();
        assert_eq!((special.fade_start, special.fade_end), (127, 0));
        assert_eq!(
            (special.exist_time, special.move_time, special.pause_time),
            (7000, 500, 30)
        );
        assert_eq!((special.fr_z, special.fr_y), (10, 20));
        assert_eq!(special.font_name, b"Example Font");
        assert_eq!(special.end_x, 0.7);
    }

    #[test]
    fn invalid_time_and_missing_special_data_are_safe_errors() {
        for time in ["nan", "inf", "2147484"] {
            let input = format!("<d p=\"{time},1,25,0,0\">x</d>");
            assert_eq!(
                parse(input.as_bytes()).unwrap_err().status,
                DF_ERR_UNSUPPORTED_INPUT
            );
        }
        assert_eq!(
            parse(b"<d p=\"1,-5,25,0,0\">x</d>").unwrap_err().status,
            DF_ERR_UNSUPPORTED_INPUT
        );
        assert_eq!(parse(b"<d p=\"-2,1,25,0,0\">x</d>").unwrap()[0].time, 0);
    }

    #[test]
    fn windows_text_mode_converts_crlf_and_stops_at_sub() {
        let input = b"<d p=\"1,1,25,0,0\">one\r\ntwo</d>\x1A<d p=\"2,1,25,0,0\">hidden</d>";
        let mut scanner = Scanner::new(BufReader::with_capacity(1, Cursor::new(input)));
        scanner.text_mode = true;
        let items = read(&mut scanner, false, 0.0, false).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, b"one\ntwo");
    }

    #[test]
    fn offsets_are_added_to_float_seconds_before_rounding_and_carry_forward() {
        let input = b"<d p=\"0.0004,1,25,0,0\">first</d><d>second</d>";
        let mut scanner = Scanner::new(BufReader::new(Cursor::new(input)));
        let items = read(&mut scanner, false, 0.0004, false).unwrap();
        assert_eq!(
            items.iter().map(|item| item.time).collect::<Vec<_>>(),
            [1, 1]
        );
        let mut scanner = Scanner::new(BufReader::new(Cursor::new(b"<i></i>")));
        assert!(read(&mut scanner, false, 0.0, true).unwrap().is_empty());
    }

    #[test]
    fn null_message_text_is_distinct_from_an_empty_normal_payload() {
        let items = parse(b"<gift ts=\"1\"></gift><guard ts=\"2\"></guard><sc ts=\"3\"></sc><d p=\"4,13,25,0,0\"></d>").unwrap();
        assert_eq!(
            items
                .iter()
                .map(|item| item.text_is_null)
                .collect::<Vec<_>>(),
            [true, true, false, false]
        );
    }
}
