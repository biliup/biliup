// Translated from PCRE2 10.49 pcre2_string_utils.c; code unit width 16.
// /*************************************************
// *      Perl-Compatible Regular Expressions       *
// *************************************************/
//
// /* PCRE is a library of functions to support regular expressions whose syntax
// and semantics are as close as possible to those of the Perl 5 language.
//
//                        Written by Philip Hazel
//      Original API code Copyright (c) 1997-2012 University of Cambridge
//           New API code Copyright (c) 2018-2021 University of Cambridge
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
// /* This module contains internal functions for comparing and finding the length
// of strings. These are used instead of strcmp() etc because the standard
// functions work only on 8-bit data. */
pub type __uint16_t = u16;
pub type size_t = usize;
pub type uint16_t = __uint16_t;
pub type PCRE2_UCHAR16 = uint16_t;
pub type PCRE2_SPTR16 = *const PCRE2_UCHAR16;
#[no_mangle]
pub unsafe extern "C" fn _pcre2_strcmp_16(
    mut str1: PCRE2_SPTR16,
    mut str2: PCRE2_SPTR16,
) -> ::core::ffi::c_int {
    let mut c1: PCRE2_UCHAR16 = 0;
    let mut c2: PCRE2_UCHAR16 = 0;
    while *str1 as ::core::ffi::c_int != '\0' as i32 || *str2 as ::core::ffi::c_int != '\0' as i32 {
        let fresh0 = str1;
        str1 = str1.offset(1);
        c1 = *fresh0;
        let fresh1 = str2;
        str2 = str2.offset(1);
        c2 = *fresh1;
        if c1 as ::core::ffi::c_int != c2 as ::core::ffi::c_int {
            return (((c1 as ::core::ffi::c_int > c2 as ::core::ffi::c_int) as ::core::ffi::c_int)
                << 1 as ::core::ffi::c_int)
                - 1 as ::core::ffi::c_int;
        }
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_strcmp_c8_16(
    mut str1: PCRE2_SPTR16,
    mut str2: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut c1: PCRE2_UCHAR16 = 0;
    let mut c2: PCRE2_UCHAR16 = 0;
    while *str1 as ::core::ffi::c_int != '\0' as i32 || *str2 as ::core::ffi::c_int != '\0' as i32 {
        let fresh2 = str1;
        str1 = str1.offset(1);
        c1 = *fresh2;
        let fresh3 = str2;
        str2 = str2.offset(1);
        c2 = *fresh3 as PCRE2_UCHAR16;
        if c1 as ::core::ffi::c_int != c2 as ::core::ffi::c_int {
            return (((c1 as ::core::ffi::c_int > c2 as ::core::ffi::c_int) as ::core::ffi::c_int)
                << 1 as ::core::ffi::c_int)
                - 1 as ::core::ffi::c_int;
        }
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_strncmp_16(
    mut str1: PCRE2_SPTR16,
    mut str2: PCRE2_SPTR16,
    mut len: size_t,
) -> ::core::ffi::c_int {
    let mut c1: PCRE2_UCHAR16 = 0;
    let mut c2: PCRE2_UCHAR16 = 0;
    while len > 0 as size_t {
        let fresh7 = str1;
        str1 = str1.offset(1);
        c1 = *fresh7;
        let fresh8 = str2;
        str2 = str2.offset(1);
        c2 = *fresh8;
        if c1 as ::core::ffi::c_int != c2 as ::core::ffi::c_int {
            return (((c1 as ::core::ffi::c_int > c2 as ::core::ffi::c_int) as ::core::ffi::c_int)
                << 1 as ::core::ffi::c_int)
                - 1 as ::core::ffi::c_int;
        }
        len = len.wrapping_sub(1);
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_strncmp_c8_16(
    mut str1: PCRE2_SPTR16,
    mut str2: *const ::core::ffi::c_char,
    mut len: size_t,
) -> ::core::ffi::c_int {
    let mut c1: PCRE2_UCHAR16 = 0;
    let mut c2: PCRE2_UCHAR16 = 0;
    while len > 0 as size_t {
        let fresh9 = str1;
        str1 = str1.offset(1);
        c1 = *fresh9;
        let fresh10 = str2;
        str2 = str2.offset(1);
        c2 = *fresh10 as PCRE2_UCHAR16;
        if c1 as ::core::ffi::c_int != c2 as ::core::ffi::c_int {
            return (((c1 as ::core::ffi::c_int > c2 as ::core::ffi::c_int) as ::core::ffi::c_int)
                << 1 as ::core::ffi::c_int)
                - 1 as ::core::ffi::c_int;
        }
        len = len.wrapping_sub(1);
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_strlen_16(mut str: PCRE2_SPTR16) -> size_t {
    let mut c: size_t = 0 as size_t;
    loop {
        let fresh6 = str;
        str = str.offset(1);
        if !(*fresh6 as ::core::ffi::c_int != 0 as ::core::ffi::c_int) {
            break;
        }
        c = c.wrapping_add(1);
    }
    return c;
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_strcpy_c8_16(
    mut str1: *mut PCRE2_UCHAR16,
    mut str2: *const ::core::ffi::c_char,
) -> size_t {
    let mut t: *mut PCRE2_UCHAR16 = str1;
    while *str2 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        let fresh4 = str2;
        str2 = str2.offset(1);
        let fresh5 = t;
        t = t.offset(1);
        *fresh5 = *fresh4 as PCRE2_UCHAR16;
    }
    *t = 0 as PCRE2_UCHAR16;
    return t.offset_from(str1) as isize as size_t;
}
