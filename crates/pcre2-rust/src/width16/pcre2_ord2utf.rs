// Translated from PCRE2 10.49 pcre2_ord2utf.c; code unit width 16.
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
// /* This file contains a function that converts a Unicode character code point
// into a UTF string. The behaviour is different for each code unit width. */
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR16 = uint16_t;
#[no_mangle]
pub unsafe extern "C" fn _pcre2_ord2utf_16(
    mut cvalue: uint32_t,
    mut buffer: *mut PCRE2_UCHAR16,
) -> ::core::ffi::c_uint {
    if cvalue <= 0xffff as uint32_t {
        *buffer = cvalue as PCRE2_UCHAR16;
        return 1 as ::core::ffi::c_uint;
    }
    cvalue = cvalue.wrapping_sub(0x10000 as uint32_t);
    let fresh0 = buffer;
    buffer = buffer.offset(1);
    *fresh0 = (0xd800 as uint32_t | cvalue >> 10 as ::core::ffi::c_int) as PCRE2_UCHAR16;
    *buffer = (0xdc00 as uint32_t | cvalue & 0x3ff as uint32_t) as PCRE2_UCHAR16;
    return 2 as ::core::ffi::c_uint;
}
