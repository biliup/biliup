//! Hand-written bindings for `csrc/df_ffi.h`, the C interface to the vendored
//! DanmakuFactory core. Use the `danmaku-factory` crate instead: it checks the
//! options, converts paths, serialises calls and maps errors.
//!
//! The interface is deliberately small, so bindgen (and with it libclang) is
//! not needed; `df_ffi_layout` lets the tests verify that the structs below
//! match the C compiler's layout on every target.
#![allow(non_camel_case_types)]

use std::ffi::c_char;

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

unsafe extern "C" {
    pub fn df_ffi_abi_version() -> u32;
    /// A static NUL-terminated string; never free it.
    pub fn df_ffi_version() -> *const c_char;
    /// Fills the CLI defaults; `font_name` then points to a static string.
    pub fn df_options_init(options: *mut df_options);
    /// Converts one XML file to ASS. See `csrc/df_ffi.h` for the contract:
    /// all pointers are borrowed, nothing is returned that needs freeing.
    pub fn df_convert_xml_to_ass(
        input_path: *const c_char,
        output_path: *const c_char,
        options: *const df_options,
        result: *mut df_result,
    ) -> i32;
    pub fn df_ffi_layout(layout: *mut df_layout);
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
}
