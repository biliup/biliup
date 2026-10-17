//! Locale independent numeric prefixes, preserving the defined atof/atoi path.
use crate::{ConvertError, ErrorKind};

fn invalid(field: &str) -> ConvertError {
    ConvertError::new(
        ErrorKind::UnsupportedInput,
        format!("invalid or overflowing {field}"),
    )
}

pub(crate) fn checked_i32(value: f64, field: &str) -> Result<i32, ConvertError> {
    let truncated = value.trunc();
    if !value.is_finite() || truncated < i32::MIN as f64 || truncated > i32::MAX as f64 {
        return Err(invalid(field));
    }
    Ok(truncated as i32)
}

pub(crate) fn checked_i64(value: f64, field: &str) -> Result<i64, ConvertError> {
    let truncated = value.trunc();
    // i64::MAX rounds up to 2^63 as f64; that exclusive upper bound must
    // therefore be rejected rather than relying on Rust's saturating cast.
    if !value.is_finite() || truncated < i64::MIN as f64 || truncated >= -(i64::MIN as f64) {
        return Err(invalid(field));
    }
    Ok(truncated as i64)
}

pub(crate) fn int_prefix(text: &str) -> Result<i32, ConvertError> {
    let text = text.trim_start_matches(|ch: char| ch.is_ascii_whitespace());
    let bytes = text.as_bytes();
    let start = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let mut end = start;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    if end == start {
        return Ok(0);
    }
    text[..end].parse().map_err(|_| invalid("integer"))
}

pub(crate) fn float_prefix(text: &str) -> Result<f64, ConvertError> {
    let text = text.trim_start_matches(|ch: char| ch.is_ascii_whitespace());
    let bytes = text.as_bytes();
    let start = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let tail = &text[start..];
    if tail
        .get(..3)
        .is_some_and(|s| s.eq_ignore_ascii_case("inf") || s.eq_ignore_ascii_case("nan"))
    {
        return Err(invalid("number"));
    }
    if tail.starts_with("0x") || tail.starts_with("0X") {
        return hex_prefix(text, start);
    }
    let mut end = start;
    let mut digits = 0;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
        digits += 1;
    }
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return Ok(0.);
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let exponent = end;
        end += 1;
        if matches!(bytes.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let first = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if end == first {
            end = exponent;
        }
    }
    let value: f64 = text[..end].parse().map_err(|_| invalid("number"))?;
    if !value.is_finite() {
        return Err(invalid("number"));
    }
    Ok(value)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Retain the first 64 significant bits and a sticky bit, then round once to
/// IEEE binary64 (including subnormals). No decimal locale or C runtime calls.
fn hex_prefix(text: &str, sign_len: usize) -> Result<f64, ConvertError> {
    let bytes = text.as_bytes();
    let mut at = sign_len + 2;
    let mut after_point = false;
    let mut digits = 0;
    let mut fractional = 0i64;
    let mut retained = 0;
    let mut discarded = 0i64;
    let mut significand = 0u64;
    let mut sticky = false;
    while let Some(&byte) = bytes.get(at) {
        if byte == b'.' && !after_point {
            after_point = true;
            at += 1;
            continue;
        }
        let Some(digit) = hex_digit(byte) else { break };
        digits += 1;
        fractional += i64::from(after_point);
        if retained == 0 && digit == 0 {
            at += 1;
            continue;
        }
        if retained < 16 {
            significand = (significand << 4) | u64::from(digit);
            retained += 1;
        } else {
            discarded += 1;
            sticky |= digit != 0;
        }
        at += 1;
    }
    if digits == 0 {
        return Ok(0.);
    }
    let mut exponent = 0i64;
    if matches!(bytes.get(at), Some(b'p' | b'P')) {
        let mut position = at + 1;
        let negative = bytes.get(position) == Some(&b'-');
        if matches!(bytes.get(position), Some(b'+' | b'-')) {
            position += 1;
        }
        while let Some(byte @ b'0'..=b'9') = bytes.get(position) {
            exponent = exponent
                .saturating_mul(10)
                .saturating_add(i64::from(byte - b'0'));
            position += 1;
        }
        if negative {
            exponent = exponent.saturating_neg();
        }
    }
    let negative = bytes.first() == Some(&b'-');
    let sign = u64::from(negative) << 63;
    if significand == 0 {
        return Ok(f64::from_bits(sign));
    }
    exponent = exponent
        .saturating_sub(fractional.saturating_mul(4))
        .saturating_add(discarded.saturating_mul(4));
    let high_bit = 63 - significand.leading_zeros() as i64;
    let mut high_exponent = exponent.saturating_add(high_bit);
    if high_exponent > 1023 {
        return Err(invalid("hex number"));
    }
    if high_exponent < -1075 {
        return Ok(f64::from_bits(sign));
    }
    let shift = if high_exponent < -1022 {
        -1074 - exponent
    } else {
        high_bit - 52
    };
    let mut rounded = if shift <= 0 {
        significand << (-shift as u32)
    } else if shift > 64 {
        0
    } else {
        let kept = if shift == 64 { 0 } else { significand >> shift };
        let half = 1u64 << (shift - 1);
        let remainder = significand & (half | (half - 1));
        kept + u64::from(remainder > half || (remainder == half && (sticky || kept & 1 != 0)))
    };
    if high_exponent < -1022 {
        return Ok(f64::from_bits(sign | rounded));
    }
    if rounded == 1u64 << 53 {
        rounded >>= 1;
        high_exponent += 1;
    }
    if high_exponent > 1023 {
        return Err(invalid("hex number"));
    }
    Ok(f64::from_bits(
        sign | ((high_exponent + 1023) as u64) << 52 | (rounded & ((1u64 << 52) - 1)),
    ))
}

pub(crate) fn source_time_ms(text: &str) -> Result<i64, ConvertError> {
    let mut seconds = float_prefix(text)? as f32;
    if !seconds.is_finite() {
        return Err(invalid("time"));
    }
    if (seconds as f64) < 1e-4 {
        seconds = 0.;
    }
    Ok(i64::from(checked_i32(
        (seconds * 1000. + 0.5) as f64,
        "time",
    )?))
}

pub(crate) fn ass_start_ms(ms: i64) -> Result<i64, ConvertError> {
    if ms
        .checked_add(5)
        .is_none_or(|v| v > i32::MAX as i64 || v < i32::MIN as i64)
    {
        return Err(invalid("ASS time"));
    }
    let value = ms
        .checked_add(5)
        .map(|v| v / 10 * 10)
        .ok_or_else(|| invalid("ASS time"))?;
    if !(0..=2_147_483_640).contains(&value) {
        return Err(invalid("ASS time"));
    }
    Ok(value)
}

pub(crate) fn ass_duration_ms(seconds: f32) -> Result<i64, ConvertError> {
    let centiseconds = checked_i32((seconds * 100. + 0.55) as f64, "duration")?;
    let value = i64::from(centiseconds) * 10;
    if value < i32::MIN as i64 || value > 2_147_483_640 {
        return Err(invalid("duration"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_prefixes_preserve_defined_c_behavior() {
        for (text, expected) in [
            (" \t-12.5junk", -12.5),
            ("1.5e2tail", 150.),
            ("1e+", 1.),
            ("nonsense", 0.),
            (".5x", 0.5),
            ("0x1.8p2x", 6.),
            ("-0x.8p+1", -1.),
            ("0x1p-1074", f64::from_bits(1)),
            ("0x1.00000000000008p0", 1.),
            ("0x1.0000000000000801p0", f64::from_bits(1f64.to_bits() + 1)),
        ] {
            assert_eq!(float_prefix(text).unwrap(), expected, "{text}");
        }
        assert_eq!(int_prefix(" -12.5").unwrap(), -12);
        assert_eq!(int_prefix("abc").unwrap(), 0);
        for invalid in ["NaN", "-Infinity", "1e400", "0x1p1024"] {
            assert!(float_prefix(invalid).is_err(), "{invalid}");
        }
        assert!(int_prefix("2147483648").is_err());
        assert!(checked_i32(f64::NAN, "test").is_err());
    }

    #[test]
    fn wide_geometry_conversion_checks_the_exact_float_integer_boundary() {
        assert_eq!(
            checked_i64(4_000_000_000.9, "width").unwrap(),
            4_000_000_000
        );
        assert_eq!(
            checked_i64(-4_000_000_000.9, "width").unwrap(),
            -4_000_000_000
        );
        assert_eq!(checked_i64(i64::MIN as f64, "width").unwrap(), i64::MIN);
        let largest_float_below_upper_bound = f64::from_bits((-(i64::MIN as f64)).to_bits() - 1);
        assert_eq!(
            checked_i64(largest_float_below_upper_bound, "width").unwrap(),
            9_223_372_036_854_774_784
        );
        for invalid in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            i64::MAX as f64,
            f64::from_bits((i64::MIN as f64).to_bits() + 1),
        ] {
            assert!(checked_i64(invalid, "width").is_err());
        }
    }

    #[test]
    fn time_matches_independent_c_probe_before_ass_quantization() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/time.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let source = source_time_ms(case["seconds"].as_str().unwrap()).unwrap();
            assert_eq!(source, case["source_time_ms"].as_i64().unwrap());
            assert_eq!(
                ass_start_ms(source).unwrap(),
                case["ass_start_ms"].as_i64().unwrap()
            );
        }
        assert_eq!(source_time_ms("-3").unwrap(), 0);
        assert!(source_time_ms("2147484").is_err());
        assert_eq!(ass_duration_ms(12.).unwrap(), 12000);
        assert_eq!(ass_duration_ms(0.0001).unwrap(), 0);
    }
}
