// Translated from PCRE2 10.49 pcre2_auto_possess.c; code unit width 16.
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
// /* This module contains functions that scan a compiled pattern and change
// repeats into possessive repeats where possible. */
extern "C" {
    static _pcre2_OP_lengths_16: [uint8_t; 0];
    static _pcre2_ucd_caseless_sets_16: [uint32_t; 0];
    static _pcre2_ucd_script_sets_16: [uint32_t; 0];
    static _pcre2_ucd_records_16: [ucd_record; 0];
    static _pcre2_ucd_stage1_16: [uint16_t; 0];
    static _pcre2_ucd_stage2_16: [uint16_t; 0];
    static _pcre2_ucp_gentype_16: [uint32_t; 0];
    fn _pcre2_xclass_16(_: uint32_t, _: PCRE2_SPTR16, _: *const uint8_t, _: BOOL) -> BOOL;
    fn _pcre2_eclass_16(
        _: uint32_t,
        _: PCRE2_SPTR16,
        _: PCRE2_SPTR16,
        _: *const uint8_t,
        _: BOOL,
    ) -> BOOL;
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
pub type PCRE2_UCHAR16 = uint16_t;
pub type PCRE2_SPTR16 = *const PCRE2_UCHAR16;
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
pub struct pcre2_real_compile_context_16 {
    pub memctl: pcre2_memctl,
    pub stack_guard:
        Option<unsafe extern "C" fn(uint32_t, *mut ::core::ffi::c_void) -> ::core::ffi::c_int>,
    pub stack_guard_data: *mut ::core::ffi::c_void,
    pub tables: *const uint8_t,
    pub max_pattern_length: size_t,
    pub max_pattern_compiled_length: size_t,
    pub bsr_convention: uint16_t,
    pub newline_convention: uint16_t,
    pub parens_nest_limit: uint32_t,
    pub extra_options: uint32_t,
    pub max_varlookbehind: uint32_t,
    pub optimization_flags: uint32_t,
}
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
pub type C2RustUnnamed_1 = ::core::ffi::c_uint;
pub const OP_TABLE_LENGTH: C2RustUnnamed_1 = 173;
pub const OP_UCP_WORD_BOUNDARY: C2RustUnnamed_1 = 172;
pub const OP_NOT_UCP_WORD_BOUNDARY: C2RustUnnamed_1 = 171;
pub const OP_DEFINE: C2RustUnnamed_1 = 170;
pub const OP_SKIPZERO: C2RustUnnamed_1 = 169;
pub const OP_CLOSE: C2RustUnnamed_1 = 168;
pub const OP_ASSERT_ACCEPT: C2RustUnnamed_1 = 167;
pub const OP_ACCEPT: C2RustUnnamed_1 = 166;
pub const OP_FAIL: C2RustUnnamed_1 = 165;
pub const OP_COMMIT_ARG: C2RustUnnamed_1 = 164;
pub const OP_COMMIT: C2RustUnnamed_1 = 163;
pub const OP_THEN_ARG: C2RustUnnamed_1 = 162;
pub const OP_THEN: C2RustUnnamed_1 = 161;
pub const OP_SKIP_ARG: C2RustUnnamed_1 = 160;
pub const OP_SKIP: C2RustUnnamed_1 = 159;
pub const OP_PRUNE_ARG: C2RustUnnamed_1 = 158;
pub const OP_PRUNE: C2RustUnnamed_1 = 157;
pub const OP_MARK: C2RustUnnamed_1 = 156;
pub const OP_BRAPOSZERO: C2RustUnnamed_1 = 155;
pub const OP_BRAMINZERO: C2RustUnnamed_1 = 154;
pub const OP_BRAZERO: C2RustUnnamed_1 = 153;
pub const OP_TRUE: C2RustUnnamed_1 = 152;
pub const OP_FALSE: C2RustUnnamed_1 = 151;
pub const OP_DNRREF: C2RustUnnamed_1 = 150;
pub const OP_RREF: C2RustUnnamed_1 = 149;
pub const OP_DNCREF: C2RustUnnamed_1 = 148;
pub const OP_CREF: C2RustUnnamed_1 = 147;
pub const OP_SCOND: C2RustUnnamed_1 = 146;
pub const OP_SCBRAPOS: C2RustUnnamed_1 = 145;
pub const OP_SCBRA: C2RustUnnamed_1 = 144;
pub const OP_SBRAPOS: C2RustUnnamed_1 = 143;
pub const OP_SBRA: C2RustUnnamed_1 = 142;
pub const OP_COND: C2RustUnnamed_1 = 141;
pub const OP_CBRAPOS: C2RustUnnamed_1 = 140;
pub const OP_CBRA: C2RustUnnamed_1 = 139;
pub const OP_BRAPOS: C2RustUnnamed_1 = 138;
pub const OP_BRA: C2RustUnnamed_1 = 137;
pub const OP_SCRIPT_RUN: C2RustUnnamed_1 = 136;
pub const OP_ONCE: C2RustUnnamed_1 = 135;
pub const OP_ASSERT_SCS: C2RustUnnamed_1 = 134;
pub const OP_ASSERTBACK_NA: C2RustUnnamed_1 = 133;
pub const OP_ASSERT_NA: C2RustUnnamed_1 = 132;
pub const OP_ASSERTBACK_NOT: C2RustUnnamed_1 = 131;
pub const OP_ASSERTBACK: C2RustUnnamed_1 = 130;
pub const OP_ASSERT_NOT: C2RustUnnamed_1 = 129;
pub const OP_ASSERT: C2RustUnnamed_1 = 128;
pub const OP_VREVERSE: C2RustUnnamed_1 = 127;
pub const OP_REVERSE: C2RustUnnamed_1 = 126;
pub const OP_KETRPOS: C2RustUnnamed_1 = 125;
pub const OP_KETRMIN: C2RustUnnamed_1 = 124;
pub const OP_KETRMAX: C2RustUnnamed_1 = 123;
pub const OP_KET: C2RustUnnamed_1 = 122;
pub const OP_ALT: C2RustUnnamed_1 = 121;
pub const OP_CALLOUT_STR: C2RustUnnamed_1 = 120;
pub const OP_CALLOUT: C2RustUnnamed_1 = 119;
pub const OP_RECURSE: C2RustUnnamed_1 = 118;
pub const OP_DNREFI: C2RustUnnamed_1 = 117;
pub const OP_DNREF: C2RustUnnamed_1 = 116;
pub const OP_REFI: C2RustUnnamed_1 = 115;
pub const OP_REF: C2RustUnnamed_1 = 114;
pub const OP_ECLASS: C2RustUnnamed_1 = 113;
pub const OP_XCLASS: C2RustUnnamed_1 = 112;
pub const OP_NCLASS: C2RustUnnamed_1 = 111;
pub const OP_CLASS: C2RustUnnamed_1 = 110;
pub const OP_CRPOSRANGE: C2RustUnnamed_1 = 109;
pub const OP_CRPOSQUERY: C2RustUnnamed_1 = 108;
pub const OP_CRPOSPLUS: C2RustUnnamed_1 = 107;
pub const OP_CRPOSSTAR: C2RustUnnamed_1 = 106;
pub const OP_CRMINRANGE: C2RustUnnamed_1 = 105;
pub const OP_CRRANGE: C2RustUnnamed_1 = 104;
pub const OP_CRMINQUERY: C2RustUnnamed_1 = 103;
pub const OP_CRQUERY: C2RustUnnamed_1 = 102;
pub const OP_CRMINPLUS: C2RustUnnamed_1 = 101;
pub const OP_CRPLUS: C2RustUnnamed_1 = 100;
pub const OP_CRMINSTAR: C2RustUnnamed_1 = 99;
pub const OP_CRSTAR: C2RustUnnamed_1 = 98;
pub const OP_TYPEPOSUPTO: C2RustUnnamed_1 = 97;
pub const OP_TYPEPOSQUERY: C2RustUnnamed_1 = 96;
pub const OP_TYPEPOSPLUS: C2RustUnnamed_1 = 95;
pub const OP_TYPEPOSSTAR: C2RustUnnamed_1 = 94;
pub const OP_TYPEEXACT: C2RustUnnamed_1 = 93;
pub const OP_TYPEMINUPTO: C2RustUnnamed_1 = 92;
pub const OP_TYPEUPTO: C2RustUnnamed_1 = 91;
pub const OP_TYPEMINQUERY: C2RustUnnamed_1 = 90;
pub const OP_TYPEQUERY: C2RustUnnamed_1 = 89;
pub const OP_TYPEMINPLUS: C2RustUnnamed_1 = 88;
pub const OP_TYPEPLUS: C2RustUnnamed_1 = 87;
pub const OP_TYPEMINSTAR: C2RustUnnamed_1 = 86;
pub const OP_TYPESTAR: C2RustUnnamed_1 = 85;
pub const OP_NOTPOSUPTOI: C2RustUnnamed_1 = 84;
pub const OP_NOTPOSQUERYI: C2RustUnnamed_1 = 83;
pub const OP_NOTPOSPLUSI: C2RustUnnamed_1 = 82;
pub const OP_NOTPOSSTARI: C2RustUnnamed_1 = 81;
pub const OP_NOTEXACTI: C2RustUnnamed_1 = 80;
pub const OP_NOTMINUPTOI: C2RustUnnamed_1 = 79;
pub const OP_NOTUPTOI: C2RustUnnamed_1 = 78;
pub const OP_NOTMINQUERYI: C2RustUnnamed_1 = 77;
pub const OP_NOTQUERYI: C2RustUnnamed_1 = 76;
pub const OP_NOTMINPLUSI: C2RustUnnamed_1 = 75;
pub const OP_NOTPLUSI: C2RustUnnamed_1 = 74;
pub const OP_NOTMINSTARI: C2RustUnnamed_1 = 73;
pub const OP_NOTSTARI: C2RustUnnamed_1 = 72;
pub const OP_NOTPOSUPTO: C2RustUnnamed_1 = 71;
pub const OP_NOTPOSQUERY: C2RustUnnamed_1 = 70;
pub const OP_NOTPOSPLUS: C2RustUnnamed_1 = 69;
pub const OP_NOTPOSSTAR: C2RustUnnamed_1 = 68;
pub const OP_NOTEXACT: C2RustUnnamed_1 = 67;
pub const OP_NOTMINUPTO: C2RustUnnamed_1 = 66;
pub const OP_NOTUPTO: C2RustUnnamed_1 = 65;
pub const OP_NOTMINQUERY: C2RustUnnamed_1 = 64;
pub const OP_NOTQUERY: C2RustUnnamed_1 = 63;
pub const OP_NOTMINPLUS: C2RustUnnamed_1 = 62;
pub const OP_NOTPLUS: C2RustUnnamed_1 = 61;
pub const OP_NOTMINSTAR: C2RustUnnamed_1 = 60;
pub const OP_NOTSTAR: C2RustUnnamed_1 = 59;
pub const OP_POSUPTOI: C2RustUnnamed_1 = 58;
pub const OP_POSQUERYI: C2RustUnnamed_1 = 57;
pub const OP_POSPLUSI: C2RustUnnamed_1 = 56;
pub const OP_POSSTARI: C2RustUnnamed_1 = 55;
pub const OP_EXACTI: C2RustUnnamed_1 = 54;
pub const OP_MINUPTOI: C2RustUnnamed_1 = 53;
pub const OP_UPTOI: C2RustUnnamed_1 = 52;
pub const OP_MINQUERYI: C2RustUnnamed_1 = 51;
pub const OP_QUERYI: C2RustUnnamed_1 = 50;
pub const OP_MINPLUSI: C2RustUnnamed_1 = 49;
pub const OP_PLUSI: C2RustUnnamed_1 = 48;
pub const OP_MINSTARI: C2RustUnnamed_1 = 47;
pub const OP_STARI: C2RustUnnamed_1 = 46;
pub const OP_POSUPTO: C2RustUnnamed_1 = 45;
pub const OP_POSQUERY: C2RustUnnamed_1 = 44;
pub const OP_POSPLUS: C2RustUnnamed_1 = 43;
pub const OP_POSSTAR: C2RustUnnamed_1 = 42;
pub const OP_EXACT: C2RustUnnamed_1 = 41;
pub const OP_MINUPTO: C2RustUnnamed_1 = 40;
pub const OP_UPTO: C2RustUnnamed_1 = 39;
pub const OP_MINQUERY: C2RustUnnamed_1 = 38;
pub const OP_QUERY: C2RustUnnamed_1 = 37;
pub const OP_MINPLUS: C2RustUnnamed_1 = 36;
pub const OP_PLUS: C2RustUnnamed_1 = 35;
pub const OP_MINSTAR: C2RustUnnamed_1 = 34;
pub const OP_STAR: C2RustUnnamed_1 = 33;
pub const OP_NOTI: C2RustUnnamed_1 = 32;
pub const OP_NOT: C2RustUnnamed_1 = 31;
pub const OP_CHARI: C2RustUnnamed_1 = 30;
pub const OP_CHAR: C2RustUnnamed_1 = 29;
pub const OP_CIRCM: C2RustUnnamed_1 = 28;
pub const OP_CIRC: C2RustUnnamed_1 = 27;
pub const OP_DOLLM: C2RustUnnamed_1 = 26;
pub const OP_DOLL: C2RustUnnamed_1 = 25;
pub const OP_EOD: C2RustUnnamed_1 = 24;
pub const OP_EODN: C2RustUnnamed_1 = 23;
pub const OP_EXTUNI: C2RustUnnamed_1 = 22;
pub const OP_VSPACE: C2RustUnnamed_1 = 21;
pub const OP_NOT_VSPACE: C2RustUnnamed_1 = 20;
pub const OP_HSPACE: C2RustUnnamed_1 = 19;
pub const OP_NOT_HSPACE: C2RustUnnamed_1 = 18;
pub const OP_ANYNL: C2RustUnnamed_1 = 17;
pub const OP_PROP: C2RustUnnamed_1 = 16;
pub const OP_NOTPROP: C2RustUnnamed_1 = 15;
pub const OP_ANYBYTE: C2RustUnnamed_1 = 14;
pub const OP_ALLANY: C2RustUnnamed_1 = 13;
pub const OP_ANY: C2RustUnnamed_1 = 12;
pub const OP_WORDCHAR: C2RustUnnamed_1 = 11;
pub const OP_NOT_WORDCHAR: C2RustUnnamed_1 = 10;
pub const OP_WHITESPACE: C2RustUnnamed_1 = 9;
pub const OP_NOT_WHITESPACE: C2RustUnnamed_1 = 8;
pub const OP_DIGIT: C2RustUnnamed_1 = 7;
pub const OP_NOT_DIGIT: C2RustUnnamed_1 = 6;
pub const OP_WORD_BOUNDARY: C2RustUnnamed_1 = 5;
pub const OP_NOT_WORD_BOUNDARY: C2RustUnnamed_1 = 4;
pub const OP_SET_SOM: C2RustUnnamed_1 = 3;
pub const OP_SOM: C2RustUnnamed_1 = 2;
pub const OP_SOD: C2RustUnnamed_1 = 1;
pub const OP_END: C2RustUnnamed_1 = 0;
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
pub struct named_group_16 {
    pub name: PCRE2_SPTR16,
    pub number: uint32_t,
    pub length: uint16_t,
    pub hash_dup: uint16_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct compile_data {
    pub next: *mut compile_data,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union class_bits_storage {
    pub classbits: [uint8_t; 32],
    pub classwords: [uint32_t; 8],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct compile_block_16 {
    pub cx: *mut pcre2_real_compile_context_16,
    pub lcc: *const uint8_t,
    pub fcc: *const uint8_t,
    pub cbits: *const uint8_t,
    pub ctypes: *const uint8_t,
    pub start_workspace: *mut PCRE2_UCHAR16,
    pub start_code: *mut PCRE2_UCHAR16,
    pub start_pattern: PCRE2_SPTR16,
    pub end_pattern: PCRE2_SPTR16,
    pub name_table: *mut PCRE2_UCHAR16,
    pub workspace_size: size_t,
    pub small_ref_offset: [size_t; 10],
    pub erroroffset: size_t,
    pub classbits: class_bits_storage,
    pub names_found: uint16_t,
    pub name_entry_size: uint16_t,
    pub parens_depth: uint16_t,
    pub assert_depth: uint16_t,
    pub named_groups: *mut named_group_16,
    pub named_group_list_size: uint32_t,
    pub external_options: uint32_t,
    pub external_flags: uint32_t,
    pub bracount: uint32_t,
    pub lastcapture: uint32_t,
    pub parsed_pattern: *mut uint32_t,
    pub parsed_pattern_end: *mut uint32_t,
    pub groupinfo: *mut uint32_t,
    pub top_backref: uint32_t,
    pub backref_map: uint32_t,
    pub nltype: uint32_t,
    pub nllen: uint32_t,
    pub nl: [PCRE2_UCHAR16; 4],
    pub class_op_used: [uint8_t; 15],
    pub req_varyopt: uint32_t,
    pub max_varlookbehind: uint32_t,
    pub max_lookbehind: ::core::ffi::c_int,
    pub had_accept: BOOL,
    pub had_pruneorskip: BOOL,
    pub had_recurse: BOOL,
    pub dupnames: BOOL,
    pub first_data: *mut compile_data,
    pub last_data: *mut compile_data,
    pub char_lists_size: size_t,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PCRE2_UCP: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PCRE2_UTF: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const NOTACHAR: ::core::ffi::c_uint = 0xffffffff as ::core::ffi::c_uint;
pub const cbit_space: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const cbit_digit: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const cbit_word: ::core::ffi::c_int = 160 as ::core::ffi::c_int;
pub const ctype_space: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const ctype_digit: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const ctype_word: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const CHAR_HT: uint32_t = 9 as uint32_t;
pub const CHAR_VT: uint32_t = 11 as uint32_t;
pub const CHAR_FF: uint32_t = 12 as uint32_t;
pub const CHAR_CR: uint32_t = 13 as uint32_t;
pub const CHAR_LF: uint32_t = 10 as uint32_t;
pub const CHAR_NEL: uint32_t = 133 as uint32_t;
pub const CHAR_SPACE: uint32_t = 32 as uint32_t;
pub const CHAR_UNDERSCORE: ::core::ffi::c_int = '_' as i32;
pub const CHAR_NBSP: uint32_t = 160 as uint32_t;
pub const PT_LAMP: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
pub const PT_GC: ::core::ffi::c_uint = 1 as ::core::ffi::c_uint;
pub const PT_PC: ::core::ffi::c_uint = 2 as ::core::ffi::c_uint;
pub const PT_SC: ::core::ffi::c_uint = 3 as ::core::ffi::c_uint;
pub const PT_SCX: ::core::ffi::c_uint = 4 as ::core::ffi::c_uint;
pub const PT_ALNUM: ::core::ffi::c_uint = 5 as ::core::ffi::c_uint;
pub const PT_SPACE: ::core::ffi::c_uint = 6 as ::core::ffi::c_uint;
pub const PT_PXSPACE: ::core::ffi::c_uint = 7 as ::core::ffi::c_uint;
pub const PT_WORD: ::core::ffi::c_uint = 8 as ::core::ffi::c_uint;
pub const PT_CLIST: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const PT_BIDICL: ::core::ffi::c_uint = 11 as ::core::ffi::c_uint;
pub const PT_BOOL: ::core::ffi::c_uint = 12 as ::core::ffi::c_uint;
pub const XCL_NOT: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const XCL_MAP: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const XCL_HASPROP: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const LINK_SIZE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const IMM2_SIZE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const MAX_LIST: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
static mut autoposstab: [[uint8_t; 21]; 17] = [
    [
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
];
static mut propposstab: [[uint8_t; 13]; 13] = [
    [
        3 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        4 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        9 as ::core::ffi::c_int as uint8_t,
        10 as ::core::ffi::c_int as uint8_t,
        10 as ::core::ffi::c_int as uint8_t,
        11 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        5 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        15 as ::core::ffi::c_int as uint8_t,
        16 as ::core::ffi::c_int as uint8_t,
        16 as ::core::ffi::c_int as uint8_t,
        17 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        3 as ::core::ffi::c_int as uint8_t,
        6 as ::core::ffi::c_int as uint8_t,
        12 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        1 as ::core::ffi::c_int as uint8_t,
        7 as ::core::ffi::c_int as uint8_t,
        13 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        1 as ::core::ffi::c_int as uint8_t,
        7 as ::core::ffi::c_int as uint8_t,
        13 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        8 as ::core::ffi::c_int as uint8_t,
        14 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
];
static mut catposstab: [[uint8_t; 30]; 7] = [
    [
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
    ],
    [
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
    ],
    [
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
    ],
    [
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
    ],
    [
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
    ],
    [
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
    ],
    [
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
    ],
];
static mut posspropstab: [[uint8_t; 4]; 3] = [
    [
        ucp_L as ::core::ffi::c_int as uint8_t,
        ucp_N as ::core::ffi::c_int as uint8_t,
        ucp_N as ::core::ffi::c_int as uint8_t,
        ucp_Nl as ::core::ffi::c_int as uint8_t,
    ],
    [
        ucp_Z as ::core::ffi::c_int as uint8_t,
        ucp_Z as ::core::ffi::c_int as uint8_t,
        ucp_C as ::core::ffi::c_int as uint8_t,
        ucp_Cc as ::core::ffi::c_int as uint8_t,
    ],
    [
        ucp_L as ::core::ffi::c_int as uint8_t,
        ucp_N as ::core::ffi::c_int as uint8_t,
        ucp_P as ::core::ffi::c_int as uint8_t,
        ucp_Po as ::core::ffi::c_int as uint8_t,
    ],
];
unsafe extern "C" fn check_char_prop(
    mut c: uint32_t,
    mut ptype: ::core::ffi::c_uint,
    mut pdata: ::core::ffi::c_uint,
    mut negated: BOOL,
) -> BOOL {
    let mut ok: BOOL = 0;
    let mut rc: BOOL = 0;
    let mut p: *const uint32_t = ::core::ptr::null::<uint32_t>();
    let mut prop: *const ucd_record = (&raw const _pcre2_ucd_records_16 as *const ucd_record)
        .offset(
            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                    .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                    as ::core::ffi::c_int
                    * UCD_BLOCK_SIZE
                    + c as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
            ) as ::core::ffi::c_int as isize,
        );
    match ptype {
        0 => {
            return (((*prop).chartype as ::core::ffi::c_int == ucp_Lu as ::core::ffi::c_int
                || (*prop).chartype as ::core::ffi::c_int == ucp_Ll as ::core::ffi::c_int
                || (*prop).chartype as ::core::ffi::c_int == ucp_Lt as ::core::ffi::c_int)
                as ::core::ffi::c_int
                == negated) as ::core::ffi::c_int;
        }
        1 => {
            return ((pdata as uint32_t
                == *(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                    .offset((*prop).chartype as isize)) as ::core::ffi::c_int
                == negated) as ::core::ffi::c_int;
        }
        2 => {
            return ((pdata == (*prop).chartype as ::core::ffi::c_uint) as ::core::ffi::c_int
                == negated) as ::core::ffi::c_int;
        }
        3 => {
            return ((pdata == (*prop).script as ::core::ffi::c_uint) as ::core::ffi::c_int
                == negated) as ::core::ffi::c_int;
        }
        4 => {
            ok = (pdata == (*prop).script as ::core::ffi::c_uint
                || *(&raw const _pcre2_ucd_script_sets_16 as *const uint32_t)
                    .offset(
                        ((*prop).scriptx_bidiclass as ::core::ffi::c_int
                            & 0x3ff as ::core::ffi::c_int) as isize,
                    )
                    .offset(pdata.wrapping_div(32 as ::core::ffi::c_uint) as isize)
                    & (1 as uint32_t) << pdata.wrapping_rem(32 as ::core::ffi::c_uint)
                    != 0 as uint32_t) as ::core::ffi::c_int as BOOL;
            return (ok == negated) as ::core::ffi::c_int;
        }
        5 => {
            return ((*(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                .offset((*prop).chartype as isize)
                == ucp_L as ::core::ffi::c_int as uint32_t
                || *(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                    .offset((*prop).chartype as isize)
                    == ucp_N as ::core::ffi::c_int as uint32_t)
                as ::core::ffi::c_int
                == negated) as ::core::ffi::c_int;
        }
        6 | 7 => {
            match c {
                9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194 | 8195 | 8196 | 8197 | 8198
                | 8199 | 8200 | 8201 | 8202 | 8239 | 8287 | 12288 | 10 | 11 | 12 | 13 | 133
                | 8232 | 8233 => {
                    rc = negated;
                }
                _ => {
                    rc = ((*(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                        .offset((*prop).chartype as isize)
                        == ucp_Z as ::core::ffi::c_int as uint32_t)
                        as ::core::ffi::c_int
                        == negated) as ::core::ffi::c_int as BOOL;
                }
            }
            return rc;
        }
        8 => {
            return ((*(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                .offset((*prop).chartype as isize)
                == ucp_L as ::core::ffi::c_int as uint32_t
                || *(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                    .offset((*prop).chartype as isize)
                    == ucp_N as ::core::ffi::c_int as uint32_t
                || c == CHAR_UNDERSCORE as uint32_t) as ::core::ffi::c_int
                == negated) as ::core::ffi::c_int;
        }
        9 => {
            p = (&raw const _pcre2_ucd_caseless_sets_16 as *const uint32_t)
                .offset((*prop).caseset as ::core::ffi::c_int as isize);
            loop {
                if c < *p {
                    return (negated == 0) as ::core::ffi::c_int;
                }
                let fresh4 = p;
                p = p.offset(1);
                if c == *fresh4 {
                    return negated;
                }
            }
        }
        11 => return FALSE,
        12 => return FALSE,
        _ => {}
    }
    return FALSE;
}
unsafe extern "C" fn get_repeat_base(mut c: PCRE2_UCHAR16) -> PCRE2_UCHAR16 {
    return (if c as ::core::ffi::c_int > OP_TYPEPOSUPTO as ::core::ffi::c_int {
        c as ::core::ffi::c_int
    } else if c as ::core::ffi::c_int >= OP_TYPESTAR as ::core::ffi::c_int {
        OP_TYPESTAR as ::core::ffi::c_int
    } else if c as ::core::ffi::c_int >= OP_NOTSTARI as ::core::ffi::c_int {
        OP_NOTSTARI as ::core::ffi::c_int
    } else if c as ::core::ffi::c_int >= OP_NOTSTAR as ::core::ffi::c_int {
        OP_NOTSTAR as ::core::ffi::c_int
    } else if c as ::core::ffi::c_int >= OP_STARI as ::core::ffi::c_int {
        OP_STARI as ::core::ffi::c_int
    } else {
        OP_STAR as ::core::ffi::c_int
    }) as PCRE2_UCHAR16;
}
unsafe extern "C" fn get_chr_property_list(
    mut code: PCRE2_SPTR16,
    mut utf: BOOL,
    mut ucp: BOOL,
    mut fcc: *const uint8_t,
    mut list: *mut uint32_t,
) -> PCRE2_SPTR16 {
    let mut c: PCRE2_UCHAR16 = *code;
    let mut base: PCRE2_UCHAR16 = 0;
    let mut end: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut class_end: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut chr: uint32_t = 0;
    let mut clist_dest: *mut uint32_t = ::core::ptr::null_mut::<uint32_t>();
    let mut clist_src: *const uint32_t = ::core::ptr::null::<uint32_t>();
    *list.offset(0 as ::core::ffi::c_int as isize) = c as uint32_t;
    *list.offset(1 as ::core::ffi::c_int as isize) = FALSE as uint32_t;
    code = code.offset(1);
    if c as ::core::ffi::c_int >= OP_STAR as ::core::ffi::c_int
        && c as ::core::ffi::c_int <= OP_TYPEPOSUPTO as ::core::ffi::c_int
    {
        base = get_repeat_base(c);
        c = (c as ::core::ffi::c_int - (base as ::core::ffi::c_int - OP_STAR as ::core::ffi::c_int))
            as PCRE2_UCHAR16;
        if c as ::core::ffi::c_int == OP_UPTO as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_MINUPTO as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_EXACT as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_POSUPTO as ::core::ffi::c_int
        {
            code = code.offset(IMM2_SIZE as isize);
        }
        *list.offset(1 as ::core::ffi::c_int as isize) =
            (c as ::core::ffi::c_int != OP_PLUS as ::core::ffi::c_int
                && c as ::core::ffi::c_int != OP_MINPLUS as ::core::ffi::c_int
                && c as ::core::ffi::c_int != OP_EXACT as ::core::ffi::c_int
                && c as ::core::ffi::c_int != OP_POSPLUS as ::core::ffi::c_int)
                as ::core::ffi::c_int as uint32_t;
        match base as ::core::ffi::c_int {
            33 => {
                *list.offset(0 as ::core::ffi::c_int as isize) =
                    OP_CHAR as ::core::ffi::c_int as uint32_t;
            }
            46 => {
                *list.offset(0 as ::core::ffi::c_int as isize) =
                    OP_CHARI as ::core::ffi::c_int as uint32_t;
            }
            59 => {
                *list.offset(0 as ::core::ffi::c_int as isize) =
                    OP_NOT as ::core::ffi::c_int as uint32_t;
            }
            72 => {
                *list.offset(0 as ::core::ffi::c_int as isize) =
                    OP_NOTI as ::core::ffi::c_int as uint32_t;
            }
            85 => {
                *list.offset(0 as ::core::ffi::c_int as isize) = *code as uint32_t;
                code = code.offset(1);
            }
            _ => {}
        }
        c = *list.offset(0 as ::core::ffi::c_int as isize) as PCRE2_UCHAR16;
    }
    match c as ::core::ffi::c_int {
        6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 17 | 18 | 19 | 20 | 21 | 22 | 23 | 24 | 25 | 26 => {
            return code
        }
        29 | 31 => {
            let fresh5 = code;
            code = code.offset(1);
            chr = *fresh5 as uint32_t;
            if utf != 0 && chr & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                let fresh6 = code;
                code = code.offset(1);
                chr = ((chr & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                    | *fresh6 as uint32_t & 0x3ff as uint32_t)
                    .wrapping_add(0x10000 as uint32_t);
            }
            *list.offset(2 as ::core::ffi::c_int as isize) = chr;
            *list.offset(3 as ::core::ffi::c_int as isize) = NOTACHAR as uint32_t;
            return code;
        }
        30 | 32 => {
            *list.offset(0 as ::core::ffi::c_int as isize) =
                (if c as ::core::ffi::c_int == OP_CHARI as ::core::ffi::c_int {
                    OP_CHAR as ::core::ffi::c_int
                } else {
                    OP_NOT as ::core::ffi::c_int
                }) as uint32_t;
            let fresh7 = code;
            code = code.offset(1);
            chr = *fresh7 as uint32_t;
            if utf != 0 && chr & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                let fresh8 = code;
                code = code.offset(1);
                chr = ((chr & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                    | *fresh8 as uint32_t & 0x3ff as uint32_t)
                    .wrapping_add(0x10000 as uint32_t);
            }
            *list.offset(2 as ::core::ffi::c_int as isize) = chr;
            if chr < 128 as uint32_t || chr < 256 as uint32_t && utf == 0 && ucp == 0 {
                *list.offset(3 as ::core::ffi::c_int as isize) =
                    *fcc.offset(chr as isize) as uint32_t;
            } else {
                *list.offset(3 as ::core::ffi::c_int as isize) = (chr as ::core::ffi::c_int
                    + (*(&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
                        *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                            (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                .offset((chr as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                                as ::core::ffi::c_int
                                * UCD_BLOCK_SIZE
                                + chr as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                as isize,
                        ) as ::core::ffi::c_int as isize,
                    ))
                    .other_case as ::core::ffi::c_int)
                    as uint32_t;
            }
            if chr == *list.offset(3 as ::core::ffi::c_int as isize) {
                *list.offset(3 as ::core::ffi::c_int as isize) = NOTACHAR as uint32_t;
            } else {
                *list.offset(4 as ::core::ffi::c_int as isize) = NOTACHAR as uint32_t;
            }
            return code;
        }
        16 | 15 => {
            if *code.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int != PT_CLIST {
                *list.offset(2 as ::core::ffi::c_int as isize) =
                    *code.offset(0 as ::core::ffi::c_int as isize) as uint32_t;
                *list.offset(3 as ::core::ffi::c_int as isize) =
                    *code.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                return code.offset(2 as ::core::ffi::c_int as isize);
            }
            clist_src = (&raw const _pcre2_ucd_caseless_sets_16 as *const uint32_t).offset(
                *code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int as isize,
            );
            clist_dest = list.offset(2 as ::core::ffi::c_int as isize);
            code = code.offset(2 as ::core::ffi::c_int as isize);
            loop {
                if clist_dest >= list.offset(MAX_LIST as isize) {
                    *list.offset(2 as ::core::ffi::c_int as isize) =
                        *code.offset(0 as ::core::ffi::c_int as isize) as uint32_t;
                    *list.offset(3 as ::core::ffi::c_int as isize) =
                        *code.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    return code;
                }
                let fresh9 = clist_dest;
                clist_dest = clist_dest.offset(1);
                *fresh9 = *clist_src;
                let fresh10 = clist_src;
                clist_src = clist_src.offset(1);
                if !(*fresh10 != NOTACHAR as uint32_t) {
                    break;
                }
            }
            *list.offset(0 as ::core::ffi::c_int as isize) =
                (if c as ::core::ffi::c_int == OP_PROP as ::core::ffi::c_int {
                    OP_CHAR as ::core::ffi::c_int
                } else {
                    OP_NOT as ::core::ffi::c_int
                }) as uint32_t;
            return code;
        }
        111 | 110 | 112 | 113 => {
            if c as ::core::ffi::c_int == OP_XCLASS as ::core::ffi::c_int
                || c as ::core::ffi::c_int == OP_ECLASS as ::core::ffi::c_int
            {
                end = code
                    .offset(
                        *code.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            as isize,
                    )
                    .offset(-(1 as ::core::ffi::c_int as isize));
            } else {
                end = code.offset(
                    (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR16>() as usize)
                        as isize,
                );
            }
            class_end = end;
            match *end as ::core::ffi::c_int {
                98 | 99 | 102 | 103 | 106 | 108 => {
                    *list.offset(1 as ::core::ffi::c_int as isize) = TRUE as uint32_t;
                    end = end.offset(1);
                }
                100 | 101 | 107 => {
                    end = end.offset(1);
                }
                104 | 105 | 109 => {
                    *list.offset(1 as ::core::ffi::c_int as isize) =
                        (*end.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            == 0 as ::core::ffi::c_int)
                            as ::core::ffi::c_int as uint32_t;
                    end = end.offset(
                        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE) as isize,
                    );
                }
                _ => {}
            }
            *list.offset(2 as ::core::ffi::c_int as isize) =
                end.offset_from(code) as isize as uint32_t;
            *list.offset(3 as ::core::ffi::c_int as isize) =
                end.offset_from(class_end) as isize as uint32_t;
            return end;
        }
        _ => {}
    }
    return ::core::ptr::null::<PCRE2_UCHAR16>();
}
unsafe extern "C" fn compare_opcodes(
    mut code: PCRE2_SPTR16,
    mut utf: BOOL,
    mut ucp: BOOL,
    mut cb: *const compile_block_16,
    mut base_list: *const uint32_t,
    mut base_end: PCRE2_SPTR16,
    mut rec_limit: *mut ::core::ffi::c_int,
) -> BOOL {
    let mut c: PCRE2_UCHAR16 = 0;
    let mut list: [uint32_t; 8] = [0; 8];
    let mut chr_ptr: *const uint32_t = ::core::ptr::null::<uint32_t>();
    let mut ochr_ptr: *const uint32_t = ::core::ptr::null::<uint32_t>();
    let mut list_ptr: *const uint32_t = ::core::ptr::null::<uint32_t>();
    let mut next_code: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut xclass_flags: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut class_bitset: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut set1: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut set2: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut set_end: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut chr: uint32_t = 0;
    let mut accepted: BOOL = 0;
    let mut invert_bits: BOOL = 0;
    let mut entered_a_group: BOOL = FALSE;
    *rec_limit -= 1;
    if *rec_limit <= 0 as ::core::ffi::c_int {
        return FALSE;
    }
    let mut current_block_175: u64;
    loop {
        let mut bracode: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
        c = *code;
        if c as ::core::ffi::c_int == OP_CALLOUT as ::core::ffi::c_int {
            code = code.offset(
                *(&raw const _pcre2_OP_lengths_16 as *const uint8_t).offset(c as isize)
                    as ::core::ffi::c_int as isize,
            );
        } else if c as ::core::ffi::c_int == OP_CALLOUT_STR as ::core::ffi::c_int {
            code = code.offset(*code.offset(
                (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * 1 as ::core::ffi::c_int)
                    as isize,
            ) as ::core::ffi::c_int as isize);
        } else {
            if c as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int {
                loop {
                    code = code.offset(*code.offset(1 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int as isize);
                    if !(*code as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                        break;
                    }
                }
                c = *code;
            }
            match c as ::core::ffi::c_int {
                0 => {
                    return (*base_list.offset(1 as ::core::ffi::c_int as isize) != 0 as uint32_t)
                        as ::core::ffi::c_int;
                }
                122 | 125 => {
                    if *base_list.offset(1 as ::core::ffi::c_int as isize) == 0 as uint32_t {
                        return FALSE;
                    }
                    bracode = code.offset(
                        -(*code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            as isize),
                    );
                    match *bracode as ::core::ffi::c_int {
                        139 | 144 | 140 | 145 => {
                            if (*cb).had_recurse != 0 {
                                return FALSE;
                            }
                        }
                        136 => {
                            if *base_list.offset(0 as ::core::ffi::c_int as isize)
                                != OP_CHAR as ::core::ffi::c_int as uint32_t
                                && *base_list.offset(0 as ::core::ffi::c_int as isize)
                                    != OP_CHARI as ::core::ffi::c_int as uint32_t
                            {
                                return FALSE;
                            }
                        }
                        128 | 129 | 135 => {
                            return (entered_a_group == 0) as ::core::ffi::c_int;
                        }
                        130 | 131 => {
                            loop {
                                if *bracode.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize)
                                    as ::core::ffi::c_int
                                    == OP_VREVERSE as ::core::ffi::c_int
                                {
                                    return FALSE;
                                }
                                bracode = bracode.offset(
                                    *bracode.offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        as isize,
                                );
                                if !(*bracode as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int)
                                {
                                    break;
                                }
                            }
                            return (entered_a_group == 0) as ::core::ffi::c_int;
                        }
                        132 | 133 => return FALSE,
                        _ => {}
                    }
                    code = code.offset(
                        *(&raw const _pcre2_OP_lengths_16 as *const uint8_t).offset(c as isize)
                            as ::core::ffi::c_int as isize,
                    );
                }
                135 | 137 | 139 => {
                    next_code = code.offset(*code.offset(1 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int as isize);
                    code = code.offset(
                        *(&raw const _pcre2_OP_lengths_16 as *const uint8_t).offset(c as isize)
                            as ::core::ffi::c_int as isize,
                    );
                    while *next_code as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int {
                        if compare_opcodes(code, utf, ucp, cb, base_list, base_end, rec_limit) == 0
                        {
                            return FALSE;
                        }
                        code = next_code
                            .offset(1 as ::core::ffi::c_int as isize)
                            .offset(LINK_SIZE as isize);
                        next_code = next_code
                            .offset(*next_code.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int as isize);
                    }
                    entered_a_group = TRUE as BOOL;
                }
                153 | 154 => {
                    next_code = code.offset(1 as ::core::ffi::c_int as isize);
                    if *next_code as ::core::ffi::c_int != OP_BRA as ::core::ffi::c_int
                        && *next_code as ::core::ffi::c_int != OP_CBRA as ::core::ffi::c_int
                        && *next_code as ::core::ffi::c_int != OP_ONCE as ::core::ffi::c_int
                    {
                        return FALSE;
                    }
                    loop {
                        next_code = next_code
                            .offset(*next_code.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int as isize);
                        if !(*next_code as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                            break;
                        }
                    }
                    next_code = next_code.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                    if compare_opcodes(next_code, utf, ucp, cb, base_list, base_end, rec_limit) == 0
                    {
                        return FALSE;
                    }
                    code = code.offset(
                        *(&raw const _pcre2_OP_lengths_16 as *const uint8_t).offset(c as isize)
                            as ::core::ffi::c_int as isize,
                    );
                }
                _ => {
                    code = get_chr_property_list(
                        code,
                        utf,
                        ucp,
                        (*cb).fcc,
                        &raw mut list as *mut uint32_t,
                    );
                    if code.is_null() {
                        return FALSE;
                    }
                    if *base_list.offset(0 as ::core::ffi::c_int as isize)
                        == OP_CHAR as ::core::ffi::c_int as uint32_t
                    {
                        chr_ptr = base_list.offset(2 as ::core::ffi::c_int as isize);
                        list_ptr = &raw mut list as *mut uint32_t;
                    } else if list[0 as ::core::ffi::c_int as usize]
                        == OP_CHAR as ::core::ffi::c_int as uint32_t
                    {
                        chr_ptr = (&raw mut list as *mut uint32_t)
                            .offset(2 as ::core::ffi::c_int as isize);
                        list_ptr = base_list;
                    } else if *base_list.offset(0 as ::core::ffi::c_int as isize)
                        == OP_CLASS as ::core::ffi::c_int as uint32_t
                        || list[0 as ::core::ffi::c_int as usize]
                            == OP_CLASS as ::core::ffi::c_int as uint32_t
                    {
                        if *base_list.offset(0 as ::core::ffi::c_int as isize)
                            == OP_CLASS as ::core::ffi::c_int as uint32_t
                        {
                            set1 = base_end.offset(
                                -(*base_list.offset(2 as ::core::ffi::c_int as isize) as isize),
                            ) as *const uint8_t;
                            list_ptr = &raw mut list as *mut uint32_t;
                        } else {
                            set1 = code.offset(-(list[2 as ::core::ffi::c_int as usize] as isize))
                                as *const uint8_t;
                            list_ptr = base_list;
                        }
                        invert_bits = FALSE as BOOL;
                        match *list_ptr.offset(0 as ::core::ffi::c_int as isize) {
                            110 | 111 => {
                                set2 = (if list_ptr
                                    == &raw mut list as *mut uint32_t as *const uint32_t
                                {
                                    code
                                } else {
                                    base_end
                                })
                                .offset(
                                    -(*list_ptr.offset(2 as ::core::ffi::c_int as isize) as isize),
                                ) as *const uint8_t;
                                current_block_175 = 5207889489643863322;
                            }
                            112 => {
                                xclass_flags = (if list_ptr
                                    == &raw mut list as *mut uint32_t as *const uint32_t
                                {
                                    code
                                } else {
                                    base_end
                                })
                                .offset(
                                    -(*list_ptr.offset(2 as ::core::ffi::c_int as isize) as isize),
                                )
                                .offset(LINK_SIZE as isize);
                                if *xclass_flags as ::core::ffi::c_int & XCL_HASPROP
                                    != 0 as ::core::ffi::c_int
                                {
                                    return FALSE;
                                }
                                if *xclass_flags as ::core::ffi::c_int & XCL_MAP
                                    == 0 as ::core::ffi::c_int
                                {
                                    if list[1 as ::core::ffi::c_int as usize] == 0 as uint32_t {
                                        return (*xclass_flags as ::core::ffi::c_int & XCL_NOT
                                            == 0 as ::core::ffi::c_int)
                                            as ::core::ffi::c_int;
                                    }
                                    continue;
                                } else {
                                    set2 = xclass_flags.offset(1 as ::core::ffi::c_int as isize)
                                        as *const uint8_t;
                                }
                                current_block_175 = 5207889489643863322;
                            }
                            6 => {
                                invert_bits = TRUE as BOOL;
                                current_block_175 = 16214297756323152477;
                            }
                            7 => {
                                current_block_175 = 16214297756323152477;
                            }
                            8 => {
                                invert_bits = TRUE as BOOL;
                                current_block_175 = 4504381626022579262;
                            }
                            9 => {
                                current_block_175 = 4504381626022579262;
                            }
                            10 => {
                                invert_bits = TRUE as BOOL;
                                current_block_175 = 3426079656821633100;
                            }
                            11 => {
                                current_block_175 = 3426079656821633100;
                            }
                            _ => return FALSE,
                        }
                        match current_block_175 {
                            16214297756323152477 => {
                                set2 = (*cb).cbits.offset(cbit_digit as isize);
                            }
                            4504381626022579262 => {
                                set2 = (*cb).cbits.offset(cbit_space as isize);
                            }
                            3426079656821633100 => {
                                set2 = (*cb).cbits.offset(cbit_word as isize);
                            }
                            _ => {}
                        }
                        set_end = set1.offset(32 as ::core::ffi::c_int as isize);
                        if invert_bits != 0 {
                            loop {
                                let fresh0 = set1;
                                set1 = set1.offset(1);
                                let fresh1 = set2;
                                set2 = set2.offset(1);
                                if *fresh0 as ::core::ffi::c_int & !(*fresh1 as ::core::ffi::c_int)
                                    != 0 as ::core::ffi::c_int
                                {
                                    return FALSE;
                                }
                                if !(set1 < set_end) {
                                    break;
                                }
                            }
                        } else {
                            loop {
                                let fresh2 = set1;
                                set1 = set1.offset(1);
                                let fresh3 = set2;
                                set2 = set2.offset(1);
                                if *fresh2 as ::core::ffi::c_int & *fresh3 as ::core::ffi::c_int
                                    != 0 as ::core::ffi::c_int
                                {
                                    return FALSE;
                                }
                                if !(set1 < set_end) {
                                    break;
                                }
                            }
                        }
                        if list[1 as ::core::ffi::c_int as usize] == 0 as uint32_t {
                            return TRUE;
                        }
                        continue;
                    } else {
                        let mut leftop: uint32_t = 0;
                        let mut rightop: uint32_t = 0;
                        leftop = *base_list.offset(0 as ::core::ffi::c_int as isize);
                        rightop = list[0 as ::core::ffi::c_int as usize];
                        accepted = FALSE as BOOL;
                        if leftop == OP_PROP as ::core::ffi::c_int as uint32_t
                            || leftop == OP_NOTPROP as ::core::ffi::c_int as uint32_t
                        {
                            if rightop == OP_EOD as ::core::ffi::c_int as uint32_t {
                                accepted = TRUE as BOOL;
                            } else if rightop == OP_PROP as ::core::ffi::c_int as uint32_t
                                || rightop == OP_NOTPROP as ::core::ffi::c_int as uint32_t
                            {
                                let mut n: ::core::ffi::c_int = 0;
                                let mut p: *const uint8_t = ::core::ptr::null::<uint8_t>();
                                let mut same: BOOL = (leftop == rightop) as ::core::ffi::c_int;
                                let mut lisprop: BOOL = (leftop
                                    == OP_PROP as ::core::ffi::c_int as uint32_t)
                                    as ::core::ffi::c_int;
                                let mut risprop: BOOL = (rightop
                                    == OP_PROP as ::core::ffi::c_int as uint32_t)
                                    as ::core::ffi::c_int;
                                let mut bothprop: BOOL =
                                    (lisprop != 0 && risprop != 0) as ::core::ffi::c_int;
                                n = propposstab
                                    [*base_list.offset(2 as ::core::ffi::c_int as isize) as usize]
                                    [list[2 as ::core::ffi::c_int as usize] as usize]
                                    as ::core::ffi::c_int;
                                match n {
                                    1 => {
                                        accepted = bothprop;
                                    }
                                    2 => {
                                        accepted = ((*base_list
                                            .offset(3 as ::core::ffi::c_int as isize)
                                            == list[3 as ::core::ffi::c_int as usize])
                                            as ::core::ffi::c_int
                                            != same)
                                            as ::core::ffi::c_int
                                            as BOOL;
                                    }
                                    3 => {
                                        accepted = (same == 0) as ::core::ffi::c_int as BOOL;
                                    }
                                    4 => {
                                        accepted = (risprop != 0
                                            && catposstab[*base_list
                                                .offset(3 as ::core::ffi::c_int as isize)
                                                as usize]
                                                [list[3 as ::core::ffi::c_int as usize] as usize]
                                                as ::core::ffi::c_int
                                                == same)
                                            as ::core::ffi::c_int
                                            as BOOL;
                                    }
                                    5 => {
                                        accepted = (lisprop != 0
                                            && catposstab
                                                [list[3 as ::core::ffi::c_int as usize] as usize]
                                                [*base_list.offset(3 as ::core::ffi::c_int as isize)
                                                    as usize]
                                                as ::core::ffi::c_int
                                                == same)
                                            as ::core::ffi::c_int
                                            as BOOL;
                                    }
                                    6 | 7 | 8 => {
                                        p = &raw const *(&raw const posspropstab
                                            as *const [uint8_t; 4])
                                            .offset((n - 6 as ::core::ffi::c_int) as isize)
                                            as *const uint8_t;
                                        accepted = (risprop != 0
                                            && lisprop
                                                == (list[3 as ::core::ffi::c_int as usize]
                                                    != *p.offset(0 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                    && list[3 as ::core::ffi::c_int as usize]
                                                        != *p.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as uint32_t
                                                    && (list[3 as ::core::ffi::c_int as usize]
                                                        != *p.offset(
                                                            2 as ::core::ffi::c_int as isize,
                                                        )
                                                            as uint32_t
                                                        || lisprop == 0))
                                                    as ::core::ffi::c_int)
                                            as ::core::ffi::c_int
                                            as BOOL;
                                    }
                                    9 | 10 | 11 => {
                                        p = &raw const *(&raw const posspropstab
                                            as *const [uint8_t; 4])
                                            .offset((n - 9 as ::core::ffi::c_int) as isize)
                                            as *const uint8_t;
                                        accepted = (lisprop != 0
                                            && risprop
                                                == (*base_list
                                                    .offset(3 as ::core::ffi::c_int as isize)
                                                    != *p.offset(0 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                    && *base_list
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        != *p.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as uint32_t
                                                    && (*base_list
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        != *p.offset(
                                                            2 as ::core::ffi::c_int as isize,
                                                        )
                                                            as uint32_t
                                                        || risprop == 0))
                                                    as ::core::ffi::c_int)
                                            as ::core::ffi::c_int
                                            as BOOL;
                                    }
                                    12 | 13 | 14 => {
                                        p = &raw const *(&raw const posspropstab
                                            as *const [uint8_t; 4])
                                            .offset((n - 12 as ::core::ffi::c_int) as isize)
                                            as *const uint8_t;
                                        accepted = (risprop != 0
                                            && lisprop
                                                == (catposstab[*p
                                                    .offset(0 as ::core::ffi::c_int as isize)
                                                    as usize]
                                                    [list[3 as ::core::ffi::c_int as usize]
                                                        as usize]
                                                    as ::core::ffi::c_int
                                                    != 0
                                                    && catposstab[*p
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as usize]
                                                        [list[3 as ::core::ffi::c_int as usize]
                                                            as usize]
                                                        as ::core::ffi::c_int
                                                        != 0
                                                    && (list[3 as ::core::ffi::c_int as usize]
                                                        != *p.offset(
                                                            3 as ::core::ffi::c_int as isize,
                                                        )
                                                            as uint32_t
                                                        || lisprop == 0))
                                                    as ::core::ffi::c_int)
                                            as ::core::ffi::c_int
                                            as BOOL;
                                    }
                                    15 | 16 | 17 => {
                                        p = &raw const *(&raw const posspropstab
                                            as *const [uint8_t; 4])
                                            .offset((n - 15 as ::core::ffi::c_int) as isize)
                                            as *const uint8_t;
                                        accepted = (lisprop != 0
                                            && risprop
                                                == (catposstab[*p
                                                    .offset(0 as ::core::ffi::c_int as isize)
                                                    as usize]
                                                    [*base_list
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        as usize]
                                                    as ::core::ffi::c_int
                                                    != 0
                                                    && catposstab[*p
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as usize]
                                                        [*base_list.offset(
                                                            3 as ::core::ffi::c_int as isize,
                                                        )
                                                            as usize]
                                                        as ::core::ffi::c_int
                                                        != 0
                                                    && (*base_list
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        != *p.offset(
                                                            3 as ::core::ffi::c_int as isize,
                                                        )
                                                            as uint32_t
                                                        || risprop == 0))
                                                    as ::core::ffi::c_int)
                                            as ::core::ffi::c_int
                                            as BOOL;
                                    }
                                    0 | _ => {}
                                }
                            }
                        } else {
                            accepted = (leftop >= OP_NOT_DIGIT as ::core::ffi::c_int as uint32_t
                                && leftop <= OP_EXTUNI as ::core::ffi::c_int as uint32_t
                                && rightop >= OP_NOT_DIGIT as ::core::ffi::c_int as uint32_t
                                && rightop <= OP_DOLLM as ::core::ffi::c_int as uint32_t
                                && autoposstab[leftop
                                    .wrapping_sub(OP_NOT_DIGIT as ::core::ffi::c_int as uint32_t)
                                    as usize][rightop
                                    .wrapping_sub(OP_NOT_DIGIT as ::core::ffi::c_int as uint32_t)
                                    as usize]
                                    as ::core::ffi::c_int
                                    != 0)
                                as ::core::ffi::c_int
                                as BOOL;
                        }
                        if accepted == 0 {
                            return FALSE;
                        }
                        if list[1 as ::core::ffi::c_int as usize] == 0 as uint32_t {
                            return TRUE;
                        }
                        continue;
                    }
                    loop {
                        chr = *chr_ptr;
                        let mut current_block_169: u64;
                        match *list_ptr.offset(0 as ::core::ffi::c_int as isize) {
                            29 => {
                                ochr_ptr = list_ptr.offset(2 as ::core::ffi::c_int as isize);
                                loop {
                                    if chr == *ochr_ptr {
                                        return FALSE;
                                    }
                                    ochr_ptr = ochr_ptr.offset(1);
                                    if !(*ochr_ptr != NOTACHAR as uint32_t) {
                                        break;
                                    }
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            31 => {
                                ochr_ptr = list_ptr.offset(2 as ::core::ffi::c_int as isize);
                                while !(chr == *ochr_ptr) {
                                    ochr_ptr = ochr_ptr.offset(1);
                                    if !(*ochr_ptr != NOTACHAR as uint32_t) {
                                        break;
                                    }
                                }
                                if *ochr_ptr == NOTACHAR as uint32_t {
                                    return FALSE;
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            7 => {
                                if chr < 256 as uint32_t
                                    && *(*cb).ctypes.offset(chr as isize) as ::core::ffi::c_int
                                        & ctype_digit
                                        != 0 as ::core::ffi::c_int
                                {
                                    return FALSE;
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            6 => {
                                if chr > 255 as uint32_t
                                    || *(*cb).ctypes.offset(chr as isize) as ::core::ffi::c_int
                                        & ctype_digit
                                        == 0 as ::core::ffi::c_int
                                {
                                    return FALSE;
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            9 => {
                                if chr < 256 as uint32_t
                                    && *(*cb).ctypes.offset(chr as isize) as ::core::ffi::c_int
                                        & ctype_space
                                        != 0 as ::core::ffi::c_int
                                {
                                    return FALSE;
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            8 => {
                                if chr > 255 as uint32_t
                                    || *(*cb).ctypes.offset(chr as isize) as ::core::ffi::c_int
                                        & ctype_space
                                        == 0 as ::core::ffi::c_int
                                {
                                    return FALSE;
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            11 => {
                                if chr < 255 as uint32_t
                                    && *(*cb).ctypes.offset(chr as isize) as ::core::ffi::c_int
                                        & ctype_word
                                        != 0 as ::core::ffi::c_int
                                {
                                    return FALSE;
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            10 => {
                                if chr > 255 as uint32_t
                                    || *(*cb).ctypes.offset(chr as isize) as ::core::ffi::c_int
                                        & ctype_word
                                        == 0 as ::core::ffi::c_int
                                {
                                    return FALSE;
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            19 => {
                                match chr {
                                    9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194 | 8195
                                    | 8196 | 8197 | 8198 | 8199 | 8200 | 8201 | 8202 | 8239
                                    | 8287 | 12288 => return FALSE,
                                    _ => {}
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            18 => {
                                match chr {
                                    9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194 | 8195
                                    | 8196 | 8197 | 8198 | 8199 | 8200 | 8201 | 8202 | 8239
                                    | 8287 | 12288 => {}
                                    _ => return FALSE,
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            17 | 21 => {
                                match chr {
                                    10 | 11 | 12 | 13 | 133 | 8232 | 8233 => return FALSE,
                                    _ => {}
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            20 => {
                                match chr {
                                    10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {}
                                    _ => return FALSE,
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            25 | 23 => {
                                match chr {
                                    13 | 10 | 11 | 12 | 133 | 8232 | 8233 => return FALSE,
                                    _ => {}
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            24 => {
                                current_block_169 = 10257223768985283691;
                            }
                            16 | 15 => {
                                if check_char_prop(
                                    chr,
                                    *list_ptr.offset(2 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_uint,
                                    *list_ptr.offset(3 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_uint,
                                    (*list_ptr.offset(0 as ::core::ffi::c_int as isize)
                                        == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                        as ::core::ffi::c_int,
                                ) == 0
                                {
                                    return FALSE;
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            111 => {
                                if chr > 255 as uint32_t {
                                    return FALSE;
                                }
                                current_block_169 = 6555712040169370672;
                            }
                            110 => {
                                current_block_169 = 6555712040169370672;
                            }
                            112 => {
                                if _pcre2_xclass_16(
                                    chr,
                                    (if list_ptr
                                        == &raw mut list as *mut uint32_t as *const uint32_t
                                    {
                                        code
                                    } else {
                                        base_end
                                    })
                                    .offset(
                                        -(*list_ptr.offset(2 as ::core::ffi::c_int as isize)
                                            as isize),
                                    )
                                    .offset(LINK_SIZE as isize),
                                    (*cb).start_code as *const uint8_t,
                                    utf,
                                ) != 0
                                {
                                    return FALSE;
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            113 => {
                                if _pcre2_eclass_16(
                                    chr,
                                    (if list_ptr
                                        == &raw mut list as *mut uint32_t as *const uint32_t
                                    {
                                        code
                                    } else {
                                        base_end
                                    })
                                    .offset(
                                        -(*list_ptr.offset(2 as ::core::ffi::c_int as isize)
                                            as isize),
                                    )
                                    .offset(LINK_SIZE as isize),
                                    (if list_ptr
                                        == &raw mut list as *mut uint32_t as *const uint32_t
                                    {
                                        code
                                    } else {
                                        base_end
                                    })
                                    .offset(
                                        -(*list_ptr.offset(3 as ::core::ffi::c_int as isize)
                                            as isize),
                                    ),
                                    (*cb).start_code as *const uint8_t,
                                    utf,
                                ) != 0
                                {
                                    return FALSE;
                                }
                                current_block_169 = 10257223768985283691;
                            }
                            _ => return FALSE,
                        }
                        match current_block_169 {
                            6555712040169370672 => {
                                if !(chr > 255 as uint32_t) {
                                    class_bitset = (if list_ptr
                                        == &raw mut list as *mut uint32_t as *const uint32_t
                                    {
                                        code
                                    } else {
                                        base_end
                                    })
                                    .offset(
                                        -(*list_ptr.offset(2 as ::core::ffi::c_int as isize)
                                            as isize),
                                    )
                                        as *const uint8_t;
                                    if *class_bitset
                                        .offset((chr >> 3 as ::core::ffi::c_int) as isize)
                                        as ::core::ffi::c_uint
                                        & (1 as ::core::ffi::c_uint) << (chr & 7 as uint32_t)
                                        != 0 as ::core::ffi::c_uint
                                    {
                                        return FALSE;
                                    }
                                }
                            }
                            _ => {}
                        }
                        chr_ptr = chr_ptr.offset(1);
                        if !(*chr_ptr != NOTACHAR as uint32_t) {
                            break;
                        }
                    }
                    if list[1 as ::core::ffi::c_int as usize] == 0 as uint32_t {
                        return TRUE;
                    }
                }
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_auto_possessify_16(
    mut code: *mut PCRE2_UCHAR16,
    mut cb: *const compile_block_16,
) -> ::core::ffi::c_int {
    let mut c: PCRE2_UCHAR16 = 0;
    let mut end: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut repeat_opcode: *mut PCRE2_UCHAR16 = ::core::ptr::null_mut::<PCRE2_UCHAR16>();
    let mut list: [uint32_t; 8] = [0; 8];
    let mut rec_limit: ::core::ffi::c_int = 1000 as ::core::ffi::c_int;
    let mut utf: BOOL =
        ((*cb).external_options & PCRE2_UTF as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    let mut ucp: BOOL =
        ((*cb).external_options & PCRE2_UCP as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    loop {
        c = *code;
        if c as ::core::ffi::c_int >= OP_TABLE_LENGTH as ::core::ffi::c_int {
            return -(1 as ::core::ffi::c_int);
        }
        if c as ::core::ffi::c_int >= OP_STAR as ::core::ffi::c_int
            && c as ::core::ffi::c_int <= OP_TYPEPOSUPTO as ::core::ffi::c_int
        {
            c = (c as ::core::ffi::c_int
                - (get_repeat_base(c) as ::core::ffi::c_int - OP_STAR as ::core::ffi::c_int))
                as PCRE2_UCHAR16;
            end = if c as ::core::ffi::c_int <= OP_MINUPTO as ::core::ffi::c_int {
                get_chr_property_list(
                    code as PCRE2_SPTR16,
                    utf,
                    ucp,
                    (*cb).fcc,
                    &raw mut list as *mut uint32_t,
                )
            } else {
                ::core::ptr::null::<PCRE2_UCHAR16>()
            };
            list[1 as ::core::ffi::c_int as usize] =
                (c as ::core::ffi::c_int == OP_STAR as ::core::ffi::c_int
                    || c as ::core::ffi::c_int == OP_PLUS as ::core::ffi::c_int
                    || c as ::core::ffi::c_int == OP_QUERY as ::core::ffi::c_int
                    || c as ::core::ffi::c_int == OP_UPTO as ::core::ffi::c_int)
                    as ::core::ffi::c_int as uint32_t;
            if !end.is_null()
                && compare_opcodes(
                    end,
                    utf,
                    ucp,
                    cb,
                    &raw mut list as *mut uint32_t,
                    end,
                    &raw mut rec_limit,
                ) != 0
            {
                match c as ::core::ffi::c_int {
                    33 => {
                        *code = (*code as ::core::ffi::c_int
                            + (OP_POSSTAR as ::core::ffi::c_int - OP_STAR as ::core::ffi::c_int))
                            as PCRE2_UCHAR16;
                    }
                    34 => {
                        *code = (*code as ::core::ffi::c_int
                            + (OP_POSSTAR as ::core::ffi::c_int - OP_MINSTAR as ::core::ffi::c_int))
                            as PCRE2_UCHAR16;
                    }
                    35 => {
                        *code = (*code as ::core::ffi::c_int
                            + (OP_POSPLUS as ::core::ffi::c_int - OP_PLUS as ::core::ffi::c_int))
                            as PCRE2_UCHAR16;
                    }
                    36 => {
                        *code = (*code as ::core::ffi::c_int
                            + (OP_POSPLUS as ::core::ffi::c_int - OP_MINPLUS as ::core::ffi::c_int))
                            as PCRE2_UCHAR16;
                    }
                    37 => {
                        *code = (*code as ::core::ffi::c_int
                            + (OP_POSQUERY as ::core::ffi::c_int - OP_QUERY as ::core::ffi::c_int))
                            as PCRE2_UCHAR16;
                    }
                    38 => {
                        *code = (*code as ::core::ffi::c_int
                            + (OP_POSQUERY as ::core::ffi::c_int
                                - OP_MINQUERY as ::core::ffi::c_int))
                            as PCRE2_UCHAR16;
                    }
                    39 => {
                        *code = (*code as ::core::ffi::c_int
                            + (OP_POSUPTO as ::core::ffi::c_int - OP_UPTO as ::core::ffi::c_int))
                            as PCRE2_UCHAR16;
                    }
                    40 => {
                        *code = (*code as ::core::ffi::c_int
                            + (OP_POSUPTO as ::core::ffi::c_int - OP_MINUPTO as ::core::ffi::c_int))
                            as PCRE2_UCHAR16;
                    }
                    _ => {}
                }
            }
            c = *code;
        } else if c as ::core::ffi::c_int == OP_CLASS as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_NCLASS as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_XCLASS as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_ECLASS as ::core::ffi::c_int
        {
            if c as ::core::ffi::c_int == OP_XCLASS as ::core::ffi::c_int
                || c as ::core::ffi::c_int == OP_ECLASS as ::core::ffi::c_int
            {
                repeat_opcode = code.offset(
                    *code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int as isize
                );
            } else {
                repeat_opcode = code.offset(1 as ::core::ffi::c_int as isize).offset(
                    (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR16>() as usize)
                        as isize,
                );
            }
            c = *repeat_opcode;
            if c as ::core::ffi::c_int >= OP_CRSTAR as ::core::ffi::c_int
                && c as ::core::ffi::c_int <= OP_CRMINRANGE as ::core::ffi::c_int
            {
                end = get_chr_property_list(
                    code as PCRE2_SPTR16,
                    utf,
                    ucp,
                    (*cb).fcc,
                    &raw mut list as *mut uint32_t,
                );
                list[1 as ::core::ffi::c_int as usize] =
                    (c as ::core::ffi::c_int & 1 as ::core::ffi::c_int == 0 as ::core::ffi::c_int)
                        as ::core::ffi::c_int as uint32_t;
                if !end.is_null()
                    && compare_opcodes(
                        end,
                        utf,
                        ucp,
                        cb,
                        &raw mut list as *mut uint32_t,
                        end,
                        &raw mut rec_limit,
                    ) != 0
                {
                    match c as ::core::ffi::c_int {
                        98 | 99 => {
                            *repeat_opcode = OP_CRPOSSTAR as ::core::ffi::c_int as PCRE2_UCHAR16;
                        }
                        100 | 101 => {
                            *repeat_opcode = OP_CRPOSPLUS as ::core::ffi::c_int as PCRE2_UCHAR16;
                        }
                        102 | 103 => {
                            *repeat_opcode = OP_CRPOSQUERY as ::core::ffi::c_int as PCRE2_UCHAR16;
                        }
                        104 | 105 => {
                            *repeat_opcode = OP_CRPOSRANGE as ::core::ffi::c_int as PCRE2_UCHAR16;
                        }
                        _ => {}
                    }
                }
            }
            c = *code;
        }
        match c as ::core::ffi::c_int {
            0 => return 0 as ::core::ffi::c_int,
            85 | 86 | 87 | 88 | 89 | 90 | 94 | 95 | 96 => {
                if *code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    == OP_PROP as ::core::ffi::c_int
                    || *code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == OP_NOTPROP as ::core::ffi::c_int
                {
                    code = code.offset(2 as ::core::ffi::c_int as isize);
                }
            }
            91 | 92 | 93 | 97 => {
                if *code.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize)
                    as ::core::ffi::c_int
                    == OP_PROP as ::core::ffi::c_int
                    || *code.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize)
                        as ::core::ffi::c_int
                        == OP_NOTPROP as ::core::ffi::c_int
                {
                    code = code.offset(2 as ::core::ffi::c_int as isize);
                }
            }
            120 => {
                code = code.offset(*code.offset(
                    (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * 1 as ::core::ffi::c_int)
                        as isize,
                ) as ::core::ffi::c_int as isize);
            }
            112 | 113 => {
                code = code.offset(
                    *code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int as isize
                );
            }
            156 | 164 | 158 | 160 | 162 => {
                code = code.offset(
                    *code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int as isize
                );
            }
            _ => {}
        }
        code = code.offset(
            *(&raw const _pcre2_OP_lengths_16 as *const uint8_t).offset(c as isize)
                as ::core::ffi::c_int as isize,
        );
        if utf != 0 {
            match c as ::core::ffi::c_int {
                29 | 30 | 31 | 32 | 33 | 34 | 35 | 36 | 37 | 38 | 39 | 40 | 41 | 42 | 43 | 44
                | 45 | 46 | 47 | 48 | 49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 | 58 | 59 | 60
                | 61 | 62 | 63 | 64 | 65 | 66 | 67 | 68 | 69 | 70 | 71 | 72 | 73 | 74 | 75 | 76
                | 77 | 78 | 79 | 80 | 81 | 82 | 83 | 84 => {
                    if *code.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_uint
                        & 0xfc00 as ::core::ffi::c_uint
                        == 0xd800 as ::core::ffi::c_uint
                    {
                        code = code.offset(1 as ::core::ffi::c_int as isize);
                    }
                }
                _ => {}
            }
        }
    }
}
