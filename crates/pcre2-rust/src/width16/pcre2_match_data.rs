// Translated from PCRE2 10.49 pcre2_match_data.c; code unit width 16.
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
    fn _pcre2_memctl_malloc_16(_: size_t, _: *mut pcre2_memctl) -> *mut ::core::ffi::c_void;
}
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type size_t = usize;
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
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const UINT16_MAX: ::core::ffi::c_int = 65535 as ::core::ffi::c_int;
pub const PCRE2_MD_COPIED_SUBJECT: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
#[no_mangle]
pub unsafe extern "C" fn pcre2_match_data_create_16(
    mut oveccount: uint32_t,
    mut gcontext: *mut pcre2_general_context_16,
) -> *mut pcre2_match_data_16 {
    let mut yield_0: *mut pcre2_match_data_16 = ::core::ptr::null_mut::<pcre2_match_data_16>();
    if oveccount < 1 as uint32_t {
        oveccount = 1 as uint32_t;
    }
    if oveccount > UINT16_MAX as uint32_t {
        oveccount = UINT16_MAX as uint32_t;
    }
    yield_0 = _pcre2_memctl_malloc_16(
        (120 as size_t).wrapping_add(
            ((2 as uint32_t).wrapping_mul(oveccount) as size_t)
                .wrapping_mul(::core::mem::size_of::<size_t>() as size_t),
        ),
        gcontext as *mut pcre2_memctl,
    ) as *mut pcre2_match_data_16;
    if yield_0.is_null() {
        return ::core::ptr::null_mut::<pcre2_match_data_16>();
    }
    (*yield_0).oveccount = oveccount as uint16_t;
    (*yield_0).flags = 0 as uint8_t;
    (*yield_0).heapframes = ::core::ptr::null_mut::<heapframe>();
    (*yield_0).heapframes_size = 0 as size_t;
    return yield_0;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_match_data_create_from_pattern_16(
    mut code: *const pcre2_code_16,
    mut gcontext: *mut pcre2_general_context_16,
) -> *mut pcre2_match_data_16 {
    if code.is_null() {
        return ::core::ptr::null_mut::<pcre2_match_data_16>();
    }
    if gcontext.is_null() {
        gcontext = code as *mut pcre2_general_context_16;
    }
    return pcre2_match_data_create_16(
        ((*(code as *const pcre2_real_code_16)).top_bracket as ::core::ffi::c_int
            + 1 as ::core::ffi::c_int) as uint32_t,
        gcontext,
    );
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_match_data_free_16(mut match_data: *mut pcre2_match_data_16) {
    if !match_data.is_null() {
        if !(*match_data).heapframes.is_null() {
            (*match_data)
                .memctl
                .free
                .expect("non-null function pointer")(
                (*match_data).heapframes as *mut ::core::ffi::c_void,
                (*match_data).memctl.memory_data,
            );
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
        }
        (*match_data)
            .memctl
            .free
            .expect("non-null function pointer")(
            match_data as *mut ::core::ffi::c_void,
            (*match_data).memctl.memory_data,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_get_mark_16(
    mut match_data: *mut pcre2_match_data_16,
) -> PCRE2_SPTR16 {
    return (*match_data).mark;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_get_ovector_pointer_16(
    mut match_data: *mut pcre2_match_data_16,
) -> *mut size_t {
    return &raw mut (*match_data).ovector as *mut size_t;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_get_ovector_count_16(
    mut match_data: *mut pcre2_match_data_16,
) -> uint32_t {
    return (*match_data).oveccount as uint32_t;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_get_startchar_16(
    mut match_data: *mut pcre2_match_data_16,
) -> size_t {
    return (*match_data).startchar;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_get_match_data_size_16(
    mut match_data: *mut pcre2_match_data_16,
) -> size_t {
    return (120 as size_t).wrapping_add(
        ((2 as ::core::ffi::c_int * (*match_data).oveccount as ::core::ffi::c_int) as size_t)
            .wrapping_mul(::core::mem::size_of::<size_t>() as size_t),
    );
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_get_match_data_heapframes_size_16(
    mut match_data: *mut pcre2_match_data_16,
) -> size_t {
    return (*match_data).heapframes_size;
}
