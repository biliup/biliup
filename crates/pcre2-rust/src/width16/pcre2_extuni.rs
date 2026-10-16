// Translated from PCRE2 10.49 pcre2_extuni.c; code unit width 16.
// /*************************************************
// *      Perl-Compatible Regular Expressions       *
// *************************************************/
//
// /* PCRE is a library of functions to support regular expressions whose syntax
// and semantics are as close as possible to those of the Perl 5 language.
//
//                        Written by Philip Hazel
//      Original API code Copyright (c) 1997-2012 University of Cambridge
//           New API code Copyright (c) 2016-2024 University of Cambridge
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
// /* This module contains an internal function that is used to match a Unicode
// extended grapheme sequence. It is used by both pcre2_match() and
// pcre2_dfa_match(). However, it is called only when Unicode support is being
// compiled. Nevertheless, we provide a dummy function when there is no Unicode
// support, because some compilers do not like functionless source files. */
extern "C" {
    static _pcre2_ucd_records_16: [ucd_record; 0];
    static _pcre2_ucd_stage1_16: [uint16_t; 0];
    static _pcre2_ucd_stage2_16: [uint16_t; 0];
    static _pcre2_ucp_gbtable_16: [uint32_t; 0];
}
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type int32_t = __int32_t;
pub type BOOL = ::core::ffi::c_int;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR16 = uint16_t;
pub type PCRE2_SPTR16 = *const PCRE2_UCHAR16;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const ucp_gbExtended_Pictographic: C2RustUnnamed = 14;
pub const ucp_gbZWJ: C2RustUnnamed = 13;
pub const ucp_gbOther: C2RustUnnamed = 12;
pub const ucp_gbRegional_Indicator: C2RustUnnamed = 11;
pub const ucp_gbLVT: C2RustUnnamed = 10;
pub const ucp_gbLV: C2RustUnnamed = 9;
pub const ucp_gbT: C2RustUnnamed = 8;
pub const ucp_gbV: C2RustUnnamed = 7;
pub const ucp_gbL: C2RustUnnamed = 6;
pub const ucp_gbSpacingMark: C2RustUnnamed = 5;
pub const ucp_gbPrepend: C2RustUnnamed = 4;
pub const ucp_gbExtend: C2RustUnnamed = 3;
pub const ucp_gbControl: C2RustUnnamed = 2;
pub const ucp_gbLF: C2RustUnnamed = 1;
pub const ucp_gbCR: C2RustUnnamed = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ucd_record {
    pub script: uint8_t,
    pub chartype: uint8_t,
    pub gbprop: uint8_t,
    pub caseset: uint8_t,
    pub other_case: int32_t,
    pub scriptx_bidiclass: uint16_t,
    pub bprops: uint16_t,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn _pcre2_extuni_16(
    mut c: uint32_t,
    mut eptr: PCRE2_SPTR16,
    mut check_subject: PCRE2_SPTR16,
    mut end_subject: PCRE2_SPTR16,
    mut utf: BOOL,
    mut xcount: *mut ::core::ffi::c_int,
) -> PCRE2_SPTR16 {
    let mut was_ep_ZWJ: BOOL = FALSE;
    let mut lgb: ::core::ffi::c_int = (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
        .offset(
            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                    .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                    as ::core::ffi::c_int
                    * UCD_BLOCK_SIZE
                    + c as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
            ) as ::core::ffi::c_int as isize,
        ))
    .gbprop as ::core::ffi::c_int;
    while eptr < end_subject {
        let mut rgb: ::core::ffi::c_int = 0;
        let mut len: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
        if utf == 0 {
            c = *eptr as uint32_t;
        } else {
            c = *eptr as uint32_t;
            if c & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                c = ((c & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                    | *eptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                        & 0x3ff as uint32_t)
                    .wrapping_add(0x10000 as uint32_t);
                len += 1;
            }
        }
        rgb = (*(&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                    .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                    as ::core::ffi::c_int
                    * UCD_BLOCK_SIZE
                    + c as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
            ) as ::core::ffi::c_int as isize,
        ))
        .gbprop as ::core::ffi::c_int;
        if *(&raw const _pcre2_ucp_gbtable_16 as *const uint32_t).offset(lgb as isize)
            & (1 as uint32_t) << rgb
            == 0 as uint32_t
        {
            break;
        }
        if lgb == ucp_gbZWJ as ::core::ffi::c_int
            && rgb == ucp_gbExtended_Pictographic as ::core::ffi::c_int
            && was_ep_ZWJ == 0
        {
            break;
        }
        if lgb == ucp_gbRegional_Indicator as ::core::ffi::c_int
            && rgb == ucp_gbRegional_Indicator as ::core::ffi::c_int
        {
            let mut ricount: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            let mut bptr: PCRE2_SPTR16 = eptr.offset(-(1 as ::core::ffi::c_int as isize));
            if utf != 0 {
                if *bptr as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                    == 0xdc00 as ::core::ffi::c_uint
                {
                    bptr = bptr.offset(-1);
                }
            }
            while bptr > check_subject {
                bptr = bptr.offset(-1);
                if utf != 0 {
                    if *bptr as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                        == 0xdc00 as ::core::ffi::c_uint
                    {
                        bptr = bptr.offset(-1);
                    }
                    c = *bptr as uint32_t;
                    if c & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                        c = ((c & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                            | *bptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3ff as uint32_t)
                            .wrapping_add(0x10000 as uint32_t);
                    }
                } else {
                    c = *bptr as uint32_t;
                }
                if (*(&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
                    *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                        (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                            .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                            as ::core::ffi::c_int
                            * UCD_BLOCK_SIZE
                            + c as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                            as isize,
                    ) as ::core::ffi::c_int as isize,
                ))
                .gbprop as ::core::ffi::c_int
                    != ucp_gbRegional_Indicator as ::core::ffi::c_int
                {
                    break;
                }
                ricount += 1;
            }
            if ricount & 1 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                break;
            }
        }
        was_ep_ZWJ = (lgb == ucp_gbExtended_Pictographic as ::core::ffi::c_int
            && rgb == ucp_gbZWJ as ::core::ffi::c_int) as ::core::ffi::c_int
            as BOOL;
        if rgb != ucp_gbExtend as ::core::ffi::c_int
            || lgb != ucp_gbExtended_Pictographic as ::core::ffi::c_int
        {
            lgb = rgb;
        }
        eptr = eptr.offset(len as isize);
        if !xcount.is_null() {
            *xcount += 1 as ::core::ffi::c_int;
        }
    }
    return eptr;
}
