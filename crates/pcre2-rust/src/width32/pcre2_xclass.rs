// Translated from PCRE2 10.49 pcre2_xclass.c; code unit width 32.
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
// /* This module contains two internal functions that are used to match
// OP_XCLASS and OP_ECLASS. It is used by pcre2_auto_possessify() and by both
// pcre2_match() and pcre2_dfa_match(). */
extern "C" {
    static _pcre2_ucd_boolprop_sets_32: [uint32_t; 0];
    static _pcre2_ucd_script_sets_32: [uint32_t; 0];
    static _pcre2_ucd_records_32: [ucd_record; 0];
    static _pcre2_dummy_ucd_record_32: [ucd_record; 0];
    static _pcre2_ucd_stage1_32: [uint16_t; 0];
    static _pcre2_ucd_stage2_32: [uint16_t; 0];
    static _pcre2_ucp_gentype_32: [uint32_t; 0];
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
pub type PCRE2_UCHAR32 = uint32_t;
pub type PCRE2_SPTR32 = *const PCRE2_UCHAR32;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const ucp_Z: C2RustUnnamed = 6;
pub const ucp_S: C2RustUnnamed = 5;
pub const ucp_P: C2RustUnnamed = 4;
pub const ucp_N: C2RustUnnamed = 3;
pub const ucp_M: C2RustUnnamed = 2;
pub const ucp_L: C2RustUnnamed = 1;
pub const ucp_C: C2RustUnnamed = 0;
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const ucp_Zs: C2RustUnnamed_0 = 29;
pub const ucp_Zp: C2RustUnnamed_0 = 28;
pub const ucp_Zl: C2RustUnnamed_0 = 27;
pub const ucp_So: C2RustUnnamed_0 = 26;
pub const ucp_Sm: C2RustUnnamed_0 = 25;
pub const ucp_Sk: C2RustUnnamed_0 = 24;
pub const ucp_Sc: C2RustUnnamed_0 = 23;
pub const ucp_Ps: C2RustUnnamed_0 = 22;
pub const ucp_Po: C2RustUnnamed_0 = 21;
pub const ucp_Pi: C2RustUnnamed_0 = 20;
pub const ucp_Pf: C2RustUnnamed_0 = 19;
pub const ucp_Pe: C2RustUnnamed_0 = 18;
pub const ucp_Pd: C2RustUnnamed_0 = 17;
pub const ucp_Pc: C2RustUnnamed_0 = 16;
pub const ucp_No: C2RustUnnamed_0 = 15;
pub const ucp_Nl: C2RustUnnamed_0 = 14;
pub const ucp_Nd: C2RustUnnamed_0 = 13;
pub const ucp_Mn: C2RustUnnamed_0 = 12;
pub const ucp_Me: C2RustUnnamed_0 = 11;
pub const ucp_Mc: C2RustUnnamed_0 = 10;
pub const ucp_Lu: C2RustUnnamed_0 = 9;
pub const ucp_Lt: C2RustUnnamed_0 = 8;
pub const ucp_Lo: C2RustUnnamed_0 = 7;
pub const ucp_Lm: C2RustUnnamed_0 = 6;
pub const ucp_Ll: C2RustUnnamed_0 = 5;
pub const ucp_Cs: C2RustUnnamed_0 = 4;
pub const ucp_Co: C2RustUnnamed_0 = 3;
pub const ucp_Cn: C2RustUnnamed_0 = 2;
pub const ucp_Cf: C2RustUnnamed_0 = 1;
pub const ucp_Cc: C2RustUnnamed_0 = 0;
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
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const MAX_UTF_CODE_POINT: ::core::ffi::c_int = 0x10ffff as ::core::ffi::c_int;
pub const CHAR_HT: uint32_t = 9 as uint32_t;
pub const CHAR_VT: uint32_t = 11 as uint32_t;
pub const CHAR_FF: uint32_t = 12 as uint32_t;
pub const CHAR_CR: uint32_t = 13 as uint32_t;
pub const CHAR_LF: uint32_t = 10 as uint32_t;
pub const CHAR_NEL: uint32_t = 133 as uint32_t;
pub const CHAR_SPACE: uint32_t = 32 as uint32_t;
pub const CHAR_DOLLAR_SIGN: ::core::ffi::c_int = '$' as i32;
pub const CHAR_0: ::core::ffi::c_int = '0' as i32;
pub const CHAR_9: ::core::ffi::c_int = '9' as i32;
pub const CHAR_COMMERCIAL_AT: ::core::ffi::c_int = '@' as i32;
pub const CHAR_A: ::core::ffi::c_int = 'A' as i32;
pub const CHAR_F: ::core::ffi::c_int = 'F' as i32;
pub const CHAR_GRAVE_ACCENT: ::core::ffi::c_int = '`' as i32;
pub const CHAR_a: ::core::ffi::c_int = 'a' as i32;
pub const CHAR_f: ::core::ffi::c_int = 'f' as i32;
pub const CHAR_NBSP: uint32_t = 160 as uint32_t;
pub const PT_LAMP: PCRE2_UCHAR32 = 0 as PCRE2_UCHAR32;
pub const PT_GC: PCRE2_UCHAR32 = 1 as PCRE2_UCHAR32;
pub const PT_PC: PCRE2_UCHAR32 = 2 as PCRE2_UCHAR32;
pub const PT_SC: PCRE2_UCHAR32 = 3 as PCRE2_UCHAR32;
pub const PT_SCX: PCRE2_UCHAR32 = 4 as PCRE2_UCHAR32;
pub const PT_ALNUM: PCRE2_UCHAR32 = 5 as PCRE2_UCHAR32;
pub const PT_SPACE: PCRE2_UCHAR32 = 6 as PCRE2_UCHAR32;
pub const PT_PXSPACE: PCRE2_UCHAR32 = 7 as PCRE2_UCHAR32;
pub const PT_WORD: PCRE2_UCHAR32 = 8 as PCRE2_UCHAR32;
pub const PT_UCNC: PCRE2_UCHAR32 = 10 as PCRE2_UCHAR32;
pub const PT_BIDICL: PCRE2_UCHAR32 = 11 as PCRE2_UCHAR32;
pub const PT_BOOL: PCRE2_UCHAR32 = 12 as PCRE2_UCHAR32;
pub const PT_PXGRAPH: PCRE2_UCHAR32 = 14 as PCRE2_UCHAR32;
pub const PT_PXPRINT: PCRE2_UCHAR32 = 15 as PCRE2_UCHAR32;
pub const PT_PXPUNCT: PCRE2_UCHAR32 = 16 as PCRE2_UCHAR32;
pub const PT_PXXDIGIT: PCRE2_UCHAR32 = 17 as PCRE2_UCHAR32;
pub const XCL_NOT: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const XCL_MAP: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const XCL_END: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const XCL_SINGLE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const XCL_PROP: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const XCL_NOTPROP: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_HIGH_16_START: ::core::ffi::c_int = 0x8000 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_LOW_32_START: ::core::ffi::c_int = 0x10000 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_HIGH_32_START: ::core::ffi::c_uint = 0x80000000 as ::core::ffi::c_uint;
pub const XCL_TYPE_MASK: ::core::ffi::c_int = 0xfff as ::core::ffi::c_int;
pub const XCL_TYPE_BIT_LEN: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const XCL_BEGIN_WITH_RANGE: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const XCL_ITEM_COUNT_MASK: ::core::ffi::c_int = 0x3 as ::core::ffi::c_int;
pub const XCL_CHAR_END: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const XCL_CHAR_SHIFT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ECL_MAP: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const ECL_AND: PCRE2_UCHAR32 = 1 as PCRE2_UCHAR32;
pub const ECL_OR: PCRE2_UCHAR32 = 2 as PCRE2_UCHAR32;
pub const ECL_XOR: PCRE2_UCHAR32 = 3 as PCRE2_UCHAR32;
pub const ECL_NOT: PCRE2_UCHAR32 = 4 as PCRE2_UCHAR32;
pub const ECL_XCLASS: PCRE2_UCHAR32 = 5 as PCRE2_UCHAR32;
pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const UCD_BIDICLASS_SHIFT: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const LINK_SIZE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn _pcre2_xclass_32(
    mut c: uint32_t,
    mut data: PCRE2_SPTR32,
    mut char_lists_end: *const uint8_t,
    mut utf: BOOL,
) -> BOOL {
    let mut t: PCRE2_UCHAR32 = 0;
    let mut not_negated: BOOL =
        (*data & XCL_NOT as PCRE2_UCHAR32 == 0 as PCRE2_UCHAR32) as ::core::ffi::c_int;
    let mut type_0: uint32_t = 0;
    let mut max_index: uint32_t = 0;
    let mut min_index: uint32_t = 0;
    let mut value: uint32_t = 0;
    let mut next_char: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let fresh0 = data;
    data = data.offset(1);
    if *fresh0 & XCL_MAP as PCRE2_UCHAR32 != 0 as PCRE2_UCHAR32 {
        if c < 256 as uint32_t {
            return (*(data as *const uint8_t).offset(c.wrapping_div(8 as uint32_t) as isize)
                as ::core::ffi::c_uint
                & (1 as ::core::ffi::c_uint) << (c & 7 as uint32_t)
                != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int;
        }
        data = data.offset(
            (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR32>() as usize) as isize,
        );
    }
    if *data == XCL_PROP as PCRE2_UCHAR32 || *data == XCL_NOTPROP as PCRE2_UCHAR32 {
        let mut prop: *const ucd_record = if c > MAX_UTF_CODE_POINT as uint32_t {
            &raw const _pcre2_dummy_ucd_record_32 as *const ucd_record
        } else {
            (&raw const _pcre2_ucd_records_32 as *const ucd_record).offset(
                *(&raw const _pcre2_ucd_stage2_32 as *const uint16_t).offset(
                    (*(&raw const _pcre2_ucd_stage1_32 as *const uint16_t)
                        .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                        as ::core::ffi::c_int
                        * UCD_BLOCK_SIZE
                        + c as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                ) as ::core::ffi::c_int as isize,
            )
        };
        loop {
            let mut chartype: ::core::ffi::c_int = 0;
            let fresh1 = data;
            data = data.offset(1);
            let mut isprop: BOOL = (*fresh1 == XCL_PROP as PCRE2_UCHAR32) as ::core::ffi::c_int;
            let mut ok: BOOL = 0;
            match *data {
                0 => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (chartype == ucp_Lu as ::core::ffi::c_int
                        || chartype == ucp_Ll as ::core::ffi::c_int
                        || chartype == ucp_Lt as ::core::ffi::c_int)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                1 => {
                    if (*data.offset(1 as ::core::ffi::c_int as isize)
                        == *(&raw const _pcre2_ucp_gentype_32 as *const uint32_t)
                            .offset((*prop).chartype as isize))
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                2 => {
                    if (*data.offset(1 as ::core::ffi::c_int as isize)
                        == (*prop).chartype as PCRE2_UCHAR32)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                3 => {
                    if (*data.offset(1 as ::core::ffi::c_int as isize)
                        == (*prop).script as PCRE2_UCHAR32)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                4 => {
                    ok = (*data.offset(1 as ::core::ffi::c_int as isize)
                        == (*prop).script as PCRE2_UCHAR32
                        || *(&raw const _pcre2_ucd_script_sets_32 as *const uint32_t)
                            .offset(
                                ((*prop).scriptx_bidiclass as ::core::ffi::c_int
                                    & 0x3ff as ::core::ffi::c_int)
                                    as isize,
                            )
                            .offset(
                                (*data.offset(1 as ::core::ffi::c_int as isize))
                                    .wrapping_div(32 as PCRE2_UCHAR32)
                                    as isize,
                            )
                            & (1 as uint32_t)
                                << (*data.offset(1 as ::core::ffi::c_int as isize))
                                    .wrapping_rem(32 as PCRE2_UCHAR32)
                            != 0 as uint32_t) as ::core::ffi::c_int
                        as BOOL;
                    if ok == isprop {
                        return not_negated;
                    }
                }
                5 => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_32 as *const uint32_t)
                        .offset(chartype as isize)
                        == ucp_L as ::core::ffi::c_int as uint32_t
                        || *(&raw const _pcre2_ucp_gentype_32 as *const uint32_t)
                            .offset(chartype as isize)
                            == ucp_N as ::core::ffi::c_int as uint32_t)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                6 | 7 => match c {
                    9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194 | 8195 | 8196 | 8197 | 8198
                    | 8199 | 8200 | 8201 | 8202 | 8239 | 8287 | 12288 | 10 | 11 | 12 | 13 | 133
                    | 8232 | 8233 => {
                        if isprop != 0 {
                            return not_negated;
                        }
                    }
                    _ => {
                        if (*(&raw const _pcre2_ucp_gentype_32 as *const uint32_t)
                            .offset((*prop).chartype as isize)
                            == ucp_Z as ::core::ffi::c_int as uint32_t)
                            as ::core::ffi::c_int
                            == isprop
                        {
                            return not_negated;
                        }
                    }
                },
                8 => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_32 as *const uint32_t)
                        .offset(chartype as isize)
                        == ucp_L as ::core::ffi::c_int as uint32_t
                        || *(&raw const _pcre2_ucp_gentype_32 as *const uint32_t)
                            .offset(chartype as isize)
                            == ucp_N as ::core::ffi::c_int as uint32_t
                        || chartype == ucp_Mn as ::core::ffi::c_int
                        || chartype == ucp_Pc as ::core::ffi::c_int)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                10 => {
                    if c < 0xa0 as uint32_t {
                        if (c == CHAR_DOLLAR_SIGN as uint32_t
                            || c == CHAR_COMMERCIAL_AT as uint32_t
                            || c == CHAR_GRAVE_ACCENT as uint32_t)
                            as ::core::ffi::c_int
                            == isprop
                        {
                            return not_negated;
                        }
                    } else if (c < 0xd800 as uint32_t || c > 0xdfff as uint32_t)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                11 => {
                    if (((*prop).scriptx_bidiclass as ::core::ffi::c_int >> UCD_BIDICLASS_SHIFT)
                        as PCRE2_UCHAR32
                        == *data.offset(1 as ::core::ffi::c_int as isize))
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                12 => {
                    ok = (*(&raw const _pcre2_ucd_boolprop_sets_32 as *const uint32_t)
                        .offset(
                            ((*prop).bprops as ::core::ffi::c_int & 0xfff as ::core::ffi::c_int)
                                as isize,
                        )
                        .offset(
                            (*data.offset(1 as ::core::ffi::c_int as isize))
                                .wrapping_div(32 as PCRE2_UCHAR32)
                                as isize,
                        )
                        & (1 as uint32_t)
                            << (*data.offset(1 as ::core::ffi::c_int as isize))
                                .wrapping_rem(32 as PCRE2_UCHAR32)
                        != 0 as uint32_t) as ::core::ffi::c_int as BOOL;
                    if ok == isprop {
                        return not_negated;
                    }
                }
                14 => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_32 as *const uint32_t)
                        .offset(chartype as isize)
                        != ucp_Z as ::core::ffi::c_int as uint32_t
                        && (*(&raw const _pcre2_ucp_gentype_32 as *const uint32_t)
                            .offset(chartype as isize)
                            != ucp_C as ::core::ffi::c_int as uint32_t
                            || chartype == ucp_Cf as ::core::ffi::c_int
                                && c != 0x61c as uint32_t
                                && c != 0x180e as uint32_t
                                && (c < 0x2066 as uint32_t || c > 0x2069 as uint32_t)))
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                15 => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (chartype != ucp_Zl as ::core::ffi::c_int
                        && chartype != ucp_Zp as ::core::ffi::c_int
                        && (*(&raw const _pcre2_ucp_gentype_32 as *const uint32_t)
                            .offset(chartype as isize)
                            != ucp_C as ::core::ffi::c_int as uint32_t
                            || chartype == ucp_Cf as ::core::ffi::c_int
                                && c != 0x61c as uint32_t
                                && (c < 0x2066 as uint32_t || c > 0x2069 as uint32_t)))
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                16 => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_32 as *const uint32_t)
                        .offset(chartype as isize)
                        == ucp_P as ::core::ffi::c_int as uint32_t
                        || c < 128 as uint32_t
                            && *(&raw const _pcre2_ucp_gentype_32 as *const uint32_t)
                                .offset(chartype as isize)
                                == ucp_S as ::core::ffi::c_int as uint32_t)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                17 => {
                    if (c >= CHAR_0 as uint32_t && c <= CHAR_9 as uint32_t
                        || c >= CHAR_A as uint32_t && c <= CHAR_F as uint32_t
                        || c >= CHAR_a as uint32_t && c <= CHAR_f as uint32_t
                        || c >= 0xff10 as uint32_t && c <= 0xff19 as uint32_t
                        || c >= 0xff21 as uint32_t && c <= 0xff26 as uint32_t
                        || c >= 0xff41 as uint32_t && c <= 0xff46 as uint32_t)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                _ => return FALSE,
            }
            data = data.offset(2 as ::core::ffi::c_int as isize);
            if !(*data == XCL_PROP as PCRE2_UCHAR32 || *data == XCL_NOTPROP as PCRE2_UCHAR32) {
                break;
            }
        }
    }
    if *data
        < (if ::core::mem::size_of::<PCRE2_UCHAR32>() as usize == 1 as usize {
            0x10 as ::core::ffi::c_int
        } else {
            0x1000 as ::core::ffi::c_int
        }) as PCRE2_UCHAR32
    {
        loop {
            let fresh2 = data;
            data = data.offset(1);
            t = *fresh2;
            if !(t != XCL_END as PCRE2_UCHAR32) {
                break;
            }
            let mut x: uint32_t = 0;
            let mut y: uint32_t = 0;
            if utf != 0 {
                let fresh3 = data;
                data = data.offset(1);
                x = *fresh3 as uint32_t;
            } else {
                let fresh4 = data;
                data = data.offset(1);
                x = *fresh4 as uint32_t;
            }
            if t == XCL_SINGLE as PCRE2_UCHAR32 {
                if c <= x {
                    return if c == x {
                        not_negated
                    } else {
                        (not_negated == 0) as ::core::ffi::c_int
                    };
                }
            } else {
                if utf != 0 {
                    let fresh5 = data;
                    data = data.offset(1);
                    y = *fresh5 as uint32_t;
                } else {
                    let fresh6 = data;
                    data = data.offset(1);
                    y = *fresh6 as uint32_t;
                }
                if c <= y {
                    return if c >= x {
                        not_negated
                    } else {
                        (not_negated == 0) as ::core::ffi::c_int
                    };
                }
            }
        }
        return (not_negated == 0) as ::core::ffi::c_int;
    }
    type_0 = *data.offset(0 as ::core::ffi::c_int as isize) as uint32_t;
    data = data.offset(1);
    next_char = char_lists_end.offset(
        -((*data.offset(0 as ::core::ffi::c_int as isize) << 1 as ::core::ffi::c_int) as isize),
    );
    type_0 &= XCL_TYPE_MASK as uint32_t;
    if c >= XCL_CHAR_LIST_HIGH_16_START as uint32_t {
        max_index = type_0 & XCL_ITEM_COUNT_MASK as uint32_t;
        if max_index == XCL_ITEM_COUNT_MASK as uint32_t {
            max_index = *(next_char as *const uint16_t) as uint32_t;
            next_char = next_char.offset(2 as ::core::ffi::c_int as isize);
        }
        next_char = next_char.offset((max_index << 1 as ::core::ffi::c_int) as isize);
        type_0 >>= XCL_TYPE_BIT_LEN;
    }
    if c < XCL_CHAR_LIST_LOW_32_START as uint32_t {
        max_index = type_0 & XCL_ITEM_COUNT_MASK as uint32_t;
        c = (c << XCL_CHAR_SHIFT | XCL_CHAR_END as uint32_t) as uint16_t as uint32_t;
        if max_index == XCL_ITEM_COUNT_MASK as uint32_t {
            max_index = *(next_char as *const uint16_t) as uint32_t;
            next_char = next_char.offset(2 as ::core::ffi::c_int as isize);
        }
        if max_index == 0 as uint32_t || c < *(next_char as *const uint16_t) as uint32_t {
            return ((type_0 & XCL_BEGIN_WITH_RANGE as uint32_t != 0 as uint32_t)
                as ::core::ffi::c_int
                == not_negated) as ::core::ffi::c_int;
        }
        min_index = 0 as uint32_t;
        max_index = max_index.wrapping_sub(1);
        value = *(next_char as *const uint16_t).offset(max_index as isize) as uint32_t;
        if c >= value {
            return ((value == c || value & XCL_CHAR_END as uint32_t == 0 as uint32_t)
                as ::core::ffi::c_int
                == not_negated) as ::core::ffi::c_int;
        }
        max_index = max_index.wrapping_sub(1);
        loop {
            let mut mid_index: uint32_t =
                min_index.wrapping_add(max_index) >> 1 as ::core::ffi::c_int;
            value = *(next_char as *const uint16_t).offset(mid_index as isize) as uint32_t;
            if c < value {
                max_index = mid_index.wrapping_sub(1 as uint32_t);
            } else if *(next_char as *const uint16_t)
                .offset(mid_index.wrapping_add(1 as uint32_t) as isize)
                as uint32_t
                <= c
            {
                min_index = mid_index.wrapping_add(1 as uint32_t);
            } else {
                return ((value == c || value & XCL_CHAR_END as uint32_t == 0 as uint32_t)
                    as ::core::ffi::c_int
                    == not_negated) as ::core::ffi::c_int;
            }
        }
    }
    max_index = type_0 & XCL_ITEM_COUNT_MASK as uint32_t;
    if max_index == XCL_ITEM_COUNT_MASK as uint32_t {
        max_index = *(next_char as *const uint16_t) as uint32_t;
        next_char = next_char.offset(2 as ::core::ffi::c_int as isize);
    }
    next_char = next_char.offset((max_index << 1 as ::core::ffi::c_int) as isize);
    type_0 >>= XCL_TYPE_BIT_LEN;
    max_index = type_0 & XCL_ITEM_COUNT_MASK as uint32_t;
    if c >= XCL_CHAR_LIST_HIGH_32_START as uint32_t {
        if max_index == XCL_ITEM_COUNT_MASK as uint32_t {
            max_index = *(next_char as *const uint32_t);
            next_char = next_char.offset(4 as ::core::ffi::c_int as isize);
        }
        next_char = next_char.offset((max_index << 2 as ::core::ffi::c_int) as isize);
        type_0 >>= XCL_TYPE_BIT_LEN;
        max_index = type_0 & XCL_ITEM_COUNT_MASK as uint32_t;
    }
    c = c << XCL_CHAR_SHIFT | XCL_CHAR_END as uint32_t;
    if max_index == XCL_ITEM_COUNT_MASK as uint32_t {
        max_index = *(next_char as *const uint32_t);
        next_char = next_char.offset(4 as ::core::ffi::c_int as isize);
    }
    if max_index == 0 as uint32_t || c < *(next_char as *const uint32_t) {
        return ((type_0 & XCL_BEGIN_WITH_RANGE as uint32_t != 0 as uint32_t) as ::core::ffi::c_int
            == not_negated) as ::core::ffi::c_int;
    }
    min_index = 0 as uint32_t;
    max_index = max_index.wrapping_sub(1);
    value = *(next_char as *const uint32_t).offset(max_index as isize);
    if c >= value {
        return ((value == c || value & XCL_CHAR_END as uint32_t == 0 as uint32_t)
            as ::core::ffi::c_int
            == not_negated) as ::core::ffi::c_int;
    }
    max_index = max_index.wrapping_sub(1);
    loop {
        let mut mid_index_0: uint32_t =
            min_index.wrapping_add(max_index) >> 1 as ::core::ffi::c_int;
        value = *(next_char as *const uint32_t).offset(mid_index_0 as isize);
        if c < value {
            max_index = mid_index_0.wrapping_sub(1 as uint32_t);
        } else if *(next_char as *const uint32_t)
            .offset(mid_index_0.wrapping_add(1 as uint32_t) as isize)
            <= c
        {
            min_index = mid_index_0.wrapping_add(1 as uint32_t);
        } else {
            return ((value == c || value & XCL_CHAR_END as uint32_t == 0 as uint32_t)
                as ::core::ffi::c_int
                == not_negated) as ::core::ffi::c_int;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_eclass_32(
    mut c: uint32_t,
    mut data_start: PCRE2_SPTR32,
    mut data_end: PCRE2_SPTR32,
    mut char_lists_end: *const uint8_t,
    mut utf: BOOL,
) -> BOOL {
    let mut ptr: PCRE2_SPTR32 = data_start;
    let mut flags: PCRE2_UCHAR32 = 0;
    let mut stack: uint32_t = 0 as uint32_t;
    let mut stack_depth: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let fresh7 = ptr;
    ptr = ptr.offset(1);
    flags = *fresh7;
    if flags & ECL_MAP as PCRE2_UCHAR32 != 0 as PCRE2_UCHAR32 {
        if c < 256 as uint32_t {
            return (*(ptr as *const uint8_t).offset(c.wrapping_div(8 as uint32_t) as isize)
                as ::core::ffi::c_uint
                & (1 as ::core::ffi::c_uint) << (c & 7 as uint32_t)
                != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int;
        }
        ptr = ptr.offset(
            (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR32>() as usize) as isize,
        );
    }
    while ptr < data_end {
        match *ptr {
            1 => {
                ptr = ptr.offset(1);
                stack = stack >> 1 as ::core::ffi::c_int
                    & (stack | !(1 as ::core::ffi::c_uint as uint32_t));
                stack_depth -= 1;
            }
            2 => {
                ptr = ptr.offset(1);
                stack =
                    stack >> 1 as ::core::ffi::c_int | stack & 1 as ::core::ffi::c_uint as uint32_t;
                stack_depth -= 1;
            }
            3 => {
                ptr = ptr.offset(1);
                stack =
                    stack >> 1 as ::core::ffi::c_int ^ stack & 1 as ::core::ffi::c_uint as uint32_t;
                stack_depth -= 1;
            }
            4 => {
                ptr = ptr.offset(1);
                stack ^= 1 as ::core::ffi::c_uint as uint32_t;
            }
            5 => {
                let mut matched: uint32_t = _pcre2_xclass_32(
                    c,
                    ptr.offset(1 as ::core::ffi::c_int as isize)
                        .offset(LINK_SIZE as isize),
                    char_lists_end,
                    utf,
                ) as uint32_t;
                ptr = ptr.offset(*ptr.offset(1 as ::core::ffi::c_int as isize) as isize);
                stack = stack << 1 as ::core::ffi::c_int | matched;
                stack_depth += 1;
            }
            _ => return FALSE,
        }
    }
    return (stack & 1 as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
}
