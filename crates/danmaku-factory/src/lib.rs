//! XML -> ASS danmaku conversion with DanmakuFactory's core, compiled into
//! this program (see `danmaku-factory-sys`) instead of run as an external
//! DanmakuFactory executable.
//!
//! [`convert_xml_to_ass`] runs the C core on the calling thread and returns
//! when it is done: it cannot be cancelled or timed out, and a crash in C
//! ends the whole process. Callers that need cancellation, a timeout or crash
//! isolation run it in a separate process; biliup does that with its internal
//! danmaku worker mode.

use danmaku_factory_sys as sys;
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::fmt;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

/// Bits of [`ConvertOptions::block_mode`] (the CLI's `--blockmode`).
pub mod block {
    pub const R2L: u32 = 1;
    pub const L2R: u32 = 2;
    pub const TOP: u32 = 4;
    pub const BOTTOM: u32 = 8;
    pub const SPECIAL: u32 = 16;
    /// Every danmaku that is not white.
    pub const COLOR: u32 = 32;
    /// The same text while an earlier one is still on screen.
    pub const REPEAT: u32 = 64;
}

/// One conversion's settings, named after the DanmakuFactory CLI options.
/// [`Default`] is the CLI's built-in configuration. The core checks the
/// ranges (those of the CLI, plus NaN and overflow checks) and returns
/// [`ErrorKind::InvalidArgument`] for values outside them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConvertOptions {
    /// `--resolution`, 1..=16384 each
    pub width: u32,
    pub height: u32,
    /// `--displayarea`, share of the height used by danmaku, 0.0001..=1
    pub display_area: f32,
    /// `--scrollarea`, share of the height used by scrolling danmaku
    pub scroll_area: f32,
    /// `--scrolltime`, seconds a scrolling danmaku takes to cross the screen
    pub scroll_time: f32,
    /// `--fixtime`, seconds a top or bottom danmaku stays
    pub fix_time: f32,
    /// `--density`: -1 never overlap, 0 unlimited, n at most n on screen
    pub density: i32,
    /// `--line-spacing`, pixels
    pub line_spacing: i32,
    /// `--top-margin`, pixels
    pub top_margin: i32,
    /// `--bottom-margin`, pixels
    pub bottom_margin: i32,
    /// `--fontsize`, 1..=4096
    pub font_size: i32,
    /// `--font-size-strict`: ignore the sizes in the XML
    pub font_size_strict: bool,
    /// `--font-size-norm`: ignore them when one is out of range
    pub font_size_norm: bool,
    /// `--fontname`, under 128 bytes, without `,` or control characters
    pub font_name: String,
    /// `--opacity`, 1..=255
    pub opacity: u8,
    /// `--outline`, 0..=4
    pub outline: f32,
    /// `--outline-blur`, 0..=100
    pub outline_blur: f32,
    /// `--outline-opacity`
    pub outline_opacity: u8,
    /// `--shadow`, 0..=4
    pub shadow: f32,
    /// `--bold`
    pub bold: bool,
    /// `--saveblocked`: keep blocked danmaku as ASS comments
    pub save_blocked: bool,
    /// `--blockmode`, [`block`] bits
    pub block_mode: u32,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        // defaultConfig in DanmakuFactory's main.c; tests compare these with
        // df_options_init().
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

/// What a successful conversion read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertStats {
    /// Danmaku read from the XML.
    pub danmaku_count: u32,
    /// Of those, blocked by type, density or repetition (written as ASS
    /// comments, or left out when `save_blocked` is off).
    pub blocked_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// An option is out of range, or a path or string cannot be passed to C.
    InvalidArgument,
    /// The process's LC_NUMERIC does not use '.' as the decimal point.
    Locale,
    InputOpen,
    InputRead,
    /// The XML contains no danmaku.
    NoDanmaku,
    /// The XML contains a danmaku the core cannot write safely.
    UnsupportedInput,
    OutOfMemory,
    /// The output path already exists; it is never overwritten.
    OutputExists,
    OutputCreate,
    /// Writing or closing the output failed, e.g. because the disk is full.
    OutputWrite,
    Internal,
}

impl ErrorKind {
    fn from_status(status: i32) -> Self {
        match status {
            sys::DF_ERR_INVALID_ARGUMENT => Self::InvalidArgument,
            sys::DF_ERR_LOCALE => Self::Locale,
            sys::DF_ERR_INPUT_OPEN => Self::InputOpen,
            sys::DF_ERR_INPUT_READ => Self::InputRead,
            sys::DF_ERR_NO_DANMAKU => Self::NoDanmaku,
            sys::DF_ERR_UNSUPPORTED_INPUT => Self::UnsupportedInput,
            sys::DF_ERR_OUT_OF_MEMORY => Self::OutOfMemory,
            sys::DF_ERR_OUTPUT_CREATE => Self::OutputCreate,
            sys::DF_ERR_OUTPUT_WRITE => Self::OutputWrite,
            _ => Self::Internal,
        }
    }

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
    /// Status of the failing core function (readXml, sortList or writeAss).
    pub upstream_code: i32,
    /// `errno` of a failed open or write.
    pub os_error: Option<i32>,
    pub detail: String,
}

impl ConvertError {
    fn new(kind: ErrorKind, detail: impl Into<String>) -> Self {
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
            write!(f, " ({})", os_error_text(errno))?;
        }
        if self.upstream_code != 0 {
            write!(f, " [code {}]", self.upstream_code)?;
        }
        Ok(())
    }
}

impl std::error::Error for ConvertError {}

/// errno from the C runtime. On Unix it is the OS error; Windows' CRT errno
/// values are not Win32 error codes, so only the number is shown there.
fn os_error_text(errno: i32) -> String {
    if cfg!(unix) {
        std::io::Error::from_raw_os_error(errno).to_string()
    } else {
        format!("errno {errno}")
    }
}

/// The pinned DanmakuFactory commit, patch level and PCRE2 version.
pub fn version() -> &'static str {
    // SAFETY: a static NUL-terminated ASCII string.
    unsafe { CStr::from_ptr(sys::df_ffi_version()) }
        .to_str()
        .unwrap_or("DanmakuFactory")
}

/// Calls into the core are serialised. The audit in csrc/df_ffi.c found no
/// shared mutable state on the conversion path, but nothing upstream
/// promises that, so the guarantee does not depend on it.
static CORE: Mutex<()> = Mutex::new(());

/// Converts the XML at `input` to a new ASS file at `output`, like
/// `DanmakuFactory -i input.xml -o output.ass` with `options`.
///
/// `output` must not exist; after a failure nothing is left there. Blocks
/// the calling thread for the whole conversion (see the crate docs).
pub fn convert_xml_to_ass(
    input: &Path,
    output: &Path,
    options: &ConvertOptions,
) -> Result<ConvertStats, ConvertError> {
    let font_name = CString::new(options.font_name.as_str())
        .map_err(|_| ConvertError::new(ErrorKind::InvalidArgument, "invalid font_name"))?;
    let input_path = c_path(input)?;
    let output_path = c_path(output)?;
    if output.symlink_metadata().is_ok() {
        return Err(ConvertError::new(
            ErrorKind::OutputExists,
            output.display().to_string(),
        ));
    }
    let to_i32 = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
    let raw = sys::df_options {
        struct_size: size_of::<sys::df_options>() as u32,
        resolution_x: to_i32(options.width),
        resolution_y: to_i32(options.height),
        display_area: options.display_area,
        scroll_area: options.scroll_area,
        scroll_time: options.scroll_time,
        fix_time: options.fix_time,
        density: options.density,
        line_spacing: options.line_spacing,
        top_margin: options.top_margin,
        bottom_margin: options.bottom_margin,
        font_size: options.font_size,
        font_size_strict: options.font_size_strict.into(),
        font_size_norm: options.font_size_norm.into(),
        font_name: font_name.as_ptr(),
        opacity: options.opacity.into(),
        outline: options.outline,
        outline_blur: options.outline_blur,
        outline_opacity: options.outline_opacity.into(),
        shadow: options.shadow,
        bold: options.bold.into(),
        save_blocked: options.save_blocked.into(),
        block_mode: to_i32(options.block_mode),
    };
    let mut result = sys::df_result::new();
    let status = {
        let _core = CORE.lock().unwrap_or_else(PoisonError::into_inner);
        // SAFETY: every pointer is valid and NUL-terminated for the whole
        // call, `raw` and `result` carry their own sizes, and the core keeps
        // no pointer after returning.
        unsafe {
            sys::df_convert_xml_to_ass(
                input_path.as_ptr(),
                output_path.as_ptr(),
                &raw,
                &mut result,
            )
        }
    };
    if status == sys::DF_OK {
        return Ok(ConvertStats {
            danmaku_count: result.danmaku_count.max(0) as u32,
            blocked_count: result.blocked_count.max(0) as u32,
        });
    }
    // The output did not exist before the call, so whatever is there now is
    // an incomplete file from this conversion.
    let _ = std::fs::remove_file(output);
    // SAFETY: the core always NUL-terminates the message buffer.
    let detail = unsafe { CStr::from_ptr(result.message.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    Err(ConvertError {
        kind: ErrorKind::from_status(status),
        upstream_code: result.upstream_code,
        os_error: (result.os_error != 0).then_some(result.os_error),
        detail,
    })
}

/// fopen() takes the path bytes on Unix. On Windows the core converts UTF-8
/// to UTF-16 for _wfopen, so the path must be Unicode; long absolute paths
/// get the `\\?\` prefix, which _wfopen needs beyond MAX_PATH.
fn c_path(path: &Path) -> Result<CString, ConvertError> {
    let invalid = || ConvertError::new(ErrorKind::InvalidArgument, format!("unusable path {path:?}"));
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    };
    #[cfg(windows)]
    let bytes = windows_path(path).ok_or_else(invalid)?.into_bytes();
    #[cfg(not(any(unix, windows)))]
    let bytes = path.to_str().ok_or_else(invalid)?.as_bytes().to_vec();
    if bytes.is_empty() {
        return Err(invalid());
    }
    CString::new(bytes).map_err(|_| invalid())
}

#[cfg(windows)]
fn windows_path(path: &Path) -> Option<String> {
    use std::path::{Component, Prefix};
    let text = path.to_str()?;
    // MAX_PATH is 260 UTF-16 units including the terminator.
    if text.encode_utf16().count() < 240 {
        return Some(text.to_owned());
    }
    // absolute() also turns '/' into '\' and resolves "." and "..", which
    // verbatim paths no longer do.
    let absolute = std::path::absolute(path).ok()?;
    let text = absolute.to_str()?;
    Some(match absolute.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(_) => format!(r"\\?\{text}"),
            Prefix::UNC(..) => format!(r"\\?\UNC\{}", &text[2..]),
            // already verbatim, or a device path
            _ => text.to_owned(),
        },
        _ => text.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_core() {
        let mut raw = std::mem::MaybeUninit::<sys::df_options>::zeroed();
        // SAFETY: df_options_init fills every field.
        let raw = unsafe {
            sys::df_options_init(raw.as_mut_ptr());
            raw.assume_init()
        };
        let ours = ConvertOptions::default();
        let font = unsafe { CStr::from_ptr(raw.font_name) }.to_str().unwrap();
        assert_eq!(font, ours.font_name);
        assert_eq!(
            (raw.resolution_x, raw.resolution_y),
            (ours.width as i32, ours.height as i32)
        );
        assert_eq!(
            [
                raw.display_area,
                raw.scroll_area,
                raw.scroll_time,
                raw.fix_time,
                raw.outline,
                raw.outline_blur,
                raw.shadow
            ],
            [
                ours.display_area,
                ours.scroll_area,
                ours.scroll_time,
                ours.fix_time,
                ours.outline,
                ours.outline_blur,
                ours.shadow
            ]
        );
        assert_eq!(
            [
                raw.density,
                raw.line_spacing,
                raw.top_margin,
                raw.bottom_margin,
                raw.font_size,
                raw.font_size_strict,
                raw.font_size_norm,
                raw.opacity,
                raw.outline_opacity,
                raw.bold,
                raw.save_blocked,
                raw.block_mode
            ],
            [
                ours.density,
                ours.line_spacing,
                ours.top_margin,
                ours.bottom_margin,
                ours.font_size,
                ours.font_size_strict as i32,
                ours.font_size_norm as i32,
                ours.opacity as i32,
                ours.outline_opacity as i32,
                ours.bold as i32,
                ours.save_blocked as i32,
                ours.block_mode as i32
            ]
        );
    }

    #[test]
    fn options_round_trip_through_json_exactly() {
        let options = ConvertOptions {
            display_area: 0.4,
            outline: 2.5,
            scroll_time: 8.3,
            font_name: "Noto Sans CJK SC".into(),
            ..Default::default()
        };
        let json = serde_json::to_string(&options).unwrap();
        assert_eq!(
            serde_json::from_str::<ConvertOptions>(&json).unwrap(),
            options
        );
        assert!(serde_json::from_str::<ConvertOptions>(r#"{"unknown":1}"#).is_err());
    }
}
