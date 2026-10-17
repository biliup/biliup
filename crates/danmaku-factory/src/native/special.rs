//! The legacy type 7 payload is a sequence of delimited fields, not JSON.
#![forbid(unsafe_code)]

use super::number::{checked_i32, float_prefix, int_prefix};
use crate::{ConvertError, ErrorKind};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpecialDanmaku {
    pub(crate) exist_ms: i64,
    pub(crate) move_ms: i64,
    pub(crate) pause_ms: i64,
    pub(crate) start_x: f32,
    pub(crate) start_y: f32,
    pub(crate) end_x: f32,
    pub(crate) end_y: f32,
    pub(crate) fade_start: i32,
    pub(crate) fade_end: i32,
    pub(crate) rotation_y: i32,
    pub(crate) rotation_z: i32,
    pub(crate) font_name: String,
}

/// Takes a field only when its terminating delimiter exists. The C helper
/// returns an empty field and exhausts its input when the delimiter is missing.
/// A slice keeps that rule without the C helper's fixed byte buffers.
fn take_before<'a>(remaining: &mut &'a str, delimiter: char) -> &'a str {
    match remaining.split_once(delimiter) {
        Some((field, tail)) => {
            *remaining = tail;
            field
        }
        None => {
            *remaining = "";
            ""
        }
    }
}

/// C's deQuotMarks trims these four ASCII whitespace characters and removes
/// leading and trailing quotes independently, without requiring a matched pair.
fn dequote(field: &str) -> &str {
    let field = field.trim_matches(|ch| matches!(ch, ' ' | '\n' | '\t' | '\r'));
    let field = field
        .strip_prefix('"')
        .or_else(|| field.strip_prefix('\''))
        .unwrap_or(field);
    field
        .strip_suffix('"')
        .or_else(|| field.strip_suffix('\''))
        .unwrap_or(field)
}

fn coordinate(field: &str) -> Result<f32, ConvertError> {
    let value = float_prefix(dequote(field))? as f32;
    if !value.is_finite() {
        return Err(ConvertError::new(
            ErrorKind::UnsupportedInput,
            "special danmaku coordinate is outside the finite f32 range",
        ));
    }
    // Scaling and the final integer coordinate depend on the resolution and
    // are checked by layout rather than changing the parsed coordinate here.
    Ok(value)
}

pub(crate) fn parse(payload: &str) -> Result<(SpecialDanmaku, String), ConvertError> {
    let mut remaining = payload;
    take_before(&mut remaining, '[');
    let start_x = coordinate(take_before(&mut remaining, ','))?;
    let start_y = coordinate(take_before(&mut remaining, ','))?;
    let fade_start = checked_i32(
        (1.0 - float_prefix(dequote(take_before(&mut remaining, '-')))?) * 255.0,
        "special danmaku fade start",
    )?;
    let fade_end = checked_i32(
        (1.0 - float_prefix(dequote(take_before(&mut remaining, ',')))?) * 255.0,
        "special danmaku fade end",
    )?;
    // Unlike the ordinary XML start time, C feeds atof's double directly to
    // GET_MS_FLT here. Do not introduce an intermediate f32 conversion.
    let exist_ms = i64::from(checked_i32(
        float_prefix(dequote(take_before(&mut remaining, ',')))? * 1000.0 + 0.5,
        "special danmaku lifetime",
    )?);

    take_before(&mut remaining, '"');
    let text = take_before(&mut remaining, '"').replace("/n", "\n");
    take_before(&mut remaining, ',');

    let rotation_z = int_prefix(dequote(take_before(&mut remaining, ',')))?;
    let rotation_y = int_prefix(dequote(take_before(&mut remaining, ',')))?;
    let end_x = coordinate(take_before(&mut remaining, ','))?;
    let end_y = coordinate(take_before(&mut remaining, ','))?;
    let move_ms = int_prefix(dequote(take_before(&mut remaining, ',')))?;
    let pause_ms = int_prefix(dequote(take_before(&mut remaining, ',')))?;
    // Layout checks move + pause only if the resolved coordinates select the
    // timed move branch. Fixed-position payloads never perform that C sum.

    // Skip the historical boolean field, whose value was never interpreted.
    // The next pair of quotes selects the font, even in a partial payload.
    take_before(&mut remaining, '"');
    let font_name = dequote(take_before(&mut remaining, '"')).to_owned();

    Ok((
        SpecialDanmaku {
            exist_ms,
            move_ms: i64::from(move_ms),
            pause_ms: i64::from(pause_ms),
            start_x,
            start_y,
            end_x,
            end_y,
            fade_start,
            fade_end,
            rotation_y,
            rotation_z,
            font_name,
        },
        text,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_payload_keeps_typed_fields_and_legacy_newline() {
        let (special, text) = parse(
            r#"[0.1,0.2,"1-0.5",4.5,"特殊弹幕/n第二行",-45,30,0.8,0.9,1500,200,true,"微软雅黑",1]"#,
        )
        .unwrap();
        assert_eq!(
            special,
            SpecialDanmaku {
                exist_ms: 4500,
                move_ms: 1500,
                pause_ms: 200,
                start_x: 0.1,
                start_y: 0.2,
                end_x: 0.8,
                end_y: 0.9,
                fade_start: 0,
                fade_end: 127,
                rotation_y: 30,
                rotation_z: -45,
                font_name: "微软雅黑".into(),
            }
        );
        assert_eq!(text, "特殊弹幕\n第二行");
    }

    #[test]
    fn missing_delimiters_exhaust_the_input_instead_of_using_a_tail_value() {
        let expected = SpecialDanmaku {
            exist_ms: 0,
            move_ms: 0,
            pause_ms: 0,
            start_x: 0.0,
            start_y: 0.0,
            end_x: 0.0,
            end_y: 0.0,
            fade_start: 255,
            fade_end: 255,
            rotation_y: 0,
            rotation_z: 0,
            font_name: String::new(),
        };
        // These three partial inputs are explicitly accepted by the C
        // sanitizer fixtures. A missing '[' has the same exhausted state.
        for payload in ["[0.5", "[", "", "not an array"] {
            let (special, text) = parse(payload).unwrap();
            assert_eq!(special, expected, "{payload:?}");
            assert_eq!(text, "");
        }
        let (partial, text) = parse("[0.5,0.75").unwrap();
        assert_eq!(partial.start_x, 0.5);
        assert_eq!(partial.start_y, 0.0);
        assert_eq!(partial.fade_start, 255);
        assert_eq!(text, "");
    }

    #[test]
    fn json_escapes_and_ass_tags_are_literal_but_slash_n_is_a_newline() {
        let (_, text) =
            parse(r#"[0,0,"1-1",1,"literal\n\u4e2d;old/nline;{\c&HFF}\N",0,0,0,0,0,0,false,"",1]"#)
                .unwrap();
        assert_eq!(text, "literal\\n\\u4e2d;old\nline;{\\c&HFF}\\N");

        let (special, text) = parse(r#"[0,0,"1-1",1,"a\"b",2,3,4,5,6,7,true,"font",1]"#).unwrap();
        assert_eq!(text, "a\\");
        assert_eq!(special.rotation_z, 2);
        assert_eq!(special.rotation_y, 3);
        assert_eq!(special.font_name, "font");
    }

    #[test]
    fn lifetime_uses_double_precision_and_c_truncation() {
        for (seconds, expected) in [
            ("0.0004999999999999", 0),
            ("0.0005", 1),
            ("-0.0016", -1),
            ("1800000.001", 1_800_000_001),
        ] {
            let payload = format!(r#"[0,0,"1-1",{seconds},"x",0,0,0,0,0,0,true,"",1]"#);
            assert_eq!(parse(&payload).unwrap().0.exist_ms, expected, "{seconds}");
        }
    }

    #[test]
    fn prefix_numbers_and_defined_negative_parameters_are_preserved() {
        let (special, _) = parse(
            r#"[ ' -12.75px ',1.25suffix,"2--1",-0.003seconds,"x",-45deg,30.9,0.8end,-0.5end,10ms,-50ms,ignored," font ",1]"#,
        )
        .unwrap();
        assert_eq!(special.start_x, -12.75);
        assert_eq!(special.start_y, 1.25);
        assert_eq!(special.fade_start, -255);
        assert_eq!(special.fade_end, 510);
        assert_eq!(special.exist_ms, -2);
        assert_eq!(special.rotation_z, -45);
        assert_eq!(special.rotation_y, 30);
        assert_eq!(special.end_x, 0.8);
        assert_eq!(special.end_y, -0.5);
        assert_eq!(special.move_ms, 10);
        assert_eq!(special.pause_ms, -50);
        assert_eq!(special.font_name, "font");
    }

    #[test]
    fn complete_text_and_font_have_no_fixed_c_buffer_limit() {
        let body = "长文👩‍👩‍👧‍👦e\u{301}".repeat(300);
        let font = "字体".repeat(100);
        let payload = format!(r#"[0,0,"1-1",1,"{body}",0,0,0,0,0,0,true,"{font}",1]"#);
        let (special, text) = parse(&payload).unwrap();
        assert_eq!(text, body);
        assert_eq!(special.font_name, font);
    }

    #[test]
    fn parsing_does_not_evaluate_movement_that_a_fixed_position_never_uses() {
        let (special, _) = parse(r#"[0,0,"1-1",1,"x",0,0,0,0,2147483647,1,true,"",1]"#).unwrap();
        assert_eq!(special.start_x, special.end_x);
        assert_eq!(special.start_y, special.end_y);
        assert_eq!(special.move_ms, i64::from(i32::MAX));
        assert_eq!(special.pause_ms, 1);
    }

    #[test]
    fn nonfinite_values_and_c_integer_overflows_are_rejected() {
        for payload in [
            r#"[NaN,0,"1-1",1,"x",0,0,0,0,0,0,true,"",1]"#,
            r#"[3.5e38,0,"1-1",1,"x",0,0,0,0,0,0,true,"",1]"#,
            r#"[0,0,"inf-1",1,"x",0,0,0,0,0,0,true,"",1]"#,
            r#"[0,0,"100000000-1",1,"x",0,0,0,0,0,0,true,"",1]"#,
            r#"[0,0,"1-1",2147484,"x",0,0,0,0,0,0,true,"",1]"#,
            r#"[0,0,"1-1",1,"x",2147483648,0,0,0,0,0,true,"",1]"#,
            r#"[0,0,"1-1",1,"x",0,0,0,0,2147483648,0,true,"",1]"#,
            r#"[0,0,"1-1",1,"x",0,0,0,0,0,-2147483649,true,"",1]"#,
        ] {
            assert_eq!(
                parse(payload).unwrap_err().kind,
                ErrorKind::UnsupportedInput,
                "{payload}"
            );
        }
    }
}
