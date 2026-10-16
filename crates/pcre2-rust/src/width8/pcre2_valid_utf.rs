// Translated from PCRE2 10.49 pcre2_valid_utf.c; code unit width 8.
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
//
//
// /* This module contains an internal function for validating UTF character
// strings. This file is also #included by the pcre2test program, which uses
// macros to change names from _pcre2_xxx to xxxx, thereby avoiding name clashes
// with the library. In this case, PCRE2_PCRE2TEST is defined. */
extern "C" {
    static _pcre2_utf8_table4: [uint8_t; 0];
}
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type size_t = usize;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR8 = uint8_t;
pub type PCRE2_SPTR8 = *const PCRE2_UCHAR8;
pub const PCRE2_ERROR_UTF8_ERR1: ::core::ffi::c_int = -(3 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR2: ::core::ffi::c_int = -(4 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR3: ::core::ffi::c_int = -(5 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR4: ::core::ffi::c_int = -(6 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR5: ::core::ffi::c_int = -(7 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR6: ::core::ffi::c_int = -(8 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR7: ::core::ffi::c_int = -(9 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR8: ::core::ffi::c_int = -(10 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR9: ::core::ffi::c_int = -(11 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR10: ::core::ffi::c_int = -(12 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR11: ::core::ffi::c_int = -(13 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR12: ::core::ffi::c_int = -(14 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR13: ::core::ffi::c_int = -(15 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR14: ::core::ffi::c_int = -(16 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR15: ::core::ffi::c_int = -(17 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR16: ::core::ffi::c_int = -(18 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR17: ::core::ffi::c_int = -(19 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR18: ::core::ffi::c_int = -(20 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR19: ::core::ffi::c_int = -(21 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR20: ::core::ffi::c_int = -(22 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR21: ::core::ffi::c_int = -(23 as ::core::ffi::c_int);
#[no_mangle]
pub unsafe extern "C" fn _pcre2_valid_utf_8(
    mut string: PCRE2_SPTR8,
    mut length: size_t,
    mut erroroffset: *mut size_t,
) -> ::core::ffi::c_int {
    let mut p: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut c: uint32_t = 0;
    p = string;
    while length > 0 as size_t {
        let mut ab: uint32_t = 0;
        let mut d: uint32_t = 0;
        c = *p as uint32_t;
        length = length.wrapping_sub(1);
        if !(c < 128 as uint32_t) {
            if c < 0xc0 as uint32_t {
                *erroroffset = p.offset_from(string) as isize as size_t;
                return PCRE2_ERROR_UTF8_ERR20;
            }
            if c >= 0xfe as uint32_t {
                *erroroffset = p.offset_from(string) as isize as size_t;
                return PCRE2_ERROR_UTF8_ERR21;
            }
            ab = *(&raw const _pcre2_utf8_table4 as *const uint8_t)
                .offset((c & 0x3f as uint32_t) as isize) as uint32_t;
            if length < ab as size_t {
                *erroroffset = p.offset_from(string) as isize as size_t;
                match (ab as size_t).wrapping_sub(length) {
                    1 => return PCRE2_ERROR_UTF8_ERR1,
                    2 => return PCRE2_ERROR_UTF8_ERR2,
                    3 => return PCRE2_ERROR_UTF8_ERR3,
                    4 => return PCRE2_ERROR_UTF8_ERR4,
                    5 => return PCRE2_ERROR_UTF8_ERR5,
                    _ => {}
                }
            }
            length = length.wrapping_sub(ab as size_t);
            p = p.offset(1);
            d = *p as uint32_t;
            if d & 0xc0 as uint32_t != 0x80 as uint32_t {
                *erroroffset = (p.offset_from(string) as isize as size_t)
                    .wrapping_sub(1 as size_t);
                return PCRE2_ERROR_UTF8_ERR6;
            }
            match ab {
                1 => {
                    if c & 0x3e as uint32_t == 0 as uint32_t {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(1 as size_t);
                        return PCRE2_ERROR_UTF8_ERR15;
                    }
                }
                2 => {
                    p = p.offset(1);
                    if *p as ::core::ffi::c_int & 0xc0 as ::core::ffi::c_int
                        != 0x80 as ::core::ffi::c_int
                    {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(2 as size_t);
                        return PCRE2_ERROR_UTF8_ERR7;
                    }
                    if c == 0xe0 as uint32_t && d & 0x20 as uint32_t == 0 as uint32_t {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(2 as size_t);
                        return PCRE2_ERROR_UTF8_ERR16;
                    }
                    if c == 0xed as uint32_t && d >= 0xa0 as uint32_t {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(2 as size_t);
                        return PCRE2_ERROR_UTF8_ERR14;
                    }
                }
                3 => {
                    p = p.offset(1);
                    if *p as ::core::ffi::c_int & 0xc0 as ::core::ffi::c_int
                        != 0x80 as ::core::ffi::c_int
                    {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(2 as size_t);
                        return PCRE2_ERROR_UTF8_ERR7;
                    }
                    p = p.offset(1);
                    if *p as ::core::ffi::c_int & 0xc0 as ::core::ffi::c_int
                        != 0x80 as ::core::ffi::c_int
                    {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(3 as size_t);
                        return PCRE2_ERROR_UTF8_ERR8;
                    }
                    if c == 0xf0 as uint32_t && d & 0x30 as uint32_t == 0 as uint32_t {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(3 as size_t);
                        return PCRE2_ERROR_UTF8_ERR17;
                    }
                    if c > 0xf4 as uint32_t || c == 0xf4 as uint32_t && d > 0x8f as uint32_t {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(3 as size_t);
                        return PCRE2_ERROR_UTF8_ERR13;
                    }
                }
                4 => {
                    p = p.offset(1);
                    if *p as ::core::ffi::c_int & 0xc0 as ::core::ffi::c_int
                        != 0x80 as ::core::ffi::c_int
                    {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(2 as size_t);
                        return PCRE2_ERROR_UTF8_ERR7;
                    }
                    p = p.offset(1);
                    if *p as ::core::ffi::c_int & 0xc0 as ::core::ffi::c_int
                        != 0x80 as ::core::ffi::c_int
                    {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(3 as size_t);
                        return PCRE2_ERROR_UTF8_ERR8;
                    }
                    p = p.offset(1);
                    if *p as ::core::ffi::c_int & 0xc0 as ::core::ffi::c_int
                        != 0x80 as ::core::ffi::c_int
                    {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(4 as size_t);
                        return PCRE2_ERROR_UTF8_ERR9;
                    }
                    if c == 0xf8 as uint32_t && d & 0x38 as uint32_t == 0 as uint32_t {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(4 as size_t);
                        return PCRE2_ERROR_UTF8_ERR18;
                    }
                }
                5 => {
                    p = p.offset(1);
                    if *p as ::core::ffi::c_int & 0xc0 as ::core::ffi::c_int
                        != 0x80 as ::core::ffi::c_int
                    {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(2 as size_t);
                        return PCRE2_ERROR_UTF8_ERR7;
                    }
                    p = p.offset(1);
                    if *p as ::core::ffi::c_int & 0xc0 as ::core::ffi::c_int
                        != 0x80 as ::core::ffi::c_int
                    {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(3 as size_t);
                        return PCRE2_ERROR_UTF8_ERR8;
                    }
                    p = p.offset(1);
                    if *p as ::core::ffi::c_int & 0xc0 as ::core::ffi::c_int
                        != 0x80 as ::core::ffi::c_int
                    {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(4 as size_t);
                        return PCRE2_ERROR_UTF8_ERR9;
                    }
                    p = p.offset(1);
                    if *p as ::core::ffi::c_int & 0xc0 as ::core::ffi::c_int
                        != 0x80 as ::core::ffi::c_int
                    {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(5 as size_t);
                        return PCRE2_ERROR_UTF8_ERR10;
                    }
                    if c == 0xfc as uint32_t && d & 0x3c as uint32_t == 0 as uint32_t {
                        *erroroffset = (p.offset_from(string) as isize as size_t)
                            .wrapping_sub(5 as size_t);
                        return PCRE2_ERROR_UTF8_ERR19;
                    }
                }
                _ => {}
            }
            if ab > 3 as uint32_t {
                *erroroffset = (p.offset_from(string) as isize as size_t)
                    .wrapping_sub(ab as size_t);
                return if ab == 4 as uint32_t {
                    PCRE2_ERROR_UTF8_ERR11
                } else {
                    PCRE2_ERROR_UTF8_ERR12
                };
            }
        }
        p = p.offset(1);
    }
    return 0 as ::core::ffi::c_int;
}
