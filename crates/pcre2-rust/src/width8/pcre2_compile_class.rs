// Translated from PCRE2 10.49 pcre2_compile_class.c; code unit width 8.
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
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memmove(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    static _pcre2_hspace_list_8: [uint32_t; 0];
    static _pcre2_vspace_list_8: [uint32_t; 0];
    static _pcre2_ucd_boolprop_sets_8: [uint32_t; 0];
    static _pcre2_ucd_caseless_sets_8: [uint32_t; 0];
    static _pcre2_ucd_turkish_dotted_i_caseset_8: uint32_t;
    static _pcre2_ucd_nocase_ranges_8: [uint32_t; 0];
    static _pcre2_ucd_nocase_ranges_size_8: uint32_t;
    static _pcre2_ucd_script_sets_8: [uint32_t; 0];
    static _pcre2_ucd_records_8: [ucd_record; 0];
    static _pcre2_ucd_stage1_8: [uint16_t; 0];
    static _pcre2_ucd_stage2_8: [uint16_t; 0];
    static _pcre2_ucp_gentype_8: [uint32_t; 0];
    fn _pcre2_ord2utf_8(_: uint32_t, _: *mut PCRE2_UCHAR8) -> ::core::ffi::c_uint;
    static _pcre2_posix_class_maps8: [::core::ffi::c_int; 0];
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
pub struct pcre2_real_compile_context_8 {
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
pub const ESC_ub: C2RustUnnamed_1 = 29;
pub const ESC_k: C2RustUnnamed_1 = 28;
pub const ESC_g: C2RustUnnamed_1 = 27;
pub const ESC_Q: C2RustUnnamed_1 = 26;
pub const ESC_E: C2RustUnnamed_1 = 25;
pub const ESC_z: C2RustUnnamed_1 = 24;
pub const ESC_Z: C2RustUnnamed_1 = 23;
pub const ESC_X: C2RustUnnamed_1 = 22;
pub const ESC_v: C2RustUnnamed_1 = 21;
pub const ESC_V: C2RustUnnamed_1 = 20;
pub const ESC_h: C2RustUnnamed_1 = 19;
pub const ESC_H: C2RustUnnamed_1 = 18;
pub const ESC_R: C2RustUnnamed_1 = 17;
pub const ESC_p: C2RustUnnamed_1 = 16;
pub const ESC_P: C2RustUnnamed_1 = 15;
pub const ESC_C: C2RustUnnamed_1 = 14;
pub const ESC_dum: C2RustUnnamed_1 = 13;
pub const ESC_N: C2RustUnnamed_1 = 12;
pub const ESC_w: C2RustUnnamed_1 = 11;
pub const ESC_W: C2RustUnnamed_1 = 10;
pub const ESC_s: C2RustUnnamed_1 = 9;
pub const ESC_S: C2RustUnnamed_1 = 8;
pub const ESC_d: C2RustUnnamed_1 = 7;
pub const ESC_D: C2RustUnnamed_1 = 6;
pub const ESC_b: C2RustUnnamed_1 = 5;
pub const ESC_B: C2RustUnnamed_1 = 4;
pub const ESC_K: C2RustUnnamed_1 = 3;
pub const ESC_G: C2RustUnnamed_1 = 2;
pub const ESC_A: C2RustUnnamed_1 = 1;
pub type C2RustUnnamed_2 = ::core::ffi::c_uint;
pub const OP_TABLE_LENGTH: C2RustUnnamed_2 = 173;
pub const OP_UCP_WORD_BOUNDARY: C2RustUnnamed_2 = 172;
pub const OP_NOT_UCP_WORD_BOUNDARY: C2RustUnnamed_2 = 171;
pub const OP_DEFINE: C2RustUnnamed_2 = 170;
pub const OP_SKIPZERO: C2RustUnnamed_2 = 169;
pub const OP_CLOSE: C2RustUnnamed_2 = 168;
pub const OP_ASSERT_ACCEPT: C2RustUnnamed_2 = 167;
pub const OP_ACCEPT: C2RustUnnamed_2 = 166;
pub const OP_FAIL: C2RustUnnamed_2 = 165;
pub const OP_COMMIT_ARG: C2RustUnnamed_2 = 164;
pub const OP_COMMIT: C2RustUnnamed_2 = 163;
pub const OP_THEN_ARG: C2RustUnnamed_2 = 162;
pub const OP_THEN: C2RustUnnamed_2 = 161;
pub const OP_SKIP_ARG: C2RustUnnamed_2 = 160;
pub const OP_SKIP: C2RustUnnamed_2 = 159;
pub const OP_PRUNE_ARG: C2RustUnnamed_2 = 158;
pub const OP_PRUNE: C2RustUnnamed_2 = 157;
pub const OP_MARK: C2RustUnnamed_2 = 156;
pub const OP_BRAPOSZERO: C2RustUnnamed_2 = 155;
pub const OP_BRAMINZERO: C2RustUnnamed_2 = 154;
pub const OP_BRAZERO: C2RustUnnamed_2 = 153;
pub const OP_TRUE: C2RustUnnamed_2 = 152;
pub const OP_FALSE: C2RustUnnamed_2 = 151;
pub const OP_DNRREF: C2RustUnnamed_2 = 150;
pub const OP_RREF: C2RustUnnamed_2 = 149;
pub const OP_DNCREF: C2RustUnnamed_2 = 148;
pub const OP_CREF: C2RustUnnamed_2 = 147;
pub const OP_SCOND: C2RustUnnamed_2 = 146;
pub const OP_SCBRAPOS: C2RustUnnamed_2 = 145;
pub const OP_SCBRA: C2RustUnnamed_2 = 144;
pub const OP_SBRAPOS: C2RustUnnamed_2 = 143;
pub const OP_SBRA: C2RustUnnamed_2 = 142;
pub const OP_COND: C2RustUnnamed_2 = 141;
pub const OP_CBRAPOS: C2RustUnnamed_2 = 140;
pub const OP_CBRA: C2RustUnnamed_2 = 139;
pub const OP_BRAPOS: C2RustUnnamed_2 = 138;
pub const OP_BRA: C2RustUnnamed_2 = 137;
pub const OP_SCRIPT_RUN: C2RustUnnamed_2 = 136;
pub const OP_ONCE: C2RustUnnamed_2 = 135;
pub const OP_ASSERT_SCS: C2RustUnnamed_2 = 134;
pub const OP_ASSERTBACK_NA: C2RustUnnamed_2 = 133;
pub const OP_ASSERT_NA: C2RustUnnamed_2 = 132;
pub const OP_ASSERTBACK_NOT: C2RustUnnamed_2 = 131;
pub const OP_ASSERTBACK: C2RustUnnamed_2 = 130;
pub const OP_ASSERT_NOT: C2RustUnnamed_2 = 129;
pub const OP_ASSERT: C2RustUnnamed_2 = 128;
pub const OP_VREVERSE: C2RustUnnamed_2 = 127;
pub const OP_REVERSE: C2RustUnnamed_2 = 126;
pub const OP_KETRPOS: C2RustUnnamed_2 = 125;
pub const OP_KETRMIN: C2RustUnnamed_2 = 124;
pub const OP_KETRMAX: C2RustUnnamed_2 = 123;
pub const OP_KET: C2RustUnnamed_2 = 122;
pub const OP_ALT: C2RustUnnamed_2 = 121;
pub const OP_CALLOUT_STR: C2RustUnnamed_2 = 120;
pub const OP_CALLOUT: C2RustUnnamed_2 = 119;
pub const OP_RECURSE: C2RustUnnamed_2 = 118;
pub const OP_DNREFI: C2RustUnnamed_2 = 117;
pub const OP_DNREF: C2RustUnnamed_2 = 116;
pub const OP_REFI: C2RustUnnamed_2 = 115;
pub const OP_REF: C2RustUnnamed_2 = 114;
pub const OP_ECLASS: C2RustUnnamed_2 = 113;
pub const OP_XCLASS: C2RustUnnamed_2 = 112;
pub const OP_NCLASS: C2RustUnnamed_2 = 111;
pub const OP_CLASS: C2RustUnnamed_2 = 110;
pub const OP_CRPOSRANGE: C2RustUnnamed_2 = 109;
pub const OP_CRPOSQUERY: C2RustUnnamed_2 = 108;
pub const OP_CRPOSPLUS: C2RustUnnamed_2 = 107;
pub const OP_CRPOSSTAR: C2RustUnnamed_2 = 106;
pub const OP_CRMINRANGE: C2RustUnnamed_2 = 105;
pub const OP_CRRANGE: C2RustUnnamed_2 = 104;
pub const OP_CRMINQUERY: C2RustUnnamed_2 = 103;
pub const OP_CRQUERY: C2RustUnnamed_2 = 102;
pub const OP_CRMINPLUS: C2RustUnnamed_2 = 101;
pub const OP_CRPLUS: C2RustUnnamed_2 = 100;
pub const OP_CRMINSTAR: C2RustUnnamed_2 = 99;
pub const OP_CRSTAR: C2RustUnnamed_2 = 98;
pub const OP_TYPEPOSUPTO: C2RustUnnamed_2 = 97;
pub const OP_TYPEPOSQUERY: C2RustUnnamed_2 = 96;
pub const OP_TYPEPOSPLUS: C2RustUnnamed_2 = 95;
pub const OP_TYPEPOSSTAR: C2RustUnnamed_2 = 94;
pub const OP_TYPEEXACT: C2RustUnnamed_2 = 93;
pub const OP_TYPEMINUPTO: C2RustUnnamed_2 = 92;
pub const OP_TYPEUPTO: C2RustUnnamed_2 = 91;
pub const OP_TYPEMINQUERY: C2RustUnnamed_2 = 90;
pub const OP_TYPEQUERY: C2RustUnnamed_2 = 89;
pub const OP_TYPEMINPLUS: C2RustUnnamed_2 = 88;
pub const OP_TYPEPLUS: C2RustUnnamed_2 = 87;
pub const OP_TYPEMINSTAR: C2RustUnnamed_2 = 86;
pub const OP_TYPESTAR: C2RustUnnamed_2 = 85;
pub const OP_NOTPOSUPTOI: C2RustUnnamed_2 = 84;
pub const OP_NOTPOSQUERYI: C2RustUnnamed_2 = 83;
pub const OP_NOTPOSPLUSI: C2RustUnnamed_2 = 82;
pub const OP_NOTPOSSTARI: C2RustUnnamed_2 = 81;
pub const OP_NOTEXACTI: C2RustUnnamed_2 = 80;
pub const OP_NOTMINUPTOI: C2RustUnnamed_2 = 79;
pub const OP_NOTUPTOI: C2RustUnnamed_2 = 78;
pub const OP_NOTMINQUERYI: C2RustUnnamed_2 = 77;
pub const OP_NOTQUERYI: C2RustUnnamed_2 = 76;
pub const OP_NOTMINPLUSI: C2RustUnnamed_2 = 75;
pub const OP_NOTPLUSI: C2RustUnnamed_2 = 74;
pub const OP_NOTMINSTARI: C2RustUnnamed_2 = 73;
pub const OP_NOTSTARI: C2RustUnnamed_2 = 72;
pub const OP_NOTPOSUPTO: C2RustUnnamed_2 = 71;
pub const OP_NOTPOSQUERY: C2RustUnnamed_2 = 70;
pub const OP_NOTPOSPLUS: C2RustUnnamed_2 = 69;
pub const OP_NOTPOSSTAR: C2RustUnnamed_2 = 68;
pub const OP_NOTEXACT: C2RustUnnamed_2 = 67;
pub const OP_NOTMINUPTO: C2RustUnnamed_2 = 66;
pub const OP_NOTUPTO: C2RustUnnamed_2 = 65;
pub const OP_NOTMINQUERY: C2RustUnnamed_2 = 64;
pub const OP_NOTQUERY: C2RustUnnamed_2 = 63;
pub const OP_NOTMINPLUS: C2RustUnnamed_2 = 62;
pub const OP_NOTPLUS: C2RustUnnamed_2 = 61;
pub const OP_NOTMINSTAR: C2RustUnnamed_2 = 60;
pub const OP_NOTSTAR: C2RustUnnamed_2 = 59;
pub const OP_POSUPTOI: C2RustUnnamed_2 = 58;
pub const OP_POSQUERYI: C2RustUnnamed_2 = 57;
pub const OP_POSPLUSI: C2RustUnnamed_2 = 56;
pub const OP_POSSTARI: C2RustUnnamed_2 = 55;
pub const OP_EXACTI: C2RustUnnamed_2 = 54;
pub const OP_MINUPTOI: C2RustUnnamed_2 = 53;
pub const OP_UPTOI: C2RustUnnamed_2 = 52;
pub const OP_MINQUERYI: C2RustUnnamed_2 = 51;
pub const OP_QUERYI: C2RustUnnamed_2 = 50;
pub const OP_MINPLUSI: C2RustUnnamed_2 = 49;
pub const OP_PLUSI: C2RustUnnamed_2 = 48;
pub const OP_MINSTARI: C2RustUnnamed_2 = 47;
pub const OP_STARI: C2RustUnnamed_2 = 46;
pub const OP_POSUPTO: C2RustUnnamed_2 = 45;
pub const OP_POSQUERY: C2RustUnnamed_2 = 44;
pub const OP_POSPLUS: C2RustUnnamed_2 = 43;
pub const OP_POSSTAR: C2RustUnnamed_2 = 42;
pub const OP_EXACT: C2RustUnnamed_2 = 41;
pub const OP_MINUPTO: C2RustUnnamed_2 = 40;
pub const OP_UPTO: C2RustUnnamed_2 = 39;
pub const OP_MINQUERY: C2RustUnnamed_2 = 38;
pub const OP_QUERY: C2RustUnnamed_2 = 37;
pub const OP_MINPLUS: C2RustUnnamed_2 = 36;
pub const OP_PLUS: C2RustUnnamed_2 = 35;
pub const OP_MINSTAR: C2RustUnnamed_2 = 34;
pub const OP_STAR: C2RustUnnamed_2 = 33;
pub const OP_NOTI: C2RustUnnamed_2 = 32;
pub const OP_NOT: C2RustUnnamed_2 = 31;
pub const OP_CHARI: C2RustUnnamed_2 = 30;
pub const OP_CHAR: C2RustUnnamed_2 = 29;
pub const OP_CIRCM: C2RustUnnamed_2 = 28;
pub const OP_CIRC: C2RustUnnamed_2 = 27;
pub const OP_DOLLM: C2RustUnnamed_2 = 26;
pub const OP_DOLL: C2RustUnnamed_2 = 25;
pub const OP_EOD: C2RustUnnamed_2 = 24;
pub const OP_EODN: C2RustUnnamed_2 = 23;
pub const OP_EXTUNI: C2RustUnnamed_2 = 22;
pub const OP_VSPACE: C2RustUnnamed_2 = 21;
pub const OP_NOT_VSPACE: C2RustUnnamed_2 = 20;
pub const OP_HSPACE: C2RustUnnamed_2 = 19;
pub const OP_NOT_HSPACE: C2RustUnnamed_2 = 18;
pub const OP_ANYNL: C2RustUnnamed_2 = 17;
pub const OP_PROP: C2RustUnnamed_2 = 16;
pub const OP_NOTPROP: C2RustUnnamed_2 = 15;
pub const OP_ANYBYTE: C2RustUnnamed_2 = 14;
pub const OP_ALLANY: C2RustUnnamed_2 = 13;
pub const OP_ANY: C2RustUnnamed_2 = 12;
pub const OP_WORDCHAR: C2RustUnnamed_2 = 11;
pub const OP_NOT_WORDCHAR: C2RustUnnamed_2 = 10;
pub const OP_WHITESPACE: C2RustUnnamed_2 = 9;
pub const OP_NOT_WHITESPACE: C2RustUnnamed_2 = 8;
pub const OP_DIGIT: C2RustUnnamed_2 = 7;
pub const OP_NOT_DIGIT: C2RustUnnamed_2 = 6;
pub const OP_WORD_BOUNDARY: C2RustUnnamed_2 = 5;
pub const OP_NOT_WORD_BOUNDARY: C2RustUnnamed_2 = 4;
pub const OP_SET_SOM: C2RustUnnamed_2 = 3;
pub const OP_SOM: C2RustUnnamed_2 = 2;
pub const OP_SOD: C2RustUnnamed_2 = 1;
pub const OP_END: C2RustUnnamed_2 = 0;
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
pub struct named_group_8 {
    pub name: PCRE2_SPTR8,
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
pub struct class_ranges {
    pub header: compile_data,
    pub char_lists_size: size_t,
    pub char_lists_start: size_t,
    pub range_list_size: uint16_t,
    pub char_lists_types: uint16_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union class_bits_storage {
    pub classbits: [uint8_t; 32],
    pub classwords: [uint32_t; 8],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct compile_block_8 {
    pub cx: *mut pcre2_real_compile_context_8,
    pub lcc: *const uint8_t,
    pub fcc: *const uint8_t,
    pub cbits: *const uint8_t,
    pub ctypes: *const uint8_t,
    pub start_workspace: *mut PCRE2_UCHAR8,
    pub start_code: *mut PCRE2_UCHAR8,
    pub start_pattern: PCRE2_SPTR8,
    pub end_pattern: PCRE2_SPTR8,
    pub name_table: *mut PCRE2_UCHAR8,
    pub workspace_size: size_t,
    pub small_ref_offset: [size_t; 10],
    pub erroroffset: size_t,
    pub classbits: class_bits_storage,
    pub names_found: uint16_t,
    pub name_entry_size: uint16_t,
    pub parens_depth: uint16_t,
    pub assert_depth: uint16_t,
    pub named_groups: *mut named_group_8,
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
    pub nl: [PCRE2_UCHAR8; 4],
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
pub type C2RustUnnamed_3 = ::core::ffi::c_uint;
pub const ERR120: C2RustUnnamed_3 = 220;
pub const ERR119: C2RustUnnamed_3 = 219;
pub const ERR118: C2RustUnnamed_3 = 218;
pub const ERR117: C2RustUnnamed_3 = 217;
pub const ERR116: C2RustUnnamed_3 = 216;
pub const ERR115: C2RustUnnamed_3 = 215;
pub const ERR114: C2RustUnnamed_3 = 214;
pub const ERR113: C2RustUnnamed_3 = 213;
pub const ERR112: C2RustUnnamed_3 = 212;
pub const ERR111: C2RustUnnamed_3 = 211;
pub const ERR110: C2RustUnnamed_3 = 210;
pub const ERR109: C2RustUnnamed_3 = 209;
pub const ERR108: C2RustUnnamed_3 = 208;
pub const ERR107: C2RustUnnamed_3 = 207;
pub const ERR106: C2RustUnnamed_3 = 206;
pub const ERR105: C2RustUnnamed_3 = 205;
pub const ERR104: C2RustUnnamed_3 = 204;
pub const ERR103: C2RustUnnamed_3 = 203;
pub const ERR102: C2RustUnnamed_3 = 202;
pub const ERR101: C2RustUnnamed_3 = 201;
pub const ERR100: C2RustUnnamed_3 = 200;
pub const ERR99: C2RustUnnamed_3 = 199;
pub const ERR98: C2RustUnnamed_3 = 198;
pub const ERR97: C2RustUnnamed_3 = 197;
pub const ERR96: C2RustUnnamed_3 = 196;
pub const ERR95: C2RustUnnamed_3 = 195;
pub const ERR94: C2RustUnnamed_3 = 194;
pub const ERR93: C2RustUnnamed_3 = 193;
pub const ERR92: C2RustUnnamed_3 = 192;
pub const ERR91: C2RustUnnamed_3 = 191;
pub const ERR90: C2RustUnnamed_3 = 190;
pub const ERR89: C2RustUnnamed_3 = 189;
pub const ERR88: C2RustUnnamed_3 = 188;
pub const ERR87: C2RustUnnamed_3 = 187;
pub const ERR86: C2RustUnnamed_3 = 186;
pub const ERR85: C2RustUnnamed_3 = 185;
pub const ERR84: C2RustUnnamed_3 = 184;
pub const ERR83: C2RustUnnamed_3 = 183;
pub const ERR82: C2RustUnnamed_3 = 182;
pub const ERR81: C2RustUnnamed_3 = 181;
pub const ERR80: C2RustUnnamed_3 = 180;
pub const ERR79: C2RustUnnamed_3 = 179;
pub const ERR78: C2RustUnnamed_3 = 178;
pub const ERR77: C2RustUnnamed_3 = 177;
pub const ERR76: C2RustUnnamed_3 = 176;
pub const ERR75: C2RustUnnamed_3 = 175;
pub const ERR74: C2RustUnnamed_3 = 174;
pub const ERR73: C2RustUnnamed_3 = 173;
pub const ERR72: C2RustUnnamed_3 = 172;
pub const ERR71: C2RustUnnamed_3 = 171;
pub const ERR70: C2RustUnnamed_3 = 170;
pub const ERR69: C2RustUnnamed_3 = 169;
pub const ERR68: C2RustUnnamed_3 = 168;
pub const ERR67: C2RustUnnamed_3 = 167;
pub const ERR66: C2RustUnnamed_3 = 166;
pub const ERR65: C2RustUnnamed_3 = 165;
pub const ERR64: C2RustUnnamed_3 = 164;
pub const ERR63: C2RustUnnamed_3 = 163;
pub const ERR62: C2RustUnnamed_3 = 162;
pub const ERR61: C2RustUnnamed_3 = 161;
pub const ERR60: C2RustUnnamed_3 = 160;
pub const ERR59: C2RustUnnamed_3 = 159;
pub const ERR58: C2RustUnnamed_3 = 158;
pub const ERR57: C2RustUnnamed_3 = 157;
pub const ERR56: C2RustUnnamed_3 = 156;
pub const ERR55: C2RustUnnamed_3 = 155;
pub const ERR54: C2RustUnnamed_3 = 154;
pub const ERR53: C2RustUnnamed_3 = 153;
pub const ERR52: C2RustUnnamed_3 = 152;
pub const ERR51: C2RustUnnamed_3 = 151;
pub const ERR50: C2RustUnnamed_3 = 150;
pub const ERR49: C2RustUnnamed_3 = 149;
pub const ERR48: C2RustUnnamed_3 = 148;
pub const ERR47: C2RustUnnamed_3 = 147;
pub const ERR46: C2RustUnnamed_3 = 146;
pub const ERR45: C2RustUnnamed_3 = 145;
pub const ERR44: C2RustUnnamed_3 = 144;
pub const ERR43: C2RustUnnamed_3 = 143;
pub const ERR42: C2RustUnnamed_3 = 142;
pub const ERR41: C2RustUnnamed_3 = 141;
pub const ERR40: C2RustUnnamed_3 = 140;
pub const ERR39: C2RustUnnamed_3 = 139;
pub const ERR38: C2RustUnnamed_3 = 138;
pub const ERR37: C2RustUnnamed_3 = 137;
pub const ERR36: C2RustUnnamed_3 = 136;
pub const ERR35: C2RustUnnamed_3 = 135;
pub const ERR34: C2RustUnnamed_3 = 134;
pub const ERR33: C2RustUnnamed_3 = 133;
pub const ERR32: C2RustUnnamed_3 = 132;
pub const ERR31: C2RustUnnamed_3 = 131;
pub const ERR30: C2RustUnnamed_3 = 130;
pub const ERR29: C2RustUnnamed_3 = 129;
pub const ERR28: C2RustUnnamed_3 = 128;
pub const ERR27: C2RustUnnamed_3 = 127;
pub const ERR26: C2RustUnnamed_3 = 126;
pub const ERR25: C2RustUnnamed_3 = 125;
pub const ERR24: C2RustUnnamed_3 = 124;
pub const ERR23: C2RustUnnamed_3 = 123;
pub const ERR22: C2RustUnnamed_3 = 122;
pub const ERR21: C2RustUnnamed_3 = 121;
pub const ERR20: C2RustUnnamed_3 = 120;
pub const ERR19: C2RustUnnamed_3 = 119;
pub const ERR18: C2RustUnnamed_3 = 118;
pub const ERR17: C2RustUnnamed_3 = 117;
pub const ERR16: C2RustUnnamed_3 = 116;
pub const ERR15: C2RustUnnamed_3 = 115;
pub const ERR14: C2RustUnnamed_3 = 114;
pub const ERR13: C2RustUnnamed_3 = 113;
pub const ERR12: C2RustUnnamed_3 = 112;
pub const ERR11: C2RustUnnamed_3 = 111;
pub const ERR10: C2RustUnnamed_3 = 110;
pub const ERR9: C2RustUnnamed_3 = 109;
pub const ERR8: C2RustUnnamed_3 = 108;
pub const ERR7: C2RustUnnamed_3 = 107;
pub const ERR6: C2RustUnnamed_3 = 106;
pub const ERR5: C2RustUnnamed_3 = 105;
pub const ERR4: C2RustUnnamed_3 = 104;
pub const ERR3: C2RustUnnamed_3 = 103;
pub const ERR2: C2RustUnnamed_3 = 102;
pub const ERR1: C2RustUnnamed_3 = 101;
pub const ERR0: C2RustUnnamed_3 = 100;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eclass_op_info {
    pub code_start: *mut PCRE2_UCHAR8,
    pub length: size_t,
    pub op_single_type: uint8_t,
    pub bits: class_bits_storage,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eclass_context {
    pub options: uint32_t,
    pub xoptions: uint32_t,
    pub errorcodeptr: *mut ::core::ffi::c_int,
    pub cb: *mut compile_block_8,
    pub needs_bitmap: BOOL,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SIZE_MAX: usize = usize::MAX;
pub const PCRE2_CASELESS: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PCRE2_UCP: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PCRE2_UTF: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PCRE2_EXTRA_CASELESS_RESTRICT: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PCRE2_EXTRA_ASCII_POSIX: ::core::ffi::c_uint = 0x800 as ::core::ffi::c_uint;
pub const PCRE2_EXTRA_TURKISH_CASING: ::core::ffi::c_uint = 0x10000 as ::core::ffi::c_uint;
pub const PCRE2_SIZE_MAX: usize = SIZE_MAX;
pub const NOTACHAR: ::core::ffi::c_uint = 0xffffffff as ::core::ffi::c_uint;
pub const MAX_UTF_CODE_POINT: ::core::ffi::c_int = 0x10ffff as ::core::ffi::c_int;
pub const PCRE2_HASCRORLF: ::core::ffi::c_uint = 0x800 as ::core::ffi::c_uint;
pub const cbit_space: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const cbit_digit: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const cbit_word: ::core::ffi::c_int = 160 as ::core::ffi::c_int;
pub const CHAR_HT: ::core::ffi::c_int = 9;
pub const CHAR_VT: ::core::ffi::c_int = 11;
pub const CHAR_FF: ::core::ffi::c_int = 12;
pub const CHAR_CR: ::core::ffi::c_int = 13;
pub const CHAR_LF: ::core::ffi::c_int = 10;
pub const CHAR_NL: ::core::ffi::c_int = CHAR_LF;
pub const CHAR_NEL: ::core::ffi::c_int = 133 as ::core::ffi::c_int;
pub const CHAR_SPACE: ::core::ffi::c_int = 32;
pub const CHAR_DOLLAR_SIGN: ::core::ffi::c_int = '$' as i32;
pub const CHAR_0: ::core::ffi::c_int = '0' as i32;
pub const CHAR_9: ::core::ffi::c_int = '9' as i32;
pub const CHAR_COMMERCIAL_AT: ::core::ffi::c_int = '@' as i32;
pub const CHAR_A: ::core::ffi::c_int = 'A' as i32;
pub const CHAR_F: ::core::ffi::c_int = 'F' as i32;
pub const CHAR_GRAVE_ACCENT: ::core::ffi::c_int = '`' as i32;
pub const CHAR_a: ::core::ffi::c_int = 'a' as i32;
pub const CHAR_f: ::core::ffi::c_int = 'f' as i32;
pub const CHAR_NBSP: ::core::ffi::c_int = 160 as ::core::ffi::c_int;
pub const PT_LAMP: uint32_t = 0 as uint32_t;
pub const PT_GC: uint32_t = 1 as uint32_t;
pub const PT_PC: uint32_t = 2 as uint32_t;
pub const PT_SC: uint32_t = 3 as uint32_t;
pub const PT_SCX: uint32_t = 4 as uint32_t;
pub const PT_ALNUM: uint32_t = 5 as uint32_t;
pub const PT_SPACE: uint32_t = 6 as uint32_t;
pub const PT_PXSPACE: uint32_t = 7 as uint32_t;
pub const PT_WORD: uint32_t = 8 as uint32_t;
pub const PT_UCNC: uint32_t = 10 as uint32_t;
pub const PT_BIDICL: uint32_t = 11 as uint32_t;
pub const PT_BOOL: uint32_t = 12 as uint32_t;
pub const PT_ANY: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const PT_PXGRAPH: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const PT_PXPRINT: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const PT_PXPUNCT: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const XCL_NOT: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const XCL_MAP: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const XCL_HASPROP: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const XCL_END: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const XCL_SINGLE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const XCL_RANGE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const XCL_PROP: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const XCL_NOTPROP: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_LOW_16_START: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_HIGH_16_START: ::core::ffi::c_int = 0x8000 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_LOW_32_START: ::core::ffi::c_int = 0x10000 as ::core::ffi::c_int;
pub const XCL_CHAR_LIST_LOW_32_END: ::core::ffi::c_int = 0x7fffffff as ::core::ffi::c_int;
pub const XCL_TYPE_BIT_LEN: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const XCL_BEGIN_WITH_RANGE: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const XCL_ITEM_COUNT_MASK: ::core::ffi::c_int = 0x3 as ::core::ffi::c_int;
pub const XCL_CHAR_END: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const XCL_CHAR_SHIFT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ECL_MAP: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const ECL_AND: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ECL_OR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ECL_XOR: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const ECL_NOT: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ECL_XCLASS: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const ECL_ANY: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const ECL_NONE: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const UCD_BIDICLASS_SHIFT: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const MAX_PATTERN_SIZE: ::core::ffi::c_int =
    (1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int;
pub const META_END: ::core::ffi::c_uint = 0x80000000 as ::core::ffi::c_uint;
pub const META_BIGVALUE: ::core::ffi::c_uint = 0x80050000 as ::core::ffi::c_uint;
pub const META_CLASS: ::core::ffi::c_uint = 0x800a0000 as ::core::ffi::c_uint;
pub const META_CLASS_EMPTY: ::core::ffi::c_uint = 0x800b0000 as ::core::ffi::c_uint;
pub const META_CLASS_EMPTY_NOT: ::core::ffi::c_uint = 2148270080;
pub const META_CLASS_END: ::core::ffi::c_uint = 0x800d0000 as ::core::ffi::c_uint;
pub const META_CLASS_NOT: ::core::ffi::c_uint = 0x800e0000 as ::core::ffi::c_uint;
pub const META_ESCAPE: ::core::ffi::c_uint = 2149318656;
pub const META_POSIX: ::core::ffi::c_uint = 2149580800;
pub const META_POSIX_NEG: ::core::ffi::c_uint = 0x80210000 as ::core::ffi::c_uint;
pub const META_RANGE_ESCAPED: ::core::ffi::c_uint = 0x80220000 as ::core::ffi::c_uint;
pub const META_RANGE_LITERAL: ::core::ffi::c_uint = 0x80230000 as ::core::ffi::c_uint;
pub const META_ECLASS_AND: ::core::ffi::c_uint = 0x80440000 as ::core::ffi::c_uint;
pub const META_ECLASS_OR: ::core::ffi::c_uint = 0x80450000 as ::core::ffi::c_uint;
pub const META_ECLASS_SUB: ::core::ffi::c_uint = 0x80460000 as ::core::ffi::c_uint;
pub const META_ECLASS_XOR: ::core::ffi::c_uint = 0x80470000 as ::core::ffi::c_uint;
pub const META_ECLASS_NOT: ::core::ffi::c_uint = 0x80480000 as ::core::ffi::c_uint;
pub const CLASS_IS_ECLASS: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const MAX_UCHAR_VALUE: ::core::ffi::c_uint = 0xff as ::core::ffi::c_uint;
pub const PC_GRAPH: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const PC_PRINT: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const PC_PUNCT: ::core::ffi::c_int = 10;
unsafe extern "C" fn do_heapify(mut buffer: *mut uint32_t, mut size: size_t, mut i: size_t) {
    let mut max: size_t = 0;
    let mut left: size_t = 0;
    let mut right: size_t = 0;
    let mut tmp1: uint32_t = 0;
    let mut tmp2: uint32_t = 0;
    loop {
        max = i;
        left = (i << 1 as ::core::ffi::c_int).wrapping_add(2 as size_t);
        right = left.wrapping_add(2 as size_t);
        if left < size && *buffer.offset(left as isize) > *buffer.offset(max as isize) {
            max = left;
        }
        if right < size && *buffer.offset(right as isize) > *buffer.offset(max as isize) {
            max = right;
        }
        if i == max {
            return;
        }
        tmp1 = *buffer.offset(i as isize);
        tmp2 = *buffer.offset(i.wrapping_add(1 as size_t) as isize);
        *buffer.offset(i as isize) = *buffer.offset(max as isize);
        *buffer.offset(i.wrapping_add(1 as size_t) as isize) =
            *buffer.offset(max.wrapping_add(1 as size_t) as isize);
        *buffer.offset(max as isize) = tmp1;
        *buffer.offset(max.wrapping_add(1 as size_t) as isize) = tmp2;
        i = max;
    }
}
pub const PARSE_CLASS_UTF: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const PARSE_CLASS_CASELESS_UTF: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const PARSE_CLASS_RESTRICTED_UTF: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const PARSE_CLASS_TURKISH_UTF: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
unsafe extern "C" fn get_nocase_range(mut c: uint32_t) -> *const uint32_t {
    let mut left: uint32_t = 0 as uint32_t;
    let mut right: uint32_t = _pcre2_ucd_nocase_ranges_size_8;
    let mut middle: uint32_t = 0;
    if c > MAX_UTF_CODE_POINT as uint32_t {
        return (&raw const _pcre2_ucd_nocase_ranges_8 as *const uint32_t).offset(right as isize);
    }
    loop {
        middle = left.wrapping_add(right) >> 1 as ::core::ffi::c_int | 0x1 as uint32_t;
        if *(&raw const _pcre2_ucd_nocase_ranges_8 as *const uint32_t).offset(middle as isize) <= c
        {
            left = middle.wrapping_add(1 as uint32_t);
        } else if middle > 1 as uint32_t
            && *(&raw const _pcre2_ucd_nocase_ranges_8 as *const uint32_t)
                .offset(middle.wrapping_sub(2 as uint32_t) as isize)
                > c
        {
            right = middle.wrapping_sub(1 as uint32_t);
        } else {
            return (&raw const _pcre2_ucd_nocase_ranges_8 as *const uint32_t)
                .offset(middle.wrapping_sub(1 as uint32_t) as isize);
        }
    }
}
unsafe extern "C" fn utf_caseless_extend(
    mut start: uint32_t,
    mut end: uint32_t,
    mut options: uint32_t,
    mut buffer: *mut uint32_t,
) -> size_t {
    let mut new_start: uint32_t = start;
    let mut new_end: uint32_t = end;
    let mut c: uint32_t = start;
    let mut list: *const uint32_t = ::core::ptr::null::<uint32_t>();
    let mut tmp: [uint32_t; 3] = [0; 3];
    let mut result: size_t = 2 as size_t;
    let mut skip_range: *const uint32_t = get_nocase_range(c);
    let mut skip_start: uint32_t = *skip_range.offset(0 as ::core::ffi::c_int as isize);
    while c <= end {
        let mut co: uint32_t = 0;
        if c > skip_start {
            c = *skip_range.offset(1 as ::core::ffi::c_int as isize);
            skip_range = skip_range.offset(2 as ::core::ffi::c_int as isize);
            skip_start = *skip_range.offset(0 as ::core::ffi::c_int as isize);
        } else {
            if options & (PARSE_CLASS_TURKISH_UTF | PARSE_CLASS_RESTRICTED_UTF) as uint32_t
                == PARSE_CLASS_TURKISH_UTF as uint32_t
                && (c | 0x20 as uint32_t == 0x69 as uint32_t
                    || c | 1 as uint32_t == 0x131 as uint32_t)
            {
                co = _pcre2_ucd_turkish_dotted_i_caseset_8.wrapping_add(
                    (if c == 0x69 as uint32_t || c == 0x130 as uint32_t {
                        0 as ::core::ffi::c_int
                    } else {
                        3 as ::core::ffi::c_int
                    }) as uint32_t,
                );
            } else {
                co = (*(&raw const _pcre2_ucd_records_8 as *const ucd_record).offset(
                    *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t).offset(
                        (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                            .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                            as ::core::ffi::c_int
                            * UCD_BLOCK_SIZE
                            + c as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                            as isize,
                    ) as ::core::ffi::c_int as isize,
                ))
                .caseset as uint32_t;
                if co != 0 as uint32_t
                    && options & PARSE_CLASS_RESTRICTED_UTF as uint32_t != 0 as uint32_t
                    && *(&raw const _pcre2_ucd_caseless_sets_8 as *const uint32_t)
                        .offset(co as isize)
                        < 128 as uint32_t
                {
                    co = 0 as uint32_t;
                }
            }
            if co != 0 as uint32_t {
                list =
                    (&raw const _pcre2_ucd_caseless_sets_8 as *const uint32_t).offset(co as isize);
            } else {
                co = (c as ::core::ffi::c_int
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
                list = &raw mut tmp as *mut uint32_t;
                tmp[0 as ::core::ffi::c_int as usize] = c;
                tmp[1 as ::core::ffi::c_int as usize] = NOTACHAR as uint32_t;
                if co != c {
                    tmp[1 as ::core::ffi::c_int as usize] = co;
                    tmp[2 as ::core::ffi::c_int as usize] = NOTACHAR as uint32_t;
                }
            }
            c = c.wrapping_add(1);
            let mut current_block_30: u64;
            loop {
                if *list < new_start {
                    if (*list).wrapping_add(1 as uint32_t) == new_start {
                        new_start = new_start.wrapping_sub(1);
                        current_block_30 = 11307063007268554308;
                    } else {
                        current_block_30 = 2569451025026770673;
                    }
                } else if *list > new_end {
                    if (*list).wrapping_sub(1 as uint32_t) == new_end {
                        new_end = new_end.wrapping_add(1);
                        current_block_30 = 11307063007268554308;
                    } else {
                        current_block_30 = 2569451025026770673;
                    }
                } else {
                    current_block_30 = 11307063007268554308;
                }
                match current_block_30 {
                    2569451025026770673 => {
                        result = result.wrapping_add(2 as size_t);
                        if !buffer.is_null() {
                            *buffer.offset(0 as ::core::ffi::c_int as isize) = *list;
                            *buffer.offset(1 as ::core::ffi::c_int as isize) = *list;
                            buffer = buffer.offset(2 as ::core::ffi::c_int as isize);
                        }
                    }
                    _ => {}
                }
                list = list.offset(1);
                if !(*list != NOTACHAR as uint32_t) {
                    break;
                }
            }
        }
    }
    if !buffer.is_null() {
        *buffer.offset(0 as ::core::ffi::c_int as isize) = new_start;
        *buffer.offset(1 as ::core::ffi::c_int as isize) = new_end;
        buffer = buffer.offset(2 as ::core::ffi::c_int as isize);
    }
    return result;
}
unsafe extern "C" fn append_char_list(mut p: *const uint32_t, mut buffer: *mut uint32_t) -> size_t {
    let mut n: *const uint32_t = ::core::ptr::null::<uint32_t>();
    let mut result: size_t = 0 as size_t;
    while *p != NOTACHAR as uint32_t {
        n = p;
        while *n.offset(0 as ::core::ffi::c_int as isize)
            == (*n.offset(1 as ::core::ffi::c_int as isize)).wrapping_sub(1 as uint32_t)
        {
            n = n.offset(1);
        }
        if !buffer.is_null() {
            *buffer.offset(0 as ::core::ffi::c_int as isize) = *p;
            *buffer.offset(1 as ::core::ffi::c_int as isize) = *n;
            buffer = buffer.offset(2 as ::core::ffi::c_int as isize);
        }
        result = result.wrapping_add(2 as size_t);
        p = n.offset(1 as ::core::ffi::c_int as isize);
    }
    return result;
}
unsafe extern "C" fn get_highest_char(mut options: uint32_t) -> uint32_t {
    return MAX_UTF_CODE_POINT as uint32_t;
}
unsafe extern "C" fn append_negated_char_list(
    mut p: *const uint32_t,
    mut options: uint32_t,
    mut buffer: *mut uint32_t,
) -> size_t {
    let mut n: *const uint32_t = ::core::ptr::null::<uint32_t>();
    let mut start: uint32_t = 0 as uint32_t;
    let mut result: size_t = 2 as size_t;
    while *p != NOTACHAR as uint32_t {
        n = p;
        while *n.offset(0 as ::core::ffi::c_int as isize)
            == (*n.offset(1 as ::core::ffi::c_int as isize)).wrapping_sub(1 as uint32_t)
        {
            n = n.offset(1);
        }
        if !buffer.is_null() {
            *buffer.offset(0 as ::core::ffi::c_int as isize) = start;
            *buffer.offset(1 as ::core::ffi::c_int as isize) = (*p).wrapping_sub(1 as uint32_t);
            buffer = buffer.offset(2 as ::core::ffi::c_int as isize);
        }
        result = result.wrapping_add(2 as size_t);
        start = (*n).wrapping_add(1 as uint32_t);
        p = n.offset(1 as ::core::ffi::c_int as isize);
    }
    if !buffer.is_null() {
        *buffer.offset(0 as ::core::ffi::c_int as isize) = start;
        *buffer.offset(1 as ::core::ffi::c_int as isize) = get_highest_char(options);
        buffer = buffer.offset(2 as ::core::ffi::c_int as isize);
    }
    return result;
}
unsafe extern "C" fn append_non_ascii_range(
    mut options: uint32_t,
    mut buffer: *mut uint32_t,
) -> *mut uint32_t {
    if buffer.is_null() {
        return ::core::ptr::null_mut::<uint32_t>();
    }
    *buffer.offset(0 as ::core::ffi::c_int as isize) = 0x100 as uint32_t;
    *buffer.offset(1 as ::core::ffi::c_int as isize) = get_highest_char(options);
    return buffer.offset(2 as ::core::ffi::c_int as isize);
}
unsafe extern "C" fn parse_class(
    mut ptr: *mut uint32_t,
    mut options: uint32_t,
    mut buffer: *mut uint32_t,
) -> size_t {
    let mut total_size: size_t = 0 as size_t;
    let mut size: size_t = 0;
    let mut meta_arg: uint32_t = 0;
    let mut start_char: uint32_t = 0;
    loop {
        match *ptr & 0xffff0000 as uint32_t {
            META_ESCAPE => {
                meta_arg = *ptr & 0xffff as uint32_t;
                match meta_arg {
                    6 | 10 | 8 => {
                        buffer = append_non_ascii_range(options, buffer);
                        total_size = total_size.wrapping_add(2 as size_t);
                    }
                    19 => {
                        size = append_char_list(
                            &raw const _pcre2_hspace_list_8 as *const uint32_t,
                            buffer,
                        );
                        total_size = total_size.wrapping_add(size);
                        if !buffer.is_null() {
                            buffer = buffer.offset(size as isize);
                        }
                    }
                    18 => {
                        size = append_negated_char_list(
                            &raw const _pcre2_hspace_list_8 as *const uint32_t,
                            options,
                            buffer,
                        );
                        total_size = total_size.wrapping_add(size);
                        if !buffer.is_null() {
                            buffer = buffer.offset(size as isize);
                        }
                    }
                    21 => {
                        size = append_char_list(
                            &raw const _pcre2_vspace_list_8 as *const uint32_t,
                            buffer,
                        );
                        total_size = total_size.wrapping_add(size);
                        if !buffer.is_null() {
                            buffer = buffer.offset(size as isize);
                        }
                    }
                    20 => {
                        size = append_negated_char_list(
                            &raw const _pcre2_vspace_list_8 as *const uint32_t,
                            options,
                            buffer,
                        );
                        total_size = total_size.wrapping_add(size);
                        if !buffer.is_null() {
                            buffer = buffer.offset(size as isize);
                        }
                    }
                    16 | 15 => {
                        ptr = ptr.offset(1);
                        if meta_arg == ESC_p as ::core::ffi::c_int as uint32_t
                            && *ptr >> 16 as ::core::ffi::c_int == PT_ANY as uint32_t
                        {
                            if !buffer.is_null() {
                                *buffer.offset(0 as ::core::ffi::c_int as isize) = 0 as uint32_t;
                                *buffer.offset(1 as ::core::ffi::c_int as isize) =
                                    get_highest_char(options);
                                buffer = buffer.offset(2 as ::core::ffi::c_int as isize);
                            }
                            total_size = total_size.wrapping_add(2 as size_t);
                        }
                    }
                    _ => {}
                }
                ptr = ptr.offset(1);
                continue;
            }
            META_POSIX_NEG => {
                buffer = append_non_ascii_range(options, buffer);
                total_size = total_size.wrapping_add(2 as size_t);
                ptr = ptr.offset(2 as ::core::ffi::c_int as isize);
                continue;
            }
            META_POSIX => {
                ptr = ptr.offset(2 as ::core::ffi::c_int as isize);
                continue;
            }
            META_BIGVALUE => {
                ptr = ptr.offset(1);
            }
            _ => {
                if *ptr >= META_END as uint32_t {
                    return total_size;
                }
            }
        }
        start_char = *ptr;
        if *ptr.offset(1 as ::core::ffi::c_int as isize) == META_RANGE_LITERAL as uint32_t
            || *ptr.offset(1 as ::core::ffi::c_int as isize) == META_RANGE_ESCAPED as uint32_t
        {
            ptr = ptr.offset(2 as ::core::ffi::c_int as isize);
            if *ptr == META_BIGVALUE as uint32_t {
                ptr = ptr.offset(1);
            }
        }
        if options & PARSE_CLASS_CASELESS_UTF as uint32_t != 0 {
            let fresh33 = ptr;
            ptr = ptr.offset(1);
            size = utf_caseless_extend(start_char, *fresh33, options, buffer);
            if !buffer.is_null() {
                buffer = buffer.offset(size as isize);
            }
            total_size = total_size.wrapping_add(size);
        } else {
            if !buffer.is_null() {
                *buffer.offset(0 as ::core::ffi::c_int as isize) = start_char;
                *buffer.offset(1 as ::core::ffi::c_int as isize) = *ptr;
                buffer = buffer.offset(2 as ::core::ffi::c_int as isize);
            }
            ptr = ptr.offset(1);
            total_size = total_size.wrapping_add(2 as size_t);
        }
    }
}
pub const CHAR_LIST_EXTRA_SIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
static mut char_list_starts: [uint32_t; 3] = [
    XCL_CHAR_LIST_LOW_32_START as uint32_t,
    XCL_CHAR_LIST_HIGH_16_START as uint32_t,
    XCL_CHAR_LIST_LOW_16_START as uint32_t,
];
unsafe extern "C" fn compile_optimize_class(
    mut start_ptr: *mut uint32_t,
    mut options: uint32_t,
    mut xoptions: uint32_t,
    mut errorcodeptr: *mut ::core::ffi::c_int,
    mut cb: *mut compile_block_8,
) -> *mut class_ranges {
    let mut cranges: *mut class_ranges = ::core::ptr::null_mut::<class_ranges>();
    let mut ptr: *mut uint32_t = ::core::ptr::null_mut::<uint32_t>();
    let mut buffer: *mut uint32_t = ::core::ptr::null_mut::<uint32_t>();
    let mut dst: *mut uint32_t = ::core::ptr::null_mut::<uint32_t>();
    let mut class_options: uint32_t = 0 as uint32_t;
    let mut range_list_size: size_t = 0 as size_t;
    let mut total_size: size_t = 0;
    let mut i: size_t = 0;
    let mut tmp1: uint32_t = 0;
    let mut tmp2: uint32_t = 0;
    let mut char_list_next: *const uint32_t = ::core::ptr::null::<uint32_t>();
    let mut next_char: *mut uint16_t = ::core::ptr::null_mut::<uint16_t>();
    let mut char_list_start: uint32_t = 0;
    let mut char_list_end: uint32_t = 0;
    let mut range_start: uint32_t = 0;
    let mut range_end: uint32_t = 0;
    if options & PCRE2_UTF as uint32_t != 0 {
        class_options |= PARSE_CLASS_UTF as uint32_t;
    }
    if options & PCRE2_CASELESS as uint32_t != 0
        && options & (PCRE2_UTF as uint32_t | PCRE2_UCP as uint32_t) != 0
    {
        class_options |= PARSE_CLASS_CASELESS_UTF as uint32_t;
    }
    if xoptions & PCRE2_EXTRA_CASELESS_RESTRICT as uint32_t != 0 {
        class_options |= PARSE_CLASS_RESTRICTED_UTF as uint32_t;
    }
    if xoptions & PCRE2_EXTRA_TURKISH_CASING as uint32_t != 0 {
        class_options |= PARSE_CLASS_TURKISH_UTF as uint32_t;
    }
    range_list_size = parse_class(
        start_ptr,
        class_options,
        ::core::ptr::null_mut::<uint32_t>(),
    );
    total_size = range_list_size.wrapping_add(
        (if range_list_size >= 2 as size_t {
            CHAR_LIST_EXTRA_SIZE
        } else {
            0 as ::core::ffi::c_int
        }) as size_t,
    );
    if total_size
        > (PCRE2_SIZE_MAX as usize)
            .wrapping_sub(::core::mem::size_of::<class_ranges>() as usize)
            .wrapping_div(::core::mem::size_of::<uint32_t>() as usize)
    {
        *errorcodeptr = ERR20 as ::core::ffi::c_int;
        (*cb).erroroffset = 0 as size_t;
        return ::core::ptr::null_mut::<class_ranges>();
    }
    cranges = (*(*cb).cx)
        .memctl
        .malloc
        .expect("non-null function pointer")(
        (::core::mem::size_of::<class_ranges>() as size_t)
            .wrapping_add(total_size.wrapping_mul(::core::mem::size_of::<uint32_t>() as size_t)),
        (*(*cb).cx).memctl.memory_data,
    ) as *mut class_ranges;
    if cranges.is_null() {
        *errorcodeptr = ERR21 as ::core::ffi::c_int;
        (*cb).erroroffset = 0 as size_t;
        return ::core::ptr::null_mut::<class_ranges>();
    }
    (*cranges).header.next = ::core::ptr::null_mut::<compile_data>();
    (*cranges).range_list_size = range_list_size as uint16_t;
    (*cranges).char_lists_types = 0 as uint16_t;
    (*cranges).char_lists_size = 0 as size_t;
    (*cranges).char_lists_start = 0 as size_t;
    if range_list_size == 0 as size_t {
        return cranges;
    }
    buffer = cranges.offset(1 as ::core::ffi::c_int as isize) as *mut uint32_t;
    parse_class(start_ptr, class_options, buffer);
    if range_list_size <= 2 as size_t {
        return cranges;
    }
    i = (range_list_size >> 2 as ::core::ffi::c_int).wrapping_sub(1 as size_t)
        << 1 as ::core::ffi::c_int;
    loop {
        do_heapify(buffer, range_list_size, i);
        if i == 0 as size_t {
            break;
        }
        i = i.wrapping_sub(2 as size_t);
    }
    i = range_list_size.wrapping_sub(2 as size_t);
    loop {
        tmp1 = *buffer.offset(i as isize);
        tmp2 = *buffer.offset(i.wrapping_add(1 as size_t) as isize);
        *buffer.offset(i as isize) = *buffer.offset(0 as ::core::ffi::c_int as isize);
        *buffer.offset(i.wrapping_add(1 as size_t) as isize) =
            *buffer.offset(1 as ::core::ffi::c_int as isize);
        *buffer.offset(0 as ::core::ffi::c_int as isize) = tmp1;
        *buffer.offset(1 as ::core::ffi::c_int as isize) = tmp2;
        do_heapify(buffer, i, 0 as size_t);
        if i == 0 as size_t {
            break;
        }
        i = i.wrapping_sub(2 as size_t);
    }
    dst = buffer;
    ptr = buffer.offset(2 as ::core::ffi::c_int as isize);
    range_list_size = range_list_size.wrapping_sub(2 as size_t);
    while range_list_size > 0 as size_t
        && *dst.offset(1 as ::core::ffi::c_int as isize) != !(0 as ::core::ffi::c_int as uint32_t)
    {
        if (*dst.offset(1 as ::core::ffi::c_int as isize)).wrapping_add(1 as uint32_t)
            < *ptr.offset(0 as ::core::ffi::c_int as isize)
        {
            dst = dst.offset(2 as ::core::ffi::c_int as isize);
            *dst.offset(0 as ::core::ffi::c_int as isize) =
                *ptr.offset(0 as ::core::ffi::c_int as isize);
            *dst.offset(1 as ::core::ffi::c_int as isize) =
                *ptr.offset(1 as ::core::ffi::c_int as isize);
        } else if *dst.offset(1 as ::core::ffi::c_int as isize)
            < *ptr.offset(1 as ::core::ffi::c_int as isize)
        {
            *dst.offset(1 as ::core::ffi::c_int as isize) =
                *ptr.offset(1 as ::core::ffi::c_int as isize);
        }
        ptr = ptr.offset(2 as ::core::ffi::c_int as isize);
        range_list_size = range_list_size.wrapping_sub(2 as size_t);
    }
    ptr = buffer;
    while ptr < dst && *ptr.offset(1 as ::core::ffi::c_int as isize) < 0x100 as uint32_t {
        ptr = ptr.offset(2 as ::core::ffi::c_int as isize);
    }
    if (dst.offset_from(ptr) as isize)
        < (2 as ::core::ffi::c_int * (6 as ::core::ffi::c_int - 1 as ::core::ffi::c_int))
            as isize
    {
        (*cranges).range_list_size =
            dst.offset(2 as ::core::ffi::c_int as isize)
                .offset_from(buffer) as isize as uint16_t;
        return cranges;
    }
    char_list_next = &raw const char_list_starts as *const uint32_t;
    let fresh31 = char_list_next;
    char_list_next = char_list_next.offset(1);
    char_list_start = *fresh31;
    char_list_end = XCL_CHAR_LIST_LOW_32_END as uint32_t;
    next_char = buffer.offset(total_size as isize) as *mut uint16_t;
    tmp1 = 0 as uint32_t;
    tmp2 = (::core::mem::size_of::<[uint32_t; 3]>() as usize)
        .wrapping_div(::core::mem::size_of::<uint32_t>() as usize)
        .wrapping_sub(1 as usize)
        .wrapping_mul(XCL_TYPE_BIT_LEN as usize) as uint32_t;
    range_start = *dst.offset(0 as ::core::ffi::c_int as isize);
    range_end = *dst.offset(1 as ::core::ffi::c_int as isize);
    loop {
        if range_start >= char_list_start {
            if range_start == range_end || range_end < char_list_end {
                tmp1 = tmp1.wrapping_add(1);
                next_char = next_char.offset(-1);
                if char_list_start < XCL_CHAR_LIST_LOW_32_START as uint32_t {
                    *next_char =
                        (range_end << XCL_CHAR_SHIFT | XCL_CHAR_END as uint32_t) as uint16_t;
                } else {
                    next_char = next_char.offset(-1);
                    *(next_char as *mut uint32_t) =
                        range_end << XCL_CHAR_SHIFT | XCL_CHAR_END as uint32_t;
                }
            }
            if range_start < range_end {
                if range_start > char_list_start {
                    tmp1 = tmp1.wrapping_add(1);
                    next_char = next_char.offset(-1);
                    if char_list_start < XCL_CHAR_LIST_LOW_32_START as uint32_t {
                        *next_char = (range_start << XCL_CHAR_SHIFT) as uint16_t;
                    } else {
                        next_char = next_char.offset(-1);
                        *(next_char as *mut uint32_t) = range_start << XCL_CHAR_SHIFT;
                    }
                } else {
                    (*cranges).char_lists_types =
                        ((*cranges).char_lists_types as ::core::ffi::c_int
                            | XCL_BEGIN_WITH_RANGE << tmp2) as uint16_t;
                }
            }
            if dst > buffer {
                dst = dst.offset(-(2 as ::core::ffi::c_int as isize));
                range_start = *dst.offset(0 as ::core::ffi::c_int as isize);
                range_end = *dst.offset(1 as ::core::ffi::c_int as isize);
                continue;
            } else {
                range_start = 0 as uint32_t;
                range_end = 0 as uint32_t;
            }
        }
        if range_end >= char_list_start {
            if range_end < char_list_end {
                tmp1 = tmp1.wrapping_add(1);
                next_char = next_char.offset(-1);
                if char_list_start < XCL_CHAR_LIST_LOW_32_START as uint32_t {
                    *next_char =
                        (range_end << XCL_CHAR_SHIFT | XCL_CHAR_END as uint32_t) as uint16_t;
                } else {
                    next_char = next_char.offset(-1);
                    *(next_char as *mut uint32_t) =
                        range_end << XCL_CHAR_SHIFT | XCL_CHAR_END as uint32_t;
                }
            }
            (*cranges).char_lists_types = ((*cranges).char_lists_types as ::core::ffi::c_int
                | XCL_BEGIN_WITH_RANGE << tmp2)
                as uint16_t;
        }
        if tmp1 >= XCL_ITEM_COUNT_MASK as uint32_t {
            (*cranges).char_lists_types = ((*cranges).char_lists_types as ::core::ffi::c_int
                | XCL_ITEM_COUNT_MASK << tmp2)
                as uint16_t;
            next_char = next_char.offset(-1);
            if char_list_start < XCL_CHAR_LIST_LOW_32_START as uint32_t {
                *next_char = tmp1 as uint16_t;
            } else {
                next_char = next_char.offset(-1);
                *(next_char as *mut uint32_t) = tmp1;
            }
        } else {
            (*cranges).char_lists_types =
                ((*cranges).char_lists_types as uint32_t | tmp1 << tmp2) as uint16_t;
        }
        if range_end < XCL_CHAR_LIST_LOW_16_START as uint32_t || tmp2 == 0 as uint32_t {
            break;
        }
        char_list_end = char_list_start.wrapping_sub(1 as uint32_t);
        let fresh32 = char_list_next;
        char_list_next = char_list_next.offset(1);
        char_list_start = *fresh32;
        tmp1 = 0 as uint32_t;
        tmp2 = tmp2.wrapping_sub(XCL_TYPE_BIT_LEN as uint32_t);
    }
    if *dst.offset(0 as ::core::ffi::c_int as isize) < XCL_CHAR_LIST_LOW_16_START as uint32_t {
        dst = dst.offset(2 as ::core::ffi::c_int as isize);
    }
    (*cranges).char_lists_size = (buffer.offset(total_size as isize) as *mut uint8_t)
        .offset_from(next_char as *mut uint8_t)
        as isize as size_t;
    (*cranges).char_lists_start = (next_char as *mut uint8_t).offset_from(buffer as *mut uint8_t)
        as isize as size_t;
    (*cranges).range_list_size = dst.offset_from(buffer) as isize as uint16_t;
    return cranges;
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_update_classbits_8(
    mut ptype: uint32_t,
    mut pdata: uint32_t,
    mut negated: BOOL,
    mut classbits: *mut uint8_t,
) {
    let mut c: ::core::ffi::c_int = 0;
    let mut chartype: ::core::ffi::c_int = 0;
    let mut prop: *const ucd_record = ::core::ptr::null::<ucd_record>();
    let mut gentype: uint32_t = 0;
    let mut set_bit: BOOL = 0;
    if ptype == PT_ANY as uint32_t {
        if negated == 0 {
            memset(
                classbits as *mut ::core::ffi::c_void,
                0xff as ::core::ffi::c_int,
                32 as size_t,
            );
        }
        return;
    }
    c = 0 as ::core::ffi::c_int;
    while c < 256 as ::core::ffi::c_int {
        prop = (&raw const _pcre2_ucd_records_8 as *const ucd_record).offset(
            *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t).offset(
                (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                    .offset((c / UCD_BLOCK_SIZE) as isize) as ::core::ffi::c_int
                    * UCD_BLOCK_SIZE
                    + c % UCD_BLOCK_SIZE) as isize,
            ) as ::core::ffi::c_int as isize,
        );
        set_bit = FALSE as BOOL;
        match ptype {
            0 => {
                chartype = (*prop).chartype as ::core::ffi::c_int;
                set_bit = (chartype == ucp_Lu as ::core::ffi::c_int
                    || chartype == ucp_Ll as ::core::ffi::c_int
                    || chartype == ucp_Lt as ::core::ffi::c_int)
                    as ::core::ffi::c_int as BOOL;
            }
            1 => {
                set_bit = (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                    .offset((*prop).chartype as isize)
                    == pdata) as ::core::ffi::c_int as BOOL;
            }
            2 => {
                set_bit = ((*prop).chartype as uint32_t == pdata) as ::core::ffi::c_int as BOOL;
            }
            3 => {
                set_bit = ((*prop).script as uint32_t == pdata) as ::core::ffi::c_int as BOOL;
            }
            4 => {
                set_bit = ((*prop).script as uint32_t == pdata
                    || *(&raw const _pcre2_ucd_script_sets_8 as *const uint32_t)
                        .offset(
                            ((*prop).scriptx_bidiclass as ::core::ffi::c_int
                                & 0x3ff as ::core::ffi::c_int) as isize,
                        )
                        .offset(pdata.wrapping_div(32 as uint32_t) as isize)
                        & (1 as uint32_t) << pdata.wrapping_rem(32 as uint32_t)
                        != 0 as uint32_t) as ::core::ffi::c_int as BOOL;
            }
            5 => {
                gentype = *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                    .offset((*prop).chartype as isize);
                set_bit = (gentype == ucp_L as ::core::ffi::c_int as uint32_t
                    || gentype == ucp_N as ::core::ffi::c_int as uint32_t)
                    as ::core::ffi::c_int as BOOL;
            }
            6 | 7 => match c {
                CHAR_HT | CHAR_SPACE | 160 | CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | 133 => {
                    set_bit = TRUE as BOOL;
                }
                _ => {
                    set_bit = (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                        .offset((*prop).chartype as isize)
                        == ucp_Z as ::core::ffi::c_int as uint32_t)
                        as ::core::ffi::c_int as BOOL;
                }
            },
            8 => {
                chartype = (*prop).chartype as ::core::ffi::c_int;
                gentype =
                    *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t).offset(chartype as isize);
                set_bit = (gentype == ucp_L as ::core::ffi::c_int as uint32_t
                    || gentype == ucp_N as ::core::ffi::c_int as uint32_t
                    || chartype == ucp_Mn as ::core::ffi::c_int
                    || chartype == ucp_Pc as ::core::ffi::c_int)
                    as ::core::ffi::c_int as BOOL;
            }
            10 => {
                set_bit = (c == CHAR_DOLLAR_SIGN
                    || c == CHAR_COMMERCIAL_AT
                    || c == CHAR_GRAVE_ACCENT
                    || c >= 0xa0 as ::core::ffi::c_int)
                    as ::core::ffi::c_int as BOOL;
            }
            11 => {
                set_bit = (((*prop).scriptx_bidiclass as ::core::ffi::c_int >> UCD_BIDICLASS_SHIFT)
                    as uint32_t
                    == pdata) as ::core::ffi::c_int as BOOL;
            }
            12 => {
                set_bit = (*(&raw const _pcre2_ucd_boolprop_sets_8 as *const uint32_t)
                    .offset(
                        ((*prop).bprops as ::core::ffi::c_int & 0xfff as ::core::ffi::c_int)
                            as isize,
                    )
                    .offset(pdata.wrapping_div(32 as uint32_t) as isize)
                    & (1 as uint32_t) << pdata.wrapping_rem(32 as uint32_t)
                    != 0 as uint32_t) as ::core::ffi::c_int as BOOL;
            }
            14 => {
                chartype = (*prop).chartype as ::core::ffi::c_int;
                gentype =
                    *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t).offset(chartype as isize);
                set_bit = (gentype != ucp_Z as ::core::ffi::c_int as uint32_t
                    && (gentype != ucp_C as ::core::ffi::c_int as uint32_t
                        || chartype == ucp_Cf as ::core::ffi::c_int))
                    as ::core::ffi::c_int as BOOL;
            }
            15 => {
                chartype = (*prop).chartype as ::core::ffi::c_int;
                set_bit = (chartype != ucp_Zl as ::core::ffi::c_int
                    && chartype != ucp_Zp as ::core::ffi::c_int
                    && (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                        .offset(chartype as isize)
                        != ucp_C as ::core::ffi::c_int as uint32_t
                        || chartype == ucp_Cf as ::core::ffi::c_int))
                    as ::core::ffi::c_int as BOOL;
            }
            16 => {
                gentype = *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                    .offset((*prop).chartype as isize);
                set_bit = (gentype == ucp_P as ::core::ffi::c_int as uint32_t
                    || c < 128 as ::core::ffi::c_int
                        && gentype == ucp_S as ::core::ffi::c_int as uint32_t)
                    as ::core::ffi::c_int as BOOL;
            }
            _ => {
                set_bit = (c >= CHAR_0 && c <= CHAR_9
                    || c >= CHAR_A && c <= CHAR_F
                    || c >= CHAR_a && c <= CHAR_f) as ::core::ffi::c_int
                    as BOOL;
            }
        }
        if negated != 0 {
            set_bit = (set_bit == 0) as ::core::ffi::c_int as BOOL;
        }
        if set_bit != 0 {
            *classbits = (*classbits as ::core::ffi::c_int
                | ((1 as ::core::ffi::c_int) << (c & 0x7 as ::core::ffi::c_int)) as uint8_t
                    as ::core::ffi::c_int) as uint8_t;
        }
        if c & 0x7 as ::core::ffi::c_int == 0x7 as ::core::ffi::c_int {
            classbits = classbits.offset(1);
        }
        c += 1;
    }
}
pub const XCLASS_REQUIRED: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const XCLASS_HAS_8BIT_CHARS: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const XCLASS_HAS_PROPS: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const XCLASS_HAS_CHAR_LISTS: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const XCLASS_HIGH_ANY: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
unsafe extern "C" fn add_to_class(
    mut options: uint32_t,
    mut xoptions: uint32_t,
    mut cb: *mut compile_block_8,
    mut start: uint32_t,
    mut end: uint32_t,
) {
    let mut classbits: *mut uint8_t = &raw mut (*cb).classbits.classbits as *mut uint8_t;
    let mut c: uint32_t = 0;
    let mut byte_start: uint32_t = 0;
    let mut byte_end: uint32_t = 0;
    let mut classbits_end: uint32_t = if end <= 0xff as uint32_t {
        end
    } else {
        0xff as uint32_t
    };
    if options & PCRE2_CASELESS as uint32_t != 0 as uint32_t {
        if options & (PCRE2_UTF as uint32_t | PCRE2_UCP as uint32_t) != 0 as uint32_t {
            let mut turkish_i: BOOL = (xoptions
                & (PCRE2_EXTRA_TURKISH_CASING as uint32_t
                    | PCRE2_EXTRA_CASELESS_RESTRICT as uint32_t)
                == PCRE2_EXTRA_TURKISH_CASING as uint32_t)
                as ::core::ffi::c_int;
            if start < 128 as uint32_t {
                let mut lo_end: uint32_t = if classbits_end < 127 as uint32_t {
                    classbits_end
                } else {
                    127 as uint32_t
                };
                c = start;
                while c <= lo_end {
                    if !(turkish_i != 0
                        && (c | 0x20 as uint32_t == 0x69 as uint32_t
                            || c | 1 as uint32_t == 0x131 as uint32_t))
                    {
                        let ref mut fresh25 = *classbits.offset(
                            (*(*cb).fcc.offset(c as isize) as ::core::ffi::c_int
                                >> 3 as ::core::ffi::c_int) as isize,
                        );
                        *fresh25 = (*fresh25 as ::core::ffi::c_int
                            | ((1 as ::core::ffi::c_uint)
                                << (*(*cb).fcc.offset(c as isize) as ::core::ffi::c_int
                                    & 0x7 as ::core::ffi::c_int))
                                as uint8_t as ::core::ffi::c_int)
                            as uint8_t;
                    }
                    c = c.wrapping_add(1);
                }
            }
            if classbits_end >= 128 as uint32_t {
                let mut hi_start: uint32_t = if start > 128 as uint32_t {
                    start
                } else {
                    128 as uint32_t
                };
                c = hi_start;
                while c <= classbits_end {
                    let mut co: uint32_t = (c as ::core::ffi::c_int
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
                        .other_case as ::core::ffi::c_int)
                        as uint32_t;
                    if co <= 0xff as uint32_t {
                        let ref mut fresh26 =
                            *classbits.offset((co >> 3 as ::core::ffi::c_int) as isize);
                        *fresh26 = (*fresh26 as ::core::ffi::c_int
                            | ((1 as ::core::ffi::c_uint) << (co & 0x7 as uint32_t)) as uint8_t
                                as ::core::ffi::c_int)
                            as uint8_t;
                    }
                    c = c.wrapping_add(1);
                }
            }
        } else {
            c = start;
            while c <= classbits_end {
                let ref mut fresh27 = *classbits.offset(
                    (*(*cb).fcc.offset(c as isize) as ::core::ffi::c_int >> 3 as ::core::ffi::c_int)
                        as isize,
                );
                *fresh27 = (*fresh27 as ::core::ffi::c_int
                    | ((1 as ::core::ffi::c_uint)
                        << (*(*cb).fcc.offset(c as isize) as ::core::ffi::c_int
                            & 0x7 as ::core::ffi::c_int)) as uint8_t
                        as ::core::ffi::c_int) as uint8_t;
                c = c.wrapping_add(1);
            }
        }
    }
    byte_start = start.wrapping_add(7 as uint32_t) >> 3 as ::core::ffi::c_int;
    byte_end = classbits_end.wrapping_add(1 as uint32_t) >> 3 as ::core::ffi::c_int;
    if byte_start >= byte_end {
        c = start;
        while c <= classbits_end {
            let ref mut fresh28 = *classbits.offset((c >> 3 as ::core::ffi::c_int) as isize);
            *fresh28 = (*fresh28 as ::core::ffi::c_int
                | ((1 as ::core::ffi::c_uint) << (c & 0x7 as uint32_t)) as uint8_t
                    as ::core::ffi::c_int) as uint8_t;
            c = c.wrapping_add(1);
        }
        return;
    }
    c = byte_start;
    while c < byte_end {
        *classbits.offset(c as isize) = 0xff as uint8_t;
        c = c.wrapping_add(1);
    }
    byte_start <<= 3 as ::core::ffi::c_int;
    byte_end <<= 3 as ::core::ffi::c_int;
    c = start;
    while c < byte_start {
        let ref mut fresh29 = *classbits.offset((c >> 3 as ::core::ffi::c_int) as isize);
        *fresh29 = (*fresh29 as ::core::ffi::c_int
            | ((1 as ::core::ffi::c_uint) << (c & 0x7 as uint32_t)) as uint8_t
                as ::core::ffi::c_int) as uint8_t;
        c = c.wrapping_add(1);
    }
    c = byte_end;
    while c <= classbits_end {
        let ref mut fresh30 = *classbits.offset((c >> 3 as ::core::ffi::c_int) as isize);
        *fresh30 = (*fresh30 as ::core::ffi::c_int
            | ((1 as ::core::ffi::c_uint) << (c & 0x7 as uint32_t)) as uint8_t
                as ::core::ffi::c_int) as uint8_t;
        c = c.wrapping_add(1);
    }
}
unsafe extern "C" fn add_list_to_class(
    mut options: uint32_t,
    mut xoptions: uint32_t,
    mut cb: *mut compile_block_8,
    mut p: *const uint32_t,
) {
    while *p.offset(0 as ::core::ffi::c_int as isize) < 256 as uint32_t {
        let mut n: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
        while *p.offset(n.wrapping_add(1 as ::core::ffi::c_uint) as isize)
            == (*p.offset(0 as ::core::ffi::c_int as isize))
                .wrapping_add(n as uint32_t)
                .wrapping_add(1 as uint32_t)
        {
            n = n.wrapping_add(1);
        }
        add_to_class(
            options,
            xoptions,
            cb,
            *p.offset(0 as ::core::ffi::c_int as isize),
            *p.offset(n as isize),
        );
        p = p.offset(n.wrapping_add(1 as ::core::ffi::c_uint) as isize);
    }
}
unsafe extern "C" fn add_not_list_to_class(
    mut options: uint32_t,
    mut xoptions: uint32_t,
    mut cb: *mut compile_block_8,
    mut p: *const uint32_t,
) {
    if *p.offset(0 as ::core::ffi::c_int as isize) > 0 as uint32_t {
        add_to_class(
            options,
            xoptions,
            cb,
            0 as uint32_t,
            (*p.offset(0 as ::core::ffi::c_int as isize)).wrapping_sub(1 as uint32_t),
        );
    }
    while *p.offset(0 as ::core::ffi::c_int as isize) < 256 as uint32_t {
        while *p.offset(1 as ::core::ffi::c_int as isize)
            == (*p.offset(0 as ::core::ffi::c_int as isize)).wrapping_add(1 as uint32_t)
        {
            p = p.offset(1);
        }
        add_to_class(
            options,
            xoptions,
            cb,
            (*p.offset(0 as ::core::ffi::c_int as isize)).wrapping_add(1 as uint32_t),
            if *p.offset(1 as ::core::ffi::c_int as isize) > 255 as uint32_t {
                255 as uint32_t
            } else {
                (*p.offset(1 as ::core::ffi::c_int as isize)).wrapping_sub(1 as uint32_t)
            },
        );
        p = p.offset(1);
    }
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_compile_class_not_nested_8(
    mut options: uint32_t,
    mut xoptions: uint32_t,
    mut start_ptr: *mut uint32_t,
    mut pcode: *mut *mut PCRE2_UCHAR8,
    mut negate_class: BOOL,
    mut has_bitmap: *mut BOOL,
    mut errorcodeptr: *mut ::core::ffi::c_int,
    mut cb: *mut compile_block_8,
    mut lengthptr: *mut size_t,
) -> *mut uint32_t {
    let mut current_block: u64;
    let mut pptr: *mut uint32_t = start_ptr;
    let mut code: *mut PCRE2_UCHAR8 = *pcode;
    let mut should_flip_negation: BOOL = 0;
    let mut cbits: *const uint8_t = (*cb).cbits;
    let classbits: *mut uint8_t = &raw mut (*cb).classbits.classbits as *mut uint8_t;
    let mut utf: BOOL = (options & PCRE2_UTF as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    let mut xclass_props: uint32_t = 0;
    let mut class_uchardata: *mut PCRE2_UCHAR8 = ::core::ptr::null_mut::<PCRE2_UCHAR8>();
    let mut cranges: *mut class_ranges = ::core::ptr::null_mut::<class_ranges>();
    should_flip_negation = FALSE as BOOL;
    xclass_props = 0 as uint32_t;
    cranges = ::core::ptr::null_mut::<class_ranges>();
    if utf != 0 {
        if !lengthptr.is_null() {
            cranges = compile_optimize_class(pptr, options, xoptions, errorcodeptr, cb);
            if cranges.is_null() {
                return ::core::ptr::null_mut::<uint32_t>();
            }
            if !(*cb).last_data.is_null() {
                (*(*cb).last_data).next = &raw mut (*cranges).header as *mut compile_data;
            } else {
                (*cb).first_data = &raw mut (*cranges).header;
            }
            (*cb).last_data = &raw mut (*cranges).header;
        } else {
            cranges = (*cb).first_data as *mut class_ranges;
            (*cb).first_data = (*cranges).header.next as *mut compile_data;
        }
        if (*cranges).range_list_size as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
            let mut ranges: *const uint32_t =
                cranges.offset(1 as ::core::ffi::c_int as isize) as *const uint32_t;
            if *ranges.offset(0 as ::core::ffi::c_int as isize) <= 255 as uint32_t {
                xclass_props |= XCLASS_HAS_8BIT_CHARS as uint32_t;
            }
            if *ranges.offset(
                ((*cranges).range_list_size as ::core::ffi::c_int - 1 as ::core::ffi::c_int)
                    as isize,
            ) == (if utf != 0 {
                MAX_UTF_CODE_POINT as uint32_t
            } else {
                MAX_UCHAR_VALUE as uint32_t
            }) && *ranges.offset(
                ((*cranges).range_list_size as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
                    as isize,
            ) <= 256 as uint32_t
            {
                xclass_props |= XCLASS_HIGH_ANY as uint32_t;
            }
        }
    }
    class_uchardata = code
        .offset(LINK_SIZE as isize)
        .offset(2 as ::core::ffi::c_int as isize);
    memset(
        classbits as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        32 as size_t,
    );
    loop {
        let fresh0 = pptr;
        pptr = pptr.offset(1);
        let mut meta: uint32_t = *fresh0;
        let mut local_negate: BOOL = 0;
        let mut posix_class: ::core::ffi::c_int = 0;
        let mut taboffset: ::core::ffi::c_int = 0;
        let mut tabopt: ::core::ffi::c_int = 0;
        let mut pbits: class_bits_storage = class_bits_storage { classbits: [0; 32] };
        let mut escape: uint32_t = 0;
        let mut c: uint32_t = 0;
        match meta & 0xffff0000 as uint32_t {
            META_POSIX | META_POSIX_NEG => {
                local_negate = (meta == META_POSIX_NEG as uint32_t) as ::core::ffi::c_int as BOOL;
                let fresh1 = pptr;
                pptr = pptr.offset(1);
                posix_class = *fresh1 as ::core::ffi::c_int;
                if local_negate != 0 {
                    should_flip_negation = TRUE as BOOL;
                }
                if options & PCRE2_CASELESS as uint32_t != 0 as uint32_t
                    && posix_class <= 2 as ::core::ffi::c_int
                {
                    posix_class = 0 as ::core::ffi::c_int;
                }
                if options & PCRE2_UCP as uint32_t != 0 as uint32_t
                    && xoptions & PCRE2_EXTRA_ASCII_POSIX as uint32_t == 0 as uint32_t
                {
                    let mut ptype: uint32_t = 0;
                    match posix_class {
                        PC_GRAPH | PC_PRINT | PC_PUNCT => {
                            ptype = (if posix_class == PC_GRAPH {
                                PT_PXGRAPH
                            } else if posix_class == PC_PRINT {
                                PT_PXPRINT
                            } else {
                                PT_PXPUNCT
                            }) as uint32_t;
                            _pcre2_update_classbits_8(
                                ptype,
                                0 as uint32_t,
                                local_negate,
                                classbits,
                            );
                            if xclass_props & XCLASS_HIGH_ANY as uint32_t == 0 as uint32_t {
                                if !lengthptr.is_null() {
                                    *lengthptr = (*lengthptr).wrapping_add(3 as size_t);
                                } else {
                                    let fresh2 = class_uchardata;
                                    class_uchardata = class_uchardata.offset(1);
                                    *fresh2 = (if local_negate != 0 {
                                        XCL_NOTPROP
                                    } else {
                                        XCL_PROP
                                    })
                                        as PCRE2_UCHAR8;
                                    let fresh3 = class_uchardata;
                                    class_uchardata = class_uchardata.offset(1);
                                    *fresh3 = ptype as PCRE2_UCHAR8;
                                    let fresh4 = class_uchardata;
                                    class_uchardata = class_uchardata.offset(1);
                                    *fresh4 = 0 as PCRE2_UCHAR8;
                                }
                                xclass_props |= (XCLASS_REQUIRED | XCLASS_HAS_PROPS) as uint32_t;
                            }
                            continue;
                        }
                        _ => {}
                    }
                }
                posix_class *= 3 as ::core::ffi::c_int;
                memcpy(
                    &raw mut pbits.classbits as *mut uint8_t as *mut ::core::ffi::c_void,
                    cbits.offset(
                        *(&raw const _pcre2_posix_class_maps8 as *const ::core::ffi::c_int)
                            .offset(posix_class as isize) as isize,
                    ) as *const ::core::ffi::c_void,
                    32 as size_t,
                );
                taboffset = *(&raw const _pcre2_posix_class_maps8 as *const ::core::ffi::c_int)
                    .offset((posix_class + 1 as ::core::ffi::c_int) as isize);
                tabopt = *(&raw const _pcre2_posix_class_maps8 as *const ::core::ffi::c_int)
                    .offset((posix_class + 2 as ::core::ffi::c_int) as isize);
                if taboffset >= 0 as ::core::ffi::c_int {
                    if tabopt >= 0 as ::core::ffi::c_int {
                        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        while i < 32 as ::core::ffi::c_int {
                            pbits.classbits[i as usize] = (pbits.classbits[i as usize]
                                as ::core::ffi::c_int
                                | *cbits.offset((i + taboffset) as isize) as ::core::ffi::c_int)
                                as uint8_t;
                            i += 1;
                        }
                    } else {
                        let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        while i_0 < 32 as ::core::ffi::c_int {
                            pbits.classbits[i_0 as usize] = (pbits.classbits[i_0 as usize]
                                as ::core::ffi::c_int
                                & !(*cbits.offset((i_0 + taboffset) as isize) as ::core::ffi::c_int)
                                    as uint8_t
                                    as ::core::ffi::c_int)
                                as uint8_t;
                            i_0 += 1;
                        }
                    }
                }
                if tabopt < 0 as ::core::ffi::c_int {
                    tabopt = -tabopt;
                }
                if tabopt == 1 as ::core::ffi::c_int {
                    pbits.classbits[1 as ::core::ffi::c_int as usize] =
                        (pbits.classbits[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                            & !(0x3c as ::core::ffi::c_int)) as uint8_t;
                } else if tabopt == 2 as ::core::ffi::c_int {
                    pbits.classbits[11 as ::core::ffi::c_int as usize] =
                        (pbits.classbits[11 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                            & 0x7f as ::core::ffi::c_int) as uint8_t;
                }
                let mut classwords: *mut uint32_t =
                    &raw mut (*cb).classbits.classwords as *mut uint32_t;
                if local_negate != 0 {
                    let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                    while i_1 < 8 as ::core::ffi::c_int {
                        *classwords.offset(i_1 as isize) |= !pbits.classwords[i_1 as usize];
                        i_1 += 1;
                    }
                } else {
                    let mut i_2: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                    while i_2 < 8 as ::core::ffi::c_int {
                        *classwords.offset(i_2 as isize) |= pbits.classwords[i_2 as usize];
                        i_2 += 1;
                    }
                }
                xclass_props |= XCLASS_HAS_8BIT_CHARS as uint32_t;
                continue;
            }
            META_BIGVALUE => {
                let fresh5 = pptr;
                pptr = pptr.offset(1);
                meta = *fresh5;
            }
            META_ESCAPE => {
                escape = meta & 0xffff as uint32_t;
                match escape {
                    7 => {
                        let mut i_3: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        while i_3 < 32 as ::core::ffi::c_int {
                            let ref mut fresh6 = *classbits.offset(i_3 as isize);
                            *fresh6 = (*fresh6 as ::core::ffi::c_int
                                | *cbits.offset((i_3 + cbit_digit) as isize) as ::core::ffi::c_int)
                                as uint8_t;
                            i_3 += 1;
                        }
                    }
                    6 => {
                        should_flip_negation = TRUE as BOOL;
                        let mut i_4: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        while i_4 < 32 as ::core::ffi::c_int {
                            let ref mut fresh7 = *classbits.offset(i_4 as isize);
                            *fresh7 = (*fresh7 as ::core::ffi::c_int
                                | !(*cbits.offset((i_4 + cbit_digit) as isize)
                                    as ::core::ffi::c_int)
                                    as uint8_t
                                    as ::core::ffi::c_int)
                                as uint8_t;
                            i_4 += 1;
                        }
                    }
                    11 => {
                        let mut i_5: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        while i_5 < 32 as ::core::ffi::c_int {
                            let ref mut fresh8 = *classbits.offset(i_5 as isize);
                            *fresh8 = (*fresh8 as ::core::ffi::c_int
                                | *cbits.offset((i_5 + cbit_word) as isize) as ::core::ffi::c_int)
                                as uint8_t;
                            i_5 += 1;
                        }
                    }
                    10 => {
                        should_flip_negation = TRUE as BOOL;
                        let mut i_6: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        while i_6 < 32 as ::core::ffi::c_int {
                            let ref mut fresh9 = *classbits.offset(i_6 as isize);
                            *fresh9 = (*fresh9 as ::core::ffi::c_int
                                | !(*cbits.offset((i_6 + cbit_word) as isize) as ::core::ffi::c_int)
                                    as uint8_t
                                    as ::core::ffi::c_int)
                                as uint8_t;
                            i_6 += 1;
                        }
                    }
                    9 => {
                        let mut i_7: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        while i_7 < 32 as ::core::ffi::c_int {
                            let ref mut fresh10 = *classbits.offset(i_7 as isize);
                            *fresh10 = (*fresh10 as ::core::ffi::c_int
                                | *cbits.offset((i_7 + cbit_space) as isize) as ::core::ffi::c_int)
                                as uint8_t;
                            i_7 += 1;
                        }
                    }
                    8 => {
                        should_flip_negation = TRUE as BOOL;
                        let mut i_8: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        while i_8 < 32 as ::core::ffi::c_int {
                            let ref mut fresh11 = *classbits.offset(i_8 as isize);
                            *fresh11 = (*fresh11 as ::core::ffi::c_int
                                | !(*cbits.offset((i_8 + cbit_space) as isize)
                                    as ::core::ffi::c_int)
                                    as uint8_t
                                    as ::core::ffi::c_int)
                                as uint8_t;
                            i_8 += 1;
                        }
                    }
                    19 => {
                        if cranges.is_null() {
                            add_list_to_class(
                                options & !(PCRE2_CASELESS as uint32_t),
                                xoptions,
                                cb,
                                &raw const _pcre2_hspace_list_8 as *const uint32_t,
                            );
                        }
                    }
                    18 => {
                        if cranges.is_null() {
                            add_not_list_to_class(
                                options & !(PCRE2_CASELESS as uint32_t),
                                xoptions,
                                cb,
                                &raw const _pcre2_hspace_list_8 as *const uint32_t,
                            );
                        }
                    }
                    21 => {
                        if cranges.is_null() {
                            add_list_to_class(
                                options & !(PCRE2_CASELESS as uint32_t),
                                xoptions,
                                cb,
                                &raw const _pcre2_vspace_list_8 as *const uint32_t,
                            );
                        }
                    }
                    20 => {
                        if cranges.is_null() {
                            add_not_list_to_class(
                                options & !(PCRE2_CASELESS as uint32_t),
                                xoptions,
                                cb,
                                &raw const _pcre2_vspace_list_8 as *const uint32_t,
                            );
                        }
                    }
                    16 | 15 => {
                        let mut ptype_0: uint32_t = *pptr >> 16 as ::core::ffi::c_int;
                        let fresh12 = pptr;
                        pptr = pptr.offset(1);
                        let mut pdata: uint32_t = *fresh12 & 0xffff as uint32_t;
                        if ptype_0 == PT_ANY as uint32_t {
                            if utf == 0 && escape == ESC_p as ::core::ffi::c_int as uint32_t {
                                memset(
                                    classbits as *mut ::core::ffi::c_void,
                                    0xff as ::core::ffi::c_int,
                                    32 as size_t,
                                );
                            }
                            continue;
                        } else {
                            _pcre2_update_classbits_8(
                                ptype_0,
                                pdata,
                                (escape == ESC_P as ::core::ffi::c_int as uint32_t)
                                    as ::core::ffi::c_int,
                                classbits,
                            );
                            if xclass_props & XCLASS_HIGH_ANY as uint32_t == 0 as uint32_t {
                                if !lengthptr.is_null() {
                                    *lengthptr = (*lengthptr).wrapping_add(3 as size_t);
                                } else {
                                    let fresh13 = class_uchardata;
                                    class_uchardata = class_uchardata.offset(1);
                                    *fresh13 =
                                        (if escape == ESC_p as ::core::ffi::c_int as uint32_t {
                                            XCL_PROP
                                        } else {
                                            XCL_NOTPROP
                                        }) as PCRE2_UCHAR8;
                                    let fresh14 = class_uchardata;
                                    class_uchardata = class_uchardata.offset(1);
                                    *fresh14 = ptype_0 as PCRE2_UCHAR8;
                                    let fresh15 = class_uchardata;
                                    class_uchardata = class_uchardata.offset(1);
                                    *fresh15 = pdata as PCRE2_UCHAR8;
                                }
                                xclass_props |= (XCLASS_REQUIRED | XCLASS_HAS_PROPS) as uint32_t;
                            }
                            continue;
                        }
                    }
                    _ => {}
                }
                xclass_props |= XCLASS_HAS_8BIT_CHARS as uint32_t;
                continue;
            }
            _ => {
                if !(meta < META_END as uint32_t) {
                    break;
                }
            }
        }
        c = meta;
        if c == CHAR_CR as uint32_t || c == CHAR_NL as uint32_t {
            (*cb).external_flags =
                ((*cb).external_flags as ::core::ffi::c_uint | PCRE2_HASCRORLF) as uint32_t;
        }
        if *pptr == META_RANGE_LITERAL as uint32_t || *pptr == META_RANGE_ESCAPED as uint32_t {
            let mut d: uint32_t = 0;
            pptr = pptr.offset(1);
            let fresh16 = pptr;
            pptr = pptr.offset(1);
            d = *fresh16;
            if d == META_BIGVALUE as uint32_t {
                let fresh17 = pptr;
                pptr = pptr.offset(1);
                d = *fresh17;
            }
            if d == CHAR_CR as uint32_t || d == CHAR_NL as uint32_t {
                (*cb).external_flags =
                    ((*cb).external_flags as ::core::ffi::c_uint | PCRE2_HASCRORLF) as uint32_t;
            }
            if !cranges.is_null() {
                continue;
            }
            xclass_props |= XCLASS_HAS_8BIT_CHARS as uint32_t;
            add_to_class(options, xoptions, cb, c, d);
        } else {
            if !cranges.is_null() {
                continue;
            }
            xclass_props |= XCLASS_HAS_8BIT_CHARS as uint32_t;
            add_to_class(options, xoptions, cb, meta, meta);
        }
    }
    if !cranges.is_null() {
        let mut range: *mut uint32_t =
            cranges.offset(1 as ::core::ffi::c_int as isize) as *mut uint32_t;
        let mut end: *mut uint32_t =
            range.offset((*cranges).range_list_size as ::core::ffi::c_int as isize);
        while range < end && *range.offset(0 as ::core::ffi::c_int as isize) < 256 as uint32_t {
            add_to_class(
                if options & (PCRE2_UTF as uint32_t | PCRE2_UCP as uint32_t) != 0 as uint32_t {
                    options & !(PCRE2_CASELESS as uint32_t)
                } else {
                    options
                },
                xoptions,
                cb,
                *range.offset(0 as ::core::ffi::c_int as isize),
                *range.offset(1 as ::core::ffi::c_int as isize),
            );
            if *range.offset(1 as ::core::ffi::c_int as isize) > 255 as uint32_t {
                break;
            }
            range = range.offset(2 as ::core::ffi::c_int as isize);
        }
        if (*cranges).char_lists_size > 0 as size_t {
            xclass_props |= (XCLASS_REQUIRED | XCLASS_HAS_CHAR_LISTS) as uint32_t;
        } else {
            if xclass_props & XCLASS_HIGH_ANY as uint32_t != 0 as uint32_t {
                should_flip_negation = TRUE as BOOL;
                range = end;
            }
            while range < end {
                let mut range_start: uint32_t = *range.offset(0 as ::core::ffi::c_int as isize);
                let mut range_end: uint32_t = *range.offset(1 as ::core::ffi::c_int as isize);
                range = range.offset(2 as ::core::ffi::c_int as isize);
                xclass_props |= XCLASS_REQUIRED as uint32_t;
                if range_start < 256 as uint32_t {
                    range_start = 256 as uint32_t;
                }
                if !lengthptr.is_null() {
                    if utf != 0 {
                        *lengthptr = (*lengthptr).wrapping_add(1 as size_t);
                        if range_start < range_end {
                            *lengthptr = (*lengthptr).wrapping_add(_pcre2_ord2utf_8(
                                range_start,
                                class_uchardata,
                            )
                                as size_t);
                        }
                        *lengthptr = (*lengthptr)
                            .wrapping_add(_pcre2_ord2utf_8(range_end, class_uchardata) as size_t);
                    } else {
                        *lengthptr = (*lengthptr).wrapping_add(
                            (if range_start < range_end {
                                3 as ::core::ffi::c_int
                            } else {
                                2 as ::core::ffi::c_int
                            }) as size_t,
                        );
                    }
                } else {
                    if !(utf != 0) {
                        continue;
                    }
                    if range_start < range_end {
                        let fresh18 = class_uchardata;
                        class_uchardata = class_uchardata.offset(1);
                        *fresh18 = XCL_RANGE as PCRE2_UCHAR8;
                        class_uchardata =
                            class_uchardata
                                .offset(_pcre2_ord2utf_8(range_start, class_uchardata) as isize);
                    } else {
                        let fresh19 = class_uchardata;
                        class_uchardata = class_uchardata.offset(1);
                        *fresh19 = XCL_SINGLE as PCRE2_UCHAR8;
                    }
                    class_uchardata = class_uchardata
                        .offset(_pcre2_ord2utf_8(range_end, class_uchardata) as isize);
                }
            }
            if lengthptr.is_null() {
                (*(*cb).cx).memctl.free.expect("non-null function pointer")(
                    cranges as *mut ::core::ffi::c_void,
                    (*(*cb).cx).memctl.memory_data,
                );
            }
        }
    }
    if xclass_props & XCLASS_REQUIRED as uint32_t != 0 as uint32_t {
        let mut previous: *mut PCRE2_UCHAR8 = code;
        if xclass_props & XCLASS_HAS_CHAR_LISTS as uint32_t == 0 as uint32_t {
            let fresh20 = class_uchardata;
            class_uchardata = class_uchardata.offset(1);
            *fresh20 = XCL_END as PCRE2_UCHAR8;
        }
        let fresh21 = code;
        code = code.offset(1);
        *fresh21 = OP_XCLASS as ::core::ffi::c_int as PCRE2_UCHAR8;
        code = code.offset(LINK_SIZE as isize);
        *code = (if negate_class != 0 {
            XCL_NOT
        } else {
            0 as ::core::ffi::c_int
        }) as PCRE2_UCHAR8;
        if xclass_props & XCLASS_HAS_PROPS as uint32_t != 0 as uint32_t {
            *code = (*code as ::core::ffi::c_int | XCL_HASPROP) as PCRE2_UCHAR8;
        }
        if xclass_props & XCLASS_HAS_8BIT_CHARS as uint32_t != 0 as uint32_t
            || !has_bitmap.is_null()
        {
            if negate_class != 0 {
                let mut classwords_0: *mut uint32_t =
                    &raw mut (*cb).classbits.classwords as *mut uint32_t;
                let mut i_9: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                while i_9 < 8 as ::core::ffi::c_int {
                    *classwords_0.offset(i_9 as isize) = !*classwords_0.offset(i_9 as isize);
                    i_9 += 1;
                }
            }
            if has_bitmap.is_null() {
                let fresh22 = code;
                code = code.offset(1);
                *fresh22 = (*fresh22 as ::core::ffi::c_int | XCL_MAP) as PCRE2_UCHAR8;
                memmove(
                    code.offset(
                        (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize)
                            as isize,
                    ) as *mut ::core::ffi::c_void,
                    code as *const ::core::ffi::c_void,
                    (class_uchardata.offset_from(code) as isize
                        * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as isize)
                        as size_t,
                );
                memcpy(
                    code as *mut ::core::ffi::c_void,
                    classbits as *const ::core::ffi::c_void,
                    32 as size_t,
                );
                code = class_uchardata.offset(
                    (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize)
                        as isize,
                );
            } else {
                code = class_uchardata;
                if xclass_props & XCLASS_HAS_8BIT_CHARS as uint32_t != 0 as uint32_t {
                    *has_bitmap = TRUE as BOOL;
                }
            }
        } else {
            code = class_uchardata;
        }
        if xclass_props & XCLASS_HAS_CHAR_LISTS as uint32_t != 0 as uint32_t {
            let mut char_lists_size: size_t = (*cranges).char_lists_size;
            if !lengthptr.is_null() {
                char_lists_size = char_lists_size
                    .wrapping_add(::core::mem::size_of::<uint32_t>().wrapping_sub(1 as size_t))
                    & !::core::mem::size_of::<uint32_t>().wrapping_sub(1 as size_t);
                *lengthptr =
                    (*lengthptr).wrapping_add((2 as ::core::ffi::c_int + LINK_SIZE) as size_t);
                if char_lists_size > (PCRE2_SIZE_MAX as size_t).wrapping_sub((*cb).char_lists_size)
                    || char_lists_size
                        .wrapping_div((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t)
                        > MAX_PATTERN_SIZE as size_t
                    || (*cb)
                        .char_lists_size
                        .wrapping_div((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t)
                        > (MAX_PATTERN_SIZE as size_t).wrapping_sub(char_lists_size.wrapping_div(
                            (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                        ))
                {
                    *errorcodeptr = ERR20 as ::core::ffi::c_int;
                    return ::core::ptr::null_mut::<uint32_t>();
                }
                (*cb).char_lists_size = (*cb).char_lists_size.wrapping_add(char_lists_size);
            } else {
                let mut data: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
                *code.offset(0 as ::core::ffi::c_int as isize) =
                    ((if ::core::mem::size_of::<PCRE2_UCHAR8>() as usize == 1 as usize {
                        0x10 as ::core::ffi::c_int
                    } else {
                        0x1000 as ::core::ffi::c_int
                    }) | (*cranges).char_lists_types as ::core::ffi::c_int
                        >> 8 as ::core::ffi::c_int) as uint8_t as PCRE2_UCHAR8;
                *code.offset(1 as ::core::ffi::c_int as isize) =
                    (*cranges).char_lists_types as uint8_t as PCRE2_UCHAR8;
                code = code.offset(2 as ::core::ffi::c_int as isize);
                (*cb).char_lists_size = (*cb).char_lists_size.wrapping_add(char_lists_size);
                data = ((*cb).start_code as *mut uint8_t).offset(-((*cb).char_lists_size as isize));
                memcpy(
                    data as *mut ::core::ffi::c_void,
                    (cranges.offset(1 as ::core::ffi::c_int as isize) as *mut uint8_t)
                        .offset((*cranges).char_lists_start as isize)
                        as *const ::core::ffi::c_void,
                    char_lists_size,
                );
                char_lists_size = (*cb).char_lists_size;
                *code.offset(0 as ::core::ffi::c_int as isize) =
                    ((char_lists_size >> 1 as ::core::ffi::c_int) as uint32_t
                        >> 8 as ::core::ffi::c_int) as PCRE2_UCHAR8;
                *code.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize) =
                    ((char_lists_size >> 1 as ::core::ffi::c_int) as uint32_t & 255 as uint32_t)
                        as PCRE2_UCHAR8;
                code = code.offset(LINK_SIZE as isize);
                if char_lists_size & 0x2 as size_t != 0 as size_t {
                    *(data as *mut uint16_t).offset(-(1 as ::core::ffi::c_int) as isize) =
                        0xdead as uint16_t;
                }
                (*cb).char_lists_size = char_lists_size
                    .wrapping_add(::core::mem::size_of::<uint32_t>().wrapping_sub(1 as size_t))
                    & !::core::mem::size_of::<uint32_t>().wrapping_sub(1 as size_t);
                (*(*cb).cx).memctl.free.expect("non-null function pointer")(
                    cranges as *mut ::core::ffi::c_void,
                    (*(*cb).cx).memctl.memory_data,
                );
            }
        }
        *previous.offset(1 as ::core::ffi::c_int as isize) =
            (code.offset_from(previous) as isize as ::core::ffi::c_int
                >> 8 as ::core::ffi::c_int) as PCRE2_UCHAR8;
        *previous.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize) =
            (code.offset_from(previous) as isize as ::core::ffi::c_int
                & 255 as ::core::ffi::c_int) as PCRE2_UCHAR8;
    } else {
        if negate_class != 0 {
            let mut classwords_1: *mut uint32_t =
                &raw mut (*cb).classbits.classwords as *mut uint32_t;
            let mut i_10: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_10 < 8 as ::core::ffi::c_int {
                *classwords_1.offset(i_10 as isize) = !*classwords_1.offset(i_10 as isize);
                i_10 += 1;
            }
        }
        if (utf == 0 || negate_class != should_flip_negation)
            && (*cb).classbits.classwords[0 as ::core::ffi::c_int as usize]
                == !(0 as ::core::ffi::c_int as uint32_t)
        {
            let mut classwords_2: *const uint32_t =
                &raw mut (*cb).classbits.classwords as *mut uint32_t;
            let mut i_11: ::core::ffi::c_int = 0;
            i_11 = 0 as ::core::ffi::c_int;
            while i_11 < 8 as ::core::ffi::c_int {
                if *classwords_2.offset(i_11 as isize) != !(0 as ::core::ffi::c_int as uint32_t) {
                    break;
                }
                i_11 += 1;
            }
            if i_11 == 8 as ::core::ffi::c_int {
                let fresh23 = code;
                code = code.offset(1);
                *fresh23 = OP_ALLANY as ::core::ffi::c_int as PCRE2_UCHAR8;
                current_block = 16435072578689821955;
            } else {
                current_block = 7409542616634851420;
            }
        } else {
            current_block = 7409542616634851420;
        }
        match current_block {
            16435072578689821955 => {}
            _ => {
                let fresh24 = code;
                code = code.offset(1);
                *fresh24 = (if negate_class == should_flip_negation {
                    OP_CLASS as ::core::ffi::c_int
                } else {
                    OP_NCLASS as ::core::ffi::c_int
                }) as PCRE2_UCHAR8;
                memcpy(
                    code as *mut ::core::ffi::c_void,
                    classbits as *const ::core::ffi::c_void,
                    32 as size_t,
                );
                code = code.offset(
                    (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize)
                        as isize,
                );
            }
        }
    }
    *pcode = code;
    return pptr.offset(-(1 as ::core::ffi::c_int as isize));
}
unsafe extern "C" fn fold_negation(
    mut pop_info: *mut eclass_op_info,
    mut lengthptr: *mut size_t,
    mut preserve_classbits: BOOL,
) {
    if (*pop_info).op_single_type as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        if !lengthptr.is_null() {
            *lengthptr = (*lengthptr).wrapping_add(1 as size_t);
        } else {
            *(*pop_info).code_start.offset((*pop_info).length as isize) = ECL_NOT as PCRE2_UCHAR8;
        }
        (*pop_info).length = (*pop_info).length.wrapping_add(1 as size_t);
    } else if (*pop_info).op_single_type as ::core::ffi::c_int == ECL_ANY
        || (*pop_info).op_single_type as ::core::ffi::c_int == ECL_NONE
    {
        (*pop_info).op_single_type =
            (if (*pop_info).op_single_type as ::core::ffi::c_int == ECL_NONE {
                ECL_ANY
            } else {
                ECL_NONE
            }) as uint8_t;
        if lengthptr.is_null() {
            *(*pop_info).code_start = (*pop_info).op_single_type as PCRE2_UCHAR8;
        }
    } else if lengthptr.is_null() {
        let ref mut fresh46 = *(*pop_info)
            .code_start
            .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
        *fresh46 = (*fresh46 as ::core::ffi::c_int ^ XCL_NOT) as PCRE2_UCHAR8;
    }
    if preserve_classbits == 0 {
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < 8 as ::core::ffi::c_int {
            (*pop_info).bits.classwords[i as usize] = !(*pop_info).bits.classwords[i as usize];
            i += 1;
        }
    }
}
unsafe extern "C" fn fold_binary(
    mut op: ::core::ffi::c_int,
    mut lhs_op_info: *mut eclass_op_info,
    mut rhs_op_info: *mut eclass_op_info,
    mut lengthptr: *mut size_t,
) {
    match op {
        ECL_AND => {
            if !((*rhs_op_info).op_single_type as ::core::ffi::c_int == ECL_ANY) {
                if (*lhs_op_info).op_single_type as ::core::ffi::c_int == ECL_ANY {
                    if lengthptr.is_null() {
                        memmove(
                            (*lhs_op_info).code_start as *mut ::core::ffi::c_void,
                            (*rhs_op_info).code_start as *const ::core::ffi::c_void,
                            (*rhs_op_info).length.wrapping_mul(
                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                            ),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
                } else if (*rhs_op_info).op_single_type as ::core::ffi::c_int == ECL_NONE {
                    if lengthptr.is_null() {
                        *(*lhs_op_info)
                            .code_start
                            .offset(0 as ::core::ffi::c_int as isize) = ECL_NONE as PCRE2_UCHAR8;
                    }
                    (*lhs_op_info).length = 1 as size_t;
                    (*lhs_op_info).op_single_type = ECL_NONE as uint8_t;
                } else if !((*lhs_op_info).op_single_type as ::core::ffi::c_int == ECL_NONE) {
                    if !lengthptr.is_null() {
                        *lengthptr = (*lengthptr).wrapping_add(1 as size_t);
                    } else {
                        *(*rhs_op_info)
                            .code_start
                            .offset((*rhs_op_info).length as isize) = ECL_AND as PCRE2_UCHAR8;
                    }
                    (*lhs_op_info).length = (*lhs_op_info)
                        .length
                        .wrapping_add((*rhs_op_info).length.wrapping_add(1 as size_t));
                    (*lhs_op_info).op_single_type = 0 as uint8_t;
                }
            }
            let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i < 8 as ::core::ffi::c_int {
                (*lhs_op_info).bits.classwords[i as usize] &=
                    (*rhs_op_info).bits.classwords[i as usize];
                i += 1;
            }
        }
        ECL_OR => {
            if !((*rhs_op_info).op_single_type as ::core::ffi::c_int == ECL_NONE) {
                if (*lhs_op_info).op_single_type as ::core::ffi::c_int == ECL_NONE {
                    if lengthptr.is_null() {
                        memmove(
                            (*lhs_op_info).code_start as *mut ::core::ffi::c_void,
                            (*rhs_op_info).code_start as *const ::core::ffi::c_void,
                            (*rhs_op_info).length.wrapping_mul(
                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                            ),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
                } else if (*rhs_op_info).op_single_type as ::core::ffi::c_int == ECL_ANY {
                    if lengthptr.is_null() {
                        *(*lhs_op_info)
                            .code_start
                            .offset(0 as ::core::ffi::c_int as isize) = ECL_ANY as PCRE2_UCHAR8;
                    }
                    (*lhs_op_info).length = 1 as size_t;
                    (*lhs_op_info).op_single_type = ECL_ANY as uint8_t;
                } else if !((*lhs_op_info).op_single_type as ::core::ffi::c_int == ECL_ANY) {
                    if !lengthptr.is_null() {
                        *lengthptr = (*lengthptr).wrapping_add(1 as size_t);
                    } else {
                        *(*rhs_op_info)
                            .code_start
                            .offset((*rhs_op_info).length as isize) = ECL_OR as PCRE2_UCHAR8;
                    }
                    (*lhs_op_info).length = (*lhs_op_info)
                        .length
                        .wrapping_add((*rhs_op_info).length.wrapping_add(1 as size_t));
                    (*lhs_op_info).op_single_type = 0 as uint8_t;
                }
            }
            let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_0 < 8 as ::core::ffi::c_int {
                (*lhs_op_info).bits.classwords[i_0 as usize] |=
                    (*rhs_op_info).bits.classwords[i_0 as usize];
                i_0 += 1;
            }
        }
        ECL_XOR => {
            if !((*rhs_op_info).op_single_type as ::core::ffi::c_int == ECL_NONE) {
                if (*lhs_op_info).op_single_type as ::core::ffi::c_int == ECL_NONE {
                    if lengthptr.is_null() {
                        memmove(
                            (*lhs_op_info).code_start as *mut ::core::ffi::c_void,
                            (*rhs_op_info).code_start as *const ::core::ffi::c_void,
                            (*rhs_op_info).length.wrapping_mul(
                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                            ),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
                } else if (*rhs_op_info).op_single_type as ::core::ffi::c_int == ECL_ANY {
                    fold_negation(lhs_op_info, lengthptr, TRUE);
                } else if (*lhs_op_info).op_single_type as ::core::ffi::c_int == ECL_ANY {
                    if lengthptr.is_null() {
                        memmove(
                            (*lhs_op_info).code_start as *mut ::core::ffi::c_void,
                            (*rhs_op_info).code_start as *const ::core::ffi::c_void,
                            (*rhs_op_info).length.wrapping_mul(
                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                            ),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
                    fold_negation(lhs_op_info, lengthptr, TRUE);
                } else {
                    if !lengthptr.is_null() {
                        *lengthptr = (*lengthptr).wrapping_add(1 as size_t);
                    } else {
                        *(*rhs_op_info)
                            .code_start
                            .offset((*rhs_op_info).length as isize) = ECL_XOR as PCRE2_UCHAR8;
                    }
                    (*lhs_op_info).length = (*lhs_op_info)
                        .length
                        .wrapping_add((*rhs_op_info).length.wrapping_add(1 as size_t));
                    (*lhs_op_info).op_single_type = 0 as uint8_t;
                }
            }
            let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_1 < 8 as ::core::ffi::c_int {
                (*lhs_op_info).bits.classwords[i_1 as usize] ^=
                    (*rhs_op_info).bits.classwords[i_1 as usize];
                i_1 += 1;
            }
        }
        _ => {}
    };
}
unsafe extern "C" fn compile_class_operand(
    mut context: *mut eclass_context,
    mut negated: BOOL,
    mut pptr: *mut *mut uint32_t,
    mut pcode: *mut *mut PCRE2_UCHAR8,
    mut pop_info: *mut eclass_op_info,
    mut lengthptr: *mut size_t,
) -> BOOL {
    let mut current_block: u64;
    let mut ptr: *mut uint32_t = *pptr;
    let mut prev_ptr: *mut uint32_t = ::core::ptr::null_mut::<uint32_t>();
    let mut code: *mut PCRE2_UCHAR8 = *pcode;
    let mut code_start: *mut PCRE2_UCHAR8 = code;
    let mut prev_length: size_t = if !lengthptr.is_null() {
        *lengthptr
    } else {
        0 as size_t
    };
    let mut extra_length: size_t = 0;
    let mut meta: uint32_t = *ptr & 0xffff0000 as uint32_t;
    match meta {
        META_CLASS_EMPTY_NOT | META_CLASS_EMPTY => {
            ptr = ptr.offset(1);
            (*pop_info).length = 1 as size_t;
            if (meta == META_CLASS_EMPTY as uint32_t) as ::core::ffi::c_int == negated {
                (*pop_info).op_single_type = ECL_ANY as uint8_t;
                let fresh47 = code;
                code = code.offset(1);
                *fresh47 = (*pop_info).op_single_type as PCRE2_UCHAR8;
                memset(
                    &raw mut (*pop_info).bits.classbits as *mut uint8_t as *mut ::core::ffi::c_void,
                    0xff as ::core::ffi::c_int,
                    32 as size_t,
                );
            } else {
                (*pop_info).op_single_type = ECL_NONE as uint8_t;
                let fresh48 = code;
                code = code.offset(1);
                *fresh48 = (*pop_info).op_single_type as PCRE2_UCHAR8;
                memset(
                    &raw mut (*pop_info).bits.classbits as *mut uint8_t as *mut ::core::ffi::c_void,
                    0 as ::core::ffi::c_int,
                    32 as size_t,
                );
            }
            current_block = 6545907279487748450;
        }
        META_CLASS | META_CLASS_NOT => {
            if *ptr & CLASS_IS_ECLASS as uint32_t != 0 as uint32_t {
                if compile_eclass_nested(
                    context,
                    negated,
                    &raw mut ptr,
                    &raw mut code,
                    pop_info,
                    lengthptr,
                ) == 0
                {
                    return FALSE;
                }
                ptr = ptr.offset(1);
                current_block = 8869332144787829186;
            } else {
                ptr = ptr.offset(1);
                current_block = 13164282745453918834;
            }
        }
        _ => {
            current_block = 13164282745453918834;
        }
    }
    match current_block {
        13164282745453918834 => {
            prev_ptr = ptr;
            ptr = _pcre2_compile_class_not_nested_8(
                (*context).options,
                (*context).xoptions,
                ptr,
                &raw mut code,
                ((meta != META_CLASS_NOT as uint32_t) as ::core::ffi::c_int == negated)
                    as ::core::ffi::c_int,
                &raw mut (*context).needs_bitmap,
                (*context).errorcodeptr,
                (*context).cb,
                lengthptr,
            );
            if ptr.is_null() {
                return FALSE;
            }
            if ptr <= prev_ptr {
                return FALSE;
            }
            if meta == META_CLASS as uint32_t || meta == META_CLASS_NOT as uint32_t {
                ptr = ptr.offset(1);
            }
            extra_length = if !lengthptr.is_null() {
                (*lengthptr).wrapping_sub(prev_length)
            } else {
                0 as size_t
            };
            if *code_start as ::core::ffi::c_int == OP_ALLANY as ::core::ffi::c_int {
                (*pop_info).length = 1 as size_t;
                (*pop_info).op_single_type = ECL_ANY as uint8_t;
                *code_start = (*pop_info).op_single_type as PCRE2_UCHAR8;
                memset(
                    &raw mut (*pop_info).bits.classbits as *mut uint8_t as *mut ::core::ffi::c_void,
                    0xff as ::core::ffi::c_int,
                    32 as size_t,
                );
            } else if *code_start as ::core::ffi::c_int == OP_CLASS as ::core::ffi::c_int
                || *code_start as ::core::ffi::c_int == OP_NCLASS as ::core::ffi::c_int
            {
                (*pop_info).length = 1 as size_t;
                (*pop_info).op_single_type =
                    (if *code_start as ::core::ffi::c_int == OP_CLASS as ::core::ffi::c_int {
                        ECL_NONE
                    } else {
                        ECL_ANY
                    }) as uint8_t;
                *code_start = (*pop_info).op_single_type as PCRE2_UCHAR8;
                memcpy(
                    &raw mut (*pop_info).bits.classbits as *mut uint8_t as *mut ::core::ffi::c_void,
                    code_start.offset(1 as ::core::ffi::c_int as isize)
                        as *const ::core::ffi::c_void,
                    32 as size_t,
                );
                if !lengthptr.is_null() {
                    *lengthptr = (*lengthptr).wrapping_add(
                        code.offset_from(code_start.offset(1 as ::core::ffi::c_int as isize))
                            as isize as size_t,
                    );
                }
                code = code_start.offset(1 as ::core::ffi::c_int as isize);
                if (*context).needs_bitmap == 0 && *code_start as ::core::ffi::c_int == ECL_NONE {
                    let mut classwords: *mut uint32_t =
                        &raw mut (*pop_info).bits.classwords as *mut uint32_t;
                    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                    while i < 8 as ::core::ffi::c_int {
                        if *classwords.offset(i as isize) != 0 as uint32_t {
                            (*context).needs_bitmap = TRUE as BOOL;
                            break;
                        } else {
                            i += 1;
                        }
                    }
                } else {
                    (*context).needs_bitmap = TRUE as BOOL;
                }
            } else {
                (*pop_info).op_single_type = ECL_XCLASS as uint8_t;
                *code_start = (*pop_info).op_single_type as PCRE2_UCHAR8;
                memcpy(
                    &raw mut (*pop_info).bits.classbits as *mut uint8_t as *mut ::core::ffi::c_void,
                    &raw mut (*(*context).cb).classbits.classbits as *mut uint8_t
                        as *const ::core::ffi::c_void,
                    32 as size_t,
                );
                (*pop_info).length = (code.offset_from(code_start) as isize
                    as size_t)
                    .wrapping_add(extra_length);
            }
            current_block = 6545907279487748450;
        }
        _ => {}
    }
    match current_block {
        6545907279487748450 => {
            (*pop_info).code_start = if lengthptr.is_null() {
                code_start
            } else {
                ::core::ptr::null_mut::<PCRE2_UCHAR8>()
            };
            if !lengthptr.is_null() {
                *lengthptr = (*lengthptr)
                    .wrapping_add(code.offset_from(code_start) as isize as size_t);
                code = code_start;
            }
        }
        _ => {}
    }
    *pptr = ptr;
    *pcode = code;
    return TRUE;
}
unsafe extern "C" fn compile_class_juxtaposition(
    mut context: *mut eclass_context,
    mut negated: BOOL,
    mut pptr: *mut *mut uint32_t,
    mut pcode: *mut *mut PCRE2_UCHAR8,
    mut pop_info: *mut eclass_op_info,
    mut lengthptr: *mut size_t,
) -> BOOL {
    let mut ptr: *mut uint32_t = *pptr;
    let mut code: *mut PCRE2_UCHAR8 = *pcode;
    if compile_class_operand(
        context,
        negated,
        &raw mut ptr,
        &raw mut code,
        pop_info,
        lengthptr,
    ) == 0
    {
        return FALSE;
    }
    while *ptr != META_CLASS_END as uint32_t
        && !(*ptr >= META_ECLASS_AND as uint32_t && *ptr <= META_ECLASS_NOT as uint32_t)
    {
        let mut op: uint32_t = 0;
        let mut rhs_negated: BOOL = 0;
        let mut rhs_op_info: eclass_op_info = eclass_op_info {
            code_start: ::core::ptr::null_mut::<PCRE2_UCHAR8>(),
            length: 0,
            op_single_type: 0,
            bits: class_bits_storage { classbits: [0; 32] },
        };
        if negated != 0 {
            op = ECL_AND as uint32_t;
            rhs_negated = TRUE as BOOL;
        } else {
            op = ECL_OR as uint32_t;
            rhs_negated = FALSE as BOOL;
        }
        if compile_class_operand(
            context,
            rhs_negated,
            &raw mut ptr,
            &raw mut code,
            &raw mut rhs_op_info,
            lengthptr,
        ) == 0
        {
            return FALSE;
        }
        fold_binary(
            op as ::core::ffi::c_int,
            pop_info,
            &raw mut rhs_op_info,
            lengthptr,
        );
        if lengthptr.is_null() {
            code = (*pop_info).code_start.offset((*pop_info).length as isize);
        }
    }
    *pptr = ptr;
    *pcode = code;
    return TRUE;
}
unsafe extern "C" fn compile_class_unary(
    mut context: *mut eclass_context,
    mut negated: BOOL,
    mut pptr: *mut *mut uint32_t,
    mut pcode: *mut *mut PCRE2_UCHAR8,
    mut pop_info: *mut eclass_op_info,
    mut lengthptr: *mut size_t,
) -> BOOL {
    let mut ptr: *mut uint32_t = *pptr;
    while *ptr == META_ECLASS_NOT as uint32_t {
        ptr = ptr.offset(1);
        negated = (negated == 0) as ::core::ffi::c_int as BOOL;
    }
    *pptr = ptr;
    if compile_class_juxtaposition(context, negated, pptr, pcode, pop_info, lengthptr) == 0 {
        return FALSE;
    }
    return TRUE;
}
unsafe extern "C" fn compile_class_binary_tight(
    mut context: *mut eclass_context,
    mut negated: BOOL,
    mut pptr: *mut *mut uint32_t,
    mut pcode: *mut *mut PCRE2_UCHAR8,
    mut pop_info: *mut eclass_op_info,
    mut lengthptr: *mut size_t,
) -> BOOL {
    let mut ptr: *mut uint32_t = *pptr;
    let mut code: *mut PCRE2_UCHAR8 = *pcode;
    if compile_class_unary(
        context,
        negated,
        &raw mut ptr,
        &raw mut code,
        pop_info,
        lengthptr,
    ) == 0
    {
        return FALSE;
    }
    while *ptr == META_ECLASS_AND as uint32_t {
        let mut op: uint32_t = 0;
        let mut rhs_negated: BOOL = 0;
        let mut rhs_op_info: eclass_op_info = eclass_op_info {
            code_start: ::core::ptr::null_mut::<PCRE2_UCHAR8>(),
            length: 0,
            op_single_type: 0,
            bits: class_bits_storage { classbits: [0; 32] },
        };
        if negated != 0 {
            op = ECL_OR as uint32_t;
            rhs_negated = TRUE as BOOL;
        } else {
            op = ECL_AND as uint32_t;
            rhs_negated = FALSE as BOOL;
        }
        ptr = ptr.offset(1);
        if compile_class_unary(
            context,
            rhs_negated,
            &raw mut ptr,
            &raw mut code,
            &raw mut rhs_op_info,
            lengthptr,
        ) == 0
        {
            return FALSE;
        }
        fold_binary(
            op as ::core::ffi::c_int,
            pop_info,
            &raw mut rhs_op_info,
            lengthptr,
        );
        if lengthptr.is_null() {
            code = (*pop_info).code_start.offset((*pop_info).length as isize);
        }
    }
    *pptr = ptr;
    *pcode = code;
    return TRUE;
}
unsafe extern "C" fn compile_class_binary_loose(
    mut context: *mut eclass_context,
    mut negated: BOOL,
    mut pptr: *mut *mut uint32_t,
    mut pcode: *mut *mut PCRE2_UCHAR8,
    mut pop_info: *mut eclass_op_info,
    mut lengthptr: *mut size_t,
) -> BOOL {
    let mut ptr: *mut uint32_t = *pptr;
    let mut code: *mut PCRE2_UCHAR8 = *pcode;
    if compile_class_binary_tight(
        context,
        negated,
        &raw mut ptr,
        &raw mut code,
        pop_info,
        lengthptr,
    ) == 0
    {
        return FALSE;
    }
    while *ptr >= META_ECLASS_OR as uint32_t && *ptr <= META_ECLASS_XOR as uint32_t {
        let mut op: uint32_t = 0;
        let mut op_neg: BOOL = 0;
        let mut rhs_negated: BOOL = 0;
        let mut rhs_op_info: eclass_op_info = eclass_op_info {
            code_start: ::core::ptr::null_mut::<PCRE2_UCHAR8>(),
            length: 0,
            op_single_type: 0,
            bits: class_bits_storage { classbits: [0; 32] },
        };
        if negated != 0 {
            op = (if *ptr == META_ECLASS_OR as uint32_t {
                ECL_AND
            } else if *ptr == META_ECLASS_SUB as uint32_t {
                ECL_OR
            } else {
                ECL_XOR
            }) as uint32_t;
            op_neg = (*ptr == META_ECLASS_XOR as uint32_t) as ::core::ffi::c_int as BOOL;
            rhs_negated = (*ptr != META_ECLASS_SUB as uint32_t) as ::core::ffi::c_int as BOOL;
        } else {
            op = (if *ptr == META_ECLASS_OR as uint32_t {
                ECL_OR
            } else if *ptr == META_ECLASS_SUB as uint32_t {
                ECL_AND
            } else {
                ECL_XOR
            }) as uint32_t;
            op_neg = FALSE as BOOL;
            rhs_negated = (*ptr == META_ECLASS_SUB as uint32_t) as ::core::ffi::c_int as BOOL;
        }
        ptr = ptr.offset(1);
        if compile_class_binary_tight(
            context,
            rhs_negated,
            &raw mut ptr,
            &raw mut code,
            &raw mut rhs_op_info,
            lengthptr,
        ) == 0
        {
            return FALSE;
        }
        fold_binary(
            op as ::core::ffi::c_int,
            pop_info,
            &raw mut rhs_op_info,
            lengthptr,
        );
        if op_neg != 0 {
            fold_negation(pop_info, lengthptr, FALSE);
        }
        if lengthptr.is_null() {
            code = (*pop_info).code_start.offset((*pop_info).length as isize);
        }
    }
    *pptr = ptr;
    *pcode = code;
    return TRUE;
}
unsafe extern "C" fn compile_eclass_nested(
    mut context: *mut eclass_context,
    mut negated: BOOL,
    mut pptr: *mut *mut uint32_t,
    mut pcode: *mut *mut PCRE2_UCHAR8,
    mut pop_info: *mut eclass_op_info,
    mut lengthptr: *mut size_t,
) -> BOOL {
    let mut ptr: *mut uint32_t = *pptr;
    let fresh45 = ptr;
    ptr = ptr.offset(1);
    if *fresh45 == META_CLASS_NOT as uint32_t | CLASS_IS_ECLASS as uint32_t {
        negated = (negated == 0) as ::core::ffi::c_int as BOOL;
    }
    *pptr = (*pptr).offset(1);
    if compile_class_binary_loose(context, negated, pptr, pcode, pop_info, lengthptr) == 0 {
        return FALSE;
    }
    return TRUE;
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_compile_class_nested_8(
    mut options: uint32_t,
    mut xoptions: uint32_t,
    mut pptr: *mut *mut uint32_t,
    mut pcode: *mut *mut PCRE2_UCHAR8,
    mut errorcodeptr: *mut ::core::ffi::c_int,
    mut cb: *mut compile_block_8,
    mut lengthptr: *mut size_t,
) -> BOOL {
    let mut context: eclass_context = eclass_context {
        options: 0,
        xoptions: 0,
        errorcodeptr: ::core::ptr::null_mut::<::core::ffi::c_int>(),
        cb: ::core::ptr::null_mut::<compile_block_8>(),
        needs_bitmap: 0,
    };
    let mut op_info: eclass_op_info = eclass_op_info {
        code_start: ::core::ptr::null_mut::<PCRE2_UCHAR8>(),
        length: 0,
        op_single_type: 0,
        bits: class_bits_storage { classbits: [0; 32] },
    };
    let mut previous_length: size_t = if !lengthptr.is_null() {
        *lengthptr
    } else {
        0 as size_t
    };
    let mut code: *mut PCRE2_UCHAR8 = *pcode;
    let mut previous: *mut PCRE2_UCHAR8 = ::core::ptr::null_mut::<PCRE2_UCHAR8>();
    let mut allbitsone: BOOL = TRUE;
    context.needs_bitmap = FALSE as BOOL;
    context.options = options;
    context.xoptions = xoptions;
    context.errorcodeptr = errorcodeptr;
    context.cb = cb;
    previous = code;
    let fresh34 = code;
    code = code.offset(1);
    *fresh34 = OP_ECLASS as ::core::ffi::c_int as PCRE2_UCHAR8;
    code = code.offset(LINK_SIZE as isize);
    let fresh35 = code;
    code = code.offset(1);
    *fresh35 = 0 as PCRE2_UCHAR8;
    if compile_eclass_nested(
        &raw mut context,
        FALSE,
        pptr,
        &raw mut code,
        &raw mut op_info,
        lengthptr,
    ) == 0
    {
        return FALSE;
    }
    if !lengthptr.is_null() {
        *lengthptr =
            (*lengthptr).wrapping_add(code.offset_from(previous) as isize as size_t);
        code = previous;
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 8 as ::core::ffi::c_int {
        if op_info.bits.classwords[i as usize] != 0xffffffff as uint32_t {
            allbitsone = FALSE as BOOL;
            break;
        } else {
            i += 1;
        }
    }
    if op_info.op_single_type as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        code = previous;
        if op_info.op_single_type as ::core::ffi::c_int == ECL_ANY && allbitsone != 0 {
            if !lengthptr.is_null() {
                *lengthptr = (*lengthptr).wrapping_sub(1 as size_t);
            }
            let fresh36 = code;
            code = code.offset(1);
            *fresh36 = OP_ALLANY as ::core::ffi::c_int as PCRE2_UCHAR8;
        } else if op_info.op_single_type as ::core::ffi::c_int == ECL_ANY
            || op_info.op_single_type as ::core::ffi::c_int == ECL_NONE
        {
            let mut required_len: size_t = (1 as size_t).wrapping_add(
                (32 as size_t).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as size_t),
            );
            if !lengthptr.is_null() {
                if required_len > (*lengthptr).wrapping_sub(previous_length) {
                    *lengthptr = previous_length.wrapping_add(required_len);
                }
            }
            if !lengthptr.is_null() {
                *lengthptr = (*lengthptr).wrapping_sub(required_len);
            }
            let fresh37 = code;
            code = code.offset(1);
            *fresh37 = (if op_info.op_single_type as ::core::ffi::c_int == ECL_ANY {
                OP_NCLASS as ::core::ffi::c_int
            } else {
                OP_CLASS as ::core::ffi::c_int
            }) as PCRE2_UCHAR8;
            memcpy(
                code as *mut ::core::ffi::c_void,
                &raw mut op_info.bits.classbits as *mut uint8_t as *const ::core::ffi::c_void,
                32 as size_t,
            );
            code = code.offset(
                (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize)
                    as isize,
            );
        } else {
            let mut need_map: BOOL = context.needs_bitmap;
            let mut required_len_0: size_t = 0;
            required_len_0 = op_info.length.wrapping_add(
                (if need_map != 0 {
                    (32 as size_t).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as size_t)
                } else {
                    0 as size_t
                }),
            );
            if !lengthptr.is_null() {
                if required_len_0 > (*lengthptr).wrapping_sub(previous_length) {
                    *lengthptr = previous_length.wrapping_add(required_len_0);
                }
                *lengthptr = (*lengthptr).wrapping_sub(
                    (1 as ::core::ffi::c_int + LINK_SIZE + 1 as ::core::ffi::c_int) as size_t,
                );
                let fresh38 = code;
                code = code.offset(1);
                *fresh38 = OP_XCLASS as ::core::ffi::c_int as PCRE2_UCHAR8;
                *code.offset(0 as ::core::ffi::c_int as isize) =
                    (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int + 1 as ::core::ffi::c_int
                        >> 8 as ::core::ffi::c_int) as PCRE2_UCHAR8;
                *code.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize) =
                    (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int + 1 as ::core::ffi::c_int
                        & 255 as ::core::ffi::c_int) as PCRE2_UCHAR8;
                code = code.offset(LINK_SIZE as isize);
                let fresh39 = code;
                code = code.offset(1);
                *fresh39 = 0 as PCRE2_UCHAR8;
            } else {
                let mut rest: *mut PCRE2_UCHAR8 = ::core::ptr::null_mut::<PCRE2_UCHAR8>();
                let mut rest_len: size_t = 0;
                let mut flags: PCRE2_UCHAR8 = 0;
                rest = op_info
                    .code_start
                    .offset(1 as ::core::ffi::c_int as isize)
                    .offset(LINK_SIZE as isize)
                    .offset(1 as ::core::ffi::c_int as isize);
                rest_len = op_info
                    .code_start
                    .offset(op_info.length as isize)
                    .offset_from(rest) as isize as size_t;
                flags = *op_info
                    .code_start
                    .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                memmove(
                    code.offset(1 as ::core::ffi::c_int as isize)
                        .offset(LINK_SIZE as isize)
                        .offset(1 as ::core::ffi::c_int as isize)
                        .offset(
                            (if need_map != 0 {
                                (32 as usize)
                                    .wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize)
                            } else {
                                0 as usize
                            }) as isize,
                        ) as *mut ::core::ffi::c_void,
                    rest as *const ::core::ffi::c_void,
                    rest_len
                        .wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
                );
                let fresh40 = code;
                code = code.offset(1);
                *fresh40 = OP_XCLASS as ::core::ffi::c_int as PCRE2_UCHAR8;
                *code.offset(0 as ::core::ffi::c_int as isize) =
                    (required_len_0 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                        as PCRE2_UCHAR8;
                *code.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize) =
                    (required_len_0 as ::core::ffi::c_int & 255 as ::core::ffi::c_int)
                        as PCRE2_UCHAR8;
                code = code.offset(LINK_SIZE as isize);
                let fresh41 = code;
                code = code.offset(1);
                *fresh41 = (flags as ::core::ffi::c_int
                    | (if need_map != 0 {
                        XCL_MAP
                    } else {
                        0 as ::core::ffi::c_int
                    })) as PCRE2_UCHAR8;
                if need_map != 0 {
                    memcpy(
                        code as *mut ::core::ffi::c_void,
                        &raw mut op_info.bits.classbits as *mut uint8_t
                            as *const ::core::ffi::c_void,
                        32 as size_t,
                    );
                    code = code.offset(
                        (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize)
                            as isize,
                    );
                }
                code = code.offset(rest_len as isize);
            }
        }
    } else {
        let mut need_map_0: BOOL = context.needs_bitmap;
        let mut required_len_1: size_t = ((1 as ::core::ffi::c_int
            + LINK_SIZE
            + 1 as ::core::ffi::c_int) as size_t)
            .wrapping_add(
                (if need_map_0 != 0 {
                    (32 as size_t).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as size_t)
                } else {
                    0 as size_t
                }),
            )
            .wrapping_add(op_info.length);
        if !lengthptr.is_null() {
            if required_len_1 > (*lengthptr).wrapping_sub(previous_length) {
                *lengthptr = previous_length.wrapping_add(required_len_1);
            }
            *lengthptr = (*lengthptr).wrapping_sub(
                (1 as ::core::ffi::c_int + LINK_SIZE + 1 as ::core::ffi::c_int) as size_t,
            );
            let fresh42 = code;
            code = code.offset(1);
            *fresh42 = OP_ECLASS as ::core::ffi::c_int as PCRE2_UCHAR8;
            *code.offset(0 as ::core::ffi::c_int as isize) =
                (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int + 1 as ::core::ffi::c_int
                    >> 8 as ::core::ffi::c_int) as PCRE2_UCHAR8;
            *code.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize) =
                (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int + 1 as ::core::ffi::c_int
                    & 255 as ::core::ffi::c_int) as PCRE2_UCHAR8;
            code = code.offset(LINK_SIZE as isize);
            let fresh43 = code;
            code = code.offset(1);
            *fresh43 = 0 as PCRE2_UCHAR8;
        } else {
            if need_map_0 != 0 {
                let mut map_start: *mut PCRE2_UCHAR8 = previous
                    .offset(1 as ::core::ffi::c_int as isize)
                    .offset(LINK_SIZE as isize)
                    .offset(1 as ::core::ffi::c_int as isize);
                let ref mut fresh44 =
                    *previous.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                *fresh44 = (*fresh44 as ::core::ffi::c_int | ECL_MAP) as PCRE2_UCHAR8;
                memmove(
                    map_start.offset(
                        (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize)
                            as isize,
                    ) as *mut ::core::ffi::c_void,
                    map_start as *const ::core::ffi::c_void,
                    (code.offset_from(map_start) as isize
                        * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as isize)
                        as size_t,
                );
                memcpy(
                    map_start as *mut ::core::ffi::c_void,
                    &raw mut op_info.bits.classbits as *mut uint8_t as *const ::core::ffi::c_void,
                    32 as size_t,
                );
                code = code.offset(
                    (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize)
                        as isize,
                );
            }
            *previous.offset(1 as ::core::ffi::c_int as isize) =
                (code.offset_from(previous) as isize as ::core::ffi::c_int
                    >> 8 as ::core::ffi::c_int) as PCRE2_UCHAR8;
            *previous.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize) =
                (code.offset_from(previous) as isize as ::core::ffi::c_int
                    & 255 as ::core::ffi::c_int) as PCRE2_UCHAR8;
        }
    }
    *pcode = code;
    return TRUE;
}
pub const PCRE2_CODE_UNIT_WIDTH: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const LINK_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
