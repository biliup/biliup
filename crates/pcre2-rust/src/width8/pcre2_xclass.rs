// Translated from PCRE2 10.49 pcre2_xclass.c; code unit width 8.
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
    static _pcre2_ucd_boolprop_sets_8: [uint32_t; 0];
    static _pcre2_ucd_script_sets_8: [uint32_t; 0];
    static _pcre2_ucd_records_8: [ucd_record; 0];
    static _pcre2_ucd_stage1_8: [uint16_t; 0];
    static _pcre2_ucd_stage2_8: [uint16_t; 0];
    static _pcre2_ucp_gentype_8: [uint32_t; 0];
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
pub type PCRE2_UCHAR8 = uint8_t;
pub type PCRE2_SPTR8 = *const PCRE2_UCHAR8;
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
pub const LINK_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
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
pub const PT_LAMP: ::core::ffi::c_int = 0;
pub const PT_GC: ::core::ffi::c_int = 1;
pub const PT_PC: ::core::ffi::c_int = 2;
pub const PT_SC: ::core::ffi::c_int = 3;
pub const PT_SCX: ::core::ffi::c_int = 4;
pub const PT_ALNUM: ::core::ffi::c_int = 5;
pub const PT_SPACE: ::core::ffi::c_int = 6;
pub const PT_PXSPACE: ::core::ffi::c_int = 7;
pub const PT_WORD: ::core::ffi::c_int = 8;
pub const PT_UCNC: ::core::ffi::c_int = 10;
pub const PT_BIDICL: ::core::ffi::c_int = 11;
pub const PT_BOOL: ::core::ffi::c_int = 12;
pub const PT_PXGRAPH: ::core::ffi::c_int = 14;
pub const PT_PXPRINT: ::core::ffi::c_int = 15;
pub const PT_PXPUNCT: ::core::ffi::c_int = 16;
pub const PT_PXXDIGIT: ::core::ffi::c_int = 17;
pub const XCL_NOT: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const XCL_MAP: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const XCL_END: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const XCL_SINGLE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const XCL_PROP: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const XCL_NOTPROP: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_HIGH_16_START: ::core::ffi::c_int = 0x8000 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_LOW_32_START: ::core::ffi::c_int = 0x10000 as ::core::ffi::c_int;
pub const XCL_TYPE_MASK: ::core::ffi::c_int = 0xfff as ::core::ffi::c_int;
pub const XCL_TYPE_BIT_LEN: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const XCL_BEGIN_WITH_RANGE: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const XCL_ITEM_COUNT_MASK: ::core::ffi::c_int = 0x3 as ::core::ffi::c_int;
pub const XCL_CHAR_END: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const XCL_CHAR_SHIFT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ECL_MAP: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const ECL_AND: ::core::ffi::c_int = 1;
pub const ECL_OR: ::core::ffi::c_int = 2;
pub const ECL_XOR: ::core::ffi::c_int = 3;
pub const ECL_NOT: ::core::ffi::c_int = 4;
pub const ECL_XCLASS: ::core::ffi::c_int = 5;
pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const UCD_BIDICLASS_SHIFT: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn _pcre2_xclass_8(
    mut c: uint32_t,
    mut data: PCRE2_SPTR8,
    mut char_lists_end: *const uint8_t,
    mut utf: BOOL,
) -> BOOL {
    let mut t: PCRE2_UCHAR8 = 0;
    let mut not_negated: BOOL =
        (*data as ::core::ffi::c_int & XCL_NOT == 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
    let mut type_0: uint32_t = 0;
    let mut max_index: uint32_t = 0;
    let mut min_index: uint32_t = 0;
    let mut value: uint32_t = 0;
    let mut next_char: *const uint8_t = ::core::ptr::null::<uint8_t>();
    utf = TRUE as BOOL;
    let fresh0 = data;
    data = data.offset(1);
    if *fresh0 as ::core::ffi::c_int & XCL_MAP != 0 as ::core::ffi::c_int {
        if c < 256 as uint32_t {
            return (*(data as *const uint8_t).offset(c.wrapping_div(8 as uint32_t) as isize)
                as ::core::ffi::c_uint
                & (1 as ::core::ffi::c_uint) << (c & 7 as uint32_t)
                != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int;
        }
        data = data.offset(
            (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize) as isize,
        );
    }
    if *data as ::core::ffi::c_int == XCL_PROP || *data as ::core::ffi::c_int == XCL_NOTPROP {
        let mut prop: *const ucd_record = (&raw const _pcre2_ucd_records_8 as *const ucd_record)
            .offset(
                *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t).offset(
                    (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                        .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                        as ::core::ffi::c_int
                        * UCD_BLOCK_SIZE
                        + c as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                ) as ::core::ffi::c_int as isize,
            );
        loop {
            let mut chartype: ::core::ffi::c_int = 0;
            let fresh1 = data;
            data = data.offset(1);
            let mut isprop: BOOL =
                (*fresh1 as ::core::ffi::c_int == XCL_PROP) as ::core::ffi::c_int;
            let mut ok: BOOL = 0;
            match *data as ::core::ffi::c_int {
                PT_LAMP => {
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
                PT_GC => {
                    if (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                        == *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                            .offset((*prop).chartype as isize))
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_PC => {
                    if (*data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == (*prop).chartype as ::core::ffi::c_int)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_SC => {
                    if (*data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == (*prop).script as ::core::ffi::c_int)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_SCX => {
                    ok = (*data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == (*prop).script as ::core::ffi::c_int
                        || *(&raw const _pcre2_ucd_script_sets_8 as *const uint32_t)
                            .offset(
                                ((*prop).scriptx_bidiclass as ::core::ffi::c_int
                                    & 0x3ff as ::core::ffi::c_int)
                                    as isize,
                            )
                            .offset(
                                (*data.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int
                                    / 32 as ::core::ffi::c_int)
                                    as isize,
                            )
                            & (1 as uint32_t)
                                << *data.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int
                                    % 32 as ::core::ffi::c_int
                            != 0 as uint32_t) as ::core::ffi::c_int
                        as BOOL;
                    if ok == isprop {
                        return not_negated;
                    }
                }
                PT_ALNUM => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                        .offset(chartype as isize)
                        == ucp_L as ::core::ffi::c_int as uint32_t
                        || *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                            .offset(chartype as isize)
                            == ucp_N as ::core::ffi::c_int as uint32_t)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_SPACE | PT_PXSPACE => match c {
                    9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194 | 8195 | 8196 | 8197 | 8198
                    | 8199 | 8200 | 8201 | 8202 | 8239 | 8287 | 12288 | 10 | 11 | 12 | 13 | 133
                    | 8232 | 8233 => {
                        if isprop != 0 {
                            return not_negated;
                        }
                    }
                    _ => {
                        if (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                            .offset((*prop).chartype as isize)
                            == ucp_Z as ::core::ffi::c_int as uint32_t)
                            as ::core::ffi::c_int
                            == isprop
                        {
                            return not_negated;
                        }
                    }
                },
                PT_WORD => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                        .offset(chartype as isize)
                        == ucp_L as ::core::ffi::c_int as uint32_t
                        || *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
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
                PT_UCNC => {
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
                PT_BIDICL => {
                    if ((*prop).scriptx_bidiclass as ::core::ffi::c_int >> UCD_BIDICLASS_SHIFT
                        == *data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_BOOL => {
                    ok = (*(&raw const _pcre2_ucd_boolprop_sets_8 as *const uint32_t)
                        .offset(
                            ((*prop).bprops as ::core::ffi::c_int & 0xfff as ::core::ffi::c_int)
                                as isize,
                        )
                        .offset(
                            (*data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                / 32 as ::core::ffi::c_int) as isize,
                        )
                        & (1 as uint32_t)
                            << *data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                % 32 as ::core::ffi::c_int
                        != 0 as uint32_t) as ::core::ffi::c_int as BOOL;
                    if ok == isprop {
                        return not_negated;
                    }
                }
                PT_PXGRAPH => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                        .offset(chartype as isize)
                        != ucp_Z as ::core::ffi::c_int as uint32_t
                        && (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
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
                PT_PXPRINT => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (chartype != ucp_Zl as ::core::ffi::c_int
                        && chartype != ucp_Zp as ::core::ffi::c_int
                        && (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
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
                PT_PXPUNCT => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                        .offset(chartype as isize)
                        == ucp_P as ::core::ffi::c_int as uint32_t
                        || c < 128 as uint32_t
                            && *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                                .offset(chartype as isize)
                                == ucp_S as ::core::ffi::c_int as uint32_t)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_PXXDIGIT => {
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
            if !(*data as ::core::ffi::c_int == XCL_PROP
                || *data as ::core::ffi::c_int == XCL_NOTPROP)
            {
                break;
            }
        }
    }
    if (*data as ::core::ffi::c_int)
        < (if ::core::mem::size_of::<PCRE2_UCHAR8>() as usize == 1 as usize {
            0x10 as ::core::ffi::c_int
        } else {
            0x1000 as ::core::ffi::c_int
        })
    {
        loop {
            let fresh2 = data;
            data = data.offset(1);
            t = *fresh2;
            if !(t as ::core::ffi::c_int != XCL_END) {
                break;
            }
            let mut x: uint32_t = 0;
            let mut y: uint32_t = 0;
            if utf != 0 {
                let fresh3 = data;
                data = data.offset(1);
                x = *fresh3 as uint32_t;
                if x >= 0xc0 as uint32_t {
                    if x & 0x20 as uint32_t == 0 as uint32_t {
                        let fresh4 = data;
                        data = data.offset(1);
                        x = (x & 0x1f as uint32_t) << 6 as ::core::ffi::c_int
                            | *fresh4 as uint32_t & 0x3f as uint32_t;
                    } else if x & 0x10 as uint32_t == 0 as uint32_t {
                        x = (x & 0xf as uint32_t) << 12 as ::core::ffi::c_int
                            | (*data as uint32_t & 0x3f as uint32_t) << 6 as ::core::ffi::c_int
                            | *data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t;
                        data = data.offset(2 as ::core::ffi::c_int as isize);
                    } else if x & 0x8 as uint32_t == 0 as uint32_t {
                        x = (x & 0x7 as uint32_t) << 18 as ::core::ffi::c_int
                            | (*data as uint32_t & 0x3f as uint32_t) << 12 as ::core::ffi::c_int
                            | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 6 as ::core::ffi::c_int
                            | *data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t;
                        data = data.offset(3 as ::core::ffi::c_int as isize);
                    } else if x & 0x4 as uint32_t == 0 as uint32_t {
                        x = (x & 0x3 as uint32_t) << 24 as ::core::ffi::c_int
                            | (*data as uint32_t & 0x3f as uint32_t) << 18 as ::core::ffi::c_int
                            | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 12 as ::core::ffi::c_int
                            | (*data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 6 as ::core::ffi::c_int
                            | *data.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t;
                        data = data.offset(4 as ::core::ffi::c_int as isize);
                    } else {
                        x = (x & 0x1 as uint32_t) << 30 as ::core::ffi::c_int
                            | (*data as uint32_t & 0x3f as uint32_t) << 24 as ::core::ffi::c_int
                            | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 18 as ::core::ffi::c_int
                            | (*data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 12 as ::core::ffi::c_int
                            | (*data.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 6 as ::core::ffi::c_int
                            | *data.offset(4 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t;
                        data = data.offset(5 as ::core::ffi::c_int as isize);
                    }
                }
            } else {
                let fresh5 = data;
                data = data.offset(1);
                x = *fresh5 as uint32_t;
            }
            if t as ::core::ffi::c_int == XCL_SINGLE {
                if c <= x {
                    return if c == x {
                        not_negated
                    } else {
                        (not_negated == 0) as ::core::ffi::c_int
                    };
                }
            } else {
                if utf != 0 {
                    let fresh6 = data;
                    data = data.offset(1);
                    y = *fresh6 as uint32_t;
                    if y >= 0xc0 as uint32_t {
                        if y & 0x20 as uint32_t == 0 as uint32_t {
                            let fresh7 = data;
                            data = data.offset(1);
                            y = (y & 0x1f as uint32_t) << 6 as ::core::ffi::c_int
                                | *fresh7 as uint32_t & 0x3f as uint32_t;
                        } else if y & 0x10 as uint32_t == 0 as uint32_t {
                            y = (y & 0xf as uint32_t) << 12 as ::core::ffi::c_int
                                | (*data as uint32_t & 0x3f as uint32_t) << 6 as ::core::ffi::c_int
                                | *data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                            data = data.offset(2 as ::core::ffi::c_int as isize);
                        } else if y & 0x8 as uint32_t == 0 as uint32_t {
                            y = (y & 0x7 as uint32_t) << 18 as ::core::ffi::c_int
                                | (*data as uint32_t & 0x3f as uint32_t)
                                    << 12 as ::core::ffi::c_int
                                | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 6 as ::core::ffi::c_int
                                | *data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                            data = data.offset(3 as ::core::ffi::c_int as isize);
                        } else if y & 0x4 as uint32_t == 0 as uint32_t {
                            y = (y & 0x3 as uint32_t) << 24 as ::core::ffi::c_int
                                | (*data as uint32_t & 0x3f as uint32_t)
                                    << 18 as ::core::ffi::c_int
                                | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 12 as ::core::ffi::c_int
                                | (*data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 6 as ::core::ffi::c_int
                                | *data.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                            data = data.offset(4 as ::core::ffi::c_int as isize);
                        } else {
                            y = (y & 0x1 as uint32_t) << 30 as ::core::ffi::c_int
                                | (*data as uint32_t & 0x3f as uint32_t)
                                    << 24 as ::core::ffi::c_int
                                | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 18 as ::core::ffi::c_int
                                | (*data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 12 as ::core::ffi::c_int
                                | (*data.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 6 as ::core::ffi::c_int
                                | *data.offset(4 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                            data = data.offset(5 as ::core::ffi::c_int as isize);
                        }
                    }
                } else {
                    let fresh8 = data;
                    data = data.offset(1);
                    y = *fresh8 as uint32_t;
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
    type_0 = ((*data.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
        << 8 as ::core::ffi::c_int) as uint32_t
        | *data.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
    data = data.offset(2 as ::core::ffi::c_int as isize);
    next_char = char_lists_end.offset(
        -(((((*data.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *data.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int) as ::core::ffi::c_uint)
            << 1 as ::core::ffi::c_int) as isize),
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
pub unsafe extern "C" fn _pcre2_eclass_8(
    mut c: uint32_t,
    mut data_start: PCRE2_SPTR8,
    mut data_end: PCRE2_SPTR8,
    mut char_lists_end: *const uint8_t,
    mut utf: BOOL,
) -> BOOL {
    let mut ptr: PCRE2_SPTR8 = data_start;
    let mut flags: PCRE2_UCHAR8 = 0;
    let mut stack: uint32_t = 0 as uint32_t;
    let mut stack_depth: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let fresh9 = ptr;
    ptr = ptr.offset(1);
    flags = *fresh9;
    if flags as ::core::ffi::c_int & ECL_MAP != 0 as ::core::ffi::c_int {
        if c < 256 as uint32_t {
            return (*(ptr as *const uint8_t).offset(c.wrapping_div(8 as uint32_t) as isize)
                as ::core::ffi::c_uint
                & (1 as ::core::ffi::c_uint) << (c & 7 as uint32_t)
                != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int;
        }
        ptr = ptr.offset(
            (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize) as isize,
        );
    }
    while ptr < data_end {
        match *ptr as ::core::ffi::c_int {
            ECL_AND => {
                ptr = ptr.offset(1);
                stack = stack >> 1 as ::core::ffi::c_int
                    & (stack | !(1 as ::core::ffi::c_uint as uint32_t));
                stack_depth -= 1;
            }
            ECL_OR => {
                ptr = ptr.offset(1);
                stack =
                    stack >> 1 as ::core::ffi::c_int | stack & 1 as ::core::ffi::c_uint as uint32_t;
                stack_depth -= 1;
            }
            ECL_XOR => {
                ptr = ptr.offset(1);
                stack =
                    stack >> 1 as ::core::ffi::c_int ^ stack & 1 as ::core::ffi::c_uint as uint32_t;
                stack_depth -= 1;
            }
            ECL_NOT => {
                ptr = ptr.offset(1);
                stack ^= 1 as ::core::ffi::c_uint as uint32_t;
            }
            ECL_XCLASS => {
                let mut matched: uint32_t = _pcre2_xclass_8(
                    c,
                    ptr.offset(1 as ::core::ffi::c_int as isize)
                        .offset(LINK_SIZE as isize),
                    char_lists_end,
                    utf,
                ) as uint32_t;
                ptr = ptr.offset(
                    ((*ptr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *ptr.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint
                        as isize,
                );
                stack = stack << 1 as ::core::ffi::c_int | matched;
                stack_depth += 1;
            }
            _ => return FALSE,
        }
    }
    return (stack & 1 as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
}
