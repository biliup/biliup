// Translated from PCRE2 10.49 pcre2_substitute.c; code unit width 16.
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
    fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn pcre2_get_ovector_count_16(_: *mut pcre2_match_data_16) -> uint32_t;
    fn pcre2_substring_nametable_scan_16(
        _: *const pcre2_code_16,
        _: PCRE2_SPTR16,
        _: *mut PCRE2_SPTR16,
        _: *mut PCRE2_SPTR16,
    ) -> ::core::ffi::c_int;
    fn pcre2_get_ovector_pointer_16(_: *mut pcre2_match_data_16) -> *mut size_t;
    fn pcre2_next_match_16(
        _: *mut pcre2_match_data_16,
        _: *mut size_t,
        _: *mut uint32_t,
    ) -> ::core::ffi::c_int;
    fn pcre2_match_data_create_16(
        _: uint32_t,
        _: *mut pcre2_general_context_16,
    ) -> *mut pcre2_match_data_16;
    fn pcre2_substring_length_bynumber_16(
        _: *mut pcre2_match_data_16,
        _: uint32_t,
        _: *mut size_t,
    ) -> ::core::ffi::c_int;
    fn pcre2_match_data_create_from_pattern_16(
        _: *const pcre2_code_16,
        _: *mut pcre2_general_context_16,
    ) -> *mut pcre2_match_data_16;
    fn pcre2_match_data_free_16(_: *mut pcre2_match_data_16);
    fn pcre2_match_16(
        _: *const pcre2_code_16,
        _: PCRE2_SPTR16,
        _: size_t,
        _: size_t,
        _: uint32_t,
        _: *mut pcre2_match_data_16,
        _: *mut pcre2_match_context_16,
    ) -> ::core::ffi::c_int;
    fn pcre2_get_mark_16(_: *mut pcre2_match_data_16) -> PCRE2_SPTR16;
    static _pcre2_ucd_records_16: [ucd_record; 0];
    static _pcre2_ucd_stage1_16: [uint16_t; 0];
    static _pcre2_ucd_stage2_16: [uint16_t; 0];
    static _pcre2_ucp_gentype_16: [uint32_t; 0];
    fn _pcre2_check_escape_16(
        _: *mut PCRE2_SPTR16,
        _: PCRE2_SPTR16,
        _: *mut uint32_t,
        _: *mut ::core::ffi::c_int,
        _: uint32_t,
        _: uint32_t,
        _: uint32_t,
        _: BOOL,
        _: *mut compile_block_16,
    ) -> ::core::ffi::c_int;
    fn _pcre2_ord2utf_16(_: uint32_t, _: *mut PCRE2_UCHAR16) -> ::core::ffi::c_uint;
    fn _pcre2_strcmp_c8_16(_: PCRE2_SPTR16, _: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn _pcre2_strlen_16(_: PCRE2_SPTR16) -> size_t;
    fn _pcre2_valid_utf_16(_: PCRE2_SPTR16, _: size_t, _: *mut size_t) -> ::core::ffi::c_int;
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
pub struct pcre2_real_general_context_16 {
    pub memctl: pcre2_memctl,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_memctl {
    pub malloc:
        Option<unsafe extern "C" fn(size_t, *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void>,
    pub free:
        Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> ()>,
    pub memory_data: *mut ::core::ffi::c_void,
}
pub type pcre2_general_context_16 = pcre2_real_general_context_16;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_real_match_context_16 {
    pub memctl: pcre2_memctl,
    pub jit_callback: pcre2_jit_callback_16,
    pub jit_callback_data: *mut ::core::ffi::c_void,
    pub callout: Option<
        unsafe extern "C" fn(
            *mut pcre2_callout_block_16,
            *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
    pub callout_data: *mut ::core::ffi::c_void,
    pub substitute_callout: Option<
        unsafe extern "C" fn(
            *mut pcre2_substitute_callout_block_16,
            *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
    pub substitute_callout_data: *mut ::core::ffi::c_void,
    pub substitute_case_callout: Option<
        unsafe extern "C" fn(
            PCRE2_SPTR16,
            size_t,
            *mut PCRE2_UCHAR16,
            size_t,
            ::core::ffi::c_int,
            *mut ::core::ffi::c_void,
        ) -> size_t,
    >,
    pub substitute_case_callout_data: *mut ::core::ffi::c_void,
    pub offset_limit: size_t,
    pub heap_limit: uint32_t,
    pub match_limit: uint32_t,
    pub depth_limit: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_substitute_callout_block_16 {
    pub version: uint32_t,
    pub input: PCRE2_SPTR16,
    pub output: PCRE2_SPTR16,
    pub output_offsets: [size_t; 2],
    pub ovector: *mut size_t,
    pub oveccount: uint32_t,
    pub subscount: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_callout_block_16 {
    pub version: uint32_t,
    pub callout_number: uint32_t,
    pub capture_top: uint32_t,
    pub capture_last: uint32_t,
    pub offset_vector: *mut size_t,
    pub mark: PCRE2_SPTR16,
    pub subject: PCRE2_SPTR16,
    pub subject_length: size_t,
    pub start_match: size_t,
    pub current_position: size_t,
    pub pattern_position: size_t,
    pub next_item_length: size_t,
    pub callout_string_offset: size_t,
    pub callout_string_length: size_t,
    pub callout_string: PCRE2_SPTR16,
    pub callout_flags: uint32_t,
}
pub type pcre2_jit_callback_16 =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> *mut pcre2_jit_stack_16>;
pub type pcre2_jit_stack_16 = pcre2_real_jit_stack_16;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_real_jit_stack_16 {
    pub memctl: pcre2_memctl,
    pub stack: *mut ::core::ffi::c_void,
}
pub type pcre2_match_context_16 = pcre2_real_match_context_16;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_real_code_16 {
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
pub type pcre2_code_16 = pcre2_real_code_16;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_real_match_data_16 {
    pub memctl: pcre2_memctl,
    pub code: *const pcre2_real_code_16,
    pub subject: PCRE2_SPTR16,
    pub mark: PCRE2_SPTR16,
    pub heapframes: *mut heapframe,
    pub heapframes_size: size_t,
    pub subject_length: size_t,
    pub start_offset: size_t,
    pub leftchar: size_t,
    pub rightchar: size_t,
    pub startchar: size_t,
    pub matchedby: uint8_t,
    pub flags: uint8_t,
    pub oveccount: uint16_t,
    pub options: uint32_t,
    pub rc: ::core::ffi::c_int,
    pub ovector: [size_t; 131072],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct heapframe {
    pub ecode: PCRE2_SPTR16,
    pub back_frame: size_t,
    pub rdepth: uint32_t,
    pub group_frame_type: uint32_t,
    pub return_id: uint8_t,
    pub op: uint8_t,
    pub byte1: uint8_t,
    pub byte2: uint8_t,
    pub fields: C2RustUnnamed,
    pub eptr: PCRE2_SPTR16,
    pub start_match: PCRE2_SPTR16,
    pub mark: PCRE2_SPTR16,
    pub recurse_last_used: PCRE2_SPTR16,
    pub current_recurse: uint32_t,
    pub capture_last: uint32_t,
    pub last_group_offset: size_t,
    pub offset_top: size_t,
    pub ovector: [size_t; 131072],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub char_repeat: C2RustUnnamed_12,
    pub charnot_repeat: C2RustUnnamed_11,
    pub class_repeat: C2RustUnnamed_10,
    pub xclass_repeat: C2RustUnnamed_9,
    pub eclass_repeat: C2RustUnnamed_8,
    pub type_repeat: C2RustUnnamed_7,
    pub ref_repeat: C2RustUnnamed_6,
    pub op_bra: C2RustUnnamed_5,
    pub op_brapos: C2RustUnnamed_4,
    pub op_recurse: C2RustUnnamed_3,
    pub op_assert_scs: C2RustUnnamed_2,
    pub op_cond: C2RustUnnamed_1,
    pub op_vreverse: C2RustUnnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_0 {
    pub min: uint32_t,
    pub max: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_1 {
    pub start_branch: PCRE2_SPTR16,
    pub length: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_2 {
    pub saved_end_subject: PCRE2_SPTR16,
    pub saved_eptr: PCRE2_SPTR16,
    pub true_end_extra: size_t,
    pub saved_moptions: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_3 {
    pub start_branch: PCRE2_SPTR16,
    pub frame_type: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_4 {
    pub start_eptr: PCRE2_SPTR16,
    pub start_group: PCRE2_SPTR16,
    pub frame_type: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_5 {
    pub frame_type: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_6 {
    pub start: PCRE2_SPTR16,
    pub offset: size_t,
    pub length: size_t,
    pub min: uint32_t,
    pub max: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_7 {
    pub start_eptr: PCRE2_SPTR16,
    pub min: uint32_t,
    pub max: uint32_t,
    pub ctype: uint32_t,
    pub propvalue: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_8 {
    pub start_eptr: PCRE2_SPTR16,
    pub eclass_data: PCRE2_SPTR16,
    pub eclass_len: size_t,
    pub min: uint32_t,
    pub max: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_9 {
    pub start_eptr: PCRE2_SPTR16,
    pub xclass_data: PCRE2_SPTR16,
    pub min: uint32_t,
    pub max: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_10 {
    pub start_eptr: PCRE2_SPTR16,
    pub byte_map_address: PCRE2_SPTR16,
    pub min: uint32_t,
    pub max: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_11 {
    pub start_eptr: PCRE2_SPTR16,
    pub min: uint32_t,
    pub max: uint32_t,
    pub c: uint32_t,
    pub oc: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_12 {
    pub start_eptr: PCRE2_SPTR16,
    pub charptr: PCRE2_SPTR16,
    pub min: uint32_t,
    pub max: uint32_t,
    pub c: uint32_t,
    pub oc: C2RustUnnamed_13,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_13 {
    pub oc: uint32_t,
    pub occu: [PCRE2_UCHAR16; 2],
}
pub type pcre2_match_data_16 = pcre2_real_match_data_16;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct case_state {
    pub to_case: ::core::ffi::c_int,
    pub single_char: BOOL,
}
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
pub const ucp_Ll: C2RustUnnamed_15 = 5;
pub const ucp_Lu: C2RustUnnamed_15 = 9;
pub const ucp_L: C2RustUnnamed_14 = 1;
pub const ucp_Nd: C2RustUnnamed_15 = 13;
pub const ESC_g: C2RustUnnamed_17 = 27;
pub const ESC_v: C2RustUnnamed_17 = 21;
pub const ESC_b: C2RustUnnamed_17 = 5;
pub const ESC_Q: C2RustUnnamed_17 = 26;
pub const ESC_E: C2RustUnnamed_17 = 25;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct compile_data {
    pub next: *mut compile_data,
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
pub union class_bits_storage {
    pub classbits: [uint8_t; 32],
    pub classwords: [uint32_t; 8],
}
pub const PCRE2_MATCHEDBY_DFA_INTERPRETER: C2RustUnnamed_16 = 1;
pub type C2RustUnnamed_14 = ::core::ffi::c_uint;
pub const ucp_Z: C2RustUnnamed_14 = 6;
pub const ucp_S: C2RustUnnamed_14 = 5;
pub const ucp_P: C2RustUnnamed_14 = 4;
pub const ucp_N: C2RustUnnamed_14 = 3;
pub const ucp_M: C2RustUnnamed_14 = 2;
pub const ucp_C: C2RustUnnamed_14 = 0;
pub type C2RustUnnamed_15 = ::core::ffi::c_uint;
pub const ucp_Zs: C2RustUnnamed_15 = 29;
pub const ucp_Zp: C2RustUnnamed_15 = 28;
pub const ucp_Zl: C2RustUnnamed_15 = 27;
pub const ucp_So: C2RustUnnamed_15 = 26;
pub const ucp_Sm: C2RustUnnamed_15 = 25;
pub const ucp_Sk: C2RustUnnamed_15 = 24;
pub const ucp_Sc: C2RustUnnamed_15 = 23;
pub const ucp_Ps: C2RustUnnamed_15 = 22;
pub const ucp_Po: C2RustUnnamed_15 = 21;
pub const ucp_Pi: C2RustUnnamed_15 = 20;
pub const ucp_Pf: C2RustUnnamed_15 = 19;
pub const ucp_Pe: C2RustUnnamed_15 = 18;
pub const ucp_Pd: C2RustUnnamed_15 = 17;
pub const ucp_Pc: C2RustUnnamed_15 = 16;
pub const ucp_No: C2RustUnnamed_15 = 15;
pub const ucp_Nl: C2RustUnnamed_15 = 14;
pub const ucp_Mn: C2RustUnnamed_15 = 12;
pub const ucp_Me: C2RustUnnamed_15 = 11;
pub const ucp_Mc: C2RustUnnamed_15 = 10;
pub const ucp_Lt: C2RustUnnamed_15 = 8;
pub const ucp_Lo: C2RustUnnamed_15 = 7;
pub const ucp_Lm: C2RustUnnamed_15 = 6;
pub const ucp_Cs: C2RustUnnamed_15 = 4;
pub const ucp_Co: C2RustUnnamed_15 = 3;
pub const ucp_Cn: C2RustUnnamed_15 = 2;
pub const ucp_Cf: C2RustUnnamed_15 = 1;
pub const ucp_Cc: C2RustUnnamed_15 = 0;
pub type C2RustUnnamed_16 = ::core::ffi::c_uint;
pub const PCRE2_MATCHEDBY_JIT: C2RustUnnamed_16 = 2;
pub const PCRE2_MATCHEDBY_INTERPRETER: C2RustUnnamed_16 = 0;
pub type C2RustUnnamed_17 = ::core::ffi::c_uint;
pub const ESC_ub: C2RustUnnamed_17 = 29;
pub const ESC_k: C2RustUnnamed_17 = 28;
pub const ESC_z: C2RustUnnamed_17 = 24;
pub const ESC_Z: C2RustUnnamed_17 = 23;
pub const ESC_X: C2RustUnnamed_17 = 22;
pub const ESC_V: C2RustUnnamed_17 = 20;
pub const ESC_h: C2RustUnnamed_17 = 19;
pub const ESC_H: C2RustUnnamed_17 = 18;
pub const ESC_R: C2RustUnnamed_17 = 17;
pub const ESC_p: C2RustUnnamed_17 = 16;
pub const ESC_P: C2RustUnnamed_17 = 15;
pub const ESC_C: C2RustUnnamed_17 = 14;
pub const ESC_dum: C2RustUnnamed_17 = 13;
pub const ESC_N: C2RustUnnamed_17 = 12;
pub const ESC_w: C2RustUnnamed_17 = 11;
pub const ESC_W: C2RustUnnamed_17 = 10;
pub const ESC_s: C2RustUnnamed_17 = 9;
pub const ESC_S: C2RustUnnamed_17 = 8;
pub const ESC_d: C2RustUnnamed_17 = 7;
pub const ESC_D: C2RustUnnamed_17 = 6;
pub const ESC_B: C2RustUnnamed_17 = 4;
pub const ESC_K: C2RustUnnamed_17 = 3;
pub const ESC_G: C2RustUnnamed_17 = 2;
pub const ESC_A: C2RustUnnamed_17 = 1;
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PCRE2_NO_UTF_CHECK: ::core::ffi::c_uint = 0x40000000 as ::core::ffi::c_uint;
pub const PCRE2_UCP: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PCRE2_UTF: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PCRE2_PARTIAL_SOFT: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PCRE2_PARTIAL_HARD: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PCRE2_SUBSTITUTE_GLOBAL: ::core::ffi::c_uint = 0x100 as ::core::ffi::c_uint;
pub const PCRE2_SUBSTITUTE_EXTENDED: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PCRE2_SUBSTITUTE_UNSET_EMPTY: ::core::ffi::c_uint = 0x400 as ::core::ffi::c_uint;
pub const PCRE2_SUBSTITUTE_UNKNOWN_UNSET: ::core::ffi::c_uint = 0x800 as ::core::ffi::c_uint;
pub const PCRE2_SUBSTITUTE_OVERFLOW_LENGTH: ::core::ffi::c_uint = 0x1000 as ::core::ffi::c_uint;
pub const PCRE2_COPY_MATCHED_SUBJECT: ::core::ffi::c_uint = 0x4000 as ::core::ffi::c_uint;
pub const PCRE2_SUBSTITUTE_LITERAL: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PCRE2_SUBSTITUTE_MATCHED: ::core::ffi::c_uint = 0x10000 as ::core::ffi::c_uint;
pub const PCRE2_SUBSTITUTE_REPLACEMENT_ONLY: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PCRE2_ERROR_NOMATCH: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADOFFSET: ::core::ffi::c_int = -(33 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADOPTION: ::core::ffi::c_int = -(34 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADREPLACEMENT: ::core::ffi::c_int = -(35 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DFA_UFUNC: ::core::ffi::c_int = -(41 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NOMEMORY: ::core::ffi::c_int = -(48 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NOSUBSTRING: ::core::ffi::c_int = -(49 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NULL: ::core::ffi::c_int = -(51 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UNAVAILABLE: ::core::ffi::c_int = -(54 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UNSET: ::core::ffi::c_int = -(55 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADREPESCAPE: ::core::ffi::c_int = -(57 as ::core::ffi::c_int);
pub const PCRE2_ERROR_REPMISSINGBRACE: ::core::ffi::c_int = -(58 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADSUBSTITUTION: ::core::ffi::c_int = -(59 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADSUBSPATTERN: ::core::ffi::c_int = -(60 as ::core::ffi::c_int);
pub const PCRE2_ERROR_TOOMANYREPLACE: ::core::ffi::c_int = -(61 as ::core::ffi::c_int);
pub const PCRE2_ERROR_INTERNAL_DUPMATCH: ::core::ffi::c_int = -(65 as ::core::ffi::c_int);
pub const PCRE2_ERROR_REPLACECASE: ::core::ffi::c_int = -(69 as ::core::ffi::c_int);
pub const PCRE2_ERROR_TOOLARGEREPLACE: ::core::ffi::c_int = -(70 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DIFFSUBSPATTERN: ::core::ffi::c_int = -(71 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DIFFSUBSSUBJECT: ::core::ffi::c_int = -(72 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DIFFSUBSOFFSET: ::core::ffi::c_int = -(73 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DIFFSUBSOPTIONS: ::core::ffi::c_int = -(74 as ::core::ffi::c_int);
pub const PCRE2_ERROR_PARTIALSUBS: ::core::ffi::c_int = -(76 as ::core::ffi::c_int);
pub const PCRE2_SUBSTITUTE_CASE_LOWER: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PCRE2_SUBSTITUTE_CASE_UPPER: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PCRE2_SUBSTITUTE_CASE_TITLE_FIRST: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const PCRE2_ZERO_TERMINATED: size_t = !(0 as ::core::ffi::c_int as size_t);
pub const PCRE2_UNSET: size_t = !(0 as ::core::ffi::c_int as size_t);
pub const PCRE2_MD_COPIED_SUBJECT: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const cbit_upper: ::core::ffi::c_int = 96 as ::core::ffi::c_int;
pub const cbit_lower: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const cbit_length: ::core::ffi::c_int = 320 as ::core::ffi::c_int;
pub const ctype_word: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const fcc_offset: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const cbits_offset: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
pub const ctypes_offset: ::core::ffi::c_int = cbits_offset + cbit_length;
pub const CHAR_VT: ::core::ffi::c_int = '\u{b}' as i32;
pub const CHAR_BS: ::core::ffi::c_int = '\u{8}' as i32;
pub const CHAR_DOLLAR_SIGN: ::core::ffi::c_int = '$' as i32;
pub const CHAR_AMPERSAND: ::core::ffi::c_int = '&' as i32;
pub const CHAR_APOSTROPHE: ::core::ffi::c_int = '\'' as i32;
pub const CHAR_ASTERISK: ::core::ffi::c_int = '*' as i32;
pub const CHAR_PLUS: ::core::ffi::c_int = '+' as i32;
pub const CHAR_MINUS: ::core::ffi::c_int = '-' as i32;
pub const CHAR_0: ::core::ffi::c_int = '0' as i32;
pub const CHAR_9: ::core::ffi::c_int = '9' as i32;
pub const CHAR_COLON: ::core::ffi::c_int = ':' as i32;
pub const CHAR_LESS_THAN_SIGN: ::core::ffi::c_int = '<' as i32;
pub const CHAR_GREATER_THAN_SIGN: ::core::ffi::c_int = '>' as i32;
pub const CHAR_E: ::core::ffi::c_int = 'E' as i32;
pub const CHAR_L: ::core::ffi::c_int = 'L' as i32;
pub const CHAR_U: ::core::ffi::c_int = 85;
pub const CHAR_BACKSLASH: ::core::ffi::c_int = '\\' as i32;
pub const CHAR_UNDERSCORE: ::core::ffi::c_int = '_' as i32;
pub const CHAR_GRAVE_ACCENT: ::core::ffi::c_int = '`' as i32;
pub const CHAR_l: ::core::ffi::c_int = 108;
pub const CHAR_u: ::core::ffi::c_int = 117;
pub const CHAR_LEFT_CURLY_BRACKET: ::core::ffi::c_int = '{' as i32;
pub const CHAR_RIGHT_CURLY_BRACKET: ::core::ffi::c_int = '}' as i32;
pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const PTR_STACK_SIZE: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const SUBSTITUTE_OPTIONS: ::core::ffi::c_uint = PCRE2_SUBSTITUTE_EXTENDED
    | PCRE2_SUBSTITUTE_GLOBAL
    | PCRE2_SUBSTITUTE_LITERAL
    | PCRE2_SUBSTITUTE_MATCHED
    | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
    | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY
    | PCRE2_SUBSTITUTE_UNKNOWN_UNSET
    | PCRE2_SUBSTITUTE_UNSET_EMPTY;
unsafe extern "C" fn find_text_end(
    mut code: *const pcre2_code_16,
    mut ptrptr: *mut PCRE2_SPTR16,
    mut ptrend: PCRE2_SPTR16,
    mut last: BOOL,
) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut rc: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut nestlevel: uint32_t = 0 as uint32_t;
    let mut literal: BOOL = FALSE;
    let mut ptr: PCRE2_SPTR16 = *ptrptr;
    loop {
        if !(ptr < ptrend) {
            current_block = 1836292691772056875;
            break;
        }
        if literal != 0 {
            if *ptr.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == CHAR_BACKSLASH
                && ptr < ptrend.offset(-(1 as ::core::ffi::c_int as isize))
                && *ptr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == CHAR_E
            {
                literal = FALSE as BOOL;
                ptr = ptr.offset(1 as ::core::ffi::c_int as isize);
            }
        } else if *ptr as ::core::ffi::c_int == CHAR_RIGHT_CURLY_BRACKET {
            if nestlevel == 0 as uint32_t {
                current_block = 7151867007581078410;
                break;
            }
            nestlevel = nestlevel.wrapping_sub(1);
        } else {
            if *ptr as ::core::ffi::c_int == CHAR_COLON && last == 0 && nestlevel == 0 as uint32_t {
                current_block = 7151867007581078410;
                break;
            }
            if *ptr as ::core::ffi::c_int == CHAR_DOLLAR_SIGN {
                if ptr < ptrend.offset(-(1 as ::core::ffi::c_int as isize))
                    && *ptr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == CHAR_LEFT_CURLY_BRACKET
                {
                    nestlevel = nestlevel.wrapping_add(1);
                    ptr = ptr.offset(1 as ::core::ffi::c_int as isize);
                }
            } else if *ptr as ::core::ffi::c_int == CHAR_BACKSLASH {
                let mut erc: ::core::ffi::c_int = 0;
                let mut errorcode: ::core::ffi::c_int = 0;
                let mut ch: uint32_t = 0;
                let mut esc_end_ptr: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
                if ptr < ptrend.offset(-(1 as ::core::ffi::c_int as isize)) {
                    match *ptr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int {
                        CHAR_L | CHAR_l | CHAR_U | CHAR_u => {
                            ptr = ptr.offset(1 as ::core::ffi::c_int as isize);
                            current_block = 8258075665625361029;
                        }
                        _ => {
                            current_block = 15089075282327824602;
                        }
                    }
                } else {
                    current_block = 15089075282327824602;
                }
                match current_block {
                    8258075665625361029 => {}
                    _ => {
                        ptr = ptr.offset(1 as ::core::ffi::c_int as isize);
                        erc = _pcre2_check_escape_16(
                            &raw mut ptr,
                            ptrend,
                            &raw mut ch,
                            &raw mut errorcode,
                            (*code).overall_options,
                            (*code).extra_options,
                            (*code).top_bracket as uint32_t,
                            FALSE,
                            ::core::ptr::null_mut::<compile_block_16>(),
                        );
                        if errorcode != 0 as ::core::ffi::c_int {
                            rc = PCRE2_ERROR_BADREPESCAPE;
                            current_block = 7151867007581078410;
                            break;
                        } else {
                            esc_end_ptr = ptr;
                            ptr = ptr.offset(-(1 as ::core::ffi::c_int as isize));
                            match erc {
                                26 => {
                                    current_block = 5252126906147815065;
                                    match current_block {
                                        5252126906147815065 => {
                                            literal = TRUE as BOOL;
                                        }
                                        _ => {
                                            if !(erc < 0 as ::core::ffi::c_int) {
                                                ptr = esc_end_ptr;
                                                rc = PCRE2_ERROR_BADREPESCAPE;
                                                current_block = 7151867007581078410;
                                                break;
                                            }
                                        }
                                    }
                                }
                                0 | 5 | 21 | 25 | 27 => {}
                                _ => {
                                    current_block = 15203473443888960181;
                                    match current_block {
                                        5252126906147815065 => {
                                            literal = TRUE as BOOL;
                                        }
                                        _ => {
                                            if !(erc < 0 as ::core::ffi::c_int) {
                                                ptr = esc_end_ptr;
                                                rc = PCRE2_ERROR_BADREPESCAPE;
                                                current_block = 7151867007581078410;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        ptr = ptr.offset(1);
    }
    match current_block {
        1836292691772056875 => {
            rc = PCRE2_ERROR_REPMISSINGBRACE;
        }
        _ => {}
    }
    *ptrptr = ptr;
    return rc;
}
unsafe extern "C" fn read_name_subst(
    mut ptrptr: *mut PCRE2_SPTR16,
    mut ptrend: PCRE2_SPTR16,
    mut utf: BOOL,
    mut ctypes: *const uint8_t,
) -> BOOL {
    let mut ptr: PCRE2_SPTR16 = *ptrptr;
    let mut nameptr: PCRE2_SPTR16 = ptr;
    if !(ptr >= ptrend) {
        if utf != 0 {
            let mut c: uint32_t = 0;
            let mut type_0: uint32_t = 0;
            while ptr < ptrend {
                c = *ptr as uint32_t;
                if c & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                    c = ((c & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                        | *ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3ff as uint32_t)
                        .wrapping_add(0x10000 as uint32_t);
                }
                type_0 = (*(&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
                    *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                        (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                            .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                            as ::core::ffi::c_int
                            * UCD_BLOCK_SIZE
                            + c as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                            as isize,
                    ) as ::core::ffi::c_int as isize,
                ))
                .chartype as uint32_t;
                if type_0 != ucp_Nd as ::core::ffi::c_int as uint32_t
                    && *(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                        .offset(type_0 as isize)
                        != ucp_L as ::core::ffi::c_int as uint32_t
                    && c != CHAR_UNDERSCORE as uint32_t
                {
                    break;
                }
                ptr = ptr.offset(1);
                if ptr < ptrend
                    && *ptr as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                        == 0xdc00 as ::core::ffi::c_uint
                {
                    ptr = ptr.offset(1);
                }
            }
        } else {
            while ptr < ptrend
                && *ptr as ::core::ffi::c_uint <= 255 as ::core::ffi::c_uint
                && *ctypes.offset(*ptr as isize) as ::core::ffi::c_int & ctype_word
                    != 0 as ::core::ffi::c_int
            {
                ptr = ptr.offset(1);
            }
        }
        if !(ptr.offset_from(nameptr) as isize > MAX_NAME_SIZE as isize)
        {
            if !(ptr == nameptr) {
                *ptrptr = ptr;
                return TRUE;
            }
        }
    }
    *ptrptr = ptr;
    return FALSE;
}
pub const PCRE2_SUBSTITUTE_CASE_NONE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST: ::core::ffi::c_int = 4;
unsafe extern "C" fn pessimistic_case_inflation(mut len: size_t) -> size_t {
    return (len >> 3 as ::core::ffi::c_uint).wrapping_add(10 as size_t);
}
unsafe extern "C" fn default_substitute_case_callout(
    mut input: PCRE2_SPTR16,
    mut input_len: size_t,
    mut output: *mut PCRE2_UCHAR16,
    mut output_cap: size_t,
    mut state: *mut case_state,
    mut code: *const pcre2_code_16,
) -> size_t {
    let mut input_end: PCRE2_SPTR16 = input.offset(input_len as isize);
    let mut utf: BOOL = 0;
    let mut ucp: BOOL = 0;
    let mut temp: [PCRE2_UCHAR16; 6] = [0; 6];
    let mut next_to_upper: BOOL = 0;
    let mut rest_to_upper: BOOL = 0;
    let mut single_char: BOOL = 0;
    let mut overflow: BOOL = FALSE;
    let mut written: size_t = 0 as size_t;
    utf = ((*code).overall_options & PCRE2_UTF as uint32_t != 0 as uint32_t) as ::core::ffi::c_int
        as BOOL;
    ucp = ((*code).overall_options & PCRE2_UCP as uint32_t != 0 as uint32_t) as ::core::ffi::c_int
        as BOOL;
    if input_len == 0 as size_t {
        return 0 as size_t;
    }
    match (*state).to_case {
        PCRE2_SUBSTITUTE_CASE_LOWER | PCRE2_SUBSTITUTE_CASE_UPPER => {
            rest_to_upper =
                ((*state).to_case == PCRE2_SUBSTITUTE_CASE_UPPER) as ::core::ffi::c_int as BOOL;
            next_to_upper = rest_to_upper;
        }
        PCRE2_SUBSTITUTE_CASE_TITLE_FIRST => {
            next_to_upper = TRUE as BOOL;
            rest_to_upper = FALSE as BOOL;
            (*state).to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
        }
        PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST => {
            next_to_upper = FALSE as BOOL;
            rest_to_upper = TRUE as BOOL;
            (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
        }
        _ => return 0 as size_t,
    }
    single_char = (*state).single_char;
    if single_char != 0 {
        (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE;
    }
    while input < input_end {
        let mut ch: uint32_t = 0;
        let mut chlen: ::core::ffi::c_uint = 0;
        let fresh6 = input;
        input = input.offset(1);
        ch = *fresh6 as uint32_t;
        if utf != 0 && ch & 0xfc00 as uint32_t == 0xd800 as uint32_t {
            let fresh7 = input;
            input = input.offset(1);
            ch = ((ch & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                | *fresh7 as uint32_t & 0x3ff as uint32_t)
                .wrapping_add(0x10000 as uint32_t);
        }
        if (utf != 0 || ucp != 0) && ch >= 128 as uint32_t {
            let mut type_0: uint32_t = (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                .offset(
                    *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                        (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                            .offset((ch as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                            as ::core::ffi::c_int
                            * UCD_BLOCK_SIZE
                            + ch as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                            as isize,
                    ) as ::core::ffi::c_int as isize,
                ))
            .chartype as uint32_t;
            if *(&raw const _pcre2_ucp_gentype_16 as *const uint32_t).offset(type_0 as isize)
                == ucp_L as ::core::ffi::c_int as uint32_t
                && type_0
                    != (if next_to_upper != 0 {
                        ucp_Lu as ::core::ffi::c_int
                    } else {
                        ucp_Ll as ::core::ffi::c_int
                    }) as uint32_t
            {
                ch = (ch as ::core::ffi::c_int
                    + (*(&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
                        *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                            (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                .offset((ch as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                                as ::core::ffi::c_int
                                * UCD_BLOCK_SIZE
                                + ch as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                as isize,
                        ) as ::core::ffi::c_int as isize,
                    ))
                    .other_case as ::core::ffi::c_int) as uint32_t;
            }
        } else if ch <= 255 as uint32_t {
            if *(*code)
                .tables
                .offset(cbits_offset as isize)
                .offset(
                    (if next_to_upper != 0 {
                        cbit_upper
                    } else {
                        cbit_lower
                    }) as isize,
                )
                .offset(ch.wrapping_div(8 as uint32_t) as isize)
                as ::core::ffi::c_uint
                & (1 as ::core::ffi::c_uint) << ch.wrapping_rem(8 as uint32_t)
                == 0 as ::core::ffi::c_uint
            {
                ch = *(*code)
                    .tables
                    .offset(fcc_offset as isize)
                    .offset(ch as isize) as uint32_t;
            }
        }
        if utf != 0 {
            chlen = _pcre2_ord2utf_16(ch, &raw mut temp as *mut PCRE2_UCHAR16);
        } else {
            temp[0 as ::core::ffi::c_int as usize] = ch as PCRE2_UCHAR16;
            chlen = 1 as ::core::ffi::c_uint;
        }
        if overflow == 0 && chlen as size_t <= output_cap {
            memcpy(
                output as *mut ::core::ffi::c_void,
                &raw mut temp as *mut PCRE2_UCHAR16 as *const ::core::ffi::c_void,
                chlen.wrapping_mul(
                    (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as ::core::ffi::c_uint,
                ) as size_t,
            );
            output = output.offset(chlen as isize);
            output_cap = output_cap.wrapping_sub(chlen as size_t);
        } else {
            overflow = TRUE as BOOL;
        }
        if chlen as size_t > (!(0 as ::core::ffi::c_int as size_t)).wrapping_sub(written) {
            return !(0 as ::core::ffi::c_int as size_t);
        }
        written = written.wrapping_add(chlen as size_t);
        next_to_upper = rest_to_upper;
        if single_char != 0 {
            let mut rest_len: size_t =
                input_end.offset_from(input) as isize as size_t;
            if overflow == 0 && rest_len <= output_cap {
                memcpy(
                    output as *mut ::core::ffi::c_void,
                    input as *const ::core::ffi::c_void,
                    rest_len
                        .wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
                );
            }
            if rest_len > (!(0 as ::core::ffi::c_int as size_t)).wrapping_sub(written) {
                return !(0 as ::core::ffi::c_int as size_t);
            }
            written = written.wrapping_add(rest_len);
            return written;
        }
    }
    return written;
}
unsafe extern "C" fn do_case_copy(
    mut input_output: *mut PCRE2_UCHAR16,
    mut input_len: size_t,
    mut output_cap: size_t,
    mut state: *mut case_state,
    mut utf: BOOL,
    mut substitute_case_callout: Option<
        unsafe extern "C" fn(
            PCRE2_SPTR16,
            size_t,
            *mut PCRE2_UCHAR16,
            size_t,
            ::core::ffi::c_int,
            *mut ::core::ffi::c_void,
        ) -> size_t,
    >,
    mut substitute_case_callout_data: *mut ::core::ffi::c_void,
) -> size_t {
    let mut input: PCRE2_SPTR16 = input_output as PCRE2_SPTR16;
    let mut output: *mut PCRE2_UCHAR16 = input_output;
    let mut rc: size_t = 0;
    let mut rc2: size_t = 0;
    let mut ch1_to_case: ::core::ffi::c_int = 0;
    let mut rest_to_case: ::core::ffi::c_int = 0;
    let mut ch1: [PCRE2_UCHAR16; 6] = [0; 6];
    let mut ch1_len: size_t = 0;
    let mut rest: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut rest_len: size_t = 0;
    let mut ch1_overflow: BOOL = FALSE;
    let mut rest_overflow: BOOL = FALSE;
    match (*state).to_case {
        PCRE2_SUBSTITUTE_CASE_LOWER
        | PCRE2_SUBSTITUTE_CASE_UPPER
        | PCRE2_SUBSTITUTE_CASE_TITLE_FIRST => {
            if (*state).single_char == FALSE {
                rc = substitute_case_callout.expect("non-null function pointer")(
                    input,
                    input_len,
                    output,
                    output_cap,
                    (*state).to_case,
                    substitute_case_callout_data,
                );
                if (*state).to_case == PCRE2_SUBSTITUTE_CASE_TITLE_FIRST {
                    (*state).to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
                }
                return rc;
            }
            ch1_to_case = (*state).to_case;
            rest_to_case = PCRE2_SUBSTITUTE_CASE_NONE;
        }
        PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST => {
            ch1_to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
            rest_to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
        }
        _ => return 0 as size_t,
    }
    let mut ch_end: PCRE2_SPTR16 = input;
    let mut ch: uint32_t = 0;
    let fresh4 = ch_end;
    ch_end = ch_end.offset(1);
    ch = *fresh4 as uint32_t;
    if utf != 0 && ch & 0xfc00 as uint32_t == 0xd800 as uint32_t {
        let fresh5 = ch_end;
        ch_end = ch_end.offset(1);
        ch = ((ch & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
            | *fresh5 as uint32_t & 0x3ff as uint32_t)
            .wrapping_add(0x10000 as uint32_t);
    }
    ch1_len = ch_end.offset_from(input) as isize as size_t;
    memcpy(
        &raw mut ch1 as *mut PCRE2_UCHAR16 as *mut ::core::ffi::c_void,
        input as *const ::core::ffi::c_void,
        ch1_len.wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
    );
    rest = input.offset(ch1_len as isize);
    rest_len = input_len.wrapping_sub(ch1_len);
    let mut ch1_cap: size_t = 0;
    let mut max_ch1_cap: size_t = 0;
    ch1_cap = ch1_len;
    max_ch1_cap = output_cap.wrapping_sub(rest_len);
    loop {
        rc = substitute_case_callout.expect("non-null function pointer")(
            &raw mut ch1 as *mut PCRE2_UCHAR16 as PCRE2_SPTR16,
            ch1_len,
            output,
            ch1_cap,
            ch1_to_case,
            substitute_case_callout_data,
        );
        if rc == !(0 as ::core::ffi::c_int as size_t) {
            return rc;
        }
        if rc <= ch1_cap {
            break;
        }
        if rc > max_ch1_cap {
            ch1_overflow = TRUE as BOOL;
            break;
        } else {
            memmove(
                input_output.offset(rc as isize) as *mut ::core::ffi::c_void,
                rest as *const ::core::ffi::c_void,
                rest_len.wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
            );
            rest = input.offset(rc as isize);
            ch1_cap = rc;
        }
    }
    if rest_to_case == PCRE2_SUBSTITUTE_CASE_NONE {
        if ch1_overflow == 0 {
            memmove(
                output.offset(rc as isize) as *mut ::core::ffi::c_void,
                rest as *const ::core::ffi::c_void,
                rest_len.wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
            );
        }
        rc2 = rest_len;
        (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE;
    } else {
        let mut dummy: [PCRE2_UCHAR16; 1] = [0; 1];
        rc2 = substitute_case_callout.expect("non-null function pointer")(
            rest,
            rest_len,
            if ch1_overflow != 0 {
                &raw mut dummy as *mut PCRE2_UCHAR16
            } else {
                output.offset(rc as isize)
            },
            if ch1_overflow != 0 {
                0 as size_t
            } else {
                output_cap.wrapping_sub(rc)
            },
            rest_to_case,
            substitute_case_callout_data,
        );
        if rc2 == !(0 as ::core::ffi::c_int as size_t) {
            return rc2;
        }
        if ch1_overflow == 0 && rc2 > output_cap.wrapping_sub(rc) {
            rest_overflow = TRUE as BOOL;
        }
        if ch1_overflow != 0 && rc2 < rest_len {
            rc2 = rest_len;
        }
        (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
    }
    if rc2 > (!(0 as ::core::ffi::c_int as size_t)).wrapping_sub(rc) {
        return !(0 as ::core::ffi::c_int as size_t);
    }
    return rc.wrapping_add(rc2);
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_substitute_16(
    mut code: *const pcre2_code_16,
    mut subject: PCRE2_SPTR16,
    mut length: size_t,
    mut start_offset: size_t,
    mut options: uint32_t,
    mut match_data: *mut pcre2_match_data_16,
    mut mcontext: *mut pcre2_match_context_16,
    mut replacement: PCRE2_SPTR16,
    mut rlength: size_t,
    mut buffer: *mut PCRE2_UCHAR16,
    mut blength: *mut size_t,
) -> ::core::ffi::c_int {
    let mut inparens: BOOL = 0;
    let mut inangle: BOOL = 0;
    let mut star: BOOL = 0;
    let mut sublength: size_t = 0;
    let mut next: PCRE2_UCHAR16 = 0;
    let mut subptr: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut subptrend: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut ch_start: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut current_block: u64;
    let mut rc: ::core::ffi::c_int = 0;
    let mut subs: ::core::ffi::c_int = 0;
    let mut ovector_count: uint32_t = 0;
    let mut goptions: uint32_t = 0 as uint32_t;
    let mut suboptions: uint32_t = 0;
    let mut internal_match_data: *mut pcre2_match_data_16 =
        ::core::ptr::null_mut::<pcre2_match_data_16>();
    let mut escaped_literal: BOOL = FALSE;
    let mut overflowed: BOOL = FALSE;
    let mut use_existing_match: BOOL = 0;
    let mut replacement_only: BOOL = 0;
    let mut utf: BOOL =
        ((*code).overall_options & PCRE2_UTF as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    let mut partial: BOOL = (options
        & (PCRE2_PARTIAL_HARD as uint32_t | PCRE2_PARTIAL_SOFT as uint32_t)
        != 0 as uint32_t) as ::core::ffi::c_int;
    let mut temp: [PCRE2_UCHAR16; 6] = [0; 6];
    let mut null_str: [PCRE2_UCHAR16; 1] = [0xcd as ::core::ffi::c_int as PCRE2_UCHAR16];
    let mut original_subject: PCRE2_SPTR16 = subject;
    let mut ptr: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut repend: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut extra_needed: size_t = 0 as size_t;
    let mut buff_offset: size_t = 0;
    let mut buff_length: size_t = 0;
    let mut lengthleft: size_t = 0;
    let mut fraglength: size_t = 0;
    let mut ovector: *mut size_t = ::core::ptr::null_mut::<size_t>();
    let mut ovecsave: [size_t; 2] = [
        0 as ::core::ffi::c_int as size_t,
        0 as ::core::ffi::c_int as size_t,
    ];
    let mut scb: pcre2_substitute_callout_block_16 = pcre2_substitute_callout_block_16 {
        version: 0,
        input: ::core::ptr::null::<PCRE2_UCHAR16>(),
        output: ::core::ptr::null::<PCRE2_UCHAR16>(),
        output_offsets: [0; 2],
        ovector: ::core::ptr::null_mut::<size_t>(),
        oveccount: 0,
        subscount: 0,
    };
    let mut sub_start_extra_needed: size_t = 0;
    let mut substitute_case_callout: Option<
        unsafe extern "C" fn(
            PCRE2_SPTR16,
            size_t,
            *mut PCRE2_UCHAR16,
            size_t,
            ::core::ffi::c_int,
            *mut ::core::ffi::c_void,
        ) -> size_t,
    > = None;
    let mut substitute_case_callout_data: *mut ::core::ffi::c_void = NULL;
    buff_offset = 0 as size_t;
    buff_length = *blength;
    lengthleft = buff_length;
    *blength = PCRE2_UNSET;
    if !mcontext.is_null() {
        substitute_case_callout = (*mcontext).substitute_case_callout;
        substitute_case_callout_data = (*mcontext).substitute_case_callout_data;
    }
    if partial != 0 && options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY as uint32_t == 0 as uint32_t {
        return PCRE2_ERROR_BADOPTION;
    }
    if replacement.is_null() {
        if rlength != 0 as size_t {
            return PCRE2_ERROR_NULL;
        }
        replacement = &raw mut null_str as *mut PCRE2_UCHAR16 as PCRE2_SPTR16;
    }
    if rlength == PCRE2_ZERO_TERMINATED {
        rlength = _pcre2_strlen_16(replacement);
    }
    repend = replacement.offset(rlength as isize);
    if subject.is_null() {
        if length != 0 as size_t {
            return PCRE2_ERROR_NULL;
        }
        subject = &raw mut null_str as *mut PCRE2_UCHAR16 as PCRE2_SPTR16;
    }
    if length == PCRE2_ZERO_TERMINATED {
        length = _pcre2_strlen_16(subject);
    }
    use_existing_match = (options & PCRE2_SUBSTITUTE_MATCHED as uint32_t != 0 as uint32_t)
        as ::core::ffi::c_int as BOOL;
    replacement_only = (options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY as uint32_t != 0 as uint32_t)
        as ::core::ffi::c_int as BOOL;
    if use_existing_match != 0 && match_data.is_null() {
        return PCRE2_ERROR_NULL;
    }
    if use_existing_match != 0 {
        if (*match_data).rc < 0 as ::core::ffi::c_int && (*match_data).rc != PCRE2_ERROR_NOMATCH {
            return (*match_data).rc;
        }
        if (*match_data).matchedby as ::core::ffi::c_int
            == PCRE2_MATCHEDBY_DFA_INTERPRETER as ::core::ffi::c_int
        {
            return PCRE2_ERROR_DFA_UFUNC;
        }
        if code != (*match_data).code {
            return PCRE2_ERROR_DIFFSUBSPATTERN;
        }
        if length != (*match_data).subject_length
            || !(original_subject == (*match_data).subject
                || (*match_data).flags as ::core::ffi::c_uint & PCRE2_MD_COPIED_SUBJECT
                    != 0 as ::core::ffi::c_uint
                    && (length == 0 as size_t
                        || memcmp(
                            subject as *const ::core::ffi::c_void,
                            (*match_data).subject as *const ::core::ffi::c_void,
                            length.wrapping_mul(
                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                            ),
                        ) == 0 as ::core::ffi::c_int))
        {
            return PCRE2_ERROR_DIFFSUBSSUBJECT;
        }
        if start_offset != (*match_data).start_offset {
            return PCRE2_ERROR_DIFFSUBSOFFSET;
        }
        if options & !(SUBSTITUTE_OPTIONS as uint32_t | PCRE2_NO_UTF_CHECK as uint32_t)
            != (*match_data).options & !(PCRE2_NO_UTF_CHECK as uint32_t)
        {
            return PCRE2_ERROR_DIFFSUBSOPTIONS;
        }
    }
    if match_data.is_null() {
        let mut gcontext: pcre2_general_context_16 = pcre2_general_context_16 {
            memctl: pcre2_memctl {
                malloc: None,
                free: None,
                memory_data: ::core::ptr::null_mut::<::core::ffi::c_void>(),
            },
        };
        gcontext.memctl = if mcontext.is_null() {
            (*(code as *mut pcre2_real_code_16)).memctl
        } else {
            (*(mcontext as *mut pcre2_real_match_context_16)).memctl
        };
        internal_match_data = pcre2_match_data_create_from_pattern_16(code, &raw mut gcontext);
        match_data = internal_match_data;
        if internal_match_data.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }
    } else if use_existing_match != 0 {
        let mut pairs: ::core::ffi::c_int = 0;
        let mut gcontext_0: pcre2_general_context_16 = pcre2_general_context_16 {
            memctl: pcre2_memctl {
                malloc: None,
                free: None,
                memory_data: ::core::ptr::null_mut::<::core::ffi::c_void>(),
            },
        };
        gcontext_0.memctl = if mcontext.is_null() {
            (*(code as *mut pcre2_real_code_16)).memctl
        } else {
            (*(mcontext as *mut pcre2_real_match_context_16)).memctl
        };
        pairs = if ((*code).top_bracket as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
            < (*match_data).oveccount as ::core::ffi::c_int
        {
            (*code).top_bracket as ::core::ffi::c_int + 1 as ::core::ffi::c_int
        } else {
            (*match_data).oveccount as ::core::ffi::c_int
        };
        internal_match_data =
            pcre2_match_data_create_16((*match_data).oveccount as uint32_t, &raw mut gcontext_0);
        if internal_match_data.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }
        memcpy(
            internal_match_data as *mut ::core::ffi::c_void,
            match_data as *const ::core::ffi::c_void,
            (120 as size_t).wrapping_add(
                ((2 as ::core::ffi::c_int * pairs) as size_t)
                    .wrapping_mul(::core::mem::size_of::<size_t>() as size_t),
            ),
        );
        (*internal_match_data).heapframes = ::core::ptr::null_mut::<heapframe>();
        (*internal_match_data).heapframes_size = 0 as size_t;
        (*internal_match_data).flags = ((*internal_match_data).flags as ::core::ffi::c_uint
            & !PCRE2_MD_COPIED_SUBJECT) as uint8_t;
        match_data = internal_match_data;
    }
    if !internal_match_data.is_null() {
        options = (options as ::core::ffi::c_uint & !PCRE2_COPY_MATCHED_SUBJECT) as uint32_t;
    }
    ovector = pcre2_get_ovector_pointer_16(match_data);
    ovector_count = pcre2_get_ovector_count_16(match_data);
    scb.version = 0 as uint32_t;
    scb.input = subject;
    scb.output = buffer as PCRE2_SPTR16;
    scb.ovector = ovector;
    if utf != 0 && options & PCRE2_NO_UTF_CHECK as uint32_t == 0 as uint32_t {
        rc = _pcre2_valid_utf_16(replacement, rlength, &raw mut (*match_data).startchar);
        if rc != 0 as ::core::ffi::c_int {
            (*match_data).leftchar = 0 as size_t;
            current_block = 10288652812641984457;
        } else {
            current_block = 5807581744382915773;
        }
    } else {
        current_block = 5807581744382915773;
    }
    match current_block {
        5807581744382915773 => {
            suboptions = options & SUBSTITUTE_OPTIONS as uint32_t;
            options = (options as ::core::ffi::c_uint & !SUBSTITUTE_OPTIONS) as uint32_t;
            if start_offset > length {
                (*match_data).leftchar = 0 as size_t;
                rc = PCRE2_ERROR_BADOFFSET;
            } else {
                if replacement_only == 0 {
                    let mut chkmc_length: size_t = start_offset;
                    if overflowed != 0 {
                        if chkmc_length
                            > (!(0 as ::core::ffi::c_int as size_t)).wrapping_sub(extra_needed)
                        {
                            current_block = 271895580782157815;
                        } else {
                            extra_needed = extra_needed.wrapping_add(chkmc_length);
                            current_block = 851619935621435220;
                        }
                    } else if lengthleft < chkmc_length {
                        if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                            == 0 as uint32_t
                        {
                            current_block = 1593672184456546103;
                        } else {
                            overflowed = TRUE as BOOL;
                            extra_needed = chkmc_length.wrapping_sub(lengthleft);
                            current_block = 851619935621435220;
                        }
                    } else {
                        memcpy(
                            buffer.offset(buff_offset as isize) as *mut ::core::ffi::c_void,
                            subject as *const ::core::ffi::c_void,
                            chkmc_length.wrapping_mul(
                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                            ),
                        );
                        buff_offset = buff_offset.wrapping_add(chkmc_length);
                        lengthleft = lengthleft.wrapping_sub(chkmc_length);
                        current_block = 851619935621435220;
                    }
                } else {
                    current_block = 851619935621435220;
                }
                match current_block {
                    851619935621435220 => {
                        subs = 0 as ::core::ffi::c_int;
                        's_407: loop {
                            let mut ptrstack: [PCRE2_SPTR16; 20] =
                                [::core::ptr::null::<PCRE2_UCHAR16>(); 20];
                            let mut ptrstackptr: uint32_t = 0 as uint32_t;
                            let mut forcecase: case_state = case_state {
                                to_case: PCRE2_SUBSTITUTE_CASE_NONE,
                                single_char: FALSE,
                            };
                            let mut casestart_offset: size_t = 0 as size_t;
                            let mut casestart_extra_needed: size_t = 0 as size_t;
                            if use_existing_match != 0 {
                                rc = (*match_data).rc;
                                use_existing_match = FALSE as BOOL;
                            } else {
                                rc = pcre2_match_16(
                                    code,
                                    subject,
                                    length,
                                    start_offset,
                                    options | goptions,
                                    match_data,
                                    mcontext,
                                );
                            }
                            if utf != 0 {
                                options = (options as ::core::ffi::c_uint | PCRE2_NO_UTF_CHECK)
                                    as uint32_t;
                            }
                            if rc == PCRE2_ERROR_NOMATCH {
                                current_block = 11608549431655460403;
                                break;
                            }
                            if rc < 0 as ::core::ffi::c_int {
                                current_block = 10288652812641984457;
                                break;
                            }
                            if *ovector.offset(1 as ::core::ffi::c_int as isize)
                                < *ovector.offset(0 as ::core::ffi::c_int as isize)
                                || *ovector.offset(0 as ::core::ffi::c_int as isize) < start_offset
                            {
                                rc = PCRE2_ERROR_BADSUBSPATTERN;
                                current_block = 10288652812641984457;
                                break;
                            } else if subs > 0 as ::core::ffi::c_int
                                && !(*ovector.offset(1 as ::core::ffi::c_int as isize)
                                    > ovecsave[1 as ::core::ffi::c_int as usize]
                                    || *ovector.offset(1 as ::core::ffi::c_int as isize)
                                        == *ovector.offset(0 as ::core::ffi::c_int as isize)
                                        && ovecsave[1 as ::core::ffi::c_int as usize]
                                            > ovecsave[0 as ::core::ffi::c_int as usize]
                                        && *ovector.offset(1 as ::core::ffi::c_int as isize)
                                            == ovecsave[1 as ::core::ffi::c_int as usize])
                            {
                                rc = PCRE2_ERROR_INTERNAL_DUPMATCH;
                                current_block = 10288652812641984457;
                                break;
                            } else {
                                ovecsave[0 as ::core::ffi::c_int as usize] =
                                    *ovector.offset(0 as ::core::ffi::c_int as isize);
                                ovecsave[1 as ::core::ffi::c_int as usize] =
                                    *ovector.offset(1 as ::core::ffi::c_int as isize);
                                if subs == INT_MAX {
                                    rc = PCRE2_ERROR_TOOMANYREPLACE;
                                    current_block = 10288652812641984457;
                                    break;
                                } else {
                                    subs += 1;
                                    if rc == 0 as ::core::ffi::c_int {
                                        rc = ovector_count as ::core::ffi::c_int;
                                    }
                                    fraglength = (*ovector
                                        .offset(0 as ::core::ffi::c_int as isize))
                                    .wrapping_sub(start_offset);
                                    if replacement_only == 0 {
                                        let mut chkmc_length_0: size_t = fraglength;
                                        if overflowed != 0 {
                                            if chkmc_length_0
                                                > (!(0 as ::core::ffi::c_int as size_t))
                                                    .wrapping_sub(extra_needed)
                                            {
                                                current_block = 271895580782157815;
                                                break;
                                            }
                                            extra_needed =
                                                extra_needed.wrapping_add(chkmc_length_0);
                                        } else if lengthleft < chkmc_length_0 {
                                            if suboptions
                                                & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                == 0 as uint32_t
                                            {
                                                current_block = 1593672184456546103;
                                                break;
                                            }
                                            overflowed = TRUE as BOOL;
                                            extra_needed = chkmc_length_0.wrapping_sub(lengthleft);
                                        } else {
                                            memcpy(
                                                buffer.offset(buff_offset as isize)
                                                    as *mut ::core::ffi::c_void,
                                                subject.offset(start_offset as isize)
                                                    as *const ::core::ffi::c_void,
                                                chkmc_length_0.wrapping_mul(
                                                    (PCRE2_CODE_UNIT_WIDTH
                                                        / 8 as ::core::ffi::c_int)
                                                        as size_t,
                                                ),
                                            );
                                            buff_offset = buff_offset.wrapping_add(chkmc_length_0);
                                            lengthleft = lengthleft.wrapping_sub(chkmc_length_0);
                                        }
                                    }
                                    scb.output_offsets[0 as ::core::ffi::c_int as usize] =
                                        buff_offset;
                                    scb.oveccount = rc as uint32_t;
                                    sub_start_extra_needed = extra_needed;
                                    ptr = replacement;
                                    if suboptions & PCRE2_SUBSTITUTE_LITERAL as uint32_t
                                        != 0 as uint32_t
                                    {
                                        let mut chkmc_length_1: size_t = rlength;
                                        if overflowed != 0 {
                                            if chkmc_length_1
                                                > (!(0 as ::core::ffi::c_int as size_t))
                                                    .wrapping_sub(extra_needed)
                                            {
                                                current_block = 271895580782157815;
                                                break;
                                            }
                                            extra_needed =
                                                extra_needed.wrapping_add(chkmc_length_1);
                                        } else if lengthleft < chkmc_length_1 {
                                            if suboptions
                                                & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                == 0 as uint32_t
                                            {
                                                current_block = 1593672184456546103;
                                                break;
                                            }
                                            overflowed = TRUE as BOOL;
                                            extra_needed = chkmc_length_1.wrapping_sub(lengthleft);
                                        } else {
                                            memcpy(
                                                buffer.offset(buff_offset as isize)
                                                    as *mut ::core::ffi::c_void,
                                                ptr as *const ::core::ffi::c_void,
                                                chkmc_length_1.wrapping_mul(
                                                    (PCRE2_CODE_UNIT_WIDTH
                                                        / 8 as ::core::ffi::c_int)
                                                        as size_t,
                                                ),
                                            );
                                            buff_offset = buff_offset.wrapping_add(chkmc_length_1);
                                            lengthleft = lengthleft.wrapping_sub(chkmc_length_1);
                                        }
                                    } else {
                                        loop {
                                            let mut ch: uint32_t = 0;
                                            let mut chlen: ::core::ffi::c_uint = 0;
                                            let mut group: ::core::ffi::c_int = 0;
                                            let mut special: uint32_t = 0;
                                            let mut text1_start: PCRE2_SPTR16 =
                                                ::core::ptr::null::<PCRE2_UCHAR16>();
                                            let mut text1_end: PCRE2_SPTR16 =
                                                ::core::ptr::null::<PCRE2_UCHAR16>();
                                            let mut text2_start: PCRE2_SPTR16 =
                                                ::core::ptr::null::<PCRE2_UCHAR16>();
                                            let mut text2_end: PCRE2_SPTR16 =
                                                ::core::ptr::null::<PCRE2_UCHAR16>();
                                            let mut name: [PCRE2_UCHAR16; 129] = [0; 129];
                                            if ptr >= repend {
                                                if ptrstackptr == 0 as uint32_t {
                                                    break;
                                                }
                                                ptrstackptr = ptrstackptr.wrapping_sub(1);
                                                repend = ptrstack[ptrstackptr as usize];
                                                ptrstackptr = ptrstackptr.wrapping_sub(1);
                                                ptr = ptrstack[ptrstackptr as usize];
                                            } else {
                                                if escaped_literal != 0 {
                                                    if *ptr.offset(0 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                        == CHAR_BACKSLASH
                                                        && ptr
                                                            < repend.offset(
                                                                -(1 as ::core::ffi::c_int as isize),
                                                            )
                                                        && *ptr.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            == CHAR_E
                                                    {
                                                        escaped_literal = FALSE as BOOL;
                                                        ptr = ptr.offset(
                                                            2 as ::core::ffi::c_int as isize,
                                                        );
                                                        continue;
                                                    }
                                                } else {
                                                    if *ptr as ::core::ffi::c_int
                                                        == CHAR_DOLLAR_SIGN
                                                    {
                                                        inparens = 0;
                                                        inangle = 0;
                                                        star = 0;
                                                        sublength = 0;
                                                        next = 0;
                                                        subptr =
                                                            ::core::ptr::null::<PCRE2_UCHAR16>();
                                                        subptrend =
                                                            ::core::ptr::null::<PCRE2_UCHAR16>();
                                                        ptr = ptr.offset(1);
                                                        if ptr >= repend {
                                                            current_block = 11196033572913570296;
                                                            break 's_407;
                                                        }
                                                        next = *ptr;
                                                        if next as ::core::ffi::c_int
                                                            == CHAR_DOLLAR_SIGN
                                                        {
                                                            current_block = 11728966381135444667;
                                                        } else {
                                                            special = 0 as uint32_t;
                                                            text1_start =
                                                                ::core::ptr::null::<PCRE2_UCHAR16>(
                                                                );
                                                            text1_end =
                                                                ::core::ptr::null::<PCRE2_UCHAR16>(
                                                                );
                                                            text2_start =
                                                                ::core::ptr::null::<PCRE2_UCHAR16>(
                                                                );
                                                            text2_end =
                                                                ::core::ptr::null::<PCRE2_UCHAR16>(
                                                                );
                                                            group = -(1 as ::core::ffi::c_int);
                                                            inparens = FALSE as BOOL;
                                                            inangle = FALSE as BOOL;
                                                            star = FALSE as BOOL;
                                                            subptr =
                                                                ::core::ptr::null::<PCRE2_UCHAR16>(
                                                                );
                                                            subptrend =
                                                                ::core::ptr::null::<PCRE2_UCHAR16>(
                                                                );
                                                            if next as ::core::ffi::c_int
                                                                == CHAR_AMPERSAND
                                                            {
                                                                ptr = ptr.offset(1);
                                                                group = 0 as ::core::ffi::c_int;
                                                                current_block = 7268916931342848830;
                                                            } else if next as ::core::ffi::c_int
                                                                == CHAR_GRAVE_ACCENT
                                                                || next as ::core::ffi::c_int
                                                                    == CHAR_APOSTROPHE
                                                            {
                                                                ptr = ptr.offset(1);
                                                                rc = pcre2_substring_length_bynumber_16(
                                                                    match_data,
                                                                    0 as uint32_t,
                                                                    &raw mut sublength,
                                                                );
                                                                if rc < 0 as ::core::ffi::c_int {
                                                                    current_block =
                                                                        12248883293374827016;
                                                                    break 's_407;
                                                                }
                                                                if next as ::core::ffi::c_int
                                                                    == CHAR_GRAVE_ACCENT
                                                                {
                                                                    subptr = subject;
                                                                    subptrend =
                                                                        subject
                                                                            .offset(*ovector.offset(
                                                                            0 as ::core::ffi::c_int
                                                                                as isize,
                                                                        )
                                                                            as isize);
                                                                } else if partial != 0 {
                                                                    rc = PCRE2_ERROR_PARTIALSUBS;
                                                                    current_block =
                                                                        12248883293374827016;
                                                                    break 's_407;
                                                                } else {
                                                                    subptr =
                                                                        subject
                                                                            .offset(*ovector.offset(
                                                                            1 as ::core::ffi::c_int
                                                                                as isize,
                                                                        )
                                                                            as isize);
                                                                    subptrend = subject
                                                                        .offset(length as isize);
                                                                }
                                                                current_block = 475358603377927448;
                                                            } else if next as ::core::ffi::c_int
                                                                == CHAR_UNDERSCORE
                                                            {
                                                                ptr = ptr.offset(1);
                                                                if partial != 0 {
                                                                    rc = PCRE2_ERROR_PARTIALSUBS;
                                                                    current_block =
                                                                        12248883293374827016;
                                                                    break 's_407;
                                                                } else {
                                                                    subptr = subject;
                                                                    subptrend = subject
                                                                        .offset(length as isize);
                                                                }
                                                                current_block = 475358603377927448;
                                                            } else {
                                                                if next as ::core::ffi::c_int == CHAR_PLUS
                                                                    && !(ptr.offset(1 as ::core::ffi::c_int as isize) < repend
                                                                        && *ptr.offset(1 as ::core::ffi::c_int as isize)
                                                                            as ::core::ffi::c_int == CHAR_LEFT_CURLY_BRACKET)
                                                                {
                                                                    ptr = ptr.offset(1);
                                                                    if (*code).top_bracket as ::core::ffi::c_int
                                                                        == 0 as ::core::ffi::c_int
                                                                    {
                                                                        if suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET as uint32_t
                                                                            == 0 as uint32_t
                                                                        {
                                                                            rc = PCRE2_ERROR_NOSUBSTRING;
                                                                            current_block = 12248883293374827016;
                                                                            break 's_407;
                                                                        } else {
                                                                            group = 0 as ::core::ffi::c_int;
                                                                        }
                                                                    } else if ((*match_data).oveccount as ::core::ffi::c_int)
                                                                        < (*code).top_bracket as ::core::ffi::c_int
                                                                            + 1 as ::core::ffi::c_int
                                                                    {
                                                                        rc = PCRE2_ERROR_UNAVAILABLE;
                                                                        current_block = 12248883293374827016;
                                                                        break 's_407;
                                                                    } else {
                                                                        group = (*code).top_bracket as ::core::ffi::c_int;
                                                                        while group > 0 as ::core::ffi::c_int {
                                                                            if *ovector
                                                                                .offset((2 as ::core::ffi::c_int * group) as isize)
                                                                                != PCRE2_UNSET
                                                                            {
                                                                                break;
                                                                            }
                                                                            group -= 1;
                                                                        }
                                                                    }
                                                                    if group == 0 as ::core::ffi::c_int {
                                                                        if suboptions & PCRE2_SUBSTITUTE_UNSET_EMPTY as uint32_t
                                                                            != 0 as uint32_t
                                                                        {
                                                                            continue;
                                                                        }
                                                                        rc = PCRE2_ERROR_UNSET;
                                                                        current_block = 12248883293374827016;
                                                                        break 's_407;
                                                                    }
                                                                } else {
                                                                    if next as ::core::ffi::c_int == CHAR_LEFT_CURLY_BRACKET {
                                                                        ptr = ptr.offset(1);
                                                                        if ptr >= repend {
                                                                            current_block = 11196033572913570296;
                                                                            break 's_407;
                                                                        }
                                                                        next = *ptr;
                                                                        inparens = TRUE as BOOL;
                                                                    } else if next as ::core::ffi::c_int == CHAR_LESS_THAN_SIGN
                                                                    {
                                                                        ptr = ptr.offset(1);
                                                                        if ptr >= repend {
                                                                            current_block = 11196033572913570296;
                                                                            break 's_407;
                                                                        }
                                                                        next = *ptr;
                                                                        inangle = TRUE as BOOL;
                                                                    }
                                                                    if inangle == 0
                                                                        && next as ::core::ffi::c_int == CHAR_ASTERISK
                                                                    {
                                                                        ptr = ptr.offset(1);
                                                                        if ptr >= repend {
                                                                            current_block = 11196033572913570296;
                                                                            break 's_407;
                                                                        }
                                                                        next = *ptr;
                                                                        star = TRUE as BOOL;
                                                                    }
                                                                    if star == 0 && inangle == 0
                                                                        && next as ::core::ffi::c_int >= CHAR_0
                                                                        && next as ::core::ffi::c_int <= CHAR_9
                                                                    {
                                                                        group = next as ::core::ffi::c_int - CHAR_0;
                                                                        loop {
                                                                            ptr = ptr.offset(1);
                                                                            if !(ptr < repend) {
                                                                                break;
                                                                            }
                                                                            next = *ptr;
                                                                            if (next as ::core::ffi::c_int) < CHAR_0
                                                                                || next as ::core::ffi::c_int > CHAR_9
                                                                            {
                                                                                break;
                                                                            }
                                                                            group = group * 10 as ::core::ffi::c_int
                                                                                + (next as ::core::ffi::c_int - CHAR_0);
                                                                            if !(group > (*code).top_bracket as ::core::ffi::c_int) {
                                                                                continue;
                                                                            }
                                                                            if suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET as uint32_t
                                                                                != 0 as uint32_t
                                                                            {
                                                                                loop {
                                                                                    ptr = ptr.offset(1);
                                                                                    if !(ptr < repend && *ptr as ::core::ffi::c_int >= CHAR_0
                                                                                        && *ptr as ::core::ffi::c_int <= CHAR_9)
                                                                                    {
                                                                                        break;
                                                                                    }
                                                                                }
                                                                                break;
                                                                            } else {
                                                                                rc = PCRE2_ERROR_NOSUBSTRING;
                                                                                current_block = 12248883293374827016;
                                                                                break 's_407;
                                                                            }
                                                                        }
                                                                    } else {
                                                                        let mut name_len: size_t = 0;
                                                                        let mut name_start: PCRE2_SPTR16 = ptr;
                                                                        if read_name_subst(
                                                                            &raw mut ptr,
                                                                            repend,
                                                                            utf,
                                                                            (*code).tables.offset(ctypes_offset as isize),
                                                                        ) == 0
                                                                        {
                                                                            current_block = 11196033572913570296;
                                                                            break 's_407;
                                                                        }
                                                                        name_len = ptr.offset_from(name_start)
                                                                            as isize as size_t;
                                                                        memcpy(
                                                                            &raw mut name as *mut PCRE2_UCHAR16
                                                                                as *mut ::core::ffi::c_void,
                                                                            name_start as *const ::core::ffi::c_void,
                                                                            name_len
                                                                                .wrapping_mul(
                                                                                    (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                ),
                                                                        );
                                                                        name[name_len as usize] = 0 as PCRE2_UCHAR16;
                                                                    }
                                                                    next = 0 as PCRE2_UCHAR16;
                                                                    if inparens != 0 {
                                                                        if suboptions & PCRE2_SUBSTITUTE_EXTENDED as uint32_t
                                                                            != 0 as uint32_t && star == 0
                                                                            && ptr < repend.offset(-(2 as ::core::ffi::c_int as isize))
                                                                            && *ptr as ::core::ffi::c_int == CHAR_COLON
                                                                        {
                                                                            ptr = ptr.offset(1);
                                                                            special = *ptr as uint32_t;
                                                                            if special != CHAR_PLUS as uint32_t
                                                                                && special != CHAR_MINUS as uint32_t
                                                                            {
                                                                                rc = PCRE2_ERROR_BADSUBSTITUTION;
                                                                                current_block = 12248883293374827016;
                                                                                break 's_407;
                                                                            } else {
                                                                                ptr = ptr.offset(1);
                                                                                text1_start = ptr;
                                                                                rc = find_text_end(
                                                                                    code,
                                                                                    &raw mut ptr,
                                                                                    repend,
                                                                                    (special == CHAR_MINUS as uint32_t) as ::core::ffi::c_int,
                                                                                );
                                                                                if rc != 0 as ::core::ffi::c_int {
                                                                                    current_block = 12248883293374827016;
                                                                                    break 's_407;
                                                                                }
                                                                                text1_end = ptr;
                                                                                if special == CHAR_PLUS as uint32_t
                                                                                    && *ptr as ::core::ffi::c_int == CHAR_COLON
                                                                                {
                                                                                    ptr = ptr.offset(1);
                                                                                    text2_start = ptr;
                                                                                    rc = find_text_end(code, &raw mut ptr, repend, TRUE);
                                                                                    if rc != 0 as ::core::ffi::c_int {
                                                                                        current_block = 12248883293374827016;
                                                                                        break 's_407;
                                                                                    }
                                                                                    text2_end = ptr;
                                                                                }
                                                                            }
                                                                        } else if ptr >= repend
                                                                            || *ptr as ::core::ffi::c_int != CHAR_RIGHT_CURLY_BRACKET
                                                                        {
                                                                            rc = PCRE2_ERROR_REPMISSINGBRACE;
                                                                            current_block = 12248883293374827016;
                                                                            break 's_407;
                                                                        }
                                                                        ptr = ptr.offset(1);
                                                                    }
                                                                    if inangle != 0 {
                                                                        if ptr >= repend
                                                                            || *ptr as ::core::ffi::c_int != CHAR_GREATER_THAN_SIGN
                                                                        {
                                                                            current_block = 11196033572913570296;
                                                                            break 's_407;
                                                                        }
                                                                        ptr = ptr.offset(1);
                                                                    }
                                                                    if star != 0 {
                                                                        if !(_pcre2_strcmp_c8_16(
                                                                            &raw mut name as *mut PCRE2_UCHAR16 as PCRE2_SPTR16,
                                                                            b"MARK\0" as *const u8 as *const ::core::ffi::c_char,
                                                                        ) == 0 as ::core::ffi::c_int)
                                                                        {
                                                                            current_block = 11196033572913570296;
                                                                            break 's_407;
                                                                        }
                                                                        let mut mark: PCRE2_SPTR16 = pcre2_get_mark_16(match_data);
                                                                        if mark.is_null() {
                                                                            continue;
                                                                        }
                                                                        fraglength = *mark
                                                                            .offset(-(1 as ::core::ffi::c_int) as isize) as size_t;
                                                                        if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                                                            && substitute_case_callout.is_none()
                                                                        {
                                                                            let mut chkcc_length: size_t = fraglength;
                                                                            let mut chkcc_rc: size_t = 0;
                                                                            chkcc_rc = default_substitute_case_callout(
                                                                                mark,
                                                                                chkcc_length,
                                                                                buffer.offset(buff_offset as isize),
                                                                                if overflowed != 0 { 0 as size_t } else { lengthleft },
                                                                                &raw mut forcecase,
                                                                                code,
                                                                            );
                                                                            if overflowed != 0 {
                                                                                if chkcc_rc
                                                                                    > (!(0 as ::core::ffi::c_int as size_t))
                                                                                        .wrapping_sub(extra_needed)
                                                                                {
                                                                                    current_block = 271895580782157815;
                                                                                    break 's_407;
                                                                                }
                                                                                extra_needed = extra_needed.wrapping_add(chkcc_rc);
                                                                                continue;
                                                                            } else if lengthleft < chkcc_rc {
                                                                                if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                    == 0 as uint32_t
                                                                                {
                                                                                    current_block = 1593672184456546103;
                                                                                    break 's_407;
                                                                                }
                                                                                overflowed = TRUE as BOOL;
                                                                                extra_needed = chkcc_rc.wrapping_sub(lengthleft);
                                                                                continue;
                                                                            } else {
                                                                                buff_offset = buff_offset.wrapping_add(chkcc_rc);
                                                                                lengthleft = lengthleft.wrapping_sub(chkcc_rc);
                                                                                continue;
                                                                            }
                                                                        } else {
                                                                            let mut chkmc_length_2: size_t = fraglength;
                                                                            if overflowed != 0 {
                                                                                if chkmc_length_2
                                                                                    > (!(0 as ::core::ffi::c_int as size_t))
                                                                                        .wrapping_sub(extra_needed)
                                                                                {
                                                                                    current_block = 271895580782157815;
                                                                                    break 's_407;
                                                                                }
                                                                                extra_needed = extra_needed.wrapping_add(chkmc_length_2);
                                                                                continue;
                                                                            } else if lengthleft < chkmc_length_2 {
                                                                                if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                    == 0 as uint32_t
                                                                                {
                                                                                    current_block = 1593672184456546103;
                                                                                    break 's_407;
                                                                                }
                                                                                overflowed = TRUE as BOOL;
                                                                                extra_needed = chkmc_length_2.wrapping_sub(lengthleft);
                                                                                continue;
                                                                            } else {
                                                                                memcpy(
                                                                                    buffer.offset(buff_offset as isize)
                                                                                        as *mut ::core::ffi::c_void,
                                                                                    mark as *const ::core::ffi::c_void,
                                                                                    chkmc_length_2
                                                                                        .wrapping_mul(
                                                                                            (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                        ),
                                                                                );
                                                                                buff_offset = buff_offset.wrapping_add(chkmc_length_2);
                                                                                lengthleft = lengthleft.wrapping_sub(chkmc_length_2);
                                                                                continue;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                current_block = 7268916931342848830;
                                                            }
                                                        }
                                                    } else if suboptions
                                                        & PCRE2_SUBSTITUTE_EXTENDED as uint32_t
                                                        != 0 as uint32_t
                                                        && *ptr as ::core::ffi::c_int
                                                            == CHAR_BACKSLASH
                                                    {
                                                        let mut errorcode: ::core::ffi::c_int = 0;
                                                        let mut new_forcecase: case_state =
                                                            case_state {
                                                                to_case: PCRE2_SUBSTITUTE_CASE_NONE,
                                                                single_char: FALSE,
                                                            };
                                                        if ptr
                                                            < repend.offset(
                                                                -(1 as ::core::ffi::c_int as isize),
                                                            )
                                                        {
                                                            match *ptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                            {
                                                                CHAR_L => {
                                                                    new_forcecase.to_case =
                                                                        PCRE2_SUBSTITUTE_CASE_LOWER;
                                                                    new_forcecase.single_char =
                                                                        FALSE as BOOL;
                                                                    ptr = ptr.offset(
                                                                        2 as ::core::ffi::c_int
                                                                            as isize,
                                                                    );
                                                                }
                                                                CHAR_l => {
                                                                    new_forcecase.to_case =
                                                                        PCRE2_SUBSTITUTE_CASE_LOWER;
                                                                    new_forcecase.single_char =
                                                                        TRUE as BOOL;
                                                                    ptr = ptr.offset(
                                                                        2 as ::core::ffi::c_int
                                                                            as isize,
                                                                    );
                                                                    if ptr.offset(
                                                                        2 as ::core::ffi::c_int
                                                                            as isize,
                                                                    ) < repend
                                                                        && *ptr.offset(
                                                                            0 as ::core::ffi::c_int
                                                                                as isize,
                                                                        )
                                                                            as ::core::ffi::c_int
                                                                            == CHAR_BACKSLASH
                                                                        && *ptr.offset(
                                                                            1 as ::core::ffi::c_int
                                                                                as isize,
                                                                        )
                                                                            as ::core::ffi::c_int
                                                                            == CHAR_U
                                                                    {
                                                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST;
                                                                        new_forcecase.single_char =
                                                                            FALSE as BOOL;
                                                                        ptr = ptr.offset(
                                                                            2 as ::core::ffi::c_int
                                                                                as isize,
                                                                        );
                                                                    }
                                                                }
                                                                CHAR_U => {
                                                                    new_forcecase.to_case =
                                                                        PCRE2_SUBSTITUTE_CASE_UPPER;
                                                                    new_forcecase.single_char =
                                                                        FALSE as BOOL;
                                                                    ptr = ptr.offset(
                                                                        2 as ::core::ffi::c_int
                                                                            as isize,
                                                                    );
                                                                }
                                                                CHAR_u => {
                                                                    new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_TITLE_FIRST;
                                                                    new_forcecase.single_char =
                                                                        TRUE as BOOL;
                                                                    ptr = ptr.offset(
                                                                        2 as ::core::ffi::c_int
                                                                            as isize,
                                                                    );
                                                                    if ptr.offset(
                                                                        2 as ::core::ffi::c_int
                                                                            as isize,
                                                                    ) < repend
                                                                        && *ptr.offset(
                                                                            0 as ::core::ffi::c_int
                                                                                as isize,
                                                                        )
                                                                            as ::core::ffi::c_int
                                                                            == CHAR_BACKSLASH
                                                                        && *ptr.offset(
                                                                            1 as ::core::ffi::c_int
                                                                                as isize,
                                                                        )
                                                                            as ::core::ffi::c_int
                                                                            == CHAR_L
                                                                    {
                                                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_TITLE_FIRST;
                                                                        new_forcecase.single_char =
                                                                            FALSE as BOOL;
                                                                        ptr = ptr.offset(
                                                                            2 as ::core::ffi::c_int
                                                                                as isize,
                                                                        );
                                                                    }
                                                                }
                                                                _ => {}
                                                            }
                                                        }
                                                        if new_forcecase.to_case
                                                            != PCRE2_SUBSTITUTE_CASE_NONE
                                                        {
                                                            current_block = 17330731906949607220;
                                                        } else {
                                                            ptr = ptr.offset(1);
                                                            rc = _pcre2_check_escape_16(
                                                                &raw mut ptr,
                                                                repend,
                                                                &raw mut ch,
                                                                &raw mut errorcode,
                                                                (*code).overall_options,
                                                                (*code).extra_options,
                                                                (*code).top_bracket as uint32_t,
                                                                FALSE,
                                                                ::core::ptr::null_mut::<
                                                                    compile_block_16,
                                                                >(
                                                                ),
                                                            );
                                                            if errorcode != 0 as ::core::ffi::c_int
                                                            {
                                                                current_block =
                                                                    13886604107758561977;
                                                                break 's_407;
                                                            }
                                                            match rc {
                                                                25 => {
                                                                    current_block =
                                                                        17330731906949607220;
                                                                }
                                                                26 => {
                                                                    current_block =
                                                                        17620377729944291641;
                                                                    match current_block {
                                                                        17058499098102203106 => {
                                                                            let mut name_len_0: size_t = 0;
                                                                            let mut name_start_0: PCRE2_SPTR16 = ::core::ptr::null::<
                                                                                PCRE2_UCHAR16,
                                                                            >();
                                                                            if ptr >= repend
                                                                                || *ptr as ::core::ffi::c_int != CHAR_LESS_THAN_SIGN
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            ptr = ptr.offset(1);
                                                                            name_start_0 = ptr;
                                                                            if read_name_subst(
                                                                                &raw mut ptr,
                                                                                repend,
                                                                                utf,
                                                                                (*code)
                                                                                    .tables
                                                                                    .offset(
                                                                                    ctypes_offset
                                                                                        as isize,
                                                                                ),
                                                                            ) == 0
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            name_len_0 = ptr.offset_from(name_start_0)
                                                                                as isize as size_t;
                                                                            if ptr >= repend
                                                                                || *ptr as ::core::ffi::c_int != CHAR_GREATER_THAN_SIGN
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            ptr = ptr.offset(1);
                                                                            special = 0 as uint32_t;
                                                                            group = -(1 as ::core::ffi::c_int);
                                                                            memcpy(
                                                                                &raw mut name as *mut PCRE2_UCHAR16
                                                                                    as *mut ::core::ffi::c_void,
                                                                                name_start_0 as *const ::core::ffi::c_void,
                                                                                name_len_0
                                                                                    .wrapping_mul(
                                                                                        (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                    ),
                                                                            );
                                                                            name[name_len_0
                                                                                as usize] =
                                                                                0 as PCRE2_UCHAR16;
                                                                            current_block =
                                                                                7268916931342848830;
                                                                        }
                                                                        11305664347990019861 => {
                                                                            if !(rc < 0 as ::core::ffi::c_int) {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            special = 0 as uint32_t;
                                                                            group = -rc - 1 as ::core::ffi::c_int;
                                                                            current_block =
                                                                                7268916931342848830;
                                                                        }
                                                                        17620377729944291641 => {
                                                                            escaped_literal =
                                                                                TRUE as BOOL;
                                                                            continue;
                                                                        }
                                                                        11058672248517667920 => {
                                                                            current_block =
                                                                                2151901123398569495;
                                                                        }
                                                                        _ => {}
                                                                    }
                                                                    match current_block {
                                                                        7268916931342848830 => {}
                                                                        _ => {
                                                                            if rc == ESC_b as ::core::ffi::c_int {
                                                                                ch = CHAR_BS as uint32_t;
                                                                            }
                                                                            if rc == ESC_v as ::core::ffi::c_int {
                                                                                ch = CHAR_VT as uint32_t;
                                                                            }
                                                                            if utf != 0 {
                                                                                chlen = _pcre2_ord2utf_16(
                                                                                    ch,
                                                                                    &raw mut temp as *mut PCRE2_UCHAR16,
                                                                                );
                                                                            } else {
                                                                                temp[0 as ::core::ffi::c_int as usize] = ch
                                                                                    as PCRE2_UCHAR16;
                                                                                chlen = 1 as ::core::ffi::c_uint;
                                                                            }
                                                                            if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                                                                && substitute_case_callout.is_none()
                                                                            {
                                                                                let mut chkcc_length_2: size_t = chlen as size_t;
                                                                                let mut chkcc_rc_2: size_t = 0;
                                                                                chkcc_rc_2 = default_substitute_case_callout(
                                                                                    &raw mut temp as *mut PCRE2_UCHAR16 as PCRE2_SPTR16,
                                                                                    chkcc_length_2,
                                                                                    buffer.offset(buff_offset as isize),
                                                                                    if overflowed != 0 { 0 as size_t } else { lengthleft },
                                                                                    &raw mut forcecase,
                                                                                    code,
                                                                                );
                                                                                if overflowed != 0 {
                                                                                    if chkcc_rc_2
                                                                                        > (!(0 as ::core::ffi::c_int as size_t))
                                                                                            .wrapping_sub(extra_needed)
                                                                                    {
                                                                                        current_block = 271895580782157815;
                                                                                        break 's_407;
                                                                                    }
                                                                                    extra_needed = extra_needed.wrapping_add(chkcc_rc_2);
                                                                                    continue;
                                                                                } else if lengthleft < chkcc_rc_2 {
                                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                        == 0 as uint32_t
                                                                                    {
                                                                                        current_block = 1593672184456546103;
                                                                                        break 's_407;
                                                                                    }
                                                                                    overflowed = TRUE as BOOL;
                                                                                    extra_needed = chkcc_rc_2.wrapping_sub(lengthleft);
                                                                                    continue;
                                                                                } else {
                                                                                    buff_offset = buff_offset.wrapping_add(chkcc_rc_2);
                                                                                    lengthleft = lengthleft.wrapping_sub(chkcc_rc_2);
                                                                                    continue;
                                                                                }
                                                                            } else {
                                                                                let mut chkmc_length_4: size_t = chlen as size_t;
                                                                                if overflowed != 0 {
                                                                                    if chkmc_length_4
                                                                                        > (!(0 as ::core::ffi::c_int as size_t))
                                                                                            .wrapping_sub(extra_needed)
                                                                                    {
                                                                                        current_block = 271895580782157815;
                                                                                        break 's_407;
                                                                                    }
                                                                                    extra_needed = extra_needed.wrapping_add(chkmc_length_4);
                                                                                    continue;
                                                                                } else if lengthleft < chkmc_length_4 {
                                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                        == 0 as uint32_t
                                                                                    {
                                                                                        current_block = 1593672184456546103;
                                                                                        break 's_407;
                                                                                    }
                                                                                    overflowed = TRUE as BOOL;
                                                                                    extra_needed = chkmc_length_4.wrapping_sub(lengthleft);
                                                                                    continue;
                                                                                } else {
                                                                                    memcpy(
                                                                                        buffer.offset(buff_offset as isize)
                                                                                            as *mut ::core::ffi::c_void,
                                                                                        &raw mut temp as *mut PCRE2_UCHAR16
                                                                                            as *const ::core::ffi::c_void,
                                                                                        chkmc_length_4
                                                                                            .wrapping_mul(
                                                                                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                            ),
                                                                                    );
                                                                                    buff_offset = buff_offset.wrapping_add(chkmc_length_4);
                                                                                    lengthleft = lengthleft.wrapping_sub(chkmc_length_4);
                                                                                    continue;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                0 => {
                                                                    current_block =
                                                                        11058672248517667920;
                                                                    match current_block {
                                                                        17058499098102203106 => {
                                                                            let mut name_len_0: size_t = 0;
                                                                            let mut name_start_0: PCRE2_SPTR16 = ::core::ptr::null::<
                                                                                PCRE2_UCHAR16,
                                                                            >();
                                                                            if ptr >= repend
                                                                                || *ptr as ::core::ffi::c_int != CHAR_LESS_THAN_SIGN
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            ptr = ptr.offset(1);
                                                                            name_start_0 = ptr;
                                                                            if read_name_subst(
                                                                                &raw mut ptr,
                                                                                repend,
                                                                                utf,
                                                                                (*code)
                                                                                    .tables
                                                                                    .offset(
                                                                                    ctypes_offset
                                                                                        as isize,
                                                                                ),
                                                                            ) == 0
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            name_len_0 = ptr.offset_from(name_start_0)
                                                                                as isize as size_t;
                                                                            if ptr >= repend
                                                                                || *ptr as ::core::ffi::c_int != CHAR_GREATER_THAN_SIGN
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            ptr = ptr.offset(1);
                                                                            special = 0 as uint32_t;
                                                                            group = -(1 as ::core::ffi::c_int);
                                                                            memcpy(
                                                                                &raw mut name as *mut PCRE2_UCHAR16
                                                                                    as *mut ::core::ffi::c_void,
                                                                                name_start_0 as *const ::core::ffi::c_void,
                                                                                name_len_0
                                                                                    .wrapping_mul(
                                                                                        (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                    ),
                                                                            );
                                                                            name[name_len_0
                                                                                as usize] =
                                                                                0 as PCRE2_UCHAR16;
                                                                            current_block =
                                                                                7268916931342848830;
                                                                        }
                                                                        11305664347990019861 => {
                                                                            if !(rc < 0 as ::core::ffi::c_int) {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            special = 0 as uint32_t;
                                                                            group = -rc - 1 as ::core::ffi::c_int;
                                                                            current_block =
                                                                                7268916931342848830;
                                                                        }
                                                                        17620377729944291641 => {
                                                                            escaped_literal =
                                                                                TRUE as BOOL;
                                                                            continue;
                                                                        }
                                                                        11058672248517667920 => {
                                                                            current_block =
                                                                                2151901123398569495;
                                                                        }
                                                                        _ => {}
                                                                    }
                                                                    match current_block {
                                                                        7268916931342848830 => {}
                                                                        _ => {
                                                                            if rc == ESC_b as ::core::ffi::c_int {
                                                                                ch = CHAR_BS as uint32_t;
                                                                            }
                                                                            if rc == ESC_v as ::core::ffi::c_int {
                                                                                ch = CHAR_VT as uint32_t;
                                                                            }
                                                                            if utf != 0 {
                                                                                chlen = _pcre2_ord2utf_16(
                                                                                    ch,
                                                                                    &raw mut temp as *mut PCRE2_UCHAR16,
                                                                                );
                                                                            } else {
                                                                                temp[0 as ::core::ffi::c_int as usize] = ch
                                                                                    as PCRE2_UCHAR16;
                                                                                chlen = 1 as ::core::ffi::c_uint;
                                                                            }
                                                                            if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                                                                && substitute_case_callout.is_none()
                                                                            {
                                                                                let mut chkcc_length_2: size_t = chlen as size_t;
                                                                                let mut chkcc_rc_2: size_t = 0;
                                                                                chkcc_rc_2 = default_substitute_case_callout(
                                                                                    &raw mut temp as *mut PCRE2_UCHAR16 as PCRE2_SPTR16,
                                                                                    chkcc_length_2,
                                                                                    buffer.offset(buff_offset as isize),
                                                                                    if overflowed != 0 { 0 as size_t } else { lengthleft },
                                                                                    &raw mut forcecase,
                                                                                    code,
                                                                                );
                                                                                if overflowed != 0 {
                                                                                    if chkcc_rc_2
                                                                                        > (!(0 as ::core::ffi::c_int as size_t))
                                                                                            .wrapping_sub(extra_needed)
                                                                                    {
                                                                                        current_block = 271895580782157815;
                                                                                        break 's_407;
                                                                                    }
                                                                                    extra_needed = extra_needed.wrapping_add(chkcc_rc_2);
                                                                                    continue;
                                                                                } else if lengthleft < chkcc_rc_2 {
                                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                        == 0 as uint32_t
                                                                                    {
                                                                                        current_block = 1593672184456546103;
                                                                                        break 's_407;
                                                                                    }
                                                                                    overflowed = TRUE as BOOL;
                                                                                    extra_needed = chkcc_rc_2.wrapping_sub(lengthleft);
                                                                                    continue;
                                                                                } else {
                                                                                    buff_offset = buff_offset.wrapping_add(chkcc_rc_2);
                                                                                    lengthleft = lengthleft.wrapping_sub(chkcc_rc_2);
                                                                                    continue;
                                                                                }
                                                                            } else {
                                                                                let mut chkmc_length_4: size_t = chlen as size_t;
                                                                                if overflowed != 0 {
                                                                                    if chkmc_length_4
                                                                                        > (!(0 as ::core::ffi::c_int as size_t))
                                                                                            .wrapping_sub(extra_needed)
                                                                                    {
                                                                                        current_block = 271895580782157815;
                                                                                        break 's_407;
                                                                                    }
                                                                                    extra_needed = extra_needed.wrapping_add(chkmc_length_4);
                                                                                    continue;
                                                                                } else if lengthleft < chkmc_length_4 {
                                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                        == 0 as uint32_t
                                                                                    {
                                                                                        current_block = 1593672184456546103;
                                                                                        break 's_407;
                                                                                    }
                                                                                    overflowed = TRUE as BOOL;
                                                                                    extra_needed = chkmc_length_4.wrapping_sub(lengthleft);
                                                                                    continue;
                                                                                } else {
                                                                                    memcpy(
                                                                                        buffer.offset(buff_offset as isize)
                                                                                            as *mut ::core::ffi::c_void,
                                                                                        &raw mut temp as *mut PCRE2_UCHAR16
                                                                                            as *const ::core::ffi::c_void,
                                                                                        chkmc_length_4
                                                                                            .wrapping_mul(
                                                                                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                            ),
                                                                                    );
                                                                                    buff_offset = buff_offset.wrapping_add(chkmc_length_4);
                                                                                    lengthleft = lengthleft.wrapping_sub(chkmc_length_4);
                                                                                    continue;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                5 | 21 => {
                                                                    current_block =
                                                                        2151901123398569495;
                                                                    match current_block {
                                                                        17058499098102203106 => {
                                                                            let mut name_len_0: size_t = 0;
                                                                            let mut name_start_0: PCRE2_SPTR16 = ::core::ptr::null::<
                                                                                PCRE2_UCHAR16,
                                                                            >();
                                                                            if ptr >= repend
                                                                                || *ptr as ::core::ffi::c_int != CHAR_LESS_THAN_SIGN
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            ptr = ptr.offset(1);
                                                                            name_start_0 = ptr;
                                                                            if read_name_subst(
                                                                                &raw mut ptr,
                                                                                repend,
                                                                                utf,
                                                                                (*code)
                                                                                    .tables
                                                                                    .offset(
                                                                                    ctypes_offset
                                                                                        as isize,
                                                                                ),
                                                                            ) == 0
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            name_len_0 = ptr.offset_from(name_start_0)
                                                                                as isize as size_t;
                                                                            if ptr >= repend
                                                                                || *ptr as ::core::ffi::c_int != CHAR_GREATER_THAN_SIGN
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            ptr = ptr.offset(1);
                                                                            special = 0 as uint32_t;
                                                                            group = -(1 as ::core::ffi::c_int);
                                                                            memcpy(
                                                                                &raw mut name as *mut PCRE2_UCHAR16
                                                                                    as *mut ::core::ffi::c_void,
                                                                                name_start_0 as *const ::core::ffi::c_void,
                                                                                name_len_0
                                                                                    .wrapping_mul(
                                                                                        (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                    ),
                                                                            );
                                                                            name[name_len_0
                                                                                as usize] =
                                                                                0 as PCRE2_UCHAR16;
                                                                            current_block =
                                                                                7268916931342848830;
                                                                        }
                                                                        11305664347990019861 => {
                                                                            if !(rc < 0 as ::core::ffi::c_int) {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            special = 0 as uint32_t;
                                                                            group = -rc - 1 as ::core::ffi::c_int;
                                                                            current_block =
                                                                                7268916931342848830;
                                                                        }
                                                                        17620377729944291641 => {
                                                                            escaped_literal =
                                                                                TRUE as BOOL;
                                                                            continue;
                                                                        }
                                                                        11058672248517667920 => {
                                                                            current_block =
                                                                                2151901123398569495;
                                                                        }
                                                                        _ => {}
                                                                    }
                                                                    match current_block {
                                                                        7268916931342848830 => {}
                                                                        _ => {
                                                                            if rc == ESC_b as ::core::ffi::c_int {
                                                                                ch = CHAR_BS as uint32_t;
                                                                            }
                                                                            if rc == ESC_v as ::core::ffi::c_int {
                                                                                ch = CHAR_VT as uint32_t;
                                                                            }
                                                                            if utf != 0 {
                                                                                chlen = _pcre2_ord2utf_16(
                                                                                    ch,
                                                                                    &raw mut temp as *mut PCRE2_UCHAR16,
                                                                                );
                                                                            } else {
                                                                                temp[0 as ::core::ffi::c_int as usize] = ch
                                                                                    as PCRE2_UCHAR16;
                                                                                chlen = 1 as ::core::ffi::c_uint;
                                                                            }
                                                                            if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                                                                && substitute_case_callout.is_none()
                                                                            {
                                                                                let mut chkcc_length_2: size_t = chlen as size_t;
                                                                                let mut chkcc_rc_2: size_t = 0;
                                                                                chkcc_rc_2 = default_substitute_case_callout(
                                                                                    &raw mut temp as *mut PCRE2_UCHAR16 as PCRE2_SPTR16,
                                                                                    chkcc_length_2,
                                                                                    buffer.offset(buff_offset as isize),
                                                                                    if overflowed != 0 { 0 as size_t } else { lengthleft },
                                                                                    &raw mut forcecase,
                                                                                    code,
                                                                                );
                                                                                if overflowed != 0 {
                                                                                    if chkcc_rc_2
                                                                                        > (!(0 as ::core::ffi::c_int as size_t))
                                                                                            .wrapping_sub(extra_needed)
                                                                                    {
                                                                                        current_block = 271895580782157815;
                                                                                        break 's_407;
                                                                                    }
                                                                                    extra_needed = extra_needed.wrapping_add(chkcc_rc_2);
                                                                                    continue;
                                                                                } else if lengthleft < chkcc_rc_2 {
                                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                        == 0 as uint32_t
                                                                                    {
                                                                                        current_block = 1593672184456546103;
                                                                                        break 's_407;
                                                                                    }
                                                                                    overflowed = TRUE as BOOL;
                                                                                    extra_needed = chkcc_rc_2.wrapping_sub(lengthleft);
                                                                                    continue;
                                                                                } else {
                                                                                    buff_offset = buff_offset.wrapping_add(chkcc_rc_2);
                                                                                    lengthleft = lengthleft.wrapping_sub(chkcc_rc_2);
                                                                                    continue;
                                                                                }
                                                                            } else {
                                                                                let mut chkmc_length_4: size_t = chlen as size_t;
                                                                                if overflowed != 0 {
                                                                                    if chkmc_length_4
                                                                                        > (!(0 as ::core::ffi::c_int as size_t))
                                                                                            .wrapping_sub(extra_needed)
                                                                                    {
                                                                                        current_block = 271895580782157815;
                                                                                        break 's_407;
                                                                                    }
                                                                                    extra_needed = extra_needed.wrapping_add(chkmc_length_4);
                                                                                    continue;
                                                                                } else if lengthleft < chkmc_length_4 {
                                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                        == 0 as uint32_t
                                                                                    {
                                                                                        current_block = 1593672184456546103;
                                                                                        break 's_407;
                                                                                    }
                                                                                    overflowed = TRUE as BOOL;
                                                                                    extra_needed = chkmc_length_4.wrapping_sub(lengthleft);
                                                                                    continue;
                                                                                } else {
                                                                                    memcpy(
                                                                                        buffer.offset(buff_offset as isize)
                                                                                            as *mut ::core::ffi::c_void,
                                                                                        &raw mut temp as *mut PCRE2_UCHAR16
                                                                                            as *const ::core::ffi::c_void,
                                                                                        chkmc_length_4
                                                                                            .wrapping_mul(
                                                                                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                            ),
                                                                                    );
                                                                                    buff_offset = buff_offset.wrapping_add(chkmc_length_4);
                                                                                    lengthleft = lengthleft.wrapping_sub(chkmc_length_4);
                                                                                    continue;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                27 => {
                                                                    current_block =
                                                                        17058499098102203106;
                                                                    match current_block {
                                                                        17058499098102203106 => {
                                                                            let mut name_len_0: size_t = 0;
                                                                            let mut name_start_0: PCRE2_SPTR16 = ::core::ptr::null::<
                                                                                PCRE2_UCHAR16,
                                                                            >();
                                                                            if ptr >= repend
                                                                                || *ptr as ::core::ffi::c_int != CHAR_LESS_THAN_SIGN
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            ptr = ptr.offset(1);
                                                                            name_start_0 = ptr;
                                                                            if read_name_subst(
                                                                                &raw mut ptr,
                                                                                repend,
                                                                                utf,
                                                                                (*code)
                                                                                    .tables
                                                                                    .offset(
                                                                                    ctypes_offset
                                                                                        as isize,
                                                                                ),
                                                                            ) == 0
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            name_len_0 = ptr.offset_from(name_start_0)
                                                                                as isize as size_t;
                                                                            if ptr >= repend
                                                                                || *ptr as ::core::ffi::c_int != CHAR_GREATER_THAN_SIGN
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            ptr = ptr.offset(1);
                                                                            special = 0 as uint32_t;
                                                                            group = -(1 as ::core::ffi::c_int);
                                                                            memcpy(
                                                                                &raw mut name as *mut PCRE2_UCHAR16
                                                                                    as *mut ::core::ffi::c_void,
                                                                                name_start_0 as *const ::core::ffi::c_void,
                                                                                name_len_0
                                                                                    .wrapping_mul(
                                                                                        (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                    ),
                                                                            );
                                                                            name[name_len_0
                                                                                as usize] =
                                                                                0 as PCRE2_UCHAR16;
                                                                            current_block =
                                                                                7268916931342848830;
                                                                        }
                                                                        11305664347990019861 => {
                                                                            if !(rc < 0 as ::core::ffi::c_int) {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            special = 0 as uint32_t;
                                                                            group = -rc - 1 as ::core::ffi::c_int;
                                                                            current_block =
                                                                                7268916931342848830;
                                                                        }
                                                                        17620377729944291641 => {
                                                                            escaped_literal =
                                                                                TRUE as BOOL;
                                                                            continue;
                                                                        }
                                                                        11058672248517667920 => {
                                                                            current_block =
                                                                                2151901123398569495;
                                                                        }
                                                                        _ => {}
                                                                    }
                                                                    match current_block {
                                                                        7268916931342848830 => {}
                                                                        _ => {
                                                                            if rc == ESC_b as ::core::ffi::c_int {
                                                                                ch = CHAR_BS as uint32_t;
                                                                            }
                                                                            if rc == ESC_v as ::core::ffi::c_int {
                                                                                ch = CHAR_VT as uint32_t;
                                                                            }
                                                                            if utf != 0 {
                                                                                chlen = _pcre2_ord2utf_16(
                                                                                    ch,
                                                                                    &raw mut temp as *mut PCRE2_UCHAR16,
                                                                                );
                                                                            } else {
                                                                                temp[0 as ::core::ffi::c_int as usize] = ch
                                                                                    as PCRE2_UCHAR16;
                                                                                chlen = 1 as ::core::ffi::c_uint;
                                                                            }
                                                                            if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                                                                && substitute_case_callout.is_none()
                                                                            {
                                                                                let mut chkcc_length_2: size_t = chlen as size_t;
                                                                                let mut chkcc_rc_2: size_t = 0;
                                                                                chkcc_rc_2 = default_substitute_case_callout(
                                                                                    &raw mut temp as *mut PCRE2_UCHAR16 as PCRE2_SPTR16,
                                                                                    chkcc_length_2,
                                                                                    buffer.offset(buff_offset as isize),
                                                                                    if overflowed != 0 { 0 as size_t } else { lengthleft },
                                                                                    &raw mut forcecase,
                                                                                    code,
                                                                                );
                                                                                if overflowed != 0 {
                                                                                    if chkcc_rc_2
                                                                                        > (!(0 as ::core::ffi::c_int as size_t))
                                                                                            .wrapping_sub(extra_needed)
                                                                                    {
                                                                                        current_block = 271895580782157815;
                                                                                        break 's_407;
                                                                                    }
                                                                                    extra_needed = extra_needed.wrapping_add(chkcc_rc_2);
                                                                                    continue;
                                                                                } else if lengthleft < chkcc_rc_2 {
                                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                        == 0 as uint32_t
                                                                                    {
                                                                                        current_block = 1593672184456546103;
                                                                                        break 's_407;
                                                                                    }
                                                                                    overflowed = TRUE as BOOL;
                                                                                    extra_needed = chkcc_rc_2.wrapping_sub(lengthleft);
                                                                                    continue;
                                                                                } else {
                                                                                    buff_offset = buff_offset.wrapping_add(chkcc_rc_2);
                                                                                    lengthleft = lengthleft.wrapping_sub(chkcc_rc_2);
                                                                                    continue;
                                                                                }
                                                                            } else {
                                                                                let mut chkmc_length_4: size_t = chlen as size_t;
                                                                                if overflowed != 0 {
                                                                                    if chkmc_length_4
                                                                                        > (!(0 as ::core::ffi::c_int as size_t))
                                                                                            .wrapping_sub(extra_needed)
                                                                                    {
                                                                                        current_block = 271895580782157815;
                                                                                        break 's_407;
                                                                                    }
                                                                                    extra_needed = extra_needed.wrapping_add(chkmc_length_4);
                                                                                    continue;
                                                                                } else if lengthleft < chkmc_length_4 {
                                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                        == 0 as uint32_t
                                                                                    {
                                                                                        current_block = 1593672184456546103;
                                                                                        break 's_407;
                                                                                    }
                                                                                    overflowed = TRUE as BOOL;
                                                                                    extra_needed = chkmc_length_4.wrapping_sub(lengthleft);
                                                                                    continue;
                                                                                } else {
                                                                                    memcpy(
                                                                                        buffer.offset(buff_offset as isize)
                                                                                            as *mut ::core::ffi::c_void,
                                                                                        &raw mut temp as *mut PCRE2_UCHAR16
                                                                                            as *const ::core::ffi::c_void,
                                                                                        chkmc_length_4
                                                                                            .wrapping_mul(
                                                                                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                            ),
                                                                                    );
                                                                                    buff_offset = buff_offset.wrapping_add(chkmc_length_4);
                                                                                    lengthleft = lengthleft.wrapping_sub(chkmc_length_4);
                                                                                    continue;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                _ => {
                                                                    current_block =
                                                                        11305664347990019861;
                                                                    match current_block {
                                                                        17058499098102203106 => {
                                                                            let mut name_len_0: size_t = 0;
                                                                            let mut name_start_0: PCRE2_SPTR16 = ::core::ptr::null::<
                                                                                PCRE2_UCHAR16,
                                                                            >();
                                                                            if ptr >= repend
                                                                                || *ptr as ::core::ffi::c_int != CHAR_LESS_THAN_SIGN
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            ptr = ptr.offset(1);
                                                                            name_start_0 = ptr;
                                                                            if read_name_subst(
                                                                                &raw mut ptr,
                                                                                repend,
                                                                                utf,
                                                                                (*code)
                                                                                    .tables
                                                                                    .offset(
                                                                                    ctypes_offset
                                                                                        as isize,
                                                                                ),
                                                                            ) == 0
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            name_len_0 = ptr.offset_from(name_start_0)
                                                                                as isize as size_t;
                                                                            if ptr >= repend
                                                                                || *ptr as ::core::ffi::c_int != CHAR_GREATER_THAN_SIGN
                                                                            {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            ptr = ptr.offset(1);
                                                                            special = 0 as uint32_t;
                                                                            group = -(1 as ::core::ffi::c_int);
                                                                            memcpy(
                                                                                &raw mut name as *mut PCRE2_UCHAR16
                                                                                    as *mut ::core::ffi::c_void,
                                                                                name_start_0 as *const ::core::ffi::c_void,
                                                                                name_len_0
                                                                                    .wrapping_mul(
                                                                                        (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                    ),
                                                                            );
                                                                            name[name_len_0
                                                                                as usize] =
                                                                                0 as PCRE2_UCHAR16;
                                                                            current_block =
                                                                                7268916931342848830;
                                                                        }
                                                                        11305664347990019861 => {
                                                                            if !(rc < 0 as ::core::ffi::c_int) {
                                                                                current_block = 13886604107758561977;
                                                                                break 's_407;
                                                                            }
                                                                            special = 0 as uint32_t;
                                                                            group = -rc - 1 as ::core::ffi::c_int;
                                                                            current_block =
                                                                                7268916931342848830;
                                                                        }
                                                                        17620377729944291641 => {
                                                                            escaped_literal =
                                                                                TRUE as BOOL;
                                                                            continue;
                                                                        }
                                                                        11058672248517667920 => {
                                                                            current_block =
                                                                                2151901123398569495;
                                                                        }
                                                                        _ => {}
                                                                    }
                                                                    match current_block {
                                                                        7268916931342848830 => {}
                                                                        _ => {
                                                                            if rc == ESC_b as ::core::ffi::c_int {
                                                                                ch = CHAR_BS as uint32_t;
                                                                            }
                                                                            if rc == ESC_v as ::core::ffi::c_int {
                                                                                ch = CHAR_VT as uint32_t;
                                                                            }
                                                                            if utf != 0 {
                                                                                chlen = _pcre2_ord2utf_16(
                                                                                    ch,
                                                                                    &raw mut temp as *mut PCRE2_UCHAR16,
                                                                                );
                                                                            } else {
                                                                                temp[0 as ::core::ffi::c_int as usize] = ch
                                                                                    as PCRE2_UCHAR16;
                                                                                chlen = 1 as ::core::ffi::c_uint;
                                                                            }
                                                                            if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                                                                && substitute_case_callout.is_none()
                                                                            {
                                                                                let mut chkcc_length_2: size_t = chlen as size_t;
                                                                                let mut chkcc_rc_2: size_t = 0;
                                                                                chkcc_rc_2 = default_substitute_case_callout(
                                                                                    &raw mut temp as *mut PCRE2_UCHAR16 as PCRE2_SPTR16,
                                                                                    chkcc_length_2,
                                                                                    buffer.offset(buff_offset as isize),
                                                                                    if overflowed != 0 { 0 as size_t } else { lengthleft },
                                                                                    &raw mut forcecase,
                                                                                    code,
                                                                                );
                                                                                if overflowed != 0 {
                                                                                    if chkcc_rc_2
                                                                                        > (!(0 as ::core::ffi::c_int as size_t))
                                                                                            .wrapping_sub(extra_needed)
                                                                                    {
                                                                                        current_block = 271895580782157815;
                                                                                        break 's_407;
                                                                                    }
                                                                                    extra_needed = extra_needed.wrapping_add(chkcc_rc_2);
                                                                                    continue;
                                                                                } else if lengthleft < chkcc_rc_2 {
                                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                        == 0 as uint32_t
                                                                                    {
                                                                                        current_block = 1593672184456546103;
                                                                                        break 's_407;
                                                                                    }
                                                                                    overflowed = TRUE as BOOL;
                                                                                    extra_needed = chkcc_rc_2.wrapping_sub(lengthleft);
                                                                                    continue;
                                                                                } else {
                                                                                    buff_offset = buff_offset.wrapping_add(chkcc_rc_2);
                                                                                    lengthleft = lengthleft.wrapping_sub(chkcc_rc_2);
                                                                                    continue;
                                                                                }
                                                                            } else {
                                                                                let mut chkmc_length_4: size_t = chlen as size_t;
                                                                                if overflowed != 0 {
                                                                                    if chkmc_length_4
                                                                                        > (!(0 as ::core::ffi::c_int as size_t))
                                                                                            .wrapping_sub(extra_needed)
                                                                                    {
                                                                                        current_block = 271895580782157815;
                                                                                        break 's_407;
                                                                                    }
                                                                                    extra_needed = extra_needed.wrapping_add(chkmc_length_4);
                                                                                    continue;
                                                                                } else if lengthleft < chkmc_length_4 {
                                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                        == 0 as uint32_t
                                                                                    {
                                                                                        current_block = 1593672184456546103;
                                                                                        break 's_407;
                                                                                    }
                                                                                    overflowed = TRUE as BOOL;
                                                                                    extra_needed = chkmc_length_4.wrapping_sub(lengthleft);
                                                                                    continue;
                                                                                } else {
                                                                                    memcpy(
                                                                                        buffer.offset(buff_offset as isize)
                                                                                            as *mut ::core::ffi::c_void,
                                                                                        &raw mut temp as *mut PCRE2_UCHAR16
                                                                                            as *const ::core::ffi::c_void,
                                                                                        chkmc_length_4
                                                                                            .wrapping_mul(
                                                                                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                                            ),
                                                                                    );
                                                                                    buff_offset = buff_offset.wrapping_add(chkmc_length_4);
                                                                                    lengthleft = lengthleft.wrapping_sub(chkmc_length_4);
                                                                                    continue;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        match current_block {
                                                            7268916931342848830 => {}
                                                            _ => {
                                                                if substitute_case_callout.is_some()
                                                                    && forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                                                {
                                                                    let mut chars_outstanding: size_t = buff_offset
                                                                        .wrapping_sub(casestart_offset)
                                                                        .wrapping_add(
                                                                            extra_needed.wrapping_sub(casestart_extra_needed),
                                                                        );
                                                                    if chars_outstanding > 0 as size_t {
                                                                        if overflowed != 0 {
                                                                            let mut guess: size_t = pessimistic_case_inflation(
                                                                                chars_outstanding,
                                                                            );
                                                                            if guess
                                                                                > (!(0 as ::core::ffi::c_int as size_t))
                                                                                    .wrapping_sub(extra_needed)
                                                                            {
                                                                                current_block = 271895580782157815;
                                                                                break 's_407;
                                                                            }
                                                                            extra_needed = extra_needed.wrapping_add(guess);
                                                                        } else {
                                                                            lengthleft = lengthleft
                                                                                .wrapping_add(buff_offset.wrapping_sub(casestart_offset));
                                                                            buff_offset = casestart_offset;
                                                                            let mut chkcc_length_1: size_t = chars_outstanding;
                                                                            let mut chkcc_rc_1: size_t = 0;
                                                                            chkcc_rc_1 = do_case_copy(
                                                                                buffer.offset(buff_offset as isize),
                                                                                chkcc_length_1,
                                                                                lengthleft,
                                                                                &raw mut forcecase,
                                                                                utf,
                                                                                substitute_case_callout,
                                                                                substitute_case_callout_data,
                                                                            );
                                                                            if chkcc_rc_1 == !(0 as ::core::ffi::c_int as size_t) {
                                                                                current_block = 4648626524714889401;
                                                                                break 's_407;
                                                                            }
                                                                            if lengthleft < chkcc_rc_1 {
                                                                                if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                                    == 0 as uint32_t
                                                                                {
                                                                                    current_block = 1593672184456546103;
                                                                                    break 's_407;
                                                                                }
                                                                                overflowed = TRUE as BOOL;
                                                                                extra_needed = chkcc_rc_1.wrapping_sub(lengthleft);
                                                                            } else {
                                                                                buff_offset = buff_offset.wrapping_add(chkcc_rc_1);
                                                                                lengthleft = lengthleft.wrapping_sub(chkcc_rc_1);
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                forcecase = new_forcecase;
                                                                casestart_offset = buff_offset;
                                                                casestart_extra_needed =
                                                                    extra_needed;
                                                                continue;
                                                            }
                                                        }
                                                    } else {
                                                        ch_start =
                                                            ::core::ptr::null::<PCRE2_UCHAR16>();
                                                        current_block = 11728966381135444667;
                                                    }
                                                    match current_block {
                                                        11728966381135444667 => {}
                                                        _ => {
                                                            match current_block {
                                                                7268916931342848830 => {
                                                                    if group
                                                                        < 0 as ::core::ffi::c_int
                                                                    {
                                                                        let mut first: PCRE2_SPTR16 = ::core::ptr::null::<
                                                                            PCRE2_UCHAR16,
                                                                        >();
                                                                        let mut last: PCRE2_SPTR16 =
                                                                            ::core::ptr::null::<
                                                                                PCRE2_UCHAR16,
                                                                            >(
                                                                            );
                                                                        let mut entry: PCRE2_SPTR16 = ::core::ptr::null::<
                                                                            PCRE2_UCHAR16,
                                                                        >();
                                                                        rc = pcre2_substring_nametable_scan_16(
                                                                            code,
                                                                            &raw mut name as *mut PCRE2_UCHAR16 as PCRE2_SPTR16,
                                                                            &raw mut first,
                                                                            &raw mut last,
                                                                        );
                                                                        if rc == PCRE2_ERROR_NOSUBSTRING
                                                                            && suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET as uint32_t
                                                                                != 0 as uint32_t
                                                                        {
                                                                            group = (*code).top_bracket as ::core::ffi::c_int
                                                                                + 1 as ::core::ffi::c_int;
                                                                        } else {
                                                                            if rc < 0 as ::core::ffi::c_int {
                                                                                current_block = 12248883293374827016;
                                                                                break 's_407;
                                                                            }
                                                                            entry = first;
                                                                            while entry <= last {
                                                                                let mut ng: uint32_t = *entry
                                                                                    .offset(0 as ::core::ffi::c_int as isize) as uint32_t;
                                                                                if ng < ovector_count {
                                                                                    if group < 0 as ::core::ffi::c_int {
                                                                                        group = ng as ::core::ffi::c_int;
                                                                                    }
                                                                                    if *ovector.offset(ng.wrapping_mul(2 as uint32_t) as isize)
                                                                                        != PCRE2_UNSET
                                                                                    {
                                                                                        group = ng as ::core::ffi::c_int;
                                                                                        break;
                                                                                    }
                                                                                }
                                                                                entry = entry.offset(rc as isize);
                                                                            }
                                                                            if group < 0 as ::core::ffi::c_int {
                                                                                group = *first.offset(0 as ::core::ffi::c_int as isize)
                                                                                    as ::core::ffi::c_int;
                                                                            }
                                                                        }
                                                                    }
                                                                    rc = pcre2_substring_length_bynumber_16(
                                                                        match_data,
                                                                        group as uint32_t,
                                                                        &raw mut sublength,
                                                                    );
                                                                    if rc < 0 as ::core::ffi::c_int
                                                                    {
                                                                        if rc == PCRE2_ERROR_NOSUBSTRING
                                                                            && suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET as uint32_t
                                                                                != 0 as uint32_t
                                                                        {
                                                                            rc = PCRE2_ERROR_UNSET;
                                                                        }
                                                                        if rc != PCRE2_ERROR_UNSET {
                                                                            current_block = 12248883293374827016;
                                                                            break 's_407;
                                                                        }
                                                                        if special == 0 as uint32_t
                                                                        {
                                                                            if suboptions & PCRE2_SUBSTITUTE_UNSET_EMPTY as uint32_t
                                                                                != 0 as uint32_t
                                                                            {
                                                                                continue;
                                                                            } else {
                                                                                current_block = 12248883293374827016;
                                                                                break 's_407;
                                                                            }
                                                                        }
                                                                    }
                                                                    if special != 0 as uint32_t {
                                                                        if special
                                                                            == CHAR_MINUS
                                                                                as uint32_t
                                                                        {
                                                                            if rc == 0 as ::core::ffi::c_int {
                                                                                current_block = 12659877271294107784;
                                                                            } else {
                                                                                text2_start = text1_start;
                                                                                text2_end = text1_end;
                                                                                current_block = 15840544472253023770;
                                                                            }
                                                                        } else {
                                                                            current_block = 15840544472253023770;
                                                                        }
                                                                        match current_block {
                                                                            12659877271294107784 => {}
                                                                            _ => {
                                                                                if ptrstackptr >= PTR_STACK_SIZE as uint32_t {
                                                                                    current_block = 11196033572913570296;
                                                                                    break 's_407;
                                                                                }
                                                                                let fresh0 = ptrstackptr;
                                                                                ptrstackptr = ptrstackptr.wrapping_add(1);
                                                                                ptrstack[fresh0 as usize] = ptr;
                                                                                let fresh1 = ptrstackptr;
                                                                                ptrstackptr = ptrstackptr.wrapping_add(1);
                                                                                ptrstack[fresh1 as usize] = repend;
                                                                                if rc == 0 as ::core::ffi::c_int {
                                                                                    ptr = text1_start;
                                                                                    repend = text1_end;
                                                                                } else {
                                                                                    ptr = text2_start;
                                                                                    repend = text2_end;
                                                                                }
                                                                                continue;
                                                                            }
                                                                        }
                                                                    }
                                                                    subptr = subject
                                                                        .offset(
                                                                            *ovector.offset((group * 2 as ::core::ffi::c_int) as isize)
                                                                                as isize,
                                                                        );
                                                                    subptrend = subject
                                                                        .offset(
                                                                            *ovector
                                                                                .offset(
                                                                                    (group * 2 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                                                        as isize,
                                                                                ) as isize,
                                                                        );
                                                                }
                                                                _ => {}
                                                            }
                                                            if forcecase.to_case
                                                                != PCRE2_SUBSTITUTE_CASE_NONE
                                                                && substitute_case_callout.is_none()
                                                            {
                                                                let mut chkcc_length_0: size_t =
                                                                    subptrend.offset_from(subptr)
                                                                        as isize
                                                                        as size_t;
                                                                let mut chkcc_rc_0: size_t = 0;
                                                                chkcc_rc_0 =
                                                                    default_substitute_case_callout(
                                                                        subptr,
                                                                        chkcc_length_0,
                                                                        buffer.offset(
                                                                            buff_offset as isize,
                                                                        ),
                                                                        if overflowed != 0 {
                                                                            0 as size_t
                                                                        } else {
                                                                            lengthleft
                                                                        },
                                                                        &raw mut forcecase,
                                                                        code,
                                                                    );
                                                                if overflowed != 0 {
                                                                    if chkcc_rc_0
                                                                        > (!(0 as ::core::ffi::c_int
                                                                            as size_t))
                                                                            .wrapping_sub(
                                                                                extra_needed,
                                                                            )
                                                                    {
                                                                        current_block =
                                                                            271895580782157815;
                                                                        break 's_407;
                                                                    }
                                                                    extra_needed = extra_needed
                                                                        .wrapping_add(chkcc_rc_0);
                                                                    continue;
                                                                } else if lengthleft < chkcc_rc_0 {
                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                        == 0 as uint32_t
                                                                    {
                                                                        current_block = 1593672184456546103;
                                                                        break 's_407;
                                                                    }
                                                                    overflowed = TRUE as BOOL;
                                                                    extra_needed = chkcc_rc_0
                                                                        .wrapping_sub(lengthleft);
                                                                    continue;
                                                                } else {
                                                                    buff_offset = buff_offset
                                                                        .wrapping_add(chkcc_rc_0);
                                                                    lengthleft = lengthleft
                                                                        .wrapping_sub(chkcc_rc_0);
                                                                    continue;
                                                                }
                                                            } else {
                                                                let mut chkmc_length_3: size_t =
                                                                    subptrend.offset_from(subptr)
                                                                        as isize
                                                                        as size_t;
                                                                if overflowed != 0 {
                                                                    if chkmc_length_3
                                                                        > (!(0 as ::core::ffi::c_int
                                                                            as size_t))
                                                                            .wrapping_sub(
                                                                                extra_needed,
                                                                            )
                                                                    {
                                                                        current_block =
                                                                            271895580782157815;
                                                                        break 's_407;
                                                                    }
                                                                    extra_needed = extra_needed
                                                                        .wrapping_add(
                                                                            chkmc_length_3,
                                                                        );
                                                                    continue;
                                                                } else if lengthleft
                                                                    < chkmc_length_3
                                                                {
                                                                    if suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                                        == 0 as uint32_t
                                                                    {
                                                                        current_block = 1593672184456546103;
                                                                        break 's_407;
                                                                    }
                                                                    overflowed = TRUE as BOOL;
                                                                    extra_needed = chkmc_length_3
                                                                        .wrapping_sub(lengthleft);
                                                                    continue;
                                                                } else {
                                                                    memcpy(
                                                                        buffer.offset(buff_offset as isize)
                                                                            as *mut ::core::ffi::c_void,
                                                                        subptr as *const ::core::ffi::c_void,
                                                                        chkmc_length_3
                                                                            .wrapping_mul(
                                                                                (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                            ),
                                                                    );
                                                                    buff_offset = buff_offset
                                                                        .wrapping_add(
                                                                            chkmc_length_3,
                                                                        );
                                                                    lengthleft = lengthleft
                                                                        .wrapping_sub(
                                                                            chkmc_length_3,
                                                                        );
                                                                    continue;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                ch_start = ptr;
                                                let fresh2 = ptr;
                                                ptr = ptr.offset(1);
                                                ch = *fresh2 as uint32_t;
                                                if utf != 0
                                                    && ch & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                                {
                                                    let fresh3 = ptr;
                                                    ptr = ptr.offset(1);
                                                    ch = ((ch & 0x3ff as uint32_t)
                                                        << 10 as ::core::ffi::c_int
                                                        | *fresh3 as uint32_t & 0x3ff as uint32_t)
                                                        .wrapping_add(0x10000 as uint32_t);
                                                }
                                                if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                                    && substitute_case_callout.is_none()
                                                {
                                                    let mut chkcc_length_3: size_t = ptr
                                                        .offset_from(ch_start)
                                                        as isize
                                                        as size_t;
                                                    let mut chkcc_rc_3: size_t = 0;
                                                    chkcc_rc_3 = default_substitute_case_callout(
                                                        ch_start,
                                                        chkcc_length_3,
                                                        buffer.offset(buff_offset as isize),
                                                        if overflowed != 0 {
                                                            0 as size_t
                                                        } else {
                                                            lengthleft
                                                        },
                                                        &raw mut forcecase,
                                                        code,
                                                    );
                                                    if overflowed != 0 {
                                                        if chkcc_rc_3
                                                            > (!(0 as ::core::ffi::c_int as size_t))
                                                                .wrapping_sub(extra_needed)
                                                        {
                                                            current_block = 271895580782157815;
                                                            break 's_407;
                                                        }
                                                        extra_needed =
                                                            extra_needed.wrapping_add(chkcc_rc_3);
                                                    } else if lengthleft < chkcc_rc_3 {
                                                        if suboptions
                                                            & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
                                                                as uint32_t
                                                            == 0 as uint32_t
                                                        {
                                                            current_block = 1593672184456546103;
                                                            break 's_407;
                                                        }
                                                        overflowed = TRUE as BOOL;
                                                        extra_needed =
                                                            chkcc_rc_3.wrapping_sub(lengthleft);
                                                    } else {
                                                        buff_offset =
                                                            buff_offset.wrapping_add(chkcc_rc_3);
                                                        lengthleft =
                                                            lengthleft.wrapping_sub(chkcc_rc_3);
                                                    }
                                                } else {
                                                    let mut chkmc_length_5: size_t = ptr
                                                        .offset_from(ch_start)
                                                        as isize
                                                        as size_t;
                                                    if overflowed != 0 {
                                                        if chkmc_length_5
                                                            > (!(0 as ::core::ffi::c_int as size_t))
                                                                .wrapping_sub(extra_needed)
                                                        {
                                                            current_block = 271895580782157815;
                                                            break 's_407;
                                                        }
                                                        extra_needed = extra_needed
                                                            .wrapping_add(chkmc_length_5);
                                                    } else if lengthleft < chkmc_length_5 {
                                                        if suboptions
                                                            & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
                                                                as uint32_t
                                                            == 0 as uint32_t
                                                        {
                                                            current_block = 1593672184456546103;
                                                            break 's_407;
                                                        }
                                                        overflowed = TRUE as BOOL;
                                                        extra_needed =
                                                            chkmc_length_5.wrapping_sub(lengthleft);
                                                    } else {
                                                        memcpy(
                                                            buffer.offset(buff_offset as isize)
                                                                as *mut ::core::ffi::c_void,
                                                            ch_start as *const ::core::ffi::c_void,
                                                            chkmc_length_5.wrapping_mul(
                                                                (PCRE2_CODE_UNIT_WIDTH
                                                                    / 8 as ::core::ffi::c_int)
                                                                    as size_t,
                                                            ),
                                                        );
                                                        buff_offset = buff_offset
                                                            .wrapping_add(chkmc_length_5);
                                                        lengthleft =
                                                            lengthleft.wrapping_sub(chkmc_length_5);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if substitute_case_callout.is_some()
                                        && forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                    {
                                        let mut chars_outstanding_0: size_t = buff_offset
                                            .wrapping_sub(casestart_offset)
                                            .wrapping_add(
                                                extra_needed.wrapping_sub(casestart_extra_needed),
                                            );
                                        if chars_outstanding_0 > 0 as size_t {
                                            if overflowed != 0 {
                                                let mut guess_0: size_t =
                                                    pessimistic_case_inflation(chars_outstanding_0);
                                                if guess_0
                                                    > (!(0 as ::core::ffi::c_int as size_t))
                                                        .wrapping_sub(extra_needed)
                                                {
                                                    current_block = 271895580782157815;
                                                    break;
                                                }
                                                extra_needed = extra_needed.wrapping_add(guess_0);
                                            } else {
                                                lengthleft = lengthleft.wrapping_add(
                                                    buff_offset.wrapping_sub(casestart_offset),
                                                );
                                                buff_offset = casestart_offset;
                                                let mut chkcc_length_4: size_t =
                                                    chars_outstanding_0;
                                                let mut chkcc_rc_4: size_t = 0;
                                                chkcc_rc_4 = do_case_copy(
                                                    buffer.offset(buff_offset as isize),
                                                    chkcc_length_4,
                                                    lengthleft,
                                                    &raw mut forcecase,
                                                    utf,
                                                    substitute_case_callout,
                                                    substitute_case_callout_data,
                                                );
                                                if chkcc_rc_4
                                                    == !(0 as ::core::ffi::c_int as size_t)
                                                {
                                                    current_block = 4648626524714889401;
                                                    break;
                                                }
                                                if lengthleft < chkcc_rc_4 {
                                                    if suboptions
                                                        & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
                                                            as uint32_t
                                                        == 0 as uint32_t
                                                    {
                                                        current_block = 1593672184456546103;
                                                        break;
                                                    }
                                                    overflowed = TRUE as BOOL;
                                                    extra_needed =
                                                        chkcc_rc_4.wrapping_sub(lengthleft);
                                                } else {
                                                    buff_offset =
                                                        buff_offset.wrapping_add(chkcc_rc_4);
                                                    lengthleft =
                                                        lengthleft.wrapping_sub(chkcc_rc_4);
                                                }
                                            }
                                        }
                                    }
                                    if !mcontext.is_null()
                                        && (*mcontext).substitute_callout.is_some()
                                    {
                                        if overflowed == 0 {
                                            scb.subscount = subs as uint32_t;
                                            scb.output_offsets[1 as ::core::ffi::c_int as usize] =
                                                buff_offset;
                                            rc = (*mcontext)
                                                .substitute_callout
                                                .expect("non-null function pointer")(
                                                &raw mut scb,
                                                (*mcontext).substitute_callout_data,
                                            );
                                            if rc != 0 as ::core::ffi::c_int {
                                                let mut newlength: size_t = scb.output_offsets
                                                    [1 as ::core::ffi::c_int as usize]
                                                    .wrapping_sub(
                                                        scb.output_offsets
                                                            [0 as ::core::ffi::c_int as usize],
                                                    );
                                                let mut oldlength: size_t = (*ovector
                                                    .offset(1 as ::core::ffi::c_int as isize))
                                                .wrapping_sub(
                                                    *ovector
                                                        .offset(0 as ::core::ffi::c_int as isize),
                                                );
                                                buff_offset = buff_offset.wrapping_sub(newlength);
                                                lengthleft = lengthleft.wrapping_add(newlength);
                                                if replacement_only == 0 {
                                                    let mut chkmc_length_6: size_t = oldlength;
                                                    if overflowed != 0 {
                                                        if chkmc_length_6
                                                            > (!(0 as ::core::ffi::c_int as size_t))
                                                                .wrapping_sub(extra_needed)
                                                        {
                                                            current_block = 271895580782157815;
                                                            break;
                                                        }
                                                        extra_needed = extra_needed
                                                            .wrapping_add(chkmc_length_6);
                                                    } else if lengthleft < chkmc_length_6 {
                                                        if suboptions
                                                            & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
                                                                as uint32_t
                                                            == 0 as uint32_t
                                                        {
                                                            current_block = 1593672184456546103;
                                                            break;
                                                        }
                                                        overflowed = TRUE as BOOL;
                                                        extra_needed =
                                                            chkmc_length_6.wrapping_sub(lengthleft);
                                                    } else {
                                                        memcpy(
                                                            buffer.offset(buff_offset as isize)
                                                                as *mut ::core::ffi::c_void,
                                                            subject.offset(*ovector.offset(
                                                                0 as ::core::ffi::c_int as isize,
                                                            )
                                                                as isize)
                                                                as *const ::core::ffi::c_void,
                                                            chkmc_length_6.wrapping_mul(
                                                                (PCRE2_CODE_UNIT_WIDTH
                                                                    / 8 as ::core::ffi::c_int)
                                                                    as size_t,
                                                            ),
                                                        );
                                                        buff_offset = buff_offset
                                                            .wrapping_add(chkmc_length_6);
                                                        lengthleft =
                                                            lengthleft.wrapping_sub(chkmc_length_6);
                                                    }
                                                }
                                                if rc < 0 as ::core::ffi::c_int {
                                                    suboptions = (suboptions as ::core::ffi::c_uint
                                                        & !PCRE2_SUBSTITUTE_GLOBAL)
                                                        as uint32_t;
                                                }
                                            }
                                        } else {
                                            let mut newlength_buf: size_t = buff_offset
                                                .wrapping_sub(
                                                    scb.output_offsets
                                                        [0 as ::core::ffi::c_int as usize],
                                                );
                                            let mut newlength_extra: size_t =
                                                extra_needed.wrapping_sub(sub_start_extra_needed);
                                            let mut newlength_0: size_t = if newlength_extra
                                                > (!(0 as ::core::ffi::c_int as size_t))
                                                    .wrapping_sub(newlength_buf)
                                            {
                                                !(0 as ::core::ffi::c_int as size_t)
                                            } else {
                                                newlength_buf.wrapping_add(newlength_extra)
                                            };
                                            let mut oldlength_0: size_t = (*ovector
                                                .offset(1 as ::core::ffi::c_int as isize))
                                            .wrapping_sub(
                                                *ovector.offset(0 as ::core::ffi::c_int as isize),
                                            );
                                            if oldlength_0 > newlength_0 {
                                                let mut additional: size_t =
                                                    oldlength_0.wrapping_sub(newlength_0);
                                                if additional
                                                    > (!(0 as ::core::ffi::c_int as size_t))
                                                        .wrapping_sub(extra_needed)
                                                {
                                                    current_block = 271895580782157815;
                                                    break;
                                                } else {
                                                    extra_needed =
                                                        extra_needed.wrapping_add(additional);
                                                }
                                            }
                                        }
                                    }
                                    if !(suboptions & PCRE2_SUBSTITUTE_GLOBAL as uint32_t
                                        == 0 as uint32_t
                                        || pcre2_next_match_16(
                                            match_data,
                                            &raw mut start_offset,
                                            &raw mut goptions,
                                        ) == 0)
                                    {
                                        continue;
                                    }
                                    start_offset =
                                        *ovector.offset(1 as ::core::ffi::c_int as isize);
                                    current_block = 11608549431655460403;
                                    break;
                                }
                            }
                        }
                        match current_block {
                            1593672184456546103 => {}
                            271895580782157815 => {}
                            10288652812641984457 => {}
                            _ => {
                                match current_block {
                                    11608549431655460403 => {
                                        if replacement_only == 0 {
                                            fraglength = length.wrapping_sub(start_offset);
                                            let mut chkmc_length_7: size_t = fraglength;
                                            if overflowed != 0 {
                                                if chkmc_length_7
                                                    > (!(0 as ::core::ffi::c_int as size_t))
                                                        .wrapping_sub(extra_needed)
                                                {
                                                    current_block = 271895580782157815;
                                                } else {
                                                    extra_needed =
                                                        extra_needed.wrapping_add(chkmc_length_7);
                                                    current_block = 16799364721554827256;
                                                }
                                            } else if lengthleft < chkmc_length_7 {
                                                if suboptions
                                                    & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as uint32_t
                                                    == 0 as uint32_t
                                                {
                                                    current_block = 1593672184456546103;
                                                } else {
                                                    overflowed = TRUE as BOOL;
                                                    extra_needed =
                                                        chkmc_length_7.wrapping_sub(lengthleft);
                                                    current_block = 16799364721554827256;
                                                }
                                            } else {
                                                memcpy(
                                                    buffer.offset(buff_offset as isize)
                                                        as *mut ::core::ffi::c_void,
                                                    subject.offset(start_offset as isize)
                                                        as *const ::core::ffi::c_void,
                                                    chkmc_length_7.wrapping_mul(
                                                        (PCRE2_CODE_UNIT_WIDTH
                                                            / 8 as ::core::ffi::c_int)
                                                            as size_t,
                                                    ),
                                                );
                                                buff_offset =
                                                    buff_offset.wrapping_add(chkmc_length_7);
                                                lengthleft =
                                                    lengthleft.wrapping_sub(chkmc_length_7);
                                                current_block = 16799364721554827256;
                                            }
                                        } else {
                                            current_block = 16799364721554827256;
                                        }
                                        match current_block {
                                            1593672184456546103 => {}
                                            271895580782157815 => {}
                                            _ => {
                                                temp[0 as ::core::ffi::c_int as usize] =
                                                    0 as PCRE2_UCHAR16;
                                                let mut chkmc_length_8: size_t = 1 as size_t;
                                                if overflowed != 0 {
                                                    if chkmc_length_8
                                                        > (!(0 as ::core::ffi::c_int as size_t))
                                                            .wrapping_sub(extra_needed)
                                                    {
                                                        current_block = 271895580782157815;
                                                    } else {
                                                        extra_needed = extra_needed
                                                            .wrapping_add(chkmc_length_8);
                                                        current_block = 11169118919889928053;
                                                    }
                                                } else if lengthleft < chkmc_length_8 {
                                                    if suboptions
                                                        & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
                                                            as uint32_t
                                                        == 0 as uint32_t
                                                    {
                                                        current_block = 1593672184456546103;
                                                    } else {
                                                        overflowed = TRUE as BOOL;
                                                        extra_needed =
                                                            chkmc_length_8.wrapping_sub(lengthleft);
                                                        current_block = 11169118919889928053;
                                                    }
                                                } else {
                                                    memcpy(
                                                        buffer.offset(buff_offset as isize)
                                                            as *mut ::core::ffi::c_void,
                                                        &raw mut temp as *mut PCRE2_UCHAR16
                                                            as *const ::core::ffi::c_void,
                                                        chkmc_length_8.wrapping_mul(
                                                            (PCRE2_CODE_UNIT_WIDTH
                                                                / 8 as ::core::ffi::c_int)
                                                                as size_t,
                                                        ),
                                                    );
                                                    buff_offset =
                                                        buff_offset.wrapping_add(chkmc_length_8);
                                                    lengthleft =
                                                        lengthleft.wrapping_sub(chkmc_length_8);
                                                    current_block = 11169118919889928053;
                                                }
                                                match current_block {
                                                    271895580782157815 => {}
                                                    1593672184456546103 => {}
                                                    _ => {
                                                        if overflowed != 0 {
                                                            rc = PCRE2_ERROR_NOMEMORY;
                                                            if extra_needed
                                                                > (!(0 as ::core::ffi::c_int
                                                                    as size_t))
                                                                    .wrapping_sub(buff_length)
                                                            {
                                                                current_block = 271895580782157815;
                                                            } else {
                                                                *blength = buff_length
                                                                    .wrapping_add(extra_needed);
                                                                current_block =
                                                                    10288652812641984457;
                                                            }
                                                        } else {
                                                            rc = subs;
                                                            *blength = buff_offset
                                                                .wrapping_sub(1 as size_t);
                                                            current_block = 10288652812641984457;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    13886604107758561977 => {
                                        rc = PCRE2_ERROR_BADREPESCAPE;
                                        current_block = 12248883293374827016;
                                    }
                                    4648626524714889401 => {
                                        rc = PCRE2_ERROR_REPLACECASE;
                                        current_block = 10288652812641984457;
                                    }
                                    11196033572913570296 => {
                                        rc = PCRE2_ERROR_BADREPLACEMENT;
                                        current_block = 12248883293374827016;
                                    }
                                    _ => {}
                                }
                                match current_block {
                                    10288652812641984457 => {}
                                    271895580782157815 => {}
                                    1593672184456546103 => {}
                                    _ => {
                                        *blength = ptr.offset_from(replacement)
                                            as isize
                                            as size_t;
                                        current_block = 10288652812641984457;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                match current_block {
                    10288652812641984457 => {}
                    _ => match current_block {
                        1593672184456546103 => {
                            rc = PCRE2_ERROR_NOMEMORY;
                        }
                        _ => {
                            rc = PCRE2_ERROR_TOOLARGEREPLACE;
                        }
                    },
                }
            }
        }
        _ => {}
    }
    if !internal_match_data.is_null() {
        pcre2_match_data_free_16(internal_match_data);
    } else {
        (*match_data).rc = rc;
    }
    return rc;
}
pub const PCRE2_CODE_UNIT_WIDTH: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const MAX_NAME_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
