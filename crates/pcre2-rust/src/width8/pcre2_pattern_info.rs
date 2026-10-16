// Translated from PCRE2 10.49 pcre2_pattern_info.c; code unit width 8.
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
extern "C" {
    static _pcre2_utf8_table4: [uint8_t; 0];
    static _pcre2_OP_lengths_8: [uint8_t; 0];
    fn _pcre2_jit_get_size_8(_: *mut ::core::ffi::c_void) -> size_t;
}
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type size_t = usize;
pub type BOOL = ::core::ffi::c_int;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR8 = uint8_t;
pub type PCRE2_SPTR8 = *const PCRE2_UCHAR8;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_memctl {
    pub malloc:
        Option<unsafe extern "C" fn(size_t, *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void>,
    pub free:
        Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> ()>,
    pub memory_data: *mut ::core::ffi::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_real_code_8 {
    pub memctl: pcre2_memctl,
    pub tables: *const uint8_t,
    pub executable_jit: *mut ::core::ffi::c_void,
    pub start_bitmap: [uint8_t; 32],
    pub blocksize: size_t,
    pub code_start: size_t,
    pub magic_number: uint32_t,
    pub compile_options: uint32_t,
    pub overall_options: uint32_t,
    pub extra_options: uint32_t,
    pub flags: uint32_t,
    pub limit_heap: uint32_t,
    pub limit_match: uint32_t,
    pub limit_depth: uint32_t,
    pub first_codeunit: uint32_t,
    pub last_codeunit: uint32_t,
    pub bsr_convention: uint16_t,
    pub newline_convention: uint16_t,
    pub max_lookbehind: uint16_t,
    pub minlength: uint16_t,
    pub top_bracket: uint16_t,
    pub top_backref: uint16_t,
    pub name_entry_size: uint16_t,
    pub name_count: uint16_t,
    pub optimization_flags: uint32_t,
}
pub type pcre2_code_8 = pcre2_real_code_8;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_callout_enumerate_block_8 {
    pub version: uint32_t,
    pub pattern_position: size_t,
    pub next_item_length: size_t,
    pub callout_number: uint32_t,
    pub callout_string_offset: size_t,
    pub callout_string_length: size_t,
    pub callout_string: PCRE2_SPTR8,
}
pub const OP_CALLOUT_STR: C2RustUnnamed = 120;
pub const OP_CALLOUT: C2RustUnnamed = 119;
pub const OP_THEN_ARG: C2RustUnnamed = 162;
pub const OP_SKIP_ARG: C2RustUnnamed = 160;
pub const OP_PRUNE_ARG: C2RustUnnamed = 158;
pub const OP_COMMIT_ARG: C2RustUnnamed = 164;
pub const OP_MARK: C2RustUnnamed = 156;
pub const OP_ECLASS: C2RustUnnamed = 113;
pub const OP_XCLASS: C2RustUnnamed = 112;
pub const OP_NOTPROP: C2RustUnnamed = 15;
pub const OP_PROP: C2RustUnnamed = 16;
pub const OP_TYPEPOSUPTO: C2RustUnnamed = 97;
pub const OP_TYPEPOSQUERY: C2RustUnnamed = 96;
pub const OP_TYPEPOSPLUS: C2RustUnnamed = 95;
pub const OP_TYPEPOSSTAR: C2RustUnnamed = 94;
pub const OP_TYPEEXACT: C2RustUnnamed = 93;
pub const OP_TYPEMINUPTO: C2RustUnnamed = 92;
pub const OP_TYPEUPTO: C2RustUnnamed = 91;
pub const OP_TYPEMINQUERY: C2RustUnnamed = 90;
pub const OP_TYPEQUERY: C2RustUnnamed = 89;
pub const OP_TYPEMINPLUS: C2RustUnnamed = 88;
pub const OP_TYPEPLUS: C2RustUnnamed = 87;
pub const OP_TYPEMINSTAR: C2RustUnnamed = 86;
pub const OP_TYPESTAR: C2RustUnnamed = 85;
pub const OP_NOTPOSUPTOI: C2RustUnnamed = 84;
pub const OP_NOTPOSQUERYI: C2RustUnnamed = 83;
pub const OP_NOTPOSPLUSI: C2RustUnnamed = 82;
pub const OP_NOTPOSSTARI: C2RustUnnamed = 81;
pub const OP_NOTEXACTI: C2RustUnnamed = 80;
pub const OP_NOTMINUPTOI: C2RustUnnamed = 79;
pub const OP_NOTUPTOI: C2RustUnnamed = 78;
pub const OP_NOTMINQUERYI: C2RustUnnamed = 77;
pub const OP_NOTQUERYI: C2RustUnnamed = 76;
pub const OP_NOTMINPLUSI: C2RustUnnamed = 75;
pub const OP_NOTPLUSI: C2RustUnnamed = 74;
pub const OP_NOTMINSTARI: C2RustUnnamed = 73;
pub const OP_NOTSTARI: C2RustUnnamed = 72;
pub const OP_NOTPOSUPTO: C2RustUnnamed = 71;
pub const OP_NOTPOSQUERY: C2RustUnnamed = 70;
pub const OP_NOTPOSPLUS: C2RustUnnamed = 69;
pub const OP_NOTPOSSTAR: C2RustUnnamed = 68;
pub const OP_NOTEXACT: C2RustUnnamed = 67;
pub const OP_NOTMINUPTO: C2RustUnnamed = 66;
pub const OP_NOTUPTO: C2RustUnnamed = 65;
pub const OP_NOTMINQUERY: C2RustUnnamed = 64;
pub const OP_NOTQUERY: C2RustUnnamed = 63;
pub const OP_NOTMINPLUS: C2RustUnnamed = 62;
pub const OP_NOTPLUS: C2RustUnnamed = 61;
pub const OP_NOTMINSTAR: C2RustUnnamed = 60;
pub const OP_NOTSTAR: C2RustUnnamed = 59;
pub const OP_POSUPTOI: C2RustUnnamed = 58;
pub const OP_POSQUERYI: C2RustUnnamed = 57;
pub const OP_POSPLUSI: C2RustUnnamed = 56;
pub const OP_POSSTARI: C2RustUnnamed = 55;
pub const OP_EXACTI: C2RustUnnamed = 54;
pub const OP_MINUPTOI: C2RustUnnamed = 53;
pub const OP_UPTOI: C2RustUnnamed = 52;
pub const OP_MINQUERYI: C2RustUnnamed = 51;
pub const OP_QUERYI: C2RustUnnamed = 50;
pub const OP_MINPLUSI: C2RustUnnamed = 49;
pub const OP_PLUSI: C2RustUnnamed = 48;
pub const OP_MINSTARI: C2RustUnnamed = 47;
pub const OP_STARI: C2RustUnnamed = 46;
pub const OP_POSUPTO: C2RustUnnamed = 45;
pub const OP_POSQUERY: C2RustUnnamed = 44;
pub const OP_POSPLUS: C2RustUnnamed = 43;
pub const OP_POSSTAR: C2RustUnnamed = 42;
pub const OP_EXACT: C2RustUnnamed = 41;
pub const OP_MINUPTO: C2RustUnnamed = 40;
pub const OP_UPTO: C2RustUnnamed = 39;
pub const OP_MINQUERY: C2RustUnnamed = 38;
pub const OP_QUERY: C2RustUnnamed = 37;
pub const OP_MINPLUS: C2RustUnnamed = 36;
pub const OP_PLUS: C2RustUnnamed = 35;
pub const OP_MINSTAR: C2RustUnnamed = 34;
pub const OP_STAR: C2RustUnnamed = 33;
pub const OP_NOTI: C2RustUnnamed = 32;
pub const OP_NOT: C2RustUnnamed = 31;
pub const OP_CHARI: C2RustUnnamed = 30;
pub const OP_CHAR: C2RustUnnamed = 29;
pub const OP_END: C2RustUnnamed = 0;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const OP_TABLE_LENGTH: C2RustUnnamed = 173;
pub const OP_UCP_WORD_BOUNDARY: C2RustUnnamed = 172;
pub const OP_NOT_UCP_WORD_BOUNDARY: C2RustUnnamed = 171;
pub const OP_DEFINE: C2RustUnnamed = 170;
pub const OP_SKIPZERO: C2RustUnnamed = 169;
pub const OP_CLOSE: C2RustUnnamed = 168;
pub const OP_ASSERT_ACCEPT: C2RustUnnamed = 167;
pub const OP_ACCEPT: C2RustUnnamed = 166;
pub const OP_FAIL: C2RustUnnamed = 165;
pub const OP_COMMIT: C2RustUnnamed = 163;
pub const OP_THEN: C2RustUnnamed = 161;
pub const OP_SKIP: C2RustUnnamed = 159;
pub const OP_PRUNE: C2RustUnnamed = 157;
pub const OP_BRAPOSZERO: C2RustUnnamed = 155;
pub const OP_BRAMINZERO: C2RustUnnamed = 154;
pub const OP_BRAZERO: C2RustUnnamed = 153;
pub const OP_TRUE: C2RustUnnamed = 152;
pub const OP_FALSE: C2RustUnnamed = 151;
pub const OP_DNRREF: C2RustUnnamed = 150;
pub const OP_RREF: C2RustUnnamed = 149;
pub const OP_DNCREF: C2RustUnnamed = 148;
pub const OP_CREF: C2RustUnnamed = 147;
pub const OP_SCOND: C2RustUnnamed = 146;
pub const OP_SCBRAPOS: C2RustUnnamed = 145;
pub const OP_SCBRA: C2RustUnnamed = 144;
pub const OP_SBRAPOS: C2RustUnnamed = 143;
pub const OP_SBRA: C2RustUnnamed = 142;
pub const OP_COND: C2RustUnnamed = 141;
pub const OP_CBRAPOS: C2RustUnnamed = 140;
pub const OP_CBRA: C2RustUnnamed = 139;
pub const OP_BRAPOS: C2RustUnnamed = 138;
pub const OP_BRA: C2RustUnnamed = 137;
pub const OP_SCRIPT_RUN: C2RustUnnamed = 136;
pub const OP_ONCE: C2RustUnnamed = 135;
pub const OP_ASSERT_SCS: C2RustUnnamed = 134;
pub const OP_ASSERTBACK_NA: C2RustUnnamed = 133;
pub const OP_ASSERT_NA: C2RustUnnamed = 132;
pub const OP_ASSERTBACK_NOT: C2RustUnnamed = 131;
pub const OP_ASSERTBACK: C2RustUnnamed = 130;
pub const OP_ASSERT_NOT: C2RustUnnamed = 129;
pub const OP_ASSERT: C2RustUnnamed = 128;
pub const OP_VREVERSE: C2RustUnnamed = 127;
pub const OP_REVERSE: C2RustUnnamed = 126;
pub const OP_KETRPOS: C2RustUnnamed = 125;
pub const OP_KETRMIN: C2RustUnnamed = 124;
pub const OP_KETRMAX: C2RustUnnamed = 123;
pub const OP_KET: C2RustUnnamed = 122;
pub const OP_ALT: C2RustUnnamed = 121;
pub const OP_RECURSE: C2RustUnnamed = 118;
pub const OP_DNREFI: C2RustUnnamed = 117;
pub const OP_DNREF: C2RustUnnamed = 116;
pub const OP_REFI: C2RustUnnamed = 115;
pub const OP_REF: C2RustUnnamed = 114;
pub const OP_NCLASS: C2RustUnnamed = 111;
pub const OP_CLASS: C2RustUnnamed = 110;
pub const OP_CRPOSRANGE: C2RustUnnamed = 109;
pub const OP_CRPOSQUERY: C2RustUnnamed = 108;
pub const OP_CRPOSPLUS: C2RustUnnamed = 107;
pub const OP_CRPOSSTAR: C2RustUnnamed = 106;
pub const OP_CRMINRANGE: C2RustUnnamed = 105;
pub const OP_CRRANGE: C2RustUnnamed = 104;
pub const OP_CRMINQUERY: C2RustUnnamed = 103;
pub const OP_CRQUERY: C2RustUnnamed = 102;
pub const OP_CRMINPLUS: C2RustUnnamed = 101;
pub const OP_CRPLUS: C2RustUnnamed = 100;
pub const OP_CRMINSTAR: C2RustUnnamed = 99;
pub const OP_CRSTAR: C2RustUnnamed = 98;
pub const OP_CIRCM: C2RustUnnamed = 28;
pub const OP_CIRC: C2RustUnnamed = 27;
pub const OP_DOLLM: C2RustUnnamed = 26;
pub const OP_DOLL: C2RustUnnamed = 25;
pub const OP_EOD: C2RustUnnamed = 24;
pub const OP_EODN: C2RustUnnamed = 23;
pub const OP_EXTUNI: C2RustUnnamed = 22;
pub const OP_VSPACE: C2RustUnnamed = 21;
pub const OP_NOT_VSPACE: C2RustUnnamed = 20;
pub const OP_HSPACE: C2RustUnnamed = 19;
pub const OP_NOT_HSPACE: C2RustUnnamed = 18;
pub const OP_ANYNL: C2RustUnnamed = 17;
pub const OP_ANYBYTE: C2RustUnnamed = 14;
pub const OP_ALLANY: C2RustUnnamed = 13;
pub const OP_ANY: C2RustUnnamed = 12;
pub const OP_WORDCHAR: C2RustUnnamed = 11;
pub const OP_NOT_WORDCHAR: C2RustUnnamed = 10;
pub const OP_WHITESPACE: C2RustUnnamed = 9;
pub const OP_NOT_WHITESPACE: C2RustUnnamed = 8;
pub const OP_DIGIT: C2RustUnnamed = 7;
pub const OP_NOT_DIGIT: C2RustUnnamed = 6;
pub const OP_WORD_BOUNDARY: C2RustUnnamed = 5;
pub const OP_NOT_WORD_BOUNDARY: C2RustUnnamed = 4;
pub const OP_SET_SOM: C2RustUnnamed = 3;
pub const OP_SOM: C2RustUnnamed = 2;
pub const OP_SOD: C2RustUnnamed = 1;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const UINT32_MAX: ::core::ffi::c_uint = 4294967295 as ::core::ffi::c_uint;
pub const PCRE2_UTF: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PCRE2_ERROR_BADMAGIC: ::core::ffi::c_int = -(31 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADMODE: ::core::ffi::c_int = -(32 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADOPTION: ::core::ffi::c_int = -(34 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NULL: ::core::ffi::c_int = -(51 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UNSET: ::core::ffi::c_int = -(55 as ::core::ffi::c_int);
pub const PCRE2_INFO_ALLOPTIONS: uint32_t = 0 as uint32_t;
pub const PCRE2_INFO_ARGOPTIONS: uint32_t = 1 as uint32_t;
pub const PCRE2_INFO_BACKREFMAX: uint32_t = 2 as uint32_t;
pub const PCRE2_INFO_BSR: uint32_t = 3 as uint32_t;
pub const PCRE2_INFO_CAPTURECOUNT: uint32_t = 4 as uint32_t;
pub const PCRE2_INFO_FIRSTCODEUNIT: uint32_t = 5 as uint32_t;
pub const PCRE2_INFO_FIRSTCODETYPE: uint32_t = 6 as uint32_t;
pub const PCRE2_INFO_FIRSTBITMAP: uint32_t = 7 as uint32_t;
pub const PCRE2_INFO_HASCRORLF: uint32_t = 8 as uint32_t;
pub const PCRE2_INFO_JCHANGED: uint32_t = 9 as uint32_t;
pub const PCRE2_INFO_JITSIZE: uint32_t = 10 as uint32_t;
pub const PCRE2_INFO_LASTCODEUNIT: uint32_t = 11 as uint32_t;
pub const PCRE2_INFO_LASTCODETYPE: uint32_t = 12 as uint32_t;
pub const PCRE2_INFO_MATCHEMPTY: uint32_t = 13 as uint32_t;
pub const PCRE2_INFO_MATCHLIMIT: uint32_t = 14 as uint32_t;
pub const PCRE2_INFO_MAXLOOKBEHIND: uint32_t = 15 as uint32_t;
pub const PCRE2_INFO_MINLENGTH: uint32_t = 16 as uint32_t;
pub const PCRE2_INFO_NAMECOUNT: uint32_t = 17 as uint32_t;
pub const PCRE2_INFO_NAMEENTRYSIZE: uint32_t = 18 as uint32_t;
pub const PCRE2_INFO_NAMETABLE: uint32_t = 19 as uint32_t;
pub const PCRE2_INFO_NEWLINE: uint32_t = 20 as uint32_t;
pub const PCRE2_INFO_DEPTHLIMIT: uint32_t = 21 as uint32_t;
pub const PCRE2_INFO_SIZE: uint32_t = 22 as uint32_t;
pub const PCRE2_INFO_HASBACKSLASHC: uint32_t = 23 as uint32_t;
pub const PCRE2_INFO_FRAMESIZE: uint32_t = 24 as uint32_t;
pub const PCRE2_INFO_HEAPLIMIT: uint32_t = 25 as uint32_t;
pub const PCRE2_INFO_EXTRAOPTIONS: uint32_t = 26 as uint32_t;
pub const PCRE2_FIRSTSET: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PCRE2_FIRSTMAPSET: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PCRE2_LASTSET: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PCRE2_STARTLINE: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PCRE2_JCHANGED: ::core::ffi::c_uint = 0x400 as ::core::ffi::c_uint;
pub const PCRE2_HASCRORLF: ::core::ffi::c_uint = 0x800 as ::core::ffi::c_uint;
pub const PCRE2_MATCH_EMPTY: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PCRE2_HASBKC: ::core::ffi::c_uint = 0x400000 as ::core::ffi::c_uint;
pub const MAGIC_NUMBER: usize = 0x50435245 as usize;
#[no_mangle]
pub unsafe extern "C" fn pcre2_pattern_info_8(
    mut code: *const pcre2_code_8,
    mut what: uint32_t,
    mut where_0: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let mut re: *const pcre2_real_code_8 = code as *const pcre2_real_code_8;
    if where_0.is_null() {
        match what {
            0 | 1 | 2 | 3 | 4 | 21 | 26 | 6 | 5 | 23 | 8 | 25 | 9 | 12 | 11 | 13 | 14 | 15 | 16
            | 18 | 17 | 20 => {
                return ::core::mem::size_of::<uint32_t>() as ::core::ffi::c_int;
            }
            7 => return ::core::mem::size_of::<*const uint8_t>() as ::core::ffi::c_int,
            10 | 22 | 24 => return ::core::mem::size_of::<size_t>() as ::core::ffi::c_int,
            19 => return ::core::mem::size_of::<PCRE2_SPTR8>() as ::core::ffi::c_int,
            _ => {}
        }
    }
    if re.is_null() {
        return PCRE2_ERROR_NULL;
    }
    if (*re).magic_number as usize != MAGIC_NUMBER {
        return PCRE2_ERROR_BADMAGIC;
    }
    if (*re).flags & (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as uint32_t == 0 as uint32_t
    {
        return PCRE2_ERROR_BADMODE;
    }
    match what {
        0 => {
            *(where_0 as *mut uint32_t) = (*re).overall_options;
        }
        1 => {
            *(where_0 as *mut uint32_t) = (*re).compile_options;
        }
        2 => {
            *(where_0 as *mut uint32_t) = (*re).top_backref as uint32_t;
        }
        3 => {
            *(where_0 as *mut uint32_t) = (*re).bsr_convention as uint32_t;
        }
        4 => {
            *(where_0 as *mut uint32_t) = (*re).top_bracket as uint32_t;
        }
        21 => {
            *(where_0 as *mut uint32_t) = (*re).limit_depth;
            if (*re).limit_depth == UINT32_MAX as uint32_t {
                return PCRE2_ERROR_UNSET;
            }
        }
        26 => {
            *(where_0 as *mut uint32_t) = (*re).extra_options;
        }
        6 => {
            *(where_0 as *mut uint32_t) =
                (if (*re).flags & PCRE2_FIRSTSET as uint32_t != 0 as uint32_t {
                    1 as ::core::ffi::c_int
                } else if (*re).flags & PCRE2_STARTLINE as uint32_t != 0 as uint32_t {
                    2 as ::core::ffi::c_int
                } else {
                    0 as ::core::ffi::c_int
                }) as uint32_t;
        }
        5 => {
            *(where_0 as *mut uint32_t) =
                if (*re).flags & PCRE2_FIRSTSET as uint32_t != 0 as uint32_t {
                    (*re).first_codeunit
                } else {
                    0 as uint32_t
                };
        }
        7 => {
            let ref mut fresh0 = *(where_0 as *mut *const uint8_t);
            *fresh0 = if (*re).flags & PCRE2_FIRSTMAPSET as uint32_t != 0 as uint32_t {
                (&raw const (*re).start_bitmap as *const uint8_t)
                    .offset(0 as ::core::ffi::c_int as isize) as *const uint8_t
            } else {
                ::core::ptr::null::<uint8_t>()
            };
        }
        24 => {
            *(where_0 as *mut size_t) = (120 as usize).wrapping_add(
                (((*re).top_bracket as ::core::ffi::c_int * 2 as ::core::ffi::c_int) as usize)
                    .wrapping_mul(::core::mem::size_of::<size_t>() as usize),
            ) as size_t;
        }
        23 => {
            *(where_0 as *mut uint32_t) = ((*re).flags & PCRE2_HASBKC as uint32_t != 0 as uint32_t)
                as ::core::ffi::c_int as uint32_t;
        }
        8 => {
            *(where_0 as *mut uint32_t) = ((*re).flags & PCRE2_HASCRORLF as uint32_t
                != 0 as uint32_t) as ::core::ffi::c_int
                as uint32_t;
        }
        25 => {
            *(where_0 as *mut uint32_t) = (*re).limit_heap;
            if (*re).limit_heap == UINT32_MAX as uint32_t {
                return PCRE2_ERROR_UNSET;
            }
        }
        9 => {
            *(where_0 as *mut uint32_t) = ((*re).flags & PCRE2_JCHANGED as uint32_t
                != 0 as uint32_t) as ::core::ffi::c_int
                as uint32_t;
        }
        10 => {
            *(where_0 as *mut size_t) = if !(*re).executable_jit.is_null() {
                _pcre2_jit_get_size_8((*re).executable_jit)
            } else {
                0 as size_t
            };
        }
        12 => {
            *(where_0 as *mut uint32_t) =
                (if (*re).flags & PCRE2_LASTSET as uint32_t != 0 as uint32_t {
                    1 as ::core::ffi::c_int
                } else {
                    0 as ::core::ffi::c_int
                }) as uint32_t;
        }
        11 => {
            *(where_0 as *mut uint32_t) =
                if (*re).flags & PCRE2_LASTSET as uint32_t != 0 as uint32_t {
                    (*re).last_codeunit
                } else {
                    0 as uint32_t
                };
        }
        13 => {
            *(where_0 as *mut uint32_t) = ((*re).flags & PCRE2_MATCH_EMPTY as uint32_t
                != 0 as uint32_t) as ::core::ffi::c_int
                as uint32_t;
        }
        14 => {
            *(where_0 as *mut uint32_t) = (*re).limit_match;
            if (*re).limit_match == UINT32_MAX as uint32_t {
                return PCRE2_ERROR_UNSET;
            }
        }
        15 => {
            *(where_0 as *mut uint32_t) = (*re).max_lookbehind as uint32_t;
        }
        16 => {
            *(where_0 as *mut uint32_t) = (*re).minlength as uint32_t;
        }
        18 => {
            *(where_0 as *mut uint32_t) = (*re).name_entry_size as uint32_t;
        }
        17 => {
            *(where_0 as *mut uint32_t) = (*re).name_count as uint32_t;
        }
        19 => {
            let ref mut fresh1 = *(where_0 as *mut PCRE2_SPTR8);
            *fresh1 = (re as *const ::core::ffi::c_char)
                .offset(::core::mem::size_of::<pcre2_real_code_8>() as usize as isize)
                as PCRE2_SPTR8;
        }
        20 => {
            *(where_0 as *mut uint32_t) = (*re).newline_convention as uint32_t;
        }
        22 => {
            *(where_0 as *mut size_t) = (*re).blocksize;
        }
        _ => return PCRE2_ERROR_BADOPTION,
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_callout_enumerate_8(
    mut code: *const pcre2_code_8,
    mut callback: Option<
        unsafe extern "C" fn(
            *mut pcre2_callout_enumerate_block_8,
            *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
    mut callout_data: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let mut re: *const pcre2_real_code_8 = code as *const pcre2_real_code_8;
    let mut cb: pcre2_callout_enumerate_block_8 = pcre2_callout_enumerate_block_8 {
        version: 0,
        pattern_position: 0,
        next_item_length: 0,
        callout_number: 0,
        callout_string_offset: 0,
        callout_string_length: 0,
        callout_string: ::core::ptr::null::<PCRE2_UCHAR8>(),
    };
    let mut cc: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut utf: BOOL = 0;
    if re.is_null() {
        return PCRE2_ERROR_NULL;
    }
    utf = ((*re).overall_options & PCRE2_UTF as uint32_t != 0 as uint32_t) as ::core::ffi::c_int
        as BOOL;
    if (*re).magic_number as usize != MAGIC_NUMBER {
        return PCRE2_ERROR_BADMAGIC;
    }
    if (*re).flags & (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as uint32_t == 0 as uint32_t
    {
        return PCRE2_ERROR_BADMODE;
    }
    cb.version = 0 as uint32_t;
    cc = (re as *mut uint8_t).offset((*re).code_start as isize) as PCRE2_SPTR8;
    loop {
        let mut rc: ::core::ffi::c_int = 0;
        match *cc as ::core::ffi::c_int {
            0 => return 0 as ::core::ffi::c_int,
            29 | 30 | 31 | 32 | 33 | 34 | 35 | 36 | 37 | 38 | 39 | 40 | 41 | 42 | 43 | 44 | 45
            | 46 | 47 | 48 | 49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 | 58 | 59 | 60 | 61
            | 62 | 63 | 64 | 65 | 66 | 67 | 68 | 69 | 70 | 71 | 72 | 73 | 74 | 75 | 76 | 77
            | 78 | 79 | 80 | 81 | 82 | 83 | 84 => {
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(*cc as isize)
                        as ::core::ffi::c_int as isize,
                );
                if utf != 0
                    && *cc.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                        >= 0xc0 as ::core::ffi::c_int
                {
                    cc = cc.offset(*(&raw const _pcre2_utf8_table4 as *const uint8_t).offset(
                        (*cc.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_uint
                            & 0x3f as ::core::ffi::c_uint) as isize,
                    ) as ::core::ffi::c_int as isize);
                }
            }
            85 | 86 | 87 | 88 | 89 | 90 | 91 | 92 | 93 | 94 | 95 | 96 | 97 => {
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(*cc as isize)
                        as ::core::ffi::c_int as isize,
                );
                if *cc.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                    == OP_PROP as ::core::ffi::c_int
                    || *cc.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                        == OP_NOTPROP as ::core::ffi::c_int
                {
                    cc = cc.offset(2 as ::core::ffi::c_int as isize);
                }
            }
            112 | 113 => {
                cc = cc.offset(
                    ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *cc.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint
                        as isize,
                );
            }
            156 | 164 | 158 | 160 | 162 => {
                cc = cc.offset(
                    (*(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(*cc as isize)
                        as ::core::ffi::c_int
                        + *cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as isize,
                );
            }
            119 => {
                cb.pattern_position = ((*cc.offset(1 as ::core::ffi::c_int as isize)
                    as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *cc.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                        as ::core::ffi::c_int)
                    as ::core::ffi::c_uint as size_t;
                cb.next_item_length = ((*cc
                    .offset((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as isize)
                    as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *cc.offset(
                        (1 as ::core::ffi::c_int
                            + 2 as ::core::ffi::c_int
                            + 1 as ::core::ffi::c_int) as isize,
                    ) as ::core::ffi::c_int)
                    as ::core::ffi::c_uint as size_t;
                cb.callout_number = *cc.offset(
                    (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * LINK_SIZE) as isize,
                ) as uint32_t;
                cb.callout_string_offset = 0 as size_t;
                cb.callout_string_length = 0 as size_t;
                cb.callout_string = ::core::ptr::null::<PCRE2_UCHAR8>();
                rc = callback.expect("non-null function pointer")(&raw mut cb, callout_data);
                if rc != 0 as ::core::ffi::c_int {
                    return rc;
                }
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(*cc as isize)
                        as ::core::ffi::c_int as isize,
                );
            }
            120 => {
                cb.pattern_position = ((*cc.offset(1 as ::core::ffi::c_int as isize)
                    as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *cc.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                        as ::core::ffi::c_int)
                    as ::core::ffi::c_uint as size_t;
                cb.next_item_length = ((*cc
                    .offset((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as isize)
                    as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *cc.offset(
                        (1 as ::core::ffi::c_int
                            + 2 as ::core::ffi::c_int
                            + 1 as ::core::ffi::c_int) as isize,
                    ) as ::core::ffi::c_int)
                    as ::core::ffi::c_uint as size_t;
                cb.callout_number = 0 as uint32_t;
                cb.callout_string_offset = ((*cc.offset(
                    (1 as ::core::ffi::c_int + 3 as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                        as isize,
                ) as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *cc.offset(
                        (1 as ::core::ffi::c_int
                            + 3 as ::core::ffi::c_int * 2 as ::core::ffi::c_int
                            + 1 as ::core::ffi::c_int) as isize,
                    ) as ::core::ffi::c_int)
                    as ::core::ffi::c_uint as size_t;
                cb.callout_string_length = (((*cc.offset(
                    (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                        as isize,
                ) as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *cc.offset(
                        (1 as ::core::ffi::c_int
                            + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int
                            + 1 as ::core::ffi::c_int) as isize,
                    ) as ::core::ffi::c_int)
                    as ::core::ffi::c_uint)
                    .wrapping_sub(
                        (1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * LINK_SIZE)
                            as ::core::ffi::c_uint,
                    )
                    .wrapping_sub(2 as ::core::ffi::c_uint)
                    as size_t;
                cb.callout_string = cc
                    .offset(
                        (1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * LINK_SIZE) as isize,
                    )
                    .offset(1 as ::core::ffi::c_int as isize);
                rc = callback.expect("non-null function pointer")(&raw mut cb, callout_data);
                if rc != 0 as ::core::ffi::c_int {
                    return rc;
                }
                cc = cc.offset(
                    ((*cc.offset(
                        (1 as ::core::ffi::c_int
                            + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                            as isize,
                    ) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *cc.offset(
                            (1 as ::core::ffi::c_int
                                + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int
                                + 1 as ::core::ffi::c_int) as isize,
                        ) as ::core::ffi::c_int) as ::core::ffi::c_uint
                        as isize,
                );
            }
            _ => {
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(*cc as isize)
                        as ::core::ffi::c_int as isize,
                );
            }
        }
    }
}
pub const PCRE2_CODE_UNIT_WIDTH: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const LINK_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
