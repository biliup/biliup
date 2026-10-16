// Translated from PCRE2 10.49 pcre2_valid_utf.c; code unit width 32.
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
pub type __uint32_t = u32;
pub type size_t = usize;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR32 = uint32_t;
pub type PCRE2_SPTR32 = *const PCRE2_UCHAR32;
pub const PCRE2_ERROR_UTF32_ERR1: ::core::ffi::c_int = -(27 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF32_ERR2: ::core::ffi::c_int = -(28 as ::core::ffi::c_int);
#[no_mangle]
pub unsafe extern "C" fn _pcre2_valid_utf_32(
    mut string: PCRE2_SPTR32,
    mut length: size_t,
    mut erroroffset: *mut size_t,
) -> ::core::ffi::c_int {
    let mut p: PCRE2_SPTR32 = ::core::ptr::null::<PCRE2_UCHAR32>();
    let mut c: uint32_t = 0;
    p = string;
    while length > 0 as size_t {
        c = *p as uint32_t;
        if c & 0xfffff800 as uint32_t != 0xd800 as uint32_t {
            if c > 0x10ffff as uint32_t {
                *erroroffset = p.offset_from(string) as isize as size_t;
                return PCRE2_ERROR_UTF32_ERR2;
            }
        } else {
            *erroroffset = p.offset_from(string) as isize as size_t;
            return PCRE2_ERROR_UTF32_ERR1;
        }
        length = length.wrapping_sub(1);
        p = p.offset(1);
    }
    return 0 as ::core::ffi::c_int;
}
