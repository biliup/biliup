// Translated from PCRE2 10.49 pcre2_newline.c; code unit width 16.
// /*************************************************
// *      Perl-Compatible Regular Expressions       *
// *************************************************/
//
// /* PCRE is a library of functions to support regular expressions whose syntax
// and semantics are as close as possible to those of the Perl 5 language.
//
//                        Written by Philip Hazel
//      Original API code Copyright (c) 1997-2012 University of Cambridge
//          New API code Copyright (c) 2016 University of Cambridge
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
//
//
// /* This module contains internal functions for testing newlines when more than
// one kind of newline is to be recognized. When a newline is found, its length is
// returned. In principle, we could implement several newline "types", each
// referring to a different set of newline characters. At present, PCRE2 supports
// only NLTYPE_FIXED, which gets handled without these functions, NLTYPE_ANYCRLF,
// and NLTYPE_ANY. The full list of Unicode newline characters is taken from
// http://unicode.org/unicode/reports/tr18/. */
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type BOOL = ::core::ffi::c_int;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR16 = uint16_t;
pub type PCRE2_SPTR16 = *const PCRE2_UCHAR16;
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const NLTYPE_ANYCRLF: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const CHAR_VT: uint32_t = 11 as uint32_t;
pub const CHAR_FF: uint32_t = 12 as uint32_t;
pub const CHAR_CR: ::core::ffi::c_int = '\r' as i32;
pub const CHAR_LF: ::core::ffi::c_int = '\n' as i32;
pub const CHAR_NEL: uint32_t = 133 as uint32_t;
#[no_mangle]
pub unsafe extern "C" fn _pcre2_is_newline_16(
    mut ptr: PCRE2_SPTR16,
    mut type_0: uint32_t,
    mut endptr: PCRE2_SPTR16,
    mut lenptr: *mut uint32_t,
    mut utf: BOOL,
) -> BOOL {
    let mut c: uint32_t = 0;
    if utf != 0 {
        c = *ptr as uint32_t;
        if c & 0xfc00 as uint32_t == 0xd800 as uint32_t {
            c = ((c & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                | *ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t & 0x3ff as uint32_t)
                .wrapping_add(0x10000 as uint32_t);
        }
    } else {
        c = *ptr as uint32_t;
    }
    if type_0 == NLTYPE_ANYCRLF as uint32_t {
        match c {
            10 => {
                *lenptr = 1 as uint32_t;
                return TRUE;
            }
            13 => {
                *lenptr = (if ptr < endptr.offset(-(1 as ::core::ffi::c_int as isize))
                    && *ptr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == CHAR_LF
                {
                    2 as ::core::ffi::c_int
                } else {
                    1 as ::core::ffi::c_int
                }) as uint32_t;
                return TRUE;
            }
            _ => return FALSE,
        }
    } else {
        match c {
            10 | 11 | 12 => {
                *lenptr = 1 as uint32_t;
                return TRUE;
            }
            13 => {
                *lenptr = (if ptr < endptr.offset(-(1 as ::core::ffi::c_int as isize))
                    && *ptr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == CHAR_LF
                {
                    2 as ::core::ffi::c_int
                } else {
                    1 as ::core::ffi::c_int
                }) as uint32_t;
                return TRUE;
            }
            133 | 8232 | 8233 => {
                *lenptr = 1 as uint32_t;
                return TRUE;
            }
            _ => return FALSE,
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_was_newline_16(
    mut ptr: PCRE2_SPTR16,
    mut type_0: uint32_t,
    mut startptr: PCRE2_SPTR16,
    mut lenptr: *mut uint32_t,
    mut utf: BOOL,
) -> BOOL {
    let mut c: uint32_t = 0;
    ptr = ptr.offset(-1);
    if utf != 0 {
        if *ptr as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
            == 0xdc00 as ::core::ffi::c_uint
        {
            ptr = ptr.offset(-1);
        }
        c = *ptr as uint32_t;
        if c & 0xfc00 as uint32_t == 0xd800 as uint32_t {
            c = ((c & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                | *ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t & 0x3ff as uint32_t)
                .wrapping_add(0x10000 as uint32_t);
        }
    } else {
        c = *ptr as uint32_t;
    }
    if type_0 == NLTYPE_ANYCRLF as uint32_t {
        match c {
            10 => {
                *lenptr = (if ptr > startptr
                    && *ptr.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                        == CHAR_CR
                {
                    2 as ::core::ffi::c_int
                } else {
                    1 as ::core::ffi::c_int
                }) as uint32_t;
                return TRUE;
            }
            13 => {
                *lenptr = 1 as uint32_t;
                return TRUE;
            }
            _ => return FALSE,
        }
    } else {
        match c {
            10 => {
                *lenptr = (if ptr > startptr
                    && *ptr.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                        == CHAR_CR
                {
                    2 as ::core::ffi::c_int
                } else {
                    1 as ::core::ffi::c_int
                }) as uint32_t;
                return TRUE;
            }
            11 | 12 | 13 => {
                *lenptr = 1 as uint32_t;
                return TRUE;
            }
            133 | 8232 | 8233 => {
                *lenptr = 1 as uint32_t;
                return TRUE;
            }
            _ => return FALSE,
        }
    };
}
