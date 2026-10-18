use serde::{Deserialize, Serialize};
use std::fmt;

/// Bits of [`ConvertOptions::block_mode`].
pub mod block {
    pub const R2L: u32 = 1;
    pub const L2R: u32 = 2;
    pub const TOP: u32 = 4;
    pub const BOTTOM: u32 = 8;
    pub const SPECIAL: u32 = 16;
    pub const COLOR: u32 = 32;
    pub const REPEAT: u32 = 64;
}

/// DanmakuFactory-compatible settings and defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConvertOptions {
    /// Canvas dimensions, each 1..=16384 pixels.
    pub width: u32,
    pub height: u32,
    /// Canvas height fractions, 0.0001..=1.0001 (the legacy epsilon bound).
    pub display_area: f32,
    pub scroll_area: f32,
    /// Seconds, 0.0001..=3600; quantized using the frozen f32 rules.
    pub scroll_time: f32,
    pub fix_time: f32,
    /// -1 avoids overlap; 0 has no count limit; positive limits active count.
    pub density: i32,
    /// Pixel spacing/margins, each -16384..=16384.
    pub line_spacing: i32,
    pub top_margin: i32,
    pub bottom_margin: i32,
    /// Base size 1..=4096. File sizes are relative to 25 unless normalized.
    pub font_size: i32,
    pub font_size_strict: bool,
    pub font_size_norm: bool,
    /// Nonempty UTF-8 name, under 128 bytes, without comma/ASCII controls.
    pub font_name: String,
    /// Main opacity 1..=255; outline opacity may also be zero.
    pub opacity: u8,
    pub outline: f32,
    /// Blur 0..=100; outline and shadow widths 0..=4.
    pub outline_blur: f32,
    pub outline_opacity: u8,
    pub shadow: f32,
    pub bold: bool,
    /// Preserve legacy Comment/omission rules, including type7 behavior.
    pub save_blocked: bool,
    pub block_mode: u32,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            display_area: 1.,
            scroll_area: 1.,
            scroll_time: 12.,
            fix_time: 5.,
            density: 0,
            line_spacing: 0,
            top_margin: 0,
            bottom_margin: 0,
            font_size: 38,
            font_size_strict: false,
            font_size_norm: false,
            font_name: "Microsoft YaHei".into(),
            opacity: 180,
            outline: 0.,
            outline_blur: 0.,
            outline_opacity: 255,
            shadow: 1.,
            bold: false,
            save_blocked: true,
            block_mode: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertStats {
    /// All successfully read XML comments, including omitted unknown types.
    pub danmaku_count: u32,
    pub blocked_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    InvalidArgument,
    /// Retained for protocol compatibility; native conversion never emits it.
    Locale,
    InputOpen,
    InputRead,
    NoDanmaku,
    UnsupportedInput,
    OutOfMemory,
    OutputExists,
    OutputCreate,
    OutputWrite,
    Internal,
}

impl ErrorKind {
    fn describe(self) -> &'static str {
        match self {
            Self::InvalidArgument => "invalid argument",
            Self::Locale => "unsupported numeric locale",
            Self::InputOpen => "cannot open the XML file",
            Self::InputRead => "cannot read the XML file",
            Self::NoDanmaku => "the XML file contains no danmaku",
            Self::UnsupportedInput => "the XML file contains an unsupported danmaku",
            Self::OutOfMemory => "out of memory",
            Self::OutputExists => "the ASS file already exists",
            Self::OutputCreate => "cannot create the ASS file",
            Self::OutputWrite => "cannot write the ASS file",
            Self::Internal => "internal DanmakuFactory error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertError {
    pub kind: ErrorKind,
    pub upstream_code: i32,
    pub os_error: Option<i32>,
    pub detail: String,
}

impl ConvertError {
    pub(crate) fn new(kind: ErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            upstream_code: 0,
            os_error: None,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.kind.describe())?;
        if !self.detail.is_empty() {
            write!(f, ": {}", self.detail)?;
        }
        if let Some(errno) = self.os_error {
            if cfg!(unix) {
                write!(f, " ({})", std::io::Error::from_raw_os_error(errno))?;
            } else {
                write!(f, " (errno {errno})")?;
            }
        }
        if self.upstream_code != 0 {
            write!(f, " [code {}]", self.upstream_code)?;
        }
        Ok(())
    }
}
impl std::error::Error for ConvertError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_json_shape_and_version_contract_remain_compatible() {
        let options = ConvertOptions {
            display_area: 0.4,
            outline: 2.5,
            scroll_time: 8.3,
            font_name: "Noto Sans CJK SC".into(),
            ..Default::default()
        };
        assert_eq!(
            serde_json::from_str::<ConvertOptions>(&serde_json::to_string(&options).unwrap())
                .unwrap(),
            options
        );
        assert!(serde_json::from_str::<ConvertOptions>(r#"{"unknown":1}"#).is_err());
        assert_eq!(
            serde_json::to_string(&ErrorKind::Locale).unwrap(),
            "\"locale\""
        );
        let legacy = r#"{"kind":"no_danmaku","upstream_code":8,"os_error":null,"detail":"the input contains no danmaku"}"#;
        let error: ConvertError = serde_json::from_str(legacy).unwrap();
        assert_eq!(serde_json::to_string(&error).unwrap(), legacy);
        assert!(crate::version().contains("3813e93b"));
        assert!(!crate::version().contains("PCRE2"));
    }
}
