// Translated from PCRE2 10.49 pcre2_study.c; code unit width 8.
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
// /* This module contains functions for scanning a compiled pattern and
// collecting data (e.g. minimum matching length). */
extern "C" {
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    static _pcre2_utf8_table4: [uint8_t; 0];
    static _pcre2_OP_lengths_8: [uint8_t; 0];
    static _pcre2_ucd_caseless_sets_8: [uint32_t; 0];
    static _pcre2_ucd_records_8: [ucd_record; 0];
    static _pcre2_ucd_stage1_8: [uint16_t; 0];
    static _pcre2_ucd_stage2_8: [uint16_t; 0];
    fn _pcre2_find_bracket_8(_: PCRE2_SPTR8, _: BOOL, _: ::core::ffi::c_int) -> PCRE2_SPTR8;
    fn _pcre2_ord2utf_8(_: uint32_t, _: *mut PCRE2_UCHAR8) -> ::core::ffi::c_uint;
}
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type size_t = usize;
pub type int32_t = __int32_t;
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
pub const OP_COMMIT_ARG: C2RustUnnamed = 164;
pub const OP_COMMIT: C2RustUnnamed = 163;
pub const OP_THEN_ARG: C2RustUnnamed = 162;
pub const OP_THEN: C2RustUnnamed = 161;
pub const OP_SKIP_ARG: C2RustUnnamed = 160;
pub const OP_SKIP: C2RustUnnamed = 159;
pub const OP_PRUNE_ARG: C2RustUnnamed = 158;
pub const OP_PRUNE: C2RustUnnamed = 157;
pub const OP_MARK: C2RustUnnamed = 156;
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
pub const OP_CALLOUT_STR: C2RustUnnamed = 120;
pub const OP_CALLOUT: C2RustUnnamed = 119;
pub const OP_RECURSE: C2RustUnnamed = 118;
pub const OP_DNREFI: C2RustUnnamed = 117;
pub const OP_DNREF: C2RustUnnamed = 116;
pub const OP_REFI: C2RustUnnamed = 115;
pub const OP_REF: C2RustUnnamed = 114;
pub const OP_ECLASS: C2RustUnnamed = 113;
pub const OP_XCLASS: C2RustUnnamed = 112;
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
pub const OP_PROP: C2RustUnnamed = 16;
pub const OP_NOTPROP: C2RustUnnamed = 15;
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
pub const OP_END: C2RustUnnamed = 0;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct recurse_check {
    pub prev: *mut recurse_check,
    pub group: PCRE2_SPTR8,
}
pub const SSB_DONE: C2RustUnnamed_0 = 1;
pub const SSB_UNKNOWN: C2RustUnnamed_0 = 3;
pub const SSB_FAIL: C2RustUnnamed_0 = 0;
pub const SSB_TOODEEP: C2RustUnnamed_0 = 4;
pub const SSB_CONTINUE: C2RustUnnamed_0 = 2;
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const UINT16_MAX: ::core::ffi::c_int = 65535 as ::core::ffi::c_int;
pub const PCRE2_MATCH_UNSET_BACKREF: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PCRE2_UCP: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PCRE2_UTF: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const NOTACHAR: ::core::ffi::c_uint = 0xffffffff as ::core::ffi::c_uint;
pub const PCRE2_FIRSTSET: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PCRE2_FIRSTCASELESS: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PCRE2_FIRSTMAPSET: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PCRE2_LASTSET: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PCRE2_LASTCASELESS: ::core::ffi::c_uint = 0x100 as ::core::ffi::c_uint;
pub const PCRE2_STARTLINE: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PCRE2_MATCH_EMPTY: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PCRE2_DUPCAPUSED: ::core::ffi::c_uint = 0x200000 as ::core::ffi::c_uint;
pub const PCRE2_HASACCEPT: ::core::ffi::c_uint = 0x800000 as ::core::ffi::c_uint;
pub const cbit_space: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const cbit_digit: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const cbit_word: ::core::ffi::c_int = 160 as ::core::ffi::c_int;
pub const cbits_offset: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
pub const PT_CLIST: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const XCL_NOT: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const XCL_MAP: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const XCL_HASPROP: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const XCL_END: ::core::ffi::c_int = 0;
pub const XCL_SINGLE: ::core::ffi::c_int = 1;
pub const XCL_RANGE: ::core::ffi::c_int = 2;
pub const XCL_CHAR_LIST_LOW_16_START: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_LOW_16_END: ::core::ffi::c_int = 0x7fff as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_LOW_16_ADD: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_HIGH_16_START: ::core::ffi::c_int = 0x8000 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_HIGH_16_END: ::core::ffi::c_int = 0xffff as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_HIGH_16_ADD: ::core::ffi::c_int = 0x8000 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_LOW_32_START: ::core::ffi::c_int = 0x10000 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_LOW_32_ADD: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const XCL_TYPE_MASK: ::core::ffi::c_int = 0xfff as ::core::ffi::c_int;
pub const XCL_TYPE_BIT_LEN: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const XCL_BEGIN_WITH_RANGE: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const XCL_ITEM_COUNT_MASK: ::core::ffi::c_int = 0x3 as ::core::ffi::c_int;
pub const XCL_CHAR_END: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const XCL_CHAR_SHIFT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const IMM2_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const MAX_CACHE_BACKREF: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
unsafe extern "C" fn find_minlength(
    mut re: *const pcre2_real_code_8,
    mut code: PCRE2_SPTR8,
    mut startcode: PCRE2_SPTR8,
    mut utf: BOOL,
    mut recurses: *mut recurse_check,
    mut countptr: *mut ::core::ffi::c_int,
    mut backref_cache: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut length: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    let mut branchlength: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut prev_cap_recno: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    let mut prev_cap_d: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut prev_recurse_recno: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    let mut prev_recurse_d: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut once_fudge: uint32_t = 0 as uint32_t;
    let mut had_recurse: BOOL = FALSE;
    let mut dupcapused: BOOL =
        ((*re).flags & PCRE2_DUPCAPUSED as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    let mut nextbranch: PCRE2_SPTR8 = code.offset(
        ((*code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *code.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int) as ::core::ffi::c_uint as isize,
    );
    let mut cc: PCRE2_SPTR8 = code
        .offset(1 as ::core::ffi::c_int as isize)
        .offset(LINK_SIZE as isize);
    let mut this_recurse: recurse_check = recurse_check {
        prev: ::core::ptr::null_mut::<recurse_check>(),
        group: ::core::ptr::null::<PCRE2_UCHAR8>(),
    };
    if *code as ::core::ffi::c_int >= OP_SBRA as ::core::ffi::c_int
        && *code as ::core::ffi::c_int <= OP_SCOND as ::core::ffi::c_int
    {
        return 0 as ::core::ffi::c_int;
    }
    if *code as ::core::ffi::c_int == OP_CBRA as ::core::ffi::c_int
        || *code as ::core::ffi::c_int == OP_CBRAPOS as ::core::ffi::c_int
    {
        cc = cc.offset(IMM2_SIZE as isize);
    }
    let fresh0 = *countptr;
    *countptr = *countptr + 1;
    if fresh0 > 1000 as ::core::ffi::c_int {
        return -(1 as ::core::ffi::c_int);
    }
    loop {
        let mut d: ::core::ffi::c_int = 0;
        let mut min: ::core::ffi::c_int = 0;
        let mut recno: ::core::ffi::c_int = 0;
        let mut op: PCRE2_UCHAR8 = 0;
        let mut cs: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
        let mut ce: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
        if branchlength >= UINT16_MAX {
            branchlength = UINT16_MAX;
            cc = nextbranch;
        }
        op = *cc;
        let mut current_block_209: u64;
        match op as ::core::ffi::c_int {
            141 | 146 => {
                cs = cc.offset(
                    ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *cc.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint
                        as isize,
                );
                if *cs as ::core::ffi::c_int != OP_ALT as ::core::ffi::c_int {
                    cc = cs
                        .offset(1 as ::core::ffi::c_int as isize)
                        .offset(LINK_SIZE as isize);
                    current_block_209 = 8554725522516090488;
                } else {
                    current_block_209 = 18414934893777006309;
                }
            }
            137 => {
                if *cc.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize) as ::core::ffi::c_int
                    == OP_RECURSE as ::core::ffi::c_int
                    && *cc.offset(
                        (2 as ::core::ffi::c_int * (1 as ::core::ffi::c_int + LINK_SIZE)) as isize,
                    ) as ::core::ffi::c_int
                        == OP_KET as ::core::ffi::c_int
                {
                    once_fudge = (1 as ::core::ffi::c_int + LINK_SIZE) as uint32_t;
                    cc = cc.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                    current_block_209 = 8554725522516090488;
                } else {
                    current_block_209 = 18414934893777006309;
                }
            }
            135 | 136 | 142 | 138 | 143 => {
                current_block_209 = 18414934893777006309;
            }
            139 | 144 | 140 | 145 => {
                recno = ((*cc.offset((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as isize)
                    as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *cc.offset(
                        (1 as ::core::ffi::c_int
                            + 2 as ::core::ffi::c_int
                            + 1 as ::core::ffi::c_int) as isize,
                    ) as ::core::ffi::c_int) as ::core::ffi::c_uint
                    as ::core::ffi::c_int;
                if dupcapused != 0 || recno != prev_cap_recno {
                    prev_cap_recno = recno;
                    prev_cap_d =
                        find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
                    if prev_cap_d < 0 as ::core::ffi::c_int {
                        return prev_cap_d;
                    }
                }
                branchlength += prev_cap_d;
                loop {
                    cc = cc.offset(
                        ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int
                            | *cc.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int) as ::core::ffi::c_uint
                            as isize,
                    );
                    if !(*cc as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                        break;
                    }
                }
                cc = cc.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                current_block_209 = 8554725522516090488;
            }
            166 | 167 => return -(1 as ::core::ffi::c_int),
            121 | 122 | 123 | 124 | 125 | 0 => {
                if length < 0 as ::core::ffi::c_int || had_recurse == 0 && branchlength < length {
                    length = branchlength;
                }
                if op as ::core::ffi::c_int != OP_ALT as ::core::ffi::c_int
                    || length == 0 as ::core::ffi::c_int
                {
                    return length;
                }
                nextbranch = cc.offset(
                    ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *cc.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint
                        as isize,
                );
                cc = cc.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                branchlength = 0 as ::core::ffi::c_int;
                had_recurse = FALSE as BOOL;
                current_block_209 = 8554725522516090488;
            }
            128 | 129 | 130 | 131 | 132 | 134 | 133 => {
                loop {
                    cc = cc.offset(
                        ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int
                            | *cc.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int) as ::core::ffi::c_uint
                            as isize,
                    );
                    if !(*cc as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                        break;
                    }
                }
                current_block_209 = 13407132483876364809;
            }
            126 | 127 | 147 | 148 | 149 | 150 | 151 | 152 | 119 | 1 | 2 | 24 | 23 | 27 | 28
            | 25 | 26 | 4 | 5 | 171 | 172 => {
                current_block_209 = 13407132483876364809;
            }
            120 => {
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
                current_block_209 = 8554725522516090488;
            }
            153 | 154 | 155 | 169 => {
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(*cc as isize)
                        as ::core::ffi::c_int as isize,
                );
                loop {
                    cc = cc.offset(
                        ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int
                            | *cc.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int) as ::core::ffi::c_uint
                            as isize,
                    );
                    if !(*cc as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                        break;
                    }
                }
                cc = cc.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                current_block_209 = 8554725522516090488;
            }
            29 | 30 | 31 | 32 | 35 | 48 | 36 | 49 | 43 | 56 | 61 | 74 | 62 | 75 | 69 | 82 => {
                branchlength += 1;
                cc = cc.offset(2 as ::core::ffi::c_int as isize);
                if utf != 0
                    && *cc.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                        >= 0xc0 as ::core::ffi::c_int
                {
                    cc = cc.offset(*(&raw const _pcre2_utf8_table4 as *const uint8_t).offset(
                        (*cc.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_uint
                            & 0x3f as ::core::ffi::c_uint) as isize,
                    ) as ::core::ffi::c_int as isize);
                }
                current_block_209 = 8554725522516090488;
            }
            87 | 88 | 95 => {
                branchlength += 1;
                cc = cc.offset(
                    (if *cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == OP_PROP as ::core::ffi::c_int
                        || *cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            == OP_NOTPROP as ::core::ffi::c_int
                    {
                        4 as ::core::ffi::c_int
                    } else {
                        2 as ::core::ffi::c_int
                    }) as isize,
                );
                current_block_209 = 8554725522516090488;
            }
            41 | 54 | 67 | 80 => {
                branchlength = (branchlength as ::core::ffi::c_uint).wrapping_add(
                    ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *cc.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint,
                ) as ::core::ffi::c_int as ::core::ffi::c_int;
                cc = cc.offset((2 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                if utf != 0
                    && *cc.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                        >= 0xc0 as ::core::ffi::c_int
                {
                    cc = cc.offset(*(&raw const _pcre2_utf8_table4 as *const uint8_t).offset(
                        (*cc.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_uint
                            & 0x3f as ::core::ffi::c_uint) as isize,
                    ) as ::core::ffi::c_int as isize);
                }
                current_block_209 = 8554725522516090488;
            }
            93 => {
                branchlength = (branchlength as ::core::ffi::c_uint).wrapping_add(
                    ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *cc.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint,
                ) as ::core::ffi::c_int as ::core::ffi::c_int;
                cc = cc.offset(
                    (2 as ::core::ffi::c_int
                        + IMM2_SIZE
                        + (if *cc.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize)
                            as ::core::ffi::c_int
                            == OP_PROP as ::core::ffi::c_int
                            || *cc.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize)
                                as ::core::ffi::c_int
                                == OP_NOTPROP as ::core::ffi::c_int
                        {
                            2 as ::core::ffi::c_int
                        } else {
                            0 as ::core::ffi::c_int
                        })) as isize,
                );
                current_block_209 = 8554725522516090488;
            }
            16 | 15 => {
                cc = cc.offset(2 as ::core::ffi::c_int as isize);
                current_block_209 = 1192054678973511578;
            }
            6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 22 | 19 | 18 | 21 | 20 => {
                current_block_209 = 1192054678973511578;
            }
            17 => {
                branchlength += 1 as ::core::ffi::c_int;
                cc = cc.offset(1);
                current_block_209 = 8554725522516090488;
            }
            14 => {
                if utf != 0 {
                    return -(1 as ::core::ffi::c_int);
                }
                branchlength += 1;
                cc = cc.offset(1);
                current_block_209 = 8554725522516090488;
            }
            85 | 86 | 89 | 90 | 94 | 96 => {
                if *cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    == OP_PROP as ::core::ffi::c_int
                    || *cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == OP_NOTPROP as ::core::ffi::c_int
                {
                    cc = cc.offset(2 as ::core::ffi::c_int as isize);
                }
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(op as isize)
                        as ::core::ffi::c_int as isize,
                );
                current_block_209 = 8554725522516090488;
            }
            91 | 92 | 97 => {
                if *cc.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize) as ::core::ffi::c_int
                    == OP_PROP as ::core::ffi::c_int
                    || *cc.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize)
                        as ::core::ffi::c_int
                        == OP_NOTPROP as ::core::ffi::c_int
                {
                    cc = cc.offset(2 as ::core::ffi::c_int as isize);
                }
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(op as isize)
                        as ::core::ffi::c_int as isize,
                );
                current_block_209 = 8554725522516090488;
            }
            110 | 111 | 112 | 113 => {
                if op as ::core::ffi::c_int == OP_XCLASS as ::core::ffi::c_int
                    || op as ::core::ffi::c_int == OP_ECLASS as ::core::ffi::c_int
                {
                    cc = cc.offset(
                        ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int
                            | *cc.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int) as ::core::ffi::c_uint
                            as isize,
                    );
                } else {
                    cc = cc.offset(
                        *(&raw const _pcre2_OP_lengths_8 as *const uint8_t)
                            .offset(OP_CLASS as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize,
                    );
                }
                let mut current_block_88: u64;
                match *cc as ::core::ffi::c_int {
                    100 | 101 | 107 => {
                        branchlength += 1;
                        current_block_88 = 1831542806378143500;
                    }
                    98 | 99 | 102 | 103 | 106 | 108 => {
                        current_block_88 = 1831542806378143500;
                    }
                    104 | 105 | 109 => {
                        branchlength = (branchlength as ::core::ffi::c_uint).wrapping_add(
                            ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                                << 8 as ::core::ffi::c_int
                                | *cc.offset(
                                    (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                                ) as ::core::ffi::c_int)
                                as ::core::ffi::c_uint,
                        ) as ::core::ffi::c_int
                            as ::core::ffi::c_int;
                        cc = cc.offset(
                            (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                as isize,
                        );
                        current_block_88 = 3567897568976182940;
                    }
                    _ => {
                        branchlength += 1;
                        current_block_88 = 3567897568976182940;
                    }
                }
                match current_block_88 {
                    1831542806378143500 => {
                        cc = cc.offset(1);
                    }
                    _ => {}
                }
                current_block_209 = 8554725522516090488;
            }
            116 | 117 => {
                if dupcapused == 0
                    && (*re).overall_options & PCRE2_MATCH_UNSET_BACKREF as uint32_t
                        == 0 as uint32_t
                {
                    let mut count: ::core::ffi::c_int = ((*cc
                        .offset((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as isize)
                        as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *cc.offset(
                            (1 as ::core::ffi::c_int
                                + 2 as ::core::ffi::c_int
                                + 1 as ::core::ffi::c_int) as isize,
                        ) as ::core::ffi::c_int)
                        as ::core::ffi::c_uint
                        as ::core::ffi::c_int;
                    let mut slot: PCRE2_SPTR8 = ((re as *const uint8_t)
                        .offset(::core::mem::size_of::<pcre2_real_code_8>() as usize as isize)
                        as PCRE2_SPTR8)
                        .offset(
                            (((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                                << 8 as ::core::ffi::c_int
                                | *cc.offset(
                                    (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                                ) as ::core::ffi::c_int)
                                as ::core::ffi::c_uint)
                                .wrapping_mul((*re).name_entry_size as ::core::ffi::c_uint)
                                as isize,
                        );
                    d = INT_MAX;
                    loop {
                        let fresh1 = count;
                        count = count - 1;
                        if !(fresh1 > 0 as ::core::ffi::c_int) {
                            break;
                        }
                        let mut dd: ::core::ffi::c_int = 0;
                        let mut i: ::core::ffi::c_int = 0;
                        recno = ((*slot.offset(0 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int
                            | *slot.offset(
                                (0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int)
                            as ::core::ffi::c_uint
                            as ::core::ffi::c_int;
                        if recno <= *backref_cache.offset(0 as ::core::ffi::c_int as isize)
                            && *backref_cache.offset(recno as isize) >= 0 as ::core::ffi::c_int
                        {
                            dd = *backref_cache.offset(recno as isize);
                        } else {
                            cs = _pcre2_find_bracket_8(startcode, utf, recno);
                            ce = cs;
                            if cs.is_null() {
                                return -(2 as ::core::ffi::c_int);
                            }
                            loop {
                                ce = ce.offset(
                                    ((*ce.offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int)
                                        << 8 as ::core::ffi::c_int
                                        | *ce.offset(
                                            (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                as isize,
                                        )
                                            as ::core::ffi::c_int)
                                        as ::core::ffi::c_uint
                                        as isize,
                                );
                                if !(*ce as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                                    break;
                                }
                            }
                            dd = 0 as ::core::ffi::c_int;
                            if dupcapused == 0 || _pcre2_find_bracket_8(ce, utf, recno).is_null() {
                                if cc > cs && cc < ce {
                                    had_recurse = TRUE as BOOL;
                                } else {
                                    let mut r: *mut recurse_check = recurses;
                                    r = recurses;
                                    while !r.is_null() {
                                        if (*r).group == cs {
                                            break;
                                        }
                                        r = (*r).prev as *mut recurse_check;
                                    }
                                    if !r.is_null() {
                                        had_recurse = TRUE as BOOL;
                                    } else {
                                        this_recurse.prev = recurses as *mut recurse_check;
                                        this_recurse.group = cs;
                                        dd = find_minlength(
                                            re,
                                            cs,
                                            startcode,
                                            utf,
                                            &raw mut this_recurse,
                                            countptr,
                                            backref_cache,
                                        );
                                        if dd < 0 as ::core::ffi::c_int {
                                            return dd;
                                        }
                                    }
                                }
                            }
                            *backref_cache.offset(recno as isize) = dd;
                            i = *backref_cache.offset(0 as ::core::ffi::c_int as isize)
                                + 1 as ::core::ffi::c_int;
                            while i < recno {
                                *backref_cache.offset(i as isize) = -(1 as ::core::ffi::c_int);
                                i += 1;
                            }
                            *backref_cache.offset(0 as ::core::ffi::c_int as isize) = recno;
                        }
                        if dd < d {
                            d = dd;
                        }
                        if d <= 0 as ::core::ffi::c_int {
                            break;
                        }
                        slot = slot.offset((*re).name_entry_size as ::core::ffi::c_int as isize);
                    }
                } else {
                    d = 0 as ::core::ffi::c_int;
                }
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(*cc as isize)
                        as ::core::ffi::c_int as isize,
                );
                current_block_209 = 1687459899864017148;
            }
            114 | 115 => {
                recno = ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *cc.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                        as ::core::ffi::c_int) as ::core::ffi::c_uint
                    as ::core::ffi::c_int;
                if recno <= *backref_cache.offset(0 as ::core::ffi::c_int as isize)
                    && *backref_cache.offset(recno as isize) >= 0 as ::core::ffi::c_int
                {
                    d = *backref_cache.offset(recno as isize);
                } else {
                    let mut i_0: ::core::ffi::c_int = 0;
                    d = 0 as ::core::ffi::c_int;
                    if (*re).overall_options & PCRE2_MATCH_UNSET_BACKREF as uint32_t
                        == 0 as uint32_t
                    {
                        cs = _pcre2_find_bracket_8(startcode, utf, recno);
                        ce = cs;
                        if cs.is_null() {
                            return -(2 as ::core::ffi::c_int);
                        }
                        loop {
                            ce = ce.offset(
                                ((*ce.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int)
                                    << 8 as ::core::ffi::c_int
                                    | *ce.offset(
                                        (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                            as isize,
                                    ) as ::core::ffi::c_int)
                                    as ::core::ffi::c_uint as isize,
                            );
                            if !(*ce as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                                break;
                            }
                        }
                        if dupcapused == 0 || _pcre2_find_bracket_8(ce, utf, recno).is_null() {
                            if cc > cs && cc < ce {
                                had_recurse = TRUE as BOOL;
                            } else {
                                let mut r_0: *mut recurse_check = recurses;
                                r_0 = recurses;
                                while !r_0.is_null() {
                                    if (*r_0).group == cs {
                                        break;
                                    }
                                    r_0 = (*r_0).prev as *mut recurse_check;
                                }
                                if !r_0.is_null() {
                                    had_recurse = TRUE as BOOL;
                                } else {
                                    this_recurse.prev = recurses as *mut recurse_check;
                                    this_recurse.group = cs;
                                    d = find_minlength(
                                        re,
                                        cs,
                                        startcode,
                                        utf,
                                        &raw mut this_recurse,
                                        countptr,
                                        backref_cache,
                                    );
                                    if d < 0 as ::core::ffi::c_int {
                                        return d;
                                    }
                                }
                            }
                        }
                    }
                    *backref_cache.offset(recno as isize) = d;
                    i_0 = *backref_cache.offset(0 as ::core::ffi::c_int as isize)
                        + 1 as ::core::ffi::c_int;
                    while i_0 < recno {
                        *backref_cache.offset(i_0 as isize) = -(1 as ::core::ffi::c_int);
                        i_0 += 1;
                    }
                    *backref_cache.offset(0 as ::core::ffi::c_int as isize) = recno;
                }
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(*cc as isize)
                        as ::core::ffi::c_int as isize,
                );
                current_block_209 = 1687459899864017148;
            }
            118 => {
                ce = startcode.offset(
                    ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *cc.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint
                        as isize,
                );
                cs = ce;
                recno = ((*cs.offset((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as isize)
                    as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *cs.offset(
                        (1 as ::core::ffi::c_int
                            + 2 as ::core::ffi::c_int
                            + 1 as ::core::ffi::c_int) as isize,
                    ) as ::core::ffi::c_int) as ::core::ffi::c_uint
                    as ::core::ffi::c_int;
                if recno == prev_recurse_recno {
                    branchlength += prev_recurse_d;
                } else {
                    loop {
                        ce = ce.offset(
                            ((*ce.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                                << 8 as ::core::ffi::c_int
                                | *ce.offset(
                                    (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                                ) as ::core::ffi::c_int)
                                as ::core::ffi::c_uint as isize,
                        );
                        if !(*ce as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                            break;
                        }
                    }
                    if cc > cs && cc < ce {
                        had_recurse = TRUE as BOOL;
                    } else {
                        let mut r_1: *mut recurse_check = recurses;
                        r_1 = recurses;
                        while !r_1.is_null() {
                            if (*r_1).group == cs {
                                break;
                            }
                            r_1 = (*r_1).prev as *mut recurse_check;
                        }
                        if !r_1.is_null() {
                            had_recurse = TRUE as BOOL;
                        } else {
                            this_recurse.prev = recurses as *mut recurse_check;
                            this_recurse.group = cs;
                            prev_recurse_d = find_minlength(
                                re,
                                cs,
                                startcode,
                                utf,
                                &raw mut this_recurse,
                                countptr,
                                backref_cache,
                            );
                            if prev_recurse_d < 0 as ::core::ffi::c_int {
                                return prev_recurse_d;
                            }
                            prev_recurse_recno = recno;
                            branchlength += prev_recurse_d;
                        }
                    }
                }
                cc = cc.offset(
                    ((1 as ::core::ffi::c_int + LINK_SIZE) as uint32_t).wrapping_add(once_fudge)
                        as isize,
                );
                once_fudge = 0 as uint32_t;
                current_block_209 = 8554725522516090488;
            }
            39 | 52 | 65 | 78 | 40 | 53 | 66 | 79 | 45 | 58 | 71 | 84 | 33 | 46 | 59 | 72 | 34
            | 47 | 60 | 73 | 42 | 55 | 68 | 81 | 37 | 50 | 63 | 76 | 38 | 51 | 64 | 77 | 44
            | 57 | 70 | 83 => {
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(op as isize)
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
                current_block_209 = 8554725522516090488;
            }
            156 | 164 | 158 | 160 | 162 => {
                cc = cc.offset(
                    (*(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(op as isize)
                        as ::core::ffi::c_int
                        + *cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as isize,
                );
                current_block_209 = 8554725522516090488;
            }
            168 | 163 | 165 | 157 | 3 | 159 | 161 => {
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(op as isize)
                        as ::core::ffi::c_int as isize,
                );
                current_block_209 = 8554725522516090488;
            }
            _ => return -(3 as ::core::ffi::c_int),
        }
        match current_block_209 {
            1687459899864017148 => {
                match *cc as ::core::ffi::c_int {
                    98 | 99 | 102 | 103 | 106 | 108 => {
                        min = 0 as ::core::ffi::c_int;
                        cc = cc.offset(1);
                    }
                    100 | 101 | 107 => {
                        min = 1 as ::core::ffi::c_int;
                        cc = cc.offset(1);
                    }
                    104 | 105 | 109 => {
                        min = ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int
                            | *cc.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int)
                            as ::core::ffi::c_uint
                            as ::core::ffi::c_int;
                        cc = cc.offset(
                            (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                as isize,
                        );
                    }
                    _ => {
                        min = 1 as ::core::ffi::c_int;
                    }
                }
                if d > 0 as ::core::ffi::c_int && INT_MAX / d < min
                    || UINT16_MAX - branchlength < min * d
                {
                    branchlength = UINT16_MAX;
                } else {
                    branchlength += min * d;
                }
            }
            1192054678973511578 => {
                branchlength += 1;
                cc = cc.offset(1);
            }
            18414934893777006309 => {
                d = find_minlength(re, cc, startcode, utf, recurses, countptr, backref_cache);
                if d < 0 as ::core::ffi::c_int {
                    return d;
                }
                branchlength += d;
                loop {
                    cc = cc.offset(
                        ((*cc.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int
                            | *cc.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int) as ::core::ffi::c_uint
                            as isize,
                    );
                    if !(*cc as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                        break;
                    }
                }
                cc = cc.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
            }
            13407132483876364809 => {
                cc = cc.offset(
                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(*cc as isize)
                        as ::core::ffi::c_int as isize,
                );
            }
            _ => {}
        }
    }
}
unsafe extern "C" fn set_table_bit(
    mut re: *mut pcre2_real_code_8,
    mut p: PCRE2_SPTR8,
    mut caseless: BOOL,
    mut utf: BOOL,
    mut ucp: BOOL,
) -> PCRE2_SPTR8 {
    let fresh10 = p;
    p = p.offset(1);
    let mut c: uint32_t = *fresh10 as uint32_t;
    (*re).start_bitmap[c.wrapping_div(8 as uint32_t) as usize] =
        ((*re).start_bitmap[c.wrapping_div(8 as uint32_t) as usize] as ::core::ffi::c_uint
            | (1 as ::core::ffi::c_uint) << (c & 7 as uint32_t)) as uint8_t;
    if utf != 0 {
        if c >= 0xc0 as uint32_t {
            if c & 0x20 as uint32_t == 0 as uint32_t {
                let fresh11 = p;
                p = p.offset(1);
                c = (c & 0x1f as uint32_t) << 6 as ::core::ffi::c_int
                    | *fresh11 as uint32_t & 0x3f as uint32_t;
            } else if c & 0x10 as uint32_t == 0 as uint32_t {
                c = (c & 0xf as uint32_t) << 12 as ::core::ffi::c_int
                    | (*p as uint32_t & 0x3f as uint32_t) << 6 as ::core::ffi::c_int
                    | *p.offset(1 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
                p = p.offset(2 as ::core::ffi::c_int as isize);
            } else if c & 0x8 as uint32_t == 0 as uint32_t {
                c = (c & 0x7 as uint32_t) << 18 as ::core::ffi::c_int
                    | (*p as uint32_t & 0x3f as uint32_t) << 12 as ::core::ffi::c_int
                    | (*p.offset(1 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                        << 6 as ::core::ffi::c_int
                    | *p.offset(2 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
                p = p.offset(3 as ::core::ffi::c_int as isize);
            } else if c & 0x4 as uint32_t == 0 as uint32_t {
                c = (c & 0x3 as uint32_t) << 24 as ::core::ffi::c_int
                    | (*p as uint32_t & 0x3f as uint32_t) << 18 as ::core::ffi::c_int
                    | (*p.offset(1 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                        << 12 as ::core::ffi::c_int
                    | (*p.offset(2 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                        << 6 as ::core::ffi::c_int
                    | *p.offset(3 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
                p = p.offset(4 as ::core::ffi::c_int as isize);
            } else {
                c = (c & 0x1 as uint32_t) << 30 as ::core::ffi::c_int
                    | (*p as uint32_t & 0x3f as uint32_t) << 24 as ::core::ffi::c_int
                    | (*p.offset(1 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                        << 18 as ::core::ffi::c_int
                    | (*p.offset(2 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                        << 12 as ::core::ffi::c_int
                    | (*p.offset(3 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                        << 6 as ::core::ffi::c_int
                    | *p.offset(4 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
                p = p.offset(5 as ::core::ffi::c_int as isize);
            }
        }
    }
    if caseless != 0 {
        if utf != 0 || ucp != 0 {
            c = (c as ::core::ffi::c_int
                + (*(&raw const _pcre2_ucd_records_8 as *const ucd_record).offset(
                    *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t).offset(
                        (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                            .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                            as ::core::ffi::c_int
                            * UCD_BLOCK_SIZE
                            + c as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                            as isize,
                    ) as ::core::ffi::c_int as isize,
                ))
                .other_case as ::core::ffi::c_int) as uint32_t;
            if utf != 0 {
                let mut buff: [PCRE2_UCHAR8; 6] = [0; 6];
                _pcre2_ord2utf_8(c, &raw mut buff as *mut PCRE2_UCHAR8);
                (*re).start_bitmap[(buff[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    / 8 as ::core::ffi::c_int) as usize] =
                    ((*re).start_bitmap[(buff[0 as ::core::ffi::c_int as usize]
                        as ::core::ffi::c_int
                        / 8 as ::core::ffi::c_int) as usize]
                        as ::core::ffi::c_uint
                        | (1 as ::core::ffi::c_uint)
                            << (buff[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                                & 7 as ::core::ffi::c_int)) as uint8_t;
            } else if c < 256 as uint32_t {
                (*re).start_bitmap[c.wrapping_div(8 as uint32_t) as usize] = ((*re).start_bitmap
                    [c.wrapping_div(8 as uint32_t) as usize]
                    as ::core::ffi::c_uint
                    | (1 as ::core::ffi::c_uint) << (c & 7 as uint32_t))
                    as uint8_t;
            }
        } else {
            (*re).start_bitmap[(*(*re)
                .tables
                .offset((256 as uint32_t).wrapping_add(c) as isize)
                as ::core::ffi::c_int
                / 8 as ::core::ffi::c_int) as usize] =
                ((*re).start_bitmap[(*(*re)
                    .tables
                    .offset((256 as uint32_t).wrapping_add(c) as isize)
                    as ::core::ffi::c_int
                    / 8 as ::core::ffi::c_int) as usize] as ::core::ffi::c_uint
                    | (1 as ::core::ffi::c_uint)
                        << (*(*re)
                            .tables
                            .offset((256 as uint32_t).wrapping_add(c) as isize)
                            as ::core::ffi::c_int
                            & 7 as ::core::ffi::c_int)) as uint8_t;
        }
    }
    return p;
}
unsafe extern "C" fn set_type_bits(
    mut re: *mut pcre2_real_code_8,
    mut cbit_type: ::core::ffi::c_int,
    mut table_limit: ::core::ffi::c_uint,
) {
    let mut c: uint32_t = 0;
    c = 0 as uint32_t;
    while c < table_limit as uint32_t {
        (*re).start_bitmap[c as usize] = ((*re).start_bitmap[c as usize] as ::core::ffi::c_int
            | *(*re).tables.offset(
                c.wrapping_add(cbits_offset as uint32_t)
                    .wrapping_add(cbit_type as uint32_t) as isize,
            ) as ::core::ffi::c_int) as uint8_t;
        c = c.wrapping_add(1);
    }
    if table_limit == 32 as ::core::ffi::c_uint {
        return;
    }
    c = 128 as uint32_t;
    while c < 256 as uint32_t {
        if *(*re)
            .tables
            .offset((cbits_offset as uint32_t).wrapping_add(c.wrapping_div(8 as uint32_t)) as isize)
            as ::core::ffi::c_uint
            & (1 as ::core::ffi::c_uint) << (c & 7 as uint32_t)
            != 0 as ::core::ffi::c_uint
        {
            let mut buff: [PCRE2_UCHAR8; 6] = [0; 6];
            _pcre2_ord2utf_8(c, &raw mut buff as *mut PCRE2_UCHAR8);
            (*re).start_bitmap[(buff[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                / 8 as ::core::ffi::c_int) as usize] =
                ((*re).start_bitmap[(buff[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    / 8 as ::core::ffi::c_int) as usize] as ::core::ffi::c_uint
                    | (1 as ::core::ffi::c_uint)
                        << (buff[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                            & 7 as ::core::ffi::c_int)) as uint8_t;
        }
        c = c.wrapping_add(1);
    }
}
unsafe extern "C" fn set_nottype_bits(
    mut re: *mut pcre2_real_code_8,
    mut cbit_type: ::core::ffi::c_int,
    mut table_limit: ::core::ffi::c_uint,
) {
    let mut c: uint32_t = 0;
    c = 0 as uint32_t;
    while c < table_limit as uint32_t {
        (*re).start_bitmap[c as usize] = ((*re).start_bitmap[c as usize] as ::core::ffi::c_int
            | !(*(*re).tables.offset(
                c.wrapping_add(cbits_offset as uint32_t)
                    .wrapping_add(cbit_type as uint32_t) as isize,
            ) as ::core::ffi::c_int) as uint8_t as ::core::ffi::c_int)
            as uint8_t;
        c = c.wrapping_add(1);
    }
    if table_limit != 32 as ::core::ffi::c_uint {
        c = 24 as uint32_t;
        while c < 32 as uint32_t {
            (*re).start_bitmap[c as usize] = 0xff as uint8_t;
            c = c.wrapping_add(1);
        }
    }
}
unsafe extern "C" fn study_char_list(
    mut code: PCRE2_SPTR8,
    mut start_bitmap: *mut uint8_t,
    mut char_lists_end: *const uint8_t,
) {
    let mut type_0: uint32_t = 0;
    let mut list_ind: uint32_t = 0;
    let mut char_list_add: uint32_t = XCL_CHAR_LIST_LOW_16_ADD as uint32_t;
    let mut range_start: uint32_t = !(0 as ::core::ffi::c_int as uint32_t);
    let mut range_end: uint32_t = 0 as uint32_t;
    let mut next_char: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut start_buffer: [PCRE2_UCHAR8; 6] = [0; 6];
    let mut end_buffer: [PCRE2_UCHAR8; 6] = [0; 6];
    let mut start: PCRE2_UCHAR8 = 0;
    let mut end: PCRE2_UCHAR8 = 0;
    type_0 = ((*code.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
        << 8 as ::core::ffi::c_int) as uint32_t
        | *code.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
    code = code.offset(2 as ::core::ffi::c_int as isize);
    next_char = char_lists_end.offset(
        -(((((*code.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *code.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int) as ::core::ffi::c_uint)
            << 1 as ::core::ffi::c_int) as isize),
    );
    type_0 &= XCL_TYPE_MASK as uint32_t;
    list_ind = 0 as uint32_t;
    if type_0 & XCL_BEGIN_WITH_RANGE as uint32_t != 0 as uint32_t {
        range_start = XCL_CHAR_LIST_LOW_16_START as uint32_t;
    }
    while type_0 > 0 as uint32_t {
        let mut item_count: uint32_t = type_0 & XCL_ITEM_COUNT_MASK as uint32_t;
        if item_count == XCL_ITEM_COUNT_MASK as uint32_t {
            if list_ind <= 1 as uint32_t {
                item_count = *(next_char as *const uint16_t) as uint32_t;
                next_char = next_char.offset(2 as ::core::ffi::c_int as isize);
            } else {
                item_count = *(next_char as *const uint32_t);
                next_char = next_char.offset(4 as ::core::ffi::c_int as isize);
            }
        }
        while item_count > 0 as uint32_t {
            if list_ind <= 1 as uint32_t {
                range_end = *(next_char as *const uint16_t) as uint32_t;
                next_char = next_char.offset(2 as ::core::ffi::c_int as isize);
            } else {
                range_end = *(next_char as *const uint32_t);
                next_char = next_char.offset(4 as ::core::ffi::c_int as isize);
            }
            if range_end & XCL_CHAR_END as uint32_t != 0 as uint32_t {
                range_end = char_list_add.wrapping_add(range_end >> XCL_CHAR_SHIFT);
                _pcre2_ord2utf_8(range_end, &raw mut end_buffer as *mut PCRE2_UCHAR8);
                end = end_buffer[0 as ::core::ffi::c_int as usize];
                if range_start < range_end {
                    _pcre2_ord2utf_8(range_start, &raw mut start_buffer as *mut PCRE2_UCHAR8);
                    start = start_buffer[0 as ::core::ffi::c_int as usize];
                    while start as ::core::ffi::c_int <= end as ::core::ffi::c_int {
                        let ref mut fresh7 = *start_bitmap.offset(
                            (start as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as isize,
                        );
                        *fresh7 = (*fresh7 as ::core::ffi::c_uint
                            | (1 as ::core::ffi::c_uint)
                                << (start as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                            as uint8_t;
                        start = start.wrapping_add(1);
                    }
                } else {
                    let ref mut fresh8 = *start_bitmap
                        .offset((end as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as isize);
                    *fresh8 = (*fresh8 as ::core::ffi::c_uint
                        | (1 as ::core::ffi::c_uint)
                            << (end as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                        as uint8_t;
                }
                range_start = !(0 as ::core::ffi::c_int as uint32_t);
            } else {
                range_start = char_list_add.wrapping_add(range_end >> XCL_CHAR_SHIFT);
            }
            item_count = item_count.wrapping_sub(1);
        }
        list_ind = list_ind.wrapping_add(1);
        type_0 >>= XCL_TYPE_BIT_LEN;
        if range_start == !(0 as ::core::ffi::c_int as uint32_t) {
            if type_0 & XCL_BEGIN_WITH_RANGE as uint32_t != 0 as uint32_t {
                if list_ind == 1 as uint32_t {
                    range_start = XCL_CHAR_LIST_HIGH_16_START as uint32_t;
                } else {
                    range_start = XCL_CHAR_LIST_LOW_32_START as uint32_t;
                }
            }
        } else if type_0 & XCL_BEGIN_WITH_RANGE as uint32_t == 0 as uint32_t {
            _pcre2_ord2utf_8(range_start, &raw mut start_buffer as *mut PCRE2_UCHAR8);
            if list_ind == 1 as uint32_t {
                range_end = XCL_CHAR_LIST_LOW_16_END as uint32_t;
            } else {
                range_end = XCL_CHAR_LIST_HIGH_16_END as uint32_t;
            }
            _pcre2_ord2utf_8(range_end, &raw mut end_buffer as *mut PCRE2_UCHAR8);
            end = end_buffer[0 as ::core::ffi::c_int as usize];
            start = start_buffer[0 as ::core::ffi::c_int as usize];
            while start as ::core::ffi::c_int <= end as ::core::ffi::c_int {
                let ref mut fresh9 = *start_bitmap
                    .offset((start as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as isize);
                *fresh9 = (*fresh9 as ::core::ffi::c_uint
                    | (1 as ::core::ffi::c_uint)
                        << (start as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                    as uint8_t;
                start = start.wrapping_add(1);
            }
            range_start = !(0 as ::core::ffi::c_int as uint32_t);
        }
        if list_ind == 1 as uint32_t {
            char_list_add = XCL_CHAR_LIST_HIGH_16_ADD as uint32_t;
        } else {
            char_list_add = XCL_CHAR_LIST_LOW_32_ADD as uint32_t;
        }
    }
}
unsafe extern "C" fn set_start_bits(
    mut re: *mut pcre2_real_code_8,
    mut code: PCRE2_SPTR8,
    mut utf: BOOL,
    mut ucp: BOOL,
    mut depthptr: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut c: uint32_t = 0;
    let mut yield_0: ::core::ffi::c_int = SSB_DONE as ::core::ffi::c_int;
    let mut table_limit: ::core::ffi::c_int = if utf != 0 {
        16 as ::core::ffi::c_int
    } else {
        32 as ::core::ffi::c_int
    };
    *depthptr += 1 as ::core::ffi::c_int;
    if *depthptr > 1000 as ::core::ffi::c_int {
        return SSB_TOODEEP as ::core::ffi::c_int;
    }
    loop {
        let mut try_next: BOOL = TRUE;
        let mut tcode: PCRE2_SPTR8 = code
            .offset(1 as ::core::ffi::c_int as isize)
            .offset(LINK_SIZE as isize);
        if *code as ::core::ffi::c_int == OP_CBRA as ::core::ffi::c_int
            || *code as ::core::ffi::c_int == OP_SCBRA as ::core::ffi::c_int
            || *code as ::core::ffi::c_int == OP_CBRAPOS as ::core::ffi::c_int
            || *code as ::core::ffi::c_int == OP_SCBRAPOS as ::core::ffi::c_int
        {
            tcode = tcode.offset(IMM2_SIZE as isize);
        }
        while try_next != 0 {
            let mut rc: ::core::ffi::c_int = 0;
            let mut ncode: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
            let mut classmap: *const uint8_t = ::core::ptr::null::<uint8_t>();
            let mut xclassflags: PCRE2_UCHAR8 = 0;
            match *tcode as ::core::ffi::c_int {
                166 | 167 | 13 | 12 | 14 | 28 | 168 | 163 | 164 | 141 | 147 | 151 | 152 | 148
                | 116 | 117 | 150 | 25 | 26 | 0 | 24 | 23 | 22 | 165 | 156 | 31 | 67 | 80 | 32
                | 62 | 75 | 64 | 77 | 60 | 73 | 66 | 79 | 61 | 74 | 69 | 82 | 70 | 83 | 68 | 81
                | 71 | 84 | 15 | 63 | 76 | 59 | 72 | 65 | 78 | 18 | 20 | 157 | 158 | 118 | 114
                | 115 | 126 | 127 | 149 | 146 | 3 | 159 | 160 | 1 | 2 | 161 | 162 => {
                    return SSB_FAIL as ::core::ffi::c_int
                }
                27 => {
                    tcode = tcode.offset(
                        *(&raw const _pcre2_OP_lengths_8 as *const uint8_t)
                            .offset(OP_CIRC as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize,
                    );
                    continue;
                }
                16 => {
                    if *tcode.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        != PT_CLIST
                    {
                        return SSB_FAIL as ::core::ffi::c_int;
                    }
                    let mut p: *const uint32_t = (&raw const _pcre2_ucd_caseless_sets_8
                        as *const uint32_t)
                        .offset(*tcode.offset(2 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize);
                    loop {
                        let fresh2 = p;
                        p = p.offset(1);
                        c = *fresh2;
                        if !(c < NOTACHAR as uint32_t) {
                            break;
                        }
                        if utf != 0 {
                            let mut buff: [PCRE2_UCHAR8; 6] = [0; 6];
                            _pcre2_ord2utf_8(c, &raw mut buff as *mut PCRE2_UCHAR8);
                            c = buff[0 as ::core::ffi::c_int as usize] as uint32_t;
                        }
                        if c > 0xff as uint32_t {
                            (*re).start_bitmap
                                [(0xff as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize] =
                                ((*re).start_bitmap[(0xff as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << (0xff as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                        } else {
                            (*re).start_bitmap[c.wrapping_div(8 as uint32_t) as usize] =
                                ((*re).start_bitmap[c.wrapping_div(8 as uint32_t) as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint) << (c & 7 as uint32_t))
                                    as uint8_t;
                        }
                    }
                    try_next = FALSE as BOOL;
                    continue;
                }
                5 | 4 | 172 | 171 => {
                    tcode = tcode.offset(1);
                    continue;
                }
                128 | 132 => {
                    ncode = tcode.offset(
                        ((*tcode.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int
                            | *tcode.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int) as ::core::ffi::c_uint
                            as isize,
                    );
                    while *ncode as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int {
                        ncode = ncode.offset(
                            ((*ncode.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int)
                                << 8 as ::core::ffi::c_int
                                | *ncode.offset(
                                    (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                                ) as ::core::ffi::c_int)
                                as ::core::ffi::c_uint as isize,
                        );
                    }
                    ncode = ncode.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                    let mut done: BOOL = FALSE;
                    while done == 0 {
                        match *ncode as ::core::ffi::c_int {
                            128 | 129 | 130 | 131 | 132 | 133 | 134 => {
                                ncode = ncode.offset(
                                    ((*ncode.offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int)
                                        << 8 as ::core::ffi::c_int
                                        | *ncode.offset(
                                            (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                as isize,
                                        )
                                            as ::core::ffi::c_int)
                                        as ::core::ffi::c_uint
                                        as isize,
                                );
                                while *ncode as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int {
                                    ncode = ncode.offset(
                                        ((*ncode.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int)
                                            << 8 as ::core::ffi::c_int
                                            | *ncode.offset(
                                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                    as isize,
                                            )
                                                as ::core::ffi::c_int)
                                            as ::core::ffi::c_uint
                                            as isize,
                                    );
                                }
                                ncode =
                                    ncode.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                            }
                            5 | 4 | 172 | 171 => {
                                ncode = ncode.offset(1);
                            }
                            119 => {
                                ncode = ncode.offset(
                                    *(&raw const _pcre2_OP_lengths_8 as *const uint8_t)
                                        .offset(OP_CALLOUT as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        as isize,
                                );
                            }
                            120 => {
                                ncode = ncode.offset(
                                    ((*ncode.offset(
                                        (1 as ::core::ffi::c_int
                                            + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                                            as isize,
                                    ) as ::core::ffi::c_int)
                                        << 8 as ::core::ffi::c_int
                                        | *ncode.offset(
                                            (1 as ::core::ffi::c_int
                                                + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int
                                                + 1 as ::core::ffi::c_int)
                                                as isize,
                                        )
                                            as ::core::ffi::c_int)
                                        as ::core::ffi::c_uint
                                        as isize,
                                );
                            }
                            _ => {
                                done = TRUE as BOOL;
                            }
                        }
                    }
                    match *ncode as ::core::ffi::c_int {
                        16 => {
                            current_block = 12197303086394414207;
                            match current_block {
                                12197303086394414207 => {
                                    if *ncode.offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        != PT_CLIST
                                    {
                                        current_block = 15858484809446604407;
                                    } else {
                                        current_block = 9032869313945290734;
                                    }
                                }
                                _ => {}
                            }
                            match current_block {
                                15858484809446604407 => {}
                                _ => {
                                    tcode = ncode;
                                    continue;
                                }
                            }
                        }
                        17 | 29 | 30 | 41 | 54 | 19 | 36 | 49 | 35 | 48 | 43 | 56 | 21 | 7 | 6
                        | 11 | 10 | 9 | 8 => {
                            current_block = 9032869313945290734;
                            match current_block {
                                12197303086394414207 => {
                                    if *ncode.offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        != PT_CLIST
                                    {
                                        current_block = 15858484809446604407;
                                    } else {
                                        current_block = 9032869313945290734;
                                    }
                                }
                                _ => {}
                            }
                            match current_block {
                                15858484809446604407 => {}
                                _ => {
                                    tcode = ncode;
                                    continue;
                                }
                            }
                        }
                        _ => {
                            current_block = 15858484809446604407;
                        }
                    }
                }
                137 | 142 | 139 | 144 | 138 | 143 | 140 | 145 | 135 | 136 => {
                    current_block = 15858484809446604407;
                }
                121 => {
                    yield_0 = SSB_CONTINUE as ::core::ffi::c_int;
                    try_next = FALSE as BOOL;
                    continue;
                }
                122 | 123 | 124 | 125 => return SSB_CONTINUE as ::core::ffi::c_int,
                119 => {
                    tcode = tcode.offset(
                        *(&raw const _pcre2_OP_lengths_8 as *const uint8_t)
                            .offset(OP_CALLOUT as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize,
                    );
                    continue;
                }
                120 => {
                    tcode = tcode.offset(
                        ((*tcode.offset(
                            (1 as ::core::ffi::c_int
                                + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                                as isize,
                        ) as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int
                            | *tcode.offset(
                                (1 as ::core::ffi::c_int
                                    + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int
                                    + 1 as ::core::ffi::c_int)
                                    as isize,
                            ) as ::core::ffi::c_int) as ::core::ffi::c_uint
                            as isize,
                    );
                    continue;
                }
                129 | 130 | 131 | 133 | 134 => {
                    loop {
                        tcode = tcode.offset(
                            ((*tcode.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int)
                                << 8 as ::core::ffi::c_int
                                | *tcode.offset(
                                    (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                                ) as ::core::ffi::c_int)
                                as ::core::ffi::c_uint as isize,
                        );
                        if !(*tcode as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                            break;
                        }
                    }
                    tcode = tcode.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                    continue;
                }
                153 | 154 | 155 => {
                    tcode = tcode.offset(1);
                    rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                    if rc == SSB_FAIL as ::core::ffi::c_int
                        || rc == SSB_UNKNOWN as ::core::ffi::c_int
                        || rc == SSB_TOODEEP as ::core::ffi::c_int
                    {
                        return rc;
                    }
                    loop {
                        tcode = tcode.offset(
                            ((*tcode.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int)
                                << 8 as ::core::ffi::c_int
                                | *tcode.offset(
                                    (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                                ) as ::core::ffi::c_int)
                                as ::core::ffi::c_uint as isize,
                        );
                        if !(*tcode as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                            break;
                        }
                    }
                    tcode = tcode.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                    continue;
                }
                169 => {
                    tcode = tcode.offset(1);
                    loop {
                        tcode = tcode.offset(
                            ((*tcode.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int)
                                << 8 as ::core::ffi::c_int
                                | *tcode.offset(
                                    (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                                ) as ::core::ffi::c_int)
                                as ::core::ffi::c_uint as isize,
                        );
                        if !(*tcode as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                            break;
                        }
                    }
                    tcode = tcode.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                    continue;
                }
                33 | 34 | 42 | 37 | 38 | 44 => {
                    tcode = set_table_bit(
                        re,
                        tcode.offset(1 as ::core::ffi::c_int as isize),
                        FALSE,
                        utf,
                        ucp,
                    );
                    continue;
                }
                46 | 47 | 55 | 50 | 51 | 57 => {
                    tcode = set_table_bit(
                        re,
                        tcode.offset(1 as ::core::ffi::c_int as isize),
                        TRUE,
                        utf,
                        ucp,
                    );
                    continue;
                }
                39 | 40 | 45 => {
                    tcode = set_table_bit(
                        re,
                        tcode
                            .offset(1 as ::core::ffi::c_int as isize)
                            .offset(IMM2_SIZE as isize),
                        FALSE,
                        utf,
                        ucp,
                    );
                    continue;
                }
                52 | 53 | 58 => {
                    tcode = set_table_bit(
                        re,
                        tcode
                            .offset(1 as ::core::ffi::c_int as isize)
                            .offset(IMM2_SIZE as isize),
                        TRUE,
                        utf,
                        ucp,
                    );
                    continue;
                }
                41 => {
                    tcode = tcode.offset(IMM2_SIZE as isize);
                    current_block = 4613046691852425349;
                }
                29 | 35 | 36 | 43 => {
                    current_block = 4613046691852425349;
                }
                54 => {
                    tcode = tcode.offset(IMM2_SIZE as isize);
                    current_block = 18023239131261601843;
                }
                30 | 48 | 49 | 56 => {
                    current_block = 18023239131261601843;
                }
                19 => {
                    (*re).start_bitmap[('\t' as i32 / 8 as ::core::ffi::c_int) as usize] = ((*re)
                        .start_bitmap[('\t' as i32 / 8 as ::core::ffi::c_int) as usize]
                        as ::core::ffi::c_uint
                        | (1 as ::core::ffi::c_uint) << ('\t' as i32 & 7 as ::core::ffi::c_int))
                        as uint8_t;
                    (*re).start_bitmap[(' ' as i32 / 8 as ::core::ffi::c_int) as usize] = ((*re)
                        .start_bitmap[(' ' as i32 / 8 as ::core::ffi::c_int) as usize]
                        as ::core::ffi::c_uint
                        | (1 as ::core::ffi::c_uint) << (' ' as i32 & 7 as ::core::ffi::c_int))
                        as uint8_t;
                    if utf != 0 {
                        (*re).start_bitmap
                            [(0xc2 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize] =
                            ((*re).start_bitmap
                                [(0xc2 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize]
                                as ::core::ffi::c_uint
                                | (1 as ::core::ffi::c_uint)
                                    << (0xc2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                as uint8_t;
                        (*re).start_bitmap
                            [(0xe1 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize] =
                            ((*re).start_bitmap
                                [(0xe1 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize]
                                as ::core::ffi::c_uint
                                | (1 as ::core::ffi::c_uint)
                                    << (0xe1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                as uint8_t;
                        (*re).start_bitmap
                            [(0xe2 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize] =
                            ((*re).start_bitmap
                                [(0xe2 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize]
                                as ::core::ffi::c_uint
                                | (1 as ::core::ffi::c_uint)
                                    << (0xe2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                as uint8_t;
                        (*re).start_bitmap
                            [(0xe3 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize] =
                            ((*re).start_bitmap
                                [(0xe3 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize]
                                as ::core::ffi::c_uint
                                | (1 as ::core::ffi::c_uint)
                                    << (0xe3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                as uint8_t;
                    } else {
                        (*re).start_bitmap[(-96i32 as ::core::ffi::c_uchar as ::core::ffi::c_int
                            / 8 as ::core::ffi::c_int)
                            as usize] = ((*re).start_bitmap[(-96i32 as ::core::ffi::c_uchar
                            as ::core::ffi::c_int
                            / 8 as ::core::ffi::c_int)
                            as usize]
                            as ::core::ffi::c_uint
                            | (1 as ::core::ffi::c_uint)
                                << (-96i32 as ::core::ffi::c_uchar as ::core::ffi::c_int
                                    & 7 as ::core::ffi::c_int))
                            as uint8_t;
                    }
                    try_next = FALSE as BOOL;
                    continue;
                }
                17 | 21 => {
                    (*re).start_bitmap[('\n' as i32 / 8 as ::core::ffi::c_int) as usize] = ((*re)
                        .start_bitmap[('\n' as i32 / 8 as ::core::ffi::c_int) as usize]
                        as ::core::ffi::c_uint
                        | (1 as ::core::ffi::c_uint) << ('\n' as i32 & 7 as ::core::ffi::c_int))
                        as uint8_t;
                    (*re).start_bitmap[('\u{b}' as i32 / 8 as ::core::ffi::c_int) as usize] = ((*re)
                        .start_bitmap[('\u{b}' as i32 / 8 as ::core::ffi::c_int) as usize]
                        as ::core::ffi::c_uint
                        | (1 as ::core::ffi::c_uint) << ('\u{b}' as i32 & 7 as ::core::ffi::c_int))
                        as uint8_t;
                    (*re).start_bitmap[('\u{c}' as i32 / 8 as ::core::ffi::c_int) as usize] = ((*re)
                        .start_bitmap[('\u{c}' as i32 / 8 as ::core::ffi::c_int) as usize]
                        as ::core::ffi::c_uint
                        | (1 as ::core::ffi::c_uint) << ('\u{c}' as i32 & 7 as ::core::ffi::c_int))
                        as uint8_t;
                    (*re).start_bitmap[('\r' as i32 / 8 as ::core::ffi::c_int) as usize] = ((*re)
                        .start_bitmap[('\r' as i32 / 8 as ::core::ffi::c_int) as usize]
                        as ::core::ffi::c_uint
                        | (1 as ::core::ffi::c_uint) << ('\r' as i32 & 7 as ::core::ffi::c_int))
                        as uint8_t;
                    if utf != 0 {
                        (*re).start_bitmap
                            [(0xc2 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize] =
                            ((*re).start_bitmap
                                [(0xc2 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize]
                                as ::core::ffi::c_uint
                                | (1 as ::core::ffi::c_uint)
                                    << (0xc2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                as uint8_t;
                        (*re).start_bitmap
                            [(0xe2 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize] =
                            ((*re).start_bitmap
                                [(0xe2 as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as usize]
                                as ::core::ffi::c_uint
                                | (1 as ::core::ffi::c_uint)
                                    << (0xe2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                as uint8_t;
                    } else {
                        (*re).start_bitmap[(-123i32 as ::core::ffi::c_uchar as ::core::ffi::c_int
                            / 8 as ::core::ffi::c_int)
                            as usize] = ((*re).start_bitmap[(-123i32 as ::core::ffi::c_uchar
                            as ::core::ffi::c_int
                            / 8 as ::core::ffi::c_int)
                            as usize]
                            as ::core::ffi::c_uint
                            | (1 as ::core::ffi::c_uint)
                                << (-123i32 as ::core::ffi::c_uchar as ::core::ffi::c_int
                                    & 7 as ::core::ffi::c_int))
                            as uint8_t;
                    }
                    try_next = FALSE as BOOL;
                    continue;
                }
                6 => {
                    set_nottype_bits(re, cbit_digit, table_limit as ::core::ffi::c_uint);
                    try_next = FALSE as BOOL;
                    continue;
                }
                7 => {
                    set_type_bits(re, cbit_digit, table_limit as ::core::ffi::c_uint);
                    try_next = FALSE as BOOL;
                    continue;
                }
                8 => {
                    set_nottype_bits(re, cbit_space, table_limit as ::core::ffi::c_uint);
                    try_next = FALSE as BOOL;
                    continue;
                }
                9 => {
                    set_type_bits(re, cbit_space, table_limit as ::core::ffi::c_uint);
                    try_next = FALSE as BOOL;
                    continue;
                }
                10 => {
                    set_nottype_bits(re, cbit_word, table_limit as ::core::ffi::c_uint);
                    try_next = FALSE as BOOL;
                    continue;
                }
                11 => {
                    set_type_bits(re, cbit_word, table_limit as ::core::ffi::c_uint);
                    try_next = FALSE as BOOL;
                    continue;
                }
                87 | 88 | 95 => {
                    tcode = tcode.offset(1);
                    continue;
                }
                93 => {
                    tcode = tcode.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    continue;
                }
                91 | 92 | 97 => {
                    tcode = tcode.offset(IMM2_SIZE as isize);
                    current_block = 11998637525973841654;
                }
                85 | 86 | 94 | 89 | 90 | 96 => {
                    current_block = 11998637525973841654;
                }
                113 => return SSB_FAIL as ::core::ffi::c_int,
                112 => {
                    xclassflags = *tcode.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                    if xclassflags as ::core::ffi::c_int & XCL_HASPROP != 0 as ::core::ffi::c_int
                        || xclassflags as ::core::ffi::c_int & (XCL_MAP | XCL_NOT) == XCL_NOT
                    {
                        return SSB_FAIL as ::core::ffi::c_int;
                    }
                    classmap =
                        if xclassflags as ::core::ffi::c_int & XCL_MAP == 0 as ::core::ffi::c_int {
                            ::core::ptr::null::<uint8_t>()
                        } else {
                            tcode
                                .offset(1 as ::core::ffi::c_int as isize)
                                .offset(LINK_SIZE as isize)
                                .offset(1 as ::core::ffi::c_int as isize)
                                as *const uint8_t
                        };
                    if utf != 0
                        && xclassflags as ::core::ffi::c_int & XCL_NOT == 0 as ::core::ffi::c_int
                    {
                        let mut b: PCRE2_UCHAR8 = 0;
                        let mut e: PCRE2_UCHAR8 = 0;
                        let mut p_0: PCRE2_SPTR8 = tcode
                            .offset(1 as ::core::ffi::c_int as isize)
                            .offset(LINK_SIZE as isize)
                            .offset(1 as ::core::ffi::c_int as isize)
                            .offset(
                                (if classmap.is_null() {
                                    0 as ::core::ffi::c_int
                                } else {
                                    32 as ::core::ffi::c_int
                                }) as isize,
                            );
                        tcode = tcode.offset(
                            ((*tcode.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int)
                                << 8 as ::core::ffi::c_int
                                | *tcode.offset(
                                    (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                                ) as ::core::ffi::c_int)
                                as ::core::ffi::c_uint as isize,
                        );
                        if *p_0 as ::core::ffi::c_int
                            >= (if ::core::mem::size_of::<PCRE2_UCHAR8>() as usize == 1 as usize {
                                0x10 as ::core::ffi::c_int
                            } else {
                                0x1000 as ::core::ffi::c_int
                            })
                        {
                            study_char_list(
                                p_0,
                                &raw mut (*re).start_bitmap as *mut uint8_t,
                                (re as *const uint8_t).offset((*re).code_start as isize),
                            );
                        } else {
                            loop {
                                let fresh3 = p_0;
                                p_0 = p_0.offset(1);
                                match *fresh3 as ::core::ffi::c_int {
                                    XCL_SINGLE => {
                                        let fresh4 = p_0;
                                        p_0 = p_0.offset(1);
                                        b = *fresh4;
                                        while *p_0 as ::core::ffi::c_int
                                            & 0xc0 as ::core::ffi::c_int
                                            == 0x80 as ::core::ffi::c_int
                                        {
                                            p_0 = p_0.offset(1);
                                        }
                                        (*re).start_bitmap[(b as ::core::ffi::c_int
                                            / 8 as ::core::ffi::c_int)
                                            as usize] = ((*re).start_bitmap[(b
                                            as ::core::ffi::c_int
                                            / 8 as ::core::ffi::c_int)
                                            as usize]
                                            as ::core::ffi::c_uint
                                            | (1 as ::core::ffi::c_uint)
                                                << (b as ::core::ffi::c_int
                                                    & 7 as ::core::ffi::c_int))
                                            as uint8_t;
                                    }
                                    XCL_RANGE => {
                                        let fresh5 = p_0;
                                        p_0 = p_0.offset(1);
                                        b = *fresh5;
                                        while *p_0 as ::core::ffi::c_int
                                            & 0xc0 as ::core::ffi::c_int
                                            == 0x80 as ::core::ffi::c_int
                                        {
                                            p_0 = p_0.offset(1);
                                        }
                                        let fresh6 = p_0;
                                        p_0 = p_0.offset(1);
                                        e = *fresh6;
                                        while *p_0 as ::core::ffi::c_int
                                            & 0xc0 as ::core::ffi::c_int
                                            == 0x80 as ::core::ffi::c_int
                                        {
                                            p_0 = p_0.offset(1);
                                        }
                                        while b as ::core::ffi::c_int <= e as ::core::ffi::c_int {
                                            (*re).start_bitmap[(b as ::core::ffi::c_int
                                                / 8 as ::core::ffi::c_int)
                                                as usize] = ((*re).start_bitmap[(b
                                                as ::core::ffi::c_int
                                                / 8 as ::core::ffi::c_int)
                                                as usize]
                                                as ::core::ffi::c_uint
                                                | (1 as ::core::ffi::c_uint)
                                                    << (b as ::core::ffi::c_int
                                                        & 7 as ::core::ffi::c_int))
                                                as uint8_t;
                                            b = b.wrapping_add(1);
                                        }
                                    }
                                    XCL_END => {
                                        break;
                                    }
                                    _ => return SSB_UNKNOWN as ::core::ffi::c_int,
                                }
                            }
                        }
                        current_block = 5787245504982604029;
                    } else {
                        current_block = 5252886418925945957;
                    }
                }
                111 => {
                    current_block = 5252886418925945957;
                }
                110 => {
                    current_block = 10614398376620164578;
                }
                _ => return SSB_UNKNOWN as ::core::ffi::c_int,
            }
            match current_block {
                11998637525973841654 => {
                    match *tcode.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int {
                        19 => {
                            (*re).start_bitmap[('\t' as i32 / 8 as ::core::ffi::c_int) as usize] =
                                ((*re).start_bitmap
                                    [('\t' as i32 / 8 as ::core::ffi::c_int) as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << ('\t' as i32 & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                            (*re).start_bitmap[(' ' as i32 / 8 as ::core::ffi::c_int) as usize] =
                                ((*re).start_bitmap[(' ' as i32 / 8 as ::core::ffi::c_int) as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << (' ' as i32 & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                            if utf != 0 {
                                (*re).start_bitmap[(0xc2 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize] = ((*re).start_bitmap[(0xc2 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << (0xc2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                                (*re).start_bitmap[(0xe1 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize] = ((*re).start_bitmap[(0xe1 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << (0xe1 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                                (*re).start_bitmap[(0xe2 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize] = ((*re).start_bitmap[(0xe2 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << (0xe2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                                (*re).start_bitmap[(0xe3 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize] = ((*re).start_bitmap[(0xe3 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << (0xe3 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                            } else {
                                (*re).start_bitmap[(-96i32 as ::core::ffi::c_uchar
                                    as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize] = ((*re).start_bitmap[(-96i32 as ::core::ffi::c_uchar
                                    as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << (-96i32 as ::core::ffi::c_uchar as ::core::ffi::c_int
                                            & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                            }
                        }
                        17 | 21 => {
                            (*re).start_bitmap[('\n' as i32 / 8 as ::core::ffi::c_int) as usize] =
                                ((*re).start_bitmap
                                    [('\n' as i32 / 8 as ::core::ffi::c_int) as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << ('\n' as i32 & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                            (*re).start_bitmap
                                [('\u{b}' as i32 / 8 as ::core::ffi::c_int) as usize] = ((*re)
                                .start_bitmap
                                [('\u{b}' as i32 / 8 as ::core::ffi::c_int) as usize]
                                as ::core::ffi::c_uint
                                | (1 as ::core::ffi::c_uint)
                                    << ('\u{b}' as i32 & 7 as ::core::ffi::c_int))
                                as uint8_t;
                            (*re).start_bitmap
                                [('\u{c}' as i32 / 8 as ::core::ffi::c_int) as usize] = ((*re)
                                .start_bitmap
                                [('\u{c}' as i32 / 8 as ::core::ffi::c_int) as usize]
                                as ::core::ffi::c_uint
                                | (1 as ::core::ffi::c_uint)
                                    << ('\u{c}' as i32 & 7 as ::core::ffi::c_int))
                                as uint8_t;
                            (*re).start_bitmap[('\r' as i32 / 8 as ::core::ffi::c_int) as usize] =
                                ((*re).start_bitmap
                                    [('\r' as i32 / 8 as ::core::ffi::c_int) as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << ('\r' as i32 & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                            if utf != 0 {
                                (*re).start_bitmap[(0xc2 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize] = ((*re).start_bitmap[(0xc2 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << (0xc2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                                (*re).start_bitmap[(0xe2 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize] = ((*re).start_bitmap[(0xe2 as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << (0xe2 as ::core::ffi::c_int & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                            } else {
                                (*re).start_bitmap[(-123i32 as ::core::ffi::c_uchar
                                    as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize] = ((*re).start_bitmap[(-123i32 as ::core::ffi::c_uchar
                                    as ::core::ffi::c_int
                                    / 8 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint)
                                        << (-123i32 as ::core::ffi::c_uchar as ::core::ffi::c_int
                                            & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                            }
                        }
                        6 => {
                            set_nottype_bits(re, cbit_digit, table_limit as ::core::ffi::c_uint);
                        }
                        7 => {
                            set_type_bits(re, cbit_digit, table_limit as ::core::ffi::c_uint);
                        }
                        8 => {
                            set_nottype_bits(re, cbit_space, table_limit as ::core::ffi::c_uint);
                        }
                        9 => {
                            set_type_bits(re, cbit_space, table_limit as ::core::ffi::c_uint);
                        }
                        10 => {
                            set_nottype_bits(re, cbit_word, table_limit as ::core::ffi::c_uint);
                        }
                        11 => {
                            set_type_bits(re, cbit_word, table_limit as ::core::ffi::c_uint);
                        }
                        12 | 13 | _ => return SSB_FAIL as ::core::ffi::c_int,
                    }
                    tcode = tcode.offset(2 as ::core::ffi::c_int as isize);
                    continue;
                }
                18023239131261601843 => {
                    set_table_bit(
                        re,
                        tcode.offset(1 as ::core::ffi::c_int as isize),
                        TRUE,
                        utf,
                        ucp,
                    );
                    try_next = FALSE as BOOL;
                    continue;
                }
                4613046691852425349 => {
                    set_table_bit(
                        re,
                        tcode.offset(1 as ::core::ffi::c_int as isize),
                        FALSE,
                        utf,
                        ucp,
                    );
                    try_next = FALSE as BOOL;
                    continue;
                }
                15858484809446604407 => {
                    rc = set_start_bits(re, tcode, utf, ucp, depthptr);
                    if rc == SSB_DONE as ::core::ffi::c_int {
                        try_next = FALSE as BOOL;
                    } else if rc == SSB_CONTINUE as ::core::ffi::c_int {
                        loop {
                            tcode = tcode.offset(
                                ((*tcode.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int)
                                    << 8 as ::core::ffi::c_int
                                    | *tcode.offset(
                                        (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                            as isize,
                                    ) as ::core::ffi::c_int)
                                    as ::core::ffi::c_uint as isize,
                            );
                            if !(*tcode as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                                break;
                            }
                        }
                        tcode = tcode.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                    } else {
                        return rc;
                    }
                    continue;
                }
                5252886418925945957 => {
                    if utf != 0 {
                        (*re).start_bitmap[24 as ::core::ffi::c_int as usize] = ((*re).start_bitmap
                            [24 as ::core::ffi::c_int as usize]
                            as ::core::ffi::c_int
                            | 0xf0 as ::core::ffi::c_int)
                            as uint8_t;
                        memset(
                            (&raw mut (*re).start_bitmap as *mut uint8_t)
                                .offset(25 as ::core::ffi::c_int as isize)
                                as *mut ::core::ffi::c_void,
                            0xff as ::core::ffi::c_int,
                            7 as size_t,
                        );
                    }
                    current_block = 10614398376620164578;
                }
                _ => {}
            }
            match current_block {
                10614398376620164578 => {
                    if *tcode as ::core::ffi::c_int == OP_XCLASS as ::core::ffi::c_int {
                        tcode = tcode.offset(
                            ((*tcode.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int)
                                << 8 as ::core::ffi::c_int
                                | *tcode.offset(
                                    (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                                ) as ::core::ffi::c_int)
                                as ::core::ffi::c_uint as isize,
                        );
                    } else {
                        tcode = tcode.offset(1);
                        classmap = tcode as *const uint8_t;
                        tcode = tcode.offset(
                            (32 as usize)
                                .wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize)
                                as isize,
                        );
                    }
                }
                _ => {}
            }
            if !classmap.is_null() {
                if utf != 0 {
                    c = 0 as uint32_t;
                    while c < 16 as uint32_t {
                        (*re).start_bitmap[c as usize] = ((*re).start_bitmap[c as usize]
                            as ::core::ffi::c_int
                            | *classmap.offset(c as isize) as ::core::ffi::c_int)
                            as uint8_t;
                        c = c.wrapping_add(1);
                    }
                    c = 128 as uint32_t;
                    while c < 256 as uint32_t {
                        if *classmap.offset(c.wrapping_div(8 as uint32_t) as isize)
                            as ::core::ffi::c_uint
                            & (1 as ::core::ffi::c_uint) << (c & 7 as uint32_t)
                            != 0 as ::core::ffi::c_uint
                        {
                            let mut d: ::core::ffi::c_int = (c >> 6 as ::core::ffi::c_int
                                | 0xc0 as uint32_t)
                                as ::core::ffi::c_int;
                            (*re).start_bitmap[(d / 8 as ::core::ffi::c_int) as usize] =
                                ((*re).start_bitmap[(d / 8 as ::core::ffi::c_int) as usize]
                                    as ::core::ffi::c_uint
                                    | (1 as ::core::ffi::c_uint) << (d & 7 as ::core::ffi::c_int))
                                    as uint8_t;
                            c = (c & 0xc0 as uint32_t)
                                .wrapping_add(0x40 as uint32_t)
                                .wrapping_sub(1 as uint32_t);
                        }
                        c = c.wrapping_add(1);
                    }
                } else {
                    c = 0 as uint32_t;
                    while c < 32 as uint32_t {
                        (*re).start_bitmap[c as usize] = ((*re).start_bitmap[c as usize]
                            as ::core::ffi::c_int
                            | *classmap.offset(c as isize) as ::core::ffi::c_int)
                            as uint8_t;
                        c = c.wrapping_add(1);
                    }
                }
            }
            match *tcode as ::core::ffi::c_int {
                98 | 99 | 102 | 103 | 106 | 108 => {
                    tcode = tcode.offset(1);
                }
                104 | 105 | 109 => {
                    if ((*tcode.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *tcode
                            .offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint
                        == 0 as ::core::ffi::c_uint
                    {
                        tcode = tcode.offset(
                            (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                as isize,
                        );
                    } else {
                        try_next = FALSE as BOOL;
                    }
                }
                _ => {
                    try_next = FALSE as BOOL;
                }
            }
        }
        code = code.offset(
            ((*code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                << 8 as ::core::ffi::c_int
                | *code.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                    as ::core::ffi::c_int) as ::core::ffi::c_uint as isize,
        );
        if !(*code as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
            break;
        }
    }
    return yield_0;
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_study_8(mut re: *mut pcre2_real_code_8) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut code: *mut PCRE2_UCHAR8 = ::core::ptr::null_mut::<PCRE2_UCHAR8>();
    let mut utf: BOOL =
        ((*re).overall_options & PCRE2_UTF as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    let mut ucp: BOOL =
        ((*re).overall_options & PCRE2_UCP as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    code = (re as *mut uint8_t).offset((*re).code_start as isize) as *mut PCRE2_UCHAR8;
    if (*re).flags & (PCRE2_FIRSTSET as uint32_t | PCRE2_STARTLINE as uint32_t) == 0 as uint32_t {
        let mut depth: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        let mut rc: ::core::ffi::c_int =
            set_start_bits(re, code as PCRE2_SPTR8, utf, ucp, &raw mut depth);
        if rc == SSB_UNKNOWN as ::core::ffi::c_int {
            return 1 as ::core::ffi::c_int;
        }
        if rc == SSB_DONE as ::core::ffi::c_int {
            let mut i: ::core::ffi::c_int = 0;
            let mut a: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
            let mut b: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
            let mut p: *mut uint8_t = &raw mut (*re).start_bitmap as *mut uint8_t;
            let mut flags: uint32_t = PCRE2_FIRSTMAPSET as uint32_t;
            i = 0 as ::core::ffi::c_int;
            loop {
                if !(i < 256 as ::core::ffi::c_int) {
                    current_block = 7746103178988627676;
                    break;
                }
                let mut x: uint8_t = *p;
                if x as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                    let mut c: ::core::ffi::c_int = 0;
                    let mut y: uint8_t = (x as ::core::ffi::c_int
                        & !(x as ::core::ffi::c_int) + 1 as ::core::ffi::c_int)
                        as uint8_t;
                    if y as ::core::ffi::c_int != x as ::core::ffi::c_int {
                        current_block = 8843691923059854181;
                        break;
                    }
                    c = i;
                    match x as ::core::ffi::c_int {
                        2 => {
                            c += 1 as ::core::ffi::c_int;
                        }
                        4 => {
                            c += 2 as ::core::ffi::c_int;
                        }
                        8 => {
                            c += 3 as ::core::ffi::c_int;
                        }
                        16 => {
                            c += 4 as ::core::ffi::c_int;
                        }
                        32 => {
                            c += 5 as ::core::ffi::c_int;
                        }
                        64 => {
                            c += 6 as ::core::ffi::c_int;
                        }
                        128 => {
                            c += 7 as ::core::ffi::c_int;
                        }
                        1 | _ => {}
                    }
                    if utf != 0 && c > 127 as ::core::ffi::c_int {
                        current_block = 8843691923059854181;
                        break;
                    }
                    if a < 0 as ::core::ffi::c_int {
                        a = c;
                    } else {
                        if !(b < 0 as ::core::ffi::c_int) {
                            current_block = 8843691923059854181;
                            break;
                        }
                        let mut d: ::core::ffi::c_int = *(*re)
                            .tables
                            .offset(256 as ::core::ffi::c_int as isize)
                            .offset(c as ::core::ffi::c_uint as isize)
                            as ::core::ffi::c_int;
                        if utf != 0 || ucp != 0 {
                            if (*(&raw const _pcre2_ucd_records_8 as *const ucd_record).offset(
                                *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t).offset(
                                    (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                        .offset((c / UCD_BLOCK_SIZE) as isize)
                                        as ::core::ffi::c_int
                                        * UCD_BLOCK_SIZE
                                        + c % UCD_BLOCK_SIZE)
                                        as isize,
                                ) as ::core::ffi::c_int as isize,
                            ))
                            .caseset as ::core::ffi::c_int
                                != 0 as ::core::ffi::c_int
                            {
                                current_block = 8843691923059854181;
                                break;
                            }
                            if c > 127 as ::core::ffi::c_int {
                                d = (c
                                    + (*(&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                        .offset(
                                            *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                .offset(
                                                    (*(&raw const _pcre2_ucd_stage1_8
                                                        as *const uint16_t)
                                                        .offset((c / UCD_BLOCK_SIZE) as isize)
                                                        as ::core::ffi::c_int
                                                        * UCD_BLOCK_SIZE
                                                        + c % UCD_BLOCK_SIZE)
                                                        as isize,
                                                )
                                                as ::core::ffi::c_int
                                                as isize,
                                        ))
                                    .other_case
                                        as ::core::ffi::c_int)
                                    as uint32_t
                                    as ::core::ffi::c_int;
                            }
                        }
                        if d != a {
                            current_block = 8843691923059854181;
                            break;
                        }
                        b = c;
                    }
                }
                p = p.offset(1);
                i += 8 as ::core::ffi::c_int;
            }
            match current_block {
                7746103178988627676 => {
                    if a >= 0 as ::core::ffi::c_int {
                        if (*re).flags & PCRE2_LASTSET as uint32_t != 0
                            && ((*re).last_codeunit == a as uint32_t
                                || b >= 0 as ::core::ffi::c_int
                                    && (*re).last_codeunit == b as uint32_t)
                        {
                            (*re).flags = ((*re).flags as ::core::ffi::c_uint
                                & !(PCRE2_LASTSET | PCRE2_LASTCASELESS))
                                as uint32_t;
                            (*re).last_codeunit = 0 as uint32_t;
                        }
                        (*re).first_codeunit = a as uint32_t;
                        flags = PCRE2_FIRSTSET as uint32_t;
                        if b >= 0 as ::core::ffi::c_int {
                            flags =
                                (flags as ::core::ffi::c_uint | PCRE2_FIRSTCASELESS) as uint32_t;
                        }
                    }
                }
                _ => {}
            }
            (*re).flags |= flags;
        }
    }
    if (*re).flags & (PCRE2_MATCH_EMPTY as uint32_t | PCRE2_HASACCEPT as uint32_t) == 0 as uint32_t
        && (*re).top_backref as ::core::ffi::c_int <= MAX_CACHE_BACKREF
    {
        let mut min: ::core::ffi::c_int = 0;
        let mut backref_cache: [::core::ffi::c_int; 129] = [0; 129];
        backref_cache[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_int;
        min = find_minlength(
            re,
            code as PCRE2_SPTR8,
            code as PCRE2_SPTR8,
            utf,
            ::core::ptr::null_mut::<recurse_check>(),
            &raw mut count,
            &raw mut backref_cache as *mut ::core::ffi::c_int,
        );
        match min {
            -1 => {}
            -2 => return 2 as ::core::ffi::c_int,
            -3 => return 3 as ::core::ffi::c_int,
            _ => {
                (*re).minlength = (if min > UINT16_MAX { UINT16_MAX } else { min }) as uint16_t;
            }
        }
    }
    return 0 as ::core::ffi::c_int;
}
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const LINK_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
