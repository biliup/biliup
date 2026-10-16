// Translated from PCRE2 10.49 pcre2_config.c; code unit width 16.
// /*************************************************
// *      Perl-Compatible Regular Expressions       *
// *************************************************/
//
// /* PCRE is a library of functions to support regular expressions whose syntax
// and semantics are as close as possible to those of the Perl 5 language.
//
//                        Written by Philip Hazel
//      Original API code Copyright (c) 1997-2012 University of Cambridge
//           New API code Copyright (c) 2016-2020 University of Cambridge
//
// -----------------------------------------------------------------------------
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
//     * Redistributions of source code must retain the above copyright notice,
//       this list of conditions and the following disclaimer.
//
//     * Redistributions in binary form must reproduce the above copyright
//       notice, this list of conditions and the following disclaimer in the
//       documentation and/or other materials provided with the distribution.
//
//     * Neither the name of the University of Cambridge nor the names of its
//       contributors may be used to endorse or promote products derived from
//       this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.
// -----------------------------------------------------------------------------
// */
extern "C" {
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    static mut _pcre2_unicode_version_16: *const ::core::ffi::c_char;
    fn _pcre2_jit_get_target_16() -> *const ::core::ffi::c_char;
    fn _pcre2_strcpy_c8_16(_: *mut PCRE2_UCHAR16, _: *const ::core::ffi::c_char) -> size_t;
}
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type size_t = usize;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR16 = uint16_t;
pub const HEAP_LIMIT: ::core::ffi::c_int = 20000000 as ::core::ffi::c_int;
pub const MATCH_LIMIT: ::core::ffi::c_int = 10000000 as ::core::ffi::c_int;
pub const MATCH_LIMIT_DEPTH: ::core::ffi::c_int = MATCH_LIMIT;
pub const NEWLINE_DEFAULT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PARENS_NEST_LIMIT: ::core::ffi::c_int = 250 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const PCRE2_BSR_UNICODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PCRE2_ERROR_BADOPTION: ::core::ffi::c_int = -(34 as ::core::ffi::c_int);
pub const PCRE2_CONFIG_BSR: uint32_t = 0 as uint32_t;
pub const PCRE2_CONFIG_JIT: uint32_t = 1 as uint32_t;
pub const PCRE2_CONFIG_JITTARGET: uint32_t = 2 as uint32_t;
pub const PCRE2_CONFIG_LINKSIZE: uint32_t = 3 as uint32_t;
pub const PCRE2_CONFIG_MATCHLIMIT: uint32_t = 4 as uint32_t;
pub const PCRE2_CONFIG_NEWLINE: uint32_t = 5 as uint32_t;
pub const PCRE2_CONFIG_PARENSLIMIT: uint32_t = 6 as uint32_t;
pub const PCRE2_CONFIG_DEPTHLIMIT: uint32_t = 7 as uint32_t;
pub const PCRE2_CONFIG_STACKRECURSE: uint32_t = 8 as uint32_t;
pub const PCRE2_CONFIG_UNICODE: uint32_t = 9 as uint32_t;
pub const PCRE2_CONFIG_UNICODE_VERSION: uint32_t = 10 as uint32_t;
pub const PCRE2_CONFIG_VERSION: uint32_t = 11 as uint32_t;
pub const PCRE2_CONFIG_HEAPLIMIT: uint32_t = 12 as uint32_t;
pub const PCRE2_CONFIG_NEVER_BACKSLASH_C: uint32_t = 13 as uint32_t;
pub const PCRE2_CONFIG_COMPILED_WIDTHS: uint32_t = 14 as uint32_t;
pub const PCRE2_CONFIG_TABLES_LENGTH: uint32_t = 15 as uint32_t;
pub const PCRE2_CONFIG_EFFECTIVE_LINKSIZE: uint32_t = 16 as uint32_t;
pub const cbit_length: ::core::ffi::c_int = 320 as ::core::ffi::c_int;
pub const cbits_offset: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
pub const ctypes_offset: ::core::ffi::c_int = cbits_offset + cbit_length;
pub const TABLES_LENGTH: ::core::ffi::c_int = ctypes_offset + 256 as ::core::ffi::c_int;
pub const CONFIGURED_LINK_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const LINK_SIZE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn pcre2_config_16(
    mut what: uint32_t,
    mut where_0: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    if where_0.is_null() {
        match what {
            0 | 14 | 7 | 16 | 12 | 1 | 3 | 4 | 13 | 5 | 6 | 8 | 15 | 9 => {
                return ::core::mem::size_of::<uint32_t>() as ::core::ffi::c_int;
            }
            2 | 10 | 11 => {}
            _ => return PCRE2_ERROR_BADOPTION,
        }
    }
    match what {
        0 => {
            *(where_0 as *mut uint32_t) = PCRE2_BSR_UNICODE as uint32_t;
        }
        14 => {
            *(where_0 as *mut uint32_t) = (0 as ::core::ffi::c_int
                + ((1 as ::core::ffi::c_int) << 0 as ::core::ffi::c_int)
                + ((1 as ::core::ffi::c_int) << 1 as ::core::ffi::c_int)
                + ((1 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int))
                as uint32_t;
        }
        7 => {
            *(where_0 as *mut uint32_t) = MATCH_LIMIT_DEPTH as uint32_t;
        }
        16 => {
            *(where_0 as *mut uint32_t) = (LINK_SIZE as usize)
                .wrapping_mul(::core::mem::size_of::<PCRE2_UCHAR16>() as usize)
                as uint32_t;
        }
        12 => {
            *(where_0 as *mut uint32_t) = HEAP_LIMIT as uint32_t;
        }
        1 => {
            *(where_0 as *mut uint32_t) = 1 as uint32_t;
        }
        2 => {
            let mut v: *const ::core::ffi::c_char = _pcre2_jit_get_target_16();
            return (1 as size_t).wrapping_add(
                (if where_0.is_null() {
                    strlen(v)
                } else {
                    _pcre2_strcpy_c8_16(where_0 as *mut PCRE2_UCHAR16, v)
                }),
            ) as ::core::ffi::c_int;
        }
        3 => {
            *(where_0 as *mut uint32_t) = CONFIGURED_LINK_SIZE as uint32_t;
        }
        4 => {
            *(where_0 as *mut uint32_t) = MATCH_LIMIT as uint32_t;
        }
        5 => {
            *(where_0 as *mut uint32_t) = NEWLINE_DEFAULT as uint32_t;
        }
        13 => {
            *(where_0 as *mut uint32_t) = 0 as uint32_t;
        }
        6 => {
            *(where_0 as *mut uint32_t) = PARENS_NEST_LIMIT as uint32_t;
        }
        8 => {
            *(where_0 as *mut uint32_t) = 0 as uint32_t;
        }
        15 => {
            *(where_0 as *mut uint32_t) = TABLES_LENGTH as uint32_t;
        }
        10 => {
            let mut v_0: *const ::core::ffi::c_char = _pcre2_unicode_version_16;
            return (1 as size_t).wrapping_add(
                (if where_0.is_null() {
                    strlen(v_0)
                } else {
                    _pcre2_strcpy_c8_16(where_0 as *mut PCRE2_UCHAR16, v_0)
                }),
            ) as ::core::ffi::c_int;
        }
        9 => {
            *(where_0 as *mut uint32_t) = 1 as uint32_t;
        }
        11 => {
            let mut v_1: *const ::core::ffi::c_char =
                if ::core::mem::transmute::<[u8; 2], [::core::ffi::c_char; 2]>(*b"Z\0")
                    [1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    == 0 as ::core::ffi::c_int
                {
                    b"10.49 2026-09-28\0" as *const u8 as *const ::core::ffi::c_char
                } else {
                    b"10.492026-09-28\0" as *const u8 as *const ::core::ffi::c_char
                };
            return (1 as size_t).wrapping_add(
                (if where_0.is_null() {
                    strlen(v_1)
                } else {
                    _pcre2_strcpy_c8_16(where_0 as *mut PCRE2_UCHAR16, v_1)
                }),
            ) as ::core::ffi::c_int;
        }
        _ => return PCRE2_ERROR_BADOPTION,
    }
    return 0 as ::core::ffi::c_int;
}
