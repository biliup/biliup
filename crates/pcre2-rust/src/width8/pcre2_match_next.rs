// Translated from PCRE2 10.49 pcre2_match_next.c; code unit width 8.
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
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PCRE2_UTF: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PCRE2_NOTEMPTY_ATSTART: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PCRE2_NEWLINE_CRLF: ::core::ffi::c_int = 3;
pub const PCRE2_NEWLINE_ANY: ::core::ffi::c_int = 4;
pub const PCRE2_NEWLINE_ANYCRLF: ::core::ffi::c_int = 5;
pub const CHAR_CR: ::core::ffi::c_int = '\r' as i32;
pub const CHAR_LF: ::core::ffi::c_int = '\n' as i32;
unsafe extern "C" fn do_bumpalong(
    mut match_data: *mut pcre2_match_data_8,
    mut offset: size_t,
) -> size_t {
    let mut subject: PCRE2_SPTR8 = (*match_data).subject;
    let mut subject_length: size_t = (*match_data).subject_length;
    let mut utf: BOOL = ((*(*match_data).code).overall_options & PCRE2_UTF as uint32_t
        != 0 as uint32_t) as ::core::ffi::c_int;
    if *subject.offset(offset as isize) as ::core::ffi::c_int == CHAR_CR
        && offset.wrapping_add(1 as size_t) < subject_length
        && *subject.offset(offset.wrapping_add(1 as size_t) as isize) as ::core::ffi::c_int
            == CHAR_LF
    {
        match (*(*match_data).code).newline_convention as ::core::ffi::c_int {
            PCRE2_NEWLINE_CRLF | PCRE2_NEWLINE_ANY | PCRE2_NEWLINE_ANYCRLF => {
                return offset.wrapping_add(2 as size_t);
            }
            _ => {}
        }
    }
    if utf != 0 {
        let mut next: PCRE2_SPTR8 = subject
            .offset(offset as isize)
            .offset(1 as ::core::ffi::c_int as isize);
        let mut subject_end: PCRE2_SPTR8 = subject.offset(subject_length as isize);
        while next < subject_end
            && *next as ::core::ffi::c_uint & 0xc0 as ::core::ffi::c_uint
                == 0x80 as ::core::ffi::c_uint
        {
            next = next.offset(1);
        }
        return next.offset_from(subject) as isize as size_t;
    }
    return offset.wrapping_add(1 as size_t);
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_next_match_8(
    mut match_data: *mut pcre2_match_data_8,
    mut pstart_offset: *mut size_t,
    mut poptions: *mut uint32_t,
) -> ::core::ffi::c_int {
    let mut rc: ::core::ffi::c_int = (*match_data).rc;
    let mut start_offset: size_t = (*match_data).start_offset;
    let mut ovector: *mut size_t = &raw mut (*match_data).ovector as *mut size_t;
    if rc < 0 as ::core::ffi::c_int {
        return FALSE;
    }
    if *ovector.offset(0 as ::core::ffi::c_int as isize) != start_offset
        && *ovector.offset(1 as ::core::ffi::c_int as isize) == start_offset
    {
        if start_offset >= (*match_data).subject_length {
            return FALSE;
        }
        *pstart_offset = do_bumpalong(
            match_data,
            *ovector.offset(1 as ::core::ffi::c_int as isize),
        );
        *poptions = 0 as uint32_t;
        return TRUE;
    }
    if *ovector.offset(0 as ::core::ffi::c_int as isize)
        == *ovector.offset(1 as ::core::ffi::c_int as isize)
    {
        if *ovector.offset(0 as ::core::ffi::c_int as isize) >= (*match_data).subject_length {
            return FALSE;
        }
        *pstart_offset = *ovector.offset(1 as ::core::ffi::c_int as isize);
        *poptions = PCRE2_NOTEMPTY_ATSTART as uint32_t;
        return TRUE;
    }
    *pstart_offset = *ovector.offset(1 as ::core::ffi::c_int as isize);
    *poptions = 0 as uint32_t;
    return TRUE;
}
