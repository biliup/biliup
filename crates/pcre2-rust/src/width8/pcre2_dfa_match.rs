// Translated from PCRE2 10.49 pcre2_dfa_match.c; code unit width 8.
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
// /* This module contains the external function pcre2_dfa_match(), which is an
// alternative matching function that uses a sort of DFA algorithm (not a true
// FSM). This is NOT Perl-compatible, but it has advantages in certain
// applications. */
//
//
// /* NOTE ABOUT PERFORMANCE: A user of this function sent some code that improved
// the performance of his patterns greatly. I could not use it as it stood, as it
// was not thread safe, and made assumptions about pattern sizes. Also, it caused
// test 7 to loop, and test 9 to crash with a segfault.
//
// The issue is the check for duplicate states, which is done by a simple linear
// search up the state list. (Grep for "duplicate" below to find the code.) For
// many patterns, there will never be many states active at one time, so a simple
// linear search is fine. In patterns that have many active states, it might be a
// bottleneck. The suggested code used an indexing scheme to remember which states
// had previously been used for each character, and avoided the linear search when
// it knew there was no chance of a duplicate. This was implemented when adding
// states to the state lists.
//
// I wrote some thread-safe, not-limited code to try something similar at the time
// of checking for duplicates (instead of when adding states), using index vectors
// on the stack. It did give a 13% improvement with one specially constructed
// pattern for certain subject strings, but on other strings and on many of the
// simpler patterns in the test suite it did worse. The major problem, I think,
// was the extra time to initialize the index. This had to be done for each call
// of internal_dfa_match(). (The supplied patch used a static vector, initialized
// only once - I suspect this was the cause of the problems with the tests.)
//
// Overall, I concluded that the gains in some cases did not outweigh the losses
// in others, so I abandoned this code. */
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
    fn memchr(
        __s: *const ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    static _pcre2_OP_lengths_8: [uint8_t; 0];
    static mut _pcre2_default_match_context_8: pcre2_match_context_8;
    static _pcre2_ucd_boolprop_sets_8: [uint32_t; 0];
    static _pcre2_ucd_caseless_sets_8: [uint32_t; 0];
    static _pcre2_ucd_script_sets_8: [uint32_t; 0];
    static _pcre2_ucd_records_8: [ucd_record; 0];
    static _pcre2_ucd_stage1_8: [uint16_t; 0];
    static _pcre2_ucd_stage2_8: [uint16_t; 0];
    static _pcre2_ucp_gentype_8: [uint32_t; 0];
    fn _pcre2_extuni_8(
        _: uint32_t,
        _: PCRE2_SPTR8,
        _: PCRE2_SPTR8,
        _: PCRE2_SPTR8,
        _: BOOL,
        _: *mut ::core::ffi::c_int,
    ) -> PCRE2_SPTR8;
    fn _pcre2_is_newline_8(
        _: PCRE2_SPTR8,
        _: uint32_t,
        _: PCRE2_SPTR8,
        _: *mut uint32_t,
        _: BOOL,
    ) -> BOOL;
    fn _pcre2_strlen_8(_: PCRE2_SPTR8) -> size_t;
    fn _pcre2_valid_utf_8(_: PCRE2_SPTR8, _: size_t, _: *mut size_t) -> ::core::ffi::c_int;
    fn _pcre2_was_newline_8(
        _: PCRE2_SPTR8,
        _: uint32_t,
        _: PCRE2_SPTR8,
        _: *mut uint32_t,
        _: BOOL,
    ) -> BOOL;
    fn _pcre2_xclass_8(_: uint32_t, _: PCRE2_SPTR8, _: *const uint8_t, _: BOOL) -> BOOL;
    fn _pcre2_eclass_8(
        _: uint32_t,
        _: PCRE2_SPTR8,
        _: PCRE2_SPTR8,
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
pub struct pcre2_real_match_context_8 {
    pub memctl: pcre2_memctl,
    pub jit_callback: pcre2_jit_callback_8,
    pub jit_callback_data: *mut ::core::ffi::c_void,
    pub callout: Option<
        unsafe extern "C" fn(
            *mut pcre2_callout_block_8,
            *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
    pub callout_data: *mut ::core::ffi::c_void,
    pub substitute_callout: Option<
        unsafe extern "C" fn(
            *mut pcre2_substitute_callout_block_8,
            *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
    pub substitute_callout_data: *mut ::core::ffi::c_void,
    pub substitute_case_callout: Option<
        unsafe extern "C" fn(
            PCRE2_SPTR8,
            size_t,
            *mut PCRE2_UCHAR8,
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
pub struct pcre2_substitute_callout_block_8 {
    pub version: uint32_t,
    pub input: PCRE2_SPTR8,
    pub output: PCRE2_SPTR8,
    pub output_offsets: [size_t; 2],
    pub ovector: *mut size_t,
    pub oveccount: uint32_t,
    pub subscount: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_callout_block_8 {
    pub version: uint32_t,
    pub callout_number: uint32_t,
    pub capture_top: uint32_t,
    pub capture_last: uint32_t,
    pub offset_vector: *mut size_t,
    pub mark: PCRE2_SPTR8,
    pub subject: PCRE2_SPTR8,
    pub subject_length: size_t,
    pub start_match: size_t,
    pub current_position: size_t,
    pub pattern_position: size_t,
    pub next_item_length: size_t,
    pub callout_string_offset: size_t,
    pub callout_string_length: size_t,
    pub callout_string: PCRE2_SPTR8,
    pub callout_flags: uint32_t,
}
pub type pcre2_jit_callback_8 =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> *mut pcre2_jit_stack_8>;
pub type pcre2_jit_stack_8 = pcre2_real_jit_stack_8;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_real_jit_stack_8 {
    pub memctl: pcre2_memctl,
    pub stack: *mut ::core::ffi::c_void,
}
pub type pcre2_match_context_8 = pcre2_real_match_context_8;
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
pub struct pcre2_real_match_data_8 {
    pub memctl: pcre2_memctl,
    pub code: *const pcre2_real_code_8,
    pub subject: PCRE2_SPTR8,
    pub mark: PCRE2_SPTR8,
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
    pub ecode: PCRE2_SPTR8,
    pub back_frame: size_t,
    pub rdepth: uint32_t,
    pub group_frame_type: uint32_t,
    pub return_id: uint8_t,
    pub op: uint8_t,
    pub byte1: uint8_t,
    pub byte2: uint8_t,
    pub fields: C2RustUnnamed,
    pub eptr: PCRE2_SPTR8,
    pub start_match: PCRE2_SPTR8,
    pub mark: PCRE2_SPTR8,
    pub recurse_last_used: PCRE2_SPTR8,
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
    pub start_branch: PCRE2_SPTR8,
    pub length: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_2 {
    pub saved_end_subject: PCRE2_SPTR8,
    pub saved_eptr: PCRE2_SPTR8,
    pub true_end_extra: size_t,
    pub saved_moptions: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_3 {
    pub start_branch: PCRE2_SPTR8,
    pub frame_type: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_4 {
    pub start_eptr: PCRE2_SPTR8,
    pub start_group: PCRE2_SPTR8,
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
    pub start: PCRE2_SPTR8,
    pub offset: size_t,
    pub length: size_t,
    pub min: uint32_t,
    pub max: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_7 {
    pub start_eptr: PCRE2_SPTR8,
    pub min: uint32_t,
    pub max: uint32_t,
    pub ctype: uint32_t,
    pub propvalue: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_8 {
    pub start_eptr: PCRE2_SPTR8,
    pub eclass_data: PCRE2_SPTR8,
    pub eclass_len: size_t,
    pub min: uint32_t,
    pub max: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_9 {
    pub start_eptr: PCRE2_SPTR8,
    pub xclass_data: PCRE2_SPTR8,
    pub min: uint32_t,
    pub max: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_10 {
    pub start_eptr: PCRE2_SPTR8,
    pub byte_map_address: PCRE2_SPTR8,
    pub min: uint32_t,
    pub max: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_11 {
    pub start_eptr: PCRE2_SPTR8,
    pub min: uint32_t,
    pub max: uint32_t,
    pub c: uint32_t,
    pub oc: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_12 {
    pub start_eptr: PCRE2_SPTR8,
    pub charptr: PCRE2_SPTR8,
    pub min: uint32_t,
    pub max: uint32_t,
    pub c: uint32_t,
    pub oc: C2RustUnnamed_13,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_13 {
    pub oc: uint32_t,
    pub occu: [PCRE2_UCHAR8; 4],
}
pub type pcre2_match_data_8 = pcre2_real_match_data_8;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dfa_match_block_8 {
    pub memctl: pcre2_memctl,
    pub start_code: PCRE2_SPTR8,
    pub start_subject: PCRE2_SPTR8,
    pub end_subject: PCRE2_SPTR8,
    pub start_used_ptr: PCRE2_SPTR8,
    pub last_used_ptr: PCRE2_SPTR8,
    pub tables: *const uint8_t,
    pub start_offset: size_t,
    pub heap_limit: uint32_t,
    pub heap_used: size_t,
    pub match_limit: uint32_t,
    pub match_limit_depth: uint32_t,
    pub match_call_count: uint32_t,
    pub moptions: uint32_t,
    pub poptions: uint32_t,
    pub nltype: uint32_t,
    pub nllen: uint32_t,
    pub allowemptypartial: BOOL,
    pub nl: [PCRE2_UCHAR8; 4],
    pub bsr_convention: uint16_t,
    pub cb: *mut pcre2_callout_block_8,
    pub callout_data: *mut ::core::ffi::c_void,
    pub callout: Option<
        unsafe extern "C" fn(
            *mut pcre2_callout_block_8,
            *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
    pub recursive: *mut dfa_recursion_info,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dfa_recursion_info {
    pub prevrec: *mut dfa_recursion_info,
    pub subject_position: PCRE2_SPTR8,
    pub last_used_ptr: PCRE2_SPTR8,
    pub group_num: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RWS_anchor {
    pub next: *mut RWS_anchor,
    pub size: uint32_t,
    pub free: uint32_t,
}
// Recursion stores a pointer-bearing anchor in the integer workspace. Unlike
// an int array, this union guarantees the anchor's alignment on every target.
#[repr(C)]
union RWS_workspace {
    anchor: RWS_anchor,
    words: [::core::ffi::c_int; RWS_BASE_SIZE],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stateblock {
    pub offset: ::core::ffi::c_int,
    pub count: ::core::ffi::c_int,
    pub data: ::core::ffi::c_int,
}
pub const OP_CALLOUT: C2RustUnnamed_17 = 119;
pub const OP_CALLOUT_STR: C2RustUnnamed_17 = 120;
pub const OP_KETRMIN: C2RustUnnamed_17 = 124;
pub const OP_KETRMAX: C2RustUnnamed_17 = 123;
pub const OP_ALT: C2RustUnnamed_17 = 121;
pub const OP_ONCE: C2RustUnnamed_17 = 135;
pub const OP_BRAPOSZERO: C2RustUnnamed_17 = 155;
pub const OP_SCBRAPOS: C2RustUnnamed_17 = 145;
pub const OP_CBRAPOS: C2RustUnnamed_17 = 140;
pub const OP_SBRAPOS: C2RustUnnamed_17 = 143;
pub const OP_BRAPOS: C2RustUnnamed_17 = 138;
pub const OP_CREF: C2RustUnnamed_17 = 147;
pub const OP_RECURSE: C2RustUnnamed_17 = 118;
pub const OP_ASSERTBACK: C2RustUnnamed_17 = 130;
pub const OP_ASSERT: C2RustUnnamed_17 = 128;
pub const OP_RREF: C2RustUnnamed_17 = 149;
pub const OP_TRUE: C2RustUnnamed_17 = 152;
pub const OP_FAIL: C2RustUnnamed_17 = 165;
pub const OP_FALSE: C2RustUnnamed_17 = 151;
pub const OP_DNRREF: C2RustUnnamed_17 = 150;
pub const OP_DNCREF: C2RustUnnamed_17 = 148;
pub const OP_SCOND: C2RustUnnamed_17 = 146;
pub const OP_COND: C2RustUnnamed_17 = 141;
pub const OP_ASSERTBACK_NOT: C2RustUnnamed_17 = 131;
pub const OP_ASSERT_NOT: C2RustUnnamed_17 = 129;
pub const OP_CRPOSRANGE: C2RustUnnamed_17 = 109;
pub const OP_CRMINRANGE: C2RustUnnamed_17 = 105;
pub const OP_CRRANGE: C2RustUnnamed_17 = 104;
pub const OP_CRPOSQUERY: C2RustUnnamed_17 = 108;
pub const OP_CRMINQUERY: C2RustUnnamed_17 = 103;
pub const OP_CRQUERY: C2RustUnnamed_17 = 102;
pub const OP_CRPOSPLUS: C2RustUnnamed_17 = 107;
pub const OP_CRMINPLUS: C2RustUnnamed_17 = 101;
pub const OP_CRPLUS: C2RustUnnamed_17 = 100;
pub const OP_CRPOSSTAR: C2RustUnnamed_17 = 106;
pub const OP_CRMINSTAR: C2RustUnnamed_17 = 99;
pub const OP_CRSTAR: C2RustUnnamed_17 = 98;
pub const OP_NCLASS: C2RustUnnamed_17 = 111;
pub const OP_ECLASS: C2RustUnnamed_17 = 113;
pub const OP_XCLASS: C2RustUnnamed_17 = 112;
pub const OP_CLASS: C2RustUnnamed_17 = 110;
pub const OP_NOTPOSUPTO: C2RustUnnamed_17 = 71;
pub const OP_POSUPTO: C2RustUnnamed_17 = 45;
pub const OP_NOTSTAR: C2RustUnnamed_17 = 59;
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
pub const OP_NOTMINUPTO: C2RustUnnamed_17 = 66;
pub const OP_NOTUPTO: C2RustUnnamed_17 = 65;
pub const OP_MINUPTO: C2RustUnnamed_17 = 40;
pub const OP_UPTO: C2RustUnnamed_17 = 39;
pub const OP_STAR: C2RustUnnamed_17 = 33;
pub const OP_STARI: C2RustUnnamed_17 = 46;
pub const OP_NOTPOSUPTOI: C2RustUnnamed_17 = 84;
pub const OP_NOTMINUPTOI: C2RustUnnamed_17 = 79;
pub const OP_NOTUPTOI: C2RustUnnamed_17 = 78;
pub const OP_POSUPTOI: C2RustUnnamed_17 = 58;
pub const OP_MINUPTOI: C2RustUnnamed_17 = 53;
pub const OP_UPTOI: C2RustUnnamed_17 = 52;
pub const OP_NOTEXACT: C2RustUnnamed_17 = 67;
pub const OP_EXACT: C2RustUnnamed_17 = 41;
pub const OP_NOTEXACTI: C2RustUnnamed_17 = 80;
pub const OP_EXACTI: C2RustUnnamed_17 = 54;
pub const OP_NOTPOSSTAR: C2RustUnnamed_17 = 68;
pub const OP_POSSTAR: C2RustUnnamed_17 = 42;
pub const OP_NOTMINSTAR: C2RustUnnamed_17 = 60;
pub const OP_MINSTAR: C2RustUnnamed_17 = 34;
pub const OP_NOTPOSSTARI: C2RustUnnamed_17 = 81;
pub const OP_NOTMINSTARI: C2RustUnnamed_17 = 73;
pub const OP_NOTSTARI: C2RustUnnamed_17 = 72;
pub const OP_POSSTARI: C2RustUnnamed_17 = 55;
pub const OP_MINSTARI: C2RustUnnamed_17 = 47;
pub const OP_NOTPOSQUERY: C2RustUnnamed_17 = 70;
pub const OP_POSQUERY: C2RustUnnamed_17 = 44;
pub const OP_NOTMINQUERY: C2RustUnnamed_17 = 64;
pub const OP_NOTQUERY: C2RustUnnamed_17 = 63;
pub const OP_MINQUERY: C2RustUnnamed_17 = 38;
pub const OP_QUERY: C2RustUnnamed_17 = 37;
pub const OP_NOTPOSQUERYI: C2RustUnnamed_17 = 83;
pub const OP_NOTMINQUERYI: C2RustUnnamed_17 = 77;
pub const OP_NOTQUERYI: C2RustUnnamed_17 = 76;
pub const OP_POSQUERYI: C2RustUnnamed_17 = 57;
pub const OP_MINQUERYI: C2RustUnnamed_17 = 51;
pub const OP_QUERYI: C2RustUnnamed_17 = 50;
pub const OP_NOTPOSPLUS: C2RustUnnamed_17 = 69;
pub const OP_POSPLUS: C2RustUnnamed_17 = 43;
pub const OP_NOTMINPLUS: C2RustUnnamed_17 = 62;
pub const OP_NOTPLUS: C2RustUnnamed_17 = 61;
pub const OP_MINPLUS: C2RustUnnamed_17 = 36;
pub const OP_PLUS: C2RustUnnamed_17 = 35;
pub const OP_NOTPOSPLUSI: C2RustUnnamed_17 = 82;
pub const OP_NOTMINPLUSI: C2RustUnnamed_17 = 75;
pub const OP_NOTPLUSI: C2RustUnnamed_17 = 74;
pub const OP_POSPLUSI: C2RustUnnamed_17 = 56;
pub const OP_MINPLUSI: C2RustUnnamed_17 = 49;
pub const OP_PLUSI: C2RustUnnamed_17 = 48;
pub const OP_NOTI: C2RustUnnamed_17 = 32;
pub const OP_NOT: C2RustUnnamed_17 = 31;
pub const OP_HSPACE: C2RustUnnamed_17 = 19;
pub const OP_NOT_HSPACE: C2RustUnnamed_17 = 18;
pub const OP_VSPACE: C2RustUnnamed_17 = 21;
pub const OP_NOT_VSPACE: C2RustUnnamed_17 = 20;
pub const OP_ANYNL: C2RustUnnamed_17 = 17;
pub const OP_EXTUNI: C2RustUnnamed_17 = 22;
pub const OP_CHARI: C2RustUnnamed_17 = 30;
pub const OP_CHAR: C2RustUnnamed_17 = 29;
pub const OP_TYPEPOSUPTO: C2RustUnnamed_17 = 97;
pub const OP_TYPEEXACT: C2RustUnnamed_17 = 93;
pub const OP_TYPEMINUPTO: C2RustUnnamed_17 = 92;
pub const OP_TYPEUPTO: C2RustUnnamed_17 = 91;
pub const OP_PROP: C2RustUnnamed_17 = 16;
pub const ucp_Pc: C2RustUnnamed_15 = 16;
pub const ucp_Mn: C2RustUnnamed_15 = 12;
pub const ucp_N: C2RustUnnamed_14 = 3;
pub const ucp_L: C2RustUnnamed_14 = 1;
pub const ucp_Z: C2RustUnnamed_14 = 6;
pub const ucp_Lt: C2RustUnnamed_15 = 8;
pub const ucp_Ll: C2RustUnnamed_15 = 5;
pub const ucp_Lu: C2RustUnnamed_15 = 9;
pub const OP_TYPEPOSQUERY: C2RustUnnamed_17 = 96;
pub const OP_TYPEPOSSTAR: C2RustUnnamed_17 = 94;
pub const OP_TYPEMINSTAR: C2RustUnnamed_17 = 86;
pub const OP_TYPESTAR: C2RustUnnamed_17 = 85;
pub const OP_TYPEMINQUERY: C2RustUnnamed_17 = 90;
pub const OP_TYPEQUERY: C2RustUnnamed_17 = 89;
pub const OP_TYPEPOSPLUS: C2RustUnnamed_17 = 95;
pub const OP_TYPEMINPLUS: C2RustUnnamed_17 = 88;
pub const OP_TYPEPLUS: C2RustUnnamed_17 = 87;
pub const OP_ANY: C2RustUnnamed_17 = 12;
pub const OP_WORDCHAR: C2RustUnnamed_17 = 11;
pub const OP_WHITESPACE: C2RustUnnamed_17 = 9;
pub const OP_DIGIT: C2RustUnnamed_17 = 7;
pub const OP_NOTPROP: C2RustUnnamed_17 = 15;
pub const OP_NOT_UCP_WORD_BOUNDARY: C2RustUnnamed_17 = 171;
pub const OP_NOT_WORD_BOUNDARY: C2RustUnnamed_17 = 4;
pub const OP_UCP_WORD_BOUNDARY: C2RustUnnamed_17 = 172;
pub const OP_WORD_BOUNDARY: C2RustUnnamed_17 = 5;
pub const OP_NOT_WORDCHAR: C2RustUnnamed_17 = 10;
pub const OP_NOT_WHITESPACE: C2RustUnnamed_17 = 8;
pub const OP_NOT_DIGIT: C2RustUnnamed_17 = 6;
pub const OP_DOLLM: C2RustUnnamed_17 = 26;
pub const OP_DOLL: C2RustUnnamed_17 = 25;
pub const OP_EODN: C2RustUnnamed_17 = 23;
pub const OP_ALLANY: C2RustUnnamed_17 = 13;
pub const OP_SOM: C2RustUnnamed_17 = 2;
pub const OP_SOD: C2RustUnnamed_17 = 1;
pub const OP_EOD: C2RustUnnamed_17 = 24;
pub const OP_CIRCM: C2RustUnnamed_17 = 28;
pub const OP_CIRC: C2RustUnnamed_17 = 27;
pub const OP_SKIPZERO: C2RustUnnamed_17 = 169;
pub const OP_BRAMINZERO: C2RustUnnamed_17 = 154;
pub const OP_BRAZERO: C2RustUnnamed_17 = 153;
pub const OP_SCBRA: C2RustUnnamed_17 = 144;
pub const OP_CBRA: C2RustUnnamed_17 = 139;
pub const OP_SBRA: C2RustUnnamed_17 = 142;
pub const OP_BRA: C2RustUnnamed_17 = 137;
pub const OP_KET: C2RustUnnamed_17 = 122;
pub const OP_KETRPOS: C2RustUnnamed_17 = 125;
pub const OP_ANYBYTE: C2RustUnnamed_17 = 14;
pub const OP_REVERSE: C2RustUnnamed_17 = 126;
pub const PCRE2_MATCHEDBY_DFA_INTERPRETER: C2RustUnnamed_16 = 1;
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
pub const PCRE2_MATCHEDBY_INTERPRETER: C2RustUnnamed_16 = 0;
pub type C2RustUnnamed_17 = ::core::ffi::c_uint;
pub const OP_TABLE_LENGTH: C2RustUnnamed_17 = 173;
pub const OP_DEFINE: C2RustUnnamed_17 = 170;
pub const OP_CLOSE: C2RustUnnamed_17 = 168;
pub const OP_ASSERT_ACCEPT: C2RustUnnamed_17 = 167;
pub const OP_ACCEPT: C2RustUnnamed_17 = 166;
pub const OP_COMMIT_ARG: C2RustUnnamed_17 = 164;
pub const OP_COMMIT: C2RustUnnamed_17 = 163;
pub const OP_THEN_ARG: C2RustUnnamed_17 = 162;
pub const OP_THEN: C2RustUnnamed_17 = 161;
pub const OP_SKIP_ARG: C2RustUnnamed_17 = 160;
pub const OP_SKIP: C2RustUnnamed_17 = 159;
pub const OP_PRUNE_ARG: C2RustUnnamed_17 = 158;
pub const OP_PRUNE: C2RustUnnamed_17 = 157;
pub const OP_MARK: C2RustUnnamed_17 = 156;
pub const OP_SCRIPT_RUN: C2RustUnnamed_17 = 136;
pub const OP_ASSERT_SCS: C2RustUnnamed_17 = 134;
pub const OP_ASSERTBACK_NA: C2RustUnnamed_17 = 133;
pub const OP_ASSERT_NA: C2RustUnnamed_17 = 132;
pub const OP_VREVERSE: C2RustUnnamed_17 = 127;
pub const OP_DNREFI: C2RustUnnamed_17 = 117;
pub const OP_DNREF: C2RustUnnamed_17 = 116;
pub const OP_REFI: C2RustUnnamed_17 = 115;
pub const OP_REF: C2RustUnnamed_17 = 114;
pub const OP_SET_SOM: C2RustUnnamed_17 = 3;
pub const OP_END: C2RustUnnamed_17 = 0;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const UINT32_MAX: ::core::ffi::c_uint = 4294967295 as ::core::ffi::c_uint;
pub const PCRE2_ANCHORED: ::core::ffi::c_uint = 0x80000000 as ::core::ffi::c_uint;
pub const PCRE2_NO_UTF_CHECK: ::core::ffi::c_uint = 0x40000000 as ::core::ffi::c_uint;
pub const PCRE2_ENDANCHORED: ::core::ffi::c_uint = 0x20000000 as ::core::ffi::c_uint;
pub const PCRE2_DOLLAR_ENDONLY: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PCRE2_FIRSTLINE: ::core::ffi::c_uint = 0x100 as ::core::ffi::c_uint;
pub const PCRE2_UCP: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PCRE2_UTF: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PCRE2_ALT_CIRCUMFLEX: ::core::ffi::c_uint = 0x200000 as ::core::ffi::c_uint;
pub const PCRE2_USE_OFFSET_LIMIT: ::core::ffi::c_uint = 0x800000 as ::core::ffi::c_uint;
pub const PCRE2_MATCH_INVALID_UTF: ::core::ffi::c_uint = 0x4000000 as ::core::ffi::c_uint;
pub const PCRE2_NOTBOL: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PCRE2_NOTEOL: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PCRE2_NOTEMPTY: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PCRE2_NOTEMPTY_ATSTART: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PCRE2_PARTIAL_SOFT: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PCRE2_PARTIAL_HARD: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PCRE2_DFA_RESTART: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PCRE2_DFA_SHORTEST: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PCRE2_COPY_MATCHED_SUBJECT: ::core::ffi::c_uint = 0x4000 as ::core::ffi::c_uint;
pub const PCRE2_NEWLINE_CR: ::core::ffi::c_int = 1;
pub const PCRE2_NEWLINE_LF: ::core::ffi::c_int = 2;
pub const PCRE2_NEWLINE_CRLF: ::core::ffi::c_int = 3;
pub const PCRE2_NEWLINE_ANY: ::core::ffi::c_int = 4;
pub const PCRE2_NEWLINE_ANYCRLF: ::core::ffi::c_int = 5;
pub const PCRE2_NEWLINE_NUL: ::core::ffi::c_int = 6;
pub const PCRE2_BSR_ANYCRLF: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PCRE2_ERROR_NOMATCH: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const PCRE2_ERROR_PARTIAL: ::core::ffi::c_int = -(2 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADMAGIC: ::core::ffi::c_int = -(31 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADMODE: ::core::ffi::c_int = -(32 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADOFFSET: ::core::ffi::c_int = -(33 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADOPTION: ::core::ffi::c_int = -(34 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADUTFOFFSET: ::core::ffi::c_int = -(36 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DFA_BADRESTART: ::core::ffi::c_int = -(38 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DFA_RECURSE: ::core::ffi::c_int = -(39 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DFA_UCOND: ::core::ffi::c_int = -(40 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DFA_UITEM: ::core::ffi::c_int = -(42 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DFA_WSSIZE: ::core::ffi::c_int = -(43 as ::core::ffi::c_int);
pub const PCRE2_ERROR_INTERNAL: ::core::ffi::c_int = -(44 as ::core::ffi::c_int);
pub const PCRE2_ERROR_MATCHLIMIT: ::core::ffi::c_int = -(47 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NOMEMORY: ::core::ffi::c_int = -(48 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NULL: ::core::ffi::c_int = -(51 as ::core::ffi::c_int);
pub const PCRE2_ERROR_RECURSELOOP: ::core::ffi::c_int = -(52 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DEPTHLIMIT: ::core::ffi::c_int = -(53 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADOFFSETLIMIT: ::core::ffi::c_int = -(56 as ::core::ffi::c_int);
pub const PCRE2_ERROR_HEAPLIMIT: ::core::ffi::c_int = -(63 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DFA_UINVALID_UTF: ::core::ffi::c_int = -(66 as ::core::ffi::c_int);
pub const PCRE2_ZERO_TERMINATED: size_t = !(0 as ::core::ffi::c_int as size_t);
pub const PCRE2_UNSET: size_t = !(0 as ::core::ffi::c_int as size_t);
pub const NOTACHAR: ::core::ffi::c_uint = 0xffffffff as ::core::ffi::c_uint;
pub const DFA_START_RWS_SIZE: ::core::ffi::c_int = 30720 as ::core::ffi::c_int;
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
pub const PCRE2_MATCH_EMPTY: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PCRE2_NOTEMPTY_SET: ::core::ffi::c_uint = 0x10000 as ::core::ffi::c_uint;
pub const PCRE2_NE_ATST_SET: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PCRE2_MODE_MASK: ::core::ffi::c_uint = PCRE2_MODE8 | PCRE2_MODE16 | PCRE2_MODE32;
pub const PCRE2_MD_COPIED_SUBJECT: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const MAGIC_NUMBER: usize = 0x50435245 as usize;
pub const REQ_CU_MAX: ::core::ffi::c_int = 5000 as ::core::ffi::c_int;
pub const cbit_length: ::core::ffi::c_int = 320 as ::core::ffi::c_int;
pub const ctype_space: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const ctype_digit: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const ctype_word: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const lcc_offset: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const fcc_offset: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const cbits_offset: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
pub const ctypes_offset: ::core::ffi::c_int = cbits_offset + cbit_length;
pub const PCRE2_OPTIM_START_OPTIMIZE: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const CHAR_HT: uint32_t = 9 as uint32_t;
pub const CHAR_VT: uint32_t = 11 as uint32_t;
pub const CHAR_FF: uint32_t = 12 as uint32_t;
pub const CHAR_CR: ::core::ffi::c_int = '\r' as i32;
pub const CHAR_LF: ::core::ffi::c_int = '\n' as i32;
pub const CHAR_NL: ::core::ffi::c_int = CHAR_LF;
pub const CHAR_NEL: uint32_t = 133 as uint32_t;
pub const CHAR_NUL: ::core::ffi::c_int = '\0' as i32;
pub const CHAR_SPACE: uint32_t = 32 as uint32_t;
pub const CHAR_DOLLAR_SIGN: ::core::ffi::c_int = '$' as i32;
pub const CHAR_COMMERCIAL_AT: ::core::ffi::c_int = '@' as i32;
pub const CHAR_GRAVE_ACCENT: ::core::ffi::c_int = '`' as i32;
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
pub const PT_CLIST: ::core::ffi::c_int = 9;
pub const PT_UCNC: ::core::ffi::c_int = 10;
pub const PT_BIDICL: ::core::ffi::c_int = 11;
pub const PT_BOOL: ::core::ffi::c_int = 12;
pub const RREF_ANY: ::core::ffi::c_int = 0xffff as ::core::ffi::c_int;
pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const UCD_BIDICLASS_SHIFT: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const IMM2_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PUBLIC_DFA_MATCH_OPTIONS: ::core::ffi::c_uint = PCRE2_ANCHORED
    | PCRE2_ENDANCHORED
    | PCRE2_NOTBOL
    | PCRE2_NOTEOL
    | PCRE2_NOTEMPTY
    | PCRE2_NOTEMPTY_ATSTART
    | PCRE2_NO_UTF_CHECK
    | PCRE2_PARTIAL_HARD
    | PCRE2_PARTIAL_SOFT
    | PCRE2_DFA_SHORTEST
    | PCRE2_DFA_RESTART
    | PCRE2_COPY_MATCHED_SUBJECT;
pub const OP_PROP_EXTRA: ::core::ffi::c_int = 300 as ::core::ffi::c_int;
pub const OP_EXTUNI_EXTRA: ::core::ffi::c_int = 320 as ::core::ffi::c_int;
pub const OP_ANYNL_EXTRA: ::core::ffi::c_int = 340 as ::core::ffi::c_int;
pub const OP_HSPACE_EXTRA: ::core::ffi::c_int = 360 as ::core::ffi::c_int;
pub const OP_VSPACE_EXTRA: ::core::ffi::c_int = 380 as ::core::ffi::c_int;
static mut coptable: [uint8_t; 173] = [
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
    0 as ::core::ffi::c_int as uint8_t,
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
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
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
];
static mut poptable: [uint8_t; 173] = [
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
    1 as ::core::ffi::c_int as uint8_t,
];
static mut toptable1: [uint8_t; 14] = [
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    ctype_digit as uint8_t,
    ctype_digit as uint8_t,
    ctype_space as uint8_t,
    ctype_space as uint8_t,
    ctype_word as uint8_t,
    ctype_word as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
];
static mut toptable2: [uint8_t; 14] = [
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    ctype_digit as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    ctype_space as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    ctype_word as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
];
pub const INTS_PER_STATEBLOCK: ::core::ffi::c_int = (::core::mem::size_of::<stateblock>() as usize)
    .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as usize)
    as ::core::ffi::c_int;
pub const OVEC_UNIT: usize = (::core::mem::size_of::<size_t>() as usize)
    .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as usize);
pub const RWS_BASE_SIZE: usize = (DFA_START_RWS_SIZE as usize)
    .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as usize);
pub const RWS_RSIZE: ::core::ffi::c_int = 1000 as ::core::ffi::c_int;
pub const RWS_OVEC_RSIZE: usize = (1000 as usize).wrapping_mul(OVEC_UNIT);
pub const RWS_OVEC_OSIZE: usize = (2 as usize).wrapping_mul(OVEC_UNIT);
pub const RWS_ANCHOR_SIZE: usize = (::core::mem::size_of::<RWS_anchor>() as usize)
    .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as usize);
unsafe extern "C" fn do_callout_dfa(
    mut code: PCRE2_SPTR8,
    mut offsets: *mut size_t,
    mut current_subject: PCRE2_SPTR8,
    mut ptr: PCRE2_SPTR8,
    mut mb: *mut dfa_match_block_8,
    mut extracode: size_t,
    mut lengthptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut cb: *mut pcre2_callout_block_8 = (*mb).cb;
    *lengthptr = if *code.offset(extracode as isize) as ::core::ffi::c_int
        == OP_CALLOUT as ::core::ffi::c_int
    {
        *(&raw const _pcre2_OP_lengths_8 as *const uint8_t)
            .offset(OP_CALLOUT as ::core::ffi::c_int as isize) as size_t
    } else {
        ((*code.offset(
            ((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                as size_t)
                .wrapping_add(extracode) as isize,
        ) as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *code.offset(
                ((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                    as size_t)
                    .wrapping_add(extracode)
                    .wrapping_add(1 as size_t) as isize,
            ) as ::core::ffi::c_int) as ::core::ffi::c_uint as size_t
    };
    if (*mb).callout.is_none() {
        return 0 as ::core::ffi::c_int;
    }
    (*cb).offset_vector = offsets;
    (*cb).start_match =
        current_subject.offset_from((*mb).start_subject) as isize as size_t;
    (*cb).current_position = ptr.offset_from((*mb).start_subject) as isize as size_t;
    (*cb).pattern_position = ((*code.offset((1 as size_t).wrapping_add(extracode) as isize)
        as ::core::ffi::c_int)
        << 8 as ::core::ffi::c_int
        | *code.offset(
            (1 as size_t)
                .wrapping_add(extracode)
                .wrapping_add(1 as size_t) as isize,
        ) as ::core::ffi::c_int) as ::core::ffi::c_uint as size_t;
    (*cb).next_item_length = ((*code.offset(
        ((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as size_t).wrapping_add(extracode)
            as isize,
    ) as ::core::ffi::c_int)
        << 8 as ::core::ffi::c_int
        | *code.offset(
            ((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as size_t)
                .wrapping_add(extracode)
                .wrapping_add(1 as size_t) as isize,
        ) as ::core::ffi::c_int) as ::core::ffi::c_uint as size_t;
    if *code.offset(extracode as isize) as ::core::ffi::c_int == OP_CALLOUT as ::core::ffi::c_int {
        (*cb).callout_number = *code.offset(
            ((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * LINK_SIZE) as size_t)
                .wrapping_add(extracode) as isize,
        ) as uint32_t;
        (*cb).callout_string_offset = 0 as size_t;
        (*cb).callout_string = ::core::ptr::null::<PCRE2_UCHAR8>();
        (*cb).callout_string_length = 0 as size_t;
    } else {
        (*cb).callout_number = 0 as uint32_t;
        (*cb).callout_string_offset = ((*code.offset(
            ((1 as ::core::ffi::c_int + 3 as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                as size_t)
                .wrapping_add(extracode) as isize,
        ) as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *code.offset(
                ((1 as ::core::ffi::c_int + 3 as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                    as size_t)
                    .wrapping_add(extracode)
                    .wrapping_add(1 as size_t) as isize,
            ) as ::core::ffi::c_int) as ::core::ffi::c_uint
            as size_t;
        (*cb).callout_string = code
            .offset(
                ((1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * LINK_SIZE) as size_t)
                    .wrapping_add(extracode) as isize,
            )
            .offset(1 as ::core::ffi::c_int as isize);
        (*cb).callout_string_length = (*lengthptr)
            .wrapping_sub((1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * LINK_SIZE) as size_t)
            .wrapping_sub(2 as size_t);
    }
    return (*mb).callout.expect("non-null function pointer")(cb, (*mb).callout_data);
}
unsafe extern "C" fn more_workspace(
    mut rwsptr: *mut *mut RWS_anchor,
    mut ovecsize: ::core::ffi::c_uint,
    mut mb: *mut dfa_match_block_8,
) -> ::core::ffi::c_int {
    let mut rws: *mut RWS_anchor = *rwsptr;
    let mut new: *mut RWS_anchor = ::core::ptr::null_mut::<RWS_anchor>();
    let mut requested: uint32_t = 0;
    requested = ((RWS_RSIZE as ::core::ffi::c_uint).wrapping_add(ovecsize) as usize)
        .wrapping_add(RWS_ANCHOR_SIZE) as uint32_t;
    if !(*rws).next.is_null() {
        if (*(*rws).next).size < requested {
            return PCRE2_ERROR_HEAPLIMIT;
        }
        new = (*rws).next as *mut RWS_anchor;
    } else {
        let mut newsize: uint32_t = (if (*rws).size as usize
            >= (UINT32_MAX as usize)
                .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as usize)
                .wrapping_div(2 as usize)
        {
            (UINT32_MAX as usize)
                .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as usize)
        } else {
            (*rws).size.wrapping_mul(2 as uint32_t) as usize
        }) as uint32_t;
        let mut newsizeK: uint32_t = (newsize as usize).wrapping_div(
            (1024 as usize).wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as usize),
        ) as uint32_t;
        if (*mb).heap_used >= (*mb).heap_limit as size_t {
            newsize = 0 as uint32_t;
            newsizeK = 0 as uint32_t;
        } else {
            let mut availableK: size_t = ((*mb).heap_limit as size_t).wrapping_sub((*mb).heap_used);
            if newsizeK as size_t > availableK {
                newsize = availableK.wrapping_mul(
                    (1024 as size_t)
                        .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as size_t),
                ) as uint32_t;
                newsizeK = availableK as uint32_t;
            }
        }
        if newsize < requested {
            return PCRE2_ERROR_HEAPLIMIT;
        }
        new = (*mb).memctl.malloc.expect("non-null function pointer")(
            (newsize as size_t)
                .wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as size_t),
            (*mb).memctl.memory_data,
        ) as *mut RWS_anchor;
        if new.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }
        (*mb).heap_used = (*mb).heap_used.wrapping_add(newsizeK as size_t);
        (*new).next = ::core::ptr::null_mut::<RWS_anchor>();
        (*new).size = newsize;
        (*rws).next = new as *mut RWS_anchor;
    }
    (*new).free = ((*new).size as usize).wrapping_sub(RWS_ANCHOR_SIZE) as uint32_t;
    *rwsptr = new;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn internal_dfa_match(
    mut mb: *mut dfa_match_block_8,
    mut this_start_code: PCRE2_SPTR8,
    mut current_subject: PCRE2_SPTR8,
    mut start_offset: size_t,
    mut offsets: *mut size_t,
    mut offsetcount: uint32_t,
    mut workspace: *mut ::core::ffi::c_int,
    mut wscount: ::core::ffi::c_int,
    mut rlevel: uint32_t,
    mut RWS: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut active_states: *mut stateblock = ::core::ptr::null_mut::<stateblock>();
    let mut new_states: *mut stateblock = ::core::ptr::null_mut::<stateblock>();
    let mut temp_states: *mut stateblock = ::core::ptr::null_mut::<stateblock>();
    let mut next_active_state: *mut stateblock = ::core::ptr::null_mut::<stateblock>();
    let mut next_new_state: *mut stateblock = ::core::ptr::null_mut::<stateblock>();
    let mut ctypes: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut lcc: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut fcc: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut ptr: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut end_code: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut new_recursive: dfa_recursion_info = dfa_recursion_info {
        prevrec: ::core::ptr::null_mut::<dfa_recursion_info>(),
        subject_position: ::core::ptr::null::<PCRE2_UCHAR8>(),
        last_used_ptr: ::core::ptr::null::<PCRE2_UCHAR8>(),
        group_num: 0,
    };
    let mut active_count: ::core::ffi::c_int = 0;
    let mut new_count: ::core::ffi::c_int = 0;
    let mut match_count: ::core::ffi::c_int = 0;
    let mut start_subject: PCRE2_SPTR8 = (*mb).start_subject;
    let mut end_subject: PCRE2_SPTR8 = (*mb).end_subject;
    let mut start_code: PCRE2_SPTR8 = (*mb).start_code;
    let mut utf: BOOL =
        ((*mb).poptions & PCRE2_UTF as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    let mut utf_or_ucp: BOOL =
        (utf != 0 || (*mb).poptions & PCRE2_UCP as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    let mut reset_could_continue: BOOL = FALSE;
    let fresh0 = (*mb).match_call_count;
    (*mb).match_call_count = (*mb).match_call_count.wrapping_add(1);
    if fresh0 >= (*mb).match_limit {
        return PCRE2_ERROR_MATCHLIMIT;
    }
    let fresh1 = rlevel;
    rlevel = rlevel.wrapping_add(1);
    if fresh1 > (*mb).match_limit_depth {
        return PCRE2_ERROR_DEPTHLIMIT;
    }
    offsetcount &= -(2 as ::core::ffi::c_int) as uint32_t;
    wscount -= 2 as ::core::ffi::c_int;
    wscount = (wscount - wscount % (INTS_PER_STATEBLOCK * 2 as ::core::ffi::c_int))
        / (2 as ::core::ffi::c_int * INTS_PER_STATEBLOCK);
    ctypes = (*mb).tables.offset(ctypes_offset as isize);
    lcc = (*mb).tables.offset(lcc_offset as isize);
    fcc = (*mb).tables.offset(fcc_offset as isize);
    match_count = PCRE2_ERROR_NOMATCH;
    active_states = workspace.offset(2 as ::core::ffi::c_int as isize) as *mut stateblock;
    new_states = active_states.offset(wscount as isize);
    next_new_state = new_states;
    new_count = 0 as ::core::ffi::c_int;
    if *this_start_code as ::core::ffi::c_int == OP_ASSERTBACK as ::core::ffi::c_int
        || *this_start_code as ::core::ffi::c_int == OP_ASSERTBACK_NOT as ::core::ffi::c_int
    {
        let mut max_back: size_t = 0 as size_t;
        let mut gone_back: size_t = 0;
        end_code = this_start_code;
        loop {
            let mut back: size_t = ((*end_code
                .offset((2 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int)
                << 8 as ::core::ffi::c_int
                | *end_code.offset(
                    (2 as ::core::ffi::c_int + 2 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                        as isize,
                ) as ::core::ffi::c_int) as ::core::ffi::c_uint
                as size_t;
            if back > max_back {
                max_back = back;
            }
            end_code = end_code.offset(
                ((*end_code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *end_code.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                        as ::core::ffi::c_int) as ::core::ffi::c_uint as isize,
            );
            if !(*end_code as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                break;
            }
        }
        if utf != 0 {
            gone_back = 0 as size_t;
            while gone_back < max_back {
                if current_subject <= start_subject {
                    break;
                }
                current_subject = current_subject.offset(-1);
                while current_subject > start_subject
                    && *current_subject as ::core::ffi::c_uint & 0xc0 as ::core::ffi::c_uint
                        == 0x80 as ::core::ffi::c_uint
                {
                    current_subject = current_subject.offset(-1);
                }
                gone_back = gone_back.wrapping_add(1);
            }
        } else {
            let mut current_offset: size_t =
                current_subject.offset_from(start_subject) as isize as size_t;
            gone_back = if current_offset < max_back {
                current_offset
            } else {
                max_back
            };
            current_subject = current_subject.offset(-(gone_back as isize));
        }
        if current_subject < (*mb).start_used_ptr {
            (*mb).start_used_ptr = current_subject;
        }
        end_code = this_start_code;
        loop {
            let mut revlen: uint32_t = (if *end_code
                .offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize)
                as ::core::ffi::c_int
                == OP_REVERSE as ::core::ffi::c_int
            {
                1 as ::core::ffi::c_int + IMM2_SIZE
            } else {
                0 as ::core::ffi::c_int
            }) as uint32_t;
            let mut back_0: size_t = if revlen == 0 as uint32_t {
                0 as size_t
            } else {
                ((*end_code.offset((2 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as isize)
                    as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *end_code.offset(
                        (2 as ::core::ffi::c_int
                            + 2 as ::core::ffi::c_int
                            + 1 as ::core::ffi::c_int) as isize,
                    ) as ::core::ffi::c_int) as ::core::ffi::c_uint as size_t
            };
            if back_0 <= gone_back {
                let mut bstate: ::core::ffi::c_int =
                    (end_code.offset_from(start_code) as isize
                        + 1 as isize
                        + LINK_SIZE as isize
                        + revlen as isize) as ::core::ffi::c_int;
                let fresh2 = new_count;
                new_count = new_count + 1;
                if fresh2 < wscount {
                    (*next_new_state).offset = -bstate;
                    (*next_new_state).count = 0 as ::core::ffi::c_int;
                    (*next_new_state).data = gone_back.wrapping_sub(back_0) as ::core::ffi::c_int;
                    next_new_state = next_new_state.offset(1);
                } else {
                    return PCRE2_ERROR_DFA_WSSIZE;
                }
            }
            end_code = end_code.offset(
                ((*end_code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *end_code.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                        as ::core::ffi::c_int) as ::core::ffi::c_uint as isize,
            );
            if !(*end_code as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                break;
            }
        }
    } else {
        end_code = this_start_code;
        if rlevel == 1 as uint32_t
            && (*mb).moptions & PCRE2_DFA_RESTART as uint32_t != 0 as uint32_t
        {
            loop {
                end_code = end_code.offset(
                    ((*end_code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *end_code
                            .offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint
                        as isize,
                );
                if !(*end_code as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                    break;
                }
            }
            new_count = *workspace.offset(1 as ::core::ffi::c_int as isize);
            if *workspace.offset(0 as ::core::ffi::c_int as isize) == 0 {
                memcpy(
                    new_states as *mut ::core::ffi::c_void,
                    active_states as *const ::core::ffi::c_void,
                    (new_count as size_t)
                        .wrapping_mul(::core::mem::size_of::<stateblock>() as size_t),
                );
            }
        } else {
            let mut length: ::core::ffi::c_int = 1 as ::core::ffi::c_int
                + LINK_SIZE
                + (if *this_start_code as ::core::ffi::c_int == OP_CBRA as ::core::ffi::c_int
                    || *this_start_code as ::core::ffi::c_int == OP_SCBRA as ::core::ffi::c_int
                    || *this_start_code as ::core::ffi::c_int == OP_CBRAPOS as ::core::ffi::c_int
                    || *this_start_code as ::core::ffi::c_int == OP_SCBRAPOS as ::core::ffi::c_int
                {
                    IMM2_SIZE
                } else {
                    0 as ::core::ffi::c_int
                });
            loop {
                let fresh3 = new_count;
                new_count = new_count + 1;
                if fresh3 < wscount {
                    (*next_new_state).offset = (end_code.offset_from(start_code)
                        as isize
                        + length as isize)
                        as ::core::ffi::c_int;
                    (*next_new_state).count = 0 as ::core::ffi::c_int;
                    next_new_state = next_new_state.offset(1);
                } else {
                    return PCRE2_ERROR_DFA_WSSIZE;
                }
                end_code = end_code.offset(
                    ((*end_code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *end_code
                            .offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint
                        as isize,
                );
                length = 1 as ::core::ffi::c_int + LINK_SIZE;
                if !(*end_code as ::core::ffi::c_int == OP_ALT as ::core::ffi::c_int) {
                    break;
                }
            }
        }
    }
    *workspace.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_int;
    ptr = current_subject;
    loop {
        let mut i: ::core::ffi::c_int = 0;
        let mut j: ::core::ffi::c_int = 0;
        let mut clen: ::core::ffi::c_int = 0;
        let mut dlen: ::core::ffi::c_int = 0;
        let mut c: uint32_t = 0;
        let mut d: uint32_t = 0;
        let mut partial_newline: BOOL = FALSE;
        let mut could_continue: BOOL = reset_could_continue;
        reset_could_continue = FALSE as BOOL;
        if ptr > (*mb).last_used_ptr {
            (*mb).last_used_ptr = ptr;
        }
        temp_states = active_states;
        active_states = new_states;
        new_states = temp_states;
        active_count = new_count;
        new_count = 0 as ::core::ffi::c_int;
        *workspace.offset(0 as ::core::ffi::c_int as isize) ^= 1 as ::core::ffi::c_int;
        *workspace.offset(1 as ::core::ffi::c_int as isize) = active_count;
        next_active_state = active_states.offset(active_count as isize);
        next_new_state = new_states;
        if ptr < end_subject {
            clen = 1 as ::core::ffi::c_int;
            c = *ptr as uint32_t;
            if utf != 0 && c >= 0xc0 as uint32_t {
                if c & 0x20 as uint32_t == 0 as uint32_t {
                    c = (c & 0x1f as uint32_t) << 6 as ::core::ffi::c_int
                        | *ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t;
                    clen += 1;
                } else if c & 0x10 as uint32_t == 0 as uint32_t {
                    c = (c & 0xf as uint32_t) << 12 as ::core::ffi::c_int
                        | (*ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 6 as ::core::ffi::c_int
                        | *ptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t;
                    clen += 2 as ::core::ffi::c_int;
                } else if c & 0x8 as uint32_t == 0 as uint32_t {
                    c = (c & 0x7 as uint32_t) << 18 as ::core::ffi::c_int
                        | (*ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 12 as ::core::ffi::c_int
                        | (*ptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 6 as ::core::ffi::c_int
                        | *ptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t;
                    clen += 3 as ::core::ffi::c_int;
                } else if c & 0x4 as uint32_t == 0 as uint32_t {
                    c = (c & 0x3 as uint32_t) << 24 as ::core::ffi::c_int
                        | (*ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 18 as ::core::ffi::c_int
                        | (*ptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 12 as ::core::ffi::c_int
                        | (*ptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 6 as ::core::ffi::c_int
                        | *ptr.offset(4 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t;
                    clen += 4 as ::core::ffi::c_int;
                } else {
                    c = (c & 0x1 as uint32_t) << 30 as ::core::ffi::c_int
                        | (*ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 24 as ::core::ffi::c_int
                        | (*ptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 18 as ::core::ffi::c_int
                        | (*ptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 12 as ::core::ffi::c_int
                        | (*ptr.offset(4 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 6 as ::core::ffi::c_int
                        | *ptr.offset(5 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t;
                    clen += 5 as ::core::ffi::c_int;
                }
            }
        } else {
            clen = 0 as ::core::ffi::c_int;
            c = NOTACHAR as uint32_t;
        }
        let mut current_block_1815: u64;
        i = 0 as ::core::ffi::c_int;
        while i < active_count {
            let mut current_state: *mut stateblock = active_states.offset(i as isize);
            let mut caseless: BOOL = FALSE;
            let mut code: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
            let mut codevalue: uint32_t = 0;
            let mut state_offset: ::core::ffi::c_int = (*current_state).offset;
            let mut rrc: ::core::ffi::c_int = 0;
            let mut count: ::core::ffi::c_int = 0;
            if state_offset < 0 as ::core::ffi::c_int {
                if (*current_state).data > 0 as ::core::ffi::c_int {
                    let fresh4 = new_count;
                    new_count = new_count + 1;
                    if fresh4 < wscount {
                        (*next_new_state).offset = state_offset;
                        (*next_new_state).count = (*current_state).count;
                        (*next_new_state).data = (*current_state).data - 1 as ::core::ffi::c_int;
                        next_new_state = next_new_state.offset(1);
                    } else {
                        return PCRE2_ERROR_DFA_WSSIZE;
                    }
                    if could_continue != 0 {
                        reset_could_continue = TRUE as BOOL;
                    }
                    current_block_1815 = 8937240710477387595;
                } else {
                    state_offset = -state_offset;
                    (*current_state).offset = state_offset;
                    current_block_1815 = 7019009297990327870;
                }
            } else {
                current_block_1815 = 7019009297990327870;
            }
            match current_block_1815 {
                7019009297990327870 => {
                    j = 0 as ::core::ffi::c_int;
                    loop {
                        if !(j < i) {
                            current_block_1815 = 6665878751423064961;
                            break;
                        }
                        if (*active_states.offset(j as isize)).offset == state_offset
                            && (*active_states.offset(j as isize)).count == (*current_state).count
                        {
                            current_block_1815 = 8937240710477387595;
                            break;
                        }
                        j += 1;
                    }
                    match current_block_1815 {
                        8937240710477387595 => {}
                        _ => {
                            code = start_code.offset(state_offset as isize);
                            codevalue = *code as uint32_t;
                            if clen == 0 as ::core::ffi::c_int
                                && poptable[codevalue as usize] as ::core::ffi::c_int
                                    != 0 as ::core::ffi::c_int
                            {
                                could_continue = TRUE as BOOL;
                            }
                            if coptable[codevalue as usize] as ::core::ffi::c_int
                                > 0 as ::core::ffi::c_int
                            {
                                dlen = 1 as ::core::ffi::c_int;
                                if utf != 0 {
                                    d = *code.offset(
                                        coptable[codevalue as usize] as ::core::ffi::c_int as isize,
                                    ) as uint32_t;
                                    if d >= 0xc0 as uint32_t {
                                        if d & 0x20 as uint32_t == 0 as uint32_t {
                                            d = (d & 0x1f as uint32_t) << 6 as ::core::ffi::c_int
                                                | *code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t;
                                            dlen += 1;
                                        } else if d & 0x10 as uint32_t == 0 as uint32_t {
                                            d = (d & 0xf as uint32_t) << 12 as ::core::ffi::c_int
                                                | (*code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t)
                                                    << 6 as ::core::ffi::c_int
                                                | *code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(2 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t;
                                            dlen += 2 as ::core::ffi::c_int;
                                        } else if d & 0x8 as uint32_t == 0 as uint32_t {
                                            d = (d & 0x7 as uint32_t) << 18 as ::core::ffi::c_int
                                                | (*code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t)
                                                    << 12 as ::core::ffi::c_int
                                                | (*code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(2 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t)
                                                    << 6 as ::core::ffi::c_int
                                                | *code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(3 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t;
                                            dlen += 3 as ::core::ffi::c_int;
                                        } else if d & 0x4 as uint32_t == 0 as uint32_t {
                                            d = (d & 0x3 as uint32_t) << 24 as ::core::ffi::c_int
                                                | (*code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t)
                                                    << 18 as ::core::ffi::c_int
                                                | (*code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(2 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t)
                                                    << 12 as ::core::ffi::c_int
                                                | (*code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(3 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t)
                                                    << 6 as ::core::ffi::c_int
                                                | *code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(4 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t;
                                            dlen += 4 as ::core::ffi::c_int;
                                        } else {
                                            d = (d & 0x1 as uint32_t) << 30 as ::core::ffi::c_int
                                                | (*code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t)
                                                    << 24 as ::core::ffi::c_int
                                                | (*code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(2 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t)
                                                    << 18 as ::core::ffi::c_int
                                                | (*code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(3 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t)
                                                    << 12 as ::core::ffi::c_int
                                                | (*code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(4 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t)
                                                    << 6 as ::core::ffi::c_int
                                                | *code
                                                    .offset(
                                                        coptable[codevalue as usize]
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    )
                                                    .offset(5 as ::core::ffi::c_int as isize)
                                                    as uint32_t
                                                    & 0x3f as uint32_t;
                                            dlen += 5 as ::core::ffi::c_int;
                                        }
                                    }
                                } else {
                                    d = *code.offset(coptable[codevalue as usize] as isize)
                                        as uint32_t;
                                }
                                if codevalue >= OP_TYPESTAR as ::core::ffi::c_int as uint32_t {
                                    match d {
                                        14 => return PCRE2_ERROR_DFA_UITEM,
                                        15 | 16 => {
                                            codevalue =
                                                codevalue.wrapping_add(OP_PROP_EXTRA as uint32_t);
                                        }
                                        17 => {
                                            codevalue =
                                                codevalue.wrapping_add(OP_ANYNL_EXTRA as uint32_t);
                                        }
                                        22 => {
                                            codevalue =
                                                codevalue.wrapping_add(OP_EXTUNI_EXTRA as uint32_t);
                                        }
                                        18 | 19 => {
                                            codevalue =
                                                codevalue.wrapping_add(OP_HSPACE_EXTRA as uint32_t);
                                        }
                                        20 | 21 => {
                                            codevalue =
                                                codevalue.wrapping_add(OP_VSPACE_EXTRA as uint32_t);
                                        }
                                        _ => {}
                                    }
                                }
                            } else {
                                dlen = 0 as ::core::ffi::c_int;
                                d = NOTACHAR as uint32_t;
                            }
                            let mut current_block_1814: u64;
                            match codevalue {
                                122 | 124 | 123 | 125 => {
                                    if code != end_code {
                                        let fresh5 = active_count;
                                        active_count = active_count + 1;
                                        if fresh5 < wscount {
                                            (*next_active_state).offset = state_offset
                                                + 1 as ::core::ffi::c_int
                                                + 2 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                        if codevalue != OP_KET as ::core::ffi::c_int as uint32_t {
                                            let fresh6 = active_count;
                                            active_count = active_count + 1;
                                            if fresh6 < wscount {
                                                (*next_active_state).offset = state_offset
                                                    - ((*code
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int)
                                                        << 8 as ::core::ffi::c_int
                                                        | *code.offset(
                                                            (1 as ::core::ffi::c_int
                                                                + 1 as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                            as ::core::ffi::c_int)
                                                        as ::core::ffi::c_uint
                                                        as ::core::ffi::c_int;
                                                (*next_active_state).count =
                                                    0 as ::core::ffi::c_int;
                                                next_active_state = next_active_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    } else if ptr > current_subject
                                        || (*mb).moptions & PCRE2_NOTEMPTY as uint32_t
                                            == 0 as uint32_t
                                            && ((*mb).moptions & PCRE2_NOTEMPTY_ATSTART as uint32_t
                                                == 0 as uint32_t
                                                || current_subject
                                                    > start_subject
                                                        .offset((*mb).start_offset as isize))
                                    {
                                        if match_count < 0 as ::core::ffi::c_int {
                                            match_count = if offsetcount >= 2 as uint32_t {
                                                1 as ::core::ffi::c_int
                                            } else {
                                                0 as ::core::ffi::c_int
                                            };
                                        } else if match_count > 0 as ::core::ffi::c_int && {
                                            match_count += 1;
                                            match_count * 2 as ::core::ffi::c_int
                                                > offsetcount as ::core::ffi::c_int
                                        } {
                                            match_count = 0 as ::core::ffi::c_int;
                                        }
                                        count = (if match_count == 0 as ::core::ffi::c_int {
                                            offsetcount as ::core::ffi::c_int
                                        } else {
                                            match_count * 2 as ::core::ffi::c_int
                                        }) - 2 as ::core::ffi::c_int;
                                        if count > 0 as ::core::ffi::c_int {
                                            memmove(
                                                offsets.offset(2 as ::core::ffi::c_int as isize)
                                                    as *mut ::core::ffi::c_void,
                                                offsets as *const ::core::ffi::c_void,
                                                (count as size_t).wrapping_mul(
                                                    ::core::mem::size_of::<size_t>() as size_t,
                                                ),
                                            );
                                        }
                                        if offsetcount >= 2 as uint32_t {
                                            *offsets.offset(0 as ::core::ffi::c_int as isize) =
                                                current_subject.offset_from(start_subject)
                                                    as isize
                                                    as size_t;
                                            *offsets.offset(1 as ::core::ffi::c_int as isize) = ptr
                                                .offset_from(start_subject)
                                                as isize
                                                as size_t;
                                        }
                                        if (*mb).moptions & PCRE2_DFA_SHORTEST as uint32_t
                                            != 0 as uint32_t
                                        {
                                            return match_count;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                121 => {
                                    loop {
                                        code = code.offset(
                                            ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 8 as ::core::ffi::c_int
                                                | *code.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint
                                                as isize,
                                        );
                                        if !(*code as ::core::ffi::c_int
                                            == OP_ALT as ::core::ffi::c_int)
                                        {
                                            break;
                                        }
                                    }
                                    let fresh7 = active_count;
                                    active_count = active_count + 1;
                                    if fresh7 < wscount {
                                        (*next_active_state).offset = code.offset_from(start_code)
                                            as isize
                                            as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                137 | 142 => {
                                    loop {
                                        let fresh8 = active_count;
                                        active_count = active_count + 1;
                                        if fresh8 < wscount {
                                            (*next_active_state).offset = (code
                                                .offset_from(start_code)
                                                as isize
                                                + 1 as isize
                                                + 2 as isize)
                                                as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                        code = code.offset(
                                            ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 8 as ::core::ffi::c_int
                                                | *code.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint
                                                as isize,
                                        );
                                        if !(*code as ::core::ffi::c_int
                                            == OP_ALT as ::core::ffi::c_int)
                                        {
                                            break;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                139 | 144 => {
                                    let fresh9 = active_count;
                                    active_count = active_count + 1;
                                    if fresh9 < wscount {
                                        (*next_active_state).offset = (code.offset_from(start_code)
                                            as isize
                                            + 1 as isize
                                            + 2 as isize
                                            + 2 as isize)
                                            as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    code = code.offset(
                                        ((*code.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int)
                                            << 8 as ::core::ffi::c_int
                                            | *code.offset(
                                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                    as isize,
                                            )
                                                as ::core::ffi::c_int)
                                            as ::core::ffi::c_uint
                                            as isize,
                                    );
                                    while *code as ::core::ffi::c_int
                                        == OP_ALT as ::core::ffi::c_int
                                    {
                                        let fresh10 = active_count;
                                        active_count = active_count + 1;
                                        if fresh10 < wscount {
                                            (*next_active_state).offset = (code
                                                .offset_from(start_code)
                                                as isize
                                                + 1 as isize
                                                + 2 as isize)
                                                as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                        code = code.offset(
                                            ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 8 as ::core::ffi::c_int
                                                | *code.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint
                                                as isize,
                                        );
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                153 | 154 => {
                                    let fresh11 = active_count;
                                    active_count = active_count + 1;
                                    if fresh11 < wscount {
                                        (*next_active_state).offset =
                                            state_offset + 1 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    code = code.offset((1 as ::core::ffi::c_uint).wrapping_add(
                                        ((*code.offset(2 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int)
                                            << 8 as ::core::ffi::c_int
                                            | *code.offset(
                                                (2 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                    as isize,
                                            )
                                                as ::core::ffi::c_int)
                                            as ::core::ffi::c_uint,
                                    )
                                        as isize);
                                    while *code as ::core::ffi::c_int
                                        == OP_ALT as ::core::ffi::c_int
                                    {
                                        code = code.offset(
                                            ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 8 as ::core::ffi::c_int
                                                | *code.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint
                                                as isize,
                                        );
                                    }
                                    let fresh12 = active_count;
                                    active_count = active_count + 1;
                                    if fresh12 < wscount {
                                        (*next_active_state).offset = (code.offset_from(start_code)
                                            as isize
                                            + 1 as isize
                                            + 2 as isize)
                                            as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                169 => {
                                    code = code.offset((1 as ::core::ffi::c_uint).wrapping_add(
                                        ((*code.offset(2 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int)
                                            << 8 as ::core::ffi::c_int
                                            | *code.offset(
                                                (2 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                    as isize,
                                            )
                                                as ::core::ffi::c_int)
                                            as ::core::ffi::c_uint,
                                    )
                                        as isize);
                                    while *code as ::core::ffi::c_int
                                        == OP_ALT as ::core::ffi::c_int
                                    {
                                        code = code.offset(
                                            ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 8 as ::core::ffi::c_int
                                                | *code.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint
                                                as isize,
                                        );
                                    }
                                    let fresh13 = active_count;
                                    active_count = active_count + 1;
                                    if fresh13 < wscount {
                                        (*next_active_state).offset = (code.offset_from(start_code)
                                            as isize
                                            + 1 as isize
                                            + 2 as isize)
                                            as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                27 => {
                                    if ptr == start_subject
                                        && (*mb).moptions & PCRE2_NOTBOL as uint32_t
                                            == 0 as uint32_t
                                    {
                                        let fresh14 = active_count;
                                        active_count = active_count + 1;
                                        if fresh14 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 1 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                28 => {
                                    if ptr == start_subject
                                        && (*mb).moptions & PCRE2_NOTBOL as uint32_t
                                            == 0 as uint32_t
                                        || (ptr != end_subject
                                            || (*mb).poptions & PCRE2_ALT_CIRCUMFLEX as uint32_t
                                                != 0 as uint32_t)
                                            && (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                                (ptr > (*mb).start_subject
                                                    && _pcre2_was_newline_8(
                                                        ptr,
                                                        (*mb).nltype,
                                                        (*mb).start_subject,
                                                        &raw mut (*mb).nllen,
                                                        utf,
                                                    ) != 0)
                                                    as ::core::ffi::c_int
                                            } else {
                                                (ptr >= (*mb)
                                                    .start_subject
                                                    .offset((*mb).nllen as isize)
                                                    && *ptr.offset(-((*mb).nllen as isize))
                                                        as ::core::ffi::c_int
                                                        == (*mb).nl
                                                            [0 as ::core::ffi::c_int as usize]
                                                            as ::core::ffi::c_int
                                                    && ((*mb).nllen == 1 as uint32_t
                                                        || *ptr
                                                            .offset(-((*mb).nllen as isize))
                                                            .offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            == (*mb).nl
                                                                [1 as ::core::ffi::c_int as usize]
                                                                as ::core::ffi::c_int))
                                                    as ::core::ffi::c_int
                                            }) != 0
                                    {
                                        let fresh15 = active_count;
                                        active_count = active_count + 1;
                                        if fresh15 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 1 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                24 => {
                                    if ptr >= end_subject {
                                        if (*mb).moptions & PCRE2_PARTIAL_HARD as uint32_t
                                            != 0 as uint32_t
                                        {
                                            return PCRE2_ERROR_PARTIAL;
                                        } else {
                                            let fresh16 = active_count;
                                            active_count = active_count + 1;
                                            if fresh16 < wscount {
                                                (*next_active_state).offset =
                                                    state_offset + 1 as ::core::ffi::c_int;
                                                (*next_active_state).count =
                                                    0 as ::core::ffi::c_int;
                                                next_active_state = next_active_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                1 => {
                                    if ptr == start_subject {
                                        let fresh17 = active_count;
                                        active_count = active_count + 1;
                                        if fresh17 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 1 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                2 => {
                                    if ptr == start_subject.offset(start_offset as isize) {
                                        let fresh18 = active_count;
                                        active_count = active_count + 1;
                                        if fresh18 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 1 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                12 => {
                                    if clen > 0 as ::core::ffi::c_int
                                        && (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                            (ptr < (*mb).end_subject
                                                && _pcre2_is_newline_8(
                                                    ptr,
                                                    (*mb).nltype,
                                                    (*mb).end_subject,
                                                    &raw mut (*mb).nllen,
                                                    utf,
                                                ) != 0)
                                                as ::core::ffi::c_int
                                        } else {
                                            (ptr <= (*mb)
                                                .end_subject
                                                .offset(-((*mb).nllen as isize))
                                                && *ptr as ::core::ffi::c_int
                                                    == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                        as ::core::ffi::c_int
                                                && ((*mb).nllen == 1 as uint32_t
                                                    || *ptr.offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                        == (*mb).nl
                                                            [1 as ::core::ffi::c_int as usize]
                                                            as ::core::ffi::c_int))
                                                as ::core::ffi::c_int
                                        }) == 0
                                    {
                                        if ptr.offset(1 as ::core::ffi::c_int as isize)
                                            >= (*mb).end_subject
                                            && (*mb).moptions & 0x20 as uint32_t != 0 as uint32_t
                                            && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                            && (*mb).nllen == 2 as uint32_t
                                            && c == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                as uint32_t
                                        {
                                            partial_newline = TRUE as BOOL;
                                            could_continue = partial_newline;
                                        } else {
                                            let fresh19 = new_count;
                                            new_count = new_count + 1;
                                            if fresh19 < wscount {
                                                (*next_new_state).offset =
                                                    state_offset + 1 as ::core::ffi::c_int;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                13 => {
                                    if clen > 0 as ::core::ffi::c_int {
                                        let fresh20 = new_count;
                                        new_count = new_count + 1;
                                        if fresh20 < wscount {
                                            (*next_new_state).offset =
                                                state_offset + 1 as ::core::ffi::c_int;
                                            (*next_new_state).count = 0 as ::core::ffi::c_int;
                                            next_new_state = next_new_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                23 => {
                                    if clen == 0 as ::core::ffi::c_int
                                        || (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                            (ptr < (*mb).end_subject
                                                && _pcre2_is_newline_8(
                                                    ptr,
                                                    (*mb).nltype,
                                                    (*mb).end_subject,
                                                    &raw mut (*mb).nllen,
                                                    utf,
                                                ) != 0)
                                                as ::core::ffi::c_int
                                        } else {
                                            (ptr <= (*mb)
                                                .end_subject
                                                .offset(-((*mb).nllen as isize))
                                                && *ptr as ::core::ffi::c_int
                                                    == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                        as ::core::ffi::c_int
                                                && ((*mb).nllen == 1 as uint32_t
                                                    || *ptr.offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                        == (*mb).nl
                                                            [1 as ::core::ffi::c_int as usize]
                                                            as ::core::ffi::c_int))
                                                as ::core::ffi::c_int
                                        }) != 0
                                            && ptr == end_subject.offset(-((*mb).nllen as isize))
                                    {
                                        if (*mb).moptions & PCRE2_PARTIAL_HARD as uint32_t
                                            != 0 as uint32_t
                                        {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                        let fresh21 = active_count;
                                        active_count = active_count + 1;
                                        if fresh21 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 1 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                25 => {
                                    if (*mb).moptions & PCRE2_NOTEOL as uint32_t == 0 as uint32_t {
                                        if clen == 0 as ::core::ffi::c_int
                                            && (*mb).moptions & PCRE2_PARTIAL_HARD as uint32_t
                                                != 0 as uint32_t
                                        {
                                            could_continue = TRUE as BOOL;
                                        } else if clen == 0 as ::core::ffi::c_int
                                            || (*mb).poptions & PCRE2_DOLLAR_ENDONLY as uint32_t
                                                == 0 as uint32_t
                                                && (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                                    (ptr < (*mb).end_subject
                                                        && _pcre2_is_newline_8(
                                                            ptr,
                                                            (*mb).nltype,
                                                            (*mb).end_subject,
                                                            &raw mut (*mb).nllen,
                                                            utf,
                                                        ) != 0)
                                                        as ::core::ffi::c_int
                                                } else {
                                                    (ptr <= (*mb)
                                                        .end_subject
                                                        .offset(-((*mb).nllen as isize))
                                                        && *ptr as ::core::ffi::c_int
                                                            == (*mb).nl
                                                                [0 as ::core::ffi::c_int as usize]
                                                                as ::core::ffi::c_int
                                                        && ((*mb).nllen == 1 as uint32_t
                                                            || *ptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                                == (*mb).nl[1 as ::core::ffi::c_int
                                                                    as usize]
                                                                    as ::core::ffi::c_int))
                                                        as ::core::ffi::c_int
                                                }) != 0
                                                && ptr
                                                    == end_subject.offset(-((*mb).nllen as isize))
                                        {
                                            let fresh22 = active_count;
                                            active_count = active_count + 1;
                                            if fresh22 < wscount {
                                                (*next_active_state).offset =
                                                    state_offset + 1 as ::core::ffi::c_int;
                                                (*next_active_state).count =
                                                    0 as ::core::ffi::c_int;
                                                next_active_state = next_active_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        } else if ptr.offset(1 as ::core::ffi::c_int as isize)
                                            >= (*mb).end_subject
                                            && (*mb).moptions
                                                & (PCRE2_PARTIAL_HARD as uint32_t
                                                    | PCRE2_PARTIAL_SOFT as uint32_t)
                                                != 0 as uint32_t
                                            && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                            && (*mb).nllen == 2 as uint32_t
                                            && c == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                as uint32_t
                                        {
                                            if (*mb).moptions & PCRE2_PARTIAL_HARD as uint32_t
                                                != 0 as uint32_t
                                            {
                                                reset_could_continue = TRUE as BOOL;
                                                let fresh23 = new_count;
                                                new_count = new_count + 1;
                                                if fresh23 < wscount {
                                                    (*next_new_state).offset =
                                                        -(state_offset + 1 as ::core::ffi::c_int);
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    (*next_new_state).data =
                                                        1 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else {
                                                partial_newline = TRUE as BOOL;
                                                could_continue = partial_newline;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                26 => {
                                    if (*mb).moptions & PCRE2_NOTEOL as uint32_t == 0 as uint32_t {
                                        if clen == 0 as ::core::ffi::c_int
                                            && (*mb).moptions & PCRE2_PARTIAL_HARD as uint32_t
                                                != 0 as uint32_t
                                        {
                                            could_continue = TRUE as BOOL;
                                        } else if clen == 0 as ::core::ffi::c_int
                                            || (*mb).poptions & PCRE2_DOLLAR_ENDONLY as uint32_t
                                                == 0 as uint32_t
                                                && (if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                                    (ptr < (*mb).end_subject
                                                        && _pcre2_is_newline_8(
                                                            ptr,
                                                            (*mb).nltype,
                                                            (*mb).end_subject,
                                                            &raw mut (*mb).nllen,
                                                            utf,
                                                        ) != 0)
                                                        as ::core::ffi::c_int
                                                } else {
                                                    (ptr <= (*mb)
                                                        .end_subject
                                                        .offset(-((*mb).nllen as isize))
                                                        && *ptr as ::core::ffi::c_int
                                                            == (*mb).nl
                                                                [0 as ::core::ffi::c_int as usize]
                                                                as ::core::ffi::c_int
                                                        && ((*mb).nllen == 1 as uint32_t
                                                            || *ptr.offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                                == (*mb).nl[1 as ::core::ffi::c_int
                                                                    as usize]
                                                                    as ::core::ffi::c_int))
                                                        as ::core::ffi::c_int
                                                }) != 0
                                        {
                                            let fresh24 = active_count;
                                            active_count = active_count + 1;
                                            if fresh24 < wscount {
                                                (*next_active_state).offset =
                                                    state_offset + 1 as ::core::ffi::c_int;
                                                (*next_active_state).count =
                                                    0 as ::core::ffi::c_int;
                                                next_active_state = next_active_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        } else if ptr.offset(1 as ::core::ffi::c_int as isize)
                                            >= (*mb).end_subject
                                            && (*mb).moptions
                                                & (PCRE2_PARTIAL_HARD as uint32_t
                                                    | PCRE2_PARTIAL_SOFT as uint32_t)
                                                != 0 as uint32_t
                                            && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                            && (*mb).nllen == 2 as uint32_t
                                            && c == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                as uint32_t
                                        {
                                            if (*mb).moptions & PCRE2_PARTIAL_HARD as uint32_t
                                                != 0 as uint32_t
                                            {
                                                reset_could_continue = TRUE as BOOL;
                                                let fresh25 = new_count;
                                                new_count = new_count + 1;
                                                if fresh25 < wscount {
                                                    (*next_new_state).offset =
                                                        -(state_offset + 1 as ::core::ffi::c_int);
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    (*next_new_state).data =
                                                        1 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else {
                                                partial_newline = TRUE as BOOL;
                                                could_continue = partial_newline;
                                            }
                                        }
                                    } else if if (*mb).nltype != NLTYPE_FIXED as uint32_t {
                                        (ptr < (*mb).end_subject
                                            && _pcre2_is_newline_8(
                                                ptr,
                                                (*mb).nltype,
                                                (*mb).end_subject,
                                                &raw mut (*mb).nllen,
                                                utf,
                                            ) != 0)
                                            as ::core::ffi::c_int
                                    } else {
                                        (ptr <= (*mb).end_subject.offset(-((*mb).nllen as isize))
                                            && *ptr as ::core::ffi::c_int
                                                == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                    as ::core::ffi::c_int
                                            && ((*mb).nllen == 1 as uint32_t
                                                || *ptr.offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int
                                                    == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                                        as ::core::ffi::c_int))
                                            as ::core::ffi::c_int
                                    } != 0
                                    {
                                        let fresh26 = active_count;
                                        active_count = active_count + 1;
                                        if fresh26 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 1 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                7 | 9 | 11 => {
                                    if clen > 0 as ::core::ffi::c_int
                                        && c < 256 as uint32_t
                                        && *ctypes.offset(c as isize) as ::core::ffi::c_int
                                            & toptable1[codevalue as usize] as ::core::ffi::c_int
                                            ^ toptable2[codevalue as usize] as ::core::ffi::c_int
                                            != 0 as ::core::ffi::c_int
                                    {
                                        let fresh27 = new_count;
                                        new_count = new_count + 1;
                                        if fresh27 < wscount {
                                            (*next_new_state).offset =
                                                state_offset + 1 as ::core::ffi::c_int;
                                            (*next_new_state).count = 0 as ::core::ffi::c_int;
                                            next_new_state = next_new_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                6 | 8 | 10 => {
                                    if clen > 0 as ::core::ffi::c_int
                                        && (c >= 256 as uint32_t
                                            || *ctypes.offset(c as isize) as ::core::ffi::c_int
                                                & toptable1[codevalue as usize]
                                                    as ::core::ffi::c_int
                                                ^ toptable2[codevalue as usize]
                                                    as ::core::ffi::c_int
                                                != 0 as ::core::ffi::c_int)
                                    {
                                        let fresh28 = new_count;
                                        new_count = new_count + 1;
                                        if fresh28 < wscount {
                                            (*next_new_state).offset =
                                                state_offset + 1 as ::core::ffi::c_int;
                                            (*next_new_state).count = 0 as ::core::ffi::c_int;
                                            next_new_state = next_new_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                5 | 4 | 171 | 172 => {
                                    let mut left_word: ::core::ffi::c_int = 0;
                                    let mut right_word: ::core::ffi::c_int = 0;
                                    if ptr > start_subject {
                                        let mut temp: PCRE2_SPTR8 =
                                            ptr.offset(-(1 as ::core::ffi::c_int as isize));
                                        if temp < (*mb).start_used_ptr {
                                            (*mb).start_used_ptr = temp;
                                        }
                                        if utf != 0 {
                                            while *temp as ::core::ffi::c_uint
                                                & 0xc0 as ::core::ffi::c_uint
                                                == 0x80 as ::core::ffi::c_uint
                                            {
                                                temp = temp.offset(-1);
                                            }
                                        }
                                        d = *temp as uint32_t;
                                        if utf != 0 && d >= 0xc0 as uint32_t {
                                            if d & 0x20 as uint32_t == 0 as uint32_t {
                                                d = (d & 0x1f as uint32_t)
                                                    << 6 as ::core::ffi::c_int
                                                    | *temp.offset(1 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t;
                                            } else if d & 0x10 as uint32_t == 0 as uint32_t {
                                                d = (d & 0xf as uint32_t)
                                                    << 12 as ::core::ffi::c_int
                                                    | (*temp
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t)
                                                        << 6 as ::core::ffi::c_int
                                                    | *temp.offset(2 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t;
                                            } else if d & 0x8 as uint32_t == 0 as uint32_t {
                                                d = (d & 0x7 as uint32_t)
                                                    << 18 as ::core::ffi::c_int
                                                    | (*temp
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t)
                                                        << 12 as ::core::ffi::c_int
                                                    | (*temp
                                                        .offset(2 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t)
                                                        << 6 as ::core::ffi::c_int
                                                    | *temp.offset(3 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t;
                                            } else if d & 0x4 as uint32_t == 0 as uint32_t {
                                                d = (d & 0x3 as uint32_t)
                                                    << 24 as ::core::ffi::c_int
                                                    | (*temp
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t)
                                                        << 18 as ::core::ffi::c_int
                                                    | (*temp
                                                        .offset(2 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t)
                                                        << 12 as ::core::ffi::c_int
                                                    | (*temp
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t)
                                                        << 6 as ::core::ffi::c_int
                                                    | *temp.offset(4 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t;
                                            } else {
                                                d = (d & 0x1 as uint32_t)
                                                    << 30 as ::core::ffi::c_int
                                                    | (*temp
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t)
                                                        << 24 as ::core::ffi::c_int
                                                    | (*temp
                                                        .offset(2 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t)
                                                        << 18 as ::core::ffi::c_int
                                                    | (*temp
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t)
                                                        << 12 as ::core::ffi::c_int
                                                    | (*temp
                                                        .offset(4 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t)
                                                        << 6 as ::core::ffi::c_int
                                                    | *temp.offset(5 as ::core::ffi::c_int as isize)
                                                        as uint32_t
                                                        & 0x3f as uint32_t;
                                            }
                                        }
                                        if codevalue
                                            == OP_UCP_WORD_BOUNDARY as ::core::ffi::c_int
                                                as uint32_t
                                            || codevalue
                                                == OP_NOT_UCP_WORD_BOUNDARY as ::core::ffi::c_int
                                                    as uint32_t
                                        {
                                            let mut chartype: ::core::ffi::c_int =
                                                (*(&raw const _pcre2_ucd_records_8
                                                    as *const ucd_record)
                                                    .offset(
                                                        *(&raw const _pcre2_ucd_stage2_8
                                                            as *const uint16_t)
                                                            .offset(
                                                                (*(&raw const _pcre2_ucd_stage1_8
                                                                    as *const uint16_t)
                                                                    .offset(
                                                                        (d as ::core::ffi::c_int
                                                                            / UCD_BLOCK_SIZE)
                                                                            as isize,
                                                                    )
                                                                    as ::core::ffi::c_int
                                                                    * UCD_BLOCK_SIZE
                                                                    + d as ::core::ffi::c_int
                                                                        % UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    ))
                                                .chartype
                                                    as ::core::ffi::c_int;
                                            let mut category: ::core::ffi::c_int =
                                                *(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset(chartype as isize)
                                                    as ::core::ffi::c_int;
                                            left_word = (category == ucp_L as ::core::ffi::c_int
                                                || category == ucp_N as ::core::ffi::c_int
                                                || chartype == ucp_Mn as ::core::ffi::c_int
                                                || chartype == ucp_Pc as ::core::ffi::c_int)
                                                as ::core::ffi::c_int;
                                        } else {
                                            left_word = (d < 256 as uint32_t
                                                && *ctypes.offset(d as isize) as ::core::ffi::c_int
                                                    & ctype_word
                                                    != 0 as ::core::ffi::c_int)
                                                as ::core::ffi::c_int;
                                        }
                                    } else {
                                        left_word = FALSE;
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        if ptr >= (*mb).last_used_ptr {
                                            let mut temp_0: PCRE2_SPTR8 =
                                                ptr.offset(1 as ::core::ffi::c_int as isize);
                                            if utf != 0 {
                                                while temp_0 < (*mb).end_subject
                                                    && *temp_0 as ::core::ffi::c_uint
                                                        & 0xc0 as ::core::ffi::c_uint
                                                        == 0x80 as ::core::ffi::c_uint
                                                {
                                                    temp_0 = temp_0.offset(1);
                                                }
                                            }
                                            (*mb).last_used_ptr = temp_0;
                                        }
                                        if codevalue
                                            == OP_UCP_WORD_BOUNDARY as ::core::ffi::c_int
                                                as uint32_t
                                            || codevalue
                                                == OP_NOT_UCP_WORD_BOUNDARY as ::core::ffi::c_int
                                                    as uint32_t
                                        {
                                            let mut chartype_0: ::core::ffi::c_int =
                                                (*(&raw const _pcre2_ucd_records_8
                                                    as *const ucd_record)
                                                    .offset(
                                                        *(&raw const _pcre2_ucd_stage2_8
                                                            as *const uint16_t)
                                                            .offset(
                                                                (*(&raw const _pcre2_ucd_stage1_8
                                                                    as *const uint16_t)
                                                                    .offset(
                                                                        (c as ::core::ffi::c_int
                                                                            / UCD_BLOCK_SIZE)
                                                                            as isize,
                                                                    )
                                                                    as ::core::ffi::c_int
                                                                    * UCD_BLOCK_SIZE
                                                                    + c as ::core::ffi::c_int
                                                                        % UCD_BLOCK_SIZE)
                                                                    as isize,
                                                            )
                                                            as ::core::ffi::c_int
                                                            as isize,
                                                    ))
                                                .chartype
                                                    as ::core::ffi::c_int;
                                            let mut category_0: ::core::ffi::c_int =
                                                *(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset(chartype_0 as isize)
                                                    as ::core::ffi::c_int;
                                            right_word = (category_0 == ucp_L as ::core::ffi::c_int
                                                || category_0 == ucp_N as ::core::ffi::c_int
                                                || chartype_0 == ucp_Mn as ::core::ffi::c_int
                                                || chartype_0 == ucp_Pc as ::core::ffi::c_int)
                                                as ::core::ffi::c_int;
                                        } else {
                                            right_word = (c < 256 as uint32_t
                                                && *ctypes.offset(c as isize) as ::core::ffi::c_int
                                                    & ctype_word
                                                    != 0 as ::core::ffi::c_int)
                                                as ::core::ffi::c_int;
                                        }
                                    } else {
                                        right_word = FALSE;
                                    }
                                    if (left_word == right_word) as ::core::ffi::c_int
                                        == (codevalue
                                            == OP_NOT_WORD_BOUNDARY as ::core::ffi::c_int
                                                as uint32_t
                                            || codevalue
                                                == OP_NOT_UCP_WORD_BOUNDARY as ::core::ffi::c_int
                                                    as uint32_t)
                                            as ::core::ffi::c_int
                                    {
                                        let fresh29 = active_count;
                                        active_count = active_count + 1;
                                        if fresh29 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 1 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                16 | 15 => {
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut OK: BOOL = 0;
                                        let mut chartype_1: ::core::ffi::c_int = 0;
                                        let mut cp: *const uint32_t =
                                            ::core::ptr::null::<uint32_t>();
                                        let mut prop: *const ucd_record =
                                            (&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                .offset(
                                                    *(&raw const _pcre2_ucd_stage2_8
                                                        as *const uint16_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_stage1_8
                                                                as *const uint16_t)
                                                                .offset(
                                                                    (c as ::core::ffi::c_int
                                                                        / UCD_BLOCK_SIZE)
                                                                        as isize,
                                                                )
                                                                as ::core::ffi::c_int
                                                                * UCD_BLOCK_SIZE
                                                                + c as ::core::ffi::c_int
                                                                    % UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        as isize,
                                                );
                                        match *code.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                        {
                                            PT_LAMP => {
                                                chartype_1 = (*prop).chartype as ::core::ffi::c_int;
                                                OK = (chartype_1 == ucp_Lu as ::core::ffi::c_int
                                                    || chartype_1 == ucp_Ll as ::core::ffi::c_int
                                                    || chartype_1 == ucp_Lt as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_GC => {
                                                OK = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset((*prop).chartype as isize)
                                                    == *code
                                                        .offset(2 as ::core::ffi::c_int as isize)
                                                        as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_PC => {
                                                OK = ((*prop).chartype as ::core::ffi::c_int
                                                    == *code
                                                        .offset(2 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SC => {
                                                OK = ((*prop).script as ::core::ffi::c_int
                                                    == *code
                                                        .offset(2 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SCX => {
                                                OK = ((*prop).script as ::core::ffi::c_int
                                                    == *code
                                                        .offset(2 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                    || *(&raw const _pcre2_ucd_script_sets_8
                                                        as *const uint32_t)
                                                        .offset(
                                                            ((*prop).scriptx_bidiclass
                                                                as ::core::ffi::c_int
                                                                & 0x3ff as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                        .offset(
                                                            (*code.offset(
                                                                2 as ::core::ffi::c_int as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                                / 32 as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                        & (1 as uint32_t)
                                                            << *code.offset(
                                                                2 as ::core::ffi::c_int as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                                % 32 as ::core::ffi::c_int
                                                        != 0 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_ALNUM => {
                                                chartype_1 = (*prop).chartype as ::core::ffi::c_int;
                                                OK = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset(chartype_1 as isize)
                                                    == ucp_L as ::core::ffi::c_int as uint32_t
                                                    || *(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset(chartype_1 as isize)
                                                        == ucp_N as ::core::ffi::c_int as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SPACE | PT_PXSPACE => match c {
                                                9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                                | 8195 | 8196 | 8197 | 8198 | 8199 | 8200
                                                | 8201 | 8202 | 8239 | 8287 | 12288 | 10 | 11
                                                | 12 | 13 | 133 | 8232 | 8233 => {
                                                    OK = TRUE as BOOL;
                                                }
                                                _ => {
                                                    OK = (*(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset((*prop).chartype as isize)
                                                        == ucp_Z as ::core::ffi::c_int as uint32_t)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                }
                                            },
                                            PT_WORD => {
                                                chartype_1 = (*prop).chartype as ::core::ffi::c_int;
                                                OK = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset(chartype_1 as isize)
                                                    == ucp_L as ::core::ffi::c_int as uint32_t
                                                    || *(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset(chartype_1 as isize)
                                                        == ucp_N as ::core::ffi::c_int as uint32_t
                                                    || chartype_1 == ucp_Mn as ::core::ffi::c_int
                                                    || chartype_1 == ucp_Pc as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_CLIST => {
                                                cp =
                                                    (&raw const _pcre2_ucd_caseless_sets_8
                                                        as *const uint32_t)
                                                        .offset(*code.offset(
                                                            2 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            as isize);
                                                loop {
                                                    if c < *cp {
                                                        OK = FALSE as BOOL;
                                                        break;
                                                    } else {
                                                        let fresh30 = cp;
                                                        cp = cp.offset(1);
                                                        if !(c == *fresh30) {
                                                            continue;
                                                        }
                                                        OK = TRUE as BOOL;
                                                        break;
                                                    }
                                                }
                                            }
                                            PT_UCNC => {
                                                OK = (c == CHAR_DOLLAR_SIGN as uint32_t
                                                    || c == CHAR_COMMERCIAL_AT as uint32_t
                                                    || c == CHAR_GRAVE_ACCENT as uint32_t
                                                    || c >= 0xa0 as uint32_t
                                                        && c <= 0xd7ff as uint32_t
                                                    || c >= 0xe000 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_BIDICL => {
                                                OK = ((*(&raw const _pcre2_ucd_records_8
                                                    as *const ucd_record)
                                                    .offset(
                                                        *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                            .offset(
                                                                (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                    .offset(
                                                                        (c as ::core::ffi::c_int / 128 as ::core::ffi::c_int)
                                                                            as isize,
                                                                    ) as ::core::ffi::c_int * 128 as ::core::ffi::c_int
                                                                    + c as ::core::ffi::c_int % 128 as ::core::ffi::c_int)
                                                                    as isize,
                                                            ) as ::core::ffi::c_int as isize,
                                                    ))
                                                    .scriptx_bidiclass as ::core::ffi::c_int
                                                    >> UCD_BIDICLASS_SHIFT
                                                    == *code.offset(2 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int) as ::core::ffi::c_int as BOOL;
                                            }
                                            PT_BOOL => {
                                                OK = (*(&raw const _pcre2_ucd_boolprop_sets_8
                                                    as *const uint32_t)
                                                    .offset(
                                                        ((*prop).bprops as ::core::ffi::c_int
                                                            & 0xfff as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                    .offset(
                                                        (*code.offset(
                                                            2 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            / 32 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                    & (1 as uint32_t)
                                                        << *code.offset(
                                                            2 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            % 32 as ::core::ffi::c_int
                                                    != 0 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            _ => {
                                                OK = (codevalue
                                                    != OP_PROP as ::core::ffi::c_int as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                        }
                                        if OK
                                            == (codevalue
                                                == OP_PROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            let fresh31 = new_count;
                                            new_count = new_count + 1;
                                            if fresh31 < wscount {
                                                (*next_new_state).offset =
                                                    state_offset + 3 as ::core::ffi::c_int;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                87 | 88 | 95 => {
                                    count = (*current_state).count;
                                    if count > 0 as ::core::ffi::c_int {
                                        let fresh32 = active_count;
                                        active_count = active_count + 1;
                                        if fresh32 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 2 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        if d == OP_ANY as ::core::ffi::c_int as uint32_t
                                            && ptr.offset(1 as ::core::ffi::c_int as isize)
                                                >= (*mb).end_subject
                                            && (*mb).moptions & 0x20 as uint32_t != 0 as uint32_t
                                            && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                            && (*mb).nllen == 2 as uint32_t
                                            && c == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                as uint32_t
                                        {
                                            partial_newline = TRUE as BOOL;
                                            could_continue = partial_newline;
                                        } else if c >= 256 as uint32_t
                                            && d != OP_DIGIT as ::core::ffi::c_int as uint32_t
                                            && d != OP_WHITESPACE as ::core::ffi::c_int as uint32_t
                                            && d != OP_WORDCHAR as ::core::ffi::c_int as uint32_t
                                            || c < 256 as uint32_t
                                                && (d != OP_ANY as ::core::ffi::c_int as uint32_t
                                                    || (if (*mb).nltype != NLTYPE_FIXED as uint32_t
                                                    {
                                                        (ptr < (*mb).end_subject
                                                            && _pcre2_is_newline_8(
                                                                ptr,
                                                                (*mb).nltype,
                                                                (*mb).end_subject,
                                                                &raw mut (*mb).nllen,
                                                                utf,
                                                            ) != 0)
                                                            as ::core::ffi::c_int
                                                    } else {
                                                        (ptr <= (*mb)
                                                            .end_subject
                                                            .offset(-((*mb).nllen as isize))
                                                            && *ptr as ::core::ffi::c_int
                                                                == (*mb).nl[0 as ::core::ffi::c_int
                                                                    as usize]
                                                                    as ::core::ffi::c_int
                                                            && ((*mb).nllen == 1 as uint32_t
                                                                || *ptr.offset(
                                                                    1 as ::core::ffi::c_int
                                                                        as isize,
                                                                )
                                                                    as ::core::ffi::c_int
                                                                    == (*mb).nl[1
                                                                        as ::core::ffi::c_int
                                                                        as usize]
                                                                        as ::core::ffi::c_int))
                                                            as ::core::ffi::c_int
                                                    }) == 0)
                                                && *ctypes.offset(c as isize) as ::core::ffi::c_int
                                                    & toptable1[d as usize] as ::core::ffi::c_int
                                                    ^ toptable2[d as usize] as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                        {
                                            if count > 0 as ::core::ffi::c_int
                                                && codevalue
                                                    == OP_TYPEPOSPLUS as ::core::ffi::c_int
                                                        as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            count += 1;
                                            let fresh33 = new_count;
                                            new_count = new_count + 1;
                                            if fresh33 < wscount {
                                                (*next_new_state).offset = state_offset;
                                                (*next_new_state).count = count;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                89 | 90 | 96 => {
                                    let fresh34 = active_count;
                                    active_count = active_count + 1;
                                    if fresh34 < wscount {
                                        (*next_active_state).offset =
                                            state_offset + 2 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        if d == OP_ANY as ::core::ffi::c_int as uint32_t
                                            && ptr.offset(1 as ::core::ffi::c_int as isize)
                                                >= (*mb).end_subject
                                            && (*mb).moptions & 0x20 as uint32_t != 0 as uint32_t
                                            && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                            && (*mb).nllen == 2 as uint32_t
                                            && c == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                as uint32_t
                                        {
                                            partial_newline = TRUE as BOOL;
                                            could_continue = partial_newline;
                                        } else if c >= 256 as uint32_t
                                            && d != OP_DIGIT as ::core::ffi::c_int as uint32_t
                                            && d != OP_WHITESPACE as ::core::ffi::c_int as uint32_t
                                            && d != OP_WORDCHAR as ::core::ffi::c_int as uint32_t
                                            || c < 256 as uint32_t
                                                && (d != OP_ANY as ::core::ffi::c_int as uint32_t
                                                    || (if (*mb).nltype != NLTYPE_FIXED as uint32_t
                                                    {
                                                        (ptr < (*mb).end_subject
                                                            && _pcre2_is_newline_8(
                                                                ptr,
                                                                (*mb).nltype,
                                                                (*mb).end_subject,
                                                                &raw mut (*mb).nllen,
                                                                utf,
                                                            ) != 0)
                                                            as ::core::ffi::c_int
                                                    } else {
                                                        (ptr <= (*mb)
                                                            .end_subject
                                                            .offset(-((*mb).nllen as isize))
                                                            && *ptr as ::core::ffi::c_int
                                                                == (*mb).nl[0 as ::core::ffi::c_int
                                                                    as usize]
                                                                    as ::core::ffi::c_int
                                                            && ((*mb).nllen == 1 as uint32_t
                                                                || *ptr.offset(
                                                                    1 as ::core::ffi::c_int
                                                                        as isize,
                                                                )
                                                                    as ::core::ffi::c_int
                                                                    == (*mb).nl[1
                                                                        as ::core::ffi::c_int
                                                                        as usize]
                                                                        as ::core::ffi::c_int))
                                                            as ::core::ffi::c_int
                                                    }) == 0)
                                                && *ctypes.offset(c as isize) as ::core::ffi::c_int
                                                    & toptable1[d as usize] as ::core::ffi::c_int
                                                    ^ toptable2[d as usize] as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == OP_TYPEPOSQUERY as ::core::ffi::c_int as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            let fresh35 = new_count;
                                            new_count = new_count + 1;
                                            if fresh35 < wscount {
                                                (*next_new_state).offset =
                                                    state_offset + 2 as ::core::ffi::c_int;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                85 | 86 | 94 => {
                                    let fresh36 = active_count;
                                    active_count = active_count + 1;
                                    if fresh36 < wscount {
                                        (*next_active_state).offset =
                                            state_offset + 2 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        if d == OP_ANY as ::core::ffi::c_int as uint32_t
                                            && ptr.offset(1 as ::core::ffi::c_int as isize)
                                                >= (*mb).end_subject
                                            && (*mb).moptions & 0x20 as uint32_t != 0 as uint32_t
                                            && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                            && (*mb).nllen == 2 as uint32_t
                                            && c == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                as uint32_t
                                        {
                                            partial_newline = TRUE as BOOL;
                                            could_continue = partial_newline;
                                        } else if c >= 256 as uint32_t
                                            && d != OP_DIGIT as ::core::ffi::c_int as uint32_t
                                            && d != OP_WHITESPACE as ::core::ffi::c_int as uint32_t
                                            && d != OP_WORDCHAR as ::core::ffi::c_int as uint32_t
                                            || c < 256 as uint32_t
                                                && (d != OP_ANY as ::core::ffi::c_int as uint32_t
                                                    || (if (*mb).nltype != NLTYPE_FIXED as uint32_t
                                                    {
                                                        (ptr < (*mb).end_subject
                                                            && _pcre2_is_newline_8(
                                                                ptr,
                                                                (*mb).nltype,
                                                                (*mb).end_subject,
                                                                &raw mut (*mb).nllen,
                                                                utf,
                                                            ) != 0)
                                                            as ::core::ffi::c_int
                                                    } else {
                                                        (ptr <= (*mb)
                                                            .end_subject
                                                            .offset(-((*mb).nllen as isize))
                                                            && *ptr as ::core::ffi::c_int
                                                                == (*mb).nl[0 as ::core::ffi::c_int
                                                                    as usize]
                                                                    as ::core::ffi::c_int
                                                            && ((*mb).nllen == 1 as uint32_t
                                                                || *ptr.offset(
                                                                    1 as ::core::ffi::c_int
                                                                        as isize,
                                                                )
                                                                    as ::core::ffi::c_int
                                                                    == (*mb).nl[1
                                                                        as ::core::ffi::c_int
                                                                        as usize]
                                                                        as ::core::ffi::c_int))
                                                            as ::core::ffi::c_int
                                                    }) == 0)
                                                && *ctypes.offset(c as isize) as ::core::ffi::c_int
                                                    & toptable1[d as usize] as ::core::ffi::c_int
                                                    ^ toptable2[d as usize] as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == OP_TYPEPOSSTAR as ::core::ffi::c_int as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            let fresh37 = new_count;
                                            new_count = new_count + 1;
                                            if fresh37 < wscount {
                                                (*next_new_state).offset = state_offset;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                93 => {
                                    count = (*current_state).count;
                                    if clen > 0 as ::core::ffi::c_int {
                                        if d == OP_ANY as ::core::ffi::c_int as uint32_t
                                            && ptr.offset(1 as ::core::ffi::c_int as isize)
                                                >= (*mb).end_subject
                                            && (*mb).moptions & 0x20 as uint32_t != 0 as uint32_t
                                            && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                            && (*mb).nllen == 2 as uint32_t
                                            && c == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                as uint32_t
                                        {
                                            partial_newline = TRUE as BOOL;
                                            could_continue = partial_newline;
                                        } else if c >= 256 as uint32_t
                                            && d != OP_DIGIT as ::core::ffi::c_int as uint32_t
                                            && d != OP_WHITESPACE as ::core::ffi::c_int as uint32_t
                                            && d != OP_WORDCHAR as ::core::ffi::c_int as uint32_t
                                            || c < 256 as uint32_t
                                                && (d != OP_ANY as ::core::ffi::c_int as uint32_t
                                                    || (if (*mb).nltype != NLTYPE_FIXED as uint32_t
                                                    {
                                                        (ptr < (*mb).end_subject
                                                            && _pcre2_is_newline_8(
                                                                ptr,
                                                                (*mb).nltype,
                                                                (*mb).end_subject,
                                                                &raw mut (*mb).nllen,
                                                                utf,
                                                            ) != 0)
                                                            as ::core::ffi::c_int
                                                    } else {
                                                        (ptr <= (*mb)
                                                            .end_subject
                                                            .offset(-((*mb).nllen as isize))
                                                            && *ptr as ::core::ffi::c_int
                                                                == (*mb).nl[0 as ::core::ffi::c_int
                                                                    as usize]
                                                                    as ::core::ffi::c_int
                                                            && ((*mb).nllen == 1 as uint32_t
                                                                || *ptr.offset(
                                                                    1 as ::core::ffi::c_int
                                                                        as isize,
                                                                )
                                                                    as ::core::ffi::c_int
                                                                    == (*mb).nl[1
                                                                        as ::core::ffi::c_int
                                                                        as usize]
                                                                        as ::core::ffi::c_int))
                                                            as ::core::ffi::c_int
                                                    }) == 0)
                                                && *ctypes.offset(c as isize) as ::core::ffi::c_int
                                                    & toptable1[d as usize] as ::core::ffi::c_int
                                                    ^ toptable2[d as usize] as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                        {
                                            count += 1;
                                            if count
                                                >= ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as ::core::ffi::c_int
                                            {
                                                let fresh38 = new_count;
                                                new_count = new_count + 1;
                                                if fresh38 < wscount {
                                                    (*next_new_state).offset = state_offset
                                                        + 1 as ::core::ffi::c_int
                                                        + 2 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else {
                                                let fresh39 = new_count;
                                                new_count = new_count + 1;
                                                if fresh39 < wscount {
                                                    (*next_new_state).offset = state_offset;
                                                    (*next_new_state).count = count;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                91 | 92 | 97 => {
                                    let fresh40 = active_count;
                                    active_count = active_count + 1;
                                    if fresh40 < wscount {
                                        (*next_active_state).offset = state_offset
                                            + 2 as ::core::ffi::c_int
                                            + 2 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    count = (*current_state).count;
                                    if clen > 0 as ::core::ffi::c_int {
                                        if d == OP_ANY as ::core::ffi::c_int as uint32_t
                                            && ptr.offset(1 as ::core::ffi::c_int as isize)
                                                >= (*mb).end_subject
                                            && (*mb).moptions & 0x20 as uint32_t != 0 as uint32_t
                                            && (*mb).nltype == NLTYPE_FIXED as uint32_t
                                            && (*mb).nllen == 2 as uint32_t
                                            && c == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                as uint32_t
                                        {
                                            partial_newline = TRUE as BOOL;
                                            could_continue = partial_newline;
                                        } else if c >= 256 as uint32_t
                                            && d != OP_DIGIT as ::core::ffi::c_int as uint32_t
                                            && d != OP_WHITESPACE as ::core::ffi::c_int as uint32_t
                                            && d != OP_WORDCHAR as ::core::ffi::c_int as uint32_t
                                            || c < 256 as uint32_t
                                                && (d != OP_ANY as ::core::ffi::c_int as uint32_t
                                                    || (if (*mb).nltype != NLTYPE_FIXED as uint32_t
                                                    {
                                                        (ptr < (*mb).end_subject
                                                            && _pcre2_is_newline_8(
                                                                ptr,
                                                                (*mb).nltype,
                                                                (*mb).end_subject,
                                                                &raw mut (*mb).nllen,
                                                                utf,
                                                            ) != 0)
                                                            as ::core::ffi::c_int
                                                    } else {
                                                        (ptr <= (*mb)
                                                            .end_subject
                                                            .offset(-((*mb).nllen as isize))
                                                            && *ptr as ::core::ffi::c_int
                                                                == (*mb).nl[0 as ::core::ffi::c_int
                                                                    as usize]
                                                                    as ::core::ffi::c_int
                                                            && ((*mb).nllen == 1 as uint32_t
                                                                || *ptr.offset(
                                                                    1 as ::core::ffi::c_int
                                                                        as isize,
                                                                )
                                                                    as ::core::ffi::c_int
                                                                    == (*mb).nl[1
                                                                        as ::core::ffi::c_int
                                                                        as usize]
                                                                        as ::core::ffi::c_int))
                                                            as ::core::ffi::c_int
                                                    }) == 0)
                                                && *ctypes.offset(c as isize) as ::core::ffi::c_int
                                                    & toptable1[d as usize] as ::core::ffi::c_int
                                                    ^ toptable2[d as usize] as ::core::ffi::c_int
                                                    != 0 as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == OP_TYPEPOSUPTO as ::core::ffi::c_int as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            count += 1;
                                            if count
                                                >= ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as ::core::ffi::c_int
                                            {
                                                let fresh41 = new_count;
                                                new_count = new_count + 1;
                                                if fresh41 < wscount {
                                                    (*next_new_state).offset = state_offset
                                                        + 2 as ::core::ffi::c_int
                                                        + 2 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else {
                                                let fresh42 = new_count;
                                                new_count = new_count + 1;
                                                if fresh42 < wscount {
                                                    (*next_new_state).offset = state_offset;
                                                    (*next_new_state).count = count;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                387 | 388 | 395 => {
                                    count = (*current_state).count;
                                    if count > 0 as ::core::ffi::c_int {
                                        let fresh43 = active_count;
                                        active_count = active_count + 1;
                                        if fresh43 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 4 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut OK_0: BOOL = 0;
                                        let mut chartype_2: ::core::ffi::c_int = 0;
                                        let mut cp_0: *const uint32_t =
                                            ::core::ptr::null::<uint32_t>();
                                        let mut prop_0: *const ucd_record =
                                            (&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                .offset(
                                                    *(&raw const _pcre2_ucd_stage2_8
                                                        as *const uint16_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_stage1_8
                                                                as *const uint16_t)
                                                                .offset(
                                                                    (c as ::core::ffi::c_int
                                                                        / UCD_BLOCK_SIZE)
                                                                        as isize,
                                                                )
                                                                as ::core::ffi::c_int
                                                                * UCD_BLOCK_SIZE
                                                                + c as ::core::ffi::c_int
                                                                    % UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        as isize,
                                                );
                                        match *code.offset(2 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                        {
                                            PT_LAMP => {
                                                chartype_2 =
                                                    (*prop_0).chartype as ::core::ffi::c_int;
                                                OK_0 = (chartype_2 == ucp_Lu as ::core::ffi::c_int
                                                    || chartype_2 == ucp_Ll as ::core::ffi::c_int
                                                    || chartype_2 == ucp_Lt as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_GC => {
                                                OK_0 = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset((*prop_0).chartype as isize)
                                                    == *code
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_PC => {
                                                OK_0 = ((*prop_0).chartype as ::core::ffi::c_int
                                                    == *code
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SC => {
                                                OK_0 = ((*prop_0).script as ::core::ffi::c_int
                                                    == *code
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SCX => {
                                                OK_0 = ((*prop_0).script as ::core::ffi::c_int
                                                    == *code
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                    || *(&raw const _pcre2_ucd_script_sets_8
                                                        as *const uint32_t)
                                                        .offset(
                                                            ((*prop_0).scriptx_bidiclass
                                                                as ::core::ffi::c_int
                                                                & 0x3ff as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                        .offset(
                                                            (*code.offset(
                                                                3 as ::core::ffi::c_int as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                                / 32 as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                        & (1 as uint32_t)
                                                            << *code.offset(
                                                                3 as ::core::ffi::c_int as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                                % 32 as ::core::ffi::c_int
                                                        != 0 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_ALNUM => {
                                                chartype_2 =
                                                    (*prop_0).chartype as ::core::ffi::c_int;
                                                OK_0 = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset(chartype_2 as isize)
                                                    == ucp_L as ::core::ffi::c_int as uint32_t
                                                    || *(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset(chartype_2 as isize)
                                                        == ucp_N as ::core::ffi::c_int as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SPACE | PT_PXSPACE => match c {
                                                9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                                | 8195 | 8196 | 8197 | 8198 | 8199 | 8200
                                                | 8201 | 8202 | 8239 | 8287 | 12288 | 10 | 11
                                                | 12 | 13 | 133 | 8232 | 8233 => {
                                                    OK_0 = TRUE as BOOL;
                                                }
                                                _ => {
                                                    OK_0 = (*(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset((*prop_0).chartype as isize)
                                                        == ucp_Z as ::core::ffi::c_int as uint32_t)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                }
                                            },
                                            PT_WORD => {
                                                chartype_2 =
                                                    (*prop_0).chartype as ::core::ffi::c_int;
                                                OK_0 = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset(chartype_2 as isize)
                                                    == ucp_L as ::core::ffi::c_int as uint32_t
                                                    || *(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset(chartype_2 as isize)
                                                        == ucp_N as ::core::ffi::c_int as uint32_t
                                                    || chartype_2 == ucp_Mn as ::core::ffi::c_int
                                                    || chartype_2 == ucp_Pc as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_CLIST => {
                                                cp_0 =
                                                    (&raw const _pcre2_ucd_caseless_sets_8
                                                        as *const uint32_t)
                                                        .offset(*code.offset(
                                                            3 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            as isize);
                                                loop {
                                                    if c < *cp_0 {
                                                        OK_0 = FALSE as BOOL;
                                                        break;
                                                    } else {
                                                        let fresh44 = cp_0;
                                                        cp_0 = cp_0.offset(1);
                                                        if !(c == *fresh44) {
                                                            continue;
                                                        }
                                                        OK_0 = TRUE as BOOL;
                                                        break;
                                                    }
                                                }
                                            }
                                            PT_UCNC => {
                                                OK_0 = (c == CHAR_DOLLAR_SIGN as uint32_t
                                                    || c == CHAR_COMMERCIAL_AT as uint32_t
                                                    || c == CHAR_GRAVE_ACCENT as uint32_t
                                                    || c >= 0xa0 as uint32_t
                                                        && c <= 0xd7ff as uint32_t
                                                    || c >= 0xe000 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_BIDICL => {
                                                OK_0 = ((*(&raw const _pcre2_ucd_records_8
                                                    as *const ucd_record)
                                                    .offset(
                                                        *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                            .offset(
                                                                (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                    .offset(
                                                                        (c as ::core::ffi::c_int / 128 as ::core::ffi::c_int)
                                                                            as isize,
                                                                    ) as ::core::ffi::c_int * 128 as ::core::ffi::c_int
                                                                    + c as ::core::ffi::c_int % 128 as ::core::ffi::c_int)
                                                                    as isize,
                                                            ) as ::core::ffi::c_int as isize,
                                                    ))
                                                    .scriptx_bidiclass as ::core::ffi::c_int
                                                    >> UCD_BIDICLASS_SHIFT
                                                    == *code.offset(3 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int) as ::core::ffi::c_int as BOOL;
                                            }
                                            PT_BOOL => {
                                                OK_0 = (*(&raw const _pcre2_ucd_boolprop_sets_8
                                                    as *const uint32_t)
                                                    .offset(
                                                        ((*prop_0).bprops as ::core::ffi::c_int
                                                            & 0xfff as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                    .offset(
                                                        (*code.offset(
                                                            3 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            / 32 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                    & (1 as uint32_t)
                                                        << *code.offset(
                                                            3 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            % 32 as ::core::ffi::c_int
                                                    != 0 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            _ => {
                                                OK_0 = (codevalue
                                                    != OP_PROP as ::core::ffi::c_int as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                        }
                                        if OK_0
                                            == (d == OP_PROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if count > 0 as ::core::ffi::c_int
                                                && codevalue
                                                    == (OP_PROP_EXTRA
                                                        + OP_TYPEPOSPLUS as ::core::ffi::c_int)
                                                        as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            count += 1;
                                            let fresh45 = new_count;
                                            new_count = new_count + 1;
                                            if fresh45 < wscount {
                                                (*next_new_state).offset = state_offset;
                                                (*next_new_state).count = count;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                407 | 408 | 415 => {
                                    count = (*current_state).count;
                                    if count > 0 as ::core::ffi::c_int {
                                        let fresh46 = active_count;
                                        active_count = active_count + 1;
                                        if fresh46 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 2 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut ncount: ::core::ffi::c_int =
                                            0 as ::core::ffi::c_int;
                                        if count > 0 as ::core::ffi::c_int
                                            && codevalue
                                                == (OP_EXTUNI_EXTRA
                                                    + OP_TYPEPOSPLUS as ::core::ffi::c_int)
                                                    as uint32_t
                                        {
                                            active_count -= 1;
                                            next_active_state = next_active_state.offset(-1);
                                        }
                                        _pcre2_extuni_8(
                                            c,
                                            ptr.offset(clen as isize),
                                            (*mb).start_subject,
                                            end_subject,
                                            utf,
                                            &raw mut ncount,
                                        );
                                        count += 1;
                                        let fresh47 = new_count;
                                        new_count = new_count + 1;
                                        if fresh47 < wscount {
                                            (*next_new_state).offset = -state_offset;
                                            (*next_new_state).count = count;
                                            (*next_new_state).data = ncount;
                                            next_new_state = next_new_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                427 | 428 | 435 => {
                                    count = (*current_state).count;
                                    if count > 0 as ::core::ffi::c_int {
                                        let fresh48 = active_count;
                                        active_count = active_count + 1;
                                        if fresh48 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 2 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut ncount_0: ::core::ffi::c_int =
                                            0 as ::core::ffi::c_int;
                                        let mut current_block_711: u64;
                                        match c {
                                            11 | 12 | 133 | 8232 | 8233 => {
                                                if (*mb).bsr_convention as ::core::ffi::c_int
                                                    == PCRE2_BSR_ANYCRLF
                                                {
                                                    current_block_711 = 15883148644676414581;
                                                } else {
                                                    current_block_711 = 8474664308492183916;
                                                }
                                            }
                                            13 => {
                                                if ptr.offset(1 as ::core::ffi::c_int as isize)
                                                    < end_subject
                                                    && *ptr.offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                        == CHAR_LF
                                                {
                                                    ncount_0 = 1 as ::core::ffi::c_int;
                                                }
                                                current_block_711 = 8474664308492183916;
                                            }
                                            10 => {
                                                current_block_711 = 8474664308492183916;
                                            }
                                            _ => {
                                                current_block_711 = 15883148644676414581;
                                            }
                                        }
                                        match current_block_711 {
                                            8474664308492183916 => {
                                                if count > 0 as ::core::ffi::c_int
                                                    && codevalue
                                                        == (OP_ANYNL_EXTRA
                                                            + OP_TYPEPOSPLUS as ::core::ffi::c_int)
                                                            as uint32_t
                                                {
                                                    active_count -= 1;
                                                    next_active_state =
                                                        next_active_state.offset(-1);
                                                }
                                                count += 1;
                                                let fresh49 = new_count;
                                                new_count = new_count + 1;
                                                if fresh49 < wscount {
                                                    (*next_new_state).offset = -state_offset;
                                                    (*next_new_state).count = count;
                                                    (*next_new_state).data = ncount_0;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                467 | 468 | 475 => {
                                    count = (*current_state).count;
                                    if count > 0 as ::core::ffi::c_int {
                                        let fresh50 = active_count;
                                        active_count = active_count + 1;
                                        if fresh50 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 2 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut OK_1: BOOL = 0;
                                        match c {
                                            10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                                OK_1 = TRUE as BOOL;
                                            }
                                            _ => {
                                                OK_1 = FALSE as BOOL;
                                            }
                                        }
                                        if OK_1
                                            == (d == OP_VSPACE as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if count > 0 as ::core::ffi::c_int
                                                && codevalue
                                                    == (OP_VSPACE_EXTRA
                                                        + OP_TYPEPOSPLUS as ::core::ffi::c_int)
                                                        as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            count += 1;
                                            let fresh51 = new_count;
                                            new_count = new_count + 1;
                                            if fresh51 < wscount {
                                                (*next_new_state).offset = -state_offset;
                                                (*next_new_state).count = count;
                                                (*next_new_state).data = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                447 | 448 | 455 => {
                                    count = (*current_state).count;
                                    if count > 0 as ::core::ffi::c_int {
                                        let fresh52 = active_count;
                                        active_count = active_count + 1;
                                        if fresh52 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + 2 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut OK_2: BOOL = 0;
                                        match c {
                                            9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                            | 8195 | 8196 | 8197 | 8198 | 8199 | 8200 | 8201
                                            | 8202 | 8239 | 8287 | 12288 => {
                                                OK_2 = TRUE as BOOL;
                                            }
                                            _ => {
                                                OK_2 = FALSE as BOOL;
                                            }
                                        }
                                        if OK_2
                                            == (d == OP_HSPACE as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if count > 0 as ::core::ffi::c_int
                                                && codevalue
                                                    == (OP_HSPACE_EXTRA
                                                        + OP_TYPEPOSPLUS as ::core::ffi::c_int)
                                                        as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            count += 1;
                                            let fresh53 = new_count;
                                            new_count = new_count + 1;
                                            if fresh53 < wscount {
                                                (*next_new_state).offset = -state_offset;
                                                (*next_new_state).count = count;
                                                (*next_new_state).data = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                389 | 390 | 396 => {
                                    count = 4 as ::core::ffi::c_int;
                                    current_block_1814 = 4957409898739771784;
                                }
                                385 | 386 | 394 => {
                                    count = 0 as ::core::ffi::c_int;
                                    current_block_1814 = 4957409898739771784;
                                }
                                409 | 410 | 416 => {
                                    count = 2 as ::core::ffi::c_int;
                                    current_block_1814 = 3672113957794243512;
                                }
                                405 | 406 | 414 => {
                                    count = 0 as ::core::ffi::c_int;
                                    current_block_1814 = 3672113957794243512;
                                }
                                429 | 430 | 436 => {
                                    count = 2 as ::core::ffi::c_int;
                                    current_block_1814 = 4040471979405040113;
                                }
                                425 | 426 | 434 => {
                                    count = 0 as ::core::ffi::c_int;
                                    current_block_1814 = 4040471979405040113;
                                }
                                469 | 470 | 476 => {
                                    count = 2 as ::core::ffi::c_int;
                                    current_block_1814 = 3015753210103841382;
                                }
                                465 | 466 | 474 => {
                                    count = 0 as ::core::ffi::c_int;
                                    current_block_1814 = 3015753210103841382;
                                }
                                449 | 450 | 456 => {
                                    count = 2 as ::core::ffi::c_int;
                                    current_block_1814 = 16249953603520743654;
                                }
                                445 | 446 | 454 => {
                                    count = 0 as ::core::ffi::c_int;
                                    current_block_1814 = 16249953603520743654;
                                }
                                393 | 391 | 392 | 397 => {
                                    if codevalue
                                        != (OP_PROP_EXTRA + OP_TYPEEXACT as ::core::ffi::c_int)
                                            as uint32_t
                                    {
                                        let fresh65 = active_count;
                                        active_count = active_count + 1;
                                        if fresh65 < wscount {
                                            (*next_active_state).offset = state_offset
                                                + 1 as ::core::ffi::c_int
                                                + 2 as ::core::ffi::c_int
                                                + 3 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    count = (*current_state).count;
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut OK_6: BOOL = 0;
                                        let mut chartype_4: ::core::ffi::c_int = 0;
                                        let mut cp_2: *const uint32_t =
                                            ::core::ptr::null::<uint32_t>();
                                        let mut prop_2: *const ucd_record =
                                            (&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                .offset(
                                                    *(&raw const _pcre2_ucd_stage2_8
                                                        as *const uint16_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_stage1_8
                                                                as *const uint16_t)
                                                                .offset(
                                                                    (c as ::core::ffi::c_int
                                                                        / UCD_BLOCK_SIZE)
                                                                        as isize,
                                                                )
                                                                as ::core::ffi::c_int
                                                                * UCD_BLOCK_SIZE
                                                                + c as ::core::ffi::c_int
                                                                    % UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        as isize,
                                                );
                                        match *code.offset(
                                            (1 as ::core::ffi::c_int
                                                + IMM2_SIZE
                                                + 1 as ::core::ffi::c_int)
                                                as isize,
                                        )
                                            as ::core::ffi::c_int
                                        {
                                            PT_LAMP => {
                                                chartype_4 =
                                                    (*prop_2).chartype as ::core::ffi::c_int;
                                                OK_6 = (chartype_4 == ucp_Lu as ::core::ffi::c_int
                                                    || chartype_4 == ucp_Ll as ::core::ffi::c_int
                                                    || chartype_4 == ucp_Lt as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_GC => {
                                                OK_6 = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset((*prop_2).chartype as isize)
                                                    == *code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + IMM2_SIZE
                                                            + 2 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_PC => {
                                                OK_6 = ((*prop_2).chartype as ::core::ffi::c_int
                                                    == *code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + IMM2_SIZE
                                                            + 2 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SC => {
                                                OK_6 = ((*prop_2).script as ::core::ffi::c_int
                                                    == *code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + IMM2_SIZE
                                                            + 2 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SCX => {
                                                OK_6 = ((*prop_2).script as ::core::ffi::c_int
                                                    == *code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + IMM2_SIZE
                                                            + 2 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int
                                                    || *(&raw const _pcre2_ucd_script_sets_8
                                                        as *const uint32_t)
                                                        .offset(
                                                            ((*prop_2).scriptx_bidiclass
                                                                as ::core::ffi::c_int
                                                                & 0x3ff as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                        .offset(
                                                            (*code.offset(
                                                                (1 as ::core::ffi::c_int
                                                                    + 2 as ::core::ffi::c_int
                                                                    + 2 as ::core::ffi::c_int)
                                                                    as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                                / 32 as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                        & (1 as uint32_t)
                                                            << *code.offset(
                                                                (1 as ::core::ffi::c_int
                                                                    + 2 as ::core::ffi::c_int
                                                                    + 2 as ::core::ffi::c_int)
                                                                    as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                                % 32 as ::core::ffi::c_int
                                                        != 0 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_ALNUM => {
                                                chartype_4 =
                                                    (*prop_2).chartype as ::core::ffi::c_int;
                                                OK_6 = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset(chartype_4 as isize)
                                                    == ucp_L as ::core::ffi::c_int as uint32_t
                                                    || *(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset(chartype_4 as isize)
                                                        == ucp_N as ::core::ffi::c_int as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SPACE | PT_PXSPACE => match c {
                                                9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                                | 8195 | 8196 | 8197 | 8198 | 8199 | 8200
                                                | 8201 | 8202 | 8239 | 8287 | 12288 | 10 | 11
                                                | 12 | 13 | 133 | 8232 | 8233 => {
                                                    OK_6 = TRUE as BOOL;
                                                }
                                                _ => {
                                                    OK_6 = (*(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset((*prop_2).chartype as isize)
                                                        == ucp_Z as ::core::ffi::c_int as uint32_t)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                }
                                            },
                                            PT_WORD => {
                                                chartype_4 =
                                                    (*prop_2).chartype as ::core::ffi::c_int;
                                                OK_6 = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset(chartype_4 as isize)
                                                    == ucp_L as ::core::ffi::c_int as uint32_t
                                                    || *(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset(chartype_4 as isize)
                                                        == ucp_N as ::core::ffi::c_int as uint32_t
                                                    || chartype_4 == ucp_Mn as ::core::ffi::c_int
                                                    || chartype_4 == ucp_Pc as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_CLIST => {
                                                cp_2 = (&raw const _pcre2_ucd_caseless_sets_8
                                                    as *const uint32_t)
                                                    .offset(*code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + IMM2_SIZE
                                                            + 2 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int
                                                        as isize);
                                                loop {
                                                    if c < *cp_2 {
                                                        OK_6 = FALSE as BOOL;
                                                        break;
                                                    } else {
                                                        let fresh66 = cp_2;
                                                        cp_2 = cp_2.offset(1);
                                                        if !(c == *fresh66) {
                                                            continue;
                                                        }
                                                        OK_6 = TRUE as BOOL;
                                                        break;
                                                    }
                                                }
                                            }
                                            PT_UCNC => {
                                                OK_6 = (c == CHAR_DOLLAR_SIGN as uint32_t
                                                    || c == CHAR_COMMERCIAL_AT as uint32_t
                                                    || c == CHAR_GRAVE_ACCENT as uint32_t
                                                    || c >= 0xa0 as uint32_t
                                                        && c <= 0xd7ff as uint32_t
                                                    || c >= 0xe000 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_BIDICL => {
                                                OK_6 = ((*(&raw const _pcre2_ucd_records_8
                                                    as *const ucd_record)
                                                    .offset(
                                                        *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                            .offset(
                                                                (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                    .offset(
                                                                        (c as ::core::ffi::c_int / 128 as ::core::ffi::c_int)
                                                                            as isize,
                                                                    ) as ::core::ffi::c_int * 128 as ::core::ffi::c_int
                                                                    + c as ::core::ffi::c_int % 128 as ::core::ffi::c_int)
                                                                    as isize,
                                                            ) as ::core::ffi::c_int as isize,
                                                    ))
                                                    .scriptx_bidiclass as ::core::ffi::c_int
                                                    >> UCD_BIDICLASS_SHIFT
                                                    == *code
                                                        .offset(
                                                            (1 as ::core::ffi::c_int + IMM2_SIZE
                                                                + 2 as ::core::ffi::c_int) as isize,
                                                        ) as ::core::ffi::c_int) as ::core::ffi::c_int as BOOL;
                                            }
                                            PT_BOOL => {
                                                OK_6 = (*(&raw const _pcre2_ucd_boolprop_sets_8
                                                    as *const uint32_t)
                                                    .offset(
                                                        ((*prop_2).bprops as ::core::ffi::c_int
                                                            & 0xfff as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                    .offset(
                                                        (*code.offset(
                                                            (1 as ::core::ffi::c_int
                                                                + 2 as ::core::ffi::c_int
                                                                + 2 as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            / 32 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                    & (1 as uint32_t)
                                                        << *code.offset(
                                                            (1 as ::core::ffi::c_int
                                                                + 2 as ::core::ffi::c_int
                                                                + 2 as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            % 32 as ::core::ffi::c_int
                                                    != 0 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            _ => {
                                                OK_6 = (codevalue
                                                    != OP_PROP as ::core::ffi::c_int as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                        }
                                        if OK_6
                                            == (d == OP_PROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == (OP_PROP_EXTRA
                                                    + OP_TYPEPOSUPTO as ::core::ffi::c_int)
                                                    as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            count += 1;
                                            if count
                                                >= ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as ::core::ffi::c_int
                                            {
                                                let fresh67 = new_count;
                                                new_count = new_count + 1;
                                                if fresh67 < wscount {
                                                    (*next_new_state).offset = state_offset
                                                        + 1 as ::core::ffi::c_int
                                                        + 2 as ::core::ffi::c_int
                                                        + 3 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else {
                                                let fresh68 = new_count;
                                                new_count = new_count + 1;
                                                if fresh68 < wscount {
                                                    (*next_new_state).offset = state_offset;
                                                    (*next_new_state).count = count;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                413 | 411 | 412 | 417 => {
                                    if codevalue
                                        != (OP_EXTUNI_EXTRA + OP_TYPEEXACT as ::core::ffi::c_int)
                                            as uint32_t
                                    {
                                        let fresh69 = active_count;
                                        active_count = active_count + 1;
                                        if fresh69 < wscount {
                                            (*next_active_state).offset = state_offset
                                                + 2 as ::core::ffi::c_int
                                                + 2 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    count = (*current_state).count;
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut nptr: PCRE2_SPTR8 =
                                            ::core::ptr::null::<PCRE2_UCHAR8>();
                                        let mut ncount_3: ::core::ffi::c_int =
                                            0 as ::core::ffi::c_int;
                                        if codevalue
                                            == (OP_EXTUNI_EXTRA
                                                + OP_TYPEPOSUPTO as ::core::ffi::c_int)
                                                as uint32_t
                                        {
                                            active_count -= 1;
                                            next_active_state = next_active_state.offset(-1);
                                        }
                                        nptr = _pcre2_extuni_8(
                                            c,
                                            ptr.offset(clen as isize),
                                            (*mb).start_subject,
                                            end_subject,
                                            utf,
                                            &raw mut ncount_3,
                                        );
                                        if nptr >= end_subject
                                            && (*mb).moptions & PCRE2_PARTIAL_HARD as uint32_t
                                                != 0 as uint32_t
                                        {
                                            reset_could_continue = TRUE as BOOL;
                                        }
                                        count += 1;
                                        if count
                                            >= ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 8 as ::core::ffi::c_int
                                                | *code.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint
                                                as ::core::ffi::c_int
                                        {
                                            let fresh70 = new_count;
                                            new_count = new_count + 1;
                                            if fresh70 < wscount {
                                                (*next_new_state).offset = -(state_offset
                                                    + 2 as ::core::ffi::c_int
                                                    + 2 as ::core::ffi::c_int);
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                (*next_new_state).data = ncount_3;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        } else {
                                            let fresh71 = new_count;
                                            new_count = new_count + 1;
                                            if fresh71 < wscount {
                                                (*next_new_state).offset = -state_offset;
                                                (*next_new_state).count = count;
                                                (*next_new_state).data = ncount_3;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                433 | 431 | 432 | 437 => {
                                    if codevalue
                                        != (OP_ANYNL_EXTRA + OP_TYPEEXACT as ::core::ffi::c_int)
                                            as uint32_t
                                    {
                                        let fresh72 = active_count;
                                        active_count = active_count + 1;
                                        if fresh72 < wscount {
                                            (*next_active_state).offset = state_offset
                                                + 2 as ::core::ffi::c_int
                                                + 2 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    count = (*current_state).count;
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut ncount_4: ::core::ffi::c_int =
                                            0 as ::core::ffi::c_int;
                                        let mut current_block_1033: u64;
                                        match c {
                                            11 | 12 | 133 | 8232 | 8233 => {
                                                if (*mb).bsr_convention as ::core::ffi::c_int
                                                    == PCRE2_BSR_ANYCRLF
                                                {
                                                    current_block_1033 = 9651002015275348988;
                                                } else {
                                                    current_block_1033 = 3205284525055210370;
                                                }
                                            }
                                            13 => {
                                                if ptr.offset(1 as ::core::ffi::c_int as isize)
                                                    < end_subject
                                                    && *ptr.offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                        == CHAR_LF
                                                {
                                                    ncount_4 = 1 as ::core::ffi::c_int;
                                                }
                                                current_block_1033 = 3205284525055210370;
                                            }
                                            10 => {
                                                current_block_1033 = 3205284525055210370;
                                            }
                                            _ => {
                                                current_block_1033 = 9651002015275348988;
                                            }
                                        }
                                        match current_block_1033 {
                                            3205284525055210370 => {
                                                if codevalue
                                                    == (OP_ANYNL_EXTRA
                                                        + OP_TYPEPOSUPTO as ::core::ffi::c_int)
                                                        as uint32_t
                                                {
                                                    active_count -= 1;
                                                    next_active_state =
                                                        next_active_state.offset(-1);
                                                }
                                                count += 1;
                                                if count
                                                    >= ((*code
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int)
                                                        << 8 as ::core::ffi::c_int
                                                        | *code.offset(
                                                            (1 as ::core::ffi::c_int
                                                                + 1 as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                            as ::core::ffi::c_int)
                                                        as ::core::ffi::c_uint
                                                        as ::core::ffi::c_int
                                                {
                                                    let fresh73 = new_count;
                                                    new_count = new_count + 1;
                                                    if fresh73 < wscount {
                                                        (*next_new_state).offset = -(state_offset
                                                            + 2 as ::core::ffi::c_int
                                                            + 2 as ::core::ffi::c_int);
                                                        (*next_new_state).count =
                                                            0 as ::core::ffi::c_int;
                                                        (*next_new_state).data = ncount_4;
                                                        next_new_state = next_new_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                } else {
                                                    let fresh74 = new_count;
                                                    new_count = new_count + 1;
                                                    if fresh74 < wscount {
                                                        (*next_new_state).offset = -state_offset;
                                                        (*next_new_state).count = count;
                                                        (*next_new_state).data = ncount_4;
                                                        next_new_state = next_new_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                473 | 471 | 472 | 477 => {
                                    if codevalue
                                        != (OP_VSPACE_EXTRA + OP_TYPEEXACT as ::core::ffi::c_int)
                                            as uint32_t
                                    {
                                        let fresh75 = active_count;
                                        active_count = active_count + 1;
                                        if fresh75 < wscount {
                                            (*next_active_state).offset = state_offset
                                                + 2 as ::core::ffi::c_int
                                                + 2 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    count = (*current_state).count;
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut OK_7: BOOL = 0;
                                        match c {
                                            10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                                OK_7 = TRUE as BOOL;
                                            }
                                            _ => {
                                                OK_7 = FALSE as BOOL;
                                            }
                                        }
                                        if OK_7
                                            == (d == OP_VSPACE as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == (OP_VSPACE_EXTRA
                                                    + OP_TYPEPOSUPTO as ::core::ffi::c_int)
                                                    as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            count += 1;
                                            if count
                                                >= ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as ::core::ffi::c_int
                                            {
                                                let fresh76 = new_count;
                                                new_count = new_count + 1;
                                                if fresh76 < wscount {
                                                    (*next_new_state).offset = -(state_offset
                                                        + 2 as ::core::ffi::c_int
                                                        + 2 as ::core::ffi::c_int);
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    (*next_new_state).data =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else {
                                                let fresh77 = new_count;
                                                new_count = new_count + 1;
                                                if fresh77 < wscount {
                                                    (*next_new_state).offset = -state_offset;
                                                    (*next_new_state).count = count;
                                                    (*next_new_state).data =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                453 | 451 | 452 | 457 => {
                                    if codevalue
                                        != (OP_HSPACE_EXTRA + OP_TYPEEXACT as ::core::ffi::c_int)
                                            as uint32_t
                                    {
                                        let fresh78 = active_count;
                                        active_count = active_count + 1;
                                        if fresh78 < wscount {
                                            (*next_active_state).offset = state_offset
                                                + 2 as ::core::ffi::c_int
                                                + 2 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    count = (*current_state).count;
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut OK_8: BOOL = 0;
                                        match c {
                                            9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                            | 8195 | 8196 | 8197 | 8198 | 8199 | 8200 | 8201
                                            | 8202 | 8239 | 8287 | 12288 => {
                                                OK_8 = TRUE as BOOL;
                                            }
                                            _ => {
                                                OK_8 = FALSE as BOOL;
                                            }
                                        }
                                        if OK_8
                                            == (d == OP_HSPACE as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == (OP_HSPACE_EXTRA
                                                    + OP_TYPEPOSUPTO as ::core::ffi::c_int)
                                                    as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            count += 1;
                                            if count
                                                >= ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as ::core::ffi::c_int
                                            {
                                                let fresh79 = new_count;
                                                new_count = new_count + 1;
                                                if fresh79 < wscount {
                                                    (*next_new_state).offset = -(state_offset
                                                        + 2 as ::core::ffi::c_int
                                                        + 2 as ::core::ffi::c_int);
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    (*next_new_state).data =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else {
                                                let fresh80 = new_count;
                                                new_count = new_count + 1;
                                                if fresh80 < wscount {
                                                    (*next_new_state).offset = -state_offset;
                                                    (*next_new_state).count = count;
                                                    (*next_new_state).data =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                29 => {
                                    if clen > 0 as ::core::ffi::c_int && c == d {
                                        let fresh81 = new_count;
                                        new_count = new_count + 1;
                                        if fresh81 < wscount {
                                            (*next_new_state).offset =
                                                state_offset + dlen + 1 as ::core::ffi::c_int;
                                            (*next_new_state).count = 0 as ::core::ffi::c_int;
                                            next_new_state = next_new_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                30 => {
                                    if clen == 0 as ::core::ffi::c_int {
                                        current_block_1814 = 14107599004487389265;
                                    } else {
                                        if utf_or_ucp != 0 {
                                            if c == d {
                                                let fresh82 = new_count;
                                                new_count = new_count + 1;
                                                if fresh82 < wscount {
                                                    (*next_new_state).offset = state_offset
                                                        + dlen
                                                        + 1 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else {
                                                let mut othercase: ::core::ffi::c_uint = 0;
                                                if c < 128 as uint32_t {
                                                    othercase = *fcc.offset(c as isize)
                                                        as ::core::ffi::c_uint;
                                                } else {
                                                    othercase = (c as ::core::ffi::c_int
                                                        + (*(&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                            .offset(
                                                                *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                                    .offset(
                                                                        (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                            .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                                                                            as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                            + c as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                    ) as ::core::ffi::c_int as isize,
                                                            ))
                                                            .other_case as ::core::ffi::c_int) as uint32_t
                                                        as ::core::ffi::c_uint;
                                                }
                                                if d == othercase as uint32_t {
                                                    let fresh83 = new_count;
                                                    new_count = new_count + 1;
                                                    if fresh83 < wscount {
                                                        (*next_new_state).offset = state_offset
                                                            + dlen
                                                            + 1 as ::core::ffi::c_int;
                                                        (*next_new_state).count =
                                                            0 as ::core::ffi::c_int;
                                                        next_new_state = next_new_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                }
                                            }
                                        } else if *lcc.offset(c as isize) as ::core::ffi::c_int
                                            == *lcc.offset(d as isize) as ::core::ffi::c_int
                                        {
                                            let fresh84 = new_count;
                                            new_count = new_count + 1;
                                            if fresh84 < wscount {
                                                (*next_new_state).offset =
                                                    state_offset + 2 as ::core::ffi::c_int;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                        current_block_1814 = 14107599004487389265;
                                    }
                                }
                                22 => {
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut ncount_5: ::core::ffi::c_int =
                                            0 as ::core::ffi::c_int;
                                        let mut nptr_0: PCRE2_SPTR8 = _pcre2_extuni_8(
                                            c,
                                            ptr.offset(clen as isize),
                                            (*mb).start_subject,
                                            end_subject,
                                            utf,
                                            &raw mut ncount_5,
                                        );
                                        if nptr_0 >= end_subject
                                            && (*mb).moptions & PCRE2_PARTIAL_HARD as uint32_t
                                                != 0 as uint32_t
                                        {
                                            reset_could_continue = TRUE as BOOL;
                                        }
                                        let fresh85 = new_count;
                                        new_count = new_count + 1;
                                        if fresh85 < wscount {
                                            (*next_new_state).offset =
                                                -(state_offset + 1 as ::core::ffi::c_int);
                                            (*next_new_state).count = 0 as ::core::ffi::c_int;
                                            (*next_new_state).data = ncount_5;
                                            next_new_state = next_new_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                17 => {
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut current_block_1193: u64;
                                        match c {
                                            11 | 12 | 133 | 8232 | 8233 => {
                                                if (*mb).bsr_convention as ::core::ffi::c_int
                                                    == PCRE2_BSR_ANYCRLF
                                                {
                                                    current_block_1193 = 12289794451381484760;
                                                } else {
                                                    current_block_1193 = 538492802611270279;
                                                }
                                            }
                                            10 => {
                                                current_block_1193 = 538492802611270279;
                                            }
                                            13 => {
                                                if ptr.offset(1 as ::core::ffi::c_int as isize)
                                                    >= end_subject
                                                {
                                                    let fresh87 = new_count;
                                                    new_count = new_count + 1;
                                                    if fresh87 < wscount {
                                                        (*next_new_state).offset =
                                                            state_offset + 1 as ::core::ffi::c_int;
                                                        (*next_new_state).count =
                                                            0 as ::core::ffi::c_int;
                                                        next_new_state = next_new_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                    if (*mb).moptions
                                                        & PCRE2_PARTIAL_HARD as uint32_t
                                                        != 0 as uint32_t
                                                    {
                                                        reset_could_continue = TRUE as BOOL;
                                                    }
                                                } else if *ptr
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int
                                                    == CHAR_LF
                                                {
                                                    let fresh88 = new_count;
                                                    new_count = new_count + 1;
                                                    if fresh88 < wscount {
                                                        (*next_new_state).offset = -(state_offset
                                                            + 1 as ::core::ffi::c_int);
                                                        (*next_new_state).count =
                                                            0 as ::core::ffi::c_int;
                                                        (*next_new_state).data =
                                                            1 as ::core::ffi::c_int;
                                                        next_new_state = next_new_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                } else {
                                                    let fresh89 = new_count;
                                                    new_count = new_count + 1;
                                                    if fresh89 < wscount {
                                                        (*next_new_state).offset =
                                                            state_offset + 1 as ::core::ffi::c_int;
                                                        (*next_new_state).count =
                                                            0 as ::core::ffi::c_int;
                                                        next_new_state = next_new_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                }
                                                current_block_1193 = 12289794451381484760;
                                            }
                                            _ => {
                                                current_block_1193 = 12289794451381484760;
                                            }
                                        }
                                        match current_block_1193 {
                                            538492802611270279 => {
                                                let fresh86 = new_count;
                                                new_count = new_count + 1;
                                                if fresh86 < wscount {
                                                    (*next_new_state).offset =
                                                        state_offset + 1 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                20 => {
                                    if clen > 0 as ::core::ffi::c_int {
                                        match c {
                                            10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {}
                                            _ => {
                                                let fresh90 = new_count;
                                                new_count = new_count + 1;
                                                if fresh90 < wscount {
                                                    (*next_new_state).offset =
                                                        state_offset + 1 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                21 => {
                                    if clen > 0 as ::core::ffi::c_int {
                                        match c {
                                            10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                                let fresh91 = new_count;
                                                new_count = new_count + 1;
                                                if fresh91 < wscount {
                                                    (*next_new_state).offset =
                                                        state_offset + 1 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                18 => {
                                    if clen > 0 as ::core::ffi::c_int {
                                        match c {
                                            9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                            | 8195 | 8196 | 8197 | 8198 | 8199 | 8200 | 8201
                                            | 8202 | 8239 | 8287 | 12288 => {}
                                            _ => {
                                                let fresh92 = new_count;
                                                new_count = new_count + 1;
                                                if fresh92 < wscount {
                                                    (*next_new_state).offset =
                                                        state_offset + 1 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                19 => {
                                    if clen > 0 as ::core::ffi::c_int {
                                        match c {
                                            9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                            | 8195 | 8196 | 8197 | 8198 | 8199 | 8200 | 8201
                                            | 8202 | 8239 | 8287 | 12288 => {
                                                let fresh93 = new_count;
                                                new_count = new_count + 1;
                                                if fresh93 < wscount {
                                                    (*next_new_state).offset =
                                                        state_offset + 1 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                31 => {
                                    if clen > 0 as ::core::ffi::c_int && c != d {
                                        let fresh94 = new_count;
                                        new_count = new_count + 1;
                                        if fresh94 < wscount {
                                            (*next_new_state).offset =
                                                state_offset + dlen + 1 as ::core::ffi::c_int;
                                            (*next_new_state).count = 0 as ::core::ffi::c_int;
                                            next_new_state = next_new_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                32 => {
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut otherd: uint32_t = 0;
                                        if utf_or_ucp != 0 && d >= 128 as uint32_t {
                                            otherd = (d as ::core::ffi::c_int
                                                + (*(&raw const _pcre2_ucd_records_8
                                                    as *const ucd_record)
                                                    .offset(
                                                        *(&raw const _pcre2_ucd_stage2_8
                                                            as *const uint16_t)
                                                            .offset(
                                                                (*(&raw const _pcre2_ucd_stage1_8
                                                                    as *const uint16_t)
                                                                    .offset(
                                                                        (d as ::core::ffi::c_int
                                                                            / UCD_BLOCK_SIZE)
                                                                            as isize,
                                                                    )
                                                                    as ::core::ffi::c_int
                                                                    * UCD_BLOCK_SIZE
                                                                    + d as ::core::ffi::c_int
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
                                            otherd = *fcc.offset(d as isize) as uint32_t;
                                        }
                                        if c != d && c != otherd {
                                            let fresh95 = new_count;
                                            new_count = new_count + 1;
                                            if fresh95 < wscount {
                                                (*next_new_state).offset =
                                                    state_offset + dlen + 1 as ::core::ffi::c_int;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                48 | 49 | 56 | 74 | 75 | 82 => {
                                    caseless = TRUE as BOOL;
                                    codevalue = codevalue.wrapping_sub(
                                        (OP_STARI as ::core::ffi::c_int
                                            - OP_STAR as ::core::ffi::c_int)
                                            as uint32_t,
                                    );
                                    current_block_1814 = 12389340984333060227;
                                }
                                35 | 36 | 43 | 61 | 62 | 69 => {
                                    current_block_1814 = 12389340984333060227;
                                }
                                50 | 51 | 57 | 76 | 77 | 83 => {
                                    caseless = TRUE as BOOL;
                                    codevalue = codevalue.wrapping_sub(
                                        (OP_STARI as ::core::ffi::c_int
                                            - OP_STAR as ::core::ffi::c_int)
                                            as uint32_t,
                                    );
                                    current_block_1814 = 15321635274243100895;
                                }
                                37 | 38 | 44 | 63 | 64 | 70 => {
                                    current_block_1814 = 15321635274243100895;
                                }
                                46 | 47 | 55 | 72 | 73 | 81 => {
                                    caseless = TRUE as BOOL;
                                    codevalue = codevalue.wrapping_sub(
                                        (OP_STARI as ::core::ffi::c_int
                                            - OP_STAR as ::core::ffi::c_int)
                                            as uint32_t,
                                    );
                                    current_block_1814 = 6529072442217564396;
                                }
                                33 | 34 | 42 | 59 | 60 | 68 => {
                                    current_block_1814 = 6529072442217564396;
                                }
                                54 | 80 => {
                                    caseless = TRUE as BOOL;
                                    codevalue = codevalue.wrapping_sub(
                                        (OP_STARI as ::core::ffi::c_int
                                            - OP_STAR as ::core::ffi::c_int)
                                            as uint32_t,
                                    );
                                    current_block_1814 = 12606961696430611236;
                                }
                                41 | 67 => {
                                    current_block_1814 = 12606961696430611236;
                                }
                                52 | 53 | 58 | 78 | 79 | 84 => {
                                    caseless = TRUE as BOOL;
                                    codevalue = codevalue.wrapping_sub(
                                        (OP_STARI as ::core::ffi::c_int
                                            - OP_STAR as ::core::ffi::c_int)
                                            as uint32_t,
                                    );
                                    current_block_1814 = 4611645196188390243;
                                }
                                39 | 40 | 45 | 65 | 66 | 71 => {
                                    current_block_1814 = 4611645196188390243;
                                }
                                110 | 111 | 112 | 113 => {
                                    let mut isinclass: BOOL = FALSE;
                                    let mut next_state_offset: ::core::ffi::c_int = 0;
                                    let mut ecode: PCRE2_SPTR8 =
                                        ::core::ptr::null::<PCRE2_UCHAR8>();
                                    if codevalue == OP_XCLASS as ::core::ffi::c_int as uint32_t {
                                        ecode = code.offset(
                                            ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 8 as ::core::ffi::c_int
                                                | *code.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint
                                                as isize,
                                        );
                                        if clen > 0 as ::core::ffi::c_int {
                                            isinclass = _pcre2_xclass_8(
                                                c,
                                                code.offset(1 as ::core::ffi::c_int as isize)
                                                    .offset(LINK_SIZE as isize),
                                                (*mb).start_code as *const uint8_t,
                                                utf,
                                            );
                                        }
                                    } else if codevalue
                                        == OP_ECLASS as ::core::ffi::c_int as uint32_t
                                    {
                                        ecode = code.offset(
                                            ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 8 as ::core::ffi::c_int
                                                | *code.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint
                                                as isize,
                                        );
                                        if clen > 0 as ::core::ffi::c_int {
                                            isinclass = _pcre2_eclass_8(
                                                c,
                                                code.offset(1 as ::core::ffi::c_int as isize)
                                                    .offset(LINK_SIZE as isize),
                                                ecode,
                                                (*mb).start_code as *const uint8_t,
                                                utf,
                                            );
                                        }
                                    } else {
                                        ecode = code
                                            .offset(1 as ::core::ffi::c_int as isize)
                                            .offset((32 as usize).wrapping_div(
                                                ::core::mem::size_of::<PCRE2_UCHAR8>() as usize,
                                            )
                                                as isize);
                                        if clen > 0 as ::core::ffi::c_int {
                                            isinclass = (if c > 255 as uint32_t {
                                                (codevalue
                                                    == OP_NCLASS as ::core::ffi::c_int as uint32_t)
                                                    as ::core::ffi::c_int
                                            } else {
                                                (*(code.offset(1 as ::core::ffi::c_int as isize)
                                                    as *const uint8_t)
                                                    .offset(c.wrapping_div(8 as uint32_t) as isize)
                                                    as ::core::ffi::c_uint
                                                    & (1 as ::core::ffi::c_uint)
                                                        << (c & 7 as uint32_t)
                                                    != 0 as ::core::ffi::c_uint)
                                                    as ::core::ffi::c_int
                                            })
                                                as BOOL;
                                        }
                                    }
                                    next_state_offset = ecode.offset_from(start_code)
                                        as isize
                                        as ::core::ffi::c_int;
                                    match *ecode as ::core::ffi::c_int {
                                        98 | 99 | 106 => {
                                            let fresh107 = active_count;
                                            active_count = active_count + 1;
                                            if fresh107 < wscount {
                                                (*next_active_state).offset =
                                                    next_state_offset + 1 as ::core::ffi::c_int;
                                                (*next_active_state).count =
                                                    0 as ::core::ffi::c_int;
                                                next_active_state = next_active_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                            if isinclass != 0 {
                                                if *ecode as ::core::ffi::c_int
                                                    == OP_CRPOSSTAR as ::core::ffi::c_int
                                                {
                                                    active_count -= 1;
                                                    next_active_state =
                                                        next_active_state.offset(-1);
                                                }
                                                let fresh108 = new_count;
                                                new_count = new_count + 1;
                                                if fresh108 < wscount {
                                                    (*next_new_state).offset = state_offset;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                        100 | 101 | 107 => {
                                            count = (*current_state).count;
                                            if count > 0 as ::core::ffi::c_int {
                                                let fresh109 = active_count;
                                                active_count = active_count + 1;
                                                if fresh109 < wscount {
                                                    (*next_active_state).offset =
                                                        next_state_offset + 1 as ::core::ffi::c_int;
                                                    (*next_active_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_active_state = next_active_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                            if isinclass != 0 {
                                                if count > 0 as ::core::ffi::c_int
                                                    && *ecode as ::core::ffi::c_int
                                                        == OP_CRPOSPLUS as ::core::ffi::c_int
                                                {
                                                    active_count -= 1;
                                                    next_active_state =
                                                        next_active_state.offset(-1);
                                                }
                                                count += 1;
                                                let fresh110 = new_count;
                                                new_count = new_count + 1;
                                                if fresh110 < wscount {
                                                    (*next_new_state).offset = state_offset;
                                                    (*next_new_state).count = count;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                        102 | 103 | 108 => {
                                            let fresh111 = active_count;
                                            active_count = active_count + 1;
                                            if fresh111 < wscount {
                                                (*next_active_state).offset =
                                                    next_state_offset + 1 as ::core::ffi::c_int;
                                                (*next_active_state).count =
                                                    0 as ::core::ffi::c_int;
                                                next_active_state = next_active_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                            if isinclass != 0 {
                                                if *ecode as ::core::ffi::c_int
                                                    == OP_CRPOSQUERY as ::core::ffi::c_int
                                                {
                                                    active_count -= 1;
                                                    next_active_state =
                                                        next_active_state.offset(-1);
                                                }
                                                let fresh112 = new_count;
                                                new_count = new_count + 1;
                                                if fresh112 < wscount {
                                                    (*next_new_state).offset =
                                                        next_state_offset + 1 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                        104 | 105 | 109 => {
                                            count = (*current_state).count;
                                            if count
                                                >= ((*ecode.offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *ecode.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as ::core::ffi::c_int
                                            {
                                                let fresh113 = active_count;
                                                active_count = active_count + 1;
                                                if fresh113 < wscount {
                                                    (*next_active_state).offset = next_state_offset
                                                        + 1 as ::core::ffi::c_int
                                                        + 2 as ::core::ffi::c_int
                                                            * 2 as ::core::ffi::c_int;
                                                    (*next_active_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_active_state = next_active_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                            if isinclass != 0 {
                                                let mut max: ::core::ffi::c_int = ((*ecode.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 2 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *ecode.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 2 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as ::core::ffi::c_int;
                                                if *ecode as ::core::ffi::c_int
                                                    == OP_CRPOSRANGE as ::core::ffi::c_int
                                                    && count
                                                        >= ((*ecode.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int)
                                                            << 8 as ::core::ffi::c_int
                                                            | *ecode.offset(
                                                                (1 as ::core::ffi::c_int
                                                                    + 1 as ::core::ffi::c_int)
                                                                    as isize,
                                                            )
                                                                as ::core::ffi::c_int)
                                                            as ::core::ffi::c_uint
                                                            as ::core::ffi::c_int
                                                {
                                                    active_count -= 1;
                                                    next_active_state =
                                                        next_active_state.offset(-1);
                                                }
                                                count += 1;
                                                if count >= max && max != 0 as ::core::ffi::c_int {
                                                    let fresh114 = new_count;
                                                    new_count = new_count + 1;
                                                    if fresh114 < wscount {
                                                        (*next_new_state).offset = next_state_offset
                                                            + 1 as ::core::ffi::c_int
                                                            + 2 as ::core::ffi::c_int
                                                                * 2 as ::core::ffi::c_int;
                                                        (*next_new_state).count =
                                                            0 as ::core::ffi::c_int;
                                                        next_new_state = next_new_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                } else {
                                                    let fresh115 = new_count;
                                                    new_count = new_count + 1;
                                                    if fresh115 < wscount {
                                                        (*next_new_state).offset = state_offset;
                                                        (*next_new_state).count = count;
                                                        next_new_state = next_new_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            if isinclass != 0 {
                                                let fresh116 = new_count;
                                                new_count = new_count + 1;
                                                if fresh116 < wscount {
                                                    (*next_new_state).offset = next_state_offset;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                165 => {
                                    current_block_1814 = 14107599004487389265;
                                }
                                128 | 129 | 130 | 131 => {
                                    let mut rc: ::core::ffi::c_int = 0;
                                    let mut local_workspace: *mut ::core::ffi::c_int =
                                        ::core::ptr::null_mut::<::core::ffi::c_int>();
                                    let mut local_offsets: *mut size_t =
                                        ::core::ptr::null_mut::<size_t>();
                                    let mut endasscode: PCRE2_SPTR8 = code.offset(
                                        ((*code.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int)
                                            << 8 as ::core::ffi::c_int
                                            | *code.offset(
                                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                    as isize,
                                            )
                                                as ::core::ffi::c_int)
                                            as ::core::ffi::c_uint
                                            as isize,
                                    );
                                    let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;
                                    if ((*rws).free as usize)
                                        < (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_OSIZE)
                                    {
                                        rc = more_workspace(
                                            &raw mut rws,
                                            RWS_OVEC_OSIZE as ::core::ffi::c_uint,
                                            mb,
                                        );
                                        if rc != 0 as ::core::ffi::c_int {
                                            return rc;
                                        }
                                        RWS = rws as *mut ::core::ffi::c_int;
                                    }
                                    local_offsets = RWS
                                        .offset((*rws).size as isize)
                                        .offset(-((*rws).free as isize))
                                        as *mut size_t;
                                    local_workspace = (local_offsets as *mut ::core::ffi::c_int)
                                        .offset(RWS_OVEC_OSIZE as isize);
                                    (*rws).free =
                                        ((*rws).free as usize).wrapping_sub(
                                            (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_OSIZE)
                                                as usize,
                                        ) as uint32_t
                                            as uint32_t;
                                    while *endasscode as ::core::ffi::c_int
                                        == OP_ALT as ::core::ffi::c_int
                                    {
                                        endasscode = endasscode.offset(
                                            ((*endasscode.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int)
                                                << 8 as ::core::ffi::c_int
                                                | *endasscode.offset(
                                                    (1 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint
                                                as isize,
                                        );
                                    }
                                    rc = internal_dfa_match(
                                        mb,
                                        code,
                                        ptr,
                                        ptr.offset_from(start_subject) as isize
                                            as size_t,
                                        local_offsets,
                                        RWS_OVEC_OSIZE.wrapping_div(OVEC_UNIT) as uint32_t,
                                        local_workspace,
                                        RWS_RSIZE,
                                        rlevel,
                                        RWS,
                                    );
                                    (*rws).free =
                                        ((*rws).free as usize).wrapping_add(
                                            (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_OSIZE)
                                                as usize,
                                        ) as uint32_t
                                            as uint32_t;
                                    if rc < 0 as ::core::ffi::c_int && rc != PCRE2_ERROR_NOMATCH {
                                        return rc;
                                    }
                                    if (rc >= 0 as ::core::ffi::c_int) as ::core::ffi::c_int
                                        == (codevalue
                                            == OP_ASSERT as ::core::ffi::c_int as uint32_t
                                            || codevalue
                                                == OP_ASSERTBACK as ::core::ffi::c_int as uint32_t)
                                            as ::core::ffi::c_int
                                    {
                                        let fresh117 = active_count;
                                        active_count = active_count + 1;
                                        if fresh117 < wscount {
                                            (*next_active_state).offset = endasscode
                                                .offset(2 as ::core::ffi::c_int as isize)
                                                .offset(1 as ::core::ffi::c_int as isize)
                                                .offset_from(start_code)
                                                as isize
                                                as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                141 | 146 => {
                                    let mut codelink: ::core::ffi::c_int = ((*code
                                        .offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int)
                                        << 8 as ::core::ffi::c_int
                                        | *code.offset(
                                            (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                as isize,
                                        )
                                            as ::core::ffi::c_int)
                                        as ::core::ffi::c_uint
                                        as ::core::ffi::c_int;
                                    let mut condcode: PCRE2_UCHAR8 = 0;
                                    if *code.offset((LINK_SIZE + 1 as ::core::ffi::c_int) as isize)
                                        as ::core::ffi::c_int
                                        == OP_CALLOUT as ::core::ffi::c_int
                                        || *code
                                            .offset((LINK_SIZE + 1 as ::core::ffi::c_int) as isize)
                                            as ::core::ffi::c_int
                                            == OP_CALLOUT_STR as ::core::ffi::c_int
                                    {
                                        let mut callout_length: size_t = 0;
                                        rrc = do_callout_dfa(
                                            code,
                                            offsets,
                                            current_subject,
                                            ptr,
                                            mb,
                                            (1 as ::core::ffi::c_int + LINK_SIZE) as size_t,
                                            &raw mut callout_length,
                                        );
                                        if rrc < 0 as ::core::ffi::c_int {
                                            return rrc;
                                        }
                                        if rrc > 0 as ::core::ffi::c_int {
                                            current_block_1814 = 14107599004487389265;
                                        } else {
                                            code = code.offset(callout_length as isize);
                                            current_block_1814 = 17884754765720769523;
                                        }
                                    } else {
                                        current_block_1814 = 17884754765720769523;
                                    }
                                    match current_block_1814 {
                                        14107599004487389265 => {}
                                        _ => {
                                            condcode = *code.offset(
                                                (LINK_SIZE + 1 as ::core::ffi::c_int) as isize,
                                            );
                                            if condcode as ::core::ffi::c_int
                                                == OP_CREF as ::core::ffi::c_int
                                                || condcode as ::core::ffi::c_int
                                                    == OP_DNCREF as ::core::ffi::c_int
                                                || condcode as ::core::ffi::c_int
                                                    == OP_DNRREF as ::core::ffi::c_int
                                            {
                                                return PCRE2_ERROR_DFA_UCOND;
                                            }
                                            if condcode as ::core::ffi::c_int
                                                == OP_FALSE as ::core::ffi::c_int
                                                || condcode as ::core::ffi::c_int
                                                    == OP_FAIL as ::core::ffi::c_int
                                            {
                                                let fresh118 = active_count;
                                                active_count = active_count + 1;
                                                if fresh118 < wscount {
                                                    (*next_active_state).offset = state_offset
                                                        + codelink
                                                        + 2 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int;
                                                    (*next_active_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_active_state = next_active_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else if condcode as ::core::ffi::c_int
                                                == OP_TRUE as ::core::ffi::c_int
                                            {
                                                let fresh119 = active_count;
                                                active_count = active_count + 1;
                                                if fresh119 < wscount {
                                                    (*next_active_state).offset = state_offset
                                                        + 2 as ::core::ffi::c_int
                                                        + 2 as ::core::ffi::c_int;
                                                    (*next_active_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_active_state = next_active_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else if condcode as ::core::ffi::c_int
                                                == OP_RREF as ::core::ffi::c_int
                                            {
                                                let mut value: ::core::ffi::c_uint = ((*code.offset(
                                                    (2 as ::core::ffi::c_int
                                                        + 2 as ::core::ffi::c_int)
                                                        as isize,
                                                )
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *code.offset(
                                                        (2 as ::core::ffi::c_int
                                                            + 2 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint;
                                                if value != RREF_ANY as ::core::ffi::c_uint {
                                                    return PCRE2_ERROR_DFA_UCOND;
                                                }
                                                if !(*mb).recursive.is_null() {
                                                    let fresh120 = active_count;
                                                    active_count = active_count + 1;
                                                    if fresh120 < wscount {
                                                        (*next_active_state).offset = state_offset
                                                            + 2 as ::core::ffi::c_int
                                                            + 2 as ::core::ffi::c_int
                                                            + 2 as ::core::ffi::c_int;
                                                        (*next_active_state).count =
                                                            0 as ::core::ffi::c_int;
                                                        next_active_state =
                                                            next_active_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                } else {
                                                    let fresh121 = active_count;
                                                    active_count = active_count + 1;
                                                    if fresh121 < wscount {
                                                        (*next_active_state).offset = state_offset
                                                            + codelink
                                                            + 2 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int;
                                                        (*next_active_state).count =
                                                            0 as ::core::ffi::c_int;
                                                        next_active_state =
                                                            next_active_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                }
                                            } else {
                                                let mut rc_0: ::core::ffi::c_int = 0;
                                                let mut local_workspace_0: *mut ::core::ffi::c_int =
                                                    ::core::ptr::null_mut::<::core::ffi::c_int>();
                                                let mut local_offsets_0: *mut size_t =
                                                    ::core::ptr::null_mut::<size_t>();
                                                let mut asscode: PCRE2_SPTR8 = code
                                                    .offset(LINK_SIZE as isize)
                                                    .offset(1 as ::core::ffi::c_int as isize);
                                                let mut endasscode_0: PCRE2_SPTR8 = asscode.offset(
                                                    ((*asscode
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int)
                                                        << 8 as ::core::ffi::c_int
                                                        | *asscode.offset(
                                                            (1 as ::core::ffi::c_int
                                                                + 1 as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                            as ::core::ffi::c_int)
                                                        as ::core::ffi::c_uint
                                                        as isize,
                                                );
                                                let mut rws_0: *mut RWS_anchor =
                                                    RWS as *mut RWS_anchor;
                                                if ((*rws_0).free as usize)
                                                    < (RWS_RSIZE as usize)
                                                        .wrapping_add(RWS_OVEC_OSIZE)
                                                {
                                                    rc_0 = more_workspace(
                                                        &raw mut rws_0,
                                                        RWS_OVEC_OSIZE as ::core::ffi::c_uint,
                                                        mb,
                                                    );
                                                    if rc_0 != 0 as ::core::ffi::c_int {
                                                        return rc_0;
                                                    }
                                                    RWS = rws_0 as *mut ::core::ffi::c_int;
                                                }
                                                local_offsets_0 = RWS
                                                    .offset((*rws_0).size as isize)
                                                    .offset(-((*rws_0).free as isize))
                                                    as *mut size_t;
                                                local_workspace_0 = (local_offsets_0
                                                    as *mut ::core::ffi::c_int)
                                                    .offset(RWS_OVEC_OSIZE as isize);
                                                (*rws_0).free = ((*rws_0).free
                                                    as usize)
                                                    .wrapping_sub(
                                                        (RWS_RSIZE as usize)
                                                            .wrapping_add(RWS_OVEC_OSIZE)
                                                            as usize,
                                                    )
                                                    as uint32_t
                                                    as uint32_t;
                                                while *endasscode_0 as ::core::ffi::c_int
                                                    == OP_ALT as ::core::ffi::c_int
                                                {
                                                    endasscode_0 = endasscode_0.offset(
                                                        ((*endasscode_0.offset(
                                                            1 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int)
                                                            << 8 as ::core::ffi::c_int
                                                            | *endasscode_0.offset(
                                                                (1 as ::core::ffi::c_int
                                                                    + 1 as ::core::ffi::c_int)
                                                                    as isize,
                                                            )
                                                                as ::core::ffi::c_int)
                                                            as ::core::ffi::c_uint
                                                            as isize,
                                                    );
                                                }
                                                rc_0 = internal_dfa_match(
                                                    mb,
                                                    asscode,
                                                    ptr,
                                                    ptr.offset_from(start_subject)
                                                        as isize
                                                        as size_t,
                                                    local_offsets_0,
                                                    RWS_OVEC_OSIZE.wrapping_div(OVEC_UNIT)
                                                        as uint32_t,
                                                    local_workspace_0,
                                                    RWS_RSIZE,
                                                    rlevel,
                                                    RWS,
                                                );
                                                (*rws_0).free = ((*rws_0).free
                                                    as usize)
                                                    .wrapping_add(
                                                        (RWS_RSIZE as usize)
                                                            .wrapping_add(RWS_OVEC_OSIZE)
                                                            as usize,
                                                    )
                                                    as uint32_t
                                                    as uint32_t;
                                                if rc_0 < 0 as ::core::ffi::c_int
                                                    && rc_0 != PCRE2_ERROR_NOMATCH
                                                {
                                                    return rc_0;
                                                }
                                                if (rc_0 >= 0 as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    == (condcode as ::core::ffi::c_int
                                                        == OP_ASSERT as ::core::ffi::c_int
                                                        || condcode as ::core::ffi::c_int
                                                            == OP_ASSERTBACK as ::core::ffi::c_int)
                                                        as ::core::ffi::c_int
                                                {
                                                    let fresh122 = active_count;
                                                    active_count = active_count + 1;
                                                    if fresh122 < wscount {
                                                        (*next_active_state).offset = endasscode_0
                                                            .offset(
                                                                2 as ::core::ffi::c_int as isize,
                                                            )
                                                            .offset(
                                                                1 as ::core::ffi::c_int as isize,
                                                            )
                                                            .offset_from(start_code)
                                                            as isize
                                                            as ::core::ffi::c_int;
                                                        (*next_active_state).count =
                                                            0 as ::core::ffi::c_int;
                                                        next_active_state =
                                                            next_active_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                } else {
                                                    let fresh123 = active_count;
                                                    active_count = active_count + 1;
                                                    if fresh123 < wscount {
                                                        (*next_active_state).offset = state_offset
                                                            + codelink
                                                            + 2 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int;
                                                        (*next_active_state).count =
                                                            0 as ::core::ffi::c_int;
                                                        next_active_state =
                                                            next_active_state.offset(1);
                                                    } else {
                                                        return PCRE2_ERROR_DFA_WSSIZE;
                                                    }
                                                }
                                            }
                                            current_block_1814 = 14107599004487389265;
                                        }
                                    }
                                }
                                118 => {
                                    let mut rc_1: ::core::ffi::c_int = 0;
                                    let mut local_workspace_1: *mut ::core::ffi::c_int =
                                        ::core::ptr::null_mut::<::core::ffi::c_int>();
                                    let mut local_offsets_1: *mut size_t =
                                        ::core::ptr::null_mut::<size_t>();
                                    let mut rws_1: *mut RWS_anchor = RWS as *mut RWS_anchor;
                                    let mut callpat: PCRE2_SPTR8 = start_code.offset(
                                        ((*code.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int)
                                            << 8 as ::core::ffi::c_int
                                            | *code.offset(
                                                (1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                                                    as isize,
                                            )
                                                as ::core::ffi::c_int)
                                            as ::core::ffi::c_uint
                                            as isize,
                                    );
                                    let mut recno: uint32_t = if callpat == (*mb).start_code {
                                        0 as uint32_t
                                    } else {
                                        ((*callpat.offset(
                                            (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int)
                                                as isize,
                                        )
                                            as ::core::ffi::c_int)
                                            << 8 as ::core::ffi::c_int
                                            | *callpat.offset(
                                                (1 as ::core::ffi::c_int
                                                    + 2 as ::core::ffi::c_int
                                                    + 1 as ::core::ffi::c_int)
                                                    as isize,
                                            )
                                                as ::core::ffi::c_int)
                                            as uint32_t
                                    };
                                    if *code.offset((1 as ::core::ffi::c_int + LINK_SIZE) as isize)
                                        as ::core::ffi::c_int
                                        == OP_CREF as ::core::ffi::c_int
                                    {
                                        return PCRE2_ERROR_DFA_UITEM;
                                    }
                                    if ((*rws_1).free as usize)
                                        < (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_RSIZE)
                                    {
                                        rc_1 = more_workspace(
                                            &raw mut rws_1,
                                            RWS_OVEC_RSIZE as ::core::ffi::c_uint,
                                            mb,
                                        );
                                        if rc_1 != 0 as ::core::ffi::c_int {
                                            return rc_1;
                                        }
                                        RWS = rws_1 as *mut ::core::ffi::c_int;
                                    }
                                    local_offsets_1 = RWS
                                        .offset((*rws_1).size as isize)
                                        .offset(-((*rws_1).free as isize))
                                        as *mut size_t;
                                    local_workspace_1 = (local_offsets_1
                                        as *mut ::core::ffi::c_int)
                                        .offset(RWS_OVEC_RSIZE as isize);
                                    (*rws_1).free =
                                        ((*rws_1).free as usize).wrapping_sub(
                                            (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_RSIZE)
                                                as usize,
                                        ) as uint32_t
                                            as uint32_t;
                                    let mut ri: *mut dfa_recursion_info = (*mb).recursive;
                                    while !ri.is_null() {
                                        if recno == (*ri).group_num
                                            && ptr == (*ri).subject_position
                                            && (*mb).last_used_ptr == (*ri).last_used_ptr
                                        {
                                            return PCRE2_ERROR_RECURSELOOP;
                                        }
                                        ri = (*ri).prevrec as *mut dfa_recursion_info;
                                    }
                                    new_recursive.group_num = recno;
                                    new_recursive.subject_position = ptr;
                                    new_recursive.last_used_ptr = (*mb).last_used_ptr;
                                    new_recursive.prevrec =
                                        (*mb).recursive as *mut dfa_recursion_info;
                                    (*mb).recursive = &raw mut new_recursive;
                                    rc_1 = internal_dfa_match(
                                        mb,
                                        callpat,
                                        ptr,
                                        ptr.offset_from(start_subject) as isize
                                            as size_t,
                                        local_offsets_1,
                                        RWS_OVEC_RSIZE.wrapping_div(OVEC_UNIT) as uint32_t,
                                        local_workspace_1,
                                        RWS_RSIZE,
                                        rlevel,
                                        RWS,
                                    );
                                    (*rws_1).free =
                                        ((*rws_1).free as usize).wrapping_add(
                                            (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_RSIZE)
                                                as usize,
                                        ) as uint32_t
                                            as uint32_t;
                                    (*mb).recursive =
                                        new_recursive.prevrec as *mut dfa_recursion_info;
                                    if rc_1 == 0 as ::core::ffi::c_int {
                                        return PCRE2_ERROR_DFA_RECURSE;
                                    }
                                    if rc_1 > 0 as ::core::ffi::c_int {
                                        rc_1 = rc_1 * 2 as ::core::ffi::c_int
                                            - 2 as ::core::ffi::c_int;
                                        while rc_1 >= 0 as ::core::ffi::c_int {
                                            let mut charcount: size_t = (*local_offsets_1
                                                .offset((rc_1 + 1 as ::core::ffi::c_int) as isize))
                                            .wrapping_sub(*local_offsets_1.offset(rc_1 as isize));
                                            if utf != 0 {
                                                let mut p: PCRE2_SPTR8 = start_subject
                                                    .offset(*local_offsets_1.offset(rc_1 as isize)
                                                        as isize);
                                                let mut pp: PCRE2_SPTR8 =
                                                    start_subject.offset(*local_offsets_1.offset(
                                                        (rc_1 + 1 as ::core::ffi::c_int) as isize,
                                                    )
                                                        as isize);
                                                while p < pp {
                                                    let fresh124 = p;
                                                    p = p.offset(1);
                                                    if *fresh124 as ::core::ffi::c_uint
                                                        & 0xc0 as ::core::ffi::c_uint
                                                        == 0x80 as ::core::ffi::c_uint
                                                    {
                                                        charcount = charcount.wrapping_sub(1);
                                                    }
                                                }
                                            }
                                            if charcount > 0 as size_t {
                                                let fresh125 = new_count;
                                                new_count = new_count + 1;
                                                if fresh125 < wscount {
                                                    (*next_new_state).offset = -(state_offset
                                                        + 2 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int);
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    (*next_new_state).data = charcount
                                                        .wrapping_sub(1 as size_t)
                                                        as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else {
                                                let fresh126 = active_count;
                                                active_count = active_count + 1;
                                                if fresh126 < wscount {
                                                    (*next_active_state).offset = state_offset
                                                        + 2 as ::core::ffi::c_int
                                                        + 1 as ::core::ffi::c_int;
                                                    (*next_active_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_active_state = next_active_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                            rc_1 -= 2 as ::core::ffi::c_int;
                                        }
                                    } else if rc_1 != PCRE2_ERROR_NOMATCH {
                                        return rc_1;
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                138 | 143 | 140 | 145 | 155 => {
                                    let mut rc_2: ::core::ffi::c_int = 0;
                                    let mut local_workspace_2: *mut ::core::ffi::c_int =
                                        ::core::ptr::null_mut::<::core::ffi::c_int>();
                                    let mut local_offsets_2: *mut size_t =
                                        ::core::ptr::null_mut::<size_t>();
                                    let mut charcount_0: size_t = 0;
                                    let mut matched_count: size_t = 0;
                                    let mut local_ptr: PCRE2_SPTR8 = ptr;
                                    let mut rws_2: *mut RWS_anchor = RWS as *mut RWS_anchor;
                                    let mut allow_zero: BOOL = 0;
                                    if ((*rws_2).free as usize)
                                        < (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_OSIZE)
                                    {
                                        rc_2 = more_workspace(
                                            &raw mut rws_2,
                                            RWS_OVEC_OSIZE as ::core::ffi::c_uint,
                                            mb,
                                        );
                                        if rc_2 != 0 as ::core::ffi::c_int {
                                            return rc_2;
                                        }
                                        RWS = rws_2 as *mut ::core::ffi::c_int;
                                    }
                                    local_offsets_2 = RWS
                                        .offset((*rws_2).size as isize)
                                        .offset(-((*rws_2).free as isize))
                                        as *mut size_t;
                                    local_workspace_2 = (local_offsets_2
                                        as *mut ::core::ffi::c_int)
                                        .offset(RWS_OVEC_OSIZE as isize);
                                    (*rws_2).free =
                                        ((*rws_2).free as usize).wrapping_sub(
                                            (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_OSIZE)
                                                as usize,
                                        ) as uint32_t
                                            as uint32_t;
                                    if codevalue == OP_BRAPOSZERO as ::core::ffi::c_int as uint32_t
                                    {
                                        allow_zero = TRUE as BOOL;
                                        code = code.offset(1);
                                    } else {
                                        allow_zero = FALSE as BOOL;
                                    }
                                    matched_count = 0 as size_t;
                                    loop {
                                        rc_2 = internal_dfa_match(
                                            mb,
                                            code,
                                            local_ptr,
                                            ptr.offset_from(start_subject) as isize
                                                as size_t,
                                            local_offsets_2,
                                            RWS_OVEC_OSIZE.wrapping_div(OVEC_UNIT) as uint32_t,
                                            local_workspace_2,
                                            RWS_RSIZE,
                                            rlevel,
                                            RWS,
                                        );
                                        if rc_2 < 0 as ::core::ffi::c_int {
                                            if rc_2 != PCRE2_ERROR_NOMATCH {
                                                return rc_2;
                                            }
                                            break;
                                        } else {
                                            charcount_0 = (*local_offsets_2
                                                .offset(1 as ::core::ffi::c_int as isize))
                                            .wrapping_sub(
                                                *local_offsets_2
                                                    .offset(0 as ::core::ffi::c_int as isize),
                                            );
                                            if charcount_0 == 0 as size_t {
                                                break;
                                            }
                                            local_ptr = local_ptr.offset(charcount_0 as isize);
                                            matched_count = matched_count.wrapping_add(1);
                                        }
                                    }
                                    (*rws_2).free =
                                        ((*rws_2).free as usize).wrapping_add(
                                            (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_OSIZE)
                                                as usize,
                                        ) as uint32_t
                                            as uint32_t;
                                    if matched_count > 0 as size_t || allow_zero != 0 {
                                        let mut end_subpattern: PCRE2_SPTR8 = code;
                                        let mut next_state_offset_0: ::core::ffi::c_int = 0;
                                        loop {
                                            end_subpattern = end_subpattern.offset(
                                                ((*end_subpattern
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *end_subpattern.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as isize,
                                            );
                                            if !(*end_subpattern as ::core::ffi::c_int
                                                == OP_ALT as ::core::ffi::c_int)
                                            {
                                                break;
                                            }
                                        }
                                        next_state_offset_0 = (end_subpattern
                                            .offset_from(start_code)
                                            as isize
                                            + LINK_SIZE as isize
                                            + 1 as isize)
                                            as ::core::ffi::c_int;
                                        if i + 1 as ::core::ffi::c_int >= active_count
                                            && new_count == 0 as ::core::ffi::c_int
                                        {
                                            ptr = local_ptr;
                                            clen = 0 as ::core::ffi::c_int;
                                            let fresh127 = new_count;
                                            new_count = new_count + 1;
                                            if fresh127 < wscount {
                                                (*next_new_state).offset = next_state_offset_0;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        } else {
                                            let mut p_0: PCRE2_SPTR8 = ptr;
                                            let mut pp_0: PCRE2_SPTR8 = local_ptr;
                                            charcount_0 = pp_0.offset_from(p_0)
                                                as isize
                                                as size_t;
                                            if utf != 0 {
                                                while p_0 < pp_0 {
                                                    let fresh128 = p_0;
                                                    p_0 = p_0.offset(1);
                                                    if *fresh128 as ::core::ffi::c_uint
                                                        & 0xc0 as ::core::ffi::c_uint
                                                        == 0x80 as ::core::ffi::c_uint
                                                    {
                                                        charcount_0 = charcount_0.wrapping_sub(1);
                                                    }
                                                }
                                            }
                                            let fresh129 = new_count;
                                            new_count = new_count + 1;
                                            if fresh129 < wscount {
                                                (*next_new_state).offset = -next_state_offset_0;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                (*next_new_state).data = charcount_0
                                                    .wrapping_sub(1 as size_t)
                                                    as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                135 => {
                                    let mut rc_3: ::core::ffi::c_int = 0;
                                    let mut local_workspace_3: *mut ::core::ffi::c_int =
                                        ::core::ptr::null_mut::<::core::ffi::c_int>();
                                    let mut local_offsets_3: *mut size_t =
                                        ::core::ptr::null_mut::<size_t>();
                                    let mut rws_3: *mut RWS_anchor = RWS as *mut RWS_anchor;
                                    if ((*rws_3).free as usize)
                                        < (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_OSIZE)
                                    {
                                        rc_3 = more_workspace(
                                            &raw mut rws_3,
                                            RWS_OVEC_OSIZE as ::core::ffi::c_uint,
                                            mb,
                                        );
                                        if rc_3 != 0 as ::core::ffi::c_int {
                                            return rc_3;
                                        }
                                        RWS = rws_3 as *mut ::core::ffi::c_int;
                                    }
                                    local_offsets_3 = RWS
                                        .offset((*rws_3).size as isize)
                                        .offset(-((*rws_3).free as isize))
                                        as *mut size_t;
                                    local_workspace_3 = (local_offsets_3
                                        as *mut ::core::ffi::c_int)
                                        .offset(RWS_OVEC_OSIZE as isize);
                                    (*rws_3).free =
                                        ((*rws_3).free as usize).wrapping_sub(
                                            (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_OSIZE)
                                                as usize,
                                        ) as uint32_t
                                            as uint32_t;
                                    rc_3 = internal_dfa_match(
                                        mb,
                                        code,
                                        ptr,
                                        ptr.offset_from(start_subject) as isize
                                            as size_t,
                                        local_offsets_3,
                                        RWS_OVEC_OSIZE.wrapping_div(OVEC_UNIT) as uint32_t,
                                        local_workspace_3,
                                        RWS_RSIZE,
                                        rlevel,
                                        RWS,
                                    );
                                    (*rws_3).free =
                                        ((*rws_3).free as usize).wrapping_add(
                                            (RWS_RSIZE as usize).wrapping_add(RWS_OVEC_OSIZE)
                                                as usize,
                                        ) as uint32_t
                                            as uint32_t;
                                    if rc_3 >= 0 as ::core::ffi::c_int {
                                        let mut end_subpattern_0: PCRE2_SPTR8 = code;
                                        let mut charcount_1: size_t = (*local_offsets_3
                                            .offset(1 as ::core::ffi::c_int as isize))
                                        .wrapping_sub(
                                            *local_offsets_3
                                                .offset(0 as ::core::ffi::c_int as isize),
                                        );
                                        let mut next_state_offset_1: ::core::ffi::c_int = 0;
                                        let mut repeat_state_offset: ::core::ffi::c_int = 0;
                                        loop {
                                            end_subpattern_0 = end_subpattern_0.offset(
                                                ((*end_subpattern_0
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *end_subpattern_0.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as isize,
                                            );
                                            if !(*end_subpattern_0 as ::core::ffi::c_int
                                                == OP_ALT as ::core::ffi::c_int)
                                            {
                                                break;
                                            }
                                        }
                                        next_state_offset_1 = (end_subpattern_0
                                            .offset_from(start_code)
                                            as isize
                                            + LINK_SIZE as isize
                                            + 1 as isize)
                                            as ::core::ffi::c_int;
                                        repeat_state_offset = if *end_subpattern_0
                                            as ::core::ffi::c_int
                                            == OP_KETRMAX as ::core::ffi::c_int
                                            || *end_subpattern_0 as ::core::ffi::c_int
                                                == OP_KETRMIN as ::core::ffi::c_int
                                        {
                                            (end_subpattern_0.offset_from(start_code)
                                                as isize
                                                - ((*end_subpattern_0
                                                    .offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *end_subpattern_0.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as isize)
                                                as ::core::ffi::c_int
                                        } else {
                                            -(1 as ::core::ffi::c_int)
                                        };
                                        if charcount_1 == 0 as size_t {
                                            let fresh130 = active_count;
                                            active_count = active_count + 1;
                                            if fresh130 < wscount {
                                                (*next_active_state).offset = next_state_offset_1;
                                                (*next_active_state).count =
                                                    0 as ::core::ffi::c_int;
                                                next_active_state = next_active_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        } else if i + 1 as ::core::ffi::c_int >= active_count
                                            && new_count == 0 as ::core::ffi::c_int
                                        {
                                            ptr = ptr.offset(charcount_1 as isize);
                                            clen = 0 as ::core::ffi::c_int;
                                            let fresh131 = new_count;
                                            new_count = new_count + 1;
                                            if fresh131 < wscount {
                                                (*next_new_state).offset = next_state_offset_1;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                            if repeat_state_offset >= 0 as ::core::ffi::c_int {
                                                next_active_state = active_states;
                                                active_count = 0 as ::core::ffi::c_int;
                                                i = -(1 as ::core::ffi::c_int);
                                                let fresh132 = active_count;
                                                active_count = active_count + 1;
                                                if fresh132 < wscount {
                                                    (*next_active_state).offset =
                                                        repeat_state_offset;
                                                    (*next_active_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_active_state = next_active_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        } else {
                                            if utf != 0 {
                                                let mut p_1: PCRE2_SPTR8 = start_subject.offset(
                                                    *local_offsets_3
                                                        .offset(0 as ::core::ffi::c_int as isize)
                                                        as isize,
                                                );
                                                let mut pp_1: PCRE2_SPTR8 = start_subject.offset(
                                                    *local_offsets_3
                                                        .offset(1 as ::core::ffi::c_int as isize)
                                                        as isize,
                                                );
                                                while p_1 < pp_1 {
                                                    let fresh133 = p_1;
                                                    p_1 = p_1.offset(1);
                                                    if *fresh133 as ::core::ffi::c_uint
                                                        & 0xc0 as ::core::ffi::c_uint
                                                        == 0x80 as ::core::ffi::c_uint
                                                    {
                                                        charcount_1 = charcount_1.wrapping_sub(1);
                                                    }
                                                }
                                            }
                                            let fresh134 = new_count;
                                            new_count = new_count + 1;
                                            if fresh134 < wscount {
                                                (*next_new_state).offset = -next_state_offset_1;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                (*next_new_state).data = charcount_1
                                                    .wrapping_sub(1 as size_t)
                                                    as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                            if repeat_state_offset >= 0 as ::core::ffi::c_int {
                                                let fresh135 = new_count;
                                                new_count = new_count + 1;
                                                if fresh135 < wscount {
                                                    (*next_new_state).offset = -repeat_state_offset;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    (*next_new_state).data = charcount_1
                                                        .wrapping_sub(1 as size_t)
                                                        as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                    } else if rc_3 != PCRE2_ERROR_NOMATCH {
                                        return rc_3;
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                119 | 120 => {
                                    let mut callout_length_0: size_t = 0;
                                    rrc = do_callout_dfa(
                                        code,
                                        offsets,
                                        current_subject,
                                        ptr,
                                        mb,
                                        0 as size_t,
                                        &raw mut callout_length_0,
                                    );
                                    if rrc < 0 as ::core::ffi::c_int {
                                        return rrc;
                                    }
                                    if rrc == 0 as ::core::ffi::c_int {
                                        let fresh136 = active_count;
                                        active_count = active_count + 1;
                                        if fresh136 < wscount {
                                            (*next_active_state).offset = state_offset
                                                + callout_length_0 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    current_block_1814 = 14107599004487389265;
                                }
                                _ => return PCRE2_ERROR_DFA_UITEM,
                            }
                            match current_block_1814 {
                                4611645196188390243 => {
                                    let fresh104 = active_count;
                                    active_count = active_count + 1;
                                    if fresh104 < wscount {
                                        (*next_active_state).offset = state_offset
                                            + dlen
                                            + 1 as ::core::ffi::c_int
                                            + 2 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    count = (*current_state).count;
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut otherd_4: uint32_t = NOTACHAR as uint32_t;
                                        if caseless != 0 {
                                            if utf_or_ucp != 0 && d >= 128 as uint32_t {
                                                otherd_4 = (d as ::core::ffi::c_int
                                                    + (*(&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                        .offset((d as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                                                                        as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + d as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .other_case as ::core::ffi::c_int) as uint32_t;
                                            } else {
                                                otherd_4 = *fcc.offset(d as isize) as uint32_t;
                                            }
                                        }
                                        if (c == d || c == otherd_4) as ::core::ffi::c_int
                                            == (codevalue
                                                < OP_NOTSTAR as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == OP_POSUPTO as ::core::ffi::c_int as uint32_t
                                                || codevalue
                                                    == OP_NOTPOSUPTO as ::core::ffi::c_int
                                                        as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            count += 1;
                                            if count
                                                >= ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as ::core::ffi::c_int
                                            {
                                                let fresh105 = new_count;
                                                new_count = new_count + 1;
                                                if fresh105 < wscount {
                                                    (*next_new_state).offset = state_offset
                                                        + dlen
                                                        + 1 as ::core::ffi::c_int
                                                        + 2 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else {
                                                let fresh106 = new_count;
                                                new_count = new_count + 1;
                                                if fresh106 < wscount {
                                                    (*next_new_state).offset = state_offset;
                                                    (*next_new_state).count = count;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                    }
                                }
                                12606961696430611236 => {
                                    count = (*current_state).count;
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut otherd_3: uint32_t = NOTACHAR as uint32_t;
                                        if caseless != 0 {
                                            if utf_or_ucp != 0 && d >= 128 as uint32_t {
                                                otherd_3 = (d as ::core::ffi::c_int
                                                    + (*(&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                        .offset((d as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                                                                        as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + d as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .other_case as ::core::ffi::c_int) as uint32_t;
                                            } else {
                                                otherd_3 = *fcc.offset(d as isize) as uint32_t;
                                            }
                                        }
                                        if (c == d || c == otherd_3) as ::core::ffi::c_int
                                            == (codevalue
                                                < OP_NOTSTAR as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            count += 1;
                                            if count
                                                >= ((*code.offset(1 as ::core::ffi::c_int as isize)
                                                    as ::core::ffi::c_int)
                                                    << 8 as ::core::ffi::c_int
                                                    | *code.offset(
                                                        (1 as ::core::ffi::c_int
                                                            + 1 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_uint
                                                    as ::core::ffi::c_int
                                            {
                                                let fresh102 = new_count;
                                                new_count = new_count + 1;
                                                if fresh102 < wscount {
                                                    (*next_new_state).offset = state_offset
                                                        + dlen
                                                        + 1 as ::core::ffi::c_int
                                                        + 2 as ::core::ffi::c_int;
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            } else {
                                                let fresh103 = new_count;
                                                new_count = new_count + 1;
                                                if fresh103 < wscount {
                                                    (*next_new_state).offset = state_offset;
                                                    (*next_new_state).count = count;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                        }
                                    }
                                }
                                6529072442217564396 => {
                                    let fresh100 = active_count;
                                    active_count = active_count + 1;
                                    if fresh100 < wscount {
                                        (*next_active_state).offset =
                                            state_offset + dlen + 1 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut otherd_2: uint32_t = NOTACHAR as uint32_t;
                                        if caseless != 0 {
                                            if utf_or_ucp != 0 && d >= 128 as uint32_t {
                                                otherd_2 = (d as ::core::ffi::c_int
                                                    + (*(&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                        .offset((d as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                                                                        as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + d as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .other_case as ::core::ffi::c_int) as uint32_t;
                                            } else {
                                                otherd_2 = *fcc.offset(d as isize) as uint32_t;
                                            }
                                        }
                                        if (c == d || c == otherd_2) as ::core::ffi::c_int
                                            == (codevalue
                                                < OP_NOTSTAR as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == OP_POSSTAR as ::core::ffi::c_int as uint32_t
                                                || codevalue
                                                    == OP_NOTPOSSTAR as ::core::ffi::c_int
                                                        as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            let fresh101 = new_count;
                                            new_count = new_count + 1;
                                            if fresh101 < wscount {
                                                (*next_new_state).offset = state_offset;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                }
                                15321635274243100895 => {
                                    let fresh98 = active_count;
                                    active_count = active_count + 1;
                                    if fresh98 < wscount {
                                        (*next_active_state).offset =
                                            state_offset + dlen + 1 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut otherd_1: uint32_t = NOTACHAR as uint32_t;
                                        if caseless != 0 {
                                            if utf_or_ucp != 0 && d >= 128 as uint32_t {
                                                otherd_1 = (d as ::core::ffi::c_int
                                                    + (*(&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                        .offset((d as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                                                                        as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + d as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .other_case as ::core::ffi::c_int) as uint32_t;
                                            } else {
                                                otherd_1 = *fcc.offset(d as isize) as uint32_t;
                                            }
                                        }
                                        if (c == d || c == otherd_1) as ::core::ffi::c_int
                                            == (codevalue
                                                < OP_NOTSTAR as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == OP_POSQUERY as ::core::ffi::c_int as uint32_t
                                                || codevalue
                                                    == OP_NOTPOSQUERY as ::core::ffi::c_int
                                                        as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            let fresh99 = new_count;
                                            new_count = new_count + 1;
                                            if fresh99 < wscount {
                                                (*next_new_state).offset =
                                                    state_offset + dlen + 1 as ::core::ffi::c_int;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                }
                                12389340984333060227 => {
                                    count = (*current_state).count;
                                    if count > 0 as ::core::ffi::c_int {
                                        let fresh96 = active_count;
                                        active_count = active_count + 1;
                                        if fresh96 < wscount {
                                            (*next_active_state).offset =
                                                state_offset + dlen + 1 as ::core::ffi::c_int;
                                            (*next_active_state).count = 0 as ::core::ffi::c_int;
                                            next_active_state = next_active_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut otherd_0: uint32_t = NOTACHAR as uint32_t;
                                        if caseless != 0 {
                                            if utf_or_ucp != 0 && d >= 128 as uint32_t {
                                                otherd_0 = (d as ::core::ffi::c_int
                                                    + (*(&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                        .offset(
                                                            *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                                .offset(
                                                                    (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                        .offset((d as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                                                                        as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                        + d as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                ) as ::core::ffi::c_int as isize,
                                                        ))
                                                        .other_case as ::core::ffi::c_int) as uint32_t;
                                            } else {
                                                otherd_0 = *fcc.offset(d as isize) as uint32_t;
                                            }
                                        }
                                        if (c == d || c == otherd_0) as ::core::ffi::c_int
                                            == (codevalue
                                                < OP_NOTSTAR as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if count > 0 as ::core::ffi::c_int
                                                && (codevalue
                                                    == OP_POSPLUS as ::core::ffi::c_int as uint32_t
                                                    || codevalue
                                                        == OP_NOTPOSPLUS as ::core::ffi::c_int
                                                            as uint32_t)
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            count += 1;
                                            let fresh97 = new_count;
                                            new_count = new_count + 1;
                                            if fresh97 < wscount {
                                                (*next_new_state).offset = state_offset;
                                                (*next_new_state).count = count;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                }
                                16249953603520743654 => {
                                    let fresh63 = active_count;
                                    active_count = active_count + 1;
                                    if fresh63 < wscount {
                                        (*next_active_state).offset =
                                            state_offset + 2 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut OK_5: BOOL = 0;
                                        match c {
                                            9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                            | 8195 | 8196 | 8197 | 8198 | 8199 | 8200 | 8201
                                            | 8202 | 8239 | 8287 | 12288 => {
                                                OK_5 = TRUE as BOOL;
                                            }
                                            _ => {
                                                OK_5 = FALSE as BOOL;
                                            }
                                        }
                                        if OK_5
                                            == (d == OP_HSPACE as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == (OP_HSPACE_EXTRA
                                                    + OP_TYPEPOSSTAR as ::core::ffi::c_int)
                                                    as uint32_t
                                                || codevalue
                                                    == (OP_HSPACE_EXTRA
                                                        + OP_TYPEPOSQUERY as ::core::ffi::c_int)
                                                        as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            let fresh64 = new_count;
                                            new_count = new_count + 1;
                                            if fresh64 < wscount {
                                                (*next_new_state).offset = -(state_offset + count);
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                (*next_new_state).data = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                }
                                3015753210103841382 => {
                                    let fresh61 = active_count;
                                    active_count = active_count + 1;
                                    if fresh61 < wscount {
                                        (*next_active_state).offset =
                                            state_offset + 2 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut OK_4: BOOL = 0;
                                        match c {
                                            10 | 11 | 12 | 13 | 133 | 8232 | 8233 => {
                                                OK_4 = TRUE as BOOL;
                                            }
                                            _ => {
                                                OK_4 = FALSE as BOOL;
                                            }
                                        }
                                        if OK_4
                                            == (d == OP_VSPACE as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == (OP_VSPACE_EXTRA
                                                    + OP_TYPEPOSSTAR as ::core::ffi::c_int)
                                                    as uint32_t
                                                || codevalue
                                                    == (OP_VSPACE_EXTRA
                                                        + OP_TYPEPOSQUERY as ::core::ffi::c_int)
                                                        as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            let fresh62 = new_count;
                                            new_count = new_count + 1;
                                            if fresh62 < wscount {
                                                (*next_new_state).offset = -(state_offset + count);
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                (*next_new_state).data = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                }
                                4040471979405040113 => {
                                    let fresh59 = active_count;
                                    active_count = active_count + 1;
                                    if fresh59 < wscount {
                                        (*next_active_state).offset =
                                            state_offset + 2 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut ncount_2: ::core::ffi::c_int =
                                            0 as ::core::ffi::c_int;
                                        let mut current_block_857: u64;
                                        match c {
                                            11 | 12 | 133 | 8232 | 8233 => {
                                                if (*mb).bsr_convention as ::core::ffi::c_int
                                                    == PCRE2_BSR_ANYCRLF
                                                {
                                                    current_block_857 = 5714536734526847899;
                                                } else {
                                                    current_block_857 = 304229182246232162;
                                                }
                                            }
                                            13 => {
                                                if ptr.offset(1 as ::core::ffi::c_int as isize)
                                                    < end_subject
                                                    && *ptr.offset(1 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                        == CHAR_LF
                                                {
                                                    ncount_2 = 1 as ::core::ffi::c_int;
                                                }
                                                current_block_857 = 304229182246232162;
                                            }
                                            10 => {
                                                current_block_857 = 304229182246232162;
                                            }
                                            _ => {
                                                current_block_857 = 5714536734526847899;
                                            }
                                        }
                                        match current_block_857 {
                                            304229182246232162 => {
                                                if codevalue
                                                    == (OP_ANYNL_EXTRA
                                                        + OP_TYPEPOSSTAR as ::core::ffi::c_int)
                                                        as uint32_t
                                                    || codevalue
                                                        == (OP_ANYNL_EXTRA
                                                            + OP_TYPEPOSQUERY as ::core::ffi::c_int)
                                                            as uint32_t
                                                {
                                                    active_count -= 1;
                                                    next_active_state =
                                                        next_active_state.offset(-1);
                                                }
                                                let fresh60 = new_count;
                                                new_count = new_count + 1;
                                                if fresh60 < wscount {
                                                    (*next_new_state).offset =
                                                        -(state_offset + count);
                                                    (*next_new_state).count =
                                                        0 as ::core::ffi::c_int;
                                                    (*next_new_state).data = ncount_2;
                                                    next_new_state = next_new_state.offset(1);
                                                } else {
                                                    return PCRE2_ERROR_DFA_WSSIZE;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                3672113957794243512 => {
                                    let fresh57 = active_count;
                                    active_count = active_count + 1;
                                    if fresh57 < wscount {
                                        (*next_active_state).offset =
                                            state_offset + 2 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut ncount_1: ::core::ffi::c_int =
                                            0 as ::core::ffi::c_int;
                                        if codevalue
                                            == (OP_EXTUNI_EXTRA
                                                + OP_TYPEPOSSTAR as ::core::ffi::c_int)
                                                as uint32_t
                                            || codevalue
                                                == (OP_EXTUNI_EXTRA
                                                    + OP_TYPEPOSQUERY as ::core::ffi::c_int)
                                                    as uint32_t
                                        {
                                            active_count -= 1;
                                            next_active_state = next_active_state.offset(-1);
                                        }
                                        _pcre2_extuni_8(
                                            c,
                                            ptr.offset(clen as isize),
                                            (*mb).start_subject,
                                            end_subject,
                                            utf,
                                            &raw mut ncount_1,
                                        );
                                        let fresh58 = new_count;
                                        new_count = new_count + 1;
                                        if fresh58 < wscount {
                                            (*next_new_state).offset = -(state_offset + count);
                                            (*next_new_state).count = 0 as ::core::ffi::c_int;
                                            (*next_new_state).data = ncount_1;
                                            next_new_state = next_new_state.offset(1);
                                        } else {
                                            return PCRE2_ERROR_DFA_WSSIZE;
                                        }
                                    }
                                }
                                4957409898739771784 => {
                                    let fresh54 = active_count;
                                    active_count = active_count + 1;
                                    if fresh54 < wscount {
                                        (*next_active_state).offset =
                                            state_offset + 4 as ::core::ffi::c_int;
                                        (*next_active_state).count = 0 as ::core::ffi::c_int;
                                        next_active_state = next_active_state.offset(1);
                                    } else {
                                        return PCRE2_ERROR_DFA_WSSIZE;
                                    }
                                    if clen > 0 as ::core::ffi::c_int {
                                        let mut OK_3: BOOL = 0;
                                        let mut chartype_3: ::core::ffi::c_int = 0;
                                        let mut cp_1: *const uint32_t =
                                            ::core::ptr::null::<uint32_t>();
                                        let mut prop_1: *const ucd_record =
                                            (&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                .offset(
                                                    *(&raw const _pcre2_ucd_stage2_8
                                                        as *const uint16_t)
                                                        .offset(
                                                            (*(&raw const _pcre2_ucd_stage1_8
                                                                as *const uint16_t)
                                                                .offset(
                                                                    (c as ::core::ffi::c_int
                                                                        / UCD_BLOCK_SIZE)
                                                                        as isize,
                                                                )
                                                                as ::core::ffi::c_int
                                                                * UCD_BLOCK_SIZE
                                                                + c as ::core::ffi::c_int
                                                                    % UCD_BLOCK_SIZE)
                                                                as isize,
                                                        )
                                                        as ::core::ffi::c_int
                                                        as isize,
                                                );
                                        match *code.offset(2 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                        {
                                            PT_LAMP => {
                                                chartype_3 =
                                                    (*prop_1).chartype as ::core::ffi::c_int;
                                                OK_3 = (chartype_3 == ucp_Lu as ::core::ffi::c_int
                                                    || chartype_3 == ucp_Ll as ::core::ffi::c_int
                                                    || chartype_3 == ucp_Lt as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_GC => {
                                                OK_3 = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset((*prop_1).chartype as isize)
                                                    == *code
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_PC => {
                                                OK_3 = ((*prop_1).chartype as ::core::ffi::c_int
                                                    == *code
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SC => {
                                                OK_3 = ((*prop_1).script as ::core::ffi::c_int
                                                    == *code
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SCX => {
                                                OK_3 = ((*prop_1).script as ::core::ffi::c_int
                                                    == *code
                                                        .offset(3 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int
                                                    || *(&raw const _pcre2_ucd_script_sets_8
                                                        as *const uint32_t)
                                                        .offset(
                                                            ((*prop_1).scriptx_bidiclass
                                                                as ::core::ffi::c_int
                                                                & 0x3ff as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                        .offset(
                                                            (*code.offset(
                                                                3 as ::core::ffi::c_int as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                                / 32 as ::core::ffi::c_int)
                                                                as isize,
                                                        )
                                                        & (1 as uint32_t)
                                                            << *code.offset(
                                                                3 as ::core::ffi::c_int as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                                % 32 as ::core::ffi::c_int
                                                        != 0 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_ALNUM => {
                                                chartype_3 =
                                                    (*prop_1).chartype as ::core::ffi::c_int;
                                                OK_3 = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset(chartype_3 as isize)
                                                    == ucp_L as ::core::ffi::c_int as uint32_t
                                                    || *(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset(chartype_3 as isize)
                                                        == ucp_N as ::core::ffi::c_int as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_SPACE | PT_PXSPACE => match c {
                                                9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194
                                                | 8195 | 8196 | 8197 | 8198 | 8199 | 8200
                                                | 8201 | 8202 | 8239 | 8287 | 12288 | 10 | 11
                                                | 12 | 13 | 133 | 8232 | 8233 => {
                                                    OK_3 = TRUE as BOOL;
                                                }
                                                _ => {
                                                    OK_3 = (*(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset((*prop_1).chartype as isize)
                                                        == ucp_Z as ::core::ffi::c_int as uint32_t)
                                                        as ::core::ffi::c_int
                                                        as BOOL;
                                                }
                                            },
                                            PT_WORD => {
                                                chartype_3 =
                                                    (*prop_1).chartype as ::core::ffi::c_int;
                                                OK_3 = (*(&raw const _pcre2_ucp_gentype_8
                                                    as *const uint32_t)
                                                    .offset(chartype_3 as isize)
                                                    == ucp_L as ::core::ffi::c_int as uint32_t
                                                    || *(&raw const _pcre2_ucp_gentype_8
                                                        as *const uint32_t)
                                                        .offset(chartype_3 as isize)
                                                        == ucp_N as ::core::ffi::c_int as uint32_t
                                                    || chartype_3 == ucp_Mn as ::core::ffi::c_int
                                                    || chartype_3 == ucp_Pc as ::core::ffi::c_int)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_CLIST => {
                                                cp_1 =
                                                    (&raw const _pcre2_ucd_caseless_sets_8
                                                        as *const uint32_t)
                                                        .offset(*code.offset(
                                                            3 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            as isize);
                                                loop {
                                                    if c < *cp_1 {
                                                        OK_3 = FALSE as BOOL;
                                                        break;
                                                    } else {
                                                        let fresh55 = cp_1;
                                                        cp_1 = cp_1.offset(1);
                                                        if !(c == *fresh55) {
                                                            continue;
                                                        }
                                                        OK_3 = TRUE as BOOL;
                                                        break;
                                                    }
                                                }
                                            }
                                            PT_UCNC => {
                                                OK_3 = (c == CHAR_DOLLAR_SIGN as uint32_t
                                                    || c == CHAR_COMMERCIAL_AT as uint32_t
                                                    || c == CHAR_GRAVE_ACCENT as uint32_t
                                                    || c >= 0xa0 as uint32_t
                                                        && c <= 0xd7ff as uint32_t
                                                    || c >= 0xe000 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            PT_BIDICL => {
                                                OK_3 = ((*(&raw const _pcre2_ucd_records_8
                                                    as *const ucd_record)
                                                    .offset(
                                                        *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                            .offset(
                                                                (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                    .offset(
                                                                        (c as ::core::ffi::c_int / 128 as ::core::ffi::c_int)
                                                                            as isize,
                                                                    ) as ::core::ffi::c_int * 128 as ::core::ffi::c_int
                                                                    + c as ::core::ffi::c_int % 128 as ::core::ffi::c_int)
                                                                    as isize,
                                                            ) as ::core::ffi::c_int as isize,
                                                    ))
                                                    .scriptx_bidiclass as ::core::ffi::c_int
                                                    >> UCD_BIDICLASS_SHIFT
                                                    == *code.offset(3 as ::core::ffi::c_int as isize)
                                                        as ::core::ffi::c_int) as ::core::ffi::c_int as BOOL;
                                            }
                                            PT_BOOL => {
                                                OK_3 = (*(&raw const _pcre2_ucd_boolprop_sets_8
                                                    as *const uint32_t)
                                                    .offset(
                                                        ((*prop_1).bprops as ::core::ffi::c_int
                                                            & 0xfff as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                    .offset(
                                                        (*code.offset(
                                                            3 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            / 32 as ::core::ffi::c_int)
                                                            as isize,
                                                    )
                                                    & (1 as uint32_t)
                                                        << *code.offset(
                                                            3 as ::core::ffi::c_int as isize,
                                                        )
                                                            as ::core::ffi::c_int
                                                            % 32 as ::core::ffi::c_int
                                                    != 0 as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                            _ => {
                                                OK_3 = (codevalue
                                                    != OP_PROP as ::core::ffi::c_int as uint32_t)
                                                    as ::core::ffi::c_int
                                                    as BOOL;
                                            }
                                        }
                                        if OK_3
                                            == (d == OP_PROP as ::core::ffi::c_int as uint32_t)
                                                as ::core::ffi::c_int
                                        {
                                            if codevalue
                                                == (OP_PROP_EXTRA
                                                    + OP_TYPEPOSSTAR as ::core::ffi::c_int)
                                                    as uint32_t
                                                || codevalue
                                                    == (OP_PROP_EXTRA
                                                        + OP_TYPEPOSQUERY as ::core::ffi::c_int)
                                                        as uint32_t
                                            {
                                                active_count -= 1;
                                                next_active_state = next_active_state.offset(-1);
                                            }
                                            let fresh56 = new_count;
                                            new_count = new_count + 1;
                                            if fresh56 < wscount {
                                                (*next_new_state).offset = state_offset + count;
                                                (*next_new_state).count = 0 as ::core::ffi::c_int;
                                                next_new_state = next_new_state.offset(1);
                                            } else {
                                                return PCRE2_ERROR_DFA_WSSIZE;
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if new_count <= 0 as ::core::ffi::c_int {
            if could_continue != 0
                && ((*mb).moptions & PCRE2_PARTIAL_HARD as uint32_t != 0 as uint32_t
                    || (*mb).moptions & PCRE2_PARTIAL_SOFT as uint32_t != 0 as uint32_t
                        && match_count < 0 as ::core::ffi::c_int)
                && (partial_newline != 0
                    || ptr >= end_subject
                        && (ptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0))
            {
                match_count = PCRE2_ERROR_PARTIAL;
            }
            break;
        } else {
            ptr = ptr.offset(clen as isize);
        }
    }
    if match_count >= 0 as ::core::ffi::c_int
        && ((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED as uint32_t != 0 as uint32_t
        && ptr < end_subject
    {
        match_count = PCRE2_ERROR_NOMATCH;
    }
    return match_count;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_dfa_match_8(
    mut code: *const pcre2_code_8,
    mut subject: PCRE2_SPTR8,
    mut length: size_t,
    mut start_offset: size_t,
    mut options: uint32_t,
    mut match_data: *mut pcre2_match_data_8,
    mut mcontext: *mut pcre2_match_context_8,
    mut workspace: *mut ::core::ffi::c_int,
    mut wscount: size_t,
) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut rc: ::core::ffi::c_int = 0;
    let mut re: *const pcre2_real_code_8 = code as *const pcre2_real_code_8;
    let mut original_options: uint32_t = options;
    let mut null_str: [PCRE2_UCHAR8; 1] = [0xcd as ::core::ffi::c_int as PCRE2_UCHAR8];
    let mut original_subject: PCRE2_SPTR8 = subject;
    let mut start_match: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut end_subject: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut bumpalong_limit: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut req_cu_ptr: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut utf: BOOL = 0;
    let mut anchored: BOOL = 0;
    let mut startline: BOOL = 0;
    let mut firstline: BOOL = 0;
    let mut has_first_cu: BOOL = FALSE;
    let mut has_req_cu: BOOL = FALSE;
    let mut memchr_found_first_cu: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut memchr_found_first_cu2: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut first_cu: PCRE2_UCHAR8 = 0 as PCRE2_UCHAR8;
    let mut first_cu2: PCRE2_UCHAR8 = 0 as PCRE2_UCHAR8;
    let mut req_cu: PCRE2_UCHAR8 = 0 as PCRE2_UCHAR8;
    let mut req_cu2: PCRE2_UCHAR8 = 0 as PCRE2_UCHAR8;
    let mut start_bits: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut cb: pcre2_callout_block_8 = pcre2_callout_block_8 {
        version: 0,
        callout_number: 0,
        capture_top: 0,
        capture_last: 0,
        offset_vector: ::core::ptr::null_mut::<size_t>(),
        mark: ::core::ptr::null::<PCRE2_UCHAR8>(),
        subject: ::core::ptr::null::<PCRE2_UCHAR8>(),
        subject_length: 0,
        start_match: 0,
        current_position: 0,
        pattern_position: 0,
        next_item_length: 0,
        callout_string_offset: 0,
        callout_string_length: 0,
        callout_string: ::core::ptr::null::<PCRE2_UCHAR8>(),
        callout_flags: 0,
    };
    let mut actual_match_block: dfa_match_block_8 = dfa_match_block_8 {
        memctl: pcre2_memctl {
            malloc: None,
            free: None,
            memory_data: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        },
        start_code: ::core::ptr::null::<PCRE2_UCHAR8>(),
        start_subject: ::core::ptr::null::<PCRE2_UCHAR8>(),
        end_subject: ::core::ptr::null::<PCRE2_UCHAR8>(),
        start_used_ptr: ::core::ptr::null::<PCRE2_UCHAR8>(),
        last_used_ptr: ::core::ptr::null::<PCRE2_UCHAR8>(),
        tables: ::core::ptr::null::<uint8_t>(),
        start_offset: 0,
        heap_limit: 0,
        heap_used: 0,
        match_limit: 0,
        match_limit_depth: 0,
        match_call_count: 0,
        moptions: 0,
        poptions: 0,
        nltype: 0,
        nllen: 0,
        allowemptypartial: 0,
        nl: [0; 4],
        bsr_convention: 0,
        cb: ::core::ptr::null_mut::<pcre2_callout_block_8>(),
        callout_data: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        callout: None,
        recursive: ::core::ptr::null_mut::<dfa_recursion_info>(),
    };
    let mut mb: *mut dfa_match_block_8 = &raw mut actual_match_block;
    // The matcher initializes the anchor and each state before reading them.
    // Keep the original C workspace's uninitialized storage without creating
    // an uninitialized Rust value or zeroing 30 KiB on every match.
    let mut base_recursion_workspace = ::core::mem::MaybeUninit::<RWS_workspace>::uninit();
    let mut rws: *mut RWS_anchor =
        &raw mut base_recursion_workspace as *mut ::core::ffi::c_int as *mut RWS_anchor;
    (*rws).next = ::core::ptr::null_mut::<RWS_anchor>();
    (*rws).size = RWS_BASE_SIZE as uint32_t;
    (*rws).free = RWS_BASE_SIZE.wrapping_sub(RWS_ANCHOR_SIZE) as uint32_t;
    if subject.is_null() && length == 0 as size_t {
        subject = &raw mut null_str as *mut PCRE2_UCHAR8 as PCRE2_SPTR8;
    }
    if match_data.is_null() {
        return PCRE2_ERROR_NULL;
    }
    if re.is_null() || subject.is_null() || workspace.is_null() {
        rc = PCRE2_ERROR_NULL;
    } else if options & !(PUBLIC_DFA_MATCH_OPTIONS as uint32_t) != 0 as uint32_t {
        rc = PCRE2_ERROR_BADOPTION;
    } else {
        if length == PCRE2_ZERO_TERMINATED {
            length = _pcre2_strlen_8(subject);
        }
        if wscount < 20 as size_t {
            rc = PCRE2_ERROR_DFA_WSSIZE;
        } else if start_offset > length {
            rc = PCRE2_ERROR_BADOFFSET;
        } else if options & (PCRE2_PARTIAL_HARD as uint32_t | PCRE2_PARTIAL_SOFT as uint32_t)
            != 0 as uint32_t
            && ((*re).overall_options | options) & PCRE2_ENDANCHORED as uint32_t != 0 as uint32_t
        {
            rc = PCRE2_ERROR_BADOPTION;
        } else if (*re).overall_options & PCRE2_MATCH_INVALID_UTF as uint32_t != 0 as uint32_t {
            rc = PCRE2_ERROR_DFA_UINVALID_UTF;
        } else if (*re).magic_number as usize != MAGIC_NUMBER {
            rc = PCRE2_ERROR_BADMAGIC;
        } else if (*re).flags & PCRE2_MODE_MASK as uint32_t
            != (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as uint32_t
        {
            rc = PCRE2_ERROR_BADMODE;
        } else {
            options = (options as ::core::ffi::c_uint
                | ((*re).flags & FF as uint32_t).wrapping_div(
                    (FF as uint32_t & (!(FF as uint32_t)).wrapping_add(1 as uint32_t))
                        .wrapping_div(
                            OO as uint32_t & (!(OO as uint32_t)).wrapping_add(1 as uint32_t),
                        ),
                ) as ::core::ffi::c_uint) as uint32_t;
            if options & PCRE2_DFA_RESTART as uint32_t != 0 as uint32_t {
                if *workspace.offset(0 as ::core::ffi::c_int as isize) & -(2 as ::core::ffi::c_int)
                    != 0 as ::core::ffi::c_int
                    || *workspace.offset(1 as ::core::ffi::c_int as isize) < 1 as ::core::ffi::c_int
                    || *workspace.offset(1 as ::core::ffi::c_int as isize)
                        > wscount
                            .wrapping_sub(2 as size_t)
                            .wrapping_div(INTS_PER_STATEBLOCK as size_t)
                            as ::core::ffi::c_int
                {
                    rc = PCRE2_ERROR_DFA_BADRESTART;
                    current_block = 11797992109807575499;
                } else {
                    current_block = 6450636197030046351;
                }
            } else {
                current_block = 6450636197030046351;
            }
            match current_block {
                11797992109807575499 => {}
                _ => {
                    utf = ((*re).overall_options & PCRE2_UTF as uint32_t != 0 as uint32_t)
                        as ::core::ffi::c_int as BOOL;
                    start_match = subject.offset(start_offset as isize);
                    end_subject = subject.offset(length as isize);
                    req_cu_ptr = start_match.offset(-(1 as ::core::ffi::c_int as isize));
                    anchored = (options
                        & (PCRE2_ANCHORED as uint32_t | PCRE2_DFA_RESTART as uint32_t)
                        != 0 as uint32_t
                        || (*re).overall_options & PCRE2_ANCHORED as uint32_t != 0 as uint32_t)
                        as ::core::ffi::c_int as BOOL;
                    startline = ((*re).flags & PCRE2_STARTLINE as uint32_t != 0 as uint32_t)
                        as ::core::ffi::c_int as BOOL;
                    firstline = (anchored == 0
                        && (*re).overall_options & PCRE2_FIRSTLINE as uint32_t != 0 as uint32_t)
                        as ::core::ffi::c_int as BOOL;
                    bumpalong_limit = end_subject;
                    (*mb).cb = &raw mut cb;
                    cb.version = 2 as uint32_t;
                    cb.subject = subject;
                    cb.subject_length =
                        end_subject.offset_from(subject) as isize as size_t;
                    cb.callout_flags = 0 as uint32_t;
                    cb.capture_top = 1 as uint32_t;
                    cb.capture_last = 0 as uint32_t;
                    cb.mark = ::core::ptr::null::<PCRE2_UCHAR8>();
                    if mcontext.is_null() {
                        (*mb).callout = None;
                        (*mb).memctl = (*re).memctl;
                        (*mb).match_limit = _pcre2_default_match_context_8.match_limit;
                        (*mb).match_limit_depth = _pcre2_default_match_context_8.depth_limit;
                        (*mb).heap_limit = _pcre2_default_match_context_8.heap_limit;
                        current_block = 2116367355679836638;
                    } else {
                        if (*mcontext).offset_limit != PCRE2_UNSET {
                            if (*re).overall_options & PCRE2_USE_OFFSET_LIMIT as uint32_t
                                == 0 as uint32_t
                            {
                                rc = PCRE2_ERROR_BADOFFSETLIMIT;
                                current_block = 11797992109807575499;
                            } else {
                                bumpalong_limit = subject.offset((*mcontext).offset_limit as isize);
                                current_block = 10930818133215224067;
                            }
                        } else {
                            current_block = 10930818133215224067;
                        }
                        match current_block {
                            11797992109807575499 => {}
                            _ => {
                                (*mb).callout = (*mcontext).callout;
                                (*mb).callout_data = (*mcontext).callout_data;
                                (*mb).memctl = (*mcontext).memctl;
                                (*mb).match_limit = (*mcontext).match_limit;
                                (*mb).match_limit_depth = (*mcontext).depth_limit;
                                (*mb).heap_limit = (*mcontext).heap_limit;
                                current_block = 2116367355679836638;
                            }
                        }
                    }
                    match current_block {
                        11797992109807575499 => {}
                        _ => {
                            if (*mb).match_limit > (*re).limit_match {
                                (*mb).match_limit = (*re).limit_match;
                            }
                            if (*mb).match_limit_depth > (*re).limit_depth {
                                (*mb).match_limit_depth = (*re).limit_depth;
                            }
                            if (*mb).heap_limit > (*re).limit_heap {
                                (*mb).heap_limit = (*re).limit_heap;
                            }
                            (*mb).start_code = (re as *const uint8_t)
                                .offset((*re).code_start as isize)
                                as PCRE2_SPTR8;
                            (*mb).tables = (*re).tables;
                            (*mb).start_subject = subject;
                            (*mb).end_subject = end_subject;
                            (*mb).start_offset = start_offset;
                            (*mb).allowemptypartial = ((*re).max_lookbehind as ::core::ffi::c_int
                                > 0 as ::core::ffi::c_int
                                || (*re).flags & PCRE2_MATCH_EMPTY as uint32_t != 0 as uint32_t)
                                as ::core::ffi::c_int
                                as BOOL;
                            (*mb).moptions = options;
                            (*mb).poptions = (*re).overall_options;
                            (*mb).match_call_count = 0 as uint32_t;
                            (*mb).heap_used = 0 as size_t;
                            (*mb).bsr_convention = (*re).bsr_convention;
                            (*mb).nltype = NLTYPE_FIXED as uint32_t;
                            match (*re).newline_convention as ::core::ffi::c_int {
                                PCRE2_NEWLINE_CR => {
                                    (*mb).nllen = 1 as uint32_t;
                                    (*mb).nl[0 as ::core::ffi::c_int as usize] =
                                        CHAR_CR as PCRE2_UCHAR8;
                                    current_block = 10393716428851982524;
                                }
                                PCRE2_NEWLINE_LF => {
                                    (*mb).nllen = 1 as uint32_t;
                                    (*mb).nl[0 as ::core::ffi::c_int as usize] =
                                        CHAR_NL as PCRE2_UCHAR8;
                                    current_block = 10393716428851982524;
                                }
                                PCRE2_NEWLINE_NUL => {
                                    (*mb).nllen = 1 as uint32_t;
                                    (*mb).nl[0 as ::core::ffi::c_int as usize] =
                                        CHAR_NUL as PCRE2_UCHAR8;
                                    current_block = 10393716428851982524;
                                }
                                PCRE2_NEWLINE_CRLF => {
                                    (*mb).nllen = 2 as uint32_t;
                                    (*mb).nl[0 as ::core::ffi::c_int as usize] =
                                        CHAR_CR as PCRE2_UCHAR8;
                                    (*mb).nl[1 as ::core::ffi::c_int as usize] =
                                        CHAR_NL as PCRE2_UCHAR8;
                                    current_block = 10393716428851982524;
                                }
                                PCRE2_NEWLINE_ANY => {
                                    (*mb).nltype = NLTYPE_ANY as uint32_t;
                                    current_block = 10393716428851982524;
                                }
                                PCRE2_NEWLINE_ANYCRLF => {
                                    (*mb).nltype = NLTYPE_ANYCRLF as uint32_t;
                                    current_block = 10393716428851982524;
                                }
                                _ => {
                                    rc = PCRE2_ERROR_INTERNAL;
                                    current_block = 11797992109807575499;
                                }
                            }
                            match current_block {
                                11797992109807575499 => {}
                                _ => {
                                    if utf != 0
                                        && options & PCRE2_NO_UTF_CHECK as uint32_t == 0 as uint32_t
                                    {
                                        let mut check_subject: PCRE2_SPTR8 = start_match;
                                        if start_offset > 0 as size_t {
                                            let mut i: ::core::ffi::c_uint = 0;
                                            if start_match < end_subject
                                                && *start_match as ::core::ffi::c_uint
                                                    & 0xc0 as ::core::ffi::c_uint
                                                    == 0x80 as ::core::ffi::c_uint
                                            {
                                                rc = PCRE2_ERROR_BADUTFOFFSET;
                                                current_block = 11797992109807575499;
                                            } else {
                                                i = (*re).max_lookbehind as ::core::ffi::c_uint;
                                                while i > 0 as ::core::ffi::c_uint
                                                    && check_subject > subject
                                                {
                                                    check_subject = check_subject.offset(-1);
                                                    while check_subject > subject
                                                        && *check_subject as ::core::ffi::c_int
                                                            & 0xc0 as ::core::ffi::c_int
                                                            == 0x80 as ::core::ffi::c_int
                                                    {
                                                        check_subject = check_subject.offset(-1);
                                                    }
                                                    i = i.wrapping_sub(1);
                                                }
                                                current_block = 15734707049249739970;
                                            }
                                        } else {
                                            current_block = 15734707049249739970;
                                        }
                                        match current_block {
                                            11797992109807575499 => {}
                                            _ => {
                                                rc = _pcre2_valid_utf_8(
                                                    check_subject,
                                                    length.wrapping_sub(
                                                        check_subject.offset_from(subject)
                                                            as isize
                                                            as size_t,
                                                    ),
                                                    &raw mut (*match_data).startchar,
                                                );
                                                if rc != 0 as ::core::ffi::c_int {
                                                    (*match_data).startchar =
                                                        (*match_data).startchar.wrapping_add(
                                                            check_subject.offset_from(subject)
                                                                as isize
                                                                as size_t,
                                                        );
                                                    current_block = 11797992109807575499;
                                                } else {
                                                    current_block = 796174441944384681;
                                                }
                                            }
                                        }
                                    } else {
                                        current_block = 796174441944384681;
                                    }
                                    match current_block {
                                        11797992109807575499 => {}
                                        _ => {
                                            if (*re).flags & PCRE2_FIRSTSET as uint32_t
                                                != 0 as uint32_t
                                            {
                                                has_first_cu = TRUE as BOOL;
                                                first_cu2 = (*re).first_codeunit as PCRE2_UCHAR8;
                                                first_cu = first_cu2;
                                                if (*re).flags & PCRE2_FIRSTCASELESS as uint32_t
                                                    != 0 as uint32_t
                                                {
                                                    first_cu2 = *(*mb)
                                                        .tables
                                                        .offset(256 as ::core::ffi::c_int as isize)
                                                        .offset(first_cu as isize)
                                                        as PCRE2_UCHAR8;
                                                    if first_cu as ::core::ffi::c_int
                                                        > 127 as ::core::ffi::c_int
                                                        && utf == 0
                                                        && (*re).overall_options
                                                            & PCRE2_UCP as uint32_t
                                                            != 0 as uint32_t
                                                    {
                                                        first_cu2 = (first_cu as ::core::ffi::c_int
                                                            + (*(&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                                .offset(
                                                                    *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                                        .offset(
                                                                            (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                                .offset(
                                                                                    (first_cu as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                                ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                                + first_cu as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int as isize,
                                                                ))
                                                                .other_case as ::core::ffi::c_int) as uint32_t
                                                            as PCRE2_UCHAR8;
                                                    }
                                                }
                                            } else if startline == 0
                                                && (*re).flags & PCRE2_FIRSTMAPSET as uint32_t
                                                    != 0 as uint32_t
                                            {
                                                start_bits =
                                                    &raw const (*re).start_bitmap as *const uint8_t;
                                            }
                                            if (*re).flags & PCRE2_LASTSET as uint32_t
                                                != 0 as uint32_t
                                            {
                                                has_req_cu = TRUE as BOOL;
                                                req_cu2 = (*re).last_codeunit as PCRE2_UCHAR8;
                                                req_cu = req_cu2;
                                                if (*re).flags & PCRE2_LASTCASELESS as uint32_t
                                                    != 0 as uint32_t
                                                {
                                                    req_cu2 = *(*mb)
                                                        .tables
                                                        .offset(256 as ::core::ffi::c_int as isize)
                                                        .offset(req_cu as isize)
                                                        as PCRE2_UCHAR8;
                                                    if req_cu as ::core::ffi::c_int
                                                        > 127 as ::core::ffi::c_int
                                                        && utf == 0
                                                        && (*re).overall_options
                                                            & PCRE2_UCP as uint32_t
                                                            != 0 as uint32_t
                                                    {
                                                        req_cu2 = (req_cu as ::core::ffi::c_int
                                                            + (*(&raw const _pcre2_ucd_records_8 as *const ucd_record)
                                                                .offset(
                                                                    *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t)
                                                                        .offset(
                                                                            (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                                                                                .offset(
                                                                                    (req_cu as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize,
                                                                                ) as ::core::ffi::c_int * UCD_BLOCK_SIZE
                                                                                + req_cu as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                                                                        ) as ::core::ffi::c_int as isize,
                                                                ))
                                                                .other_case as ::core::ffi::c_int) as uint32_t
                                                            as PCRE2_UCHAR8;
                                                    }
                                                }
                                            }
                                            if (*match_data).flags as ::core::ffi::c_uint
                                                & PCRE2_MD_COPIED_SUBJECT
                                                != 0 as ::core::ffi::c_uint
                                            {
                                                (*match_data)
                                                    .memctl
                                                    .free
                                                    .expect("non-null function pointer")(
                                                    (*match_data).subject
                                                        as *mut ::core::ffi::c_void,
                                                    (*match_data).memctl.memory_data,
                                                );
                                                (*match_data).flags = ((*match_data).flags
                                                    as ::core::ffi::c_uint
                                                    & !PCRE2_MD_COPIED_SUBJECT)
                                                    as uint8_t;
                                            }
                                            (*match_data).code = re;
                                            (*match_data).subject =
                                                ::core::ptr::null::<PCRE2_UCHAR8>();
                                            (*match_data).mark =
                                                ::core::ptr::null::<PCRE2_UCHAR8>();
                                            (*match_data).matchedby =
                                                PCRE2_MATCHEDBY_DFA_INTERPRETER
                                                    as ::core::ffi::c_int
                                                    as uint8_t;
                                            (*match_data).options = original_options;
                                            loop {
                                                if (*re).optimization_flags
                                                    & PCRE2_OPTIM_START_OPTIMIZE as uint32_t
                                                    != 0 as uint32_t
                                                    && options & PCRE2_DFA_RESTART as uint32_t
                                                        == 0 as uint32_t
                                                {
                                                    if firstline != 0 {
                                                        let mut t: PCRE2_SPTR8 = start_match;
                                                        if utf != 0 {
                                                            while t < end_subject
                                                                && (if (*mb).nltype
                                                                    != NLTYPE_FIXED as uint32_t
                                                                {
                                                                    (t < (*mb).end_subject
                                                                        && _pcre2_is_newline_8(
                                                                            t,
                                                                            (*mb).nltype,
                                                                            (*mb).end_subject,
                                                                            &raw mut (*mb).nllen,
                                                                            utf,
                                                                        ) != 0)
                                                                        as ::core::ffi::c_int
                                                                } else {
                                                                    (t <= (*mb).end_subject.offset(-((*mb).nllen as isize))
                                                                        && *t as ::core::ffi::c_int
                                                                            == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                                                as ::core::ffi::c_int
                                                                        && ((*mb).nllen == 1 as uint32_t
                                                                            || *t.offset(1 as ::core::ffi::c_int as isize)
                                                                                as ::core::ffi::c_int
                                                                                == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                                                                    as ::core::ffi::c_int)) as ::core::ffi::c_int
                                                                }) == 0
                                                            {
                                                                t = t.offset(1);
                                                                while t < end_subject
                                                                    && *t as ::core::ffi::c_uint
                                                                        & 0xc0
                                                                            as ::core::ffi::c_uint
                                                                        == 0x80
                                                                            as ::core::ffi::c_uint
                                                                {
                                                                    t = t.offset(1);
                                                                }
                                                            }
                                                        } else {
                                                            while t < end_subject
                                                                && (if (*mb).nltype
                                                                    != NLTYPE_FIXED as uint32_t
                                                                {
                                                                    (t < (*mb).end_subject
                                                                        && _pcre2_is_newline_8(
                                                                            t,
                                                                            (*mb).nltype,
                                                                            (*mb).end_subject,
                                                                            &raw mut (*mb).nllen,
                                                                            utf,
                                                                        ) != 0)
                                                                        as ::core::ffi::c_int
                                                                } else {
                                                                    (t <= (*mb).end_subject.offset(-((*mb).nllen as isize))
                                                                        && *t as ::core::ffi::c_int
                                                                            == (*mb).nl[0 as ::core::ffi::c_int as usize]
                                                                                as ::core::ffi::c_int
                                                                        && ((*mb).nllen == 1 as uint32_t
                                                                            || *t.offset(1 as ::core::ffi::c_int as isize)
                                                                                as ::core::ffi::c_int
                                                                                == (*mb).nl[1 as ::core::ffi::c_int as usize]
                                                                                    as ::core::ffi::c_int)) as ::core::ffi::c_int
                                                                }) == 0
                                                            {
                                                                t = t.offset(1);
                                                            }
                                                        }
                                                        end_subject = t;
                                                    }
                                                    if anchored != 0 {
                                                        if has_first_cu != 0
                                                            || !start_bits.is_null()
                                                        {
                                                            let mut ok: BOOL = (start_match
                                                                < end_subject)
                                                                as ::core::ffi::c_int;
                                                            if ok != 0 {
                                                                let mut c: PCRE2_UCHAR8 =
                                                                    *start_match;
                                                                ok = (has_first_cu != 0
                                                                    && (c as ::core::ffi::c_int
                                                                        == first_cu as ::core::ffi::c_int
                                                                        || c as ::core::ffi::c_int
                                                                            == first_cu2 as ::core::ffi::c_int)) as ::core::ffi::c_int
                                                                    as BOOL;
                                                                if ok == 0 && !start_bits.is_null()
                                                                {
                                                                    ok = (*start_bits
                                                                        .offset(
                                                                            (c as ::core::ffi::c_int / 8 as ::core::ffi::c_int) as isize,
                                                                        ) as ::core::ffi::c_uint
                                                                        & (1 as ::core::ffi::c_uint)
                                                                            << (c as ::core::ffi::c_int & 7 as ::core::ffi::c_int)
                                                                        != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int as BOOL;
                                                                }
                                                            }
                                                            if ok == 0 {
                                                                current_block = 9647479788924160755;
                                                                break;
                                                            }
                                                        }
                                                    } else if has_first_cu != 0 {
                                                        if first_cu as ::core::ffi::c_int
                                                            != first_cu2 as ::core::ffi::c_int
                                                        {
                                                            let mut pp1: PCRE2_SPTR8 =
                                                                ::core::ptr::null::<PCRE2_UCHAR8>();
                                                            let mut pp2: PCRE2_SPTR8 =
                                                                ::core::ptr::null::<PCRE2_UCHAR8>();
                                                            let mut searchlength: size_t =
                                                                end_subject.offset_from(start_match)
                                                                    as isize
                                                                    as size_t;
                                                            if memchr_found_first_cu.is_null()
                                                                || start_match
                                                                    > memchr_found_first_cu
                                                            {
                                                                pp1 = memchr(
                                                                    start_match as *const ::core::ffi::c_void,
                                                                    first_cu as ::core::ffi::c_int,
                                                                    searchlength,
                                                                ) as *const ::core::ffi::c_void as PCRE2_SPTR8;
                                                                memchr_found_first_cu =
                                                                    if pp1.is_null() {
                                                                        end_subject
                                                                    } else {
                                                                        pp1
                                                                    };
                                                            } else {
                                                                pp1 = if memchr_found_first_cu
                                                                    == end_subject
                                                                {
                                                                    ::core::ptr::null::<PCRE2_UCHAR8>(
                                                                    )
                                                                } else {
                                                                    memchr_found_first_cu
                                                                };
                                                            }
                                                            if memchr_found_first_cu2.is_null()
                                                                || start_match
                                                                    > memchr_found_first_cu2
                                                            {
                                                                pp2 = memchr(
                                                                    start_match as *const ::core::ffi::c_void,
                                                                    first_cu2 as ::core::ffi::c_int,
                                                                    searchlength,
                                                                ) as *const ::core::ffi::c_void as PCRE2_SPTR8;
                                                                memchr_found_first_cu2 =
                                                                    if pp2.is_null() {
                                                                        end_subject
                                                                    } else {
                                                                        pp2
                                                                    };
                                                            } else {
                                                                pp2 = if memchr_found_first_cu2
                                                                    == end_subject
                                                                {
                                                                    ::core::ptr::null::<PCRE2_UCHAR8>(
                                                                    )
                                                                } else {
                                                                    memchr_found_first_cu2
                                                                };
                                                            }
                                                            if pp1.is_null() {
                                                                start_match = if pp2.is_null() {
                                                                    end_subject
                                                                } else {
                                                                    pp2
                                                                };
                                                            } else {
                                                                start_match =
                                                                    if pp2.is_null() || pp1 < pp2 {
                                                                        pp1
                                                                    } else {
                                                                        pp2
                                                                    };
                                                            }
                                                        } else {
                                                            start_match = memchr(
                                                                start_match
                                                                    as *const ::core::ffi::c_void,
                                                                first_cu as ::core::ffi::c_int,
                                                                end_subject.offset_from(start_match)
                                                                    as isize
                                                                    as size_t,
                                                            )
                                                                as *const ::core::ffi::c_void
                                                                as PCRE2_SPTR8;
                                                            if start_match.is_null() {
                                                                start_match = end_subject;
                                                            }
                                                        }
                                                        if (*mb).moptions
                                                            & (PCRE2_PARTIAL_HARD as uint32_t
                                                                | PCRE2_PARTIAL_SOFT as uint32_t)
                                                            == 0 as uint32_t
                                                            && start_match >= (*mb).end_subject
                                                        {
                                                            current_block = 9647479788924160755;
                                                            break;
                                                        }
                                                    } else if startline != 0 {
                                                        if start_match
                                                            > (*mb)
                                                                .start_subject
                                                                .offset(start_offset as isize)
                                                        {
                                                            if utf != 0 {
                                                                while start_match < end_subject
                                                                    && (if (*mb).nltype
                                                                        != NLTYPE_FIXED as uint32_t
                                                                    {
                                                                        (start_match
                                                                            > (*mb).start_subject
                                                                            && _pcre2_was_newline_8(
                                                                                start_match,
                                                                                (*mb).nltype,
                                                                                (*mb).start_subject,
                                                                                &raw mut (*mb)
                                                                                    .nllen,
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
                                                                                        as ::core::ffi::c_int)) as ::core::ffi::c_int
                                                                    }) == 0
                                                                {
                                                                    start_match =
                                                                        start_match.offset(1);
                                                                    while start_match < end_subject
                                                                        && *start_match as ::core::ffi::c_uint
                                                                            & 0xc0 as ::core::ffi::c_uint == 0x80 as ::core::ffi::c_uint
                                                                    {
                                                                        start_match = start_match.offset(1);
                                                                    }
                                                                }
                                                            } else {
                                                                while start_match < end_subject
                                                                    && (if (*mb).nltype
                                                                        != NLTYPE_FIXED as uint32_t
                                                                    {
                                                                        (start_match
                                                                            > (*mb).start_subject
                                                                            && _pcre2_was_newline_8(
                                                                                start_match,
                                                                                (*mb).nltype,
                                                                                (*mb).start_subject,
                                                                                &raw mut (*mb)
                                                                                    .nllen,
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
                                                                                        as ::core::ffi::c_int)) as ::core::ffi::c_int
                                                                    }) == 0
                                                                {
                                                                    start_match =
                                                                        start_match.offset(1);
                                                                }
                                                            }
                                                            if *start_match.offset(
                                                                -(1 as ::core::ffi::c_int) as isize,
                                                            )
                                                                as ::core::ffi::c_int
                                                                == CHAR_CR
                                                                && ((*mb).nltype
                                                                    == NLTYPE_ANY as uint32_t
                                                                    || (*mb).nltype
                                                                        == NLTYPE_ANYCRLF
                                                                            as uint32_t)
                                                                && start_match < end_subject
                                                                && *start_match
                                                                    as ::core::ffi::c_int
                                                                    == CHAR_NL
                                                            {
                                                                start_match = start_match.offset(1);
                                                            }
                                                        }
                                                    } else if !start_bits.is_null() {
                                                        while start_match < end_subject {
                                                            let mut c_0: uint32_t =
                                                                *start_match as uint32_t;
                                                            if *start_bits.offset(
                                                                c_0.wrapping_div(8 as uint32_t)
                                                                    as isize,
                                                            )
                                                                as ::core::ffi::c_uint
                                                                & (1 as ::core::ffi::c_uint)
                                                                    << (c_0 & 7 as uint32_t)
                                                                != 0 as ::core::ffi::c_uint
                                                            {
                                                                break;
                                                            }
                                                            start_match = start_match.offset(1);
                                                        }
                                                        if (*mb).moptions
                                                            & (PCRE2_PARTIAL_HARD as uint32_t
                                                                | PCRE2_PARTIAL_SOFT as uint32_t)
                                                            == 0 as uint32_t
                                                            && start_match >= (*mb).end_subject
                                                        {
                                                            current_block = 9647479788924160755;
                                                            break;
                                                        }
                                                    }
                                                    end_subject = (*mb).end_subject;
                                                    if (*mb).moptions
                                                        & (PCRE2_PARTIAL_HARD as uint32_t
                                                            | PCRE2_PARTIAL_SOFT as uint32_t)
                                                        == 0 as uint32_t
                                                    {
                                                        let mut p: PCRE2_SPTR8 =
                                                            ::core::ptr::null::<PCRE2_UCHAR8>();
                                                        if (end_subject.offset_from(start_match)
                                                            as isize)
                                                            < (*re).minlength as isize
                                                        {
                                                            current_block = 9647479788924160755;
                                                            break;
                                                        }
                                                        p = start_match.offset(
                                                            (if has_first_cu != 0 {
                                                                1 as ::core::ffi::c_int
                                                            } else {
                                                                0 as ::core::ffi::c_int
                                                            })
                                                                as isize,
                                                        );
                                                        if has_req_cu != 0 && p > req_cu_ptr {
                                                            let mut check_length: size_t =
                                                                end_subject.offset_from(start_match)
                                                                    as isize
                                                                    as size_t;
                                                            if check_length < REQ_CU_MAX as size_t
                                                                || anchored == 0
                                                                    && check_length
                                                                        < (REQ_CU_MAX * 1000 as ::core::ffi::c_int) as size_t
                                                            {
                                                                if req_cu as ::core::ffi::c_int
                                                                    != req_cu2 as ::core::ffi::c_int
                                                                {
                                                                    let mut pp: PCRE2_SPTR8 = p;
                                                                    p = memchr(
                                                                        pp as *const ::core::ffi::c_void,
                                                                        req_cu as ::core::ffi::c_int,
                                                                        end_subject.offset_from(pp) as isize as size_t,
                                                                    ) as *const ::core::ffi::c_void as PCRE2_SPTR8;
                                                                    if p.is_null() {
                                                                        p = memchr(
                                                                            pp as *const ::core::ffi::c_void,
                                                                            req_cu2 as ::core::ffi::c_int,
                                                                            end_subject.offset_from(pp) as isize as size_t,
                                                                        ) as *const ::core::ffi::c_void as PCRE2_SPTR8;
                                                                        if p.is_null() {
                                                                            p = end_subject;
                                                                        }
                                                                    }
                                                                } else {
                                                                    p = memchr(
                                                                        p as *const ::core::ffi::c_void,
                                                                        req_cu as ::core::ffi::c_int,
                                                                        end_subject.offset_from(p) as isize as size_t,
                                                                    ) as *const ::core::ffi::c_void as PCRE2_SPTR8;
                                                                    if p.is_null() {
                                                                        p = end_subject;
                                                                    }
                                                                }
                                                                if p >= end_subject {
                                                                    current_block = 9647479788924160755;
                                                                    break;
                                                                }
                                                                req_cu_ptr = p;
                                                            }
                                                        }
                                                    }
                                                }
                                                if start_match > bumpalong_limit {
                                                    current_block = 9647479788924160755;
                                                    break;
                                                }
                                                (*mb).start_used_ptr = start_match;
                                                (*mb).last_used_ptr = start_match;
                                                (*mb).recursive =
                                                    ::core::ptr::null_mut::<dfa_recursion_info>();
                                                rc = internal_dfa_match(
                                                    mb,
                                                    (*mb).start_code,
                                                    start_match,
                                                    start_offset,
                                                    &raw mut (*match_data).ovector as *mut size_t,
                                                    ((*match_data).oveccount as uint32_t)
                                                        .wrapping_mul(2 as uint32_t),
                                                    workspace,
                                                    wscount as ::core::ffi::c_int,
                                                    0 as uint32_t,
                                                    &raw mut base_recursion_workspace
                                                        as *mut ::core::ffi::c_int,
                                                );
                                                if rc != PCRE2_ERROR_NOMATCH || anchored != 0 {
                                                    if rc == PCRE2_ERROR_NOMATCH {
                                                        current_block = 9647479788924160755;
                                                        break;
                                                    }
                                                    if rc == PCRE2_ERROR_PARTIAL
                                                        && (*match_data).oveccount
                                                            as ::core::ffi::c_int
                                                            > 0 as ::core::ffi::c_int
                                                    {
                                                        (*match_data).ovector
                                                            [0 as ::core::ffi::c_int as usize] =
                                                            start_match.offset_from(subject)
                                                                as isize
                                                                as size_t;
                                                        (*match_data).ovector
                                                            [1 as ::core::ffi::c_int as usize] =
                                                            end_subject.offset_from(subject)
                                                                as isize
                                                                as size_t;
                                                    }
                                                    if rc >= 0 as ::core::ffi::c_int
                                                        || rc == PCRE2_ERROR_PARTIAL
                                                    {
                                                        (*match_data).subject_length = length;
                                                        (*match_data).start_offset = start_offset;
                                                        (*match_data).leftchar = (*mb)
                                                            .start_used_ptr
                                                            .offset_from(subject)
                                                            as isize
                                                            as size_t;
                                                        (*match_data).rightchar = (*mb)
                                                            .last_used_ptr
                                                            .offset_from(subject)
                                                            as isize
                                                            as size_t;
                                                        (*match_data).startchar = start_match
                                                            .offset_from(subject)
                                                            as isize
                                                            as size_t;
                                                    }
                                                    if rc >= 0 as ::core::ffi::c_int
                                                        && options
                                                            & PCRE2_COPY_MATCHED_SUBJECT as uint32_t
                                                            != 0 as uint32_t
                                                    {
                                                        if length != 0 as size_t {
                                                            (*match_data).subject = (*match_data)
                                                                .memctl
                                                                .malloc
                                                                .expect("non-null function pointer")(
                                                                length.wrapping_mul(
                                                                    (PCRE2_CODE_UNIT_WIDTH
                                                                        / 8 as ::core::ffi::c_int)
                                                                        as size_t,
                                                                ),
                                                                (*match_data).memctl.memory_data,
                                                            )
                                                                as PCRE2_SPTR8;
                                                            if (*match_data).subject.is_null() {
                                                                rc = PCRE2_ERROR_NOMEMORY;
                                                                current_block =
                                                                    11797992109807575499;
                                                                break;
                                                            } else {
                                                                memcpy(
                                                                    (*match_data).subject as *mut ::core::ffi::c_void,
                                                                    subject as *const ::core::ffi::c_void,
                                                                    length
                                                                        .wrapping_mul(
                                                                            (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                                                                        ),
                                                                );
                                                            }
                                                        } else {
                                                            (*match_data).subject =
                                                                ::core::ptr::null::<PCRE2_UCHAR8>();
                                                        }
                                                        (*match_data).flags = ((*match_data).flags
                                                            as ::core::ffi::c_uint
                                                            | PCRE2_MD_COPIED_SUBJECT)
                                                            as uint8_t;
                                                        current_block = 11797992109807575499;
                                                        break;
                                                    } else {
                                                        if rc >= 0 as ::core::ffi::c_int
                                                            || rc == PCRE2_ERROR_PARTIAL
                                                        {
                                                            (*match_data).subject =
                                                                original_subject;
                                                        }
                                                        current_block = 11797992109807575499;
                                                        break;
                                                    }
                                                } else {
                                                    if firstline != 0
                                                        && (if (*mb).nltype
                                                            != NLTYPE_FIXED as uint32_t
                                                        {
                                                            (start_match < (*mb).end_subject
                                                                && _pcre2_is_newline_8(
                                                                    start_match,
                                                                    (*mb).nltype,
                                                                    (*mb).end_subject,
                                                                    &raw mut (*mb).nllen,
                                                                    utf,
                                                                ) != 0)
                                                                as ::core::ffi::c_int
                                                        } else {
                                                            (start_match
                                                                <= (*mb).end_subject.offset(
                                                                    -((*mb).nllen as isize),
                                                                )
                                                                && *start_match
                                                                    as ::core::ffi::c_int
                                                                    == (*mb).nl[0
                                                                        as ::core::ffi::c_int
                                                                        as usize]
                                                                        as ::core::ffi::c_int
                                                                && ((*mb).nllen == 1 as uint32_t
                                                                    || *start_match.offset(
                                                                        1 as ::core::ffi::c_int
                                                                            as isize,
                                                                    )
                                                                        as ::core::ffi::c_int
                                                                        == (*mb).nl[1
                                                                            as ::core::ffi::c_int
                                                                            as usize]
                                                                            as ::core::ffi::c_int))
                                                                as ::core::ffi::c_int
                                                        }) != 0
                                                    {
                                                        current_block = 9647479788924160755;
                                                        break;
                                                    }
                                                    start_match = start_match.offset(1);
                                                    if utf != 0 {
                                                        while start_match < end_subject
                                                            && *start_match as ::core::ffi::c_uint
                                                                & 0xc0 as ::core::ffi::c_uint
                                                                == 0x80 as ::core::ffi::c_uint
                                                        {
                                                            start_match = start_match.offset(1);
                                                        }
                                                    }
                                                    if start_match > end_subject {
                                                        current_block = 9647479788924160755;
                                                        break;
                                                    }
                                                    if *start_match
                                                        .offset(-(1 as ::core::ffi::c_int) as isize)
                                                        as ::core::ffi::c_int
                                                        == CHAR_CR
                                                        && start_match < end_subject
                                                        && *start_match as ::core::ffi::c_int
                                                            == CHAR_NL
                                                        && (*re).flags & PCRE2_HASCRORLF as uint32_t
                                                            == 0 as uint32_t
                                                        && ((*mb).nltype == NLTYPE_ANY as uint32_t
                                                            || (*mb).nltype
                                                                == NLTYPE_ANYCRLF as uint32_t
                                                            || (*mb).nllen == 2 as uint32_t)
                                                    {
                                                        start_match = start_match.offset(1);
                                                    }
                                                }
                                            }
                                            match current_block {
                                                11797992109807575499 => {}
                                                _ => {
                                                    (*match_data).subject = original_subject;
                                                    (*match_data).subject_length = length;
                                                    (*match_data).start_offset = start_offset;
                                                    rc = PCRE2_ERROR_NOMATCH;
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
        }
    }
    while !(*rws).next.is_null() {
        let mut next: *mut RWS_anchor = (*rws).next as *mut RWS_anchor;
        (*rws).next = (*next).next;
        (*mb).memctl.free.expect("non-null function pointer")(
            next as *mut ::core::ffi::c_void,
            (*mb).memctl.memory_data,
        );
    }
    (*match_data).rc = rc;
    return rc;
}
pub const FF: ::core::ffi::c_uint = PCRE2_NOTEMPTY_SET | PCRE2_NE_ATST_SET;
pub const OO: ::core::ffi::c_uint = PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART;
pub const PCRE2_CODE_UNIT_WIDTH: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const LINK_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
