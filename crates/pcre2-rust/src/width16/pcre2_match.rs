// Translated from PCRE2 10.49 pcre2_match.c; code unit width 16.
// /*************************************************
// *      Perl-Compatible Regular Expressions       *
// *************************************************/
//
// /* PCRE is a library of functions to support regular expressions whose syntax
// and semantics are as close as possible to those of the Perl 5 language.
//
//                        Written by Philip Hazel
//      Original API code Copyright (c) 1997-2012 University of Cambridge
//           New API code Copyright (c) 2015-2024 University of Cambridge
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
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn pcre2_jit_match_16(
        _: *const pcre2_code_16,
        _: PCRE2_SPTR16,
        _: size_t,
        _: size_t,
        _: uint32_t,
        _: *mut pcre2_match_data_16,
        _: *mut pcre2_match_context_16,
    ) -> ::core::ffi::c_int;
    static _pcre2_OP_lengths_16: [uint8_t; 0];
    static mut _pcre2_default_match_context_16: pcre2_match_context_16;
    static _pcre2_ucd_boolprop_sets_16: [uint32_t; 0];
    static _pcre2_ucd_caseless_sets_16: [uint32_t; 0];
    static _pcre2_ucd_script_sets_16: [uint32_t; 0];
    static _pcre2_ucd_records_16: [ucd_record; 0];
    static _pcre2_ucd_stage1_16: [uint16_t; 0];
    static _pcre2_ucd_stage2_16: [uint16_t; 0];
    static _pcre2_ucp_gbtable_16: [uint32_t; 0];
    static _pcre2_ucp_gentype_16: [uint32_t; 0];
    fn _pcre2_extuni_16(
        _: uint32_t,
        _: PCRE2_SPTR16,
        _: PCRE2_SPTR16,
        _: PCRE2_SPTR16,
        _: BOOL,
        _: *mut ::core::ffi::c_int,
    ) -> PCRE2_SPTR16;
    fn _pcre2_is_newline_16(
        _: PCRE2_SPTR16,
        _: uint32_t,
        _: PCRE2_SPTR16,
        _: *mut uint32_t,
        _: BOOL,
    ) -> BOOL;
    fn _pcre2_jit_check_exec_16(_: *mut ::core::ffi::c_void, _: uint32_t) -> BOOL;
    fn _pcre2_ord2utf_16(_: uint32_t, _: *mut PCRE2_UCHAR16) -> ::core::ffi::c_uint;
    fn _pcre2_script_run_16(_: PCRE2_SPTR16, _: PCRE2_SPTR16, _: BOOL) -> BOOL;
    fn _pcre2_strcmp_16(_: PCRE2_SPTR16, _: PCRE2_SPTR16) -> ::core::ffi::c_int;
    fn _pcre2_strlen_16(_: PCRE2_SPTR16) -> size_t;
    fn _pcre2_valid_utf_16(_: PCRE2_SPTR16, _: size_t, _: *mut size_t) -> ::core::ffi::c_int;
    fn _pcre2_was_newline_16(
        _: PCRE2_SPTR16,
        _: uint32_t,
        _: PCRE2_SPTR16,
        _: *mut uint32_t,
        _: BOOL,
    ) -> BOOL;
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
pub type ptrdiff_t = isize;
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
pub struct match_block_16 {
    pub memctl: pcre2_memctl,
    pub heap_limit: uint32_t,
    pub match_limit: uint32_t,
    pub match_limit_depth: uint32_t,
    pub match_call_count: uint32_t,
    pub hitend: BOOL,
    pub hasthen: BOOL,
    pub hasbsk: BOOL,
    pub allowemptypartial: BOOL,
    pub allowlookaroundbsk: BOOL,
    pub lcc: *const uint8_t,
    pub fcc: *const uint8_t,
    pub ctypes: *const uint8_t,
    pub start_offset: size_t,
    pub end_offset_top: size_t,
    pub partial: uint16_t,
    pub bsr_convention: uint16_t,
    pub name_count: uint16_t,
    pub name_entry_size: uint16_t,
    pub name_table: PCRE2_SPTR16,
    pub start_code: PCRE2_SPTR16,
    pub start_subject: PCRE2_SPTR16,
    pub check_subject: PCRE2_SPTR16,
    pub end_subject: PCRE2_SPTR16,
    pub true_end_subject: PCRE2_SPTR16,
    pub end_match_ptr: PCRE2_SPTR16,
    pub start_used_ptr: PCRE2_SPTR16,
    pub last_used_ptr: PCRE2_SPTR16,
    pub mark: PCRE2_SPTR16,
    pub nomatch_mark: PCRE2_SPTR16,
    pub verb_ecode_ptr: PCRE2_SPTR16,
    pub verb_skip_ptr: PCRE2_SPTR16,
    pub verb_current_recurse: uint32_t,
    pub moptions: uint32_t,
    pub poptions: uint32_t,
    pub skip_arg_count: uint32_t,
    pub ignore_skip_arg: uint32_t,
    pub nltype: uint32_t,
    pub nllen: uint32_t,
    pub nl: [PCRE2_UCHAR16; 4],
    pub cb: *mut pcre2_callout_block_16,
    pub callout_data: *mut ::core::ffi::c_void,
    pub callout: Option<
        unsafe extern "C" fn(
            *mut pcre2_callout_block_16,
            *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
}
pub const PCRE2_MATCHEDBY_INTERPRETER: C2RustUnnamed_16 = 0;
pub const RM19: C2RustUnnamed_19 = 19;
pub const OP_THEN_ARG: C2RustUnnamed_17 = 162;
pub const RM18: C2RustUnnamed_19 = 18;
pub const OP_THEN: C2RustUnnamed_17 = 161;
pub const RM17: C2RustUnnamed_19 = 17;
pub const OP_SKIP_ARG: C2RustUnnamed_17 = 160;
pub const RM16: C2RustUnnamed_19 = 16;
pub const OP_SKIP: C2RustUnnamed_17 = 159;
pub const RM15: C2RustUnnamed_19 = 15;
pub const OP_PRUNE_ARG: C2RustUnnamed_17 = 158;
pub const RM14: C2RustUnnamed_19 = 14;
pub const OP_PRUNE: C2RustUnnamed_17 = 157;
pub const RM36: C2RustUnnamed_19 = 36;
pub const OP_COMMIT_ARG: C2RustUnnamed_17 = 164;
pub const RM13: C2RustUnnamed_19 = 13;
pub const OP_COMMIT: C2RustUnnamed_17 = 163;
pub const OP_FAIL: C2RustUnnamed_17 = 165;
pub const RM12: C2RustUnnamed_19 = 12;
pub const OP_MARK: C2RustUnnamed_17 = 156;
pub const OP_UCP_WORD_BOUNDARY: C2RustUnnamed_17 = 172;
pub const OP_WORD_BOUNDARY: C2RustUnnamed_17 = 5;
pub const ucp_Pc: C2RustUnnamed_15 = 16;
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
pub const ucp_Mn: C2RustUnnamed_15 = 12;
pub const ucp_N: C2RustUnnamed_14 = 3;
pub const ucp_L: C2RustUnnamed_14 = 1;
pub const OP_NOT_UCP_WORD_BOUNDARY: C2RustUnnamed_17 = 171;
pub const OP_NOT_WORD_BOUNDARY: C2RustUnnamed_17 = 4;
pub const OP_SET_SOM: C2RustUnnamed_17 = 3;
pub const OP_SOM: C2RustUnnamed_17 = 2;
pub const OP_DOLLM: C2RustUnnamed_17 = 26;
pub const OP_CIRCM: C2RustUnnamed_17 = 28;
pub const OP_EODN: C2RustUnnamed_17 = 23;
pub const OP_EOD: C2RustUnnamed_17 = 24;
pub const OP_DOLL: C2RustUnnamed_17 = 25;
pub const OP_SOD: C2RustUnnamed_17 = 1;
pub const OP_CIRC: C2RustUnnamed_17 = 27;
pub const RM7: C2RustUnnamed_19 = 7;
pub const RM6: C2RustUnnamed_19 = 6;
pub const OP_KETRMIN: C2RustUnnamed_17 = 124;
pub const OP_KET: C2RustUnnamed_17 = 122;
pub const OP_KETRPOS: C2RustUnnamed_17 = 125;
pub const OP_CREF: C2RustUnnamed_17 = 147;
pub const OP_SCBRAPOS: C2RustUnnamed_17 = 145;
pub const OP_SCBRA: C2RustUnnamed_17 = 144;
pub const OP_CBRAPOS: C2RustUnnamed_17 = 140;
pub const OP_CBRA: C2RustUnnamed_17 = 139;
pub const OP_SCRIPT_RUN: C2RustUnnamed_17 = 136;
pub const RM39: C2RustUnnamed_19 = 39;
pub const OP_ASSERT_SCS: C2RustUnnamed_17 = 134;
pub const OP_ASSERT_NOT: C2RustUnnamed_17 = 129;
pub const OP_VREVERSE: C2RustUnnamed_17 = 127;
pub const OP_ASSERTBACK_NOT: C2RustUnnamed_17 = 131;
pub const OP_ALT: C2RustUnnamed_17 = 121;
pub const OP_ONCE: C2RustUnnamed_17 = 135;
pub const OP_ASSERT: C2RustUnnamed_17 = 128;
pub const OP_ASSERTBACK: C2RustUnnamed_17 = 130;
pub const OP_ASSERT_NA: C2RustUnnamed_17 = 132;
pub const OP_ASSERTBACK_NA: C2RustUnnamed_17 = 133;
pub const OP_SCOND: C2RustUnnamed_17 = 146;
pub const OP_COND: C2RustUnnamed_17 = 141;
pub const OP_END: C2RustUnnamed_17 = 0;
pub const OP_BRA: C2RustUnnamed_17 = 137;
pub const OP_KETRMAX: C2RustUnnamed_17 = 123;
pub const RM37: C2RustUnnamed_19 = 37;
pub const OP_REVERSE: C2RustUnnamed_17 = 126;
pub const RM35: C2RustUnnamed_19 = 35;
pub const RM5: C2RustUnnamed_19 = 5;
pub const OP_TRUE: C2RustUnnamed_17 = 152;
pub const OP_FALSE: C2RustUnnamed_17 = 151;
pub const OP_DNCREF: C2RustUnnamed_17 = 148;
pub const OP_DNRREF: C2RustUnnamed_17 = 150;
pub const OP_RREF: C2RustUnnamed_17 = 149;
pub const OP_CALLOUT: C2RustUnnamed_17 = 119;
pub const OP_CALLOUT_STR: C2RustUnnamed_17 = 120;
pub const RM38: C2RustUnnamed_19 = 38;
pub const RM4: C2RustUnnamed_19 = 4;
pub const RM3: C2RustUnnamed_19 = 3;
pub const RM11: C2RustUnnamed_19 = 11;
pub const OP_RECURSE: C2RustUnnamed_17 = 118;
pub const RM2: C2RustUnnamed_19 = 2;
pub const OP_SBRA: C2RustUnnamed_17 = 142;
pub const RM1: C2RustUnnamed_19 = 1;
pub const RM8: C2RustUnnamed_19 = 8;
pub const OP_SBRAPOS: C2RustUnnamed_17 = 143;
pub const OP_BRAPOS: C2RustUnnamed_17 = 138;
pub const OP_BRAPOSZERO: C2RustUnnamed_17 = 155;
pub const OP_SKIPZERO: C2RustUnnamed_17 = 169;
pub const RM10: C2RustUnnamed_19 = 10;
pub const OP_BRAMINZERO: C2RustUnnamed_17 = 154;
pub const RM9: C2RustUnnamed_19 = 9;
pub const OP_BRAZERO: C2RustUnnamed_17 = 153;
pub const RM22: C2RustUnnamed_19 = 22;
pub const RM21: C2RustUnnamed_19 = 21;
pub const REPTYPE_POS: C2RustUnnamed_18 = 2;
pub const RM20: C2RustUnnamed_19 = 20;
pub const REPTYPE_MIN: C2RustUnnamed_18 = 0;
pub const OP_CRSTAR: C2RustUnnamed_17 = 98;
pub const REPTYPE_MAX: C2RustUnnamed_18 = 1;
pub const OP_CRPOSRANGE: C2RustUnnamed_17 = 109;
pub const OP_CRMINRANGE: C2RustUnnamed_17 = 105;
pub const OP_CRRANGE: C2RustUnnamed_17 = 104;
pub const OP_CRPOSQUERY: C2RustUnnamed_17 = 108;
pub const OP_CRPOSPLUS: C2RustUnnamed_17 = 107;
pub const OP_CRPOSSTAR: C2RustUnnamed_17 = 106;
pub const OP_CRMINQUERY: C2RustUnnamed_17 = 103;
pub const OP_CRQUERY: C2RustUnnamed_17 = 102;
pub const OP_CRMINPLUS: C2RustUnnamed_17 = 101;
pub const OP_CRPLUS: C2RustUnnamed_17 = 100;
pub const OP_CRMINSTAR: C2RustUnnamed_17 = 99;
pub const OP_REFI: C2RustUnnamed_17 = 115;
pub const OP_REF: C2RustUnnamed_17 = 114;
pub const OP_DNREFI: C2RustUnnamed_17 = 117;
pub const OP_DNREF: C2RustUnnamed_17 = 116;
pub const OP_ANYNL: C2RustUnnamed_17 = 17;
pub const RM34: C2RustUnnamed_19 = 34;
pub const OP_WORDCHAR: C2RustUnnamed_17 = 11;
pub const OP_NOT_WORDCHAR: C2RustUnnamed_17 = 10;
pub const OP_WHITESPACE: C2RustUnnamed_17 = 9;
pub const OP_NOT_WHITESPACE: C2RustUnnamed_17 = 8;
pub const OP_DIGIT: C2RustUnnamed_17 = 7;
pub const OP_NOT_DIGIT: C2RustUnnamed_17 = 6;
pub const OP_VSPACE: C2RustUnnamed_17 = 21;
pub const OP_NOT_VSPACE: C2RustUnnamed_17 = 20;
pub const OP_HSPACE: C2RustUnnamed_17 = 19;
pub const OP_NOT_HSPACE: C2RustUnnamed_17 = 18;
pub const OP_ANYBYTE: C2RustUnnamed_17 = 14;
pub const OP_ALLANY: C2RustUnnamed_17 = 13;
pub const OP_ANY: C2RustUnnamed_17 = 12;
pub const RM220: C2RustUnnamed_21 = 220;
pub const RM219: C2RustUnnamed_21 = 219;
pub const OP_EXTUNI: C2RustUnnamed_17 = 22;
pub const RM221: C2RustUnnamed_21 = 221;
pub const OP_NOTPROP: C2RustUnnamed_17 = 15;
pub const ucp_Z: C2RustUnnamed_14 = 6;
pub const ucp_Lt: C2RustUnnamed_15 = 8;
pub const ucp_Ll: C2RustUnnamed_15 = 5;
pub const ucp_Lu: C2RustUnnamed_15 = 9;
pub const RM33: C2RustUnnamed_19 = 33;
pub const RM218: C2RustUnnamed_21 = 218;
pub const RM217: C2RustUnnamed_21 = 217;
pub const RM222: C2RustUnnamed_21 = 222;
pub const RM223: C2RustUnnamed_21 = 223;
pub const RM216: C2RustUnnamed_21 = 216;
pub const RM215: C2RustUnnamed_21 = 215;
pub const RM214: C2RustUnnamed_21 = 214;
pub const RM213: C2RustUnnamed_21 = 213;
pub const RM212: C2RustUnnamed_21 = 212;
pub const RM224: C2RustUnnamed_21 = 224;
pub const RM211: C2RustUnnamed_21 = 211;
pub const RM210: C2RustUnnamed_21 = 210;
pub const RM209: C2RustUnnamed_21 = 209;
pub const RM208: C2RustUnnamed_21 = 208;
pub const OP_PROP: C2RustUnnamed_17 = 16;
pub const OP_TYPESTAR: C2RustUnnamed_17 = 85;
pub const OP_TYPEMINQUERY: C2RustUnnamed_17 = 90;
pub const OP_TYPEQUERY: C2RustUnnamed_17 = 89;
pub const OP_TYPEMINPLUS: C2RustUnnamed_17 = 88;
pub const OP_TYPEPLUS: C2RustUnnamed_17 = 87;
pub const OP_TYPEMINSTAR: C2RustUnnamed_17 = 86;
pub const OP_TYPEPOSUPTO: C2RustUnnamed_17 = 97;
pub const OP_TYPEPOSQUERY: C2RustUnnamed_17 = 96;
pub const OP_TYPEPOSPLUS: C2RustUnnamed_17 = 95;
pub const OP_TYPEPOSSTAR: C2RustUnnamed_17 = 94;
pub const OP_TYPEMINUPTO: C2RustUnnamed_17 = 92;
pub const OP_TYPEUPTO: C2RustUnnamed_17 = 91;
pub const OP_TYPEEXACT: C2RustUnnamed_17 = 93;
pub const RM103: C2RustUnnamed_20 = 103;
pub const RM102: C2RustUnnamed_20 = 102;
pub const OP_ECLASS: C2RustUnnamed_17 = 113;
pub const RM101: C2RustUnnamed_20 = 101;
pub const RM100: C2RustUnnamed_20 = 100;
pub const OP_XCLASS: C2RustUnnamed_17 = 112;
pub const RM24: C2RustUnnamed_19 = 24;
pub const OP_CLASS: C2RustUnnamed_17 = 110;
pub const RM201: C2RustUnnamed_21 = 201;
pub const RM23: C2RustUnnamed_19 = 23;
pub const RM200: C2RustUnnamed_21 = 200;
pub const OP_NCLASS: C2RustUnnamed_17 = 111;
pub const RM32: C2RustUnnamed_19 = 32;
pub const RM207: C2RustUnnamed_21 = 207;
pub const RM31: C2RustUnnamed_19 = 31;
pub const RM206: C2RustUnnamed_21 = 206;
pub const RM30: C2RustUnnamed_19 = 30;
pub const RM205: C2RustUnnamed_21 = 205;
pub const RM29: C2RustUnnamed_19 = 29;
pub const RM204: C2RustUnnamed_21 = 204;
pub const OP_NOTSTARI: C2RustUnnamed_17 = 72;
pub const OP_NOTSTAR: C2RustUnnamed_17 = 59;
pub const OP_NOTMINQUERYI: C2RustUnnamed_17 = 77;
pub const OP_NOTMINQUERY: C2RustUnnamed_17 = 64;
pub const OP_NOTQUERYI: C2RustUnnamed_17 = 76;
pub const OP_NOTQUERY: C2RustUnnamed_17 = 63;
pub const OP_NOTMINPLUSI: C2RustUnnamed_17 = 75;
pub const OP_NOTMINPLUS: C2RustUnnamed_17 = 62;
pub const OP_NOTPLUSI: C2RustUnnamed_17 = 74;
pub const OP_NOTPLUS: C2RustUnnamed_17 = 61;
pub const OP_NOTMINSTARI: C2RustUnnamed_17 = 73;
pub const OP_NOTMINSTAR: C2RustUnnamed_17 = 60;
pub const OP_NOTPOSUPTOI: C2RustUnnamed_17 = 84;
pub const OP_NOTPOSUPTO: C2RustUnnamed_17 = 71;
pub const OP_NOTPOSQUERYI: C2RustUnnamed_17 = 83;
pub const OP_NOTPOSQUERY: C2RustUnnamed_17 = 70;
pub const OP_NOTPOSPLUSI: C2RustUnnamed_17 = 82;
pub const OP_NOTPOSPLUS: C2RustUnnamed_17 = 69;
pub const OP_NOTPOSSTARI: C2RustUnnamed_17 = 81;
pub const OP_NOTPOSSTAR: C2RustUnnamed_17 = 68;
pub const OP_NOTMINUPTOI: C2RustUnnamed_17 = 79;
pub const OP_NOTMINUPTO: C2RustUnnamed_17 = 66;
pub const OP_NOTUPTOI: C2RustUnnamed_17 = 78;
pub const OP_NOTUPTO: C2RustUnnamed_17 = 65;
pub const OP_NOTEXACTI: C2RustUnnamed_17 = 80;
pub const OP_NOTEXACT: C2RustUnnamed_17 = 67;
pub const RM28: C2RustUnnamed_19 = 28;
pub const RM27: C2RustUnnamed_19 = 27;
pub const RM26: C2RustUnnamed_19 = 26;
pub const RM25: C2RustUnnamed_19 = 25;
pub const OP_STARI: C2RustUnnamed_17 = 46;
pub const RM203: C2RustUnnamed_21 = 203;
pub const RM202: C2RustUnnamed_21 = 202;
pub const OP_STAR: C2RustUnnamed_17 = 33;
pub const OP_MINQUERYI: C2RustUnnamed_17 = 51;
pub const OP_MINQUERY: C2RustUnnamed_17 = 38;
pub const OP_QUERYI: C2RustUnnamed_17 = 50;
pub const OP_QUERY: C2RustUnnamed_17 = 37;
pub const OP_MINPLUSI: C2RustUnnamed_17 = 49;
pub const OP_MINPLUS: C2RustUnnamed_17 = 36;
pub const OP_PLUSI: C2RustUnnamed_17 = 48;
pub const OP_PLUS: C2RustUnnamed_17 = 35;
pub const OP_MINSTARI: C2RustUnnamed_17 = 47;
pub const OP_MINSTAR: C2RustUnnamed_17 = 34;
pub const OP_POSQUERYI: C2RustUnnamed_17 = 57;
pub const OP_POSQUERY: C2RustUnnamed_17 = 44;
pub const OP_POSPLUSI: C2RustUnnamed_17 = 56;
pub const OP_POSPLUS: C2RustUnnamed_17 = 43;
pub const OP_POSSTARI: C2RustUnnamed_17 = 55;
pub const OP_POSSTAR: C2RustUnnamed_17 = 42;
pub const OP_MINUPTOI: C2RustUnnamed_17 = 53;
pub const OP_MINUPTO: C2RustUnnamed_17 = 40;
pub const OP_UPTOI: C2RustUnnamed_17 = 52;
pub const OP_UPTO: C2RustUnnamed_17 = 39;
pub const OP_POSUPTOI: C2RustUnnamed_17 = 58;
pub const OP_POSUPTO: C2RustUnnamed_17 = 45;
pub const OP_EXACTI: C2RustUnnamed_17 = 54;
pub const OP_EXACT: C2RustUnnamed_17 = 41;
pub const OP_NOTI: C2RustUnnamed_17 = 32;
pub const OP_NOT: C2RustUnnamed_17 = 31;
pub const OP_CHARI: C2RustUnnamed_17 = 30;
pub const OP_CHAR: C2RustUnnamed_17 = 29;
pub const OP_ACCEPT: C2RustUnnamed_17 = 166;
pub const OP_ASSERT_ACCEPT: C2RustUnnamed_17 = 167;
pub const OP_CLOSE: C2RustUnnamed_17 = 168;
pub type C2RustUnnamed_14 = ::core::ffi::c_uint;
pub const ucp_S: C2RustUnnamed_14 = 5;
pub const ucp_P: C2RustUnnamed_14 = 4;
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
pub const ucp_No: C2RustUnnamed_15 = 15;
pub const ucp_Nl: C2RustUnnamed_15 = 14;
pub const ucp_Nd: C2RustUnnamed_15 = 13;
pub const ucp_Me: C2RustUnnamed_15 = 11;
pub const ucp_Mc: C2RustUnnamed_15 = 10;
pub const ucp_Lo: C2RustUnnamed_15 = 7;
pub const ucp_Lm: C2RustUnnamed_15 = 6;
pub const ucp_Cs: C2RustUnnamed_15 = 4;
pub const ucp_Co: C2RustUnnamed_15 = 3;
pub const ucp_Cn: C2RustUnnamed_15 = 2;
pub const ucp_Cf: C2RustUnnamed_15 = 1;
pub const ucp_Cc: C2RustUnnamed_15 = 0;
pub type C2RustUnnamed_16 = ::core::ffi::c_uint;
pub const PCRE2_MATCHEDBY_JIT: C2RustUnnamed_16 = 2;
pub const PCRE2_MATCHEDBY_DFA_INTERPRETER: C2RustUnnamed_16 = 1;
pub type C2RustUnnamed_17 = ::core::ffi::c_uint;
pub const OP_TABLE_LENGTH: C2RustUnnamed_17 = 173;
pub const OP_DEFINE: C2RustUnnamed_17 = 170;
pub type C2RustUnnamed_18 = ::core::ffi::c_uint;
pub type C2RustUnnamed_19 = ::core::ffi::c_uint;
pub type C2RustUnnamed_20 = ::core::ffi::c_uint;
pub type C2RustUnnamed_21 = ::core::ffi::c_uint;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const UINT32_MAX: ::core::ffi::c_uint = 4294967295 as ::core::ffi::c_uint;
pub const SIZE_MAX: usize = usize::MAX;
pub const PCRE2_ANCHORED: ::core::ffi::c_uint = 0x80000000 as ::core::ffi::c_uint;
pub const PCRE2_NO_UTF_CHECK: ::core::ffi::c_uint = 0x40000000 as ::core::ffi::c_uint;
pub const PCRE2_ENDANCHORED: ::core::ffi::c_uint = 0x20000000 as ::core::ffi::c_uint;
pub const PCRE2_DOLLAR_ENDONLY: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PCRE2_FIRSTLINE: ::core::ffi::c_uint = 0x100 as ::core::ffi::c_uint;
pub const PCRE2_MATCH_UNSET_BACKREF: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PCRE2_UCP: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PCRE2_UTF: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PCRE2_ALT_CIRCUMFLEX: ::core::ffi::c_uint = 0x200000 as ::core::ffi::c_uint;
pub const PCRE2_USE_OFFSET_LIMIT: ::core::ffi::c_uint = 0x800000 as ::core::ffi::c_uint;
pub const PCRE2_MATCH_INVALID_UTF: ::core::ffi::c_uint = 0x4000000 as ::core::ffi::c_uint;
pub const PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PCRE2_NOTBOL: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PCRE2_NOTEOL: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PCRE2_NOTEMPTY: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PCRE2_NOTEMPTY_ATSTART: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PCRE2_PARTIAL_SOFT: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PCRE2_PARTIAL_HARD: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PCRE2_NO_JIT: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PCRE2_COPY_MATCHED_SUBJECT: ::core::ffi::c_uint = 0x4000 as ::core::ffi::c_uint;
pub const PCRE2_DISABLE_RECURSELOOP_CHECK: ::core::ffi::c_uint = 0x40000 as ::core::ffi::c_uint;
pub const PCRE2_NEWLINE_CR: ::core::ffi::c_int = 1;
pub const PCRE2_NEWLINE_LF: ::core::ffi::c_int = 2;
pub const PCRE2_NEWLINE_CRLF: ::core::ffi::c_int = 3;
pub const PCRE2_NEWLINE_ANY: ::core::ffi::c_int = 4;
pub const PCRE2_NEWLINE_ANYCRLF: ::core::ffi::c_int = 5;
pub const PCRE2_NEWLINE_NUL: ::core::ffi::c_int = 6;
pub const PCRE2_BSR_ANYCRLF: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PCRE2_ERROR_NOMATCH: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const PCRE2_ERROR_PARTIAL: ::core::ffi::c_int = -(2 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF16_ERR3: ::core::ffi::c_int = -(26 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADMAGIC: ::core::ffi::c_int = -(31 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADMODE: ::core::ffi::c_int = -(32 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADOFFSET: ::core::ffi::c_int = -(33 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADOPTION: ::core::ffi::c_int = -(34 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADUTFOFFSET: ::core::ffi::c_int = -(36 as ::core::ffi::c_int);
pub const PCRE2_ERROR_INTERNAL: ::core::ffi::c_int = -(44 as ::core::ffi::c_int);
pub const PCRE2_ERROR_MATCHLIMIT: ::core::ffi::c_int = -(47 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NOMEMORY: ::core::ffi::c_int = -(48 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NULL: ::core::ffi::c_int = -(51 as ::core::ffi::c_int);
pub const PCRE2_ERROR_RECURSELOOP: ::core::ffi::c_int = -(52 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DEPTHLIMIT: ::core::ffi::c_int = -(53 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADOFFSETLIMIT: ::core::ffi::c_int = -(56 as ::core::ffi::c_int);
pub const PCRE2_ERROR_HEAPLIMIT: ::core::ffi::c_int = -(63 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BAD_BACKSLASH_K: ::core::ffi::c_int = -(75 as ::core::ffi::c_int);
pub const PCRE2_SIZE_MAX: usize = SIZE_MAX;
pub const PCRE2_ZERO_TERMINATED: size_t = !(0 as ::core::ffi::c_int as size_t);
pub const PCRE2_UNSET: size_t = !(0 as ::core::ffi::c_int as size_t);
pub const PCRE2_CALLOUT_STARTMATCH: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PCRE2_CALLOUT_BACKTRACK: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const START_FRAMES_SIZE: ::core::ffi::c_int = 20480 as ::core::ffi::c_int;
pub const NLTYPE_FIXED: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const NLTYPE_ANY: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const NLTYPE_ANYCRLF: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PCRE2_MODE8: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PCRE2_MODE16: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PCRE2_MODE32: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PCRE2_FIRSTSET: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PCRE2_FIRSTCASELESS: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PCRE2_FIRSTMAPSET: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PCRE2_LASTSET: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PCRE2_LASTCASELESS: ::core::ffi::c_uint = 0x100 as ::core::ffi::c_uint;
pub const PCRE2_STARTLINE: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PCRE2_HASCRORLF: ::core::ffi::c_uint = 0x800 as ::core::ffi::c_uint;
pub const PCRE2_HASTHEN: ::core::ffi::c_uint = 0x1000 as ::core::ffi::c_uint;
pub const PCRE2_MATCH_EMPTY: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PCRE2_NOTEMPTY_SET: ::core::ffi::c_uint = 0x10000 as ::core::ffi::c_uint;
pub const PCRE2_NE_ATST_SET: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PCRE2_HASBSK: ::core::ffi::c_uint = 0x1000000 as ::core::ffi::c_uint;
pub const PCRE2_MODE_MASK: ::core::ffi::c_uint = PCRE2_MODE8 | PCRE2_MODE16 | PCRE2_MODE32;
pub const PCRE2_MD_COPIED_SUBJECT: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const MAGIC_NUMBER: usize = 0x50435245 as usize;
pub const REQ_CU_MAX: ::core::ffi::c_int = 2000 as ::core::ffi::c_int;
pub const cbit_length: ::core::ffi::c_int = 320 as ::core::ffi::c_int;
pub const ctype_space: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const ctype_digit: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const ctype_word: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const lcc_offset: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const fcc_offset: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const cbits_offset: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
pub const ctypes_offset: ::core::ffi::c_int = cbits_offset + cbit_length;
pub const PCRE2_OPTIM_START_OPTIMIZE: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const CHAR_HT: ::core::ffi::c_int = 9;
pub const CHAR_VT: ::core::ffi::c_int = 11;
pub const CHAR_FF: ::core::ffi::c_int = 12;
pub const CHAR_CR: ::core::ffi::c_int = '\r' as i32;
pub const CHAR_LF: ::core::ffi::c_int = '\n' as i32;
pub const CHAR_NL: ::core::ffi::c_int = CHAR_LF;
pub const CHAR_NEL: ::core::ffi::c_uchar = -123i32 as ::core::ffi::c_uchar;
pub const CHAR_NUL: ::core::ffi::c_int = '\0' as i32;
pub const CHAR_SPACE: ::core::ffi::c_int = 32;
pub const CHAR_DOLLAR_SIGN: ::core::ffi::c_int = '$' as i32;
pub const CHAR_COMMERCIAL_AT: ::core::ffi::c_int = '@' as i32;
pub const CHAR_GRAVE_ACCENT: ::core::ffi::c_int = '`' as i32;
pub const CHAR_NBSP: ::core::ffi::c_int = 160 as ::core::ffi::c_int;
pub const PT_LAMP: ::core::ffi::c_int = 0;
pub const PT_GC: ::core::ffi::c_int = 1;
pub const PT_PC: ::core::ffi::c_int = 2;
pub const PT_SC: ::core::ffi::c_int = 3;
pub const PT_SCX: ::core::ffi::c_int = 4;
pub const PT_ALNUM: ::core::ffi::c_int = 5;
pub const PT_SPACE: ::core::ffi::c_int = 6;
pub const PT_PXSPACE: ::core::ffi::c_int = 7;
pub const PT_WORD: ::core::ffi::c_int = 8;
pub const PT_CLIST: ::core::ffi::c_int = 9;
pub const PT_UCNC: ::core::ffi::c_int = 10;
pub const PT_BIDICL: ::core::ffi::c_int = 11;
pub const PT_BOOL: ::core::ffi::c_int = 12;
pub const RREF_ANY: ::core::ffi::c_int = 0xffff as ::core::ffi::c_int;
pub const REFI_FLAG_CASELESS_RESTRICT: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const REFI_FLAG_TURKISH_CASING: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const UCD_BIDICLASS_SHIFT: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const LINK_SIZE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const IMM2_SIZE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const HEAPFRAME_ALIGNMENT: usize = 8 as usize;
pub const RECURSE_UNSET: ::core::ffi::c_uint = 0xffffffff as ::core::ffi::c_uint;
pub const PUBLIC_MATCH_OPTIONS: ::core::ffi::c_uint = PCRE2_ANCHORED
    | PCRE2_ENDANCHORED
    | PCRE2_NOTBOL
    | PCRE2_NOTEOL
    | PCRE2_NOTEMPTY
    | PCRE2_NOTEMPTY_ATSTART
    | PCRE2_NO_UTF_CHECK
    | PCRE2_PARTIAL_HARD
    | PCRE2_PARTIAL_SOFT
    | PCRE2_NO_JIT
    | PCRE2_COPY_MATCHED_SUBJECT
    | PCRE2_DISABLE_RECURSELOOP_CHECK;
pub const PUBLIC_JIT_MATCH_OPTIONS: ::core::ffi::c_uint = PCRE2_NO_UTF_CHECK
    | PCRE2_NOTBOL
    | PCRE2_NOTEOL
    | PCRE2_NOTEMPTY
    | PCRE2_NOTEMPTY_ATSTART
    | PCRE2_PARTIAL_SOFT
    | PCRE2_PARTIAL_HARD
    | PCRE2_COPY_MATCHED_SUBJECT;
pub const MATCH_MATCH: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const MATCH_NOMATCH: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const MATCH_ACCEPT: ::core::ffi::c_int = -999;
pub const MATCH_KETRPOS: ::core::ffi::c_int = -(998 as ::core::ffi::c_int);
pub const MATCH_COMMIT: ::core::ffi::c_int = -997;
pub const MATCH_PRUNE: ::core::ffi::c_int = -996;
pub const MATCH_SKIP: ::core::ffi::c_int = -995;
pub const MATCH_SKIP_ARG: ::core::ffi::c_int = -994;
pub const MATCH_THEN: ::core::ffi::c_int = -993;
pub const MATCH_BACKTRACK_MAX: ::core::ffi::c_int = MATCH_THEN;
pub const MATCH_BACKTRACK_MIN: ::core::ffi::c_int = MATCH_COMMIT;
pub const GF_CAPTURE: ::core::ffi::c_uint = 0x10000 as ::core::ffi::c_uint;
pub const GF_NOCAPTURE: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const GF_CONDASSERT: ::core::ffi::c_uint = 0x30000 as ::core::ffi::c_uint;
pub const GF_RECURSE: ::core::ffi::c_uint = 0x40000 as ::core::ffi::c_uint;
static mut rep_min: [uint32_t; 11] = [
    0 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
    1 as ::core::ffi::c_int as uint32_t,
    1 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
    1 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
];
static mut rep_max: [uint32_t; 11] = [
    UINT32_MAX,
    UINT32_MAX,
    UINT32_MAX,
    UINT32_MAX,
    1 as ::core::ffi::c_int as uint32_t,
    1 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
    UINT32_MAX,
    UINT32_MAX,
    1 as ::core::ffi::c_int as uint32_t,
];
static mut rep_typ: [uint32_t; 12] = [
    REPTYPE_MAX as ::core::ffi::c_int as uint32_t,
    REPTYPE_MIN as ::core::ffi::c_int as uint32_t,
    REPTYPE_MAX as ::core::ffi::c_int as uint32_t,
    REPTYPE_MIN as ::core::ffi::c_int as uint32_t,
    REPTYPE_MAX as ::core::ffi::c_int as uint32_t,
    REPTYPE_MIN as ::core::ffi::c_int as uint32_t,
    REPTYPE_MAX as ::core::ffi::c_int as uint32_t,
    REPTYPE_MIN as ::core::ffi::c_int as uint32_t,
    REPTYPE_POS as ::core::ffi::c_int as uint32_t,
    REPTYPE_POS as ::core::ffi::c_int as uint32_t,
    REPTYPE_POS as ::core::ffi::c_int as uint32_t,
    REPTYPE_POS as ::core::ffi::c_int as uint32_t,
];
unsafe extern "C" fn do_callout(
    mut F: *mut heapframe,
    mut mb: *mut match_block_16,
    mut lengthptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut rc: ::core::ffi::c_int = 0;
    let mut save0: size_t = 0;
    let mut save1: size_t = 0;
    let mut callout_ovector: *mut size_t = ::core::ptr::null_mut::<size_t>();
    let mut cb: *mut pcre2_callout_block_16 = ::core::ptr::null_mut::<pcre2_callout_block_16>();
    *lengthptr = (if *(*F).ecode as ::core::ffi::c_int == OP_CALLOUT as ::core::ffi::c_int {
        *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
            .offset(OP_CALLOUT as ::core::ffi::c_int as isize) as ::core::ffi::c_int
    } else {
        *(*F).ecode.offset(
            (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * 1 as ::core::ffi::c_int) as isize,
        ) as ::core::ffi::c_int
    }) as size_t;
    if (*mb).callout.is_none() {
        return 0 as ::core::ffi::c_int;
    }
    callout_ovector =
        (&raw mut (*F).ovector as *mut size_t).offset(-(2 as ::core::ffi::c_int as isize));
    cb = (*mb).cb;
    (*cb).capture_top = ((*F).offset_top as uint32_t)
        .wrapping_div(2 as uint32_t)
        .wrapping_add(1 as uint32_t);
    (*cb).capture_last = (*F).capture_last;
    (*cb).offset_vector = callout_ovector;
    (*cb).mark = (*mb).nomatch_mark;
    (*cb).current_position =
        (*F).eptr.offset_from((*mb).start_subject) as isize as size_t;
    (*cb).pattern_position = *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as size_t;
    (*cb).next_item_length = *(*F)
        .ecode
        .offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
        as size_t;
    if *(*F).ecode as ::core::ffi::c_int == OP_CALLOUT as ::core::ffi::c_int {
        (*cb).callout_number = *(*F)
            .ecode
            .offset((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * LINK_SIZE) as isize)
            as uint32_t;
        (*cb).callout_string_offset = 0 as size_t;
        (*cb).callout_string = ::core::ptr::null::<PCRE2_UCHAR16>();
        (*cb).callout_string_length = 0 as size_t;
    } else {
        (*cb).callout_number = 0 as uint32_t;
        (*cb).callout_string_offset = *(*F).ecode.offset(
            (1 as ::core::ffi::c_int + 3 as ::core::ffi::c_int * 1 as ::core::ffi::c_int) as isize,
        ) as size_t;
        (*cb).callout_string = (*F)
            .ecode
            .offset((1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * LINK_SIZE) as isize)
            .offset(1 as ::core::ffi::c_int as isize);
        (*cb).callout_string_length = (*lengthptr)
            .wrapping_sub((1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * LINK_SIZE) as size_t)
            .wrapping_sub(2 as size_t);
    }
    save0 = *callout_ovector.offset(0 as ::core::ffi::c_int as isize);
    save1 = *callout_ovector.offset(1 as ::core::ffi::c_int as isize);
    let ref mut fresh192 = *callout_ovector.offset(1 as ::core::ffi::c_int as isize);
    *fresh192 = PCRE2_UNSET;
    *callout_ovector.offset(0 as ::core::ffi::c_int as isize) = *fresh192;
    rc = (*mb).callout.expect("non-null function pointer")(cb, (*mb).callout_data);
    *callout_ovector.offset(0 as ::core::ffi::c_int as isize) = save0;
    *callout_ovector.offset(1 as ::core::ffi::c_int as isize) = save1;
    (*cb).callout_flags = 0 as uint32_t;
    return rc;
}
unsafe extern "C" fn match_ref(
    mut offset: size_t,
    mut caseless: BOOL,
    mut caseopts: ::core::ffi::c_int,
    mut F: *mut heapframe,
    mut mb: *mut match_block_16,
    mut lengthptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut p: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut length: size_t = 0;
    let mut eptr: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut eptr_start: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    if offset >= (*F).offset_top || (*F).ovector[offset as usize] == PCRE2_UNSET {
        if (*mb).poptions & PCRE2_MATCH_UNSET_BACKREF as uint32_t != 0 as uint32_t {
            *lengthptr = 0 as size_t;
            return 0 as ::core::ffi::c_int;
        } else {
            return -(1 as ::core::ffi::c_int);
        }
    }
    eptr_start = (*F).eptr;
    eptr = eptr_start;
    p = (*mb)
        .start_subject
        .offset((*F).ovector[offset as usize] as isize);
    length = (*F).ovector[offset.wrapping_add(1 as size_t) as usize]
        .wrapping_sub((*F).ovector[offset as usize]);
    if caseless != 0 {
        let mut utf: BOOL =
            ((*mb).poptions & PCRE2_UTF as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
        let mut caseless_restrict: BOOL = (caseopts & REFI_FLAG_CASELESS_RESTRICT
            != 0 as ::core::ffi::c_int)
            as ::core::ffi::c_int;
        let mut turkish_casing: BOOL = (caseless_restrict == 0
            && caseopts & REFI_FLAG_TURKISH_CASING != 0 as ::core::ffi::c_int)
            as ::core::ffi::c_int;
        if utf != 0 || (*mb).poptions & PCRE2_UCP as uint32_t != 0 as uint32_t {
            let mut endptr: PCRE2_SPTR16 = p.offset(length as isize);
            while p < endptr {
                let mut c: uint32_t = 0;
                let mut d: uint32_t = 0;
                let mut ur: *const ucd_record = ::core::ptr::null::<ucd_record>();
                if eptr >= (*mb).end_subject {
                    return 1 as ::core::ffi::c_int;
                }
                if utf != 0 {
                    let fresh193 = eptr;
                    eptr = eptr.offset(1);
                    c = *fresh193 as uint32_t;
                    if c & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                        let fresh194 = eptr;
                        eptr = eptr.offset(1);
                        c = ((c & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                            | *fresh194 as uint32_t & 0x3ff as uint32_t)
                            .wrapping_add(0x10000 as uint32_t);
                    }
                    let fresh195 = p;
                    p = p.offset(1);
                    d = *fresh195 as uint32_t;
                    if d & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                        let fresh196 = p;
                        p = p.offset(1);
                        d = ((d & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                            | *fresh196 as uint32_t & 0x3ff as uint32_t)
                            .wrapping_add(0x10000 as uint32_t);
                    }
                } else {
                    let fresh197 = eptr;
                    eptr = eptr.offset(1);
                    c = *fresh197 as uint32_t;
                    let fresh198 = p;
                    p = p.offset(1);
                    d = *fresh198 as uint32_t;
                }
                if turkish_casing != 0
                    && (d | 0x20 as uint32_t == 0x69 as uint32_t
                        || d | 1 as uint32_t == 0x131 as uint32_t)
                {
                    c = if c == 0x130 as uint32_t {
                        0x69 as uint32_t
                    } else if c == 0x49 as uint32_t {
                        0x131 as uint32_t
                    } else {
                        c
                    };
                    d = if d == 0x130 as uint32_t {
                        0x69 as uint32_t
                    } else if d == 0x49 as uint32_t {
                        0x131 as uint32_t
                    } else {
                        d
                    };
                    if c != d {
                        return -(1 as ::core::ffi::c_int);
                    }
                } else if c != d && {
                    ur = (&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
                        *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                            (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                .offset((d as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                                as ::core::ffi::c_int
                                * UCD_BLOCK_SIZE
                                + d as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                as isize,
                        ) as ::core::ffi::c_int as isize,
                    );
                    c != (d as int32_t + (*ur).other_case) as uint32_t
                } {
                    let mut pp: *const uint32_t = (&raw const _pcre2_ucd_caseless_sets_16
                        as *const uint32_t)
                        .offset((*ur).caseset as ::core::ffi::c_int as isize);
                    if caseless_restrict != 0 && *pp < 128 as uint32_t {
                        return -(1 as ::core::ffi::c_int);
                    }
                    loop {
                        if c < *pp {
                            return -(1 as ::core::ffi::c_int);
                        }
                        let fresh199 = pp;
                        pp = pp.offset(1);
                        if c == *fresh199 {
                            break;
                        }
                    }
                }
            }
        } else {
            while length > 0 as size_t {
                let mut cc: uint32_t = 0;
                let mut cp: uint32_t = 0;
                if eptr >= (*mb).end_subject {
                    return 1 as ::core::ffi::c_int;
                }
                cc = *eptr as uint32_t;
                cp = *p as uint32_t;
                if (if cp <= 255 as uint32_t {
                    *(*mb).lcc.offset(cp as isize) as uint32_t
                } else {
                    cp
                }) != (if cc <= 255 as uint32_t {
                    *(*mb).lcc.offset(cc as isize) as uint32_t
                } else {
                    cc
                }) {
                    return -(1 as ::core::ffi::c_int);
                }
                p = p.offset(1);
                eptr = eptr.offset(1);
                length = length.wrapping_sub(1);
            }
        }
    } else if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        while length > 0 as size_t {
            if eptr >= (*mb).end_subject {
                return 1 as ::core::ffi::c_int;
            }
            let fresh200 = p;
            p = p.offset(1);
            let fresh201 = eptr;
            eptr = eptr.offset(1);
            if *fresh200 as ::core::ffi::c_int != *fresh201 as ::core::ffi::c_int {
                return -(1 as ::core::ffi::c_int);
            }
            length = length.wrapping_sub(1);
        }
    } else {
        if ((*mb).end_subject.offset_from(eptr) as isize as size_t) < length
            || memcmp(
                p as *const ::core::ffi::c_void,
                eptr as *const ::core::ffi::c_void,
                length.wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
            ) != 0 as ::core::ffi::c_int
        {
            return -(1 as ::core::ffi::c_int);
        }
        eptr = eptr.offset(length as isize);
    }
    *lengthptr = eptr.offset_from(eptr_start) as isize as size_t;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn recurse_update_offsets(mut F: *mut heapframe, mut P: *mut heapframe) {
    let mut dst: *mut size_t = &raw mut (*F).ovector as *mut size_t;
    let mut src: *mut size_t = &raw mut (*P).ovector as *mut size_t;
    let mut offset: size_t = 2 as size_t;
    let mut offset_top: size_t = (*F).offset_top.wrapping_add(2 as size_t);
    let mut diff: size_t = 0;
    let mut ecode: PCRE2_SPTR16 = (*F).ecode;
    loop {
        diff = (((*ecode.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            << 1 as ::core::ffi::c_int) as size_t)
            .wrapping_sub(offset);
        ecode = ecode.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
        if offset.wrapping_add(diff) >= offset_top {
            while *ecode as ::core::ffi::c_int == OP_CREF as ::core::ffi::c_int {
                ecode = ecode.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
            }
            break;
        } else {
            if diff == 2 as size_t {
                *dst.offset(0 as ::core::ffi::c_int as isize) =
                    *src.offset(0 as ::core::ffi::c_int as isize);
                *dst.offset(1 as ::core::ffi::c_int as isize) =
                    *src.offset(1 as ::core::ffi::c_int as isize);
            } else if diff >= 4 as size_t {
                memcpy(
                    dst as *mut ::core::ffi::c_void,
                    src as *const ::core::ffi::c_void,
                    diff.wrapping_mul(::core::mem::size_of::<size_t>() as size_t),
                );
            }
            diff = diff.wrapping_add(2 as size_t);
            offset = offset.wrapping_add(diff);
            dst = dst.offset(diff as isize);
            src = src.offset(diff as isize);
            if !(*ecode as ::core::ffi::c_int == OP_CREF as ::core::ffi::c_int) {
                break;
            }
        }
    }
    diff = offset_top.wrapping_sub(offset);
    if diff == 2 as size_t {
        *dst.offset(0 as ::core::ffi::c_int as isize) =
            *src.offset(0 as ::core::ffi::c_int as isize);
        *dst.offset(1 as ::core::ffi::c_int as isize) =
            *src.offset(1 as ::core::ffi::c_int as isize);
    } else if diff >= 4 as size_t {
        memcpy(
            dst as *mut ::core::ffi::c_void,
            src as *const ::core::ffi::c_void,
            diff.wrapping_mul(::core::mem::size_of::<size_t>() as size_t),
        );
    }
    (*F).ecode = ecode;
    (*F).offset_top = if offset <= (*P).offset_top {
        (*P).offset_top
    } else {
        offset.wrapping_sub(2 as size_t)
    };
}
unsafe extern "C" fn match_0(
    mut start_eptr: PCRE2_SPTR16,
    mut start_ecode: PCRE2_SPTR16,
    mut top_bracket: uint16_t,
    mut frame_size: size_t,
    mut match_data: *mut pcre2_match_data_16,
    mut mb: *mut match_block_16,
) -> ::core::ffi::c_int {
    let mut next_ecode: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut samelengths: BOOL = 0;
    let mut next_ecode_4: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut current_branch: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut next_branch: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut next_ecode_0: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut slength_0: size_t = 0;
    let mut lgb: ::core::ffi::c_int = 0;
    let mut rgb: ::core::ffi::c_int = 0;
    let mut fptr: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut notmatch_1: BOOL = 0;
    let mut ok_4: BOOL = 0;
    let mut prop_3: *const ucd_record = ::core::ptr::null::<ucd_record>();
    let mut cp_1: *const uint32_t = ::core::ptr::null::<uint32_t>();
    let mut chartype_3: ::core::ffi::c_int = 0;
    let mut category_2: ::core::ffi::c_int = 0;
    let mut category_1: ::core::ffi::c_int = 0;
    let mut ok_3: BOOL = 0;
    let mut prop_2: *const ucd_record = ::core::ptr::null::<ucd_record>();
    let mut chartype_2: ::core::ffi::c_int = 0;
    let mut d_4: uint32_t = 0;
    let mut d_3: uint32_t = 0;
    let mut d_1: uint32_t = 0;
    let mut d_0: uint32_t = 0;
    let mut cc_2: uint32_t = 0;
    let mut othercase: uint32_t = 0;
    let mut ecode: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut count_0: ::core::ffi::c_int = 0;
    let mut slot_0: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut current_block: u64;
    let mut F: *mut heapframe = ::core::ptr::null_mut::<heapframe>();
    let mut N: *mut heapframe = ::core::ptr::null_mut::<heapframe>();
    let mut P: *mut heapframe = ::core::ptr::null_mut::<heapframe>();
    let mut frames_top: *mut heapframe = ::core::ptr::null_mut::<heapframe>();
    let mut assert_accept_frame: *mut heapframe = ::core::ptr::null_mut::<heapframe>();
    let mut frame_copy_size: size_t = 0;
    let mut branch_end: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut branch_start: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut bracode: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut offset: size_t = 0;
    let mut length: size_t = 0;
    let mut rrc: ::core::ffi::c_int = 0;
    let mut proptype: ::core::ffi::c_int = 0;
    let mut i: uint32_t = 0;
    let mut fc: uint32_t = 0;
    let mut number: uint32_t = 0;
    let mut reptype: uint32_t = 0 as uint32_t;
    let mut group_frame_type: uint32_t = 0;
    let mut condition: BOOL = 0;
    let mut cur_is_word: BOOL = 0;
    let mut prev_is_word: BOOL = 0;
    let mut utf: BOOL =
        ((*mb).poptions & PCRE2_UTF as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    let mut ucp: BOOL =
        ((*mb).poptions & PCRE2_UCP as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    frame_copy_size = frame_size.wrapping_sub(64 as size_t);
    F = (*match_data).heapframes as *mut heapframe;
    frames_top = (F as *mut ::core::ffi::c_char).offset((*match_data).heapframes_size as isize)
        as *mut heapframe;
    (*F).rdepth = 0 as uint32_t;
    (*F).capture_last = 0 as uint32_t;
    (*F).current_recurse = RECURSE_UNSET as uint32_t;
    (*F).eptr = start_eptr;
    (*F).start_match = (*F).eptr;
    (*F).mark = ::core::ptr::null::<PCRE2_UCHAR16>();
    (*F).offset_top = 0 as size_t;
    (*F).last_group_offset = PCRE2_UNSET;
    group_frame_type = 0 as uint32_t;
    loop {
        (*F).group_frame_type = group_frame_type;
        (*F).ecode = start_ecode;
        (*F).back_frame = frame_size;
        if group_frame_type != 0 as uint32_t {
            (*F).last_group_offset = (F as *mut ::core::ffi::c_char)
                .offset_from((*match_data).heapframes as *mut ::core::ffi::c_char)
                as isize as size_t;
            if group_frame_type & 0xffff0000 as uint32_t == GF_RECURSE as uint32_t {
                (*F).current_recurse = group_frame_type & 0xffff as uint32_t;
            }
            group_frame_type = 0 as uint32_t;
        }
        let fresh2 = (*mb).match_call_count;
        (*mb).match_call_count = (*mb).match_call_count.wrapping_add(1);
        if fresh2 >= (*mb).match_limit {
            return PCRE2_ERROR_MATCHLIMIT;
        }
        if (*F).rdepth >= (*mb).match_limit_depth {
            return PCRE2_ERROR_DEPTHLIMIT;
        }
        's_263: loop {
            (*F).op = *(*F).ecode as uint8_t;
            match (*F).op as ::core::ffi::c_int {
                168 => {
                    if (*F).current_recurse == RECURSE_UNSET as uint32_t {
                        number = *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                        offset = (*F).last_group_offset;
                        loop {
                            if offset == PCRE2_UNSET {
                                return PCRE2_ERROR_INTERNAL;
                            }
                            N = ((*match_data).heapframes as *mut ::core::ffi::c_char)
                                .offset(offset as isize)
                                as *mut heapframe;
                            P = (N as *mut ::core::ffi::c_char).offset(-(frame_size as isize))
                                as *mut heapframe;
                            if (*N).group_frame_type == GF_CAPTURE as uint32_t | number {
                                break;
                            }
                            offset = (*P).last_group_offset;
                        }
                        offset = (number << 1 as ::core::ffi::c_int).wrapping_sub(2 as uint32_t)
                            as size_t;
                        (*F).capture_last = number;
                        (*F).ovector[offset as usize] = (*P).eptr.offset_from((*mb).start_subject)
                            as isize
                            as size_t;
                        (*F).ovector[offset.wrapping_add(1 as size_t) as usize] =
                            (*F).eptr.offset_from((*mb).start_subject) as isize
                                as size_t;
                        if offset >= (*F).offset_top {
                            (*F).offset_top = offset.wrapping_add(2 as size_t);
                        }
                    }
                    (*F).ecode = (*F).ecode.offset(
                        *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                            .offset(*(*F).ecode as isize)
                            as ::core::ffi::c_int as isize,
                    );
                    continue;
                }
                167 => {
                    if (*F).eptr > (*mb).last_used_ptr {
                        (*mb).last_used_ptr = (*F).eptr;
                    }
                    assert_accept_frame = F;
                    rrc = -(999 as ::core::ffi::c_int);
                    current_block = 9215777819543815842;
                }
                166 => {
                    if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                        offset = (*F).last_group_offset;
                        loop {
                            if offset == PCRE2_UNSET {
                                return PCRE2_ERROR_INTERNAL;
                            }
                            N = ((*match_data).heapframes as *mut ::core::ffi::c_char)
                                .offset(offset as isize)
                                as *mut heapframe;
                            P = (N as *mut ::core::ffi::c_char).offset(-(frame_size as isize))
                                as *mut heapframe;
                            if (*N).group_frame_type & 0xffff0000 as uint32_t
                                == GF_RECURSE as uint32_t
                            {
                                break;
                            }
                            offset = (*P).last_group_offset;
                        }
                        (*P).eptr = (*F).eptr;
                        (*P).mark = (*F).mark;
                        (*P).start_match = (*F).start_match;
                        F = P;
                        (*F).ecode = (*F)
                            .ecode
                            .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                        continue;
                    } else {
                        current_block = 5359207940272634753;
                    }
                }
                0 => {
                    current_block = 5359207940272634753;
                }
                12 => {
                    if if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                        ((*F).eptr < (*mb).end_subject
                            && _pcre2_is_newline_16(
                                (*F).eptr,
                                (*mb).nltype,
                                (*mb).end_subject,
                                &raw mut (*mb).nllen,
                                utf,
                            ) != 0) as ::core::ffi::c_int
                    } else {
                        ((*F).eptr <= (*mb).end_subject.offset(-((*mb).nllen as isize))
                            && *(*F).eptr as ::core::ffi::c_int
                                == (*mb).nl[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                            && ((*mb).nllen == 1 as uint32_t
                                || *(*F).eptr.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int
                                    == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                        as ::core::ffi::c_int))
                            as ::core::ffi::c_int
                    } != 0
                    {
                        rrc = 0 as ::core::ffi::c_int;
                        current_block = 9215777819543815842;
                    } else {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && (*F).eptr
                                == (*mb)
                                    .end_subject
                                    .offset(-(1 as ::core::ffi::c_int as isize))
                            && (*mb).nltype == NLTYPE_FIXED as uint32_t
                            && (*mb).nllen == 2 as uint32_t
                            && *(*F).eptr as ::core::ffi::c_int
                                == (*mb).nl[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        current_block = 17964307082103460721;
                    }
                }
                13 => {
                    current_block = 17964307082103460721;
                }
                14 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        (*F).eptr = (*F).eptr.offset(1);
                        (*F).ecode = (*F).ecode.offset(1);
                        continue;
                    }
                    current_block = 9215777819543815842;
                }
                29 => {
                    if utf != 0 {
                        length = 1 as size_t;
                        (*F).ecode = (*F).ecode.offset(1);
                        fc = *(*F).ecode as uint32_t;
                        if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                            length = length.wrapping_add(1);
                        }
                        if length
                            > (*mb).end_subject.offset_from((*F).eptr) as isize
                                as size_t
                        {
                            if (*F).eptr >= (*mb).end_subject {
                                if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                                    && ((*F).eptr > (*mb).start_used_ptr
                                        || (*mb).allowemptypartial != 0)
                                {
                                    (*mb).hitend = TRUE as BOOL;
                                    if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int
                                    {
                                        return PCRE2_ERROR_PARTIAL;
                                    }
                                }
                            }
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            loop {
                                if !(length > 0 as size_t) {
                                    continue 's_263;
                                }
                                let fresh3 = (*F).ecode;
                                (*F).ecode = (*F).ecode.offset(1);
                                let fresh4 = (*F).eptr;
                                (*F).eptr = (*F).eptr.offset(1);
                                if *fresh3 as ::core::ffi::c_int != *fresh4 as ::core::ffi::c_int {
                                    rrc = 0 as ::core::ffi::c_int;
                                    break;
                                } else {
                                    length = length.wrapping_sub(1);
                                }
                            }
                        }
                    } else if ((*mb).end_subject.offset_from((*F).eptr) as isize)
                        < 1 as isize
                    {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh5 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        if *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int
                            != *fresh5 as ::core::ffi::c_int
                        {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            (*F).ecode = (*F).ecode.offset(2 as ::core::ffi::c_int as isize);
                            continue;
                        }
                    }
                    current_block = 9215777819543815842;
                }
                30 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else if utf != 0 {
                        length = 1 as size_t;
                        (*F).ecode = (*F).ecode.offset(1);
                        fc = *(*F).ecode as uint32_t;
                        if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                            length = length.wrapping_add(1);
                        }
                        if fc < 128 as uint32_t {
                            let mut cc: uint32_t = *(*F).eptr as uint32_t;
                            if *(*mb).lcc.offset(fc as isize) as uint32_t
                                != (if cc <= 255 as uint32_t {
                                    *(*mb).lcc.offset(cc as isize) as uint32_t
                                } else {
                                    cc
                                })
                            {
                                rrc = 0 as ::core::ffi::c_int;
                            } else {
                                (*F).ecode = (*F).ecode.offset(1);
                                (*F).eptr = (*F).eptr.offset(1);
                                continue;
                            }
                        } else {
                            let mut dc: uint32_t = 0;
                            let fresh6 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            dc = *fresh6 as uint32_t;
                            if dc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                let fresh7 = (*F).eptr;
                                (*F).eptr = (*F).eptr.offset(1);
                                dc = ((dc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                    | *fresh7 as uint32_t & 0x3ff as uint32_t)
                                    .wrapping_add(0x10000 as uint32_t);
                            }
                            (*F).ecode = (*F).ecode.offset(length as isize);
                            if !(dc != fc
                                && dc
                                    != (fc as ::core::ffi::c_int
                                        + (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                            .offset(
                                                *(&raw const _pcre2_ucd_stage2_16
                                                    as *const uint16_t)
                                                    .offset(
                                                        (*(&raw const _pcre2_ucd_stage1_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                (fc as ::core::ffi::c_int
                                                                    / UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            * UCD_BLOCK_SIZE
                                                            + fc as ::core::ffi::c_int
                                                                % UCD_BLOCK_SIZE)
                                                            as isize,
                                                    )
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            ))
                                        .other_case
                                            as ::core::ffi::c_int)
                                        as uint32_t)
                            {
                                continue;
                            }
                            rrc = 0 as ::core::ffi::c_int;
                        }
                    } else if ucp != 0 {
                        let mut cc_0: uint32_t = *(*F).eptr as uint32_t;
                        fc = *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                        if fc < 128 as uint32_t {
                            if *(*mb).lcc.offset(fc as isize) as uint32_t
                                != (if cc_0 <= 255 as uint32_t {
                                    *(*mb).lcc.offset(cc_0 as isize) as uint32_t
                                } else {
                                    cc_0
                                })
                            {
                                rrc = 0 as ::core::ffi::c_int;
                                current_block = 9215777819543815842;
                            } else {
                                current_block = 16517180880614114163;
                            }
                        } else if cc_0 != fc
                            && cc_0
                                != (fc as ::core::ffi::c_int
                                    + (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                        .offset(
                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                .offset(
                                                    (*(&raw const _pcre2_ucd_stage1_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (fc as ::core::ffi::c_int
                                                                / UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        * UCD_BLOCK_SIZE
                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                                        as isize,
                                                )
                                                as ::core::ffi::c_int
                                                as isize,
                                        ))
                                    .other_case
                                        as ::core::ffi::c_int)
                                    as uint32_t
                        {
                            rrc = 0 as ::core::ffi::c_int;
                            current_block = 9215777819543815842;
                        } else {
                            current_block = 16517180880614114163;
                        }
                        match current_block {
                            9215777819543815842 => {}
                            _ => {
                                (*F).eptr = (*F).eptr.offset(1);
                                (*F).ecode = (*F).ecode.offset(2 as ::core::ffi::c_int as isize);
                                continue;
                            }
                        }
                    } else if (if *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_uint
                        <= 255 as ::core::ffi::c_uint
                    {
                        *(*mb)
                            .lcc
                            .offset(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as isize)
                            as ::core::ffi::c_int
                    } else {
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    }) != (if *(*F).eptr as ::core::ffi::c_uint
                        <= 255 as ::core::ffi::c_uint
                    {
                        *(*mb).lcc.offset(*(*F).eptr as isize) as ::core::ffi::c_int
                    } else {
                        *(*F).eptr as ::core::ffi::c_int
                    }) {
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        (*F).eptr = (*F).eptr.offset(1);
                        (*F).ecode = (*F).ecode.offset(2 as ::core::ffi::c_int as isize);
                        continue;
                    }
                    current_block = 9215777819543815842;
                }
                31 | 32 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else if utf != 0 {
                        let mut ch: uint32_t = 0;
                        (*F).ecode = (*F).ecode.offset(1);
                        let fresh8 = (*F).ecode;
                        (*F).ecode = (*F).ecode.offset(1);
                        ch = *fresh8 as uint32_t;
                        if ch & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh9 = (*F).ecode;
                            (*F).ecode = (*F).ecode.offset(1);
                            ch = ((ch & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh9 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        let fresh10 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh10 as uint32_t;
                        if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh11 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh11 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        if ch == fc {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            if !((*F).op as ::core::ffi::c_int == OP_NOTI as ::core::ffi::c_int) {
                                continue;
                            }
                            if ch > 127 as uint32_t {
                                ch = (ch as ::core::ffi::c_int
                                    + (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                        .offset(
                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                .offset(
                                                    (*(&raw const _pcre2_ucd_stage1_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (ch as ::core::ffi::c_int
                                                                / UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        * UCD_BLOCK_SIZE
                                                        + ch as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                                        as isize,
                                                )
                                                as ::core::ffi::c_int
                                                as isize,
                                        ))
                                    .other_case
                                        as ::core::ffi::c_int)
                                    as uint32_t;
                            } else {
                                ch = *(*mb).fcc.offset(ch as isize) as uint32_t;
                            }
                            if !(ch == fc) {
                                continue;
                            }
                            rrc = 0 as ::core::ffi::c_int;
                        }
                    } else if ucp != 0 {
                        let mut ch_0: uint32_t = 0;
                        let fresh12 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh12 as uint32_t;
                        ch_0 = *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                        (*F).ecode = (*F).ecode.offset(2 as ::core::ffi::c_int as isize);
                        if ch_0 == fc {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            if !((*F).op as ::core::ffi::c_int == OP_NOTI as ::core::ffi::c_int) {
                                continue;
                            }
                            if ch_0 > 127 as uint32_t {
                                ch_0 = (ch_0 as ::core::ffi::c_int
                                    + (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                        .offset(
                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                .offset(
                                                    (*(&raw const _pcre2_ucd_stage1_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (ch_0 as ::core::ffi::c_int
                                                                / UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        * UCD_BLOCK_SIZE
                                                        + ch_0 as ::core::ffi::c_int
                                                            % UCD_BLOCK_SIZE)
                                                        as isize,
                                                )
                                                as ::core::ffi::c_int
                                                as isize,
                                        ))
                                    .other_case
                                        as ::core::ffi::c_int)
                                    as uint32_t;
                            } else {
                                ch_0 = *(*mb).fcc.offset(ch_0 as isize) as uint32_t;
                            }
                            if !(ch_0 == fc) {
                                continue;
                            }
                            rrc = 0 as ::core::ffi::c_int;
                        }
                    } else {
                        let mut ch_1: uint32_t =
                            *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                        let fresh13 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh13 as uint32_t;
                        if ch_1 == fc
                            || (*F).op as ::core::ffi::c_int == OP_NOTI as ::core::ffi::c_int
                                && (if ch_1 <= 255 as uint32_t {
                                    *(*mb).fcc.offset(ch_1 as isize) as uint32_t
                                } else {
                                    ch_1
                                }) == fc
                        {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            (*F).ecode = (*F).ecode.offset(2 as ::core::ffi::c_int as isize);
                            continue;
                        }
                    }
                    current_block = 9215777819543815842;
                }
                41 | 54 => {
                    (*F).fields.char_repeat.max =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    (*F).fields.char_repeat.min = (*F).fields.char_repeat.max;
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    current_block = 11724773952707043839;
                }
                45 | 58 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.char_repeat.min = 0 as uint32_t;
                    (*F).fields.char_repeat.max =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    current_block = 11724773952707043839;
                }
                39 | 52 => {
                    reptype = REPTYPE_MAX as ::core::ffi::c_int as uint32_t;
                    (*F).fields.char_repeat.min = 0 as uint32_t;
                    (*F).fields.char_repeat.max =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    current_block = 11724773952707043839;
                }
                40 | 53 => {
                    reptype = REPTYPE_MIN as ::core::ffi::c_int as uint32_t;
                    (*F).fields.char_repeat.min = 0 as uint32_t;
                    (*F).fields.char_repeat.max =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    current_block = 11724773952707043839;
                }
                42 | 55 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.char_repeat.min = 0 as uint32_t;
                    (*F).fields.char_repeat.max = UINT32_MAX as uint32_t;
                    (*F).ecode = (*F).ecode.offset(1);
                    current_block = 11724773952707043839;
                }
                43 | 56 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.char_repeat.min = 1 as uint32_t;
                    (*F).fields.char_repeat.max = UINT32_MAX as uint32_t;
                    (*F).ecode = (*F).ecode.offset(1);
                    current_block = 11724773952707043839;
                }
                44 | 57 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.char_repeat.min = 0 as uint32_t;
                    (*F).fields.char_repeat.max = 1 as uint32_t;
                    (*F).ecode = (*F).ecode.offset(1);
                    current_block = 11724773952707043839;
                }
                33 | 46 | 34 | 47 | 35 | 48 | 36 | 49 | 37 | 50 | 38 | 51 => {
                    let fresh14 = (*F).ecode;
                    (*F).ecode = (*F).ecode.offset(1);
                    fc = (*fresh14 as ::core::ffi::c_int
                        - (if ((*F).op as ::core::ffi::c_int) < OP_STARI as ::core::ffi::c_int {
                            OP_STAR as ::core::ffi::c_int
                        } else {
                            OP_STARI as ::core::ffi::c_int
                        })) as uint32_t;
                    (*F).fields.char_repeat.min = rep_min[fc as usize];
                    (*F).fields.char_repeat.max = rep_max[fc as usize];
                    reptype = rep_typ[fc as usize];
                    current_block = 11724773952707043839;
                }
                67 | 80 => {
                    (*F).fields.charnot_repeat.max =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    (*F).fields.charnot_repeat.min = (*F).fields.charnot_repeat.max;
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    current_block = 7201258650426958033;
                }
                65 | 78 => {
                    (*F).fields.charnot_repeat.min = 0 as uint32_t;
                    (*F).fields.charnot_repeat.max =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    reptype = REPTYPE_MAX as ::core::ffi::c_int as uint32_t;
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    current_block = 7201258650426958033;
                }
                66 | 79 => {
                    (*F).fields.charnot_repeat.min = 0 as uint32_t;
                    (*F).fields.charnot_repeat.max =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    reptype = REPTYPE_MIN as ::core::ffi::c_int as uint32_t;
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    current_block = 7201258650426958033;
                }
                68 | 81 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.charnot_repeat.min = 0 as uint32_t;
                    (*F).fields.charnot_repeat.max = UINT32_MAX as uint32_t;
                    (*F).ecode = (*F).ecode.offset(1);
                    current_block = 7201258650426958033;
                }
                69 | 82 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.charnot_repeat.min = 1 as uint32_t;
                    (*F).fields.charnot_repeat.max = UINT32_MAX as uint32_t;
                    (*F).ecode = (*F).ecode.offset(1);
                    current_block = 7201258650426958033;
                }
                70 | 83 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.charnot_repeat.min = 0 as uint32_t;
                    (*F).fields.charnot_repeat.max = 1 as uint32_t;
                    (*F).ecode = (*F).ecode.offset(1);
                    current_block = 7201258650426958033;
                }
                71 | 84 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.charnot_repeat.min = 0 as uint32_t;
                    (*F).fields.charnot_repeat.max =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    current_block = 7201258650426958033;
                }
                59 | 72 | 60 | 73 | 61 | 74 | 62 | 75 | 63 | 76 | 64 | 77 => {
                    let fresh21 = (*F).ecode;
                    (*F).ecode = (*F).ecode.offset(1);
                    fc = (*fresh21 as ::core::ffi::c_int
                        - (if (*F).op as ::core::ffi::c_int >= OP_NOTSTARI as ::core::ffi::c_int {
                            OP_NOTSTARI as ::core::ffi::c_int
                        } else {
                            OP_NOTSTAR as ::core::ffi::c_int
                        })) as uint32_t;
                    (*F).fields.charnot_repeat.min = rep_min[fc as usize];
                    (*F).fields.charnot_repeat.max = rep_max[fc as usize];
                    reptype = rep_typ[fc as usize];
                    current_block = 7201258650426958033;
                }
                111 | 110 => {
                    (*F).fields.class_repeat.byte_map_address =
                        (*F).ecode.offset(1 as ::core::ffi::c_int as isize);
                    (*F).ecode = (*F).ecode.offset(
                        (1 as usize).wrapping_add(
                            (32 as usize)
                                .wrapping_div(::core::mem::size_of::<PCRE2_UCHAR16>() as usize),
                        ) as isize,
                    );
                    match *(*F).ecode as ::core::ffi::c_int {
                        98 | 99 | 100 | 101 | 102 | 103 | 106 | 107 | 108 => {
                            let fresh38 = (*F).ecode;
                            (*F).ecode = (*F).ecode.offset(1);
                            fc = (*fresh38 as ::core::ffi::c_int - OP_CRSTAR as ::core::ffi::c_int)
                                as uint32_t;
                            (*F).fields.class_repeat.min = rep_min[fc as usize];
                            (*F).fields.class_repeat.max = rep_max[fc as usize];
                            reptype = rep_typ[fc as usize];
                        }
                        104 | 105 | 109 => {
                            (*F).fields.class_repeat.min =
                                *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                            (*F).fields.class_repeat.max = *(*F).ecode.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            )
                                as uint32_t;
                            if (*F).fields.class_repeat.max == 0 as uint32_t {
                                (*F).fields.class_repeat.max = UINT32_MAX as uint32_t;
                            }
                            reptype = rep_typ[(*(*F).ecode as ::core::ffi::c_int
                                - OP_CRSTAR as ::core::ffi::c_int)
                                as usize];
                            (*F).ecode = (*F).ecode.offset(
                                (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                    as isize,
                            );
                        }
                        _ => {
                            (*F).fields.class_repeat.max = 1 as uint32_t;
                            (*F).fields.class_repeat.min = (*F).fields.class_repeat.max;
                        }
                    }
                    if utf != 0 {
                        i = 1 as uint32_t;
                        loop {
                            if !(i <= (*F).fields.class_repeat.min) {
                                current_block = 16792120503462404091;
                                break;
                            }
                            if (*F).eptr >= (*mb).end_subject {
                                if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                                    && ((*F).eptr > (*mb).start_used_ptr
                                        || (*mb).allowemptypartial != 0)
                                {
                                    (*mb).hitend = TRUE as BOOL;
                                    if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int
                                    {
                                        return PCRE2_ERROR_PARTIAL;
                                    }
                                }
                                rrc = 0 as ::core::ffi::c_int;
                                current_block = 9215777819543815842;
                                break;
                            } else {
                                let fresh39 = (*F).eptr;
                                (*F).eptr = (*F).eptr.offset(1);
                                fc = *fresh39 as uint32_t;
                                if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                    let fresh40 = (*F).eptr;
                                    (*F).eptr = (*F).eptr.offset(1);
                                    fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                        | *fresh40 as uint32_t & 0x3ff as uint32_t)
                                        .wrapping_add(0x10000 as uint32_t);
                                }
                                if fc > 255 as uint32_t {
                                    if (*F).op as ::core::ffi::c_int
                                        == OP_CLASS as ::core::ffi::c_int
                                    {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        break;
                                    }
                                } else if *((*F).fields.class_repeat.byte_map_address
                                    as *const ::core::ffi::c_uchar)
                                    .offset(fc.wrapping_div(8 as uint32_t) as isize)
                                    as ::core::ffi::c_uint
                                    & (1 as ::core::ffi::c_uint) << (fc & 7 as uint32_t)
                                    == 0 as ::core::ffi::c_uint
                                {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    break;
                                }
                                i = i.wrapping_add(1);
                            }
                        }
                    } else {
                        i = 1 as uint32_t;
                        loop {
                            if !(i <= (*F).fields.class_repeat.min) {
                                current_block = 16792120503462404091;
                                break;
                            }
                            if (*F).eptr >= (*mb).end_subject {
                                if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                                    && ((*F).eptr > (*mb).start_used_ptr
                                        || (*mb).allowemptypartial != 0)
                                {
                                    (*mb).hitend = TRUE as BOOL;
                                    if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int
                                    {
                                        return PCRE2_ERROR_PARTIAL;
                                    }
                                }
                                rrc = 0 as ::core::ffi::c_int;
                                current_block = 9215777819543815842;
                                break;
                            } else {
                                let fresh41 = (*F).eptr;
                                (*F).eptr = (*F).eptr.offset(1);
                                fc = *fresh41 as uint32_t;
                                if fc > 255 as uint32_t {
                                    if (*F).op as ::core::ffi::c_int
                                        == OP_CLASS as ::core::ffi::c_int
                                    {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        break;
                                    }
                                } else if *((*F).fields.class_repeat.byte_map_address
                                    as *const ::core::ffi::c_uchar)
                                    .offset(fc.wrapping_div(8 as uint32_t) as isize)
                                    as ::core::ffi::c_uint
                                    & (1 as ::core::ffi::c_uint) << (fc & 7 as uint32_t)
                                    == 0 as ::core::ffi::c_uint
                                {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    break;
                                }
                                i = i.wrapping_add(1);
                            }
                        }
                    }
                    match current_block {
                        9215777819543815842 => {}
                        _ => {
                            if (*F).fields.class_repeat.min == (*F).fields.class_repeat.max {
                                continue;
                            }
                            if reptype == REPTYPE_MIN as ::core::ffi::c_int as uint32_t {
                                if utf != 0 {
                                    current_block = 10553790310514340151;
                                    break;
                                } else {
                                    current_block = 4519209830585050666;
                                    break;
                                }
                            } else {
                                (*F).fields.class_repeat.start_eptr = (*F).eptr;
                                if utf != 0 {
                                    i = (*F).fields.class_repeat.min;
                                    while i < (*F).fields.class_repeat.max {
                                        let mut len_1: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            break;
                                        } else {
                                            fc = *(*F).eptr as uint32_t;
                                            if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *(*F)
                                                        .eptr
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                                len_1 += 1;
                                            }
                                            if fc > 255 as uint32_t {
                                                if (*F).op as ::core::ffi::c_int
                                                    == OP_CLASS as ::core::ffi::c_int
                                                {
                                                    break;
                                                }
                                            } else if *((*F).fields.class_repeat.byte_map_address
                                                as *const ::core::ffi::c_uchar)
                                                .offset(fc.wrapping_div(8 as uint32_t) as isize)
                                                as ::core::ffi::c_uint
                                                & (1 as ::core::ffi::c_uint) << (fc & 7 as uint32_t)
                                                == 0 as ::core::ffi::c_uint
                                            {
                                                break;
                                            }
                                            (*F).eptr = (*F).eptr.offset(len_1 as isize);
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                    if reptype == REPTYPE_POS as ::core::ffi::c_int as uint32_t {
                                        continue;
                                    } else {
                                        current_block = 13776980873676388703;
                                        break;
                                    }
                                } else {
                                    i = (*F).fields.class_repeat.min;
                                    while i < (*F).fields.class_repeat.max {
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            break;
                                        } else {
                                            fc = *(*F).eptr as uint32_t;
                                            if fc > 255 as uint32_t {
                                                if (*F).op as ::core::ffi::c_int
                                                    == OP_CLASS as ::core::ffi::c_int
                                                {
                                                    break;
                                                }
                                            } else if *((*F).fields.class_repeat.byte_map_address
                                                as *const ::core::ffi::c_uchar)
                                                .offset(fc.wrapping_div(8 as uint32_t) as isize)
                                                as ::core::ffi::c_uint
                                                & (1 as ::core::ffi::c_uint) << (fc & 7 as uint32_t)
                                                == 0 as ::core::ffi::c_uint
                                            {
                                                break;
                                            }
                                            (*F).eptr = (*F).eptr.offset(1);
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                    if reptype == REPTYPE_POS as ::core::ffi::c_int as uint32_t {
                                        continue;
                                    }
                                }
                            }
                            current_block = 15840449704302887135;
                        }
                    }
                }
                112 => {
                    (*F).fields.xclass_repeat.xclass_data = (*F)
                        .ecode
                        .offset(1 as ::core::ffi::c_int as isize)
                        .offset(LINK_SIZE as isize);
                    (*F).ecode = (*F)
                        .ecode
                        .offset(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize);
                    match *(*F).ecode as ::core::ffi::c_int {
                        98 | 99 | 100 | 101 | 102 | 103 | 106 | 107 | 108 => {
                            let fresh48 = (*F).ecode;
                            (*F).ecode = (*F).ecode.offset(1);
                            fc = (*fresh48 as ::core::ffi::c_int - OP_CRSTAR as ::core::ffi::c_int)
                                as uint32_t;
                            (*F).fields.xclass_repeat.min = rep_min[fc as usize];
                            (*F).fields.xclass_repeat.max = rep_max[fc as usize];
                            reptype = rep_typ[fc as usize];
                        }
                        104 | 105 | 109 => {
                            (*F).fields.xclass_repeat.min =
                                *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                            (*F).fields.xclass_repeat.max = *(*F).ecode.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            )
                                as uint32_t;
                            if (*F).fields.xclass_repeat.max == 0 as uint32_t {
                                (*F).fields.xclass_repeat.max = UINT32_MAX as uint32_t;
                            }
                            reptype = rep_typ[(*(*F).ecode as ::core::ffi::c_int
                                - OP_CRSTAR as ::core::ffi::c_int)
                                as usize];
                            (*F).ecode = (*F).ecode.offset(
                                (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                    as isize,
                            );
                        }
                        _ => {
                            (*F).fields.xclass_repeat.max = 1 as uint32_t;
                            (*F).fields.xclass_repeat.min = (*F).fields.xclass_repeat.max;
                        }
                    }
                    i = 1 as uint32_t;
                    loop {
                        if !(i <= (*F).fields.xclass_repeat.min) {
                            current_block = 4755831821890624027;
                            break;
                        }
                        if (*F).eptr >= (*mb).end_subject {
                            if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                                && ((*F).eptr > (*mb).start_used_ptr
                                    || (*mb).allowemptypartial != 0)
                            {
                                (*mb).hitend = TRUE as BOOL;
                                if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                    return PCRE2_ERROR_PARTIAL;
                                }
                            }
                            rrc = 0 as ::core::ffi::c_int;
                            current_block = 9215777819543815842;
                            break;
                        } else {
                            let fresh49 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = *fresh49 as uint32_t;
                            if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                let fresh50 = (*F).eptr;
                                (*F).eptr = (*F).eptr.offset(1);
                                fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                    | *fresh50 as uint32_t & 0x3ff as uint32_t)
                                    .wrapping_add(0x10000 as uint32_t);
                            }
                            if _pcre2_xclass_16(
                                fc,
                                (*F).fields.xclass_repeat.xclass_data,
                                (*mb).start_code as *const uint8_t,
                                utf,
                            ) == 0
                            {
                                rrc = 0 as ::core::ffi::c_int;
                                current_block = 9215777819543815842;
                                break;
                            } else {
                                i = i.wrapping_add(1);
                            }
                        }
                    }
                    match current_block {
                        9215777819543815842 => {}
                        _ => {
                            if (*F).fields.xclass_repeat.min == (*F).fields.xclass_repeat.max {
                                continue;
                            }
                            if reptype == REPTYPE_MIN as ::core::ffi::c_int as uint32_t {
                                current_block = 3302068592069234587;
                                break;
                            }
                            (*F).fields.xclass_repeat.start_eptr = (*F).eptr;
                            i = (*F).fields.xclass_repeat.min;
                            while i < (*F).fields.xclass_repeat.max {
                                let mut len_2: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
                                if (*F).eptr >= (*mb).end_subject {
                                    if (*mb).partial as ::core::ffi::c_int
                                        != 0 as ::core::ffi::c_int
                                        && ((*F).eptr > (*mb).start_used_ptr
                                            || (*mb).allowemptypartial != 0)
                                    {
                                        (*mb).hitend = TRUE as BOOL;
                                        if (*mb).partial as ::core::ffi::c_int
                                            > 1 as ::core::ffi::c_int
                                        {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                    }
                                    break;
                                } else {
                                    fc = *(*F).eptr as uint32_t;
                                    if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                        fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                            | *(*F).eptr.offset(1 as ::core::ffi::c_int as isize)
                                                as uint32_t
                                                & 0x3ff as uint32_t)
                                            .wrapping_add(0x10000 as uint32_t);
                                        len_2 += 1;
                                    }
                                    if _pcre2_xclass_16(
                                        fc,
                                        (*F).fields.xclass_repeat.xclass_data,
                                        (*mb).start_code as *const uint8_t,
                                        utf,
                                    ) == 0
                                    {
                                        break;
                                    }
                                    (*F).eptr = (*F).eptr.offset(len_2 as isize);
                                    i = i.wrapping_add(1);
                                }
                            }
                            if reptype == REPTYPE_POS as ::core::ffi::c_int as uint32_t {
                                continue;
                            } else {
                                current_block = 17975895912995949440;
                                break;
                            }
                        }
                    }
                }
                113 => {
                    (*F).fields.eclass_repeat.eclass_data = (*F)
                        .ecode
                        .offset(1 as ::core::ffi::c_int as isize)
                        .offset(LINK_SIZE as isize);
                    (*F).ecode = (*F)
                        .ecode
                        .offset(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize);
                    (*F).fields.eclass_repeat.eclass_len =
                        (*F).ecode
                            .offset_from((*F).fields.eclass_repeat.eclass_data)
                            as isize as size_t;
                    match *(*F).ecode as ::core::ffi::c_int {
                        98 | 99 | 100 | 101 | 102 | 103 | 106 | 107 | 108 => {
                            let fresh55 = (*F).ecode;
                            (*F).ecode = (*F).ecode.offset(1);
                            fc = (*fresh55 as ::core::ffi::c_int - OP_CRSTAR as ::core::ffi::c_int)
                                as uint32_t;
                            (*F).fields.eclass_repeat.min = rep_min[fc as usize];
                            (*F).fields.eclass_repeat.max = rep_max[fc as usize];
                            reptype = rep_typ[fc as usize];
                        }
                        104 | 105 | 109 => {
                            (*F).fields.eclass_repeat.min =
                                *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                            (*F).fields.eclass_repeat.max = *(*F).ecode.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            )
                                as uint32_t;
                            if (*F).fields.eclass_repeat.max == 0 as uint32_t {
                                (*F).fields.eclass_repeat.max = UINT32_MAX as uint32_t;
                            }
                            reptype = rep_typ[(*(*F).ecode as ::core::ffi::c_int
                                - OP_CRSTAR as ::core::ffi::c_int)
                                as usize];
                            (*F).ecode = (*F).ecode.offset(
                                (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                    as isize,
                            );
                        }
                        _ => {
                            (*F).fields.eclass_repeat.max = 1 as uint32_t;
                            (*F).fields.eclass_repeat.min = (*F).fields.eclass_repeat.max;
                        }
                    }
                    i = 1 as uint32_t;
                    loop {
                        if !(i <= (*F).fields.eclass_repeat.min) {
                            current_block = 13124292477636358238;
                            break;
                        }
                        if (*F).eptr >= (*mb).end_subject {
                            if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                                && ((*F).eptr > (*mb).start_used_ptr
                                    || (*mb).allowemptypartial != 0)
                            {
                                (*mb).hitend = TRUE as BOOL;
                                if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                    return PCRE2_ERROR_PARTIAL;
                                }
                            }
                            rrc = 0 as ::core::ffi::c_int;
                            current_block = 9215777819543815842;
                            break;
                        } else {
                            let fresh56 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = *fresh56 as uint32_t;
                            if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                let fresh57 = (*F).eptr;
                                (*F).eptr = (*F).eptr.offset(1);
                                fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                    | *fresh57 as uint32_t & 0x3ff as uint32_t)
                                    .wrapping_add(0x10000 as uint32_t);
                            }
                            if _pcre2_eclass_16(
                                fc,
                                (*F).fields.eclass_repeat.eclass_data,
                                (*F).fields
                                    .eclass_repeat
                                    .eclass_data
                                    .offset((*F).fields.eclass_repeat.eclass_len as isize),
                                (*mb).start_code as *const uint8_t,
                                utf,
                            ) == 0
                            {
                                rrc = 0 as ::core::ffi::c_int;
                                current_block = 9215777819543815842;
                                break;
                            } else {
                                i = i.wrapping_add(1);
                            }
                        }
                    }
                    match current_block {
                        9215777819543815842 => {}
                        _ => {
                            if (*F).fields.eclass_repeat.min == (*F).fields.eclass_repeat.max {
                                continue;
                            }
                            if reptype == REPTYPE_MIN as ::core::ffi::c_int as uint32_t {
                                current_block = 6265774175505686097;
                                break;
                            }
                            (*F).fields.eclass_repeat.start_eptr = (*F).eptr;
                            i = (*F).fields.eclass_repeat.min;
                            while i < (*F).fields.eclass_repeat.max {
                                let mut len_3: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
                                if (*F).eptr >= (*mb).end_subject {
                                    if (*mb).partial as ::core::ffi::c_int
                                        != 0 as ::core::ffi::c_int
                                        && ((*F).eptr > (*mb).start_used_ptr
                                            || (*mb).allowemptypartial != 0)
                                    {
                                        (*mb).hitend = TRUE as BOOL;
                                        if (*mb).partial as ::core::ffi::c_int
                                            > 1 as ::core::ffi::c_int
                                        {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                    }
                                    break;
                                } else {
                                    fc = *(*F).eptr as uint32_t;
                                    if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                        fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                            | *(*F).eptr.offset(1 as ::core::ffi::c_int as isize)
                                                as uint32_t
                                                & 0x3ff as uint32_t)
                                            .wrapping_add(0x10000 as uint32_t);
                                        len_3 += 1;
                                    }
                                    if _pcre2_eclass_16(
                                        fc,
                                        (*F).fields.eclass_repeat.eclass_data,
                                        (*F).fields
                                            .eclass_repeat
                                            .eclass_data
                                            .offset((*F).fields.eclass_repeat.eclass_len as isize),
                                        (*mb).start_code as *const uint8_t,
                                        utf,
                                    ) == 0
                                    {
                                        break;
                                    }
                                    (*F).eptr = (*F).eptr.offset(len_3 as isize);
                                    i = i.wrapping_add(1);
                                }
                            }
                            if reptype == REPTYPE_POS as ::core::ffi::c_int as uint32_t {
                                continue;
                            } else {
                                current_block = 12574270518964373171;
                                break;
                            }
                        }
                    }
                }
                6 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh62 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh62 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh63 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh63 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        if fc <= 255 as uint32_t
                            && *(*mb).ctypes.offset(fc as isize) as ::core::ffi::c_int & ctype_digit
                                != 0 as ::core::ffi::c_int
                        {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            (*F).ecode = (*F).ecode.offset(1);
                            continue;
                        }
                    }
                    current_block = 9215777819543815842;
                }
                7 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh64 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh64 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh65 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh65 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        if !(fc <= 255 as uint32_t)
                            || *(*mb).ctypes.offset(fc as isize) as ::core::ffi::c_int & ctype_digit
                                == 0 as ::core::ffi::c_int
                        {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            (*F).ecode = (*F).ecode.offset(1);
                            continue;
                        }
                    }
                    current_block = 9215777819543815842;
                }
                8 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh66 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh66 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh67 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh67 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        if fc <= 255 as uint32_t
                            && *(*mb).ctypes.offset(fc as isize) as ::core::ffi::c_int & ctype_space
                                != 0 as ::core::ffi::c_int
                        {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            (*F).ecode = (*F).ecode.offset(1);
                            continue;
                        }
                    }
                    current_block = 9215777819543815842;
                }
                9 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh68 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh68 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh69 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh69 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        if !(fc <= 255 as uint32_t)
                            || *(*mb).ctypes.offset(fc as isize) as ::core::ffi::c_int & ctype_space
                                == 0 as ::core::ffi::c_int
                        {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            (*F).ecode = (*F).ecode.offset(1);
                            continue;
                        }
                    }
                    current_block = 9215777819543815842;
                }
                10 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh70 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh70 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh71 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh71 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        if fc <= 255 as uint32_t
                            && *(*mb).ctypes.offset(fc as isize) as ::core::ffi::c_int & ctype_word
                                != 0 as ::core::ffi::c_int
                        {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            (*F).ecode = (*F).ecode.offset(1);
                            continue;
                        }
                    }
                    current_block = 9215777819543815842;
                }
                11 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh72 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh72 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh73 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh73 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        if !(fc <= 255 as uint32_t)
                            || *(*mb).ctypes.offset(fc as isize) as ::core::ffi::c_int & ctype_word
                                == 0 as ::core::ffi::c_int
                        {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            (*F).ecode = (*F).ecode.offset(1);
                            continue;
                        }
                    }
                    current_block = 9215777819543815842;
                }
                17 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh74 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh74 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh75 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh75 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        match fc {
                            13 => {
                                if (*F).eptr >= (*mb).end_subject {
                                    if (*mb).partial as ::core::ffi::c_int
                                        != 0 as ::core::ffi::c_int
                                        && ((*F).eptr > (*mb).start_used_ptr
                                            || (*mb).allowemptypartial != 0)
                                    {
                                        (*mb).hitend = TRUE as BOOL;
                                        if (*mb).partial as ::core::ffi::c_int
                                            > 1 as ::core::ffi::c_int
                                        {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                    }
                                } else if *(*F).eptr as ::core::ffi::c_int == CHAR_LF {
                                    (*F).eptr = (*F).eptr.offset(1);
                                }
                                current_block = 17725334101907426919;
                            }
                            10 => {
                                current_block = 17725334101907426919;
                            }
                            11 | 12 | 133 | 8232 | 8233 => {
                                if (*mb).bsr_convention as ::core::ffi::c_int == PCRE2_BSR_ANYCRLF {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                } else {
                                    current_block = 17725334101907426919;
                                }
                            }
                            _ => {
                                rrc = 0 as ::core::ffi::c_int;
                                current_block = 9215777819543815842;
                            }
                        }
                        match current_block {
                            9215777819543815842 => {}
                            _ => {
                                (*F).ecode = (*F).ecode.offset(1);
                                continue;
                            }
                        }
                    }
                    current_block = 9215777819543815842;
                }
                18 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh76 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh76 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh77 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh77 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        match fc {
                            9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194 | 8195 | 8196
                            | 8197 | 8198 | 8199 | 8200 | 8201 | 8202 | 8239 | 8287 | 12288 => {
                                rrc = 0 as ::core::ffi::c_int;
                            }
                            _ => {
                                (*F).ecode = (*F).ecode.offset(1);
                                continue;
                            }
                        }
                    }
                    current_block = 9215777819543815842;
                }
                19 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh78 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh78 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh79 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh79 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        match fc {
                            9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194 | 8195 | 8196
                            | 8197 | 8198 | 8199 | 8200 | 8201 | 8202 | 8239 | 8287 | 12288 => {
                                (*F).ecode = (*F).ecode.offset(1);
                                continue;
                            }
                            _ => {
                                rrc = 0 as ::core::ffi::c_int;
                            }
                        }
                    }
                    current_block = 9215777819543815842;
                }
                20 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh80 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh80 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh81 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh81 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        match fc {
                            10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                rrc = 0 as ::core::ffi::c_int;
                            }
                            _ => {
                                (*F).ecode = (*F).ecode.offset(1);
                                continue;
                            }
                        }
                    }
                    current_block = 9215777819543815842;
                }
                21 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh82 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh82 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh83 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh83 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        match fc {
                            10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                (*F).ecode = (*F).ecode.offset(1);
                                continue;
                            }
                            _ => {
                                rrc = 0 as ::core::ffi::c_int;
                            }
                        }
                    }
                    current_block = 9215777819543815842;
                }
                16 | 15 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh84 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh84 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh85 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh85 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        let mut cp: *const uint32_t = ::core::ptr::null::<uint32_t>();
                        let mut chartype: uint32_t = 0;
                        let mut prop: *const ucd_record =
                            (&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
                                *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                                    (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t).offset(
                                        (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                    ) as ::core::ffi::c_int
                                        * UCD_BLOCK_SIZE
                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                        as isize,
                                ) as ::core::ffi::c_int as isize,
                            );
                        let mut notmatch: BOOL = ((*F).op as ::core::ffi::c_int
                            == OP_NOTPROP as ::core::ffi::c_int)
                            as ::core::ffi::c_int;
                        match *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int
                        {
                            PT_LAMP => {
                                chartype = (*prop).chartype as uint32_t;
                                if (chartype == ucp_Lu as ::core::ffi::c_int as uint32_t
                                    || chartype == ucp_Ll as ::core::ffi::c_int as uint32_t
                                    || chartype == ucp_Lt as ::core::ffi::c_int as uint32_t)
                                    as ::core::ffi::c_int
                                    == notmatch
                                {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                } else {
                                    current_block = 15787157066674164823;
                                }
                            }
                            PT_GC => {
                                if (*(*F).ecode.offset(2 as ::core::ffi::c_int as isize)
                                    as uint32_t
                                    == *(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                        .offset((*prop).chartype as isize))
                                    as ::core::ffi::c_int
                                    == notmatch
                                {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                } else {
                                    current_block = 15787157066674164823;
                                }
                            }
                            PT_PC => {
                                if (*(*F).ecode.offset(2 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int
                                    == (*prop).chartype as ::core::ffi::c_int)
                                    as ::core::ffi::c_int
                                    == notmatch
                                {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                } else {
                                    current_block = 15787157066674164823;
                                }
                            }
                            PT_SC => {
                                if (*(*F).ecode.offset(2 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int
                                    == (*prop).script as ::core::ffi::c_int)
                                    as ::core::ffi::c_int
                                    == notmatch
                                {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                } else {
                                    current_block = 15787157066674164823;
                                }
                            }
                            PT_SCX => {
                                let mut ok: BOOL = (*(*F)
                                    .ecode
                                    .offset(2 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int
                                    == (*prop).script as ::core::ffi::c_int
                                    || *(&raw const _pcre2_ucd_script_sets_16 as *const uint32_t)
                                        .offset(
                                            ((*prop).scriptx_bidiclass as ::core::ffi::c_int
                                                & 0x3ff as ::core::ffi::c_int)
                                                as isize,
                                        )
                                        .offset(
                                            (*(*F).ecode.offset(2 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                / 32 as ::core::ffi::c_int)
                                                as isize,
                                        )
                                        & (1 as uint32_t)
                                            << *(*F).ecode.offset(2 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                % 32 as ::core::ffi::c_int
                                        != 0 as uint32_t)
                                    as ::core::ffi::c_int;
                                if ok == notmatch {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                } else {
                                    current_block = 15787157066674164823;
                                }
                            }
                            PT_ALNUM => {
                                chartype = (*prop).chartype as uint32_t;
                                if (*(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                    .offset(chartype as isize)
                                    == ucp_L as ::core::ffi::c_int as uint32_t
                                    || *(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                        .offset(chartype as isize)
                                        == ucp_N as ::core::ffi::c_int as uint32_t)
                                    as ::core::ffi::c_int
                                    == notmatch
                                {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                } else {
                                    current_block = 15787157066674164823;
                                }
                            }
                            PT_SPACE | PT_PXSPACE => match fc {
                                9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194 | 8195 | 8196
                                | 8197 | 8198 | 8199 | 8200 | 8201 | 8202 | 8239 | 8287 | 12288
                                | 10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                    if notmatch != 0 {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                    } else {
                                        current_block = 15787157066674164823;
                                    }
                                }
                                _ => {
                                    if (*(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                        .offset((*prop).chartype as isize)
                                        == ucp_Z as ::core::ffi::c_int as uint32_t)
                                        as ::core::ffi::c_int
                                        == notmatch
                                    {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                    } else {
                                        current_block = 15787157066674164823;
                                    }
                                }
                            },
                            PT_WORD => {
                                chartype = (*prop).chartype as uint32_t;
                                if (*(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                    .offset(chartype as isize)
                                    == ucp_L as ::core::ffi::c_int as uint32_t
                                    || *(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                        .offset(chartype as isize)
                                        == ucp_N as ::core::ffi::c_int as uint32_t
                                    || chartype == ucp_Mn as ::core::ffi::c_int as uint32_t
                                    || chartype == ucp_Pc as ::core::ffi::c_int as uint32_t)
                                    as ::core::ffi::c_int
                                    == notmatch
                                {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                } else {
                                    current_block = 15787157066674164823;
                                }
                            }
                            PT_CLIST => {
                                cp = (&raw const _pcre2_ucd_caseless_sets_16 as *const uint32_t)
                                    .offset(*(*F).ecode.offset(2 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        as isize);
                                loop {
                                    if fc < *cp {
                                        if notmatch != 0 {
                                            current_block = 15787157066674164823;
                                            break;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        break;
                                    } else {
                                        let fresh86 = cp;
                                        cp = cp.offset(1);
                                        if !(fc == *fresh86) {
                                            continue;
                                        }
                                        if !(notmatch != 0) {
                                            current_block = 15787157066674164823;
                                            break;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        break;
                                    }
                                }
                            }
                            PT_UCNC => {
                                if (fc == CHAR_DOLLAR_SIGN as uint32_t
                                    || fc == CHAR_COMMERCIAL_AT as uint32_t
                                    || fc == CHAR_GRAVE_ACCENT as uint32_t
                                    || fc >= 0xa0 as uint32_t && fc <= 0xd7ff as uint32_t
                                    || fc >= 0xe000 as uint32_t)
                                    as ::core::ffi::c_int
                                    == notmatch
                                {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                } else {
                                    current_block = 15787157066674164823;
                                }
                            }
                            PT_BIDICL => {
                                if ((*prop).scriptx_bidiclass as ::core::ffi::c_int
                                    >> UCD_BIDICLASS_SHIFT
                                    == *(*F).ecode.offset(2 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int)
                                    as ::core::ffi::c_int
                                    == notmatch
                                {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                } else {
                                    current_block = 15787157066674164823;
                                }
                            }
                            PT_BOOL => {
                                let mut ok_0: BOOL = (*(&raw const _pcre2_ucd_boolprop_sets_16
                                    as *const uint32_t)
                                    .offset(
                                        ((*prop).bprops as ::core::ffi::c_int
                                            & 0xfff as ::core::ffi::c_int)
                                            as isize,
                                    )
                                    .offset(
                                        (*(*F).ecode.offset(2 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            / 32 as ::core::ffi::c_int)
                                            as isize,
                                    )
                                    & (1 as uint32_t)
                                        << *(*F).ecode.offset(2 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            % 32 as ::core::ffi::c_int
                                    != 0 as uint32_t)
                                    as ::core::ffi::c_int;
                                if ok_0 == notmatch {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                } else {
                                    current_block = 15787157066674164823;
                                }
                            }
                            _ => return PCRE2_ERROR_INTERNAL,
                        }
                        match current_block {
                            9215777819543815842 => {}
                            _ => {
                                (*F).ecode = (*F).ecode.offset(3 as ::core::ffi::c_int as isize);
                                continue;
                            }
                        }
                    }
                    current_block = 9215777819543815842;
                }
                22 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        let fresh87 = (*F).eptr;
                        (*F).eptr = (*F).eptr.offset(1);
                        fc = *fresh87 as uint32_t;
                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            let fresh88 = (*F).eptr;
                            (*F).eptr = (*F).eptr.offset(1);
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *fresh88 as uint32_t & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                        }
                        (*F).eptr = _pcre2_extuni_16(
                            fc,
                            (*F).eptr,
                            (*mb).check_subject,
                            (*mb).end_subject,
                            utf,
                            ::core::ptr::null_mut::<::core::ffi::c_int>(),
                        );
                        if (*F).eptr >= (*mb).end_subject {
                            if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                                && ((*F).eptr > (*mb).start_used_ptr
                                    || (*mb).allowemptypartial != 0)
                            {
                                (*mb).hitend = TRUE as BOOL;
                                if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                    return PCRE2_ERROR_PARTIAL;
                                }
                            }
                        }
                        (*F).ecode = (*F).ecode.offset(1);
                        continue;
                    }
                    current_block = 9215777819543815842;
                }
                93 => {
                    (*F).fields.type_repeat.max =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    (*F).fields.type_repeat.min = (*F).fields.type_repeat.max;
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    current_block = 6634154589341350760;
                }
                91 | 92 => {
                    (*F).fields.type_repeat.min = 0 as uint32_t;
                    (*F).fields.type_repeat.max =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    reptype = (if *(*F).ecode as ::core::ffi::c_int
                        == OP_TYPEMINUPTO as ::core::ffi::c_int
                    {
                        REPTYPE_MIN as ::core::ffi::c_int
                    } else {
                        REPTYPE_MAX as ::core::ffi::c_int
                    }) as uint32_t;
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    current_block = 6634154589341350760;
                }
                94 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.type_repeat.min = 0 as uint32_t;
                    (*F).fields.type_repeat.max = UINT32_MAX as uint32_t;
                    (*F).ecode = (*F).ecode.offset(1);
                    current_block = 6634154589341350760;
                }
                95 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.type_repeat.min = 1 as uint32_t;
                    (*F).fields.type_repeat.max = UINT32_MAX as uint32_t;
                    (*F).ecode = (*F).ecode.offset(1);
                    current_block = 6634154589341350760;
                }
                96 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.type_repeat.min = 0 as uint32_t;
                    (*F).fields.type_repeat.max = 1 as uint32_t;
                    (*F).ecode = (*F).ecode.offset(1);
                    current_block = 6634154589341350760;
                }
                97 => {
                    reptype = REPTYPE_POS as ::core::ffi::c_int as uint32_t;
                    (*F).fields.type_repeat.min = 0 as uint32_t;
                    (*F).fields.type_repeat.max =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    current_block = 6634154589341350760;
                }
                85 | 86 | 87 | 88 | 89 | 90 => {
                    let fresh89 = (*F).ecode;
                    (*F).ecode = (*F).ecode.offset(1);
                    fc = (*fresh89 as ::core::ffi::c_int - OP_TYPESTAR as ::core::ffi::c_int)
                        as uint32_t;
                    (*F).fields.type_repeat.min = rep_min[fc as usize];
                    (*F).fields.type_repeat.max = rep_max[fc as usize];
                    reptype = rep_typ[fc as usize];
                    current_block = 6634154589341350760;
                }
                116 | 117 => {
                    (*F).byte1 = ((*F).op as ::core::ffi::c_int == OP_DNREFI as ::core::ffi::c_int)
                        as ::core::ffi::c_int as uint8_t;
                    (*F).byte2 =
                        (if (*F).op as ::core::ffi::c_int == OP_DNREFI as ::core::ffi::c_int {
                            *(*F).ecode.offset(
                                (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                    as isize,
                            ) as ::core::ffi::c_int
                        } else {
                            0 as ::core::ffi::c_int
                        }) as uint8_t;
                    let mut count: ::core::ffi::c_int = *(*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                        as ::core::ffi::c_int;
                    let mut slot: PCRE2_SPTR16 = (*mb).name_table.offset(
                        (*(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            * (*mb).name_entry_size as ::core::ffi::c_int)
                            as isize,
                    );
                    (*F).ecode = (*F).ecode.offset(
                        (1 as ::core::ffi::c_int
                            + 2 as ::core::ffi::c_int * IMM2_SIZE
                            + (if (*F).op as ::core::ffi::c_int == OP_DNREFI as ::core::ffi::c_int {
                                1 as ::core::ffi::c_int
                            } else {
                                0 as ::core::ffi::c_int
                            })) as isize,
                    );
                    loop {
                        let fresh185 = count;
                        count = count - 1;
                        if !(fresh185 > 0 as ::core::ffi::c_int) {
                            break;
                        }
                        (*F).fields.ref_repeat.offset =
                            (((*slot.offset(0 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int)
                                << 1 as ::core::ffi::c_int)
                                - 2 as ::core::ffi::c_int) as size_t;
                        if (*F).fields.ref_repeat.offset < (*F).offset_top
                            && (*F).ovector[(*F).fields.ref_repeat.offset as usize] != PCRE2_UNSET
                        {
                            break;
                        }
                        slot = slot.offset((*mb).name_entry_size as ::core::ffi::c_int as isize);
                    }
                    current_block = 4095557813697852400;
                }
                114 | 115 => {
                    (*F).byte1 = ((*F).op as ::core::ffi::c_int == OP_REFI as ::core::ffi::c_int)
                        as ::core::ffi::c_int as uint8_t;
                    (*F).byte2 = (if (*F).op as ::core::ffi::c_int == OP_REFI as ::core::ffi::c_int
                    {
                        *(*F)
                            .ecode
                            .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize)
                            as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    }) as uint8_t;
                    (*F).fields.ref_repeat.offset =
                        (((*(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int)
                            << 1 as ::core::ffi::c_int)
                            - 2 as ::core::ffi::c_int) as size_t;
                    (*F).ecode = (*F).ecode.offset(
                        (1 as ::core::ffi::c_int
                            + IMM2_SIZE
                            + (if (*F).op as ::core::ffi::c_int == OP_REFI as ::core::ffi::c_int {
                                1 as ::core::ffi::c_int
                            } else {
                                0 as ::core::ffi::c_int
                            })) as isize,
                    );
                    current_block = 4095557813697852400;
                }
                153 => {
                    next_ecode = ::core::ptr::null::<PCRE2_UCHAR16>();
                    (*F).ecode = (*F).ecode.offset(1);
                    start_ecode = (*F).ecode;
                    (*F).return_id = RM9 as ::core::ffi::c_int as uint8_t;
                    current_block = 6380077529129020506;
                    break;
                }
                154 => {
                    next_ecode_0 = ::core::ptr::null::<PCRE2_UCHAR16>();
                    (*F).ecode = (*F).ecode.offset(1);
                    next_ecode_0 = (*F).ecode;
                    loop {
                        next_ecode_0 = next_ecode_0
                            .offset(*next_ecode_0.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int as isize);
                        if !(*next_ecode_0 as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                            break;
                        }
                    }
                    start_ecode = next_ecode_0
                        .offset(1 as ::core::ffi::c_int as isize)
                        .offset(1 as ::core::ffi::c_int as isize);
                    (*F).return_id = RM10 as ::core::ffi::c_int as uint8_t;
                    current_block = 6380077529129020506;
                    break;
                }
                169 => {
                    let mut next_ecode_1: PCRE2_SPTR16 =
                        (*F).ecode.offset(1 as ::core::ffi::c_int as isize);
                    loop {
                        next_ecode_1 = next_ecode_1
                            .offset(*next_ecode_1.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int as isize);
                        if !(*next_ecode_1 as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                            break;
                        }
                    }
                    (*F).ecode = next_ecode_1
                        .offset(1 as ::core::ffi::c_int as isize)
                        .offset(LINK_SIZE as isize);
                    continue;
                }
                155 => {
                    (*F).byte2 = TRUE as uint8_t;
                    (*F).ecode = (*F).ecode.offset(1 as ::core::ffi::c_int as isize);
                    if *(*F).ecode as ::core::ffi::c_int == OP_CBRAPOS as ::core::ffi::c_int
                        || *(*F).ecode as ::core::ffi::c_int == OP_SCBRAPOS as ::core::ffi::c_int
                    {
                        current_block = 8535700574364478148;
                        break;
                    } else {
                        current_block = 12426467910555810903;
                        break;
                    }
                }
                138 | 143 => {
                    (*F).byte2 = FALSE as uint8_t;
                    current_block = 12426467910555810903;
                    break;
                }
                140 | 145 => {
                    (*F).byte2 = FALSE as uint8_t;
                    current_block = 8535700574364478148;
                    break;
                }
                137 => {
                    if (*mb).hasthen != 0 || (*F).rdepth == 0 as uint32_t {
                        (*F).fields.op_bra.frame_type = 0 as uint32_t;
                        current_block = 5585757815311561815;
                        break;
                    } else {
                        current_block = 2089970334650350822;
                    }
                }
                139 | 144 => {
                    (*F).fields.op_bra.frame_type =
                        (GF_CAPTURE
                            | *(*F).ecode.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_uint) as uint32_t;
                    current_block = 5585757815311561815;
                    break;
                }
                135 | 136 | 142 => {
                    (*F).fields.op_bra.frame_type = GF_NOCAPTURE as uint32_t;
                    current_block = 5585757815311561815;
                    break;
                }
                118 => {
                    bracode = (*mb)
                        .start_code
                        .offset(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize);
                    number = (if bracode == (*mb).start_code {
                        0 as ::core::ffi::c_int
                    } else {
                        *bracode
                            .offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int
                    }) as uint32_t;
                    if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                        offset = (*F).last_group_offset;
                        while offset != PCRE2_UNSET {
                            N = ((*match_data).heapframes as *mut ::core::ffi::c_char)
                                .offset(offset as isize)
                                as *mut heapframe;
                            P = (N as *mut ::core::ffi::c_char).offset(-(frame_size as isize))
                                as *mut heapframe;
                            if (*N).group_frame_type == GF_RECURSE as uint32_t | number {
                                if (*F).eptr == (*P).eptr
                                    && (*mb).last_used_ptr == (*P).recurse_last_used
                                    && (*mb).moptions & PCRE2_DISABLE_RECURSELOOP_CHECK as uint32_t
                                        == 0 as uint32_t
                                {
                                    return PCRE2_ERROR_RECURSELOOP;
                                }
                                break;
                            } else {
                                offset = (*P).last_group_offset;
                            }
                        }
                    }
                    (*F).recurse_last_used = (*mb).last_used_ptr;
                    (*F).fields.op_recurse.start_branch = bracode;
                    (*F).fields.op_recurse.frame_type = GF_RECURSE as uint32_t | number;
                    current_block = 6367621710937907790;
                    break;
                }
                128 | 130 | 132 | 133 => {
                    current_block = 14401907612653336132;
                    break;
                }
                129 | 131 => {
                    current_block = 17451096827168202177;
                    break;
                }
                134 => {
                    length = 0 as size_t;
                    ecode = (*F)
                        .ecode
                        .offset(1 as ::core::ffi::c_int as isize)
                        .offset(LINK_SIZE as isize);
                    count_0 = 0;
                    slot_0 = ::core::ptr::null::<PCRE2_UCHAR16>();
                    offset = 0 as size_t;
                    loop {
                        if *ecode as ::core::ffi::c_int == OP_CREF as ::core::ffi::c_int {
                            length = length
                                .wrapping_add((1 as ::core::ffi::c_int + IMM2_SIZE) as size_t);
                            offset = (((*ecode.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int)
                                << 1 as ::core::ffi::c_int)
                                - 2 as ::core::ffi::c_int)
                                as size_t;
                            ecode = ecode.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                            if offset < (*F).offset_top
                                && (*F).ovector[offset as usize] != PCRE2_UNSET
                            {
                                current_block = 1164351245162749421;
                                break 's_263;
                            }
                        } else if *ecode as ::core::ffi::c_int != OP_DNCREF as ::core::ffi::c_int {
                            rrc = 0 as ::core::ffi::c_int;
                            break;
                        } else {
                            count_0 = *ecode.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            ) as ::core::ffi::c_int;
                            slot_0 = (*mb).name_table.offset(
                                (*ecode.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int
                                    * (*mb).name_entry_size as ::core::ffi::c_int)
                                    as isize,
                            );
                            length = length.wrapping_add(
                                (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                    as size_t,
                            );
                            ecode = ecode.offset(
                                (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                    as isize,
                            );
                            while count_0 > 0 as ::core::ffi::c_int {
                                offset = (((*slot_0.offset(0 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int)
                                    << 1 as ::core::ffi::c_int)
                                    - 2 as ::core::ffi::c_int)
                                    as size_t;
                                if offset < (*F).offset_top
                                    && (*F).ovector[offset as usize] != PCRE2_UNSET
                                {
                                    current_block = 1164351245162749421;
                                    break 's_263;
                                }
                                slot_0 = slot_0
                                    .offset((*mb).name_entry_size as ::core::ffi::c_int as isize);
                                count_0 -= 1;
                            }
                        }
                    }
                    current_block = 9215777819543815842;
                }
                119 | 120 => {
                    rrc = do_callout(F, mb, &raw mut length);
                    if rrc > 0 as ::core::ffi::c_int {
                        rrc = 0 as ::core::ffi::c_int;
                    } else if rrc < 0 as ::core::ffi::c_int {
                        rrc = rrc;
                    } else {
                        (*F).ecode = (*F).ecode.offset(length as isize);
                        continue;
                    }
                    current_block = 9215777819543815842;
                }
                141 | 146 => {
                    (*F).fields.op_cond.length =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as size_t;
                    if *(*F).ecode.offset((*F).fields.op_cond.length as isize) as ::core::ffi::c_int
                        != OP_ALT as ::core::ffi::c_int
                    {
                        (*F).fields.op_cond.length = (*F)
                            .fields
                            .op_cond
                            .length
                            .wrapping_sub((1 as ::core::ffi::c_int + LINK_SIZE) as size_t);
                    }
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                    if *(*F).ecode as ::core::ffi::c_int == OP_CALLOUT as ::core::ffi::c_int
                        || *(*F).ecode as ::core::ffi::c_int == OP_CALLOUT_STR as ::core::ffi::c_int
                    {
                        rrc = do_callout(F, mb, &raw mut length);
                        if rrc > 0 as ::core::ffi::c_int {
                            rrc = 0 as ::core::ffi::c_int;
                            current_block = 9215777819543815842;
                        } else if rrc < 0 as ::core::ffi::c_int {
                            rrc = rrc;
                            current_block = 9215777819543815842;
                        } else {
                            (*F).ecode = (*F).ecode.offset(length as isize);
                            (*F).fields.op_cond.length =
                                (*F).fields.op_cond.length.wrapping_sub(length);
                            current_block = 7591953283214378693;
                        }
                    } else {
                        current_block = 7591953283214378693;
                    }
                    match current_block {
                        9215777819543815842 => {}
                        _ => {
                            condition = FALSE as BOOL;
                            match *(*F).ecode as ::core::ffi::c_int {
                                149 => {
                                    current_block = 1513995137048850195;
                                    match current_block {
                                        16203646679947228034 => {
                                            condition = TRUE as BOOL;
                                        }
                                        10760133070394170635 => {
                                            (*F).byte1 = (*(*F).ecode as ::core::ffi::c_int
                                                == OP_ASSERT as ::core::ffi::c_int
                                                || *(*F).ecode as ::core::ffi::c_int
                                                    == OP_ASSERTBACK as ::core::ffi::c_int)
                                                as ::core::ffi::c_int
                                                as uint8_t;
                                            (*F).fields.op_cond.start_branch = (*F).ecode;
                                            current_block = 7298563137461813030;
                                            break;
                                        }
                                        1513995137048850195 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                number = *(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t;
                                                condition = (number == RREF_ANY as uint32_t
                                                    || number == (*F).current_recurse)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                        }
                                        17872262971949738442 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                let mut count_1: ::core::ffi::c_int =
                                                    *(*F).ecode.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int;
                                                let mut slot_1: PCRE2_SPTR16 =
                                                    (*mb).name_table.offset(
                                                        (*(*F).ecode.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            * (*mb).name_entry_size
                                                                as ::core::ffi::c_int)
                                                            as isize,
                                                    );
                                                loop {
                                                    let fresh188 = count_1;
                                                    count_1 = count_1 - 1;
                                                    if !(fresh188 > 0 as ::core::ffi::c_int) {
                                                        break;
                                                    }
                                                    number = *slot_1
                                                        .offset(0 as ::core::ffi::c_int as isize)
                                                        as uint32_t;
                                                    condition = (number == (*F).current_recurse)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                    if condition != 0 {
                                                        break;
                                                    }
                                                    slot_1 = slot_1.offset(
                                                        (*mb).name_entry_size as ::core::ffi::c_int
                                                            as isize,
                                                    );
                                                }
                                            }
                                        }
                                        12792210781472814257 => {
                                            let mut count_2: ::core::ffi::c_int =
                                                *(*F).ecode.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int;
                                            let mut slot_2: PCRE2_SPTR16 = (*mb).name_table.offset(
                                                (*(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int
                                                    * (*mb).name_entry_size as ::core::ffi::c_int)
                                                    as isize,
                                            );
                                            loop {
                                                let fresh189 = count_2;
                                                count_2 = count_2 - 1;
                                                if !(fresh189 > 0 as ::core::ffi::c_int) {
                                                    break;
                                                }
                                                offset = (((*slot_2
                                                    .offset(0 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 1 as ::core::ffi::c_int)
                                                    - 2 as ::core::ffi::c_int)
                                                    as size_t;
                                                condition = (offset < (*F).offset_top
                                                    && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                                if condition != 0 {
                                                    break;
                                                }
                                                slot_2 = slot_2.offset(
                                                    (*mb).name_entry_size as ::core::ffi::c_int
                                                        as isize,
                                                );
                                            }
                                        }
                                        _ => {
                                            offset = (((*(*F)
                                                .ecode
                                                .offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 1 as ::core::ffi::c_int)
                                                - 2 as ::core::ffi::c_int)
                                                as size_t;
                                            condition = (offset < (*F).offset_top
                                                && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                as ::core::ffi::c_int
                                                as BOOL;
                                        }
                                    }
                                    current_block = 16853744556532821084;
                                }
                                150 => {
                                    current_block = 17872262971949738442;
                                    match current_block {
                                        16203646679947228034 => {
                                            condition = TRUE as BOOL;
                                        }
                                        10760133070394170635 => {
                                            (*F).byte1 = (*(*F).ecode as ::core::ffi::c_int
                                                == OP_ASSERT as ::core::ffi::c_int
                                                || *(*F).ecode as ::core::ffi::c_int
                                                    == OP_ASSERTBACK as ::core::ffi::c_int)
                                                as ::core::ffi::c_int
                                                as uint8_t;
                                            (*F).fields.op_cond.start_branch = (*F).ecode;
                                            current_block = 7298563137461813030;
                                            break;
                                        }
                                        1513995137048850195 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                number = *(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t;
                                                condition = (number == RREF_ANY as uint32_t
                                                    || number == (*F).current_recurse)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                        }
                                        17872262971949738442 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                let mut count_1: ::core::ffi::c_int =
                                                    *(*F).ecode.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int;
                                                let mut slot_1: PCRE2_SPTR16 =
                                                    (*mb).name_table.offset(
                                                        (*(*F).ecode.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            * (*mb).name_entry_size
                                                                as ::core::ffi::c_int)
                                                            as isize,
                                                    );
                                                loop {
                                                    let fresh188 = count_1;
                                                    count_1 = count_1 - 1;
                                                    if !(fresh188 > 0 as ::core::ffi::c_int) {
                                                        break;
                                                    }
                                                    number = *slot_1
                                                        .offset(0 as ::core::ffi::c_int as isize)
                                                        as uint32_t;
                                                    condition = (number == (*F).current_recurse)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                    if condition != 0 {
                                                        break;
                                                    }
                                                    slot_1 = slot_1.offset(
                                                        (*mb).name_entry_size as ::core::ffi::c_int
                                                            as isize,
                                                    );
                                                }
                                            }
                                        }
                                        12792210781472814257 => {
                                            let mut count_2: ::core::ffi::c_int =
                                                *(*F).ecode.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int;
                                            let mut slot_2: PCRE2_SPTR16 = (*mb).name_table.offset(
                                                (*(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int
                                                    * (*mb).name_entry_size as ::core::ffi::c_int)
                                                    as isize,
                                            );
                                            loop {
                                                let fresh189 = count_2;
                                                count_2 = count_2 - 1;
                                                if !(fresh189 > 0 as ::core::ffi::c_int) {
                                                    break;
                                                }
                                                offset = (((*slot_2
                                                    .offset(0 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 1 as ::core::ffi::c_int)
                                                    - 2 as ::core::ffi::c_int)
                                                    as size_t;
                                                condition = (offset < (*F).offset_top
                                                    && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                                if condition != 0 {
                                                    break;
                                                }
                                                slot_2 = slot_2.offset(
                                                    (*mb).name_entry_size as ::core::ffi::c_int
                                                        as isize,
                                                );
                                            }
                                        }
                                        _ => {
                                            offset = (((*(*F)
                                                .ecode
                                                .offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 1 as ::core::ffi::c_int)
                                                - 2 as ::core::ffi::c_int)
                                                as size_t;
                                            condition = (offset < (*F).offset_top
                                                && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                as ::core::ffi::c_int
                                                as BOOL;
                                        }
                                    }
                                    current_block = 16853744556532821084;
                                }
                                147 => {
                                    current_block = 587451213255246846;
                                    match current_block {
                                        16203646679947228034 => {
                                            condition = TRUE as BOOL;
                                        }
                                        10760133070394170635 => {
                                            (*F).byte1 = (*(*F).ecode as ::core::ffi::c_int
                                                == OP_ASSERT as ::core::ffi::c_int
                                                || *(*F).ecode as ::core::ffi::c_int
                                                    == OP_ASSERTBACK as ::core::ffi::c_int)
                                                as ::core::ffi::c_int
                                                as uint8_t;
                                            (*F).fields.op_cond.start_branch = (*F).ecode;
                                            current_block = 7298563137461813030;
                                            break;
                                        }
                                        1513995137048850195 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                number = *(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t;
                                                condition = (number == RREF_ANY as uint32_t
                                                    || number == (*F).current_recurse)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                        }
                                        17872262971949738442 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                let mut count_1: ::core::ffi::c_int =
                                                    *(*F).ecode.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int;
                                                let mut slot_1: PCRE2_SPTR16 =
                                                    (*mb).name_table.offset(
                                                        (*(*F).ecode.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            * (*mb).name_entry_size
                                                                as ::core::ffi::c_int)
                                                            as isize,
                                                    );
                                                loop {
                                                    let fresh188 = count_1;
                                                    count_1 = count_1 - 1;
                                                    if !(fresh188 > 0 as ::core::ffi::c_int) {
                                                        break;
                                                    }
                                                    number = *slot_1
                                                        .offset(0 as ::core::ffi::c_int as isize)
                                                        as uint32_t;
                                                    condition = (number == (*F).current_recurse)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                    if condition != 0 {
                                                        break;
                                                    }
                                                    slot_1 = slot_1.offset(
                                                        (*mb).name_entry_size as ::core::ffi::c_int
                                                            as isize,
                                                    );
                                                }
                                            }
                                        }
                                        12792210781472814257 => {
                                            let mut count_2: ::core::ffi::c_int =
                                                *(*F).ecode.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int;
                                            let mut slot_2: PCRE2_SPTR16 = (*mb).name_table.offset(
                                                (*(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int
                                                    * (*mb).name_entry_size as ::core::ffi::c_int)
                                                    as isize,
                                            );
                                            loop {
                                                let fresh189 = count_2;
                                                count_2 = count_2 - 1;
                                                if !(fresh189 > 0 as ::core::ffi::c_int) {
                                                    break;
                                                }
                                                offset = (((*slot_2
                                                    .offset(0 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 1 as ::core::ffi::c_int)
                                                    - 2 as ::core::ffi::c_int)
                                                    as size_t;
                                                condition = (offset < (*F).offset_top
                                                    && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                                if condition != 0 {
                                                    break;
                                                }
                                                slot_2 = slot_2.offset(
                                                    (*mb).name_entry_size as ::core::ffi::c_int
                                                        as isize,
                                                );
                                            }
                                        }
                                        _ => {
                                            offset = (((*(*F)
                                                .ecode
                                                .offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 1 as ::core::ffi::c_int)
                                                - 2 as ::core::ffi::c_int)
                                                as size_t;
                                            condition = (offset < (*F).offset_top
                                                && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                as ::core::ffi::c_int
                                                as BOOL;
                                        }
                                    }
                                    current_block = 16853744556532821084;
                                }
                                148 => {
                                    current_block = 12792210781472814257;
                                    match current_block {
                                        16203646679947228034 => {
                                            condition = TRUE as BOOL;
                                        }
                                        10760133070394170635 => {
                                            (*F).byte1 = (*(*F).ecode as ::core::ffi::c_int
                                                == OP_ASSERT as ::core::ffi::c_int
                                                || *(*F).ecode as ::core::ffi::c_int
                                                    == OP_ASSERTBACK as ::core::ffi::c_int)
                                                as ::core::ffi::c_int
                                                as uint8_t;
                                            (*F).fields.op_cond.start_branch = (*F).ecode;
                                            current_block = 7298563137461813030;
                                            break;
                                        }
                                        1513995137048850195 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                number = *(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t;
                                                condition = (number == RREF_ANY as uint32_t
                                                    || number == (*F).current_recurse)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                        }
                                        17872262971949738442 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                let mut count_1: ::core::ffi::c_int =
                                                    *(*F).ecode.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int;
                                                let mut slot_1: PCRE2_SPTR16 =
                                                    (*mb).name_table.offset(
                                                        (*(*F).ecode.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            * (*mb).name_entry_size
                                                                as ::core::ffi::c_int)
                                                            as isize,
                                                    );
                                                loop {
                                                    let fresh188 = count_1;
                                                    count_1 = count_1 - 1;
                                                    if !(fresh188 > 0 as ::core::ffi::c_int) {
                                                        break;
                                                    }
                                                    number = *slot_1
                                                        .offset(0 as ::core::ffi::c_int as isize)
                                                        as uint32_t;
                                                    condition = (number == (*F).current_recurse)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                    if condition != 0 {
                                                        break;
                                                    }
                                                    slot_1 = slot_1.offset(
                                                        (*mb).name_entry_size as ::core::ffi::c_int
                                                            as isize,
                                                    );
                                                }
                                            }
                                        }
                                        12792210781472814257 => {
                                            let mut count_2: ::core::ffi::c_int =
                                                *(*F).ecode.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int;
                                            let mut slot_2: PCRE2_SPTR16 = (*mb).name_table.offset(
                                                (*(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int
                                                    * (*mb).name_entry_size as ::core::ffi::c_int)
                                                    as isize,
                                            );
                                            loop {
                                                let fresh189 = count_2;
                                                count_2 = count_2 - 1;
                                                if !(fresh189 > 0 as ::core::ffi::c_int) {
                                                    break;
                                                }
                                                offset = (((*slot_2
                                                    .offset(0 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 1 as ::core::ffi::c_int)
                                                    - 2 as ::core::ffi::c_int)
                                                    as size_t;
                                                condition = (offset < (*F).offset_top
                                                    && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                                if condition != 0 {
                                                    break;
                                                }
                                                slot_2 = slot_2.offset(
                                                    (*mb).name_entry_size as ::core::ffi::c_int
                                                        as isize,
                                                );
                                            }
                                        }
                                        _ => {
                                            offset = (((*(*F)
                                                .ecode
                                                .offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 1 as ::core::ffi::c_int)
                                                - 2 as ::core::ffi::c_int)
                                                as size_t;
                                            condition = (offset < (*F).offset_top
                                                && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                as ::core::ffi::c_int
                                                as BOOL;
                                        }
                                    }
                                    current_block = 16853744556532821084;
                                }
                                151 | 165 => {
                                    current_block = 16853744556532821084;
                                }
                                152 => {
                                    current_block = 16203646679947228034;
                                    match current_block {
                                        16203646679947228034 => {
                                            condition = TRUE as BOOL;
                                        }
                                        10760133070394170635 => {
                                            (*F).byte1 = (*(*F).ecode as ::core::ffi::c_int
                                                == OP_ASSERT as ::core::ffi::c_int
                                                || *(*F).ecode as ::core::ffi::c_int
                                                    == OP_ASSERTBACK as ::core::ffi::c_int)
                                                as ::core::ffi::c_int
                                                as uint8_t;
                                            (*F).fields.op_cond.start_branch = (*F).ecode;
                                            current_block = 7298563137461813030;
                                            break;
                                        }
                                        1513995137048850195 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                number = *(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t;
                                                condition = (number == RREF_ANY as uint32_t
                                                    || number == (*F).current_recurse)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                        }
                                        17872262971949738442 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                let mut count_1: ::core::ffi::c_int =
                                                    *(*F).ecode.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int;
                                                let mut slot_1: PCRE2_SPTR16 =
                                                    (*mb).name_table.offset(
                                                        (*(*F).ecode.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            * (*mb).name_entry_size
                                                                as ::core::ffi::c_int)
                                                            as isize,
                                                    );
                                                loop {
                                                    let fresh188 = count_1;
                                                    count_1 = count_1 - 1;
                                                    if !(fresh188 > 0 as ::core::ffi::c_int) {
                                                        break;
                                                    }
                                                    number = *slot_1
                                                        .offset(0 as ::core::ffi::c_int as isize)
                                                        as uint32_t;
                                                    condition = (number == (*F).current_recurse)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                    if condition != 0 {
                                                        break;
                                                    }
                                                    slot_1 = slot_1.offset(
                                                        (*mb).name_entry_size as ::core::ffi::c_int
                                                            as isize,
                                                    );
                                                }
                                            }
                                        }
                                        12792210781472814257 => {
                                            let mut count_2: ::core::ffi::c_int =
                                                *(*F).ecode.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int;
                                            let mut slot_2: PCRE2_SPTR16 = (*mb).name_table.offset(
                                                (*(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int
                                                    * (*mb).name_entry_size as ::core::ffi::c_int)
                                                    as isize,
                                            );
                                            loop {
                                                let fresh189 = count_2;
                                                count_2 = count_2 - 1;
                                                if !(fresh189 > 0 as ::core::ffi::c_int) {
                                                    break;
                                                }
                                                offset = (((*slot_2
                                                    .offset(0 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 1 as ::core::ffi::c_int)
                                                    - 2 as ::core::ffi::c_int)
                                                    as size_t;
                                                condition = (offset < (*F).offset_top
                                                    && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                                if condition != 0 {
                                                    break;
                                                }
                                                slot_2 = slot_2.offset(
                                                    (*mb).name_entry_size as ::core::ffi::c_int
                                                        as isize,
                                                );
                                            }
                                        }
                                        _ => {
                                            offset = (((*(*F)
                                                .ecode
                                                .offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 1 as ::core::ffi::c_int)
                                                - 2 as ::core::ffi::c_int)
                                                as size_t;
                                            condition = (offset < (*F).offset_top
                                                && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                as ::core::ffi::c_int
                                                as BOOL;
                                        }
                                    }
                                    current_block = 16853744556532821084;
                                }
                                _ => {
                                    current_block = 10760133070394170635;
                                    match current_block {
                                        16203646679947228034 => {
                                            condition = TRUE as BOOL;
                                        }
                                        10760133070394170635 => {
                                            (*F).byte1 = (*(*F).ecode as ::core::ffi::c_int
                                                == OP_ASSERT as ::core::ffi::c_int
                                                || *(*F).ecode as ::core::ffi::c_int
                                                    == OP_ASSERTBACK as ::core::ffi::c_int)
                                                as ::core::ffi::c_int
                                                as uint8_t;
                                            (*F).fields.op_cond.start_branch = (*F).ecode;
                                            current_block = 7298563137461813030;
                                            break;
                                        }
                                        1513995137048850195 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                number = *(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t;
                                                condition = (number == RREF_ANY as uint32_t
                                                    || number == (*F).current_recurse)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                        }
                                        17872262971949738442 => {
                                            if (*F).current_recurse != RECURSE_UNSET as uint32_t {
                                                let mut count_1: ::core::ffi::c_int =
                                                    *(*F).ecode.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int;
                                                let mut slot_1: PCRE2_SPTR16 =
                                                    (*mb).name_table.offset(
                                                        (*(*F).ecode.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            * (*mb).name_entry_size
                                                                as ::core::ffi::c_int)
                                                            as isize,
                                                    );
                                                loop {
                                                    let fresh188 = count_1;
                                                    count_1 = count_1 - 1;
                                                    if !(fresh188 > 0 as ::core::ffi::c_int) {
                                                        break;
                                                    }
                                                    number = *slot_1
                                                        .offset(0 as ::core::ffi::c_int as isize)
                                                        as uint32_t;
                                                    condition = (number == (*F).current_recurse)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                    if condition != 0 {
                                                        break;
                                                    }
                                                    slot_1 = slot_1.offset(
                                                        (*mb).name_entry_size as ::core::ffi::c_int
                                                            as isize,
                                                    );
                                                }
                                            }
                                        }
                                        12792210781472814257 => {
                                            let mut count_2: ::core::ffi::c_int =
                                                *(*F).ecode.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int;
                                            let mut slot_2: PCRE2_SPTR16 = (*mb).name_table.offset(
                                                (*(*F)
                                                    .ecode
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int
                                                    * (*mb).name_entry_size as ::core::ffi::c_int)
                                                    as isize,
                                            );
                                            loop {
                                                let fresh189 = count_2;
                                                count_2 = count_2 - 1;
                                                if !(fresh189 > 0 as ::core::ffi::c_int) {
                                                    break;
                                                }
                                                offset = (((*slot_2
                                                    .offset(0 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 1 as ::core::ffi::c_int)
                                                    - 2 as ::core::ffi::c_int)
                                                    as size_t;
                                                condition = (offset < (*F).offset_top
                                                    && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                                if condition != 0 {
                                                    break;
                                                }
                                                slot_2 = slot_2.offset(
                                                    (*mb).name_entry_size as ::core::ffi::c_int
                                                        as isize,
                                                );
                                            }
                                        }
                                        _ => {
                                            offset = (((*(*F)
                                                .ecode
                                                .offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 1 as ::core::ffi::c_int)
                                                - 2 as ::core::ffi::c_int)
                                                as size_t;
                                            condition = (offset < (*F).offset_top
                                                && (*F).ovector[offset as usize] != PCRE2_UNSET)
                                                as ::core::ffi::c_int
                                                as BOOL;
                                        }
                                    }
                                    current_block = 16853744556532821084;
                                }
                            }
                        }
                    }
                }
                126 => {
                    number = *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    if utf != 0 {
                        loop {
                            if !(number > 0 as uint32_t) {
                                current_block = 9277221749225084567;
                                break;
                            }
                            number = number.wrapping_sub(1);
                            if (*F).eptr <= (*mb).check_subject {
                                rrc = 0 as ::core::ffi::c_int;
                                current_block = 9215777819543815842;
                                break;
                            } else {
                                (*F).eptr = (*F).eptr.offset(-1);
                                if *(*F).eptr as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                                    == 0xdc00 as ::core::ffi::c_uint
                                {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                }
                            }
                        }
                    } else if number as ptrdiff_t
                        > (*F).eptr.offset_from((*mb).start_subject) as ptrdiff_t
                    {
                        rrc = 0 as ::core::ffi::c_int;
                        current_block = 9215777819543815842;
                    } else {
                        (*F).eptr = (*F).eptr.offset(-(number as isize));
                        current_block = 9277221749225084567;
                    }
                    match current_block {
                        9215777819543815842 => {}
                        _ => {
                            if (*F).eptr < (*mb).start_used_ptr {
                                (*mb).start_used_ptr = (*F).eptr;
                            }
                            (*F).ecode = (*F)
                                .ecode
                                .offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                            continue;
                        }
                    }
                }
                127 => {
                    (*F).fields.op_vreverse.min =
                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                    (*F).fields.op_vreverse.max = *(*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                        as uint32_t;
                    if utf != 0 {
                        i = 0 as uint32_t;
                        loop {
                            if !(i < (*F).fields.op_vreverse.max) {
                                current_block = 12225199092044798022;
                                break 's_263;
                            }
                            if (*F).eptr <= (*mb).check_subject {
                                if i < (*F).fields.op_vreverse.min {
                                    rrc = 0 as ::core::ffi::c_int;
                                    break;
                                } else {
                                    (*F).fields.op_vreverse.max = i;
                                    current_block = 12225199092044798022;
                                    break 's_263;
                                }
                            } else {
                                (*F).eptr = (*F).eptr.offset(-1);
                                if *(*F).eptr as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                                    == 0xdc00 as ::core::ffi::c_uint
                                {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                }
                                i = i.wrapping_add(1);
                            }
                        }
                    } else {
                        let mut diff: ptrdiff_t =
                            (*F).eptr.offset_from((*mb).start_subject) as ptrdiff_t;
                        let mut available: uint32_t =
                            (if diff > 65535 as ::core::ffi::c_int as ptrdiff_t {
                                65535 as ::core::ffi::c_int
                            } else if diff > 0 as ptrdiff_t {
                                diff as ::core::ffi::c_int
                            } else {
                                0 as ::core::ffi::c_int
                            }) as uint32_t;
                        if (*F).fields.op_vreverse.min > available {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            if (*F).fields.op_vreverse.max > available {
                                (*F).fields.op_vreverse.max = available;
                            }
                            (*F).eptr = (*F).eptr.offset(-((*F).fields.op_vreverse.max as isize));
                            current_block = 12225199092044798022;
                            break;
                        }
                    }
                    current_block = 9215777819543815842;
                }
                121 => {
                    branch_end = (*F).ecode;
                    loop {
                        (*F).ecode = (*F)
                            .ecode
                            .offset(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int as isize);
                        if !(*(*F).ecode as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                            break;
                        }
                    }
                    continue;
                }
                122 | 124 | 123 | 125 => {
                    bracode = (*F).ecode.offset(
                        -(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            as isize),
                    );
                    if branch_end.is_null() {
                        branch_end = (*F).ecode;
                    }
                    branch_start = bracode;
                    while branch_start
                        .offset(*branch_start.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize)
                        != branch_end
                    {
                        branch_start = branch_start
                            .offset(*branch_start.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int as isize);
                    }
                    branch_end = ::core::ptr::null::<PCRE2_UCHAR16>();
                    if *bracode as ::core::ffi::c_int != OP_BRA as ::core::ffi::c_int
                        && *bracode as ::core::ffi::c_int != OP_COND as ::core::ffi::c_int
                    {
                        N = ((*match_data).heapframes as *mut ::core::ffi::c_char)
                            .offset((*F).last_group_offset as isize)
                            as *mut heapframe;
                        P = (N as *mut ::core::ffi::c_char).offset(-(frame_size as isize))
                            as *mut heapframe;
                        (*F).last_group_offset = (*P).last_group_offset;
                        if (*N).group_frame_type == GF_CONDASSERT as uint32_t {
                            if (*bracode as ::core::ffi::c_int
                                == OP_ASSERTBACK as ::core::ffi::c_int
                                || *bracode as ::core::ffi::c_int
                                    == OP_ASSERTBACK_NOT as ::core::ffi::c_int)
                                && *branch_start
                                    .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize)
                                    as ::core::ffi::c_int
                                    == OP_VREVERSE as ::core::ffi::c_int
                                && (*F).eptr != (*P).eptr
                            {
                                rrc = 0 as ::core::ffi::c_int;
                            } else {
                                memcpy(
                                    (P as *mut ::core::ffi::c_char)
                                        .offset(120 as usize as isize)
                                        as *mut ::core::ffi::c_void,
                                    &raw mut (*F).ovector as *mut size_t
                                        as *const ::core::ffi::c_void,
                                    (*F).offset_top
                                        .wrapping_mul(::core::mem::size_of::<size_t>() as size_t),
                                );
                                (*P).offset_top = (*F).offset_top;
                                (*P).mark = (*F).mark;
                                (*F).back_frame = (F as *mut ::core::ffi::c_char)
                                    .offset_from(P as *mut ::core::ffi::c_char)
                                    as isize
                                    as size_t;
                                rrc = 1 as ::core::ffi::c_int;
                            }
                            current_block = 9215777819543815842;
                        } else {
                            current_block = 14210711077686619356;
                        }
                    } else {
                        P = ::core::ptr::null_mut::<heapframe>();
                        current_block = 14210711077686619356;
                    }
                    match current_block {
                        9215777819543815842 => {}
                        _ => {
                            match *bracode as ::core::ffi::c_int {
                                137 => {
                                    if (*F).current_recurse != 0 as uint32_t
                                        || *(*F)
                                            .ecode
                                            .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize)
                                            as ::core::ffi::c_int
                                            != OP_END as ::core::ffi::c_int
                                    {
                                        current_block = 403846265432066092;
                                    } else {
                                        offset = (*F).last_group_offset;
                                        if offset == PCRE2_UNSET {
                                            return PCRE2_ERROR_INTERNAL;
                                        }
                                        N = ((*match_data).heapframes as *mut ::core::ffi::c_char)
                                            .offset(offset as isize)
                                            as *mut heapframe;
                                        P = (N as *mut ::core::ffi::c_char)
                                            .offset(-(frame_size as isize))
                                            as *mut heapframe;
                                        (*F).last_group_offset = (*P).last_group_offset;
                                        (*F).ecode = (*P)
                                            .ecode
                                            .offset(1 as ::core::ffi::c_int as isize)
                                            .offset(LINK_SIZE as isize);
                                        if *(*F).ecode as ::core::ffi::c_int
                                            != OP_CREF as ::core::ffi::c_int
                                        {
                                            memcpy(
                                                &raw mut (*F).ovector as *mut size_t
                                                    as *mut ::core::ffi::c_void,
                                                &raw mut (*P).ovector as *mut size_t
                                                    as *const ::core::ffi::c_void,
                                                (*F).offset_top.wrapping_mul(
                                                    ::core::mem::size_of::<size_t>() as size_t,
                                                ),
                                            );
                                            (*F).offset_top = (*P).offset_top;
                                        } else {
                                            recurse_update_offsets(F, P);
                                        }
                                        (*F).capture_last = (*P).capture_last;
                                        (*F).current_recurse = (*P).current_recurse;
                                        continue;
                                    }
                                }
                                133 => {
                                    if *branch_start
                                        .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize)
                                        as ::core::ffi::c_int
                                        == OP_VREVERSE as ::core::ffi::c_int
                                        && (*F).eptr != (*P).eptr
                                    {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                    } else {
                                        current_block = 1109218150738618163;
                                    }
                                }
                                132 => {
                                    current_block = 1109218150738618163;
                                }
                                130 => {
                                    if *branch_start
                                        .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize)
                                        as ::core::ffi::c_int
                                        == OP_VREVERSE as ::core::ffi::c_int
                                        && (*F).eptr != (*P).eptr
                                    {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                    } else {
                                        current_block = 17014443214941663689;
                                    }
                                }
                                128 => {
                                    current_block = 17014443214941663689;
                                }
                                135 => {
                                    current_block = 4882717807625051864;
                                }
                                131 => {
                                    if *branch_start
                                        .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize)
                                        as ::core::ffi::c_int
                                        == OP_VREVERSE as ::core::ffi::c_int
                                        && (*F).eptr != (*P).eptr
                                    {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                    } else {
                                        current_block = 17085355968828643939;
                                    }
                                }
                                129 => {
                                    current_block = 17085355968828643939;
                                }
                                134 => {
                                    (*F).fields.op_assert_scs.saved_end_subject = (*mb).end_subject;
                                    (*mb).end_subject = (*P).fields.op_assert_scs.saved_end_subject;
                                    (*mb).true_end_subject = (*mb)
                                        .end_subject
                                        .offset((*P).fields.op_assert_scs.true_end_extra as isize);
                                    (*F).eptr = (*P).fields.op_assert_scs.saved_eptr;
                                    start_ecode = (*F)
                                        .ecode
                                        .offset(1 as ::core::ffi::c_int as isize)
                                        .offset(1 as ::core::ffi::c_int as isize);
                                    (*F).return_id = RM39 as ::core::ffi::c_int as uint8_t;
                                    current_block = 6380077529129020506;
                                    break;
                                }
                                136 => {
                                    if _pcre2_script_run_16((*P).eptr, (*F).eptr, utf) == 0 {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                    } else {
                                        current_block = 403846265432066092;
                                    }
                                }
                                139 | 140 | 144 | 145 => {
                                    number = *bracode.offset(
                                        (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                            as isize,
                                    ) as uint32_t;
                                    if (*F).current_recurse == number {
                                        P = (N as *mut ::core::ffi::c_char)
                                            .offset(-(frame_size as isize))
                                            as *mut heapframe;
                                        (*F).ecode = (*P)
                                            .ecode
                                            .offset(1 as ::core::ffi::c_int as isize)
                                            .offset(LINK_SIZE as isize);
                                        if *(*F).ecode as ::core::ffi::c_int
                                            != OP_CREF as ::core::ffi::c_int
                                        {
                                            memcpy(
                                                &raw mut (*F).ovector as *mut size_t
                                                    as *mut ::core::ffi::c_void,
                                                &raw mut (*P).ovector as *mut size_t
                                                    as *const ::core::ffi::c_void,
                                                (*F).offset_top.wrapping_mul(
                                                    ::core::mem::size_of::<size_t>() as size_t,
                                                ),
                                            );
                                            (*F).offset_top = (*P).offset_top;
                                        } else {
                                            recurse_update_offsets(F, P);
                                        }
                                        (*F).capture_last = (*P).capture_last;
                                        (*F).current_recurse = (*P).current_recurse;
                                        continue;
                                    } else {
                                        offset = (number << 1 as ::core::ffi::c_int)
                                            .wrapping_sub(2 as uint32_t)
                                            as size_t;
                                        (*F).capture_last = number;
                                        (*F).ovector[offset as usize] =
                                            (*P).eptr.offset_from((*mb).start_subject)
                                                as isize
                                                as size_t;
                                        (*F).ovector[offset.wrapping_add(1 as size_t) as usize] =
                                            (*F).eptr.offset_from((*mb).start_subject)
                                                as isize
                                                as size_t;
                                        if offset >= (*F).offset_top {
                                            (*F).offset_top = offset.wrapping_add(2 as size_t);
                                        }
                                    }
                                    current_block = 403846265432066092;
                                }
                                141 | 146 | _ => {
                                    current_block = 403846265432066092;
                                }
                            }
                            match current_block {
                                9215777819543815842 => {}
                                _ => {
                                    match current_block {
                                        17014443214941663689 => {
                                            if (*F).eptr > (*mb).last_used_ptr {
                                                (*mb).last_used_ptr = (*F).eptr;
                                            }
                                            (*F).eptr = (*P).eptr;
                                            current_block = 4882717807625051864;
                                        }
                                        1109218150738618163 => {
                                            if (*F).eptr > (*mb).last_used_ptr {
                                                (*mb).last_used_ptr = (*F).eptr;
                                            }
                                            (*F).eptr = (*P).eptr;
                                            current_block = 403846265432066092;
                                        }
                                        17085355968828643939 => {
                                            rrc = 1 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                        }
                                        _ => {}
                                    }
                                    match current_block {
                                        9215777819543815842 => {}
                                        _ => {
                                            match current_block {
                                                4882717807625051864 => {
                                                    (*F).back_frame = (F
                                                        as *mut ::core::ffi::c_char)
                                                        .offset_from(P as *mut ::core::ffi::c_char)
                                                        as isize
                                                        as size_t;
                                                    loop {
                                                        let mut y: uint32_t = *(*P).ecode.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as uint32_t;
                                                        if *(*P).ecode.offset(y as isize)
                                                            as ::core::ffi::c_int
                                                            != OP_ALT as ::core::ffi::c_int
                                                        {
                                                            break;
                                                        }
                                                        (*P).ecode = (*P).ecode.offset(y as isize);
                                                    }
                                                }
                                                _ => {}
                                            }
                                            if *(*F).ecode as ::core::ffi::c_int
                                                == OP_KETRPOS as ::core::ffi::c_int
                                            {
                                                memcpy(
                                                    (P as *mut ::core::ffi::c_char)
                                                        .offset(64 as usize as isize)
                                                        as *mut ::core::ffi::c_void,
                                                    (F as *mut ::core::ffi::c_char)
                                                        .offset(64 as usize as isize)
                                                        as *const ::core::ffi::c_void,
                                                    frame_copy_size,
                                                );
                                                rrc = -(998 as ::core::ffi::c_int);
                                                current_block = 9215777819543815842;
                                            } else if (*F).op as ::core::ffi::c_int
                                                != OP_KET as ::core::ffi::c_int
                                                && (P.is_null() || (*F).eptr != (*P).eptr)
                                            {
                                                if (*F).op as ::core::ffi::c_int
                                                    == OP_KETRMIN as ::core::ffi::c_int
                                                {
                                                    current_block = 6543021350661184703;
                                                    break;
                                                } else {
                                                    current_block = 14416695743247835409;
                                                    break;
                                                }
                                            } else {
                                                current_block = 6619656240033923038;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                27 => {
                    if (*F).eptr != (*mb).start_subject
                        || (*mb).moptions & PCRE2_NOTBOL as uint32_t != 0 as uint32_t
                    {
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        (*F).ecode = (*F).ecode.offset(1);
                        continue;
                    }
                    current_block = 9215777819543815842;
                }
                1 => {
                    if (*F).eptr != (*mb).start_subject {
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        (*F).ecode = (*F).ecode.offset(1);
                        continue;
                    }
                    current_block = 9215777819543815842;
                }
                25 => {
                    if (*mb).moptions & PCRE2_NOTEOL as uint32_t != 0 as uint32_t {
                        rrc = 0 as ::core::ffi::c_int;
                        current_block = 9215777819543815842;
                    } else if (*mb).poptions & PCRE2_DOLLAR_ENDONLY as uint32_t == 0 as uint32_t {
                        current_block = 5295940959186410680;
                    } else {
                        current_block = 16603835156348877170;
                    }
                }
                24 => {
                    current_block = 16603835156348877170;
                }
                23 => {
                    current_block = 5295940959186410680;
                }
                28 => {
                    if (*mb).moptions & PCRE2_NOTBOL as uint32_t != 0 as uint32_t
                        && (*F).eptr == (*mb).start_subject
                    {
                        rrc = 0 as ::core::ffi::c_int;
                    } else if (*F).eptr != (*mb).start_subject
                        && ((*F).eptr == (*mb).end_subject
                            && (*mb).poptions & PCRE2_ALT_CIRCUMFLEX as uint32_t == 0 as uint32_t
                            || (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                ((*F).eptr > (*mb).start_subject
                                    && _pcre2_was_newline_16(
                                        (*F).eptr,
                                        (*mb).nltype,
                                        (*mb).start_subject,
                                        &raw mut (*mb).nllen,
                                        utf,
                                    ) != 0) as ::core::ffi::c_int
                            } else {
                                ((*F).eptr >= (*mb).start_subject.offset((*mb).nllen as isize)
                                    && *(*F).eptr.offset(-((*mb).nllen as isize))
                                        as ::core::ffi::c_int
                                        == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                            as ::core::ffi::c_int
                                    && ((*mb).nllen == 1 as uint32_t
                                        || *(*F)
                                            .eptr
                                            .offset(-((*mb).nllen as isize))
                                            .offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                                as ::core::ffi::c_int))
                                    as ::core::ffi::c_int
                            }) == 0)
                    {
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        (*F).ecode = (*F).ecode.offset(1);
                        continue;
                    }
                    current_block = 9215777819543815842;
                }
                26 => {
                    if (*F).eptr < (*mb).end_subject {
                        if if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                            ((*F).eptr < (*mb).end_subject
                                && _pcre2_is_newline_16(
                                    (*F).eptr,
                                    (*mb).nltype,
                                    (*mb).end_subject,
                                    &raw mut (*mb).nllen,
                                    utf,
                                ) != 0) as ::core::ffi::c_int
                        } else {
                            ((*F).eptr <= (*mb).end_subject.offset(-((*mb).nllen as isize))
                                && *(*F).eptr as ::core::ffi::c_int
                                    == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                        as ::core::ffi::c_int
                                && ((*mb).nllen == 1 as uint32_t
                                    || *(*F).eptr.offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                            as ::core::ffi::c_int))
                                as ::core::ffi::c_int
                        } == 0
                        {
                            if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                                && (*F).eptr.offset(1 as ::core::ffi::c_int as isize)
                                    >= (*mb).end_subject
                                && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                && (*mb).nllen == 2 as uint32_t
                                && *(*F).eptr as ::core::ffi::c_int
                                    == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                        as ::core::ffi::c_int
                            {
                                (*mb).hitend = TRUE as BOOL;
                                if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                    return PCRE2_ERROR_PARTIAL;
                                }
                            }
                            rrc = 0 as ::core::ffi::c_int;
                            current_block = 9215777819543815842;
                        } else {
                            current_block = 11395214845484831303;
                        }
                    } else if (*mb).moptions & PCRE2_NOTEOL as uint32_t != 0 as uint32_t {
                        rrc = 0 as ::core::ffi::c_int;
                        current_block = 9215777819543815842;
                    } else {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        current_block = 11395214845484831303;
                    }
                    match current_block {
                        9215777819543815842 => {}
                        _ => {
                            (*F).ecode = (*F).ecode.offset(1);
                            continue;
                        }
                    }
                }
                2 => {
                    if (*F).eptr != (*mb).start_subject.offset((*mb).start_offset as isize) {
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        (*F).ecode = (*F).ecode.offset(1);
                        continue;
                    }
                    current_block = 9215777819543815842;
                }
                3 => {
                    (*F).start_match = (*F).eptr;
                    (*F).ecode = (*F).ecode.offset(1);
                    continue;
                }
                4 | 5 | 171 | 172 => {
                    if (*F).eptr == (*mb).check_subject {
                        prev_is_word = FALSE as BOOL;
                    } else {
                        let mut lastptr: PCRE2_SPTR16 =
                            (*F).eptr.offset(-(1 as ::core::ffi::c_int as isize));
                        if utf != 0 {
                            if *lastptr as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                                == 0xdc00 as ::core::ffi::c_uint
                            {
                                lastptr = lastptr.offset(-1);
                            }
                            fc = *lastptr as uint32_t;
                            if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                    | *lastptr.offset(1 as ::core::ffi::c_int as isize)
                                        as uint32_t
                                        & 0x3ff as uint32_t)
                                    .wrapping_add(0x10000 as uint32_t);
                            }
                        } else {
                            fc = *lastptr as uint32_t;
                        }
                        if lastptr < (*mb).start_used_ptr {
                            (*mb).start_used_ptr = lastptr;
                        }
                        if (*F).op as ::core::ffi::c_int
                            == OP_UCP_WORD_BOUNDARY as ::core::ffi::c_int
                            || (*F).op as ::core::ffi::c_int
                                == OP_NOT_UCP_WORD_BOUNDARY as ::core::ffi::c_int
                        {
                            let mut chartype_6: ::core::ffi::c_int =
                                (*(&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
                                    *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                                        (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                            .offset(
                                                (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE)
                                                    as isize,
                                            )
                                            as ::core::ffi::c_int
                                            * UCD_BLOCK_SIZE
                                            + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                            as isize,
                                    ) as ::core::ffi::c_int
                                        as isize,
                                ))
                                .chartype as ::core::ffi::c_int;
                            let mut category_5: ::core::ffi::c_int =
                                *(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                    .offset(chartype_6 as isize)
                                    as ::core::ffi::c_int;
                            prev_is_word = (category_5 == ucp_L as ::core::ffi::c_int
                                || category_5 == ucp_N as ::core::ffi::c_int
                                || chartype_6 == ucp_Mn as ::core::ffi::c_int
                                || chartype_6 == ucp_Pc as ::core::ffi::c_int)
                                as ::core::ffi::c_int
                                as BOOL;
                        } else {
                            prev_is_word = (fc <= 255 as uint32_t
                                && *(*mb).ctypes.offset(fc as isize) as ::core::ffi::c_int
                                    & ctype_word
                                    != 0 as ::core::ffi::c_int)
                                as ::core::ffi::c_int
                                as BOOL;
                        }
                    }
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        cur_is_word = FALSE as BOOL;
                    } else {
                        let mut nextptr: PCRE2_SPTR16 =
                            (*F).eptr.offset(1 as ::core::ffi::c_int as isize);
                        if utf != 0 {
                            if nextptr < (*mb).end_subject
                                && *nextptr as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                                    == 0xdc00 as ::core::ffi::c_uint
                            {
                                nextptr = nextptr.offset(1);
                            }
                            fc = *(*F).eptr as uint32_t;
                            if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                    | *(*F).eptr.offset(1 as ::core::ffi::c_int as isize)
                                        as uint32_t
                                        & 0x3ff as uint32_t)
                                    .wrapping_add(0x10000 as uint32_t);
                            }
                        } else {
                            fc = *(*F).eptr as uint32_t;
                        }
                        if nextptr > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = nextptr;
                        }
                        if (*F).op as ::core::ffi::c_int
                            == OP_UCP_WORD_BOUNDARY as ::core::ffi::c_int
                            || (*F).op as ::core::ffi::c_int
                                == OP_NOT_UCP_WORD_BOUNDARY as ::core::ffi::c_int
                        {
                            let mut chartype_7: ::core::ffi::c_int =
                                (*(&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
                                    *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                                        (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                            .offset(
                                                (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE)
                                                    as isize,
                                            )
                                            as ::core::ffi::c_int
                                            * UCD_BLOCK_SIZE
                                            + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                            as isize,
                                    ) as ::core::ffi::c_int
                                        as isize,
                                ))
                                .chartype as ::core::ffi::c_int;
                            let mut category_6: ::core::ffi::c_int =
                                *(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                    .offset(chartype_7 as isize)
                                    as ::core::ffi::c_int;
                            cur_is_word = (category_6 == ucp_L as ::core::ffi::c_int
                                || category_6 == ucp_N as ::core::ffi::c_int
                                || chartype_7 == ucp_Mn as ::core::ffi::c_int
                                || chartype_7 == ucp_Pc as ::core::ffi::c_int)
                                as ::core::ffi::c_int
                                as BOOL;
                        } else {
                            cur_is_word = (fc <= 255 as uint32_t
                                && *(*mb).ctypes.offset(fc as isize) as ::core::ffi::c_int
                                    & ctype_word
                                    != 0 as ::core::ffi::c_int)
                                as ::core::ffi::c_int
                                as BOOL;
                        }
                    }
                    let fresh191 = (*F).ecode;
                    (*F).ecode = (*F).ecode.offset(1);
                    if !(if *fresh191 as ::core::ffi::c_int
                        == OP_WORD_BOUNDARY as ::core::ffi::c_int
                        || (*F).op as ::core::ffi::c_int
                            == OP_UCP_WORD_BOUNDARY as ::core::ffi::c_int
                    {
                        (cur_is_word == prev_is_word) as ::core::ffi::c_int
                    } else {
                        (cur_is_word != prev_is_word) as ::core::ffi::c_int
                    } != 0)
                    {
                        continue;
                    }
                    rrc = 0 as ::core::ffi::c_int;
                    current_block = 9215777819543815842;
                }
                156 => {
                    (*mb).nomatch_mark = (*F).ecode.offset(2 as ::core::ffi::c_int as isize);
                    (*F).mark = (*mb).nomatch_mark;
                    start_ecode = (*F)
                        .ecode
                        .offset(
                            *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                                .offset(*(*F).ecode as isize)
                                as ::core::ffi::c_int as isize,
                        )
                        .offset(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize);
                    (*F).return_id = RM12 as ::core::ffi::c_int as uint8_t;
                    current_block = 6380077529129020506;
                    break;
                }
                165 => {
                    rrc = 0 as ::core::ffi::c_int;
                    current_block = 9215777819543815842;
                }
                163 => {
                    start_ecode = (*F).ecode.offset(
                        *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                            .offset(*(*F).ecode as isize)
                            as ::core::ffi::c_int as isize,
                    );
                    (*F).return_id = RM13 as ::core::ffi::c_int as uint8_t;
                    current_block = 6380077529129020506;
                    break;
                }
                164 => {
                    (*mb).nomatch_mark = (*F).ecode.offset(2 as ::core::ffi::c_int as isize);
                    (*F).mark = (*mb).nomatch_mark;
                    start_ecode = (*F)
                        .ecode
                        .offset(
                            *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                                .offset(*(*F).ecode as isize)
                                as ::core::ffi::c_int as isize,
                        )
                        .offset(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize);
                    (*F).return_id = RM36 as ::core::ffi::c_int as uint8_t;
                    current_block = 6380077529129020506;
                    break;
                }
                157 => {
                    start_ecode = (*F).ecode.offset(
                        *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                            .offset(*(*F).ecode as isize)
                            as ::core::ffi::c_int as isize,
                    );
                    (*F).return_id = RM14 as ::core::ffi::c_int as uint8_t;
                    current_block = 6380077529129020506;
                    break;
                }
                158 => {
                    (*mb).nomatch_mark = (*F).ecode.offset(2 as ::core::ffi::c_int as isize);
                    (*F).mark = (*mb).nomatch_mark;
                    start_ecode = (*F)
                        .ecode
                        .offset(
                            *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                                .offset(*(*F).ecode as isize)
                                as ::core::ffi::c_int as isize,
                        )
                        .offset(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize);
                    (*F).return_id = RM15 as ::core::ffi::c_int as uint8_t;
                    current_block = 6380077529129020506;
                    break;
                }
                159 => {
                    start_ecode = (*F).ecode.offset(
                        *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                            .offset(*(*F).ecode as isize)
                            as ::core::ffi::c_int as isize,
                    );
                    (*F).return_id = RM16 as ::core::ffi::c_int as uint8_t;
                    current_block = 6380077529129020506;
                    break;
                }
                160 => {
                    (*mb).skip_arg_count = (*mb).skip_arg_count.wrapping_add(1);
                    if (*mb).skip_arg_count <= (*mb).ignore_skip_arg {
                        (*F).ecode = (*F).ecode.offset(
                            (*(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                                .offset(*(*F).ecode as isize)
                                as ::core::ffi::c_int
                                + *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int) as isize,
                        );
                        continue;
                    } else {
                        start_ecode = (*F)
                            .ecode
                            .offset(
                                *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                                    .offset(*(*F).ecode as isize)
                                    as ::core::ffi::c_int as isize,
                            )
                            .offset(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int as isize);
                        (*F).return_id = RM17 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break;
                    }
                }
                161 => {
                    start_ecode = (*F).ecode.offset(
                        *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                            .offset(*(*F).ecode as isize)
                            as ::core::ffi::c_int as isize,
                    );
                    (*F).return_id = RM18 as ::core::ffi::c_int as uint8_t;
                    current_block = 6380077529129020506;
                    break;
                }
                162 => {
                    (*mb).nomatch_mark = (*F).ecode.offset(2 as ::core::ffi::c_int as isize);
                    (*F).mark = (*mb).nomatch_mark;
                    start_ecode = (*F)
                        .ecode
                        .offset(
                            *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                                .offset(*(*F).ecode as isize)
                                as ::core::ffi::c_int as isize,
                        )
                        .offset(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int as isize);
                    (*F).return_id = RM19 as ::core::ffi::c_int as uint8_t;
                    current_block = 6380077529129020506;
                    break;
                }
                _ => return PCRE2_ERROR_INTERNAL,
            }
            match current_block {
                5359207940272634753 => {
                    if (*F).eptr == (*F).start_match
                        && ((*mb).moptions & PCRE2_NOTEMPTY as uint32_t != 0 as uint32_t
                            || (*mb).moptions & PCRE2_NOTEMPTY_ATSTART as uint32_t != 0 as uint32_t
                                && (*F).start_match
                                    == (*mb).start_subject.offset((*mb).start_offset as isize))
                    {
                        rrc = 0 as ::core::ffi::c_int;
                    } else if (*F).eptr < (*mb).end_subject
                        && ((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED as uint32_t
                            != 0 as uint32_t
                    {
                        if (*F).op as ::core::ffi::c_int == OP_END as ::core::ffi::c_int {
                            rrc = 0 as ::core::ffi::c_int;
                        } else {
                            return MATCH_NOMATCH;
                        }
                    } else {
                        if (*F).start_match
                            < (*mb).start_subject.offset((*mb).start_offset as isize)
                            || (*F).start_match > (*F).eptr
                        {
                            if (*mb).allowlookaroundbsk == 0 {
                                return PCRE2_ERROR_BAD_BACKSLASH_K;
                            }
                        }
                        (*mb).end_match_ptr = (*F).eptr;
                        (*mb).end_offset_top = (*F).offset_top;
                        (*mb).mark = (*F).mark;
                        if (*F).eptr > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = (*F).eptr;
                        }
                        (*match_data).ovector[0 as ::core::ffi::c_int as usize] = (*F)
                            .start_match
                            .offset_from((*mb).start_subject)
                            as isize
                            as size_t;
                        (*match_data).ovector[1 as ::core::ffi::c_int as usize] =
                            (*F).eptr.offset_from((*mb).start_subject) as isize
                                as size_t;
                        i = (2 as ::core::ffi::c_int
                            * (if top_bracket as ::core::ffi::c_int + 1 as ::core::ffi::c_int
                                > (*match_data).oveccount as ::core::ffi::c_int
                            {
                                (*match_data).oveccount as ::core::ffi::c_int
                            } else {
                                top_bracket as ::core::ffi::c_int + 1 as ::core::ffi::c_int
                            })) as uint32_t;
                        memcpy(
                            (&raw mut (*match_data).ovector as *mut size_t)
                                .offset(2 as ::core::ffi::c_int as isize)
                                as *mut ::core::ffi::c_void,
                            &raw mut (*F).ovector as *mut size_t as *const ::core::ffi::c_void,
                            (i.wrapping_sub(2 as uint32_t) as size_t)
                                .wrapping_mul(::core::mem::size_of::<size_t>() as size_t),
                        );
                        loop {
                            i = i.wrapping_sub(1);
                            if !(i as size_t >= (*F).offset_top.wrapping_add(2 as size_t)) {
                                break;
                            }
                            (*match_data).ovector[i as usize] = PCRE2_UNSET;
                        }
                        return MATCH_MATCH;
                    }
                    current_block = 9215777819543815842;
                }
                5295940959186410680 => {
                    if (*F).eptr < (*mb).true_end_subject
                        && ((if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                            ((*F).eptr < (*mb).end_subject
                                && _pcre2_is_newline_16(
                                    (*F).eptr,
                                    (*mb).nltype,
                                    (*mb).end_subject,
                                    &raw mut (*mb).nllen,
                                    utf,
                                ) != 0) as ::core::ffi::c_int
                        } else {
                            ((*F).eptr <= (*mb).end_subject.offset(-((*mb).nllen as isize))
                                && *(*F).eptr as ::core::ffi::c_int
                                    == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                        as ::core::ffi::c_int
                                && ((*mb).nllen == 1 as uint32_t
                                    || *(*F).eptr.offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                            as ::core::ffi::c_int))
                                as ::core::ffi::c_int
                        }) == 0
                            || (*F).eptr != (*mb).true_end_subject.offset(-((*mb).nllen as isize)))
                    {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && (*F).eptr.offset(1 as ::core::ffi::c_int as isize)
                                >= (*mb).end_subject
                            && (*mb).nltype == NLTYPE_FIXED as uint32_t
                            && (*mb).nllen == 2 as uint32_t
                            && *(*F).eptr as ::core::ffi::c_int
                                == (*mb).nl[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        (*F).ecode = (*F).ecode.offset(1);
                        continue;
                    }
                    current_block = 9215777819543815842;
                }
                16603835156348877170 => {
                    if (*F).eptr < (*mb).true_end_subject {
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        (*F).ecode = (*F).ecode.offset(1);
                        continue;
                    }
                    current_block = 9215777819543815842;
                }
                4095557813697852400 => {
                    match *(*F).ecode as ::core::ffi::c_int {
                        98 | 99 | 100 | 101 | 102 | 103 | 106 | 107 | 108 => {
                            let fresh186 = (*F).ecode;
                            (*F).ecode = (*F).ecode.offset(1);
                            fc = (*fresh186 as ::core::ffi::c_int - OP_CRSTAR as ::core::ffi::c_int)
                                as uint32_t;
                            (*F).fields.ref_repeat.min = rep_min[fc as usize];
                            (*F).fields.ref_repeat.max = rep_max[fc as usize];
                            reptype = rep_typ[fc as usize];
                            current_block = 11861467105761702699;
                        }
                        104 | 105 | 109 => {
                            (*F).fields.ref_repeat.min =
                                *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
                            (*F).fields.ref_repeat.max = *(*F).ecode.offset(
                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize,
                            ) as uint32_t;
                            reptype = rep_typ[(*(*F).ecode as ::core::ffi::c_int
                                - OP_CRSTAR as ::core::ffi::c_int)
                                as usize];
                            if (*F).fields.ref_repeat.max == 0 as uint32_t {
                                (*F).fields.ref_repeat.max = UINT32_MAX as uint32_t;
                            }
                            (*F).ecode = (*F).ecode.offset(
                                (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                    as isize,
                            );
                            current_block = 11861467105761702699;
                        }
                        _ => {
                            rrc = match_ref(
                                (*F).fields.ref_repeat.offset,
                                (*F).byte1 as BOOL,
                                (*F).byte2 as ::core::ffi::c_int,
                                F,
                                mb,
                                &raw mut length,
                            );
                            if rrc != 0 as ::core::ffi::c_int {
                                if rrc > 0 as ::core::ffi::c_int {
                                    (*F).eptr = (*mb).end_subject;
                                }
                                if (*F).eptr >= (*mb).end_subject {
                                    if (*mb).partial as ::core::ffi::c_int
                                        != 0 as ::core::ffi::c_int
                                        && ((*F).eptr > (*mb).start_used_ptr
                                            || (*mb).allowemptypartial != 0)
                                    {
                                        (*mb).hitend = TRUE as BOOL;
                                        if (*mb).partial as ::core::ffi::c_int
                                            > 1 as ::core::ffi::c_int
                                        {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                    }
                                }
                                rrc = 0 as ::core::ffi::c_int;
                            } else {
                                (*F).eptr = (*F).eptr.offset(length as isize);
                                continue;
                            }
                            current_block = 9215777819543815842;
                        }
                    }
                    match current_block {
                        9215777819543815842 => {}
                        _ => {
                            if (*F).fields.ref_repeat.offset < (*F).offset_top
                                && (*F).ovector[(*F).fields.ref_repeat.offset as usize]
                                    != PCRE2_UNSET
                            {
                                if (*F).ovector[(*F).fields.ref_repeat.offset as usize]
                                    == (*F).ovector[(*F)
                                        .fields
                                        .ref_repeat
                                        .offset
                                        .wrapping_add(1 as size_t)
                                        as usize]
                                {
                                    continue;
                                }
                            } else if (*F).fields.ref_repeat.min == 0 as uint32_t
                                || (*mb).poptions & PCRE2_MATCH_UNSET_BACKREF as uint32_t
                                    != 0 as uint32_t
                            {
                                continue;
                            }
                            i = 1 as uint32_t;
                            loop {
                                if !(i <= (*F).fields.ref_repeat.min) {
                                    current_block = 14435543192788210210;
                                    break;
                                }
                                let mut slength: size_t = 0;
                                rrc = match_ref(
                                    (*F).fields.ref_repeat.offset,
                                    (*F).byte1 as BOOL,
                                    (*F).byte2 as ::core::ffi::c_int,
                                    F,
                                    mb,
                                    &raw mut slength,
                                );
                                if rrc != 0 as ::core::ffi::c_int {
                                    if rrc > 0 as ::core::ffi::c_int {
                                        (*F).eptr = (*mb).end_subject;
                                    }
                                    if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                    }
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    break;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(slength as isize);
                                    i = i.wrapping_add(1);
                                }
                            }
                            match current_block {
                                9215777819543815842 => {}
                                _ => {
                                    if (*F).fields.ref_repeat.min == (*F).fields.ref_repeat.max {
                                        continue;
                                    }
                                    if reptype == REPTYPE_MIN as ::core::ffi::c_int as uint32_t {
                                        current_block = 12869037699417280666;
                                        break;
                                    }
                                    samelengths = TRUE;
                                    (*F).fields.ref_repeat.start = (*F).eptr;
                                    (*F).fields.ref_repeat.length = (*F).ovector[(*F)
                                        .fields
                                        .ref_repeat
                                        .offset
                                        .wrapping_add(1 as size_t)
                                        as usize]
                                        .wrapping_sub(
                                            (*F).ovector[(*F).fields.ref_repeat.offset as usize],
                                        );
                                    i = (*F).fields.ref_repeat.min;
                                    while i < (*F).fields.ref_repeat.max {
                                        let mut slength_1: size_t = 0;
                                        rrc = match_ref(
                                            (*F).fields.ref_repeat.offset,
                                            (*F).byte1 as BOOL,
                                            (*F).byte2 as ::core::ffi::c_int,
                                            F,
                                            mb,
                                            &raw mut slength_1,
                                        );
                                        if rrc != 0 as ::core::ffi::c_int {
                                            if rrc > 0 as ::core::ffi::c_int
                                                && (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                && (*mb).end_subject > (*mb).start_used_ptr
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            break;
                                        } else {
                                            if slength_1 != (*F).fields.ref_repeat.length {
                                                samelengths = FALSE as BOOL;
                                            }
                                            (*F).eptr = (*F).eptr.offset(slength_1 as isize);
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                    if reptype == REPTYPE_POS as ::core::ffi::c_int as uint32_t {
                                        continue;
                                    }
                                    if samelengths != 0 {
                                        current_block = 7653578622174939700;
                                    } else {
                                        (*F).fields.ref_repeat.max = i;
                                        current_block = 2202530641731595862;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                6634154589341350760 => {
                    let fresh90 = (*F).ecode;
                    (*F).ecode = (*F).ecode.offset(1);
                    (*F).fields.type_repeat.ctype = *fresh90 as uint32_t;
                    if (*F).fields.type_repeat.ctype == OP_PROP as ::core::ffi::c_int as uint32_t
                        || (*F).fields.type_repeat.ctype
                            == OP_NOTPROP as ::core::ffi::c_int as uint32_t
                    {
                        let fresh91 = (*F).ecode;
                        (*F).ecode = (*F).ecode.offset(1);
                        proptype = *fresh91 as ::core::ffi::c_int;
                        let fresh92 = (*F).ecode;
                        (*F).ecode = (*F).ecode.offset(1);
                        (*F).fields.type_repeat.propvalue = *fresh92 as uint32_t;
                    } else {
                        proptype = -(1 as ::core::ffi::c_int);
                    }
                    if (*F).fields.type_repeat.min > 0 as uint32_t {
                        if proptype >= 0 as ::core::ffi::c_int {
                            let mut notmatch_0: BOOL = ((*F).fields.type_repeat.ctype
                                == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                as ::core::ffi::c_int;
                            match proptype {
                                PT_LAMP => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        let mut chartype_0: ::core::ffi::c_int = 0;
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh93 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh93 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh94 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh94 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            chartype_0 = (*(&raw const _pcre2_ucd_records_16
                                                as *const ucd_record)
                                                .offset(
                                                    *(&raw const _pcre2_ucd_stage2_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_stage1_16
                                                                as *const uint16_t)
                                                                .offset(
                                                                    (fc as ::core::ffi::c_int
                                                                        / UCD_BLOCK_SIZE)
                                                                        as isize,
                                                                )
                                                                as ::core::ffi::c_int
                                                                * UCD_BLOCK_SIZE
                                                                + fc as ::core::ffi::c_int
                                                                    % UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        as isize,
                                                ))
                                            .chartype
                                                as ::core::ffi::c_int;
                                            if (chartype_0 == ucp_Lu as ::core::ffi::c_int
                                                || chartype_0 == ucp_Ll as ::core::ffi::c_int
                                                || chartype_0 == ucp_Lt as ::core::ffi::c_int)
                                                as ::core::ffi::c_int
                                                == notmatch_0
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                PT_GC => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh95 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh95 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh96 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh96 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            if (*(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                                .offset(
                                                    (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                        .offset(
                                                                            (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .chartype as isize,
                                                ) == (*F).fields.type_repeat.propvalue)
                                                as ::core::ffi::c_int == notmatch_0
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                PT_PC => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh97 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh97 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh98 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh98 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            if ((*(&raw const _pcre2_ucd_records_16
                                                as *const ucd_record)
                                                .offset(
                                                    *(&raw const _pcre2_ucd_stage2_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_stage1_16
                                                                as *const uint16_t)
                                                                .offset(
                                                                    (fc as ::core::ffi::c_int
                                                                        / UCD_BLOCK_SIZE)
                                                                        as isize,
                                                                )
                                                                as ::core::ffi::c_int
                                                                * UCD_BLOCK_SIZE
                                                                + fc as ::core::ffi::c_int
                                                                    % UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        as isize,
                                                ))
                                            .chartype
                                                as uint32_t
                                                == (*F).fields.type_repeat.propvalue)
                                                as ::core::ffi::c_int
                                                == notmatch_0
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                PT_SC => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh99 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh99 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh100 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh100 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            if ((*(&raw const _pcre2_ucd_records_16
                                                as *const ucd_record)
                                                .offset(
                                                    *(&raw const _pcre2_ucd_stage2_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_stage1_16
                                                                as *const uint16_t)
                                                                .offset(
                                                                    (fc as ::core::ffi::c_int
                                                                        / UCD_BLOCK_SIZE)
                                                                        as isize,
                                                                )
                                                                as ::core::ffi::c_int
                                                                * UCD_BLOCK_SIZE
                                                                + fc as ::core::ffi::c_int
                                                                    % UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        as isize,
                                                ))
                                            .script
                                                as uint32_t
                                                == (*F).fields.type_repeat.propvalue)
                                                as ::core::ffi::c_int
                                                == notmatch_0
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                PT_SCX => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        let mut ok_1: BOOL = 0;
                                        let mut prop_0: *const ucd_record =
                                            ::core::ptr::null::<ucd_record>();
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh101 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh101 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh102 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh102 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            prop_0 = (&raw const _pcre2_ucd_records_16
                                                as *const ucd_record)
                                                .offset(
                                                    *(&raw const _pcre2_ucd_stage2_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_stage1_16
                                                                as *const uint16_t)
                                                                .offset(
                                                                    (fc as ::core::ffi::c_int
                                                                        / UCD_BLOCK_SIZE)
                                                                        as isize,
                                                                )
                                                                as ::core::ffi::c_int
                                                                * UCD_BLOCK_SIZE
                                                                + fc as ::core::ffi::c_int
                                                                    % UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        as isize,
                                                );
                                            ok_1 = ((*prop_0).script as uint32_t
                                                == (*F).fields.type_repeat.propvalue
                                                || *(&raw const _pcre2_ucd_script_sets_16
                                                    as *const uint32_t)
                                                    .offset(
                                                        ((*prop_0).scriptx_bidiclass
                                                            as ::core::ffi::c_int
                                                            & 0x3ff as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                    .offset(
                                                        (*F).fields
                                                            .type_repeat
                                                            .propvalue
                                                            .wrapping_div(32 as uint32_t)
                                                            as isize,
                                                    )
                                                    & (1 as uint32_t)
                                                        << (*F)
                                                            .fields
                                                            .type_repeat
                                                            .propvalue
                                                            .wrapping_rem(32 as uint32_t)
                                                    != 0 as uint32_t)
                                                as ::core::ffi::c_int
                                                as BOOL;
                                            if ok_1 == notmatch_0 {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                PT_ALNUM => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        let mut category: ::core::ffi::c_int = 0;
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh103 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh103 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh104 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh104 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            category = *(&raw const _pcre2_ucp_gentype_16
                                                as *const uint32_t)
                                                .offset(
                                                    (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                        .offset(
                                                                            (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .chartype as isize,
                                                ) as ::core::ffi::c_int;
                                            if (category == ucp_L as ::core::ffi::c_int
                                                || category == ucp_N as ::core::ffi::c_int)
                                                as ::core::ffi::c_int
                                                == notmatch_0
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                PT_SPACE | PT_PXSPACE => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh105 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh105 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh106 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh106 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            match fc {
                                                9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194 | 8195
                                                | 8196 | 8197 | 8198 | 8199 | 8200 | 8201 | 8202 | 8239
                                                | 8287 | 12288 | 10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                                    if notmatch_0 != 0 {
                                                        rrc = 0 as ::core::ffi::c_int;
                                                        current_block = 9215777819543815842;
                                                        break;
                                                    }
                                                }
                                                _ => {
                                                    if (*(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                                                .offset(
                                                                    *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                        .offset(
                                                                            (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                                .offset(
                                                                                    (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                                ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                                + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int as isize,
                                                                ))
                                                                .chartype as isize,
                                                        ) == ucp_Z as ::core::ffi::c_int as uint32_t)
                                                        as ::core::ffi::c_int == notmatch_0
                                                    {
                                                        rrc = 0 as ::core::ffi::c_int;
                                                        current_block = 9215777819543815842;
                                                        break;
                                                    }
                                                }
                                            }
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                PT_WORD => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        let mut chartype_1: ::core::ffi::c_int = 0;
                                        let mut category_0: ::core::ffi::c_int = 0;
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh107 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh107 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh108 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh108 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            chartype_1 = (*(&raw const _pcre2_ucd_records_16
                                                as *const ucd_record)
                                                .offset(
                                                    *(&raw const _pcre2_ucd_stage2_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_stage1_16
                                                                as *const uint16_t)
                                                                .offset(
                                                                    (fc as ::core::ffi::c_int
                                                                        / UCD_BLOCK_SIZE)
                                                                        as isize,
                                                                )
                                                                as ::core::ffi::c_int
                                                                * UCD_BLOCK_SIZE
                                                                + fc as ::core::ffi::c_int
                                                                    % UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        as isize,
                                                ))
                                            .chartype
                                                as ::core::ffi::c_int;
                                            category_0 = *(&raw const _pcre2_ucp_gentype_16
                                                as *const uint32_t)
                                                .offset(chartype_1 as isize)
                                                as ::core::ffi::c_int;
                                            if (category_0 == ucp_L as ::core::ffi::c_int
                                                || category_0 == ucp_N as ::core::ffi::c_int
                                                || chartype_1 == ucp_Mn as ::core::ffi::c_int
                                                || chartype_1 == ucp_Pc as ::core::ffi::c_int)
                                                as ::core::ffi::c_int
                                                == notmatch_0
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                PT_CLIST => {
                                    i = 1 as uint32_t;
                                    's_9363: loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        let mut cp_0: *const uint32_t =
                                            ::core::ptr::null::<uint32_t>();
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh109 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh109 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh110 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh110 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            cp_0 = (&raw const _pcre2_ucd_caseless_sets_16
                                                as *const uint32_t)
                                                .offset((*F).fields.type_repeat.propvalue as isize);
                                            loop {
                                                if fc < *cp_0 {
                                                    if notmatch_0 != 0 {
                                                        break;
                                                    }
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break 's_9363;
                                                } else {
                                                    let fresh111 = cp_0;
                                                    cp_0 = cp_0.offset(1);
                                                    if !(fc == *fresh111) {
                                                        continue;
                                                    }
                                                    if !(notmatch_0 != 0) {
                                                        break;
                                                    }
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break 's_9363;
                                                }
                                            }
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                PT_UCNC => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh112 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh112 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh113 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh113 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            if (fc == CHAR_DOLLAR_SIGN as uint32_t
                                                || fc == CHAR_COMMERCIAL_AT as uint32_t
                                                || fc == CHAR_GRAVE_ACCENT as uint32_t
                                                || fc >= 0xa0 as uint32_t
                                                    && fc <= 0xd7ff as uint32_t
                                                || fc >= 0xe000 as uint32_t)
                                                as ::core::ffi::c_int
                                                == notmatch_0
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                PT_BIDICL => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh114 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh114 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh115 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh115 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            if (((*(&raw const _pcre2_ucd_records_16
                                                as *const ucd_record)
                                                .offset(
                                                    *(&raw const _pcre2_ucd_stage2_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_stage1_16
                                                                as *const uint16_t)
                                                                .offset(
                                                                    (fc as ::core::ffi::c_int
                                                                        / 128 as ::core::ffi::c_int)
                                                                        as isize,
                                                                )
                                                                as ::core::ffi::c_int
                                                                * 128 as ::core::ffi::c_int
                                                                + fc as ::core::ffi::c_int
                                                                    % 128 as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        as isize,
                                                ))
                                            .scriptx_bidiclass
                                                as ::core::ffi::c_int
                                                >> UCD_BIDICLASS_SHIFT)
                                                as uint32_t
                                                == (*F).fields.type_repeat.propvalue)
                                                as ::core::ffi::c_int
                                                == notmatch_0
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                PT_BOOL => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        let mut ok_2: BOOL = 0;
                                        let mut prop_1: *const ucd_record =
                                            ::core::ptr::null::<ucd_record>();
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh116 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh116 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh117 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh117 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            prop_1 = (&raw const _pcre2_ucd_records_16
                                                as *const ucd_record)
                                                .offset(
                                                    *(&raw const _pcre2_ucd_stage2_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_stage1_16
                                                                as *const uint16_t)
                                                                .offset(
                                                                    (fc as ::core::ffi::c_int
                                                                        / UCD_BLOCK_SIZE)
                                                                        as isize,
                                                                )
                                                                as ::core::ffi::c_int
                                                                * UCD_BLOCK_SIZE
                                                                + fc as ::core::ffi::c_int
                                                                    % UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        as isize,
                                                );
                                            ok_2 = (*(&raw const _pcre2_ucd_boolprop_sets_16
                                                as *const uint32_t)
                                                .offset(
                                                    ((*prop_1).bprops as ::core::ffi::c_int
                                                        & 0xfff as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                .offset(
                                                    (*F).fields
                                                        .type_repeat
                                                        .propvalue
                                                        .wrapping_div(32 as uint32_t)
                                                        as isize,
                                                )
                                                & (1 as uint32_t)
                                                    << (*F)
                                                        .fields
                                                        .type_repeat
                                                        .propvalue
                                                        .wrapping_rem(32 as uint32_t)
                                                != 0 as uint32_t)
                                                as ::core::ffi::c_int
                                                as BOOL;
                                            if ok_2 == notmatch_0 {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                _ => return PCRE2_ERROR_INTERNAL,
                            }
                        } else if (*F).fields.type_repeat.ctype
                            == OP_EXTUNI as ::core::ffi::c_int as uint32_t
                        {
                            i = 1 as uint32_t;
                            loop {
                                if !(i <= (*F).fields.type_repeat.min) {
                                    current_block = 10634842702853724451;
                                    break;
                                }
                                if (*F).eptr >= (*mb).end_subject {
                                    if (*mb).partial as ::core::ffi::c_int
                                        != 0 as ::core::ffi::c_int
                                        && ((*F).eptr > (*mb).start_used_ptr
                                            || (*mb).allowemptypartial != 0)
                                    {
                                        (*mb).hitend = TRUE as BOOL;
                                        if (*mb).partial as ::core::ffi::c_int
                                            > 1 as ::core::ffi::c_int
                                        {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                    }
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    break;
                                } else {
                                    let fresh118 = (*F).eptr;
                                    (*F).eptr = (*F).eptr.offset(1);
                                    fc = *fresh118 as uint32_t;
                                    if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                        let fresh119 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                            | *fresh119 as uint32_t & 0x3ff as uint32_t)
                                            .wrapping_add(0x10000 as uint32_t);
                                    }
                                    (*F).eptr = _pcre2_extuni_16(
                                        fc,
                                        (*F).eptr,
                                        (*mb).check_subject,
                                        (*mb).end_subject,
                                        utf,
                                        ::core::ptr::null_mut::<::core::ffi::c_int>(),
                                    );
                                    if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                    }
                                    i = i.wrapping_add(1);
                                }
                            }
                        } else if utf != 0 {
                            match (*F).fields.type_repeat.ctype {
                                12 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else if if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                            ((*F).eptr < (*mb).end_subject
                                                && _pcre2_is_newline_16(
                                                    (*F).eptr,
                                                    (*mb).nltype,
                                                    (*mb).end_subject,
                                                    &raw mut (*mb).nllen,
                                                    utf,
                                                ) != 0)
                                                as ::core::ffi::c_int
                                        } else {
                                            ((*F).eptr
                                                <= (*mb)
                                                    .end_subject
                                                    .offset(-((*mb).nllen as isize))
                                                && *(*F).eptr as ::core::ffi::c_int
                                                    == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                        as ::core::ffi::c_int
                                                && ((*mb).nllen == 1 as uint32_t
                                                    || *(*F)
                                                        .eptr
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                        == (*mb).nl
                                                            [1 as ::core::ffi::c_int as usize]
                                                            as ::core::ffi::c_int))
                                                as ::core::ffi::c_int
                                        } != 0
                                        {
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && (*F)
                                                    .eptr
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    >= (*mb).end_subject
                                                && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                                && (*mb).nllen == 2 as uint32_t
                                                && *(*F).eptr as ::core::ffi::c_int
                                                    == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                        as ::core::ffi::c_int
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            (*F).eptr = (*F).eptr.offset(1);
                                            if (*F).eptr < (*mb).end_subject
                                                && *(*F).eptr as ::core::ffi::c_uint
                                                    & 0xfc00 as ::core::ffi::c_uint
                                                    == 0xdc00 as ::core::ffi::c_uint
                                            {
                                                (*F).eptr = (*F).eptr.offset(1);
                                            }
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                13 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            (*F).eptr = (*F).eptr.offset(1);
                                            if (*F).eptr < (*mb).end_subject
                                                && *(*F).eptr as ::core::ffi::c_uint
                                                    & 0xfc00 as ::core::ffi::c_uint
                                                    == 0xdc00 as ::core::ffi::c_uint
                                            {
                                                (*F).eptr = (*F).eptr.offset(1);
                                            }
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                14 => {
                                    if (*F).eptr
                                        > (*mb)
                                            .end_subject
                                            .offset(-((*F).fields.type_repeat.min as isize))
                                    {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                    } else {
                                        (*F).eptr =
                                            (*F).eptr.offset((*F).fields.type_repeat.min as isize);
                                        current_block = 10634842702853724451;
                                    }
                                }
                                17 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh120 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh120 as uint32_t;
                                            if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                                let fresh121 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh121 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            match fc {
                                                13 => {
                                                    if (*F).eptr < (*mb).end_subject
                                                        && *(*F).eptr as ::core::ffi::c_int
                                                            == CHAR_LF
                                                    {
                                                        (*F).eptr = (*F).eptr.offset(1);
                                                    }
                                                }
                                                10 => {}
                                                11 | 12 | 133 | 8232 | 8233 => {
                                                    if (*mb).bsr_convention as ::core::ffi::c_int
                                                        == PCRE2_BSR_ANYCRLF
                                                    {
                                                        rrc = 0 as ::core::ffi::c_int;
                                                        current_block = 9215777819543815842;
                                                        break;
                                                    }
                                                }
                                                _ => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break;
                                                }
                                            }
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                18 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh122 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh122 as uint32_t;
                                            if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                                let fresh123 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh123 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            match fc {
                                                9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                                | 8195 | 8196 | 8197 | 8198 | 8199 | 8200
                                                | 8201 | 8202 | 8239 | 8287 | 12288 => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break;
                                                }
                                                _ => {
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                    }
                                }
                                19 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh124 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh124 as uint32_t;
                                            if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                                let fresh125 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh125 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            match fc {
                                                9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                                | 8195 | 8196 | 8197 | 8198 | 8199 | 8200
                                                | 8201 | 8202 | 8239 | 8287 | 12288 => {
                                                    i = i.wrapping_add(1);
                                                }
                                                _ => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                20 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh126 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh126 as uint32_t;
                                            if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                                let fresh127 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh127 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            match fc {
                                                10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break;
                                                }
                                                _ => {
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                    }
                                }
                                21 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh128 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh128 as uint32_t;
                                            if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                                let fresh129 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh129 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            match fc {
                                                10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                                    i = i.wrapping_add(1);
                                                }
                                                _ => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                6 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh130 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh130 as uint32_t;
                                            if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                                let fresh131 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh131 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            if fc < 128 as uint32_t
                                                && *(*mb).ctypes.offset(fc as isize)
                                                    as ::core::ffi::c_int
                                                    & ctype_digit
                                                    != 0 as ::core::ffi::c_int
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                7 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        let mut cc_4: uint32_t = 0;
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            cc_4 = *(*F).eptr as uint32_t;
                                            if cc_4 >= 128 as uint32_t
                                                || *(*mb).ctypes.offset(cc_4 as isize)
                                                    as ::core::ffi::c_int
                                                    & ctype_digit
                                                    == 0 as ::core::ffi::c_int
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                (*F).eptr = (*F).eptr.offset(1);
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                8 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        let mut cc_5: uint32_t = 0;
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            cc_5 = *(*F).eptr as uint32_t;
                                            if cc_5 < 128 as uint32_t
                                                && *(*mb).ctypes.offset(cc_5 as isize)
                                                    as ::core::ffi::c_int
                                                    & ctype_space
                                                    != 0 as ::core::ffi::c_int
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                (*F).eptr = (*F).eptr.offset(1);
                                                if (*F).eptr < (*mb).end_subject
                                                    && *(*F).eptr as ::core::ffi::c_uint
                                                        & 0xfc00 as ::core::ffi::c_uint
                                                        == 0xdc00 as ::core::ffi::c_uint
                                                {
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                }
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                9 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        let mut cc_6: uint32_t = 0;
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            cc_6 = *(*F).eptr as uint32_t;
                                            if cc_6 >= 128 as uint32_t
                                                || *(*mb).ctypes.offset(cc_6 as isize)
                                                    as ::core::ffi::c_int
                                                    & ctype_space
                                                    == 0 as ::core::ffi::c_int
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                (*F).eptr = (*F).eptr.offset(1);
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                10 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        let mut cc_7: uint32_t = 0;
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            cc_7 = *(*F).eptr as uint32_t;
                                            if cc_7 < 128 as uint32_t
                                                && *(*mb).ctypes.offset(cc_7 as isize)
                                                    as ::core::ffi::c_int
                                                    & ctype_word
                                                    != 0 as ::core::ffi::c_int
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                (*F).eptr = (*F).eptr.offset(1);
                                                if (*F).eptr < (*mb).end_subject
                                                    && *(*F).eptr as ::core::ffi::c_uint
                                                        & 0xfc00 as ::core::ffi::c_uint
                                                        == 0xdc00 as ::core::ffi::c_uint
                                                {
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                }
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                11 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        let mut cc_8: uint32_t = 0;
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            cc_8 = *(*F).eptr as uint32_t;
                                            if cc_8 >= 128 as uint32_t
                                                || *(*mb).ctypes.offset(cc_8 as isize)
                                                    as ::core::ffi::c_int
                                                    & ctype_word
                                                    == 0 as ::core::ffi::c_int
                                            {
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                break;
                                            } else {
                                                (*F).eptr = (*F).eptr.offset(1);
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                    }
                                }
                                _ => return PCRE2_ERROR_INTERNAL,
                            }
                        } else {
                            match (*F).fields.type_repeat.ctype {
                                12 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else if if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                            ((*F).eptr < (*mb).end_subject
                                                && _pcre2_is_newline_16(
                                                    (*F).eptr,
                                                    (*mb).nltype,
                                                    (*mb).end_subject,
                                                    &raw mut (*mb).nllen,
                                                    utf,
                                                ) != 0)
                                                as ::core::ffi::c_int
                                        } else {
                                            ((*F).eptr
                                                <= (*mb)
                                                    .end_subject
                                                    .offset(-((*mb).nllen as isize))
                                                && *(*F).eptr as ::core::ffi::c_int
                                                    == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                        as ::core::ffi::c_int
                                                && ((*mb).nllen == 1 as uint32_t
                                                    || *(*F)
                                                        .eptr
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                        == (*mb).nl
                                                            [1 as ::core::ffi::c_int as usize]
                                                            as ::core::ffi::c_int))
                                                as ::core::ffi::c_int
                                        } != 0
                                        {
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && (*F)
                                                    .eptr
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    >= (*mb).end_subject
                                                && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                                && (*mb).nllen == 2 as uint32_t
                                                && *(*F).eptr as ::core::ffi::c_int
                                                    == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                        as ::core::ffi::c_int
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            (*F).eptr = (*F).eptr.offset(1);
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                13 => {
                                    if (*F).eptr
                                        > (*mb)
                                            .end_subject
                                            .offset(-((*F).fields.type_repeat.min as isize))
                                    {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                    } else {
                                        (*F).eptr =
                                            (*F).eptr.offset((*F).fields.type_repeat.min as isize);
                                        current_block = 10634842702853724451;
                                    }
                                }
                                17 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh132 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            match *fresh132 as ::core::ffi::c_int {
                                                CHAR_CR => {
                                                    if (*F).eptr < (*mb).end_subject
                                                        && *(*F).eptr as ::core::ffi::c_int
                                                            == CHAR_LF
                                                    {
                                                        (*F).eptr = (*F).eptr.offset(1);
                                                    }
                                                }
                                                CHAR_LF => {}
                                                CHAR_VT | CHAR_FF | 133 | 8232 | 8233 => {
                                                    if (*mb).bsr_convention as ::core::ffi::c_int
                                                        == PCRE2_BSR_ANYCRLF
                                                    {
                                                        rrc = 0 as ::core::ffi::c_int;
                                                        current_block = 9215777819543815842;
                                                        break;
                                                    }
                                                }
                                                _ => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break;
                                                }
                                            }
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                18 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh133 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            match *fresh133 as ::core::ffi::c_int {
                                                CHAR_HT | CHAR_SPACE | 160 | 5760 | 6158 | 8192
                                                | 8193 | 8194 | 8195 | 8196 | 8197 | 8198
                                                | 8199 | 8200 | 8201 | 8202 | 8239 | 8287
                                                | 12288 => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break;
                                                }
                                                _ => {
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                    }
                                }
                                19 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh134 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            match *fresh134 as ::core::ffi::c_int {
                                                CHAR_HT | CHAR_SPACE | 160 | 5760 | 6158 | 8192
                                                | 8193 | 8194 | 8195 | 8196 | 8197 | 8198
                                                | 8199 | 8200 | 8201 | 8202 | 8239 | 8287
                                                | 12288 => {
                                                    i = i.wrapping_add(1);
                                                }
                                                _ => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                20 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh135 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            match *fresh135 as ::core::ffi::c_int {
                                                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | 133
                                                | 8232 | 8233 => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break;
                                                }
                                                _ => {
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                    }
                                }
                                21 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            let fresh136 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            match *fresh136 as ::core::ffi::c_int {
                                                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | 133
                                                | 8232 | 8233 => {
                                                    i = i.wrapping_add(1);
                                                }
                                                _ => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                6 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else if *(*F).eptr as ::core::ffi::c_uint
                                            <= 255 as ::core::ffi::c_uint
                                            && *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                as ::core::ffi::c_int
                                                & ctype_digit
                                                != 0 as ::core::ffi::c_int
                                        {
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            (*F).eptr = (*F).eptr.offset(1);
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                7 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else if !(*(*F).eptr as ::core::ffi::c_uint
                                            <= 255 as ::core::ffi::c_uint)
                                            || *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                as ::core::ffi::c_int
                                                & ctype_digit
                                                == 0 as ::core::ffi::c_int
                                        {
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            (*F).eptr = (*F).eptr.offset(1);
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                8 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else if *(*F).eptr as ::core::ffi::c_uint
                                            <= 255 as ::core::ffi::c_uint
                                            && *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                as ::core::ffi::c_int
                                                & ctype_space
                                                != 0 as ::core::ffi::c_int
                                        {
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            (*F).eptr = (*F).eptr.offset(1);
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                9 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else if !(*(*F).eptr as ::core::ffi::c_uint
                                            <= 255 as ::core::ffi::c_uint)
                                            || *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                as ::core::ffi::c_int
                                                & ctype_space
                                                == 0 as ::core::ffi::c_int
                                        {
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            (*F).eptr = (*F).eptr.offset(1);
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                10 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else if *(*F).eptr as ::core::ffi::c_uint
                                            <= 255 as ::core::ffi::c_uint
                                            && *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                as ::core::ffi::c_int
                                                & ctype_word
                                                != 0 as ::core::ffi::c_int
                                        {
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            (*F).eptr = (*F).eptr.offset(1);
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                11 => {
                                    i = 1 as uint32_t;
                                    loop {
                                        if !(i <= (*F).fields.type_repeat.min) {
                                            current_block = 10634842702853724451;
                                            break;
                                        }
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else if !(*(*F).eptr as ::core::ffi::c_uint
                                            <= 255 as ::core::ffi::c_uint)
                                            || *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                as ::core::ffi::c_int
                                                & ctype_word
                                                == 0 as ::core::ffi::c_int
                                        {
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            (*F).eptr = (*F).eptr.offset(1);
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                _ => return PCRE2_ERROR_INTERNAL,
                            }
                        }
                    } else {
                        current_block = 10634842702853724451;
                    }
                    match current_block {
                        9215777819543815842 => {}
                        _ => {
                            if (*F).fields.type_repeat.min == (*F).fields.type_repeat.max {
                                continue;
                            }
                            if reptype == REPTYPE_MIN as ::core::ffi::c_int as uint32_t {
                                if proptype >= 0 as ::core::ffi::c_int {
                                    current_block = 685409568547905116;
                                    break;
                                } else {
                                    current_block = 4807934851113556144;
                                    break;
                                }
                            } else {
                                (*F).fields.type_repeat.start_eptr = (*F).eptr;
                                if proptype >= 0 as ::core::ffi::c_int {
                                    notmatch_1 = ((*F).fields.type_repeat.ctype
                                        == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                        as ::core::ffi::c_int;
                                    match proptype {
                                        PT_LAMP => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut chartype_4: ::core::ffi::c_int = 0;
                                                let mut len_4: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_4 += 1;
                                                    }
                                                    chartype_4 = (*(&raw const _pcre2_ucd_records_16
                                                        as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                        .offset(
                                                                            (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .chartype as ::core::ffi::c_int;
                                                    if (chartype_4 == ucp_Lu as ::core::ffi::c_int
                                                        || chartype_4
                                                            == ucp_Ll as ::core::ffi::c_int
                                                        || chartype_4
                                                            == ucp_Lt as ::core::ffi::c_int)
                                                        as ::core::ffi::c_int
                                                        == notmatch_1
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_4 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        PT_GC => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_5: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_5 += 1;
                                                    }
                                                    if (*(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                                                .offset(
                                                                    *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                        .offset(
                                                                            (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                                .offset(
                                                                                    (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                                ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                                + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int as isize,
                                                                ))
                                                                .chartype as isize,
                                                        ) == (*F).fields.type_repeat.propvalue)
                                                        as ::core::ffi::c_int == notmatch_1
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_5 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        PT_PC => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_6: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_6 += 1;
                                                    }
                                                    if ((*(&raw const _pcre2_ucd_records_16
                                                        as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                        .offset(
                                                                            (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .chartype as uint32_t == (*F).fields.type_repeat.propvalue)
                                                        as ::core::ffi::c_int == notmatch_1
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_6 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        PT_SC => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_7: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_7 += 1;
                                                    }
                                                    if ((*(&raw const _pcre2_ucd_records_16
                                                        as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                        .offset(
                                                                            (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .script as uint32_t == (*F).fields.type_repeat.propvalue)
                                                        as ::core::ffi::c_int == notmatch_1
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_7 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        PT_SCX => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut ok_5: BOOL = 0;
                                                let mut prop_4: *const ucd_record =
                                                    ::core::ptr::null::<ucd_record>();
                                                let mut len_8: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_8 += 1;
                                                    }
                                                    prop_4 = (&raw const _pcre2_ucd_records_16
                                                        as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                        .offset(
                                                                            (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        );
                                                    ok_5 = ((*prop_4).script as uint32_t
                                                        == (*F).fields.type_repeat.propvalue
                                                        || *(&raw const _pcre2_ucd_script_sets_16
                                                            as *const uint32_t)
                                                            .offset(
                                                                ((*prop_4).scriptx_bidiclass
                                                                    as ::core::ffi::c_int
                                                                    & 0x3ff as ::core::ffi::c_int)
                                                                    as isize,
                                                            )
                                                            .offset(
                                                                (*F).fields
                                                                    .type_repeat
                                                                    .propvalue
                                                                    .wrapping_div(32 as uint32_t)
                                                                    as isize,
                                                            )
                                                            & (1 as uint32_t)
                                                                << (*F)
                                                                    .fields
                                                                    .type_repeat
                                                                    .propvalue
                                                                    .wrapping_rem(32 as uint32_t)
                                                            != 0 as uint32_t)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                    if ok_5 == notmatch_1 {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_8 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        PT_ALNUM => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut category_3: ::core::ffi::c_int = 0;
                                                let mut len_9: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_9 += 1;
                                                    }
                                                    category_3 = *(&raw const _pcre2_ucp_gentype_16
                                                        as *const uint32_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                                                .offset(
                                                                    *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                        .offset(
                                                                            (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                                .offset(
                                                                                    (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                                ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                                + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int as isize,
                                                                ))
                                                                .chartype as isize,
                                                        ) as ::core::ffi::c_int;
                                                    if (category_3 == ucp_L as ::core::ffi::c_int
                                                        || category_3
                                                            == ucp_N as ::core::ffi::c_int)
                                                        as ::core::ffi::c_int
                                                        == notmatch_1
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_9 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        PT_SPACE | PT_PXSPACE => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_10: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_10 += 1;
                                                    }
                                                    match fc {
                                                        9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194 | 8195
                                                        | 8196 | 8197 | 8198 | 8199 | 8200 | 8201 | 8202 | 8239
                                                        | 8287 | 12288 | 10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                                            if notmatch_1 != 0 {
                                                                break;
                                                            }
                                                        }
                                                        _ => {
                                                            if (*(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                                                        .offset(
                                                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                                .offset(
                                                                                    (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                                        .offset(
                                                                                            (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                                        ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                                ) as ::core::ffi::c_int as isize,
                                                                        ))
                                                                        .chartype as isize,
                                                                ) == ucp_Z as ::core::ffi::c_int as uint32_t)
                                                                as ::core::ffi::c_int == notmatch_1
                                                            {
                                                                break;
                                                            }
                                                        }
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_10 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        PT_WORD => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut chartype_5: ::core::ffi::c_int = 0;
                                                let mut category_4: ::core::ffi::c_int = 0;
                                                let mut len_11: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_11 += 1;
                                                    }
                                                    chartype_5 = (*(&raw const _pcre2_ucd_records_16
                                                        as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                        .offset(
                                                                            (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .chartype as ::core::ffi::c_int;
                                                    category_4 = *(&raw const _pcre2_ucp_gentype_16
                                                        as *const uint32_t)
                                                        .offset(chartype_5 as isize)
                                                        as ::core::ffi::c_int;
                                                    if (category_4 == ucp_L as ::core::ffi::c_int
                                                        || category_4
                                                            == ucp_N as ::core::ffi::c_int
                                                        || chartype_5
                                                            == ucp_Mn as ::core::ffi::c_int
                                                        || chartype_5
                                                            == ucp_Pc as ::core::ffi::c_int)
                                                        as ::core::ffi::c_int
                                                        == notmatch_1
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_11 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        PT_CLIST => {
                                            i = (*F).fields.type_repeat.min;
                                            's_16711: while i < (*F).fields.type_repeat.max {
                                                let mut cp_2: *const uint32_t =
                                                    ::core::ptr::null::<uint32_t>();
                                                let mut len_12: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_12 += 1;
                                                    }
                                                    cp_2 = (&raw const _pcre2_ucd_caseless_sets_16
                                                        as *const uint32_t)
                                                        .offset(
                                                            (*F).fields.type_repeat.propvalue
                                                                as isize,
                                                        );
                                                    loop {
                                                        if fc < *cp_2 {
                                                            if notmatch_1 != 0 {
                                                                break;
                                                            } else {
                                                                break 's_16711;
                                                            }
                                                        } else {
                                                            let fresh182 = cp_2;
                                                            cp_2 = cp_2.offset(1);
                                                            if !(fc == *fresh182) {
                                                                continue;
                                                            }
                                                            if notmatch_1 != 0 {
                                                                break 's_16711;
                                                            } else {
                                                                break;
                                                            }
                                                        }
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_12 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        PT_UCNC => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_13: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_13 += 1;
                                                    }
                                                    if (fc == CHAR_DOLLAR_SIGN as uint32_t
                                                        || fc == CHAR_COMMERCIAL_AT as uint32_t
                                                        || fc == CHAR_GRAVE_ACCENT as uint32_t
                                                        || fc >= 0xa0 as uint32_t
                                                            && fc <= 0xd7ff as uint32_t
                                                        || fc >= 0xe000 as uint32_t)
                                                        as ::core::ffi::c_int
                                                        == notmatch_1
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_13 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        PT_BIDICL => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_14: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_14 += 1;
                                                    }
                                                    if (((*(&raw const _pcre2_ucd_records_16
                                                        as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                        .offset(
                                                                            (fc as ::core::ffi::c_int / 128 as ::core::ffi::c_int)
                                                                                as isize,
                                                                        ) as ::core::ffi::c_int * 128 as ::core::ffi::c_int
                                                                        + fc as ::core::ffi::c_int % 128 as ::core::ffi::c_int)
                                                                        as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .scriptx_bidiclass as ::core::ffi::c_int
                                                        >> UCD_BIDICLASS_SHIFT) as uint32_t
                                                        == (*F).fields.type_repeat.propvalue) as ::core::ffi::c_int
                                                        == notmatch_1
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_14 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        PT_BOOL => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut ok_6: BOOL = 0;
                                                let mut prop_5: *const ucd_record =
                                                    ::core::ptr::null::<ucd_record>();
                                                let mut len_15: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if utf != 0
                                                        && fc & 0xfc00 as uint32_t
                                                            == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_15 += 1;
                                                    }
                                                    prop_5 = (&raw const _pcre2_ucd_records_16
                                                        as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                        .offset(
                                                                            (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        );
                                                    ok_6 = (*(&raw const _pcre2_ucd_boolprop_sets_16
                                                        as *const uint32_t)
                                                        .offset(
                                                            ((*prop_5).bprops as ::core::ffi::c_int
                                                                & 0xfff as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                        .offset(
                                                            (*F).fields
                                                                .type_repeat
                                                                .propvalue
                                                                .wrapping_div(32 as uint32_t)
                                                                as isize,
                                                        )
                                                        & (1 as uint32_t)
                                                            << (*F)
                                                                .fields
                                                                .type_repeat
                                                                .propvalue
                                                                .wrapping_rem(32 as uint32_t)
                                                        != 0 as uint32_t)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                    if ok_6 == notmatch_1 {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_15 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        _ => return PCRE2_ERROR_INTERNAL,
                                    }
                                    if reptype == REPTYPE_POS as ::core::ffi::c_int as uint32_t {
                                        continue;
                                    }
                                    current_block = 943143812647028069;
                                } else if (*F).fields.type_repeat.ctype
                                    == OP_EXTUNI as ::core::ffi::c_int as uint32_t
                                {
                                    i = (*F).fields.type_repeat.min;
                                    while i < (*F).fields.type_repeat.max {
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                            break;
                                        } else {
                                            let fresh183 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = *fresh183 as uint32_t;
                                            if utf != 0
                                                && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                            {
                                                let fresh184 = (*F).eptr;
                                                (*F).eptr = (*F).eptr.offset(1);
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fresh184 as uint32_t & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                            (*F).eptr = _pcre2_extuni_16(
                                                fc,
                                                (*F).eptr,
                                                (*mb).check_subject,
                                                (*mb).end_subject,
                                                utf,
                                                ::core::ptr::null_mut::<::core::ffi::c_int>(),
                                            );
                                            if (*F).eptr >= (*mb).end_subject {
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                            }
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                    if reptype == REPTYPE_POS as ::core::ffi::c_int as uint32_t {
                                        continue;
                                    }
                                    current_block = 17293935983997591839;
                                } else if utf != 0 {
                                    match (*F).fields.type_repeat.ctype {
                                        12 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    if if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                                        ((*F).eptr < (*mb).end_subject
                                                            && _pcre2_is_newline_16(
                                                                (*F).eptr,
                                                                (*mb).nltype,
                                                                (*mb).end_subject,
                                                                &raw mut (*mb).nllen,
                                                                utf,
                                                            ) != 0)
                                                            as ::core::ffi::c_int
                                                    } else {
                                                        ((*F).eptr
                                                            <= (*mb)
                                                                .end_subject
                                                                .offset(-((*mb).nllen as isize))
                                                            && *(*F).eptr as ::core::ffi::c_int
                                                                == (*mb).nl[0 as ::core::ffi::c_int
                                                                    as usize]
                                                                    as ::core::ffi::c_int
                                                            && ((*mb).nllen == 1 as uint32_t
                                                                || *(*F).eptr.offset(
                                                                    1 as ::core::ffi::c_int
                                                                        as isize,
                                                                )
                                                                    as ::core::ffi::c_int
                                                                    == (*mb).nl[1
                                                                        as ::core::ffi::c_int
                                                                        as usize]
                                                                        as ::core::ffi::c_int))
                                                            as ::core::ffi::c_int
                                                    } != 0
                                                    {
                                                        break;
                                                    }
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && (*F).eptr.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        ) >= (*mb).end_subject
                                                        && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                                        && (*mb).nllen == 2 as uint32_t
                                                        && *(*F).eptr as ::core::ffi::c_int
                                                            == (*mb).nl
                                                                [0 as ::core::ffi::c_int as usize]
                                                                as ::core::ffi::c_int
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    if (*F).eptr < (*mb).end_subject
                                                        && *(*F).eptr as ::core::ffi::c_uint
                                                            & 0xfc00 as ::core::ffi::c_uint
                                                            == 0xdc00 as ::core::ffi::c_uint
                                                    {
                                                        (*F).eptr = (*F).eptr.offset(1);
                                                    }
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        13 => {
                                            if (*F).fields.type_repeat.max < UINT32_MAX as uint32_t
                                            {
                                                i = (*F).fields.type_repeat.min;
                                                while i < (*F).fields.type_repeat.max {
                                                    if (*F).eptr >= (*mb).end_subject {
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            != 0 as ::core::ffi::c_int
                                                            && ((*F).eptr > (*mb).start_used_ptr
                                                                || (*mb).allowemptypartial != 0)
                                                        {
                                                            (*mb).hitend = TRUE as BOOL;
                                                            if (*mb).partial as ::core::ffi::c_int
                                                                > 1 as ::core::ffi::c_int
                                                            {
                                                                return PCRE2_ERROR_PARTIAL;
                                                            }
                                                        }
                                                        break;
                                                    } else {
                                                        (*F).eptr = (*F).eptr.offset(1);
                                                        if (*F).eptr < (*mb).end_subject
                                                            && *(*F).eptr as ::core::ffi::c_uint
                                                                & 0xfc00 as ::core::ffi::c_uint
                                                                == 0xdc00 as ::core::ffi::c_uint
                                                        {
                                                            (*F).eptr = (*F).eptr.offset(1);
                                                        }
                                                        i = i.wrapping_add(1);
                                                    }
                                                }
                                            } else {
                                                (*F).eptr = (*mb).end_subject;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                            }
                                        }
                                        14 => {
                                            fc = (*F)
                                                .fields
                                                .type_repeat
                                                .max
                                                .wrapping_sub((*F).fields.type_repeat.min);
                                            if fc
                                                > (*mb).end_subject.offset_from((*F).eptr)
                                                    as isize
                                                    as uint32_t
                                            {
                                                (*F).eptr = (*mb).end_subject;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                            } else {
                                                (*F).eptr = (*F).eptr.offset(fc as isize);
                                            }
                                        }
                                        17 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_16: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_16 += 1;
                                                    }
                                                    if fc == CHAR_CR as uint32_t {
                                                        (*F).eptr = (*F).eptr.offset(1);
                                                        if (*F).eptr >= (*mb).end_subject {
                                                            break;
                                                        }
                                                        if *(*F).eptr as ::core::ffi::c_int
                                                            == CHAR_LF
                                                        {
                                                            (*F).eptr = (*F).eptr.offset(1);
                                                        }
                                                    } else {
                                                        if fc != CHAR_LF as uint32_t
                                                            && ((*mb).bsr_convention
                                                                as ::core::ffi::c_int
                                                                == PCRE2_BSR_ANYCRLF
                                                                || fc != CHAR_VT as uint32_t
                                                                    && fc != CHAR_FF as uint32_t
                                                                    && fc != CHAR_NEL as uint32_t
                                                                    && fc != 0x2028 as uint32_t
                                                                    && fc != 0x2029 as uint32_t)
                                                        {
                                                            break;
                                                        }
                                                        (*F).eptr =
                                                            (*F).eptr.offset(len_16 as isize);
                                                    }
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        18 | 19 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut gotspace: BOOL = 0;
                                                let mut len_17: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_17 += 1;
                                                    }
                                                    match fc {
                                                        9 | 32 | 160 | 5760 | 6158 | 8192
                                                        | 8193 | 8194 | 8195 | 8196 | 8197
                                                        | 8198 | 8199 | 8200 | 8201 | 8202
                                                        | 8239 | 8287 | 12288 => {
                                                            gotspace = TRUE as BOOL;
                                                        }
                                                        _ => {
                                                            gotspace = FALSE as BOOL;
                                                        }
                                                    }
                                                    if gotspace
                                                        == ((*F).fields.type_repeat.ctype
                                                            == OP_NOT_HSPACE as ::core::ffi::c_int
                                                                as uint32_t)
                                                            as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_17 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        20 | 21 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut gotspace_0: BOOL = 0;
                                                let mut len_18: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_18 += 1;
                                                    }
                                                    match fc {
                                                        10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                                            gotspace_0 = TRUE as BOOL;
                                                        }
                                                        _ => {
                                                            gotspace_0 = FALSE as BOOL;
                                                        }
                                                    }
                                                    if gotspace_0
                                                        == ((*F).fields.type_repeat.ctype
                                                            == OP_NOT_VSPACE as ::core::ffi::c_int
                                                                as uint32_t)
                                                            as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_18 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        6 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_19: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_19 += 1;
                                                    }
                                                    if fc < 256 as uint32_t
                                                        && *(*mb).ctypes.offset(fc as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_digit
                                                            != 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_19 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        7 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_20: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_20 += 1;
                                                    }
                                                    if fc >= 256 as uint32_t
                                                        || *(*mb).ctypes.offset(fc as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_digit
                                                            == 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_20 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        8 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_21: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_21 += 1;
                                                    }
                                                    if fc < 256 as uint32_t
                                                        && *(*mb).ctypes.offset(fc as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_space
                                                            != 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_21 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        9 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_22: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_22 += 1;
                                                    }
                                                    if fc >= 256 as uint32_t
                                                        || *(*mb).ctypes.offset(fc as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_space
                                                            == 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_22 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        10 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_23: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_23 += 1;
                                                    }
                                                    if fc < 256 as uint32_t
                                                        && *(*mb).ctypes.offset(fc as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_word
                                                            != 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_23 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        11 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                let mut len_24: ::core::ffi::c_int =
                                                    1 as ::core::ffi::c_int;
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                                    {
                                                        fc = ((fc & 0x3ff as uint32_t)
                                                            << 10 as ::core::ffi::c_int
                                                            | *(*F).eptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as uint32_t
                                                                & 0x3ff as uint32_t)
                                                            .wrapping_add(0x10000 as uint32_t);
                                                        len_24 += 1;
                                                    }
                                                    if fc >= 256 as uint32_t
                                                        || *(*mb).ctypes.offset(fc as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_word
                                                            == 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(len_24 as isize);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        _ => return PCRE2_ERROR_INTERNAL,
                                    }
                                    if reptype == REPTYPE_POS as ::core::ffi::c_int as uint32_t {
                                        continue;
                                    }
                                    current_block = 6158284093282687738;
                                } else {
                                    match (*F).fields.type_repeat.ctype {
                                        12 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    if if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                                        ((*F).eptr < (*mb).end_subject
                                                            && _pcre2_is_newline_16(
                                                                (*F).eptr,
                                                                (*mb).nltype,
                                                                (*mb).end_subject,
                                                                &raw mut (*mb).nllen,
                                                                utf,
                                                            ) != 0)
                                                            as ::core::ffi::c_int
                                                    } else {
                                                        ((*F).eptr
                                                            <= (*mb)
                                                                .end_subject
                                                                .offset(-((*mb).nllen as isize))
                                                            && *(*F).eptr as ::core::ffi::c_int
                                                                == (*mb).nl[0 as ::core::ffi::c_int
                                                                    as usize]
                                                                    as ::core::ffi::c_int
                                                            && ((*mb).nllen == 1 as uint32_t
                                                                || *(*F).eptr.offset(
                                                                    1 as ::core::ffi::c_int
                                                                        as isize,
                                                                )
                                                                    as ::core::ffi::c_int
                                                                    == (*mb).nl[1
                                                                        as ::core::ffi::c_int
                                                                        as usize]
                                                                        as ::core::ffi::c_int))
                                                            as ::core::ffi::c_int
                                                    } != 0
                                                    {
                                                        break;
                                                    }
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && (*F).eptr.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        ) >= (*mb).end_subject
                                                        && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                                        && (*mb).nllen == 2 as uint32_t
                                                        && *(*F).eptr as ::core::ffi::c_int
                                                            == (*mb).nl
                                                                [0 as ::core::ffi::c_int as usize]
                                                                as ::core::ffi::c_int
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        13 | 14 => {
                                            fc = (*F)
                                                .fields
                                                .type_repeat
                                                .max
                                                .wrapping_sub((*F).fields.type_repeat.min);
                                            if fc
                                                > (*mb).end_subject.offset_from((*F).eptr)
                                                    as isize
                                                    as uint32_t
                                            {
                                                (*F).eptr = (*mb).end_subject;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                            } else {
                                                (*F).eptr = (*F).eptr.offset(fc as isize);
                                            }
                                        }
                                        17 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    fc = *(*F).eptr as uint32_t;
                                                    if fc == CHAR_CR as uint32_t {
                                                        (*F).eptr = (*F).eptr.offset(1);
                                                        if (*F).eptr >= (*mb).end_subject {
                                                            break;
                                                        }
                                                        if *(*F).eptr as ::core::ffi::c_int
                                                            == CHAR_LF
                                                        {
                                                            (*F).eptr = (*F).eptr.offset(1);
                                                        }
                                                    } else {
                                                        if fc != CHAR_LF as uint32_t
                                                            && ((*mb).bsr_convention
                                                                as ::core::ffi::c_int
                                                                == PCRE2_BSR_ANYCRLF
                                                                || fc != CHAR_VT as uint32_t
                                                                    && fc != CHAR_FF as uint32_t
                                                                    && fc != CHAR_NEL as uint32_t
                                                                    && fc != 0x2028 as uint32_t
                                                                    && fc != 0x2029 as uint32_t)
                                                        {
                                                            break;
                                                        }
                                                        (*F).eptr = (*F).eptr.offset(1);
                                                    }
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        18 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    match *(*F).eptr as ::core::ffi::c_int {
                                                        CHAR_HT | CHAR_SPACE | 160 | 5760
                                                        | 6158 | 8192 | 8193 | 8194 | 8195
                                                        | 8196 | 8197 | 8198 | 8199 | 8200
                                                        | 8201 | 8202 | 8239 | 8287 | 12288 => {
                                                            break;
                                                        }
                                                        _ => {}
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        19 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    match *(*F).eptr as ::core::ffi::c_int {
                                                        CHAR_HT | CHAR_SPACE | 160 | 5760
                                                        | 6158 | 8192 | 8193 | 8194 | 8195
                                                        | 8196 | 8197 | 8198 | 8199 | 8200
                                                        | 8201 | 8202 | 8239 | 8287 | 12288 => {}
                                                        _ => {
                                                            break;
                                                        }
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        20 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    match *(*F).eptr as ::core::ffi::c_int {
                                                        CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR
                                                        | 133 | 8232 | 8233 => {
                                                            break;
                                                        }
                                                        _ => {}
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        21 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    match *(*F).eptr as ::core::ffi::c_int {
                                                        CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR
                                                        | 133 | 8232 | 8233 => {}
                                                        _ => {
                                                            break;
                                                        }
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        6 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    if *(*F).eptr as ::core::ffi::c_uint
                                                        <= 255 as ::core::ffi::c_uint
                                                        && *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_digit
                                                            != 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        7 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    if !(*(*F).eptr as ::core::ffi::c_uint
                                                        <= 255 as ::core::ffi::c_uint)
                                                        || *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_digit
                                                            == 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        8 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    if *(*F).eptr as ::core::ffi::c_uint
                                                        <= 255 as ::core::ffi::c_uint
                                                        && *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_space
                                                            != 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        9 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    if !(*(*F).eptr as ::core::ffi::c_uint
                                                        <= 255 as ::core::ffi::c_uint)
                                                        || *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_space
                                                            == 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        10 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    if *(*F).eptr as ::core::ffi::c_uint
                                                        <= 255 as ::core::ffi::c_uint
                                                        && *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_word
                                                            != 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        11 => {
                                            i = (*F).fields.type_repeat.min;
                                            while i < (*F).fields.type_repeat.max {
                                                if (*F).eptr >= (*mb).end_subject {
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        != 0 as ::core::ffi::c_int
                                                        && ((*F).eptr > (*mb).start_used_ptr
                                                            || (*mb).allowemptypartial != 0)
                                                    {
                                                        (*mb).hitend = TRUE as BOOL;
                                                        if (*mb).partial as ::core::ffi::c_int
                                                            > 1 as ::core::ffi::c_int
                                                        {
                                                            return PCRE2_ERROR_PARTIAL;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    if !(*(*F).eptr as ::core::ffi::c_uint
                                                        <= 255 as ::core::ffi::c_uint)
                                                        || *(*mb).ctypes.offset(*(*F).eptr as isize)
                                                            as ::core::ffi::c_int
                                                            & ctype_word
                                                            == 0 as ::core::ffi::c_int
                                                    {
                                                        break;
                                                    }
                                                    (*F).eptr = (*F).eptr.offset(1);
                                                    i = i.wrapping_add(1);
                                                }
                                            }
                                        }
                                        _ => return PCRE2_ERROR_INTERNAL,
                                    }
                                    if reptype == REPTYPE_POS as ::core::ffi::c_int as uint32_t {
                                        continue;
                                    }
                                    current_block = 981804248182727996;
                                }
                            }
                        }
                    }
                }
                7201258650426958033 => {
                    let fresh22 = (*F).ecode;
                    (*F).ecode = (*F).ecode.offset(1);
                    (*F).fields.charnot_repeat.c = *fresh22 as uint32_t;
                    if utf != 0
                        && (*F).fields.charnot_repeat.c & 0xfc00 as uint32_t == 0xd800 as uint32_t
                    {
                        let fresh23 = (*F).ecode;
                        (*F).ecode = (*F).ecode.offset(1);
                        (*F).fields.charnot_repeat.c = (((*F).fields.charnot_repeat.c
                            & 0x3ff as uint32_t)
                            << 10 as ::core::ffi::c_int
                            | *fresh23 as uint32_t & 0x3ff as uint32_t)
                            .wrapping_add(0x10000 as uint32_t);
                    }
                    if (*F).op as ::core::ffi::c_int >= OP_NOTSTARI as ::core::ffi::c_int {
                        if (utf != 0 || ucp != 0) && (*F).fields.charnot_repeat.c > 127 as uint32_t
                        {
                            (*F).fields.charnot_repeat.oc = ((*F).fields.charnot_repeat.c
                                as ::core::ffi::c_int
                                + (*(&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
                                    *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                                        (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                            .offset(
                                                ((*F).fields.charnot_repeat.c as ::core::ffi::c_int
                                                    / UCD_BLOCK_SIZE)
                                                    as isize,
                                            )
                                            as ::core::ffi::c_int
                                            * UCD_BLOCK_SIZE
                                            + (*F).fields.charnot_repeat.c as ::core::ffi::c_int
                                                % UCD_BLOCK_SIZE)
                                            as isize,
                                    ) as ::core::ffi::c_int
                                        as isize,
                                ))
                                .other_case as ::core::ffi::c_int)
                                as uint32_t;
                        } else {
                            (*F).fields.charnot_repeat.oc = if (*F).fields.charnot_repeat.c
                                <= 255 as uint32_t
                            {
                                *(*mb).fcc.offset((*F).fields.charnot_repeat.c as isize) as uint32_t
                            } else {
                                (*F).fields.charnot_repeat.c
                            };
                        }
                        if utf != 0 {
                            let mut d: uint32_t = 0;
                            i = 1 as uint32_t;
                            loop {
                                if !(i <= (*F).fields.charnot_repeat.min) {
                                    current_block = 10761992078184920710;
                                    break;
                                }
                                if (*F).eptr >= (*mb).end_subject {
                                    if (*mb).partial as ::core::ffi::c_int
                                        != 0 as ::core::ffi::c_int
                                        && ((*F).eptr > (*mb).start_used_ptr
                                            || (*mb).allowemptypartial != 0)
                                    {
                                        (*mb).hitend = TRUE as BOOL;
                                        if (*mb).partial as ::core::ffi::c_int
                                            > 1 as ::core::ffi::c_int
                                        {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                    }
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    break;
                                } else {
                                    let fresh24 = (*F).eptr;
                                    (*F).eptr = (*F).eptr.offset(1);
                                    d = *fresh24 as uint32_t;
                                    if d & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                        let fresh25 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        d = ((d & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                            | *fresh25 as uint32_t & 0x3ff as uint32_t)
                                            .wrapping_add(0x10000 as uint32_t);
                                    }
                                    if (*F).fields.charnot_repeat.c == d
                                        || (*F).fields.charnot_repeat.oc == d
                                    {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        break;
                                    } else {
                                        i = i.wrapping_add(1);
                                    }
                                }
                            }
                        } else {
                            i = 1 as uint32_t;
                            loop {
                                if !(i <= (*F).fields.charnot_repeat.min) {
                                    current_block = 10761992078184920710;
                                    break;
                                }
                                if (*F).eptr >= (*mb).end_subject {
                                    if (*mb).partial as ::core::ffi::c_int
                                        != 0 as ::core::ffi::c_int
                                        && ((*F).eptr > (*mb).start_used_ptr
                                            || (*mb).allowemptypartial != 0)
                                    {
                                        (*mb).hitend = TRUE as BOOL;
                                        if (*mb).partial as ::core::ffi::c_int
                                            > 1 as ::core::ffi::c_int
                                        {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                    }
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    break;
                                } else if (*F).fields.charnot_repeat.c == *(*F).eptr as uint32_t
                                    || (*F).fields.charnot_repeat.oc == *(*F).eptr as uint32_t
                                {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    break;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(1);
                                    i = i.wrapping_add(1);
                                }
                            }
                        }
                        match current_block {
                            9215777819543815842 => {}
                            _ => {
                                if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max
                                {
                                    continue;
                                }
                                if reptype == REPTYPE_MIN as ::core::ffi::c_int as uint32_t {
                                    if utf != 0 {
                                        current_block = 8038707252912297919;
                                        break;
                                    } else {
                                        current_block = 9699707990742192723;
                                        break;
                                    }
                                } else {
                                    (*F).fields.charnot_repeat.start_eptr = (*F).eptr;
                                    if utf != 0 {
                                        d_1 = 0;
                                        i = (*F).fields.charnot_repeat.min;
                                        while i < (*F).fields.charnot_repeat.max {
                                            let mut len: ::core::ffi::c_int =
                                                1 as ::core::ffi::c_int;
                                            if (*F).eptr >= (*mb).end_subject {
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                                break;
                                            } else {
                                                d_1 = *(*F).eptr as uint32_t;
                                                if d_1 & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                                    d_1 = ((d_1 & 0x3ff as uint32_t)
                                                        << 10 as ::core::ffi::c_int
                                                        | *(*F).eptr.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as uint32_t
                                                            & 0x3ff as uint32_t)
                                                        .wrapping_add(0x10000 as uint32_t);
                                                    len += 1;
                                                }
                                                if (*F).fields.charnot_repeat.c == d_1
                                                    || (*F).fields.charnot_repeat.oc == d_1
                                                {
                                                    break;
                                                }
                                                (*F).eptr = (*F).eptr.offset(len as isize);
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                        if !(reptype
                                            != REPTYPE_POS as ::core::ffi::c_int as uint32_t)
                                        {
                                            continue;
                                        }
                                        current_block = 3660592233603849974;
                                    } else {
                                        i = (*F).fields.charnot_repeat.min;
                                        while i < (*F).fields.charnot_repeat.max {
                                            if (*F).eptr >= (*mb).end_subject {
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                                break;
                                            } else {
                                                if (*F).fields.charnot_repeat.c
                                                    == *(*F).eptr as uint32_t
                                                    || (*F).fields.charnot_repeat.oc
                                                        == *(*F).eptr as uint32_t
                                                {
                                                    break;
                                                }
                                                (*F).eptr = (*F).eptr.offset(1);
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                        if !(reptype
                                            != REPTYPE_POS as ::core::ffi::c_int as uint32_t)
                                        {
                                            continue;
                                        }
                                        current_block = 12956711689462799609;
                                    }
                                }
                            }
                        }
                    } else {
                        if utf != 0 {
                            let mut d_2: uint32_t = 0;
                            i = 1 as uint32_t;
                            loop {
                                if !(i <= (*F).fields.charnot_repeat.min) {
                                    current_block = 1376215293001043184;
                                    break;
                                }
                                if (*F).eptr >= (*mb).end_subject {
                                    if (*mb).partial as ::core::ffi::c_int
                                        != 0 as ::core::ffi::c_int
                                        && ((*F).eptr > (*mb).start_used_ptr
                                            || (*mb).allowemptypartial != 0)
                                    {
                                        (*mb).hitend = TRUE as BOOL;
                                        if (*mb).partial as ::core::ffi::c_int
                                            > 1 as ::core::ffi::c_int
                                        {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                    }
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    break;
                                } else {
                                    let fresh30 = (*F).eptr;
                                    (*F).eptr = (*F).eptr.offset(1);
                                    d_2 = *fresh30 as uint32_t;
                                    if d_2 & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                        let fresh31 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        d_2 = ((d_2 & 0x3ff as uint32_t)
                                            << 10 as ::core::ffi::c_int
                                            | *fresh31 as uint32_t & 0x3ff as uint32_t)
                                            .wrapping_add(0x10000 as uint32_t);
                                    }
                                    if (*F).fields.charnot_repeat.c == d_2 {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        break;
                                    } else {
                                        i = i.wrapping_add(1);
                                    }
                                }
                            }
                        } else {
                            i = 1 as uint32_t;
                            loop {
                                if !(i <= (*F).fields.charnot_repeat.min) {
                                    current_block = 1376215293001043184;
                                    break;
                                }
                                if (*F).eptr >= (*mb).end_subject {
                                    if (*mb).partial as ::core::ffi::c_int
                                        != 0 as ::core::ffi::c_int
                                        && ((*F).eptr > (*mb).start_used_ptr
                                            || (*mb).allowemptypartial != 0)
                                    {
                                        (*mb).hitend = TRUE as BOOL;
                                        if (*mb).partial as ::core::ffi::c_int
                                            > 1 as ::core::ffi::c_int
                                        {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                    }
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    break;
                                } else {
                                    let fresh32 = (*F).eptr;
                                    (*F).eptr = (*F).eptr.offset(1);
                                    if (*F).fields.charnot_repeat.c == *fresh32 as uint32_t {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        break;
                                    } else {
                                        i = i.wrapping_add(1);
                                    }
                                }
                            }
                        }
                        match current_block {
                            9215777819543815842 => {}
                            _ => {
                                if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max
                                {
                                    continue;
                                }
                                if reptype == REPTYPE_MIN as ::core::ffi::c_int as uint32_t {
                                    if utf != 0 {
                                        current_block = 10771577056827685507;
                                        break;
                                    } else {
                                        current_block = 10082661835832785913;
                                        break;
                                    }
                                } else {
                                    (*F).fields.charnot_repeat.start_eptr = (*F).eptr;
                                    if utf != 0 {
                                        d_4 = 0;
                                        i = (*F).fields.charnot_repeat.min;
                                        while i < (*F).fields.charnot_repeat.max {
                                            let mut len_0: ::core::ffi::c_int =
                                                1 as ::core::ffi::c_int;
                                            if (*F).eptr >= (*mb).end_subject {
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                                break;
                                            } else {
                                                d_4 = *(*F).eptr as uint32_t;
                                                if d_4 & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                                    d_4 = ((d_4 & 0x3ff as uint32_t)
                                                        << 10 as ::core::ffi::c_int
                                                        | *(*F).eptr.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as uint32_t
                                                            & 0x3ff as uint32_t)
                                                        .wrapping_add(0x10000 as uint32_t);
                                                    len_0 += 1;
                                                }
                                                if (*F).fields.charnot_repeat.c == d_4 {
                                                    break;
                                                }
                                                (*F).eptr = (*F).eptr.offset(len_0 as isize);
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                        if !(reptype
                                            != REPTYPE_POS as ::core::ffi::c_int as uint32_t)
                                        {
                                            continue;
                                        }
                                        current_block = 11364986971110899152;
                                    } else {
                                        i = (*F).fields.charnot_repeat.min;
                                        while i < (*F).fields.charnot_repeat.max {
                                            if (*F).eptr >= (*mb).end_subject {
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                                break;
                                            } else {
                                                if (*F).fields.charnot_repeat.c
                                                    == *(*F).eptr as uint32_t
                                                {
                                                    break;
                                                }
                                                (*F).eptr = (*F).eptr.offset(1);
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                        if !(reptype
                                            != REPTYPE_POS as ::core::ffi::c_int as uint32_t)
                                        {
                                            continue;
                                        }
                                        current_block = 4187073793469983967;
                                    }
                                }
                            }
                        }
                    }
                }
                11724773952707043839 => {
                    if utf != 0 {
                        length = 1 as size_t;
                        (*F).fields.char_repeat.charptr = (*F).ecode;
                        fc = *(*F).ecode as uint32_t;
                        if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                            fc = ((fc & 0x3ff as uint32_t) << 10 as ::core::ffi::c_int
                                | *(*F).ecode.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3ff as uint32_t)
                                .wrapping_add(0x10000 as uint32_t);
                            length = length.wrapping_add(1);
                        }
                        (*F).ecode = (*F).ecode.offset(length as isize);
                        (*F).byte1 = length as uint8_t;
                        if length > 1 as size_t {
                            othercase = 0;
                            if (*F).op as ::core::ffi::c_int >= OP_STARI as ::core::ffi::c_int && {
                                othercase = (fc as ::core::ffi::c_int
                                    + (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                        .offset(
                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                .offset(
                                                    (*(&raw const _pcre2_ucd_stage1_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (fc as ::core::ffi::c_int
                                                                / UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        * UCD_BLOCK_SIZE
                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                                        as isize,
                                                )
                                                as ::core::ffi::c_int
                                                as isize,
                                        ))
                                    .other_case
                                        as ::core::ffi::c_int)
                                    as uint32_t;
                                othercase != fc
                            } {
                                (*F).byte2 = _pcre2_ord2utf_16(
                                    othercase,
                                    &raw mut (*F).fields.char_repeat.oc.occu as *mut PCRE2_UCHAR16,
                                ) as uint8_t;
                            } else {
                                (*F).byte2 = 0 as uint8_t;
                            }
                            i = 1 as uint32_t;
                            loop {
                                if !(i <= (*F).fields.char_repeat.min) {
                                    current_block = 2754480941084577532;
                                    break;
                                }
                                if (*F).eptr <= (*mb).end_subject.offset(-(length as isize))
                                    && memcmp(
                                        (*F).eptr as *const ::core::ffi::c_void,
                                        (*F).fields.char_repeat.charptr
                                            as *const ::core::ffi::c_void,
                                        length.wrapping_mul(
                                            (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int)
                                                as size_t,
                                        ),
                                    ) == 0 as ::core::ffi::c_int
                                {
                                    (*F).eptr = (*F).eptr.offset(length as isize);
                                } else if (*F).byte2 as ::core::ffi::c_int > 0 as ::core::ffi::c_int
                                    && (*F).eptr
                                        <= (*mb)
                                            .end_subject
                                            .offset(-((*F).byte2 as ::core::ffi::c_int as isize))
                                    && memcmp(
                                        (*F).eptr as *const ::core::ffi::c_void,
                                        &raw mut (*F).fields.char_repeat.oc.occu
                                            as *mut PCRE2_UCHAR16
                                            as *const ::core::ffi::c_void,
                                        ((*F).byte2 as ::core::ffi::c_int
                                            * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int))
                                            as size_t,
                                    ) == 0 as ::core::ffi::c_int
                                {
                                    (*F).eptr =
                                        (*F).eptr.offset((*F).byte2 as ::core::ffi::c_int as isize);
                                } else {
                                    if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                    }
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    break;
                                }
                                i = i.wrapping_add(1);
                            }
                            match current_block {
                                9215777819543815842 => {}
                                _ => {
                                    if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
                                        continue;
                                    }
                                    if reptype == REPTYPE_MIN as ::core::ffi::c_int as uint32_t {
                                        current_block = 1391843528280232292;
                                        break;
                                    }
                                    (*F).fields.char_repeat.start_eptr = (*F).eptr;
                                    i = (*F).fields.char_repeat.min;
                                    while i < (*F).fields.char_repeat.max {
                                        if (*F).eptr
                                            <= (*mb).end_subject.offset(
                                                -((*F).byte1 as ::core::ffi::c_int as isize),
                                            )
                                            && memcmp(
                                                (*F).eptr as *const ::core::ffi::c_void,
                                                (*F).fields.char_repeat.charptr
                                                    as *const ::core::ffi::c_void,
                                                ((*F).byte1 as ::core::ffi::c_int
                                                    * (PCRE2_CODE_UNIT_WIDTH
                                                        / 8 as ::core::ffi::c_int))
                                                    as size_t,
                                            ) == 0 as ::core::ffi::c_int
                                        {
                                            (*F).eptr = (*F)
                                                .eptr
                                                .offset((*F).byte1 as ::core::ffi::c_int as isize);
                                        } else if (*F).byte2 as ::core::ffi::c_int
                                            > 0 as ::core::ffi::c_int
                                            && (*F).eptr
                                                <= (*mb).end_subject.offset(
                                                    -((*F).byte2 as ::core::ffi::c_int as isize),
                                                )
                                            && memcmp(
                                                (*F).eptr as *const ::core::ffi::c_void,
                                                &raw mut (*F).fields.char_repeat.oc.occu
                                                    as *mut PCRE2_UCHAR16
                                                    as *const ::core::ffi::c_void,
                                                ((*F).byte2 as ::core::ffi::c_int
                                                    * (PCRE2_CODE_UNIT_WIDTH
                                                        / 8 as ::core::ffi::c_int))
                                                    as size_t,
                                            ) == 0 as ::core::ffi::c_int
                                        {
                                            (*F).eptr = (*F)
                                                .eptr
                                                .offset((*F).byte2 as ::core::ffi::c_int as isize);
                                        } else {
                                            if (*F).eptr >= (*mb).end_subject {
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                            }
                                            break;
                                        }
                                        i = i.wrapping_add(1);
                                    }
                                    if !(reptype != REPTYPE_POS as ::core::ffi::c_int as uint32_t) {
                                        continue;
                                    }
                                    current_block = 9029608189573075810;
                                }
                            }
                        } else {
                            (*F).fields.char_repeat.c = fc;
                            current_block = 15882700527400876337;
                        }
                    } else {
                        let fresh16 = (*F).ecode;
                        (*F).ecode = (*F).ecode.offset(1);
                        (*F).fields.char_repeat.c = *fresh16 as uint32_t;
                        current_block = 15882700527400876337;
                    }
                    match current_block {
                        9215777819543815842 => {}
                        9029608189573075810 => {}
                        _ => {
                            if (*F).op as ::core::ffi::c_int >= OP_STARI as ::core::ffi::c_int {
                                if (utf != 0 || ucp != 0)
                                    && (*F).fields.char_repeat.c > 127 as uint32_t
                                {
                                    (*F).fields.char_repeat.oc.oc = ((*F).fields.char_repeat.c
                                        as ::core::ffi::c_int
                                        + (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                            .offset(
                                                *(&raw const _pcre2_ucd_stage2_16
                                                    as *const uint16_t)
                                                    .offset(
                                                        (*(&raw const _pcre2_ucd_stage1_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                ((*F).fields.char_repeat.c
                                                                    as ::core::ffi::c_int
                                                                    / UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            * UCD_BLOCK_SIZE
                                                            + (*F).fields.char_repeat.c
                                                                as ::core::ffi::c_int
                                                                % UCD_BLOCK_SIZE)
                                                            as isize,
                                                    )
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            ))
                                        .other_case
                                            as ::core::ffi::c_int)
                                        as uint32_t;
                                } else {
                                    (*F).fields.char_repeat.oc.oc =
                                        if (*F).fields.char_repeat.c <= 255 as uint32_t {
                                            *(*mb).fcc.offset((*F).fields.char_repeat.c as isize)
                                                as uint32_t
                                        } else {
                                            (*F).fields.char_repeat.c
                                        };
                                }
                                i = 1 as uint32_t;
                                loop {
                                    if !(i <= (*F).fields.char_repeat.min) {
                                        current_block = 738707116226373839;
                                        break;
                                    }
                                    let mut cc_1: uint32_t = 0;
                                    if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        break;
                                    } else {
                                        cc_1 = *(*F).eptr as uint32_t;
                                        if (*F).fields.char_repeat.c != cc_1
                                            && (*F).fields.char_repeat.oc.oc != cc_1
                                        {
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            (*F).eptr = (*F).eptr.offset(1);
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                match current_block {
                                    9215777819543815842 => {}
                                    _ => {
                                        if (*F).fields.char_repeat.min
                                            == (*F).fields.char_repeat.max
                                        {
                                            continue;
                                        }
                                        if reptype == REPTYPE_MIN as ::core::ffi::c_int as uint32_t
                                        {
                                            current_block = 5981395450828152283;
                                            break;
                                        }
                                        (*F).fields.char_repeat.start_eptr = (*F).eptr;
                                        i = (*F).fields.char_repeat.min;
                                        while i < (*F).fields.char_repeat.max {
                                            let mut cc_3: uint32_t = 0;
                                            if (*F).eptr >= (*mb).end_subject {
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                                break;
                                            } else {
                                                cc_3 = *(*F).eptr as uint32_t;
                                                if (*F).fields.char_repeat.c != cc_3
                                                    && (*F).fields.char_repeat.oc.oc != cc_3
                                                {
                                                    break;
                                                }
                                                (*F).eptr = (*F).eptr.offset(1);
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                        if !(reptype
                                            != REPTYPE_POS as ::core::ffi::c_int as uint32_t)
                                        {
                                            continue;
                                        }
                                        current_block = 9522675792805490914;
                                    }
                                }
                            } else {
                                i = 1 as uint32_t;
                                loop {
                                    if !(i <= (*F).fields.char_repeat.min) {
                                        current_block = 2954287494858939679;
                                        break;
                                    }
                                    if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        break;
                                    } else {
                                        let fresh18 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        if (*F).fields.char_repeat.c != *fresh18 as uint32_t {
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            break;
                                        } else {
                                            i = i.wrapping_add(1);
                                        }
                                    }
                                }
                                match current_block {
                                    9215777819543815842 => {}
                                    _ => {
                                        if (*F).fields.char_repeat.min
                                            == (*F).fields.char_repeat.max
                                        {
                                            continue;
                                        }
                                        if reptype == REPTYPE_MIN as ::core::ffi::c_int as uint32_t
                                        {
                                            current_block = 6021810480259369443;
                                            break;
                                        }
                                        (*F).fields.char_repeat.start_eptr = (*F).eptr;
                                        i = (*F).fields.char_repeat.min;
                                        while i < (*F).fields.char_repeat.max {
                                            if (*F).eptr >= (*mb).end_subject {
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                                break;
                                            } else {
                                                if (*F).fields.char_repeat.c
                                                    != *(*F).eptr as uint32_t
                                                {
                                                    break;
                                                }
                                                (*F).eptr = (*F).eptr.offset(1);
                                                i = i.wrapping_add(1);
                                            }
                                        }
                                        if !(reptype
                                            != REPTYPE_POS as ::core::ffi::c_int as uint32_t)
                                        {
                                            continue;
                                        }
                                        current_block = 2574904801665210416;
                                    }
                                }
                            }
                        }
                    }
                }
                17964307082103460721 => {
                    if (*F).eptr >= (*mb).end_subject {
                        if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                            && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                        {
                            (*mb).hitend = TRUE as BOOL;
                            if (*mb).partial as ::core::ffi::c_int > 1 as ::core::ffi::c_int {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        rrc = 0 as ::core::ffi::c_int;
                    } else {
                        (*F).eptr = (*F).eptr.offset(1);
                        if utf != 0 {
                            if (*F).eptr < (*mb).end_subject
                                && *(*F).eptr as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                                    == 0xdc00 as ::core::ffi::c_uint
                            {
                                (*F).eptr = (*F).eptr.offset(1);
                            }
                        }
                        (*F).ecode = (*F).ecode.offset(1);
                        continue;
                    }
                    current_block = 9215777819543815842;
                }
                _ => {}
            }
            '_RETURN_SWITCH: loop {
                match current_block {
                    6619656240033923038 => {
                        (*F).ecode = (*F)
                            .ecode
                            .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                        continue 's_263;
                    }
                    16853744556532821084 => {
                        (*F).ecode = (*F).ecode.offset(
                            (if condition != 0 {
                                *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                                    .offset(*(*F).ecode as isize)
                                    as size_t
                            } else {
                                (*F).fields.op_cond.length
                            }) as isize,
                        );
                        if !((*F).op as ::core::ffi::c_int == OP_SCOND as ::core::ffi::c_int) {
                            continue 's_263;
                        }
                        group_frame_type = GF_NOCAPTURE as uint32_t;
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM35 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    2089970334650350822 => {
                        current_branch = (*F).ecode;
                        next_branch = current_branch
                            .offset(*current_branch.offset(1 as ::core::ffi::c_int as isize)
                                as ::core::ffi::c_int as isize);
                        if *next_branch as ::core::ffi::c_int != OP_ALT as ::core::ffi::c_int {
                            current_block = 103976322284692088;
                            break;
                        } else {
                            current_block = 13686519605006922459;
                            break;
                        }
                    }
                    7653578622174939700 => {
                        if (*F).eptr >= (*F).fields.ref_repeat.start {
                            start_ecode = (*F).ecode;
                            (*F).return_id = RM21 as ::core::ffi::c_int as uint8_t;
                            current_block = 6380077529129020506;
                            break 's_263;
                        } else {
                            current_block = 10033017790122999806;
                        }
                    }
                    981804248182727996 => {
                        if (*F).eptr == (*F).fields.type_repeat.start_eptr {
                            continue 's_263;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM34 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    6158284093282687738 => {
                        if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
                            continue 's_263;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM220 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    17293935983997591839 => {
                        lgb = 0;
                        rgb = 0;
                        fptr = ::core::ptr::null::<PCRE2_UCHAR16>();
                        if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
                            continue 's_263;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM219 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    943143812647028069 => {
                        if (*F).eptr <= (*F).fields.type_repeat.start_eptr {
                            continue 's_263;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM221 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    15840449704302887135 => {
                        if (*F).eptr >= (*F).fields.class_repeat.start_eptr {
                            start_ecode = (*F).ecode;
                            (*F).return_id = RM24 as ::core::ffi::c_int as uint8_t;
                            current_block = 6380077529129020506;
                            break 's_263;
                        } else {
                            current_block = 10848253239906386529;
                        }
                    }
                    4187073793469983967 => {
                        if (*F).eptr == (*F).fields.charnot_repeat.start_eptr {
                            continue 's_263;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM32 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    11364986971110899152 => {
                        if (*F).eptr <= (*F).fields.charnot_repeat.start_eptr {
                            continue 's_263;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM207 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    12956711689462799609 => {
                        if (*F).eptr == (*F).fields.charnot_repeat.start_eptr {
                            continue 's_263;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM30 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    3660592233603849974 => {
                        if (*F).eptr <= (*F).fields.charnot_repeat.start_eptr {
                            continue 's_263;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM205 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    2574904801665210416 => {
                        if (*F).eptr <= (*F).fields.char_repeat.start_eptr {
                            continue 's_263;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM28 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    9522675792805490914 => {
                        if (*F).eptr == (*F).fields.char_repeat.start_eptr {
                            continue 's_263;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM26 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    9029608189573075810 => {
                        if (*F).eptr <= (*F).fields.char_repeat.start_eptr {
                            continue 's_263;
                        }
                        start_ecode = (*F).ecode;
                        (*F).return_id = RM203 as ::core::ffi::c_int as uint8_t;
                        current_block = 6380077529129020506;
                        break 's_263;
                    }
                    _ => {
                        if (*F).eptr > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = (*F).eptr;
                        }
                        if (*F).rdepth == 0 as uint32_t {
                            return rrc;
                        }
                        F = (F as *mut ::core::ffi::c_char).offset(-((*F).back_frame as isize))
                            as *mut heapframe;
                        (*(*mb).cb).callout_flags =
                            ((*(*mb).cb).callout_flags as ::core::ffi::c_uint
                                | PCRE2_CALLOUT_BACKTRACK) as uint32_t;
                        match (*F).return_id as ::core::ffi::c_int {
                            1 => {
                                if !(rrc != MATCH_NOMATCH) {
                                    current_block = 2089970334650350822;
                                    continue;
                                }
                                rrc = rrc;
                                current_block = 9215777819543815842;
                                continue;
                            }
                            2 => {
                                if rrc == MATCH_THEN {
                                    let mut next_ecode_3: PCRE2_SPTR16 = (*F).ecode.offset(
                                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            as isize,
                                    );
                                    if (*mb).verb_ecode_ptr < next_ecode_3
                                        && (*(*F).ecode as ::core::ffi::c_int
                                            == OP_ALT as ::core::ffi::c_int
                                            || *next_ecode_3 as ::core::ffi::c_int
                                                == OP_ALT as ::core::ffi::c_int)
                                    {
                                        rrc = MATCH_NOMATCH;
                                    }
                                }
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).ecode = (*F).ecode.offset(
                                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            as isize,
                                    );
                                    if !(*(*F).ecode as ::core::ffi::c_int
                                        != OP_ALT as ::core::ffi::c_int)
                                    {
                                        current_block = 5585757815311561815;
                                        break 's_263;
                                    }
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            3 => {
                                if rrc == MATCH_ACCEPT {
                                    memcpy(
                                        &raw mut (*F).ovector as *mut size_t
                                            as *mut ::core::ffi::c_void,
                                        (assert_accept_frame as *mut ::core::ffi::c_char)
                                            .offset(120 as usize as isize)
                                            as *const ::core::ffi::c_void,
                                        (*assert_accept_frame).offset_top.wrapping_mul(
                                            ::core::mem::size_of::<size_t>() as size_t,
                                        ),
                                    );
                                    (*F).offset_top = (*assert_accept_frame).offset_top;
                                    (*F).mark = (*assert_accept_frame).mark;
                                    loop {
                                        (*F).ecode = (*F).ecode.offset(
                                            *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                as isize,
                                        );
                                        if !(*(*F).ecode as ::core::ffi::c_int
                                            == OP_ALT as ::core::ffi::c_int)
                                        {
                                            break;
                                        }
                                    }
                                    (*F).ecode = (*F)
                                        .ecode
                                        .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                                    continue 's_263;
                                } else if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).ecode = (*F).ecode.offset(
                                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            as isize,
                                    );
                                    if !(*(*F).ecode as ::core::ffi::c_int
                                        != OP_ALT as ::core::ffi::c_int)
                                    {
                                        current_block = 14401907612653336132;
                                        break 's_263;
                                    }
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            4 => match rrc {
                                MATCH_ACCEPT | MATCH_MATCH => {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                                MATCH_NOMATCH | MATCH_THEN => {
                                    (*F).ecode = (*F).ecode.offset(
                                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            as isize,
                                    );
                                    if *(*F).ecode as ::core::ffi::c_int
                                        != OP_ALT as ::core::ffi::c_int
                                    {
                                        current_block = 5133818031642813295;
                                        break;
                                    } else {
                                        current_block = 17451096827168202177;
                                        break 's_263;
                                    }
                                }
                                MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
                                    loop {
                                        (*F).ecode = (*F).ecode.offset(
                                            *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                as isize,
                                        );
                                        if !(*(*F).ecode as ::core::ffi::c_int
                                            == OP_ALT as ::core::ffi::c_int)
                                        {
                                            break;
                                        }
                                    }
                                    current_block = 5133818031642813295;
                                    break;
                                }
                                _ => {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            },
                            5 => {
                                match rrc {
                                    MATCH_ACCEPT => {
                                        memcpy(
                                            &raw mut (*F).ovector as *mut size_t
                                                as *mut ::core::ffi::c_void,
                                            (assert_accept_frame as *mut ::core::ffi::c_char)
                                                .offset(120 as usize as isize)
                                                as *const ::core::ffi::c_void,
                                            (*assert_accept_frame).offset_top.wrapping_mul(
                                                ::core::mem::size_of::<size_t>() as size_t,
                                            ),
                                        );
                                        (*F).offset_top = (*assert_accept_frame).offset_top;
                                        current_block = 17100209072374170184;
                                    }
                                    MATCH_MATCH => {
                                        current_block = 17100209072374170184;
                                    }
                                    MATCH_NOMATCH | MATCH_THEN => {
                                        (*F).fields.op_cond.start_branch =
                                            (*F).fields.op_cond.start_branch.offset(
                                                *(*F)
                                                    .fields
                                                    .op_cond
                                                    .start_branch
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            );
                                        if *(*F).fields.op_cond.start_branch as ::core::ffi::c_int
                                            == OP_ALT as ::core::ffi::c_int
                                        {
                                            current_block = 7298563137461813030;
                                            break 's_263;
                                        }
                                        condition = ((*F).byte1 == 0) as ::core::ffi::c_int as BOOL;
                                        current_block = 16759447092282947239;
                                    }
                                    MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
                                        condition = ((*F).byte1 == 0) as ::core::ffi::c_int as BOOL;
                                        current_block = 16759447092282947239;
                                    }
                                    _ => {
                                        rrc = rrc;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                                match current_block {
                                    17100209072374170184 => {
                                        condition = (*F).byte1 as BOOL;
                                    }
                                    _ => {}
                                }
                                if condition != 0 {
                                    loop {
                                        (*F).ecode = (*F).ecode.offset(
                                            *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                as isize,
                                        );
                                        if !(*(*F).ecode as ::core::ffi::c_int
                                            == OP_ALT as ::core::ffi::c_int)
                                        {
                                            break;
                                        }
                                    }
                                }
                                current_block = 16853744556532821084;
                                continue;
                            }
                            6 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).ecode = (*F).ecode.offset(
                                        -(*(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            as isize),
                                    );
                                    continue 's_263;
                                }
                            }
                            7 => {
                                if !(rrc != MATCH_NOMATCH) {
                                    current_block = 6619656240033923038;
                                    continue;
                                }
                                rrc = rrc;
                                current_block = 9215777819543815842;
                                continue;
                            }
                            8 => {
                                if rrc == MATCH_KETRPOS {
                                    (*F).byte1 = TRUE as uint8_t;
                                    if (*F).eptr == (*F).fields.op_brapos.start_eptr {
                                        loop {
                                            (*F).ecode = (*F).ecode.offset(
                                                *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            );
                                            if !(*(*F).ecode as ::core::ffi::c_int
                                                == OP_ALT as ::core::ffi::c_int)
                                            {
                                                break;
                                            }
                                        }
                                    } else {
                                        (*F).ecode = (*F).fields.op_brapos.start_group;
                                        current_block = 14510290228760237064;
                                        break 's_263;
                                    }
                                } else {
                                    if rrc == MATCH_THEN {
                                        let mut next_ecode_2: PCRE2_SPTR16 = (*F).ecode.offset(
                                            *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                as isize,
                                        );
                                        if (*mb).verb_ecode_ptr < next_ecode_2
                                            && (*(*F).ecode as ::core::ffi::c_int
                                                == OP_ALT as ::core::ffi::c_int
                                                || *next_ecode_2 as ::core::ffi::c_int
                                                    == OP_ALT as ::core::ffi::c_int)
                                        {
                                            rrc = MATCH_NOMATCH;
                                        }
                                    }
                                    if rrc != MATCH_NOMATCH {
                                        rrc = rrc;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        (*F).ecode = (*F).ecode.offset(
                                            *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                as isize,
                                        );
                                        if !(*(*F).ecode as ::core::ffi::c_int
                                            != OP_ALT as ::core::ffi::c_int)
                                        {
                                            current_block = 14510290228760237064;
                                            break 's_263;
                                        }
                                    }
                                }
                                if (*F).byte1 as ::core::ffi::c_int != 0
                                    || (*F).byte2 as ::core::ffi::c_int != 0
                                {
                                    (*F).ecode = (*F)
                                        .ecode
                                        .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                                    continue 's_263;
                                } else {
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            9 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    next_ecode = (*F).ecode;
                                    loop {
                                        next_ecode = next_ecode.offset(
                                            *next_ecode.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                as isize,
                                        );
                                        if !(*next_ecode as ::core::ffi::c_int
                                            == OP_ALT as ::core::ffi::c_int)
                                        {
                                            break;
                                        }
                                    }
                                    (*F).ecode = next_ecode
                                        .offset(1 as ::core::ffi::c_int as isize)
                                        .offset(LINK_SIZE as isize);
                                    continue 's_263;
                                }
                            }
                            10 => {
                                if !(rrc != MATCH_NOMATCH) {
                                    continue 's_263;
                                }
                                rrc = rrc;
                                current_block = 9215777819543815842;
                                continue;
                            }
                            11 => {
                                next_ecode_4 = (*F).fields.op_recurse.start_branch.offset(
                                    *(*F)
                                        .fields
                                        .op_recurse
                                        .start_branch
                                        .offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        as isize,
                                );
                                if rrc >= MATCH_BACKTRACK_MIN
                                    && rrc <= MATCH_BACKTRACK_MAX
                                    && (*mb).verb_current_recurse
                                        == (*F).fields.op_recurse.frame_type
                                            ^ GF_RECURSE as uint32_t
                                {
                                    if rrc == MATCH_THEN
                                        && (*mb).verb_ecode_ptr < next_ecode_4
                                        && (*(*F).fields.op_recurse.start_branch
                                            as ::core::ffi::c_int
                                            == OP_ALT as ::core::ffi::c_int
                                            || *next_ecode_4 as ::core::ffi::c_int
                                                == OP_ALT as ::core::ffi::c_int)
                                    {
                                        rrc = MATCH_NOMATCH;
                                    } else {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).fields.op_recurse.start_branch = next_ecode_4;
                                    if !(*(*F).fields.op_recurse.start_branch as ::core::ffi::c_int
                                        != OP_ALT as ::core::ffi::c_int)
                                    {
                                        current_block = 6367621710937907790;
                                        break 's_263;
                                    }
                                    rrc = 0 as ::core::ffi::c_int;
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            12 => {
                                if rrc == MATCH_SKIP_ARG
                                    && _pcre2_strcmp_16(
                                        (*F).ecode.offset(2 as ::core::ffi::c_int as isize),
                                        (*mb).verb_skip_ptr,
                                    ) == 0 as ::core::ffi::c_int
                                {
                                    (*mb).verb_skip_ptr = (*F).eptr;
                                    rrc = -(995 as ::core::ffi::c_int);
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            13 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*mb).verb_current_recurse = (*F).current_recurse;
                                    rrc = -(997 as ::core::ffi::c_int);
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            14 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*mb).verb_current_recurse = (*F).current_recurse;
                                    rrc = -(996 as ::core::ffi::c_int);
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            15 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*mb).verb_current_recurse = (*F).current_recurse;
                                    rrc = -(996 as ::core::ffi::c_int);
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            16 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*mb).verb_skip_ptr = (*F).eptr;
                                    (*mb).verb_current_recurse = (*F).current_recurse;
                                    rrc = -(995 as ::core::ffi::c_int);
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            17 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*mb).verb_skip_ptr =
                                        (*F).ecode.offset(2 as ::core::ffi::c_int as isize);
                                    (*mb).verb_current_recurse = (*F).current_recurse;
                                    rrc = -(994 as ::core::ffi::c_int);
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            18 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*mb).verb_ecode_ptr = (*F).ecode;
                                    (*mb).verb_current_recurse = (*F).current_recurse;
                                    rrc = -(993 as ::core::ffi::c_int);
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            19 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*mb).verb_ecode_ptr = (*F).ecode;
                                    (*mb).verb_current_recurse = (*F).current_recurse;
                                    rrc = -(993 as ::core::ffi::c_int);
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            20 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh187 = (*F).fields.ref_repeat.min;
                                    (*F).fields.ref_repeat.min =
                                        (*F).fields.ref_repeat.min.wrapping_add(1);
                                    if fresh187 >= (*F).fields.ref_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        rrc = match_ref(
                                            (*F).fields.ref_repeat.offset,
                                            (*F).byte1 as BOOL,
                                            (*F).byte2 as ::core::ffi::c_int,
                                            F,
                                            mb,
                                            &raw mut slength_0,
                                        );
                                        if rrc != 0 as ::core::ffi::c_int {
                                            if rrc > 0 as ::core::ffi::c_int {
                                                (*F).eptr = (*mb).end_subject;
                                            }
                                            if (*F).eptr >= (*mb).end_subject {
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && ((*F).eptr > (*mb).start_used_ptr
                                                        || (*mb).allowemptypartial != 0)
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            continue;
                                        } else {
                                            (*F).eptr = (*F).eptr.offset(slength_0 as isize);
                                            current_block = 12869037699417280666;
                                            break 's_263;
                                        }
                                    }
                                }
                            }
                            21 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).eptr =
                                        (*F).eptr.offset(-((*F).fields.ref_repeat.length as isize));
                                    current_block = 7653578622174939700;
                                    continue;
                                }
                            }
                            22 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else if !((*F).eptr == (*F).fields.ref_repeat.start) {
                                    (*F).eptr = (*F).fields.ref_repeat.start;
                                    (*F).fields.ref_repeat.max =
                                        (*F).fields.ref_repeat.max.wrapping_sub(1);
                                    i = (*F).fields.ref_repeat.min;
                                    while i < (*F).fields.ref_repeat.max {
                                        let mut slength_2: size_t = 0;
                                        match_ref(
                                            (*F).fields.ref_repeat.offset,
                                            (*F).byte1 as BOOL,
                                            (*F).byte2 as ::core::ffi::c_int,
                                            F,
                                            mb,
                                            &raw mut slength_2,
                                        );
                                        (*F).eptr = (*F).eptr.offset(slength_2 as isize);
                                        i = i.wrapping_add(1);
                                    }
                                    current_block = 2202530641731595862;
                                    break 's_263;
                                }
                                current_block = 10033017790122999806;
                            }
                            23 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh45 = (*F).fields.class_repeat.min;
                                    (*F).fields.class_repeat.min =
                                        (*F).fields.class_repeat.min.wrapping_add(1);
                                    if fresh45 >= (*F).fields.class_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh46 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh46 as uint32_t;
                                        if fc > 255 as uint32_t {
                                            if !((*F).op as ::core::ffi::c_int
                                                == OP_CLASS as ::core::ffi::c_int)
                                            {
                                                current_block = 4519209830585050666;
                                                break 's_263;
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            continue;
                                        } else {
                                            if !(*((*F).fields.class_repeat.byte_map_address
                                                as *const ::core::ffi::c_uchar)
                                                .offset(fc.wrapping_div(8 as uint32_t) as isize)
                                                as ::core::ffi::c_uint
                                                & (1 as ::core::ffi::c_uint)
                                                    << (fc & 7 as uint32_t)
                                                == 0 as ::core::ffi::c_uint)
                                            {
                                                current_block = 4519209830585050666;
                                                break 's_263;
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            continue;
                                        }
                                    }
                                }
                            }
                            24 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    current_block = 15840449704302887135;
                                    continue;
                                }
                            }
                            25 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh17 = (*F).fields.char_repeat.min;
                                    (*F).fields.char_repeat.min =
                                        (*F).fields.char_repeat.min.wrapping_add(1);
                                    if fresh17 >= (*F).fields.char_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        cc_2 = *(*F).eptr as uint32_t;
                                        if (*F).fields.char_repeat.c != cc_2
                                            && (*F).fields.char_repeat.oc.oc != cc_2
                                        {
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            continue;
                                        } else {
                                            (*F).eptr = (*F).eptr.offset(1);
                                            current_block = 5981395450828152283;
                                            break 's_263;
                                        }
                                    }
                                }
                            }
                            26 => {
                                (*F).eptr = (*F).eptr.offset(-1);
                                if !(rrc != MATCH_NOMATCH) {
                                    current_block = 9522675792805490914;
                                    continue;
                                }
                                rrc = rrc;
                                current_block = 9215777819543815842;
                                continue;
                            }
                            27 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh19 = (*F).fields.char_repeat.min;
                                    (*F).fields.char_repeat.min =
                                        (*F).fields.char_repeat.min.wrapping_add(1);
                                    if fresh19 >= (*F).fields.char_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh20 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        if !((*F).fields.char_repeat.c != *fresh20 as uint32_t) {
                                            current_block = 6021810480259369443;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            28 => {
                                (*F).eptr = (*F).eptr.offset(-1);
                                if !(rrc != MATCH_NOMATCH) {
                                    current_block = 2574904801665210416;
                                    continue;
                                }
                                rrc = rrc;
                                current_block = 9215777819543815842;
                                continue;
                            }
                            29 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh29 = (*F).fields.charnot_repeat.min;
                                    (*F).fields.charnot_repeat.min =
                                        (*F).fields.charnot_repeat.min.wrapping_add(1);
                                    if fresh29 >= (*F).fields.charnot_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).fields.charnot_repeat.c == *(*F).eptr as uint32_t
                                        || (*F).fields.charnot_repeat.oc == *(*F).eptr as uint32_t
                                    {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        (*F).eptr = (*F).eptr.offset(1);
                                        current_block = 9699707990742192723;
                                        break 's_263;
                                    }
                                }
                            }
                            30 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    current_block = 12956711689462799609;
                                    continue;
                                }
                            }
                            31 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh36 = (*F).fields.charnot_repeat.min;
                                    (*F).fields.charnot_repeat.min =
                                        (*F).fields.charnot_repeat.min.wrapping_add(1);
                                    if fresh36 >= (*F).fields.charnot_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh37 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        if !((*F).fields.charnot_repeat.c == *fresh37 as uint32_t) {
                                            current_block = 10082661835832785913;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            32 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    current_block = 4187073793469983967;
                                    continue;
                                }
                            }
                            33 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh180 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh180 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).fields.type_repeat.ctype
                                        == OP_ANY as ::core::ffi::c_int as uint32_t
                                        && (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                            ((*F).eptr < (*mb).end_subject
                                                && _pcre2_is_newline_16(
                                                    (*F).eptr,
                                                    (*mb).nltype,
                                                    (*mb).end_subject,
                                                    &raw mut (*mb).nllen,
                                                    utf,
                                                ) != 0)
                                                as ::core::ffi::c_int
                                        } else {
                                            ((*F).eptr
                                                <= (*mb)
                                                    .end_subject
                                                    .offset(-((*mb).nllen as isize))
                                                && *(*F).eptr as ::core::ffi::c_int
                                                    == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                        as ::core::ffi::c_int
                                                && ((*mb).nllen == 1 as uint32_t
                                                    || *(*F)
                                                        .eptr
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                        == (*mb).nl
                                                            [1 as ::core::ffi::c_int as usize]
                                                            as ::core::ffi::c_int))
                                                as ::core::ffi::c_int
                                        }) != 0
                                    {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh181 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh181 as uint32_t;
                                        match (*F).fields.type_repeat.ctype {
                                            12 => {
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && (*F).eptr >= (*mb).end_subject
                                                    && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                                    && (*mb).nllen == 2 as uint32_t
                                                    && fc
                                                        == (*mb).nl
                                                            [0 as ::core::ffi::c_int as usize]
                                                            as uint32_t
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                                current_block = 51091121742473818;
                                                break 's_263;
                                            }
                                            13 | 14 => {
                                                current_block = 51091121742473818;
                                                break 's_263;
                                            }
                                            17 => match fc {
                                                13 => {
                                                    if (*F).eptr < (*mb).end_subject
                                                        && *(*F).eptr as ::core::ffi::c_int
                                                            == CHAR_LF
                                                    {
                                                        (*F).eptr = (*F).eptr.offset(1);
                                                    }
                                                    current_block = 51091121742473818;
                                                    break 's_263;
                                                }
                                                10 => {
                                                    current_block = 51091121742473818;
                                                    break 's_263;
                                                }
                                                11 | 12 | 133 | 8232 | 8233 => {
                                                    if !((*mb).bsr_convention as ::core::ffi::c_int
                                                        == PCRE2_BSR_ANYCRLF)
                                                    {
                                                        current_block = 51091121742473818;
                                                        break 's_263;
                                                    }
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    continue;
                                                }
                                                _ => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    continue;
                                                }
                                            },
                                            18 => {
                                                match fc {
                                                    9 | 32 | 160 | 5760 | 6158 | 8192 | 8193
                                                    | 8194 | 8195 | 8196 | 8197 | 8198 | 8199
                                                    | 8200 | 8201 | 8202 | 8239 | 8287 | 12288 => {}
                                                    _ => {
                                                        current_block = 51091121742473818;
                                                        break 's_263;
                                                    }
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            19 => {
                                                match fc {
                                                    9 | 32 | 160 | 5760 | 6158 | 8192 | 8193
                                                    | 8194 | 8195 | 8196 | 8197 | 8198 | 8199
                                                    | 8200 | 8201 | 8202 | 8239 | 8287 | 12288 => {
                                                        current_block = 51091121742473818;
                                                        break 's_263;
                                                    }
                                                    _ => {}
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            20 => {
                                                match fc {
                                                    10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {}
                                                    _ => {
                                                        current_block = 51091121742473818;
                                                        break 's_263;
                                                    }
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            21 => {
                                                match fc {
                                                    10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                                        current_block = 51091121742473818;
                                                        break 's_263;
                                                    }
                                                    _ => {}
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            6 => {
                                                if !(fc <= 255 as uint32_t
                                                    && *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_digit
                                                        != 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 51091121742473818;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            7 => {
                                                if !(!(fc <= 255 as uint32_t)
                                                    || *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_digit
                                                        == 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 51091121742473818;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            8 => {
                                                if !(fc <= 255 as uint32_t
                                                    && *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_space
                                                        != 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 51091121742473818;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            9 => {
                                                if !(!(fc <= 255 as uint32_t)
                                                    || *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_space
                                                        == 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 51091121742473818;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            10 => {
                                                if !(fc <= 255 as uint32_t
                                                    && *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_word
                                                        != 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 51091121742473818;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            11 => {
                                                if !(!(fc <= 255 as uint32_t)
                                                    || *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_word
                                                        == 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 51091121742473818;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            _ => return PCRE2_ERROR_INTERNAL,
                                        }
                                    }
                                }
                            }
                            34 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    if (*F).fields.type_repeat.ctype
                                        == OP_ANYNL as ::core::ffi::c_int as uint32_t
                                        && (*F).eptr > (*F).fields.type_repeat.start_eptr
                                        && *(*F).eptr as ::core::ffi::c_int == CHAR_LF
                                        && *(*F).eptr.offset(-(1 as ::core::ffi::c_int) as isize)
                                            as ::core::ffi::c_int
                                            == CHAR_CR
                                    {
                                        (*F).eptr = (*F).eptr.offset(-1);
                                    }
                                    current_block = 981804248182727996;
                                    continue;
                                }
                            }
                            35 => {
                                rrc = rrc;
                                current_block = 9215777819543815842;
                                continue;
                            }
                            36 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*mb).verb_current_recurse = (*F).current_recurse;
                                    rrc = -(997 as ::core::ffi::c_int);
                                    current_block = 9215777819543815842;
                                    continue;
                                }
                            }
                            37 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh190 = (*F).fields.op_vreverse.max;
                                    (*F).fields.op_vreverse.max =
                                        (*F).fields.op_vreverse.max.wrapping_sub(1);
                                    if fresh190 <= (*F).fields.op_vreverse.min {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        (*F).eptr = (*F).eptr.offset(1);
                                        if utf != 0 {
                                            if (*F).eptr < (*mb).end_subject
                                                && *(*F).eptr as ::core::ffi::c_uint
                                                    & 0xfc00 as ::core::ffi::c_uint
                                                    == 0xdc00 as ::core::ffi::c_uint
                                            {
                                                (*F).eptr = (*F).eptr.offset(1);
                                            }
                                        }
                                        current_block = 12225199092044798022;
                                        break 's_263;
                                    }
                                }
                            }
                            38 => {
                                if rrc == MATCH_ACCEPT {
                                    memcpy(
                                        &raw mut (*F).ovector as *mut size_t
                                            as *mut ::core::ffi::c_void,
                                        (assert_accept_frame as *mut ::core::ffi::c_char)
                                            .offset(120 as usize as isize)
                                            as *const ::core::ffi::c_void,
                                        (*assert_accept_frame).offset_top.wrapping_mul(
                                            ::core::mem::size_of::<size_t>() as size_t,
                                        ),
                                    );
                                    (*F).offset_top = (*assert_accept_frame).offset_top;
                                    (*F).mark = (*assert_accept_frame).mark;
                                    (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
                                    (*mb).true_end_subject = (*mb)
                                        .end_subject
                                        .offset((*F).fields.op_assert_scs.true_end_extra as isize);
                                    (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
                                    loop {
                                        (*F).ecode = (*F).ecode.offset(
                                            *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                as isize,
                                        );
                                        if !(*(*F).ecode as ::core::ffi::c_int
                                            == OP_ALT as ::core::ffi::c_int)
                                        {
                                            break;
                                        }
                                    }
                                    (*F).ecode = (*F)
                                        .ecode
                                        .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                                    (*F).eptr = (*F).fields.op_assert_scs.saved_eptr;
                                    continue 's_263;
                                } else if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
                                    (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
                                    (*mb).true_end_subject = (*mb)
                                        .end_subject
                                        .offset((*F).fields.op_assert_scs.true_end_extra as isize);
                                    (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).ecode = (*F).ecode.offset(
                                        *(*F).ecode.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            as isize,
                                    );
                                    if *(*F).ecode as ::core::ffi::c_int
                                        != OP_ALT as ::core::ffi::c_int
                                    {
                                        (*mb).end_subject =
                                            (*F).fields.op_assert_scs.saved_end_subject;
                                        (*mb).true_end_subject = (*mb).end_subject.offset(
                                            (*F).fields.op_assert_scs.true_end_extra as isize,
                                        );
                                        (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        length = 0 as size_t;
                                        current_block = 6829137806597638593;
                                        break 's_263;
                                    }
                                }
                            }
                            39 => {
                                (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
                                (*mb).true_end_subject = (*mb).end_subject;
                                rrc = rrc;
                                current_block = 9215777819543815842;
                                continue;
                            }
                            100 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh51 = (*F).fields.xclass_repeat.min;
                                    (*F).fields.xclass_repeat.min =
                                        (*F).fields.xclass_repeat.min.wrapping_add(1);
                                    if fresh51 >= (*F).fields.xclass_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh52 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh52 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh53 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh53 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        if !(_pcre2_xclass_16(
                                            fc,
                                            (*F).fields.xclass_repeat.xclass_data,
                                            (*mb).start_code as *const uint8_t,
                                            utf,
                                        ) == 0)
                                        {
                                            current_block = 3302068592069234587;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            101 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh54 = (*F).eptr;
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    if fresh54 <= (*F).fields.xclass_repeat.start_eptr {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        if utf != 0 {
                                            if *(*F).eptr as ::core::ffi::c_uint
                                                & 0xfc00 as ::core::ffi::c_uint
                                                == 0xdc00 as ::core::ffi::c_uint
                                            {
                                                (*F).eptr = (*F).eptr.offset(-1);
                                            }
                                        }
                                        current_block = 17975895912995949440;
                                        break 's_263;
                                    }
                                }
                            }
                            102 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh58 = (*F).fields.eclass_repeat.min;
                                    (*F).fields.eclass_repeat.min =
                                        (*F).fields.eclass_repeat.min.wrapping_add(1);
                                    if fresh58 >= (*F).fields.eclass_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh59 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh59 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh60 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh60 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        if !(_pcre2_eclass_16(
                                            fc,
                                            (*F).fields.eclass_repeat.eclass_data,
                                            (*F).fields.eclass_repeat.eclass_data.offset(
                                                (*F).fields.eclass_repeat.eclass_len as isize,
                                            ),
                                            (*mb).start_code as *const uint8_t,
                                            utf,
                                        ) == 0)
                                        {
                                            current_block = 6265774175505686097;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            103 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh61 = (*F).eptr;
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    if fresh61 <= (*F).fields.eclass_repeat.start_eptr {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        if utf != 0 {
                                            if *(*F).eptr as ::core::ffi::c_uint
                                                & 0xfc00 as ::core::ffi::c_uint
                                                == 0xdc00 as ::core::ffi::c_uint
                                            {
                                                (*F).eptr = (*F).eptr.offset(-1);
                                            }
                                        }
                                        current_block = 12574270518964373171;
                                        break 's_263;
                                    }
                                }
                            }
                            200 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh42 = (*F).fields.class_repeat.min;
                                    (*F).fields.class_repeat.min =
                                        (*F).fields.class_repeat.min.wrapping_add(1);
                                    if fresh42 >= (*F).fields.class_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh43 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh43 as uint32_t;
                                        if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                            let fresh44 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh44 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        if fc > 255 as uint32_t {
                                            if !((*F).op as ::core::ffi::c_int
                                                == OP_CLASS as ::core::ffi::c_int)
                                            {
                                                current_block = 10553790310514340151;
                                                break 's_263;
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            continue;
                                        } else {
                                            if !(*((*F).fields.class_repeat.byte_map_address
                                                as *const ::core::ffi::c_uchar)
                                                .offset(fc.wrapping_div(8 as uint32_t) as isize)
                                                as ::core::ffi::c_uint
                                                & (1 as ::core::ffi::c_uint)
                                                    << (fc & 7 as uint32_t)
                                                == 0 as ::core::ffi::c_uint)
                                            {
                                                current_block = 10553790310514340151;
                                                break 's_263;
                                            }
                                            rrc = 0 as ::core::ffi::c_int;
                                            current_block = 9215777819543815842;
                                            continue;
                                        }
                                    }
                                }
                            }
                            201 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh47 = (*F).eptr;
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    if !(fresh47 <= (*F).fields.class_repeat.start_eptr) {
                                        if *(*F).eptr as ::core::ffi::c_uint
                                            & 0xfc00 as ::core::ffi::c_uint
                                            == 0xdc00 as ::core::ffi::c_uint
                                        {
                                            (*F).eptr = (*F).eptr.offset(-1);
                                        }
                                        current_block = 13776980873676388703;
                                        break 's_263;
                                    }
                                }
                                current_block = 10848253239906386529;
                            }
                            202 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh15 = (*F).fields.char_repeat.min;
                                    (*F).fields.char_repeat.min =
                                        (*F).fields.char_repeat.min.wrapping_add(1);
                                    if fresh15 >= (*F).fields.char_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr
                                        <= (*mb)
                                            .end_subject
                                            .offset(-((*F).byte1 as ::core::ffi::c_int as isize))
                                        && memcmp(
                                            (*F).eptr as *const ::core::ffi::c_void,
                                            (*F).fields.char_repeat.charptr
                                                as *const ::core::ffi::c_void,
                                            ((*F).byte1 as ::core::ffi::c_int
                                                * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int))
                                                as size_t,
                                        ) == 0 as ::core::ffi::c_int
                                    {
                                        (*F).eptr = (*F)
                                            .eptr
                                            .offset((*F).byte1 as ::core::ffi::c_int as isize);
                                        current_block = 1391843528280232292;
                                        break 's_263;
                                    } else if (*F).byte2 as ::core::ffi::c_int
                                        > 0 as ::core::ffi::c_int
                                        && (*F).eptr
                                            <= (*mb).end_subject.offset(
                                                -((*F).byte2 as ::core::ffi::c_int as isize),
                                            )
                                        && memcmp(
                                            (*F).eptr as *const ::core::ffi::c_void,
                                            &raw mut (*F).fields.char_repeat.oc.occu
                                                as *mut PCRE2_UCHAR16
                                                as *const ::core::ffi::c_void,
                                            ((*F).byte2 as ::core::ffi::c_int
                                                * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int))
                                                as size_t,
                                        ) == 0 as ::core::ffi::c_int
                                    {
                                        (*F).eptr = (*F)
                                            .eptr
                                            .offset((*F).byte2 as ::core::ffi::c_int as isize);
                                        current_block = 1391843528280232292;
                                        break 's_263;
                                    } else {
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            203 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    if *(*F).eptr as ::core::ffi::c_uint
                                        & 0xfc00 as ::core::ffi::c_uint
                                        == 0xdc00 as ::core::ffi::c_uint
                                    {
                                        (*F).eptr = (*F).eptr.offset(-1);
                                    }
                                    current_block = 9029608189573075810;
                                    continue;
                                }
                            }
                            204 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh26 = (*F).fields.charnot_repeat.min;
                                    (*F).fields.charnot_repeat.min =
                                        (*F).fields.charnot_repeat.min.wrapping_add(1);
                                    if fresh26 >= (*F).fields.charnot_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh27 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        d_0 = *fresh27 as uint32_t;
                                        if d_0 & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                            let fresh28 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            d_0 = ((d_0 & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh28 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        if !((*F).fields.charnot_repeat.c == d_0
                                            || (*F).fields.charnot_repeat.oc == d_0)
                                        {
                                            current_block = 15467235608407556447;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            205 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    if *(*F).eptr as ::core::ffi::c_uint
                                        & 0xfc00 as ::core::ffi::c_uint
                                        == 0xdc00 as ::core::ffi::c_uint
                                    {
                                        (*F).eptr = (*F).eptr.offset(-1);
                                    }
                                    current_block = 3660592233603849974;
                                    continue;
                                }
                            }
                            206 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh33 = (*F).fields.charnot_repeat.min;
                                    (*F).fields.charnot_repeat.min =
                                        (*F).fields.charnot_repeat.min.wrapping_add(1);
                                    if fresh33 >= (*F).fields.charnot_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh34 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        d_3 = *fresh34 as uint32_t;
                                        if d_3 & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                            let fresh35 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            d_3 = ((d_3 & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh35 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        if !((*F).fields.charnot_repeat.c == d_3) {
                                            current_block = 11194422880361134385;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            207 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    if *(*F).eptr as ::core::ffi::c_uint
                                        & 0xfc00 as ::core::ffi::c_uint
                                        == 0xdc00 as ::core::ffi::c_uint
                                    {
                                        (*F).eptr = (*F).eptr.offset(-1);
                                    }
                                    current_block = 11364986971110899152;
                                    continue;
                                }
                            }
                            208 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh137 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh137 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh138 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh138 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh139 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh139 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        chartype_2 = (*(&raw const _pcre2_ucd_records_16
                                            as *const ucd_record)
                                            .offset(
                                                *(&raw const _pcre2_ucd_stage2_16
                                                    as *const uint16_t)
                                                    .offset(
                                                        (*(&raw const _pcre2_ucd_stage1_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                (fc as ::core::ffi::c_int
                                                                    / UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            * UCD_BLOCK_SIZE
                                                            + fc as ::core::ffi::c_int
                                                                % UCD_BLOCK_SIZE)
                                                            as isize,
                                                    )
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            ))
                                        .chartype
                                            as ::core::ffi::c_int;
                                        if !((chartype_2 == ucp_Lu as ::core::ffi::c_int
                                            || chartype_2 == ucp_Ll as ::core::ffi::c_int
                                            || chartype_2 == ucp_Lt as ::core::ffi::c_int)
                                            as ::core::ffi::c_int
                                            == ((*F).fields.type_repeat.ctype
                                                == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int)
                                        {
                                            current_block = 7583650632015690588;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            209 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh140 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh140 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh141 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh141 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh142 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh142 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        if !((*(&raw const _pcre2_ucp_gentype_16
                                            as *const uint32_t)
                                            .offset(
                                                (*(&raw const _pcre2_ucd_records_16
                                                    as *const ucd_record)
                                                    .offset(
                                                        *(&raw const _pcre2_ucd_stage2_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                (*(&raw const _pcre2_ucd_stage1_16
                                                                    as *const uint16_t)
                                                                    .offset(
                                                                        (fc as ::core::ffi::c_int
                                                                            / UCD_BLOCK_SIZE)
                                                                            as isize,
                                                                    )
                                                                    as ::core::ffi::c_int
                                                                    * UCD_BLOCK_SIZE
                                                                    + fc as ::core::ffi::c_int
                                                                        % UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    ))
                                                .chartype
                                                    as isize,
                                            )
                                            == (*F).fields.type_repeat.propvalue)
                                            as ::core::ffi::c_int
                                            == ((*F).fields.type_repeat.ctype
                                                == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int)
                                        {
                                            current_block = 1724909796719861406;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            210 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh143 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh143 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh144 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh144 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh145 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh145 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        if !(((*(&raw const _pcre2_ucd_records_16
                                            as *const ucd_record)
                                            .offset(
                                                *(&raw const _pcre2_ucd_stage2_16
                                                    as *const uint16_t)
                                                    .offset(
                                                        (*(&raw const _pcre2_ucd_stage1_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                (fc as ::core::ffi::c_int
                                                                    / UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            * UCD_BLOCK_SIZE
                                                            + fc as ::core::ffi::c_int
                                                                % UCD_BLOCK_SIZE)
                                                            as isize,
                                                    )
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            ))
                                        .chartype
                                            as uint32_t
                                            == (*F).fields.type_repeat.propvalue)
                                            as ::core::ffi::c_int
                                            == ((*F).fields.type_repeat.ctype
                                                == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int)
                                        {
                                            current_block = 15506043489740435919;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            211 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh146 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh146 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh147 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh147 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh148 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh148 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        if !(((*(&raw const _pcre2_ucd_records_16
                                            as *const ucd_record)
                                            .offset(
                                                *(&raw const _pcre2_ucd_stage2_16
                                                    as *const uint16_t)
                                                    .offset(
                                                        (*(&raw const _pcre2_ucd_stage1_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                (fc as ::core::ffi::c_int
                                                                    / UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            * UCD_BLOCK_SIZE
                                                            + fc as ::core::ffi::c_int
                                                                % UCD_BLOCK_SIZE)
                                                            as isize,
                                                    )
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            ))
                                        .script
                                            as uint32_t
                                            == (*F).fields.type_repeat.propvalue)
                                            as ::core::ffi::c_int
                                            == ((*F).fields.type_repeat.ctype
                                                == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int)
                                        {
                                            current_block = 2403859285966545931;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            212 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh152 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh152 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh153 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh153 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh154 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh154 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        category_1 = *(&raw const _pcre2_ucp_gentype_16
                                            as *const uint32_t)
                                            .offset(
                                                (*(&raw const _pcre2_ucd_records_16
                                                    as *const ucd_record)
                                                    .offset(
                                                        *(&raw const _pcre2_ucd_stage2_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                (*(&raw const _pcre2_ucd_stage1_16
                                                                    as *const uint16_t)
                                                                    .offset(
                                                                        (fc as ::core::ffi::c_int
                                                                            / UCD_BLOCK_SIZE)
                                                                            as isize,
                                                                    )
                                                                    as ::core::ffi::c_int
                                                                    * UCD_BLOCK_SIZE
                                                                    + fc as ::core::ffi::c_int
                                                                        % UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    ))
                                                .chartype
                                                    as isize,
                                            )
                                            as ::core::ffi::c_int;
                                        if !((category_1 == ucp_L as ::core::ffi::c_int
                                            || category_1 == ucp_N as ::core::ffi::c_int)
                                            as ::core::ffi::c_int
                                            == ((*F).fields.type_repeat.ctype
                                                == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int)
                                        {
                                            current_block = 11093113080711097189;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            213 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh155 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh155 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh156 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh156 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh157 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh157 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        match fc {
                                            9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                            | 8195 | 8196 | 8197 | 8198 | 8199 | 8200 | 8201
                                            | 8202 | 8239 | 8287 | 12288 | 10 | 11 | 12 | 13
                                            | 133 | 8232 | 8233 => {
                                                if !((*F).fields.type_repeat.ctype
                                                    == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                {
                                                    current_block = 15032463550852564264;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            _ => {
                                                if !((*(&raw const _pcre2_ucp_gentype_16 as *const uint32_t)
                                                    .offset(
                                                        (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                                            .offset(
                                                                *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                                    .offset(
                                                                        (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                                                            .offset(
                                                                                (fc as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                            ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                            + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                    ) as ::core::ffi::c_int as isize,
                                                            ))
                                                            .chartype as isize,
                                                    ) == ucp_Z as ::core::ffi::c_int as uint32_t)
                                                    as ::core::ffi::c_int
                                                    == ((*F).fields.type_repeat.ctype
                                                        == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                        as ::core::ffi::c_int)
                                                {
                                                    current_block = 15032463550852564264;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }
                            214 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh158 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh158 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh159 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh159 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh160 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh160 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        chartype_3 = (*(&raw const _pcre2_ucd_records_16
                                            as *const ucd_record)
                                            .offset(
                                                *(&raw const _pcre2_ucd_stage2_16
                                                    as *const uint16_t)
                                                    .offset(
                                                        (*(&raw const _pcre2_ucd_stage1_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                (fc as ::core::ffi::c_int
                                                                    / UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            * UCD_BLOCK_SIZE
                                                            + fc as ::core::ffi::c_int
                                                                % UCD_BLOCK_SIZE)
                                                            as isize,
                                                    )
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            ))
                                        .chartype
                                            as ::core::ffi::c_int;
                                        category_2 = *(&raw const _pcre2_ucp_gentype_16
                                            as *const uint32_t)
                                            .offset(chartype_3 as isize)
                                            as ::core::ffi::c_int;
                                        if !((category_2 == ucp_L as ::core::ffi::c_int
                                            || category_2 == ucp_N as ::core::ffi::c_int
                                            || chartype_3 == ucp_Mn as ::core::ffi::c_int
                                            || chartype_3 == ucp_Pc as ::core::ffi::c_int)
                                            as ::core::ffi::c_int
                                            == ((*F).fields.type_repeat.ctype
                                                == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int)
                                        {
                                            current_block = 3420986181356424615;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            215 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh161 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh161 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh162 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh162 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh163 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh163 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        cp_1 = (&raw const _pcre2_ucd_caseless_sets_16
                                            as *const uint32_t)
                                            .offset((*F).fields.type_repeat.propvalue as isize);
                                        loop {
                                            if fc < *cp_1 {
                                                if (*F).fields.type_repeat.ctype
                                                    == OP_NOTPROP as ::core::ffi::c_int as uint32_t
                                                {
                                                    current_block = 6407021218369142540;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue '_RETURN_SWITCH;
                                            } else {
                                                let fresh164 = cp_1;
                                                cp_1 = cp_1.offset(1);
                                                if !(fc == *fresh164) {
                                                    continue;
                                                }
                                                if !((*F).fields.type_repeat.ctype
                                                    == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                {
                                                    current_block = 6407021218369142540;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue '_RETURN_SWITCH;
                                            }
                                        }
                                    }
                                }
                            }
                            216 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh165 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh165 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh166 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh166 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh167 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh167 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        if !((fc == CHAR_DOLLAR_SIGN as uint32_t
                                            || fc == CHAR_COMMERCIAL_AT as uint32_t
                                            || fc == CHAR_GRAVE_ACCENT as uint32_t
                                            || fc >= 0xa0 as uint32_t && fc <= 0xd7ff as uint32_t
                                            || fc >= 0xe000 as uint32_t)
                                            as ::core::ffi::c_int
                                            == ((*F).fields.type_repeat.ctype
                                                == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int)
                                        {
                                            current_block = 3573618632796085661;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            217 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh174 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh174 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh175 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh175 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh176 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh176 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        (*F).eptr = _pcre2_extuni_16(
                                            fc,
                                            (*F).eptr,
                                            (*mb).check_subject,
                                            (*mb).end_subject,
                                            utf,
                                            ::core::ptr::null_mut::<::core::ffi::c_int>(),
                                        );
                                        if (*F).eptr >= (*mb).end_subject {
                                            if (*mb).partial as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int
                                                && ((*F).eptr > (*mb).start_used_ptr
                                                    || (*mb).allowemptypartial != 0)
                                            {
                                                (*mb).hitend = TRUE as BOOL;
                                                if (*mb).partial as ::core::ffi::c_int
                                                    > 1 as ::core::ffi::c_int
                                                {
                                                    return PCRE2_ERROR_PARTIAL;
                                                }
                                            }
                                        }
                                        current_block = 7680255065025725177;
                                        break 's_263;
                                    }
                                }
                            }
                            218 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh177 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh177 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).fields.type_repeat.ctype
                                        == OP_ANY as ::core::ffi::c_int as uint32_t
                                        && (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                            ((*F).eptr < (*mb).end_subject
                                                && _pcre2_is_newline_16(
                                                    (*F).eptr,
                                                    (*mb).nltype,
                                                    (*mb).end_subject,
                                                    &raw mut (*mb).nllen,
                                                    utf,
                                                ) != 0)
                                                as ::core::ffi::c_int
                                        } else {
                                            ((*F).eptr
                                                <= (*mb)
                                                    .end_subject
                                                    .offset(-((*mb).nllen as isize))
                                                && *(*F).eptr as ::core::ffi::c_int
                                                    == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                        as ::core::ffi::c_int
                                                && ((*mb).nllen == 1 as uint32_t
                                                    || *(*F)
                                                        .eptr
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                        == (*mb).nl
                                                            [1 as ::core::ffi::c_int as usize]
                                                            as ::core::ffi::c_int))
                                                as ::core::ffi::c_int
                                        }) != 0
                                    {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh178 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh178 as uint32_t;
                                        if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                            let fresh179 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh179 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        match (*F).fields.type_repeat.ctype {
                                            12 => {
                                                if (*mb).partial as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                                    && (*F).eptr >= (*mb).end_subject
                                                    && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                                    && (*mb).nllen == 2 as uint32_t
                                                    && fc
                                                        == (*mb).nl
                                                            [0 as ::core::ffi::c_int as usize]
                                                            as uint32_t
                                                {
                                                    (*mb).hitend = TRUE as BOOL;
                                                    if (*mb).partial as ::core::ffi::c_int
                                                        > 1 as ::core::ffi::c_int
                                                    {
                                                        return PCRE2_ERROR_PARTIAL;
                                                    }
                                                }
                                                current_block = 10114015117503592819;
                                                break 's_263;
                                            }
                                            13 | 14 => {
                                                current_block = 10114015117503592819;
                                                break 's_263;
                                            }
                                            17 => match fc {
                                                13 => {
                                                    if (*F).eptr < (*mb).end_subject
                                                        && *(*F).eptr as ::core::ffi::c_int
                                                            == CHAR_LF
                                                    {
                                                        (*F).eptr = (*F).eptr.offset(1);
                                                    }
                                                    current_block = 10114015117503592819;
                                                    break 's_263;
                                                }
                                                10 => {
                                                    current_block = 10114015117503592819;
                                                    break 's_263;
                                                }
                                                11 | 12 | 133 | 8232 | 8233 => {
                                                    if !((*mb).bsr_convention as ::core::ffi::c_int
                                                        == PCRE2_BSR_ANYCRLF)
                                                    {
                                                        current_block = 10114015117503592819;
                                                        break 's_263;
                                                    }
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    continue;
                                                }
                                                _ => {
                                                    rrc = 0 as ::core::ffi::c_int;
                                                    current_block = 9215777819543815842;
                                                    continue;
                                                }
                                            },
                                            18 => {
                                                match fc {
                                                    9 | 32 | 160 | 5760 | 6158 | 8192 | 8193
                                                    | 8194 | 8195 | 8196 | 8197 | 8198 | 8199
                                                    | 8200 | 8201 | 8202 | 8239 | 8287 | 12288 => {}
                                                    _ => {
                                                        current_block = 10114015117503592819;
                                                        break 's_263;
                                                    }
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            19 => {
                                                match fc {
                                                    9 | 32 | 160 | 5760 | 6158 | 8192 | 8193
                                                    | 8194 | 8195 | 8196 | 8197 | 8198 | 8199
                                                    | 8200 | 8201 | 8202 | 8239 | 8287 | 12288 => {
                                                        current_block = 10114015117503592819;
                                                        break 's_263;
                                                    }
                                                    _ => {}
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            20 => {
                                                match fc {
                                                    10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {}
                                                    _ => {
                                                        current_block = 10114015117503592819;
                                                        break 's_263;
                                                    }
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            21 => {
                                                match fc {
                                                    10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                                        current_block = 10114015117503592819;
                                                        break 's_263;
                                                    }
                                                    _ => {}
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            6 => {
                                                if !(fc < 256 as uint32_t
                                                    && *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_digit
                                                        != 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 10114015117503592819;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            7 => {
                                                if !(fc >= 256 as uint32_t
                                                    || *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_digit
                                                        == 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 10114015117503592819;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            8 => {
                                                if !(fc < 256 as uint32_t
                                                    && *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_space
                                                        != 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 10114015117503592819;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            9 => {
                                                if !(fc >= 256 as uint32_t
                                                    || *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_space
                                                        == 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 10114015117503592819;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            10 => {
                                                if !(fc < 256 as uint32_t
                                                    && *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_word
                                                        != 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 10114015117503592819;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            11 => {
                                                if !(fc >= 256 as uint32_t
                                                    || *(*mb).ctypes.offset(fc as isize)
                                                        as ::core::ffi::c_int
                                                        & ctype_word
                                                        == 0 as ::core::ffi::c_int)
                                                {
                                                    current_block = 10114015117503592819;
                                                    break 's_263;
                                                }
                                                rrc = 0 as ::core::ffi::c_int;
                                                current_block = 9215777819543815842;
                                                continue;
                                            }
                                            _ => return PCRE2_ERROR_INTERNAL,
                                        }
                                    }
                                }
                            }
                            219 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    if utf == 0 {
                                        fc = *(*F).eptr as uint32_t;
                                    } else {
                                        if *(*F).eptr as ::core::ffi::c_uint
                                            & 0xfc00 as ::core::ffi::c_uint
                                            == 0xdc00 as ::core::ffi::c_uint
                                        {
                                            (*F).eptr = (*F).eptr.offset(-1);
                                        }
                                        fc = *(*F).eptr as uint32_t;
                                        if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *(*F)
                                                    .eptr
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                    }
                                    rgb = (*(&raw const _pcre2_ucd_records_16 as *const ucd_record)
                                        .offset(
                                            *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t)
                                                .offset(
                                                    (*(&raw const _pcre2_ucd_stage1_16
                                                        as *const uint16_t)
                                                        .offset(
                                                            (fc as ::core::ffi::c_int
                                                                / UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        * UCD_BLOCK_SIZE
                                                        + fc as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                                        as isize,
                                                )
                                                as ::core::ffi::c_int
                                                as isize,
                                        ))
                                    .gbprop
                                        as ::core::ffi::c_int;
                                    while !((*F).eptr <= (*F).fields.type_repeat.start_eptr) {
                                        fptr =
                                            (*F).eptr.offset(-(1 as ::core::ffi::c_int as isize));
                                        if utf == 0 {
                                            fc = *fptr as uint32_t;
                                        } else {
                                            if *fptr as ::core::ffi::c_uint
                                                & 0xfc00 as ::core::ffi::c_uint
                                                == 0xdc00 as ::core::ffi::c_uint
                                            {
                                                fptr = fptr.offset(-1);
                                            }
                                            fc = *fptr as uint32_t;
                                            if fc & 0xfc00 as uint32_t == 0xd800 as uint32_t {
                                                fc = ((fc & 0x3ff as uint32_t)
                                                    << 10 as ::core::ffi::c_int
                                                    | *fptr.offset(1 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3ff as uint32_t)
                                                    .wrapping_add(0x10000 as uint32_t);
                                            }
                                        }
                                        lgb = (*(&raw const _pcre2_ucd_records_16
                                            as *const ucd_record)
                                            .offset(
                                                *(&raw const _pcre2_ucd_stage2_16
                                                    as *const uint16_t)
                                                    .offset(
                                                        (*(&raw const _pcre2_ucd_stage1_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                (fc as ::core::ffi::c_int
                                                                    / UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            * UCD_BLOCK_SIZE
                                                            + fc as ::core::ffi::c_int
                                                                % UCD_BLOCK_SIZE)
                                                            as isize,
                                                    )
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            ))
                                        .gbprop
                                            as ::core::ffi::c_int;
                                        if *(&raw const _pcre2_ucp_gbtable_16 as *const uint32_t)
                                            .offset(lgb as isize)
                                            & (1 as uint32_t) << rgb
                                            == 0 as uint32_t
                                        {
                                            break;
                                        }
                                        (*F).eptr = fptr;
                                        rgb = lgb;
                                    }
                                    current_block = 17293935983997591839;
                                    continue;
                                }
                            }
                            220 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    if *(*F).eptr as ::core::ffi::c_uint
                                        & 0xfc00 as ::core::ffi::c_uint
                                        == 0xdc00 as ::core::ffi::c_uint
                                    {
                                        (*F).eptr = (*F).eptr.offset(-1);
                                    }
                                    if (*F).fields.type_repeat.ctype
                                        == OP_ANYNL as ::core::ffi::c_int as uint32_t
                                        && (*F).eptr > (*F).fields.type_repeat.start_eptr
                                        && *(*F).eptr as ::core::ffi::c_int == CHAR_NL
                                        && *(*F).eptr.offset(-(1 as ::core::ffi::c_int) as isize)
                                            as ::core::ffi::c_int
                                            == CHAR_CR
                                    {
                                        (*F).eptr = (*F).eptr.offset(-1);
                                    }
                                    current_block = 6158284093282687738;
                                    continue;
                                }
                            }
                            221 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    (*F).eptr = (*F).eptr.offset(-1);
                                    if utf != 0 {
                                        if *(*F).eptr as ::core::ffi::c_uint
                                            & 0xfc00 as ::core::ffi::c_uint
                                            == 0xdc00 as ::core::ffi::c_uint
                                        {
                                            (*F).eptr = (*F).eptr.offset(-1);
                                        }
                                    }
                                    current_block = 943143812647028069;
                                    continue;
                                }
                            }
                            222 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh171 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh171 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh172 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh172 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh173 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh173 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        prop_3 = (&raw const _pcre2_ucd_records_16
                                            as *const ucd_record)
                                            .offset(
                                                *(&raw const _pcre2_ucd_stage2_16
                                                    as *const uint16_t)
                                                    .offset(
                                                        (*(&raw const _pcre2_ucd_stage1_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                (fc as ::core::ffi::c_int
                                                                    / UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            * UCD_BLOCK_SIZE
                                                            + fc as ::core::ffi::c_int
                                                                % UCD_BLOCK_SIZE)
                                                            as isize,
                                                    )
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            );
                                        ok_4 = (*(&raw const _pcre2_ucd_boolprop_sets_16
                                            as *const uint32_t)
                                            .offset(
                                                ((*prop_3).bprops as ::core::ffi::c_int
                                                    & 0xfff as ::core::ffi::c_int)
                                                    as isize,
                                            )
                                            .offset(
                                                (*F).fields
                                                    .type_repeat
                                                    .propvalue
                                                    .wrapping_div(32 as uint32_t)
                                                    as isize,
                                            )
                                            & (1 as uint32_t)
                                                << (*F)
                                                    .fields
                                                    .type_repeat
                                                    .propvalue
                                                    .wrapping_rem(32 as uint32_t)
                                            != 0 as uint32_t)
                                            as ::core::ffi::c_int
                                            as BOOL;
                                        if !(ok_4
                                            == ((*F).fields.type_repeat.ctype
                                                == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int)
                                        {
                                            current_block = 2375076820243858887;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            223 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh168 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh168 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh169 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh169 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh170 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh170 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        if !((((*(&raw const _pcre2_ucd_records_16
                                            as *const ucd_record)
                                            .offset(
                                                *(&raw const _pcre2_ucd_stage2_16
                                                    as *const uint16_t)
                                                    .offset(
                                                        (*(&raw const _pcre2_ucd_stage1_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                (fc as ::core::ffi::c_int
                                                                    / 128 as ::core::ffi::c_int)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            * 128 as ::core::ffi::c_int
                                                            + fc as ::core::ffi::c_int
                                                                % 128 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            ))
                                        .scriptx_bidiclass
                                            as ::core::ffi::c_int
                                            >> UCD_BIDICLASS_SHIFT)
                                            as uint32_t
                                            == (*F).fields.type_repeat.propvalue)
                                            as ::core::ffi::c_int
                                            == ((*F).fields.type_repeat.ctype
                                                == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int)
                                        {
                                            current_block = 9721769700587795339;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            224 => {
                                if rrc != MATCH_NOMATCH {
                                    rrc = rrc;
                                    current_block = 9215777819543815842;
                                    continue;
                                } else {
                                    let fresh149 = (*F).fields.type_repeat.min;
                                    (*F).fields.type_repeat.min =
                                        (*F).fields.type_repeat.min.wrapping_add(1);
                                    if fresh149 >= (*F).fields.type_repeat.max {
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else if (*F).eptr >= (*mb).end_subject {
                                        if (*mb).partial as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                            && ((*F).eptr > (*mb).start_used_ptr
                                                || (*mb).allowemptypartial != 0)
                                        {
                                            (*mb).hitend = TRUE as BOOL;
                                            if (*mb).partial as ::core::ffi::c_int
                                                > 1 as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_PARTIAL;
                                            }
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    } else {
                                        let fresh150 = (*F).eptr;
                                        (*F).eptr = (*F).eptr.offset(1);
                                        fc = *fresh150 as uint32_t;
                                        if utf != 0 && fc & 0xfc00 as uint32_t == 0xd800 as uint32_t
                                        {
                                            let fresh151 = (*F).eptr;
                                            (*F).eptr = (*F).eptr.offset(1);
                                            fc = ((fc & 0x3ff as uint32_t)
                                                << 10 as ::core::ffi::c_int
                                                | *fresh151 as uint32_t & 0x3ff as uint32_t)
                                                .wrapping_add(0x10000 as uint32_t);
                                        }
                                        prop_2 = (&raw const _pcre2_ucd_records_16
                                            as *const ucd_record)
                                            .offset(
                                                *(&raw const _pcre2_ucd_stage2_16
                                                    as *const uint16_t)
                                                    .offset(
                                                        (*(&raw const _pcre2_ucd_stage1_16
                                                            as *const uint16_t)
                                                            .offset(
                                                                (fc as ::core::ffi::c_int
                                                                    / UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            * UCD_BLOCK_SIZE
                                                            + fc as ::core::ffi::c_int
                                                                % UCD_BLOCK_SIZE)
                                                            as isize,
                                                    )
                                                    as ::core::ffi::c_int
                                                    as isize,
                                            );
                                        ok_3 = ((*prop_2).script as uint32_t
                                            == (*F).fields.type_repeat.propvalue
                                            || *(&raw const _pcre2_ucd_script_sets_16
                                                as *const uint32_t)
                                                .offset(
                                                    ((*prop_2).scriptx_bidiclass
                                                        as ::core::ffi::c_int
                                                        & 0x3ff as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                .offset(
                                                    (*F).fields
                                                        .type_repeat
                                                        .propvalue
                                                        .wrapping_div(32 as uint32_t)
                                                        as isize,
                                                )
                                                & (1 as uint32_t)
                                                    << (*F)
                                                        .fields
                                                        .type_repeat
                                                        .propvalue
                                                        .wrapping_rem(32 as uint32_t)
                                                != 0 as uint32_t)
                                            as ::core::ffi::c_int
                                            as BOOL;
                                        if !(ok_3
                                            == ((*F).fields.type_repeat.ctype
                                                == OP_NOTPROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int)
                                        {
                                            current_block = 8229715678103841037;
                                            break 's_263;
                                        }
                                        rrc = 0 as ::core::ffi::c_int;
                                        current_block = 9215777819543815842;
                                        continue;
                                    }
                                }
                            }
                            _ => return PCRE2_ERROR_INTERNAL,
                        }
                    }
                }
                match current_block {
                    10033017790122999806 => {
                        rrc = 0 as ::core::ffi::c_int;
                        current_block = 9215777819543815842;
                    }
                    _ => {
                        rrc = 0 as ::core::ffi::c_int;
                        current_block = 9215777819543815842;
                    }
                }
            }
            match current_block {
                5133818031642813295 => {
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                }
                103976322284692088 => {
                    (*F).ecode = (*F)
                        .ecode
                        .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize);
                }
                _ => {
                    (*F).ecode = next_branch;
                    start_ecode = current_branch
                        .offset(1 as ::core::ffi::c_int as isize)
                        .offset(1 as ::core::ffi::c_int as isize);
                    (*F).return_id = RM1 as ::core::ffi::c_int as uint8_t;
                    current_block = 6380077529129020506;
                    break;
                }
            }
        }
        match current_block {
            7298563137461813030 => {
                group_frame_type = GF_CONDASSERT as uint32_t;
                start_ecode = (*F).fields.op_cond.start_branch.offset(
                    *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                        .offset(*(*F).fields.op_cond.start_branch as isize)
                        as ::core::ffi::c_int as isize,
                );
                (*F).return_id = RM5 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            2202530641731595862 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM22 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            4807934851113556144 => {
                if (*F).fields.type_repeat.ctype == OP_EXTUNI as ::core::ffi::c_int as uint32_t {
                    current_block = 7680255065025725177;
                } else if utf != 0 {
                    current_block = 10114015117503592819;
                } else {
                    current_block = 51091121742473818;
                }
            }
            685409568547905116 => match proptype {
                PT_LAMP => {
                    current_block = 7583650632015690588;
                }
                PT_GC => {
                    current_block = 1724909796719861406;
                }
                PT_PC => {
                    current_block = 15506043489740435919;
                }
                PT_SC => {
                    current_block = 2403859285966545931;
                }
                PT_SCX => {
                    current_block = 8229715678103841037;
                }
                PT_ALNUM => {
                    current_block = 11093113080711097189;
                }
                PT_SPACE | PT_PXSPACE => {
                    current_block = 15032463550852564264;
                }
                PT_WORD => {
                    current_block = 3420986181356424615;
                }
                PT_CLIST => {
                    current_block = 6407021218369142540;
                }
                PT_UCNC => {
                    current_block = 3573618632796085661;
                }
                PT_BIDICL => {
                    current_block = 9721769700587795339;
                }
                PT_BOOL => {
                    current_block = 2375076820243858887;
                }
                _ => return PCRE2_ERROR_INTERNAL,
            },
            8535700574364478148 => {
                number = *(*F)
                    .ecode
                    .offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                    as uint32_t;
                (*F).fields.op_brapos.frame_type = GF_CAPTURE as uint32_t | number;
                current_block = 7326581914713273544;
            }
            12574270518964373171 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM103 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            6265774175505686097 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM102 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            17975895912995949440 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM101 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            3302068592069234587 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM100 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            6543021350661184703 => {
                start_ecode = (*F)
                    .ecode
                    .offset(1 as ::core::ffi::c_int as isize)
                    .offset(1 as ::core::ffi::c_int as isize);
                (*F).return_id = RM6 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            14416695743247835409 => {
                start_ecode = bracode;
                (*F).return_id = RM7 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            13776980873676388703 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM201 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            4519209830585050666 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM23 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            10553790310514340151 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM200 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            6367621710937907790 => {
                next_ecode_4 = ::core::ptr::null::<PCRE2_UCHAR16>();
                group_frame_type = (*F).fields.op_recurse.frame_type;
                start_ecode = (*F).fields.op_recurse.start_branch.offset(
                    *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                        .offset(*(*F).fields.op_recurse.start_branch as isize)
                        as ::core::ffi::c_int as isize,
                );
                (*F).return_id = RM11 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            10082661835832785913 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM31 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            14401907612653336132 => {
                group_frame_type = GF_NOCAPTURE as uint32_t;
                start_ecode = (*F).ecode.offset(
                    *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                        .offset(*(*F).ecode as isize) as ::core::ffi::c_int
                        as isize,
                );
                (*F).return_id = RM3 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            17451096827168202177 => {
                group_frame_type = GF_NOCAPTURE as uint32_t;
                start_ecode = (*F).ecode.offset(
                    *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                        .offset(*(*F).ecode as isize) as ::core::ffi::c_int
                        as isize,
                );
                (*F).return_id = RM4 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            9699707990742192723 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM29 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            1164351245162749421 => {
                loop {
                    if *ecode as ::core::ffi::c_int == OP_CREF as ::core::ffi::c_int {
                        length =
                            length.wrapping_add((1 as ::core::ffi::c_int + IMM2_SIZE) as size_t);
                        ecode = ecode.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize);
                    } else {
                        if !(*ecode as ::core::ffi::c_int == OP_DNCREF as ::core::ffi::c_int) {
                            break;
                        }
                        length = length.wrapping_add(
                            (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                as size_t,
                        );
                        ecode = ecode.offset(
                            (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                                as isize,
                        );
                    }
                }
                (*F).fields.op_assert_scs.saved_end_subject = (*mb).end_subject;
                (*F).fields.op_assert_scs.true_end_extra =
                    (*mb).true_end_subject.offset_from((*mb).end_subject) as isize
                        as size_t;
                (*F).fields.op_assert_scs.saved_eptr = (*F).eptr;
                (*F).fields.op_assert_scs.saved_moptions = (*mb).moptions;
                (*F).eptr = (*mb)
                    .start_subject
                    .offset((*F).ovector[offset as usize] as isize);
                (*mb).end_subject = (*mb)
                    .start_subject
                    .offset((*F).ovector[offset.wrapping_add(1 as size_t) as usize] as isize);
                (*mb).true_end_subject = (*mb).end_subject;
                (*mb).moptions =
                    ((*mb).moptions as ::core::ffi::c_uint & !PCRE2_NOTEOL) as uint32_t;
                current_block = 6829137806597638593;
            }
            6021810480259369443 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM27 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            5981395450828152283 => {
                cc_2 = 0;
                start_ecode = (*F).ecode;
                (*F).return_id = RM25 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            1391843528280232292 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM202 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            5585757815311561815 => {
                group_frame_type = (*F).fields.op_bra.frame_type;
                start_ecode = (*F).ecode.offset(
                    *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                        .offset(*(*F).ecode as isize) as ::core::ffi::c_int
                        as isize,
                );
                (*F).return_id = RM2 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            12869037699417280666 => {
                slength_0 = 0;
                start_ecode = (*F).ecode;
                (*F).return_id = RM20 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            12225199092044798022 => {
                start_ecode = (*F)
                    .ecode
                    .offset(1 as ::core::ffi::c_int as isize)
                    .offset((2 as ::core::ffi::c_int * 1 as ::core::ffi::c_int) as isize);
                (*F).return_id = RM37 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            12426467910555810903 => {
                (*F).fields.op_brapos.frame_type = GF_NOCAPTURE as uint32_t;
                current_block = 7326581914713273544;
            }
            8038707252912297919 => {
                d_0 = 0;
                current_block = 15467235608407556447;
            }
            10771577056827685507 => {
                d_3 = 0;
                current_block = 11194422880361134385;
            }
            _ => {}
        }
        match current_block {
            7326581914713273544 => {
                (*F).byte1 = FALSE as uint8_t;
                (*F).fields.op_brapos.start_group = (*F).ecode;
                current_block = 14510290228760237064;
            }
            51091121742473818 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM33 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            10114015117503592819 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM218 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            7680255065025725177 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM217 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            2375076820243858887 => {
                ok_4 = 0;
                prop_3 = ::core::ptr::null::<ucd_record>();
                start_ecode = (*F).ecode;
                (*F).return_id = RM222 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            9721769700587795339 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM223 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            3573618632796085661 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM216 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            6407021218369142540 => {
                cp_1 = ::core::ptr::null::<uint32_t>();
                start_ecode = (*F).ecode;
                (*F).return_id = RM215 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            3420986181356424615 => {
                chartype_3 = 0;
                category_2 = 0;
                start_ecode = (*F).ecode;
                (*F).return_id = RM214 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            15032463550852564264 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM213 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            11093113080711097189 => {
                category_1 = 0;
                start_ecode = (*F).ecode;
                (*F).return_id = RM212 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            8229715678103841037 => {
                ok_3 = 0;
                prop_2 = ::core::ptr::null::<ucd_record>();
                start_ecode = (*F).ecode;
                (*F).return_id = RM224 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            2403859285966545931 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM211 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            15506043489740435919 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM210 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            1724909796719861406 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM209 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            7583650632015690588 => {
                chartype_2 = 0;
                start_ecode = (*F).ecode;
                (*F).return_id = RM208 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            11194422880361134385 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM206 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            15467235608407556447 => {
                start_ecode = (*F).ecode;
                (*F).return_id = RM204 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            6829137806597638593 => {
                group_frame_type = GF_NOCAPTURE as uint32_t;
                start_ecode = (*F)
                    .ecode
                    .offset(1 as ::core::ffi::c_int as isize)
                    .offset(1 as ::core::ffi::c_int as isize)
                    .offset(length as isize);
                (*F).return_id = RM38 as ::core::ffi::c_int as uint8_t;
                current_block = 6380077529129020506;
            }
            _ => {}
        }
        match current_block {
            14510290228760237064 => {
                (*F).fields.op_brapos.start_eptr = (*F).eptr;
                group_frame_type = (*F).fields.op_brapos.frame_type;
                start_ecode = (*F).ecode.offset(
                    *(&raw const _pcre2_OP_lengths_16 as *const uint8_t)
                        .offset(*(*F).ecode as isize) as ::core::ffi::c_int
                        as isize,
                );
                (*F).return_id = RM8 as ::core::ffi::c_int as uint8_t;
            }
            _ => {}
        }
        N = (F as *mut ::core::ffi::c_char).offset(frame_size as isize) as *mut heapframe;
        if (N as *mut ::core::ffi::c_char).offset(frame_size as isize) as *mut heapframe
            >= frames_top
        {
            let mut new: *mut heapframe = ::core::ptr::null_mut::<heapframe>();
            let mut newsize: size_t = 0;
            let mut usedsize: size_t = (N as *mut ::core::ffi::c_char)
                .offset_from((*match_data).heapframes as *mut ::core::ffi::c_char)
                as isize as size_t;
            if (*match_data).heapframes_size >= (PCRE2_SIZE_MAX as size_t).wrapping_div(2 as size_t)
            {
                if (*match_data).heapframes_size
                    == (PCRE2_SIZE_MAX as size_t).wrapping_sub(1 as size_t)
                {
                    return PCRE2_ERROR_NOMEMORY;
                }
                newsize = PCRE2_SIZE_MAX.wrapping_sub(1 as usize) as size_t;
            } else {
                newsize = (*match_data).heapframes_size.wrapping_mul(2 as size_t);
            }
            if newsize.wrapping_div(1024 as size_t) >= (*mb).heap_limit as size_t {
                let mut old_size: size_t =
                    (*match_data).heapframes_size.wrapping_div(1024 as size_t);
                if (*mb).heap_limit as size_t <= old_size {
                    return PCRE2_ERROR_HEAPLIMIT;
                } else {
                    let mut max_delta: size_t = (1024 as size_t)
                        .wrapping_mul(((*mb).heap_limit as size_t).wrapping_sub(old_size));
                    let mut over_bytes: ::core::ffi::c_int =
                        (*match_data).heapframes_size.wrapping_rem(1024 as size_t)
                            as ::core::ffi::c_int;
                    if over_bytes != 0 {
                        max_delta = max_delta
                            .wrapping_sub((1024 as ::core::ffi::c_int - over_bytes) as size_t);
                    }
                    newsize = (*match_data).heapframes_size.wrapping_add(max_delta);
                }
            }
            if newsize.wrapping_sub(usedsize) < frame_size {
                return PCRE2_ERROR_HEAPLIMIT;
            }
            new = (*match_data)
                .memctl
                .malloc
                .expect("non-null function pointer")(
                newsize, (*match_data).memctl.memory_data
            ) as *mut heapframe;
            if new.is_null() {
                return PCRE2_ERROR_NOMEMORY;
            }
            memcpy(
                new as *mut ::core::ffi::c_void,
                (*match_data).heapframes as *const ::core::ffi::c_void,
                usedsize,
            );
            N = (new as *mut ::core::ffi::c_char).offset(usedsize as isize) as *mut heapframe;
            F = (N as *mut ::core::ffi::c_char).offset(-(frame_size as isize)) as *mut heapframe;
            (*match_data)
                .memctl
                .free
                .expect("non-null function pointer")(
                (*match_data).heapframes as *mut ::core::ffi::c_void,
                (*match_data).memctl.memory_data,
            );
            (*match_data).heapframes = new as *mut heapframe;
            (*match_data).heapframes_size = newsize;
            frames_top =
                (new as *mut ::core::ffi::c_char).offset(newsize as isize) as *mut heapframe;
        }
        memcpy(
            (N as *mut ::core::ffi::c_char).offset(64 as usize as isize)
                as *mut ::core::ffi::c_void,
            (F as *mut ::core::ffi::c_char).offset(64 as usize as isize)
                as *const ::core::ffi::c_void,
            frame_copy_size,
        );
        (*N).rdepth = (*F).rdepth.wrapping_add(1 as uint32_t);
        F = N;
    }
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_match_16(
    mut code: *const pcre2_code_16,
    mut subject: PCRE2_SPTR16,
    mut length: size_t,
    mut start_offset: size_t,
    mut options: uint32_t,
    mut match_data: *mut pcre2_match_data_16,
    mut mcontext: *mut pcre2_match_context_16,
) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut rc: ::core::ffi::c_int = 0;
    let mut start_bits: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut re: *const pcre2_real_code_16 = code as *const pcre2_real_code_16;
    let mut original_options: uint32_t = options;
    let mut anchored: BOOL = 0;
    let mut firstline: BOOL = 0;
    let mut has_first_cu: BOOL = FALSE;
    let mut has_req_cu: BOOL = FALSE;
    let mut startline: BOOL = 0;
    let mut first_cu: PCRE2_UCHAR16 = 0 as PCRE2_UCHAR16;
    let mut first_cu2: PCRE2_UCHAR16 = 0 as PCRE2_UCHAR16;
    let mut req_cu: PCRE2_UCHAR16 = 0 as PCRE2_UCHAR16;
    let mut req_cu2: PCRE2_UCHAR16 = 0 as PCRE2_UCHAR16;
    let mut null_str: [PCRE2_UCHAR16; 1] = [0xcd as ::core::ffi::c_int as PCRE2_UCHAR16];
    let mut original_subject: PCRE2_SPTR16 = subject;
    let mut bumpalong_limit: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut end_subject: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut true_end_subject: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut start_match: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut req_cu_ptr: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut start_partial: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut match_partial: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut utf: BOOL = FALSE;
    let mut ucp: BOOL = FALSE;
    let mut allow_invalid: BOOL = 0;
    let mut fragment_options: uint32_t = 0 as uint32_t;
    let mut frame_size: size_t = 0;
    let mut heapframes_size: size_t = 0;
    let mut cb: pcre2_callout_block_16 = pcre2_callout_block_16 {
        version: 0,
        callout_number: 0,
        capture_top: 0,
        capture_last: 0,
        offset_vector: ::core::ptr::null_mut::<size_t>(),
        mark: ::core::ptr::null::<PCRE2_UCHAR16>(),
        subject: ::core::ptr::null::<PCRE2_UCHAR16>(),
        subject_length: 0,
        start_match: 0,
        current_position: 0,
        pattern_position: 0,
        next_item_length: 0,
        callout_string_offset: 0,
        callout_string_length: 0,
        callout_string: ::core::ptr::null::<PCRE2_UCHAR16>(),
        callout_flags: 0,
    };
    let mut actual_match_block: match_block_16 = match_block_16 {
        memctl: pcre2_memctl {
            malloc: None,
            free: None,
            memory_data: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        },
        heap_limit: 0,
        match_limit: 0,
        match_limit_depth: 0,
        match_call_count: 0,
        hitend: 0,
        hasthen: 0,
        hasbsk: 0,
        allowemptypartial: 0,
        allowlookaroundbsk: 0,
        lcc: ::core::ptr::null::<uint8_t>(),
        fcc: ::core::ptr::null::<uint8_t>(),
        ctypes: ::core::ptr::null::<uint8_t>(),
        start_offset: 0,
        end_offset_top: 0,
        partial: 0,
        bsr_convention: 0,
        name_count: 0,
        name_entry_size: 0,
        name_table: ::core::ptr::null::<PCRE2_UCHAR16>(),
        start_code: ::core::ptr::null::<PCRE2_UCHAR16>(),
        start_subject: ::core::ptr::null::<PCRE2_UCHAR16>(),
        check_subject: ::core::ptr::null::<PCRE2_UCHAR16>(),
        end_subject: ::core::ptr::null::<PCRE2_UCHAR16>(),
        true_end_subject: ::core::ptr::null::<PCRE2_UCHAR16>(),
        end_match_ptr: ::core::ptr::null::<PCRE2_UCHAR16>(),
        start_used_ptr: ::core::ptr::null::<PCRE2_UCHAR16>(),
        last_used_ptr: ::core::ptr::null::<PCRE2_UCHAR16>(),
        mark: ::core::ptr::null::<PCRE2_UCHAR16>(),
        nomatch_mark: ::core::ptr::null::<PCRE2_UCHAR16>(),
        verb_ecode_ptr: ::core::ptr::null::<PCRE2_UCHAR16>(),
        verb_skip_ptr: ::core::ptr::null::<PCRE2_UCHAR16>(),
        verb_current_recurse: 0,
        moptions: 0,
        poptions: 0,
        skip_arg_count: 0,
        ignore_skip_arg: 0,
        nltype: 0,
        nllen: 0,
        nl: [0; 4],
        cb: ::core::ptr::null_mut::<pcre2_callout_block_16>(),
        callout_data: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        callout: None,
    };
    let mut mb: *mut match_block_16 = &raw mut actual_match_block;
    if subject.is_null() && length == 0 as size_t {
        subject = &raw mut null_str as *mut PCRE2_UCHAR16 as PCRE2_SPTR16;
    }
    if match_data.is_null() {
        return PCRE2_ERROR_NULL;
    }
    if code.is_null() || subject.is_null() {
        (*match_data).rc = PCRE2_ERROR_NULL;
        return (*match_data).rc;
    }
    if options & !(PUBLIC_MATCH_OPTIONS as uint32_t) != 0 as uint32_t {
        (*match_data).rc = PCRE2_ERROR_BADOPTION;
        return (*match_data).rc;
    }
    start_match = subject.offset(start_offset as isize);
    req_cu_ptr = start_match.offset(-(1 as ::core::ffi::c_int as isize));
    if length == PCRE2_ZERO_TERMINATED {
        length = _pcre2_strlen_16(subject);
    }
    end_subject = subject.offset(length as isize);
    true_end_subject = end_subject;
    if start_offset > length {
        (*match_data).rc = PCRE2_ERROR_BADOFFSET;
        return (*match_data).rc;
    }
    if (*re).magic_number as usize != MAGIC_NUMBER {
        (*match_data).rc = PCRE2_ERROR_BADMAGIC;
        return (*match_data).rc;
    }
    if (*re).flags & PCRE2_MODE_MASK as uint32_t
        != (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as uint32_t
    {
        (*match_data).rc = PCRE2_ERROR_BADMODE;
        return (*match_data).rc;
    }
    options = (options as ::core::ffi::c_uint
        | ((*re).flags & FF as uint32_t).wrapping_div(
            (FF as uint32_t & (!(FF as uint32_t)).wrapping_add(1 as uint32_t))
                .wrapping_div(OO as uint32_t & (!(OO as uint32_t)).wrapping_add(1 as uint32_t)),
        ) as ::core::ffi::c_uint) as uint32_t;
    utf = ((*re).overall_options & PCRE2_UTF as uint32_t != 0 as uint32_t) as ::core::ffi::c_int
        as BOOL;
    allow_invalid = ((*re).overall_options & PCRE2_MATCH_INVALID_UTF as uint32_t != 0 as uint32_t)
        as ::core::ffi::c_int as BOOL;
    ucp = ((*re).overall_options & PCRE2_UCP as uint32_t != 0 as uint32_t) as ::core::ffi::c_int
        as BOOL;
    (*mb).partial = (if options & PCRE2_PARTIAL_HARD as uint32_t != 0 as uint32_t {
        2 as ::core::ffi::c_int
    } else if options & PCRE2_PARTIAL_SOFT as uint32_t != 0 as uint32_t {
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    }) as uint16_t;
    if (*mb).partial as ::core::ffi::c_int != 0 as ::core::ffi::c_int
        && ((*re).overall_options | options) & PCRE2_ENDANCHORED as uint32_t != 0 as uint32_t
    {
        (*match_data).rc = PCRE2_ERROR_BADOPTION;
        return (*match_data).rc;
    }
    if !mcontext.is_null()
        && (*mcontext).offset_limit != PCRE2_UNSET
        && (*re).overall_options & PCRE2_USE_OFFSET_LIMIT as uint32_t == 0 as uint32_t
    {
        (*match_data).rc = PCRE2_ERROR_BADOFFSETLIMIT;
        return (*match_data).rc;
    }
    if (*match_data).flags as ::core::ffi::c_uint & PCRE2_MD_COPIED_SUBJECT
        != 0 as ::core::ffi::c_uint
    {
        (*match_data)
            .memctl
            .free
            .expect("non-null function pointer")(
            (*match_data).subject as *mut ::core::ffi::c_void,
            (*match_data).memctl.memory_data,
        );
        (*match_data).flags =
            ((*match_data).flags as ::core::ffi::c_uint & !PCRE2_MD_COPIED_SUBJECT) as uint8_t;
    }
    (*match_data).subject = ::core::ptr::null::<PCRE2_UCHAR16>();
    (*match_data).startchar = 0 as size_t;
    if !(*re).executable_jit.is_null()
        && options & !(PUBLIC_JIT_MATCH_OPTIONS as uint32_t) == 0 as uint32_t
        && _pcre2_jit_check_exec_16((*re).executable_jit, options) != 0
    {
        if utf != 0
            && options & PCRE2_NO_UTF_CHECK as uint32_t == 0 as uint32_t
            && allow_invalid == 0
        {
            if start_match < end_subject
                && *start_match as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                    == 0xdc00 as ::core::ffi::c_uint
            {
                if start_offset > 0 as size_t {
                    (*match_data).rc = PCRE2_ERROR_BADUTFOFFSET;
                    return (*match_data).rc;
                }
                (*match_data).rc = PCRE2_ERROR_UTF16_ERR3;
                return (*match_data).rc;
            }
            let mut i: ::core::ffi::c_uint = (*re).max_lookbehind as ::core::ffi::c_uint;
            while i > 0 as ::core::ffi::c_uint && start_match > subject {
                start_match = start_match.offset(-1);
                while start_match > subject
                    && *start_match as ::core::ffi::c_int & 0xfc00 as ::core::ffi::c_int
                        == 0xdc00 as ::core::ffi::c_int
                {
                    start_match = start_match.offset(-1);
                }
                i = i.wrapping_sub(1);
            }
            rc =
                _pcre2_valid_utf_16(
                    start_match,
                    length.wrapping_sub(
                        start_match.offset_from(subject) as isize as size_t
                    ),
                    &raw mut (*match_data).startchar,
                );
            if rc != 0 as ::core::ffi::c_int {
                (*match_data).startchar =
                    (*match_data)
                        .startchar
                        .wrapping_add(
                            start_match.offset_from(subject) as isize as size_t
                        );
                (*match_data).rc = rc;
                return (*match_data).rc;
            }
        }
        rc = pcre2_jit_match_16(
            code,
            subject,
            length,
            start_offset,
            options,
            match_data,
            mcontext,
        );
        (*match_data).options = original_options;
        if rc >= 0 as ::core::ffi::c_int
            && options & PCRE2_COPY_MATCHED_SUBJECT as uint32_t != 0 as uint32_t
        {
            if length != 0 as size_t {
                (*match_data).subject = (*match_data)
                    .memctl
                    .malloc
                    .expect("non-null function pointer")(
                    length
                        .wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
                    (*match_data).memctl.memory_data,
                ) as PCRE2_SPTR16;
                if (*match_data).subject.is_null() {
                    (*match_data).rc = PCRE2_ERROR_NOMEMORY;
                    return (*match_data).rc;
                }
                memcpy(
                    (*match_data).subject as *mut ::core::ffi::c_void,
                    subject as *const ::core::ffi::c_void,
                    length
                        .wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
                );
            } else {
                (*match_data).subject = ::core::ptr::null::<PCRE2_UCHAR16>();
            }
            (*match_data).flags =
                ((*match_data).flags as ::core::ffi::c_uint | PCRE2_MD_COPIED_SUBJECT) as uint8_t;
        } else if !(*match_data).subject.is_null() {
            (*match_data).subject = original_subject;
        }
        return rc;
    }
    (*mb).check_subject = subject;
    if utf != 0 && (options & PCRE2_NO_UTF_CHECK as uint32_t == 0 as uint32_t || allow_invalid != 0)
    {
        let mut skipped_bad_start: BOOL = FALSE;
        if allow_invalid != 0 {
            while start_match < end_subject
                && *start_match as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                    == 0xdc00 as ::core::ffi::c_uint
            {
                start_match = start_match.offset(1);
                skipped_bad_start = TRUE as BOOL;
            }
        } else if start_match < end_subject
            && *start_match as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                == 0xdc00 as ::core::ffi::c_uint
        {
            if start_offset > 0 as size_t {
                (*match_data).rc = PCRE2_ERROR_BADUTFOFFSET;
                return (*match_data).rc;
            }
            (*match_data).rc = PCRE2_ERROR_UTF16_ERR3;
            return (*match_data).rc;
        }
        (*mb).check_subject = start_match;
        if skipped_bad_start == 0 {
            let mut i_0: ::core::ffi::c_uint = 0;
            i_0 = (*re).max_lookbehind as ::core::ffi::c_uint;
            while i_0 > 0 as ::core::ffi::c_uint && (*mb).check_subject > subject {
                (*mb).check_subject = (*mb).check_subject.offset(-1);
                while (*mb).check_subject > subject
                    && *(*mb).check_subject as ::core::ffi::c_int & 0xfc00 as ::core::ffi::c_int
                        == 0xdc00 as ::core::ffi::c_int
                {
                    (*mb).check_subject = (*mb).check_subject.offset(-1);
                }
                i_0 = i_0.wrapping_sub(1);
            }
        }
        loop {
            rc = _pcre2_valid_utf_16(
                (*mb).check_subject,
                length.wrapping_sub(
                    (*mb).check_subject.offset_from(subject) as isize as size_t,
                ),
                &raw mut (*match_data).startchar,
            );
            if rc == 0 as ::core::ffi::c_int {
                break;
            }
            (*match_data).startchar =
                (*match_data)
                    .startchar
                    .wrapping_add(
                        (*mb).check_subject.offset_from(subject) as isize as size_t,
                    );
            if allow_invalid == 0 || rc > 0 as ::core::ffi::c_int {
                (*match_data).rc = rc;
                return (*match_data).rc;
            }
            end_subject = subject.offset((*match_data).startchar as isize);
            if end_subject < start_match {
                (*mb).check_subject = end_subject.offset(1 as ::core::ffi::c_int as isize);
                while (*mb).check_subject < start_match
                    && *(*mb).check_subject as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                        == 0xdc00 as ::core::ffi::c_uint
                {
                    (*mb).check_subject = (*mb).check_subject.offset(1);
                }
                end_subject = true_end_subject;
            } else {
                fragment_options = PCRE2_NOTEOL as uint32_t;
                break;
            }
        }
    }
    if mcontext.is_null() {
        mcontext = &raw mut _pcre2_default_match_context_16;
        (*mb).memctl = (*re).memctl;
    } else {
        (*mb).memctl = (*mcontext).memctl;
    }
    anchored = (((*re).overall_options | options) & PCRE2_ANCHORED as uint32_t != 0 as uint32_t)
        as ::core::ffi::c_int as BOOL;
    firstline = (anchored == 0
        && (*re).overall_options & PCRE2_FIRSTLINE as uint32_t != 0 as uint32_t)
        as ::core::ffi::c_int as BOOL;
    startline =
        ((*re).flags & PCRE2_STARTLINE as uint32_t != 0 as uint32_t) as ::core::ffi::c_int as BOOL;
    bumpalong_limit = if (*mcontext).offset_limit == PCRE2_UNSET {
        true_end_subject
    } else {
        subject.offset((*mcontext).offset_limit as isize)
    };
    (*mb).cb = &raw mut cb;
    cb.version = 2 as uint32_t;
    cb.subject = subject;
    cb.subject_length = end_subject.offset_from(subject) as isize as size_t;
    cb.callout_flags = 0 as uint32_t;
    (*mb).callout = (*mcontext).callout;
    (*mb).callout_data = (*mcontext).callout_data;
    (*mb).start_subject = subject;
    (*mb).start_offset = start_offset;
    (*mb).end_subject = end_subject;
    (*mb).true_end_subject = true_end_subject;
    (*mb).hasthen =
        ((*re).flags & PCRE2_HASTHEN as uint32_t != 0 as uint32_t) as ::core::ffi::c_int as BOOL;
    (*mb).hasbsk =
        ((*re).flags & PCRE2_HASBSK as uint32_t != 0 as uint32_t) as ::core::ffi::c_int as BOOL;
    (*mb).allowemptypartial = ((*re).max_lookbehind as ::core::ffi::c_int > 0 as ::core::ffi::c_int
        || (*re).flags & PCRE2_MATCH_EMPTY as uint32_t != 0 as uint32_t)
        as ::core::ffi::c_int as BOOL;
    (*mb).allowlookaroundbsk = ((*re).extra_options & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK as uint32_t
        != 0 as uint32_t) as ::core::ffi::c_int as BOOL;
    (*mb).poptions = (*re).overall_options;
    (*mb).ignore_skip_arg = 0 as uint32_t;
    (*mb).nomatch_mark = ::core::ptr::null::<PCRE2_UCHAR16>();
    (*mb).mark = (*mb).nomatch_mark;
    (*mb).name_table = (re as *const uint8_t)
        .offset(::core::mem::size_of::<pcre2_real_code_16>() as usize as isize)
        as PCRE2_SPTR16;
    (*mb).name_count = (*re).name_count;
    (*mb).name_entry_size = (*re).name_entry_size;
    (*mb).start_code = (re as *const uint8_t).offset((*re).code_start as isize) as PCRE2_SPTR16;
    (*mb).bsr_convention = (*re).bsr_convention;
    (*mb).nltype = NLTYPE_FIXED as uint32_t;
    match (*re).newline_convention as ::core::ffi::c_int {
        PCRE2_NEWLINE_CR => {
            (*mb).nllen = 1 as uint32_t;
            (*mb).nl[0 as ::core::ffi::c_int as usize] = CHAR_CR as PCRE2_UCHAR16;
        }
        PCRE2_NEWLINE_LF => {
            (*mb).nllen = 1 as uint32_t;
            (*mb).nl[0 as ::core::ffi::c_int as usize] = CHAR_NL as PCRE2_UCHAR16;
        }
        PCRE2_NEWLINE_NUL => {
            (*mb).nllen = 1 as uint32_t;
            (*mb).nl[0 as ::core::ffi::c_int as usize] = CHAR_NUL as PCRE2_UCHAR16;
        }
        PCRE2_NEWLINE_CRLF => {
            (*mb).nllen = 2 as uint32_t;
            (*mb).nl[0 as ::core::ffi::c_int as usize] = CHAR_CR as PCRE2_UCHAR16;
            (*mb).nl[1 as ::core::ffi::c_int as usize] = CHAR_NL as PCRE2_UCHAR16;
        }
        PCRE2_NEWLINE_ANY => {
            (*mb).nltype = NLTYPE_ANY as uint32_t;
        }
        PCRE2_NEWLINE_ANYCRLF => {
            (*mb).nltype = NLTYPE_ANYCRLF as uint32_t;
        }
        _ => {
            (*match_data).rc = PCRE2_ERROR_INTERNAL;
            return (*match_data).rc;
        }
    }
    frame_size = ((120 as usize)
        .wrapping_add(
            (((*re).top_bracket as ::core::ffi::c_int * 2 as ::core::ffi::c_int) as usize)
                .wrapping_mul(::core::mem::size_of::<size_t>() as usize),
        )
        .wrapping_add(HEAPFRAME_ALIGNMENT as usize)
        .wrapping_sub(1 as usize)
        & !(HEAPFRAME_ALIGNMENT as usize).wrapping_sub(1 as usize)) as size_t;
    (*mb).heap_limit = if (*mcontext).heap_limit < (*re).limit_heap {
        (*mcontext).heap_limit
    } else {
        (*re).limit_heap
    };
    (*mb).match_limit = if (*mcontext).match_limit < (*re).limit_match {
        (*mcontext).match_limit
    } else {
        (*re).limit_match
    };
    (*mb).match_limit_depth = if (*mcontext).depth_limit < (*re).limit_depth {
        (*mcontext).depth_limit
    } else {
        (*re).limit_depth
    };
    heapframes_size = frame_size.wrapping_mul(10 as size_t);
    if heapframes_size < START_FRAMES_SIZE as size_t {
        heapframes_size = START_FRAMES_SIZE as size_t;
    }
    if heapframes_size.wrapping_div(1024 as size_t) > (*mb).heap_limit as size_t {
        let mut max_size: size_t = (1024 as uint32_t).wrapping_mul((*mb).heap_limit) as size_t;
        if max_size < frame_size {
            (*match_data).rc = PCRE2_ERROR_HEAPLIMIT;
            return (*match_data).rc;
        }
        heapframes_size = max_size;
    }
    if (*match_data).heapframes_size < heapframes_size {
        (*match_data)
            .memctl
            .free
            .expect("non-null function pointer")(
            (*match_data).heapframes as *mut ::core::ffi::c_void,
            (*match_data).memctl.memory_data,
        );
        (*match_data).heapframes = (*match_data)
            .memctl
            .malloc
            .expect("non-null function pointer")(
            heapframes_size, (*match_data).memctl.memory_data
        ) as *mut heapframe;
        if (*match_data).heapframes.is_null() {
            (*match_data).heapframes_size = 0 as size_t;
            (*match_data).rc = PCRE2_ERROR_NOMEMORY;
            return (*match_data).rc;
        }
        (*match_data).heapframes_size = heapframes_size;
    }
    memset(
        ((*match_data).heapframes as *mut ::core::ffi::c_char)
            .offset(120 as usize as isize) as *mut ::core::ffi::c_void,
        0xff as ::core::ffi::c_int,
        frame_size.wrapping_sub(120 as size_t),
    );
    (*mb).lcc = (*re).tables.offset(lcc_offset as isize);
    (*mb).fcc = (*re).tables.offset(fcc_offset as isize);
    (*mb).ctypes = (*re).tables.offset(ctypes_offset as isize);
    if (*re).flags & PCRE2_FIRSTSET as uint32_t != 0 as uint32_t {
        has_first_cu = TRUE as BOOL;
        first_cu2 = (*re).first_codeunit as PCRE2_UCHAR16;
        first_cu = first_cu2;
        if (*re).flags & PCRE2_FIRSTCASELESS as uint32_t != 0 as uint32_t {
            first_cu2 = (if first_cu as ::core::ffi::c_uint <= 255 as ::core::ffi::c_uint {
                *(*mb).fcc.offset(first_cu as isize) as ::core::ffi::c_int
            } else {
                first_cu as ::core::ffi::c_int
            }) as PCRE2_UCHAR16;
            if first_cu as ::core::ffi::c_int > 127 as ::core::ffi::c_int && (utf != 0 || ucp != 0)
            {
                first_cu2 = (first_cu as ::core::ffi::c_int
                    + (*(&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
                        *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                            (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                .offset((first_cu as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                                as ::core::ffi::c_int
                                * UCD_BLOCK_SIZE
                                + first_cu as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                as isize,
                        ) as ::core::ffi::c_int as isize,
                    ))
                    .other_case as ::core::ffi::c_int) as uint32_t
                    as PCRE2_UCHAR16;
            }
        }
    } else if startline == 0 && (*re).flags & PCRE2_FIRSTMAPSET as uint32_t != 0 as uint32_t {
        start_bits = &raw const (*re).start_bitmap as *const uint8_t;
    }
    if (*re).flags & PCRE2_LASTSET as uint32_t != 0 as uint32_t {
        has_req_cu = TRUE as BOOL;
        req_cu2 = (*re).last_codeunit as PCRE2_UCHAR16;
        req_cu = req_cu2;
        if (*re).flags & PCRE2_LASTCASELESS as uint32_t != 0 as uint32_t {
            req_cu2 = (if req_cu as ::core::ffi::c_uint <= 255 as ::core::ffi::c_uint {
                *(*mb).fcc.offset(req_cu as isize) as ::core::ffi::c_int
            } else {
                req_cu as ::core::ffi::c_int
            }) as PCRE2_UCHAR16;
            if req_cu as ::core::ffi::c_int > 127 as ::core::ffi::c_int && (utf != 0 || ucp != 0) {
                req_cu2 = (req_cu as ::core::ffi::c_int
                    + (*(&raw const _pcre2_ucd_records_16 as *const ucd_record).offset(
                        *(&raw const _pcre2_ucd_stage2_16 as *const uint16_t).offset(
                            (*(&raw const _pcre2_ucd_stage1_16 as *const uint16_t)
                                .offset((req_cu as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                                as ::core::ffi::c_int
                                * UCD_BLOCK_SIZE
                                + req_cu as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                                as isize,
                        ) as ::core::ffi::c_int as isize,
                    ))
                    .other_case as ::core::ffi::c_int) as uint32_t
                    as PCRE2_UCHAR16;
            }
        }
    }
    '_FRAGMENT_RESTART: loop {
        match_partial = ::core::ptr::null::<PCRE2_UCHAR16>();
        start_partial = match_partial;
        (*mb).hitend = FALSE as BOOL;
        loop {
            let mut new_start_match: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
            if (*re).optimization_flags & PCRE2_OPTIM_START_OPTIMIZE as uint32_t != 0 as uint32_t {
                if firstline != 0 {
                    let mut t: PCRE2_SPTR16 = start_match;
                    if utf != 0 {
                        while t < end_subject
                            && (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                (t < (*mb).end_subject
                                    && _pcre2_is_newline_16(
                                        t,
                                        (*mb).nltype,
                                        (*mb).end_subject,
                                        &raw mut (*mb).nllen,
                                        utf,
                                    ) != 0) as ::core::ffi::c_int
                            } else {
                                (t <= (*mb).end_subject.offset(-((*mb).nllen as isize))
                                    && *t as ::core::ffi::c_int
                                        == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                            as ::core::ffi::c_int
                                    && ((*mb).nllen == 1 as uint32_t
                                        || *t.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                                as ::core::ffi::c_int))
                                    as ::core::ffi::c_int
                            }) == 0
                        {
                            t = t.offset(1);
                            if t < end_subject
                                && *t as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                                    == 0xdc00 as ::core::ffi::c_uint
                            {
                                t = t.offset(1);
                            }
                        }
                    } else {
                        while t < end_subject
                            && (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                (t < (*mb).end_subject
                                    && _pcre2_is_newline_16(
                                        t,
                                        (*mb).nltype,
                                        (*mb).end_subject,
                                        &raw mut (*mb).nllen,
                                        utf,
                                    ) != 0) as ::core::ffi::c_int
                            } else {
                                (t <= (*mb).end_subject.offset(-((*mb).nllen as isize))
                                    && *t as ::core::ffi::c_int
                                        == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                            as ::core::ffi::c_int
                                    && ((*mb).nllen == 1 as uint32_t
                                        || *t.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                                as ::core::ffi::c_int))
                                    as ::core::ffi::c_int
                            }) == 0
                        {
                            t = t.offset(1);
                        }
                    }
                    end_subject = t;
                }
                if anchored != 0 {
                    if has_first_cu != 0 || !start_bits.is_null() {
                        let mut ok: BOOL = (start_match < end_subject) as ::core::ffi::c_int;
                        if ok != 0 {
                            let mut c: PCRE2_UCHAR16 = *start_match;
                            ok = (has_first_cu != 0
                                && (c as ::core::ffi::c_int == first_cu as ::core::ffi::c_int
                                    || c as ::core::ffi::c_int == first_cu2 as ::core::ffi::c_int))
                                as ::core::ffi::c_int as BOOL;
                            if ok == 0 && !start_bits.is_null() {
                                if c as ::core::ffi::c_int > 255 as ::core::ffi::c_int {
                                    c = 255 as PCRE2_UCHAR16;
                                }
                                ok = (*start_bits.offset(
                                    (c as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as isize,
                                ) as ::core::ffi::c_uint
                                    & (1 as ::core::ffi::c_uint)
                                        << (c as ::core::ffi::c_int & 7 as ::core::ffi::c_int)
                                    != 0 as ::core::ffi::c_uint)
                                    as ::core::ffi::c_int
                                    as BOOL;
                            }
                        }
                        if ok == 0 {
                            rc = MATCH_NOMATCH;
                            current_block = 9914851455145855695;
                            break;
                        }
                    }
                } else if has_first_cu != 0 {
                    if first_cu as ::core::ffi::c_int != first_cu2 as ::core::ffi::c_int {
                        let mut smc: PCRE2_UCHAR16 = 0;
                        while start_match < end_subject
                            && {
                                smc = *start_match;
                                smc as ::core::ffi::c_int != first_cu as ::core::ffi::c_int
                            }
                            && smc as ::core::ffi::c_int != first_cu2 as ::core::ffi::c_int
                        {
                            start_match = start_match.offset(1);
                        }
                    } else {
                        while start_match < end_subject
                            && *start_match as ::core::ffi::c_int != first_cu as ::core::ffi::c_int
                        {
                            start_match = start_match.offset(1);
                        }
                    }
                    if (*mb).partial as ::core::ffi::c_int == 0 as ::core::ffi::c_int
                        && start_match >= (*mb).end_subject
                    {
                        rc = MATCH_NOMATCH;
                        current_block = 9914851455145855695;
                        break;
                    }
                } else if startline != 0 {
                    if start_match > (*mb).start_subject.offset(start_offset as isize) {
                        if utf != 0 {
                            while start_match < end_subject
                                && (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                    (start_match > (*mb).start_subject
                                        && _pcre2_was_newline_16(
                                            start_match,
                                            (*mb).nltype,
                                            (*mb).start_subject,
                                            &raw mut (*mb).nllen,
                                            utf,
                                        ) != 0)
                                        as ::core::ffi::c_int
                                } else {
                                    (start_match
                                        >= (*mb).start_subject.offset((*mb).nllen as isize)
                                        && *start_match.offset(-((*mb).nllen as isize))
                                            as ::core::ffi::c_int
                                            == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                as ::core::ffi::c_int
                                        && ((*mb).nllen == 1 as uint32_t
                                            || *start_match
                                                .offset(-((*mb).nllen as isize))
                                                .offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                                    as ::core::ffi::c_int))
                                        as ::core::ffi::c_int
                                }) == 0
                            {
                                start_match = start_match.offset(1);
                                if start_match < end_subject
                                    && *start_match as ::core::ffi::c_uint
                                        & 0xfc00 as ::core::ffi::c_uint
                                        == 0xdc00 as ::core::ffi::c_uint
                                {
                                    start_match = start_match.offset(1);
                                }
                            }
                        } else {
                            while start_match < end_subject
                                && (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                    (start_match > (*mb).start_subject
                                        && _pcre2_was_newline_16(
                                            start_match,
                                            (*mb).nltype,
                                            (*mb).start_subject,
                                            &raw mut (*mb).nllen,
                                            utf,
                                        ) != 0)
                                        as ::core::ffi::c_int
                                } else {
                                    (start_match
                                        >= (*mb).start_subject.offset((*mb).nllen as isize)
                                        && *start_match.offset(-((*mb).nllen as isize))
                                            as ::core::ffi::c_int
                                            == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                as ::core::ffi::c_int
                                        && ((*mb).nllen == 1 as uint32_t
                                            || *start_match
                                                .offset(-((*mb).nllen as isize))
                                                .offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                                    as ::core::ffi::c_int))
                                        as ::core::ffi::c_int
                                }) == 0
                            {
                                start_match = start_match.offset(1);
                            }
                        }
                        if *start_match.offset(-(1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int
                            == CHAR_CR
                            && ((*mb).nltype == NLTYPE_ANY as uint32_t
                                || (*mb).nltype == NLTYPE_ANYCRLF as uint32_t)
                            && start_match < end_subject
                            && *start_match as ::core::ffi::c_int == CHAR_NL
                        {
                            start_match = start_match.offset(1);
                        }
                    }
                } else if !start_bits.is_null() {
                    while start_match < end_subject {
                        let mut c_0: uint32_t = *start_match as uint32_t;
                        if c_0 > 255 as uint32_t {
                            c_0 = 255 as uint32_t;
                        }
                        if *start_bits.offset(c_0.wrapping_div(8 as uint32_t) as isize)
                            as ::core::ffi::c_uint
                            & (1 as ::core::ffi::c_uint) << (c_0 & 7 as uint32_t)
                            != 0 as ::core::ffi::c_uint
                        {
                            break;
                        }
                        start_match = start_match.offset(1);
                    }
                    if (*mb).partial as ::core::ffi::c_int == 0 as ::core::ffi::c_int
                        && start_match >= (*mb).end_subject
                    {
                        rc = MATCH_NOMATCH;
                        current_block = 9914851455145855695;
                        break;
                    }
                }
                end_subject = (*mb).end_subject;
                if (*mb).partial as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                    let mut p: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
                    if (end_subject.offset_from(start_match) as isize)
                        < (*re).minlength as isize
                    {
                        rc = MATCH_NOMATCH;
                        current_block = 9914851455145855695;
                        break;
                    } else {
                        p = start_match.offset(
                            (if has_first_cu != 0 {
                                1 as ::core::ffi::c_int
                            } else {
                                0 as ::core::ffi::c_int
                            }) as isize,
                        );
                        if has_req_cu != 0 && p > req_cu_ptr {
                            let mut check_length: size_t = end_subject.offset_from(start_match)
                                as isize
                                as size_t;
                            if check_length < REQ_CU_MAX as size_t
                                || anchored == 0
                                    && check_length
                                        < (REQ_CU_MAX * 1000 as ::core::ffi::c_int) as size_t
                            {
                                if req_cu as ::core::ffi::c_int != req_cu2 as ::core::ffi::c_int {
                                    while p < end_subject {
                                        let fresh0 = p;
                                        p = p.offset(1);
                                        let mut pp: uint32_t = *fresh0 as uint32_t;
                                        if !(pp == req_cu as uint32_t || pp == req_cu2 as uint32_t)
                                        {
                                            continue;
                                        }
                                        p = p.offset(-1);
                                        break;
                                    }
                                } else {
                                    while p < end_subject {
                                        let fresh1 = p;
                                        p = p.offset(1);
                                        if !(*fresh1 as ::core::ffi::c_int
                                            == req_cu as ::core::ffi::c_int)
                                        {
                                            continue;
                                        }
                                        p = p.offset(-1);
                                        break;
                                    }
                                }
                                if p >= end_subject {
                                    rc = MATCH_NOMATCH;
                                    current_block = 9914851455145855695;
                                    break;
                                } else {
                                    req_cu_ptr = p;
                                }
                            }
                        }
                    }
                }
            }
            if start_match > bumpalong_limit {
                rc = MATCH_NOMATCH;
                current_block = 9914851455145855695;
                break;
            } else {
                cb.start_match = start_match.offset_from(subject) as isize as size_t;
                cb.callout_flags = (cb.callout_flags as ::core::ffi::c_uint
                    | PCRE2_CALLOUT_STARTMATCH) as uint32_t;
                (*mb).start_used_ptr = start_match;
                (*mb).last_used_ptr = start_match;
                (*mb).moptions = options | fragment_options;
                (*mb).match_call_count = 0 as uint32_t;
                (*mb).end_offset_top = 0 as size_t;
                (*mb).skip_arg_count = 0 as uint32_t;
                rc = match_0(
                    start_match,
                    (*mb).start_code,
                    (*re).top_bracket,
                    frame_size,
                    match_data,
                    mb,
                );
                if (*mb).hitend != 0 && start_partial.is_null() {
                    start_partial = (*mb).start_used_ptr;
                    match_partial = start_match;
                }
                match rc {
                    MATCH_SKIP_ARG => {
                        new_start_match = start_match;
                        (*mb).ignore_skip_arg = (*mb).skip_arg_count;
                        current_block = 15605369199999130895;
                    }
                    MATCH_SKIP => {
                        if (*mb).verb_skip_ptr > start_match {
                            new_start_match = (*mb).verb_skip_ptr;
                            current_block = 15605369199999130895;
                        } else {
                            current_block = 2171269653080262471;
                        }
                    }
                    MATCH_NOMATCH | MATCH_PRUNE | MATCH_THEN => {
                        current_block = 2171269653080262471;
                    }
                    MATCH_COMMIT => {
                        rc = MATCH_NOMATCH;
                        current_block = 6809027331625791801;
                        break;
                    }
                    _ => {
                        current_block = 6809027331625791801;
                        break;
                    }
                }
                match current_block {
                    2171269653080262471 => {
                        (*mb).ignore_skip_arg = 0 as uint32_t;
                        new_start_match = start_match.offset(1 as ::core::ffi::c_int as isize);
                        if utf != 0 {
                            if new_start_match < end_subject
                                && *new_start_match as ::core::ffi::c_uint
                                    & 0xfc00 as ::core::ffi::c_uint
                                    == 0xdc00 as ::core::ffi::c_uint
                            {
                                new_start_match = new_start_match.offset(1);
                            }
                        }
                    }
                    _ => {}
                }
                rc = MATCH_NOMATCH;
                if firstline != 0
                    && (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                        (start_match < (*mb).end_subject
                            && _pcre2_is_newline_16(
                                start_match,
                                (*mb).nltype,
                                (*mb).end_subject,
                                &raw mut (*mb).nllen,
                                utf,
                            ) != 0) as ::core::ffi::c_int
                    } else {
                        (start_match <= (*mb).end_subject.offset(-((*mb).nllen as isize))
                            && *start_match as ::core::ffi::c_int
                                == (*mb).nl[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                            && ((*mb).nllen == 1 as uint32_t
                                || *start_match.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int
                                    == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                        as ::core::ffi::c_int))
                            as ::core::ffi::c_int
                    }) != 0
                {
                    current_block = 9914851455145855695;
                    break;
                }
                start_match = new_start_match;
                if anchored != 0 || start_match > end_subject {
                    current_block = 9914851455145855695;
                    break;
                }
                if start_match > subject.offset(start_offset as isize)
                    && *start_match.offset(-(1 as ::core::ffi::c_int) as isize)
                        as ::core::ffi::c_int
                        == CHAR_CR
                    && start_match < end_subject
                    && *start_match as ::core::ffi::c_int == CHAR_NL
                    && (*re).flags & PCRE2_HASCRORLF as uint32_t == 0 as uint32_t
                    && ((*mb).nltype == NLTYPE_ANY as uint32_t
                        || (*mb).nltype == NLTYPE_ANYCRLF as uint32_t
                        || (*mb).nllen == 2 as uint32_t)
                {
                    start_match = start_match.offset(1);
                }
                (*mb).mark = ::core::ptr::null::<PCRE2_UCHAR16>();
            }
        }
        match current_block {
            9914851455145855695 => {}
            _ => {}
        }
        if !(utf != 0
            && end_subject != true_end_subject
            && (rc == MATCH_NOMATCH || rc == PCRE2_ERROR_PARTIAL))
        {
            break;
        }
        loop {
            start_match = end_subject.offset(1 as ::core::ffi::c_int as isize);
            while start_match < true_end_subject
                && *start_match as ::core::ffi::c_uint & 0xfc00 as ::core::ffi::c_uint
                    == 0xdc00 as ::core::ffi::c_uint
            {
                start_match = start_match.offset(1);
            }
            if start_match >= true_end_subject {
                rc = MATCH_NOMATCH;
                match_partial = ::core::ptr::null::<PCRE2_UCHAR16>();
                break '_FRAGMENT_RESTART;
            } else {
                (*mb).check_subject = start_match;
                rc = _pcre2_valid_utf_16(
                    start_match,
                    length.wrapping_sub(
                        start_match.offset_from(subject) as isize as size_t
                    ),
                    &raw mut (*match_data).startchar,
                );
                if rc == 0 as ::core::ffi::c_int {
                    end_subject = true_end_subject;
                    (*mb).end_subject = end_subject;
                    fragment_options = PCRE2_NOTBOL as uint32_t;
                    break;
                } else {
                    if !(rc < 0 as ::core::ffi::c_int) {
                        continue;
                    }
                    end_subject = start_match.offset((*match_data).startchar as isize);
                    (*mb).end_subject = end_subject;
                    if !(end_subject > start_match) {
                        continue;
                    }
                    fragment_options = (PCRE2_NOTBOL | PCRE2_NOTEOL) as uint32_t;
                    break;
                }
            }
        }
    }
    (*match_data).code = re;
    (*match_data).mark = (*mb).mark;
    (*match_data).matchedby = PCRE2_MATCHEDBY_INTERPRETER as ::core::ffi::c_int as uint8_t;
    (*match_data).options = original_options;
    if rc == MATCH_MATCH {
        (*match_data).rc = if (*mb).end_offset_top as ::core::ffi::c_int
            >= 2 as ::core::ffi::c_int * (*match_data).oveccount as ::core::ffi::c_int
        {
            0 as ::core::ffi::c_int
        } else {
            (*mb).end_offset_top as ::core::ffi::c_int / 2 as ::core::ffi::c_int
                + 1 as ::core::ffi::c_int
        };
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        (*match_data).startchar = start_match.offset_from(subject) as isize as size_t;
        (*match_data).leftchar =
            (*mb).start_used_ptr.offset_from(subject) as isize as size_t;
        (*match_data).rightchar = (if (*mb).last_used_ptr > (*mb).end_match_ptr {
            (*mb).last_used_ptr
        } else {
            (*mb).end_match_ptr
        })
        .offset_from(subject) as isize as size_t;
        if options & PCRE2_COPY_MATCHED_SUBJECT as uint32_t != 0 as uint32_t {
            if length != 0 as size_t {
                (*match_data).subject = (*match_data)
                    .memctl
                    .malloc
                    .expect("non-null function pointer")(
                    length
                        .wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
                    (*match_data).memctl.memory_data,
                ) as PCRE2_SPTR16;
                if (*match_data).subject.is_null() {
                    (*match_data).rc = PCRE2_ERROR_NOMEMORY;
                    return (*match_data).rc;
                }
                memcpy(
                    (*match_data).subject as *mut ::core::ffi::c_void,
                    subject as *const ::core::ffi::c_void,
                    length
                        .wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
                );
            } else {
                (*match_data).subject = ::core::ptr::null::<PCRE2_UCHAR16>();
            }
            (*match_data).flags =
                ((*match_data).flags as ::core::ffi::c_uint | PCRE2_MD_COPIED_SUBJECT) as uint8_t;
        } else {
            (*match_data).subject = original_subject;
        }
        return (*match_data).rc;
    }
    (*match_data).mark = (*mb).nomatch_mark;
    if rc != MATCH_NOMATCH && rc != PCRE2_ERROR_PARTIAL {
        (*match_data).rc = rc;
    } else if !match_partial.is_null() {
        (*match_data).subject = original_subject;
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        (*match_data).ovector[0 as ::core::ffi::c_int as usize] =
            match_partial.offset_from(subject) as isize as size_t;
        (*match_data).ovector[1 as ::core::ffi::c_int as usize] =
            end_subject.offset_from(subject) as isize as size_t;
        (*match_data).startchar =
            match_partial.offset_from(subject) as isize as size_t;
        (*match_data).leftchar =
            start_partial.offset_from(subject) as isize as size_t;
        (*match_data).rightchar = end_subject.offset_from(subject) as isize as size_t;
        (*match_data).rc = PCRE2_ERROR_PARTIAL;
    } else {
        (*match_data).subject = original_subject;
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        (*match_data).rc = PCRE2_ERROR_NOMATCH;
    }
    return (*match_data).rc;
}
pub const FF: ::core::ffi::c_uint = PCRE2_NOTEMPTY_SET | PCRE2_NE_ATST_SET;
pub const OO: ::core::ffi::c_uint = PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART;
pub const PCRE2_CODE_UNIT_WIDTH: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
