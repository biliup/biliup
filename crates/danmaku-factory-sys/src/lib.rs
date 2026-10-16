//! Pure Rust implementation of the project's DanmakuFactory conversion core.
//! The original C ABI remains available for existing callers. Rust callers
//! should use `danmaku-factory` for atomic output claiming and failure cleanup.
#![allow(non_camel_case_types)]

use std::ffi::{CStr, c_char};
use std::mem::{align_of, offset_of};
use std::path::{Path, PathBuf};

pub mod advanced;
pub mod ass;
pub mod filter;
pub mod formats;
pub mod model;
pub mod statistics;
pub mod string;
pub mod xml;

pub use formats::{AssEvent, AssFile, AssStyle};

use model::{Config, CoreError, EPS};

// Only the first member is read. This preserves the original public ABI's
// numeric-locale refusal; parsing/formatting themselves remain pure Rust.
#[repr(C)]
struct NumericLocale {
    decimal_point: *const c_char,
}

unsafe extern "C" {
    fn localeconv() -> *const NumericLocale;
}

fn validate_numeric_locale() -> Result<(), CoreError> {
    // SAFETY: localeconv returns libc's static locale record. The caller must
    // not concurrently mutate process-global locale, as with the old ABI.
    let locale = unsafe { localeconv() };
    if locale.is_null()
        || unsafe { (*locale).decimal_point }.is_null()
        || unsafe { CStr::from_ptr((*locale).decimal_point) } != c"."
    {
        return Err(CoreError::new(
            DF_ERR_LOCALE,
            0,
            0,
            "LC_NUMERIC must use '.' as the decimal point",
        ));
    }
    Ok(())
}

pub const DF_FFI_ABI_VERSION: u32 = 1;
pub const DF_FFI_MESSAGE_LEN: usize = 256;
/// Including the terminating NUL.
pub const DF_FFI_FONT_NAME_LEN: usize = 128;
pub const DF_FFI_MAX_RESOLUTION: i32 = 16384;

pub const DF_OK: i32 = 0;
pub const DF_ERR_INVALID_ARGUMENT: i32 = 1;
pub const DF_ERR_LOCALE: i32 = 2;
pub const DF_ERR_INPUT_OPEN: i32 = 3;
pub const DF_ERR_INPUT_READ: i32 = 4;
pub const DF_ERR_NO_DANMAKU: i32 = 5;
pub const DF_ERR_UNSUPPORTED_INPUT: i32 = 6;
pub const DF_ERR_OUT_OF_MEMORY: i32 = 7;
pub const DF_ERR_OUTPUT_CREATE: i32 = 8;
pub const DF_ERR_OUTPUT_WRITE: i32 = 9;
pub const DF_ERR_INTERNAL: i32 = 10;

pub const DF_BLOCK_R2L: i32 = 1;
pub const DF_BLOCK_L2R: i32 = 2;
pub const DF_BLOCK_TOP: i32 = 4;
pub const DF_BLOCK_BOTTOM: i32 = 8;
pub const DF_BLOCK_SPECIAL: i32 = 16;
pub const DF_BLOCK_COLOR: i32 = 32;
pub const DF_BLOCK_REPEAT: i32 = 64;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct df_options {
    pub struct_size: u32,
    pub resolution_x: i32,
    pub resolution_y: i32,
    pub display_area: f32,
    pub scroll_area: f32,
    pub scroll_time: f32,
    pub fix_time: f32,
    pub density: i32,
    pub line_spacing: i32,
    pub top_margin: i32,
    pub bottom_margin: i32,
    pub font_size: i32,
    pub font_size_strict: i32,
    pub font_size_norm: i32,
    /// Borrowed NUL-terminated UTF-8, read only during the call.
    pub font_name: *const c_char,
    pub opacity: i32,
    pub outline: f32,
    pub outline_blur: f32,
    pub outline_opacity: i32,
    pub shadow: f32,
    pub bold: i32,
    pub save_blocked: i32,
    pub block_mode: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct df_result {
    pub struct_size: u32,
    pub status: i32,
    pub upstream_code: i32,
    pub os_error: i32,
    pub danmaku_count: i32,
    pub blocked_count: i32,
    pub message: [c_char; DF_FFI_MESSAGE_LEN],
}

impl df_result {
    /// A result ready to be passed to [`df_convert_xml_to_ass`].
    pub fn new() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            status: DF_OK,
            upstream_code: 0,
            os_error: 0,
            danmaku_count: 0,
            blocked_count: 0,
            message: [0; DF_FFI_MESSAGE_LEN],
        }
    }
}

impl Default for df_result {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct df_layout {
    pub options_size: u32,
    pub options_align: u32,
    pub options_font_name_offset: u32,
    pub options_block_mode_offset: u32,
    pub result_size: u32,
    pub result_align: u32,
    pub result_message_offset: u32,
}

pub const VERSION: &str =
    "DanmakuFactory 3813e93b13b087e95901d1822baf9c3540e3f3f6 compatible Rust implementation";
const VERSION_C: &CStr =
    c"DanmakuFactory 3813e93b13b087e95901d1822baf9c3540e3f3f6 compatible Rust implementation";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConversionStats {
    pub danmaku_count: i32,
    pub blocked_count: i32,
}

fn invalid(option: &str) -> CoreError {
    CoreError::new(DF_ERR_INVALID_ARGUMENT, 0, 0, format!("invalid {option}"))
}

fn between(value: f32, low: f32, high: f32) -> bool {
    value.is_finite() && value >= low && value <= high
}

fn validate_prefix(config: &Config) -> Result<(), CoreError> {
    if !(1..=DF_FFI_MAX_RESOLUTION).contains(&config.resolution_x)
        || !(1..=DF_FFI_MAX_RESOLUTION).contains(&config.resolution_y)
    {
        return Err(invalid("resolution"));
    }
    if !between(config.display_area, EPS as f32, (1.0 + EPS) as f32) {
        return Err(invalid("display_area"));
    }
    if !between(config.scroll_area, EPS as f32, (1.0 + EPS) as f32) {
        return Err(invalid("scroll_area"));
    }
    if !between(config.scroll_time, EPS as f32, 3600.0) {
        return Err(invalid("scroll_time"));
    }
    if !between(config.fix_time, EPS as f32, 3600.0) {
        return Err(invalid("fix_time"));
    }
    if config.density < -1 {
        return Err(invalid("density"));
    }
    if !(-DF_FFI_MAX_RESOLUTION..=DF_FFI_MAX_RESOLUTION).contains(&config.line_spacing) {
        return Err(invalid("line_spacing"));
    }
    if !(-DF_FFI_MAX_RESOLUTION..=DF_FFI_MAX_RESOLUTION).contains(&config.top_margin)
        || !(-DF_FFI_MAX_RESOLUTION..=DF_FFI_MAX_RESOLUTION).contains(&config.bottom_margin)
    {
        return Err(invalid("margin"));
    }
    if !(1..=4096).contains(&config.font_size) {
        return Err(invalid("font_size"));
    }
    Ok(())
}

fn validate_suffix(config: &Config) -> Result<(), CoreError> {
    let font = config.font_name.as_bytes();
    if font.is_empty()
        || font.len() >= DF_FFI_FONT_NAME_LEN
        || font
            .iter()
            .any(|byte| *byte < 0x20 || *byte == 0x7f || *byte == b',')
    {
        return Err(invalid("font_name"));
    }
    if !(1..=255).contains(&config.opacity) {
        return Err(invalid("opacity"));
    }
    if !between(config.outline, 0.0, 4.0) {
        return Err(invalid("outline"));
    }
    if !between(config.outline_blur, 0.0, 100.0) {
        return Err(invalid("outline_blur"));
    }
    if !(0..=255).contains(&config.outline_opacity) {
        return Err(invalid("outline_opacity"));
    }
    if !between(config.shadow, 0.0, 4.0) {
        return Err(invalid("shadow"));
    }
    if config.block_mode & !127 != 0 {
        return Err(invalid("block_mode"));
    }
    Ok(())
}

fn validate_legacy_config(config: &Config) -> Result<(), CoreError> {
    validate_prefix(config)?;
    validate_suffix(config)?;
    validate_config(config)
}

/// Validates the owned core interface without imposing the narrower CLI/FFI
/// option limits. Only undefined arithmetic or unusable dimensions are refused.
pub fn validate_config(config: &Config) -> Result<(), CoreError> {
    if config.resolution_x <= 0 || config.resolution_y <= 0 || config.font_size <= 0 {
        return Err(invalid("resolution or font_size"));
    }
    for (value, name) in [
        (config.display_area, "display_area"),
        (config.scroll_area, "scroll_area"),
        (config.outline, "outline"),
        (config.outline_blur, "outline_blur"),
        (config.shadow, "shadow"),
    ] {
        if !value.is_finite() {
            return Err(invalid(name));
        }
    }
    for (value, name) in [
        (config.scroll_time, "scroll_time"),
        (config.fix_time, "fix_time"),
    ] {
        let centiseconds = value * 100.0 + 0.55;
        if !value.is_finite()
            || value <= 0.0
            || f64::from(centiseconds) * 10.0 > f64::from(i32::MAX)
        {
            return Err(invalid(name));
        }
    }
    let font = config.font_name.as_bytes();
    if font.is_empty()
        || font.len() >= DF_FFI_FONT_NAME_LEN
        || font
            .iter()
            .any(|byte| *byte < 0x20 || *byte == 0x7f || *byte == b',')
    {
        return Err(invalid("font_name"));
    }
    if 255_i32.checked_sub(config.opacity).is_none()
        || 255_i32.checked_sub(config.outline_opacity).is_none()
    {
        return Err(invalid("opacity"));
    }
    if config.show_msg_box {
        if config.msg_box_size_x <= 0 || config.msg_box_size_y <= 0 || config.msgbox_font_size <= 0
        {
            return Err(invalid("message box dimensions or font_size"));
        }
        let duration = config.msgbox_duration * 100.0 + 0.55;
        let milliseconds = f64::from(duration).trunc() * 10.0;
        if !milliseconds.is_finite()
            || milliseconds < f64::from(i32::MIN)
            || milliseconds > f64::from(i32::MAX)
        {
            return Err(invalid("msgbox_duration"));
        }
        let minimum_price = config.gift_min_price * 1000.0;
        if !config.gift_min_price.is_finite()
            || !minimum_price.is_finite()
            || f64::from(minimum_price) < f64::from(i32::MIN)
            || f64::from(minimum_price) > f64::from(i32::MAX)
        {
            return Err(invalid("gift_min_price"));
        }
    }
    Ok(())
}

/// Checks owned payloads before any writer can index their contents.
pub fn validate_danmaku(pool: &[model::Danmaku], config: &Config) -> Result<(), CoreError> {
    for (index, item) in pool.iter().enumerate() {
        if item.is_special() && item.special.is_none() {
            return Err(CoreError::new(
                DF_ERR_UNSUPPORTED_INPUT,
                0,
                0,
                format!("danmaku {} is a special danmaku without data", index + 1),
            ));
        }
        if config.show_msg_box && item.is_message() {
            let Some(gift) = item.gift.as_ref().filter(|_| item.user.is_some()) else {
                return Err(CoreError::new(
                    DF_ERR_UNSUPPORTED_INPUT,
                    0,
                    0,
                    format!(
                        "danmaku {} is a message without user or gift data",
                        index + 1
                    ),
                ));
            };
            if gift.duration <= 0 && model::ass_milliseconds(config.msgbox_duration) < 10 {
                return Err(CoreError::new(
                    DF_ERR_UNSUPPORTED_INPUT,
                    0,
                    0,
                    format!(
                        "danmaku {} is a message without a positive duration",
                        index + 1
                    ),
                ));
            }
        }
        let duration = if item.is_type(model::R2L) || item.is_type(model::L2R) {
            i64::from(model::ass_milliseconds(config.scroll_time))
        } else if item.is_type(model::TOP) || item.is_type(model::BOTTOM) {
            i64::from(model::ass_milliseconds(config.fix_time))
        } else if let Some(special) = item.special.as_ref().filter(|_| item.is_special()) {
            (i64::from(special.exist_time) + 5) / 10 * 10
        } else if let Some(gift) = item
            .gift
            .as_ref()
            .filter(|_| config.show_msg_box && item.is_message())
        {
            if model::ass_milliseconds(config.msgbox_duration) >= 10 {
                i64::from(model::ass_milliseconds(config.msgbox_duration))
            } else {
                (i64::from(gift.duration) + 5) / 10 * 10
            }
        } else {
            0
        };
        let start = (i64::from(item.time) + 5) / 10 * 10;
        let end = start + duration;
        if [start, duration, end]
            .into_iter()
            .any(|value| value < i64::from(i32::MIN) || value > i64::from(i32::MAX))
        {
            return Err(CoreError::new(
                DF_ERR_UNSUPPORTED_INPUT,
                0,
                0,
                format!(
                    "danmaku {} ends outside the supported millisecond range",
                    index + 1
                ),
            ));
        }
    }
    Ok(())
}

/// Reads one XML input and creates or truncates its ASS output. This is the
/// low-level ABI contract; the safe crate claims a new output before calling.
pub fn convert(input: &Path, output: &Path, config: &Config) -> Result<ConversionStats, CoreError> {
    validate_legacy_config(config)?;
    validate_numeric_locale()?;
    convert_full(input, output, config)
}

/// Converts using the complete owned core configuration. Numeric parsing and
/// output are locale independent; the legacy ABI keeps its original contract.
pub fn convert_full(
    input: &Path,
    output: &Path,
    config: &Config,
) -> Result<ConversionStats, CoreError> {
    validate_config(config)?;
    let mut pool = xml::read_xml(input)?;
    let count = i32::try_from(pool.len())
        .map_err(|_| CoreError::new(DF_ERR_UNSUPPORTED_INPUT, 0, 0, "too many danmaku"))?;
    validate_danmaku(&pool, config)?;
    filter::block_by_type(&mut pool, config.block_mode);
    if let Some(keys) = config.blocklist.as_ref() {
        if !config.blocklist_regex_enabled {
            filter::block_by_keywords(&mut pool, keys);
        } else {
            filter::block_by_regex_keywords(&mut pool, keys)?;
        }
    }
    filter::normalize_font_size(&mut pool, config);
    filter::sort_list(&mut pool);
    let written = ass::write_ass(output, &mut pool, config);
    let blocked = pool.iter().filter(|item| item.type_ < 0).count() as i32;
    written.map_err(|error| error.with_counts(count, blocked))?;
    Ok(ConversionStats {
        danmaku_count: count,
        blocked_count: blocked,
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn df_ffi_abi_version() -> u32 {
    DF_FFI_ABI_VERSION
}

/// Returns a static NUL-terminated UTF-8 string; never free it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn df_ffi_version() -> *const c_char {
    VERSION_C.as_ptr()
}

/// # Safety
/// `options` must be NULL or valid, aligned and writable for one df_options.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn df_options_init(options: *mut df_options) {
    if options.is_null() {
        return;
    }
    let config = Config::default();
    let raw = df_options {
        struct_size: size_of::<df_options>() as u32,
        resolution_x: config.resolution_x,
        resolution_y: config.resolution_y,
        display_area: config.display_area,
        scroll_area: config.scroll_area,
        scroll_time: config.scroll_time,
        fix_time: config.fix_time,
        density: config.density,
        line_spacing: config.line_spacing,
        top_margin: config.top_margin,
        bottom_margin: config.bottom_margin,
        font_size: config.font_size,
        font_size_strict: config.font_size_strict.into(),
        font_size_norm: config.font_size_norm.into(),
        font_name: c"Microsoft YaHei".as_ptr(),
        opacity: config.opacity,
        outline: config.outline,
        outline_blur: config.outline_blur,
        outline_opacity: config.outline_opacity,
        shadow: config.shadow,
        bold: config.bold.into(),
        save_blocked: config.save_blocked.into(),
        block_mode: config.block_mode,
    };
    // SAFETY: guaranteed by the caller, as for the original C ABI.
    unsafe {
        options.write(raw);
    }
}

/// # Safety
/// `layout` must be NULL or valid, aligned and writable for one df_layout.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn df_ffi_layout(layout: *mut df_layout) {
    if layout.is_null() {
        return;
    }
    let value = df_layout {
        options_size: size_of::<df_options>() as u32,
        options_align: align_of::<df_options>() as u32,
        options_font_name_offset: offset_of!(df_options, font_name) as u32,
        options_block_mode_offset: offset_of!(df_options, block_mode) as u32,
        result_size: size_of::<df_result>() as u32,
        result_align: align_of::<df_result>() as u32,
        result_message_offset: offset_of!(df_result, message) as u32,
    };
    unsafe {
        layout.write(value);
    }
}

unsafe fn config_from_raw(raw: &df_options) -> Result<Config, CoreError> {
    let mut config = Config {
        resolution_x: raw.resolution_x,
        resolution_y: raw.resolution_y,
        display_area: raw.display_area,
        scroll_area: raw.scroll_area,
        scroll_time: raw.scroll_time,
        fix_time: raw.fix_time,
        density: raw.density,
        line_spacing: raw.line_spacing,
        top_margin: raw.top_margin,
        bottom_margin: raw.bottom_margin,
        font_size: raw.font_size,
        font_size_strict: raw.font_size_strict == 1,
        font_size_norm: raw.font_size_norm == 1,
        font_name: String::new(),
        opacity: raw.opacity,
        outline: raw.outline,
        outline_blur: raw.outline_blur,
        outline_opacity: raw.outline_opacity,
        shadow: raw.shadow,
        bold: raw.bold == 1,
        save_blocked: raw.save_blocked == 1,
        block_mode: raw.block_mode,
        ..Config::default()
    };
    validate_prefix(&config)?;
    if [
        raw.font_size_strict,
        raw.font_size_norm,
        raw.bold,
        raw.save_blocked,
    ]
    .into_iter()
    .any(|value| value != 0 && value != 1)
    {
        return Err(invalid("boolean option"));
    }
    if raw.font_name.is_null() {
        return Err(invalid("font_name"));
    }
    config.font_name = unsafe { CStr::from_ptr(raw.font_name) }
        .to_str()
        .map_err(|_| invalid("font_name"))?
        .to_owned();
    validate_suffix(&config)?;
    Ok(config)
}

fn path_from_bytes(bytes: &[u8]) -> Result<PathBuf, CoreError> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
    }
    #[cfg(not(unix))]
    {
        std::str::from_utf8(bytes)
            .map(PathBuf::from)
            .map_err(|_| CoreError::new(DF_ERR_INVALID_ARGUMENT, 0, 0, "invalid UTF-8 path"))
    }
}

fn set_error(result: &mut df_result, error: CoreError) -> i32 {
    result.status = error.status;
    result.upstream_code = error.upstream_code;
    result.os_error = error.os_error;
    result.danmaku_count = error.danmaku_count;
    result.blocked_count = error.blocked_count;
    let bytes = error.detail.as_bytes();
    let len = bytes.len().min(DF_FFI_MESSAGE_LEN - 1);
    for (target, byte) in result.message[..len].iter_mut().zip(bytes) {
        *target = *byte as c_char;
    }
    result.message[len] = 0;
    result.status
}

/// # Safety
/// Pointer arguments must be NULL or valid and aligned for their stated type;
/// path and font pointers must contain readable NUL-terminated strings. Every
/// pointer is borrowed only for the call. The output follows the old raw ABI
/// and may be truncated or incomplete on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn df_convert_xml_to_ass(
    input_path: *const c_char,
    output_path: *const c_char,
    options: *const df_options,
    result: *mut df_result,
) -> i32 {
    if result.is_null() || unsafe { (*result).struct_size } != size_of::<df_result>() as u32 {
        return DF_ERR_INVALID_ARGUMENT;
    }
    let result = unsafe { &mut *result };
    *result = df_result::new();
    if input_path.is_null() || output_path.is_null() || options.is_null() {
        return set_error(
            result,
            CoreError::new(
                DF_ERR_INVALID_ARGUMENT,
                0,
                0,
                "missing input path, output path or options",
            ),
        );
    }
    let raw = unsafe { &*options };
    if raw.struct_size != size_of::<df_options>() as u32 {
        return set_error(
            result,
            CoreError::new(
                DF_ERR_INVALID_ARGUMENT,
                0,
                0,
                format!(
                    "df_options size {}, expected {}",
                    raw.struct_size,
                    size_of::<df_options>()
                ),
            ),
        );
    }
    let converted = std::panic::catch_unwind(|| -> Result<ConversionStats, CoreError> {
        let config = unsafe { config_from_raw(raw) }?;
        let input = path_from_bytes(unsafe { CStr::from_ptr(input_path) }.to_bytes())?;
        let output = path_from_bytes(unsafe { CStr::from_ptr(output_path) }.to_bytes())?;
        convert(&input, &output, &config)
    });
    match converted {
        Ok(Ok(stats)) => {
            result.danmaku_count = stats.danmaku_count;
            result.blocked_count = stats.blocked_count;
            DF_OK
        }
        Ok(Err(error)) => set_error(result, error),
        Err(_) => set_error(
            result,
            CoreError::new(DF_ERR_INTERNAL, 0, 0, "conversion panicked"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;
    use std::mem::{align_of, offset_of};

    #[test]
    fn rust_structs_match_the_c_layout() {
        let mut layout = df_layout::default();
        unsafe { df_ffi_layout(&mut layout) };
        assert_eq!(
            layout,
            df_layout {
                options_size: size_of::<df_options>() as u32,
                options_align: align_of::<df_options>() as u32,
                options_font_name_offset: offset_of!(df_options, font_name) as u32,
                options_block_mode_offset: offset_of!(df_options, block_mode) as u32,
                result_size: size_of::<df_result>() as u32,
                result_align: align_of::<df_result>() as u32,
                result_message_offset: offset_of!(df_result, message) as u32,
            }
        );
        assert_eq!(unsafe { df_ffi_abi_version() }, DF_FFI_ABI_VERSION);
        // Values from the original header's C layout, rather than merely
        // comparing the Rust implementation with its own offset calculation.
        assert_eq!(layout.options_font_name_offset, 56);
        assert_eq!(layout.result_size, 280);
        assert_eq!(layout.result_align, 4);
        assert_eq!(layout.result_message_offset, 24);
        if cfg!(target_pointer_width = "64") {
            assert_eq!(
                (
                    layout.options_size,
                    layout.options_align,
                    layout.options_block_mode_offset
                ),
                (96, 8, 92)
            );
        } else if cfg!(target_pointer_width = "32") {
            assert_eq!(
                (
                    layout.options_size,
                    layout.options_align,
                    layout.options_block_mode_offset
                ),
                (92, 4, 88)
            );
        }
    }

    #[test]
    fn defaults_are_the_cli_defaults() {
        let mut options = std::mem::MaybeUninit::<df_options>::zeroed();
        let options = unsafe {
            df_options_init(options.as_mut_ptr());
            options.assume_init()
        };
        assert_eq!(options.struct_size as usize, size_of::<df_options>());
        assert_eq!((options.resolution_x, options.resolution_y), (1920, 1080));
        assert_eq!((options.scroll_time, options.fix_time), (12., 5.));
        assert_eq!((options.font_size, options.opacity), (38, 180));
        let font = unsafe { CStr::from_ptr(options.font_name) };
        assert_eq!(font.to_str().unwrap(), "Microsoft YaHei");
        let version = unsafe { CStr::from_ptr(df_ffi_version()) };
        assert!(
            version
                .to_str()
                .unwrap()
                .contains("3813e93b13b087e95901d1822baf9c3540e3f3f6")
        );
    }

    #[test]
    fn unusable_result_is_rejected_without_writing() {
        let mut result = df_result::new();
        result.struct_size = 1;
        let status = unsafe {
            df_convert_xml_to_ass(
                c"in.xml".as_ptr(),
                c"out.ass".as_ptr(),
                std::ptr::null(),
                &mut result,
            )
        };
        assert_eq!(status, DF_ERR_INVALID_ARGUMENT);
        assert_eq!(result.status, DF_OK);
        let status = unsafe {
            df_convert_xml_to_ass(
                c"in.xml".as_ptr(),
                c"out.ass".as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, DF_ERR_INVALID_ARGUMENT);
    }

    #[test]
    fn raw_boolean_and_utf8_validation_keeps_error_precedence_and_resets_results() {
        let mut raw = std::mem::MaybeUninit::<df_options>::zeroed();
        unsafe { df_options_init(raw.as_mut_ptr()) };
        let mut raw = unsafe { raw.assume_init() };
        raw.bold = 2;
        raw.resolution_x = 0;
        let mut result = df_result::new();
        result.danmaku_count = 100;
        result.blocked_count = 40;
        unsafe {
            df_convert_xml_to_ass(
                c"missing.xml".as_ptr(),
                c"unused.ass".as_ptr(),
                &raw,
                &mut result,
            )
        };
        assert_eq!(
            (result.status, result.danmaku_count, result.blocked_count),
            (DF_ERR_INVALID_ARGUMENT, 0, 0)
        );
        assert_eq!(
            unsafe { CStr::from_ptr(result.message.as_ptr()) },
            c"invalid resolution"
        );
        raw.resolution_x = 1920;
        unsafe {
            df_convert_xml_to_ass(
                c"missing.xml".as_ptr(),
                c"unused.ass".as_ptr(),
                &raw,
                &mut result,
            )
        };
        assert_eq!(
            unsafe { CStr::from_ptr(result.message.as_ptr()) },
            c"invalid boolean option"
        );
        raw.bold = 0;
        let invalid_font = [0xff_u8, 0];
        raw.font_name = invalid_font.as_ptr().cast();
        unsafe {
            df_convert_xml_to_ass(
                c"missing.xml".as_ptr(),
                c"unused.ass".as_ptr(),
                &raw,
                &mut result,
            )
        };
        assert_eq!(
            unsafe { CStr::from_ptr(result.message.as_ptr()) },
            c"invalid font_name"
        );
    }

    #[test]
    fn millisecond_overflow_is_rejected_before_rounding_wraps() {
        let config = Config::default();
        let last = model::Danmaku {
            time: i32::MAX,
            type_: model::R2L,
            ..Default::default()
        };
        assert_eq!(
            validate_danmaku(&[last], &config).unwrap_err().status,
            DF_ERR_UNSUPPORTED_INPUT
        );
        let special = model::Danmaku {
            time: 0,
            type_: model::SPECIAL,
            special: Some(Box::new(model::SpecialPart {
                exist_time: i32::MAX,
                ..Default::default()
            })),
            ..Default::default()
        };
        assert_eq!(
            validate_danmaku(&[special], &config).unwrap_err().status,
            DF_ERR_UNSUPPORTED_INPUT
        );
        let allowed = model::Danmaku {
            time: 2_000_000_000,
            type_: model::R2L,
            ..Default::default()
        };
        validate_danmaku(&[allowed], &config).unwrap();
    }

    #[test]
    fn opacity_subtraction_overflow_is_rejected_by_full_core_validation() {
        for outline in [false, true] {
            let mut config = Config::default();
            if outline {
                config.outline_opacity = i32::MIN + 255;
            } else {
                config.opacity = i32::MIN + 255;
            }
            assert_eq!(
                validate_config(&config).unwrap_err().status,
                DF_ERR_INVALID_ARGUMENT
            );
            if outline {
                config.outline_opacity += 1;
            } else {
                config.opacity += 1;
            }
            validate_config(&config).unwrap();
        }
    }
}
