// Translated from PCRE2 10.49 pcre2_substring.c; code unit width 8.
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
    fn _pcre2_memctl_malloc_8(_: size_t, _: *mut pcre2_memctl) -> *mut ::core::ffi::c_void;
    fn _pcre2_strcmp_8(_: PCRE2_SPTR8, _: PCRE2_SPTR8) -> ::core::ffi::c_int;
}
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type size_t = usize;
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
pub const PCRE2_MATCHEDBY_DFA_INTERPRETER: C2RustUnnamed_14 = 1;
pub type C2RustUnnamed_14 = ::core::ffi::c_uint;
pub const PCRE2_MATCHEDBY_JIT: C2RustUnnamed_14 = 2;
pub const PCRE2_MATCHEDBY_INTERPRETER: C2RustUnnamed_14 = 0;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const SIZE_MAX: usize = usize::MAX;
pub const PCRE2_ERROR_PARTIAL: ::core::ffi::c_int = -(2 as ::core::ffi::c_int);
pub const PCRE2_ERROR_DFA_UFUNC: ::core::ffi::c_int = -(41 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NOMEMORY: ::core::ffi::c_int = -(48 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NOSUBSTRING: ::core::ffi::c_int = -(49 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NOUNIQUESUBSTRING: ::core::ffi::c_int = -(50 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UNAVAILABLE: ::core::ffi::c_int = -(54 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UNSET: ::core::ffi::c_int = -(55 as ::core::ffi::c_int);
pub const PCRE2_ERROR_INVALIDOFFSET: ::core::ffi::c_int = -(67 as ::core::ffi::c_int);
pub const PCRE2_SIZE_MAX: usize = SIZE_MAX;
pub const PCRE2_UNSET: size_t = !(0 as ::core::ffi::c_int as size_t);
pub const IMM2_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn pcre2_substring_copy_byname_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringname: PCRE2_SPTR8,
    mut buffer: *mut PCRE2_UCHAR8,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut first: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut last: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut entry: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut failrc: ::core::ffi::c_int = 0;
    let mut entrysize: ::core::ffi::c_int = 0;
    if (*match_data).matchedby as ::core::ffi::c_int
        == PCRE2_MATCHEDBY_DFA_INTERPRETER as ::core::ffi::c_int
    {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code as *const pcre2_code_8,
        stringname,
        &raw mut first,
        &raw mut last,
    );
    if entrysize < 0 as ::core::ffi::c_int {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let mut n: uint32_t = ((*entry.offset(0 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *entry.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int) as uint32_t;
        if n < (*match_data).oveccount as uint32_t {
            if (*match_data).ovector[n.wrapping_mul(2 as uint32_t) as usize] != PCRE2_UNSET {
                return pcre2_substring_copy_bynumber_8(match_data, n, buffer, sizeptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.offset(entrysize as isize);
    }
    return failrc;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_substring_copy_bynumber_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringnumber: uint32_t,
    mut buffer: *mut PCRE2_UCHAR8,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut rc: ::core::ffi::c_int = 0;
    let mut size: size_t = 0;
    rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &raw mut size);
    if rc < 0 as ::core::ffi::c_int {
        return rc;
    }
    if size.wrapping_add(1 as size_t) > *sizeptr {
        return PCRE2_ERROR_NOMEMORY;
    }
    if size != 0 as size_t {
        memcpy(
            buffer as *mut ::core::ffi::c_void,
            (*match_data).subject.offset(
                (*match_data).ovector[stringnumber.wrapping_mul(2 as uint32_t) as usize] as isize,
            ) as *const ::core::ffi::c_void,
            size.wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
        );
    }
    *buffer.offset(size as isize) = 0 as PCRE2_UCHAR8;
    *sizeptr = size;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_substring_get_byname_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringname: PCRE2_SPTR8,
    mut stringptr: *mut *mut PCRE2_UCHAR8,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut first: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut last: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut entry: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut failrc: ::core::ffi::c_int = 0;
    let mut entrysize: ::core::ffi::c_int = 0;
    if (*match_data).matchedby as ::core::ffi::c_int
        == PCRE2_MATCHEDBY_DFA_INTERPRETER as ::core::ffi::c_int
    {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code as *const pcre2_code_8,
        stringname,
        &raw mut first,
        &raw mut last,
    );
    if entrysize < 0 as ::core::ffi::c_int {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let mut n: uint32_t = ((*entry.offset(0 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *entry.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int) as uint32_t;
        if n < (*match_data).oveccount as uint32_t {
            if (*match_data).ovector[n.wrapping_mul(2 as uint32_t) as usize] != PCRE2_UNSET {
                return pcre2_substring_get_bynumber_8(match_data, n, stringptr, sizeptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.offset(entrysize as isize);
    }
    return failrc;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_substring_get_bynumber_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringnumber: uint32_t,
    mut stringptr: *mut *mut PCRE2_UCHAR8,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut rc: ::core::ffi::c_int = 0;
    let mut size: size_t = 0;
    let mut yield_0: *mut PCRE2_UCHAR8 = ::core::ptr::null_mut::<PCRE2_UCHAR8>();
    rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &raw mut size);
    if rc < 0 as ::core::ffi::c_int {
        return rc;
    }
    if size
        > (PCRE2_SIZE_MAX as usize)
            .wrapping_sub(::core::mem::size_of::<pcre2_memctl>() as usize)
            .wrapping_div(
                (1 as ::core::ffi::c_int * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int))
                    as usize,
            )
            .wrapping_sub(1 as usize)
        || {
            yield_0 = _pcre2_memctl_malloc_8(
                (::core::mem::size_of::<pcre2_memctl>() as size_t).wrapping_add(
                    size.wrapping_add(1 as size_t)
                        .wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
                ),
                match_data as *mut pcre2_memctl,
            ) as *mut PCRE2_UCHAR8;
            yield_0.is_null()
        }
    {
        return PCRE2_ERROR_NOMEMORY;
    }
    yield_0 = (yield_0 as *mut ::core::ffi::c_char)
        .offset(::core::mem::size_of::<pcre2_memctl>() as usize as isize)
        as *mut PCRE2_UCHAR8;
    if size != 0 as size_t {
        memcpy(
            yield_0 as *mut ::core::ffi::c_void,
            (*match_data).subject.offset(
                (*match_data).ovector[stringnumber.wrapping_mul(2 as uint32_t) as usize] as isize,
            ) as *const ::core::ffi::c_void,
            size.wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
        );
    }
    *yield_0.offset(size as isize) = 0 as PCRE2_UCHAR8;
    *stringptr = yield_0;
    *sizeptr = size;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_substring_free_8(mut string: *mut PCRE2_UCHAR8) {
    if !string.is_null() {
        let mut memctl: *mut pcre2_memctl = (string as *mut ::core::ffi::c_char)
            .offset(-(::core::mem::size_of::<pcre2_memctl>() as usize as isize))
            as *mut pcre2_memctl;
        (*memctl).free.expect("non-null function pointer")(
            memctl as *mut ::core::ffi::c_void,
            (*memctl).memory_data,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_substring_length_byname_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringname: PCRE2_SPTR8,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut first: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut last: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut entry: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut failrc: ::core::ffi::c_int = 0;
    let mut entrysize: ::core::ffi::c_int = 0;
    if (*match_data).matchedby as ::core::ffi::c_int
        == PCRE2_MATCHEDBY_DFA_INTERPRETER as ::core::ffi::c_int
    {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code as *const pcre2_code_8,
        stringname,
        &raw mut first,
        &raw mut last,
    );
    if entrysize < 0 as ::core::ffi::c_int {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let mut n: uint32_t = ((*entry.offset(0 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *entry.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int) as uint32_t;
        if n < (*match_data).oveccount as uint32_t {
            if (*match_data).ovector[n.wrapping_mul(2 as uint32_t) as usize] != PCRE2_UNSET {
                return pcre2_substring_length_bynumber_8(match_data, n, sizeptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.offset(entrysize as isize);
    }
    return failrc;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_substring_length_bynumber_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringnumber: uint32_t,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut left: size_t = 0;
    let mut right: size_t = 0;
    let mut count: ::core::ffi::c_int = (*match_data).rc;
    if count == PCRE2_ERROR_PARTIAL {
        if stringnumber > 0 as uint32_t {
            return PCRE2_ERROR_PARTIAL;
        }
        count = 0 as ::core::ffi::c_int;
    } else if count < 0 as ::core::ffi::c_int {
        return count;
    }
    if (*match_data).matchedby as ::core::ffi::c_int
        != PCRE2_MATCHEDBY_DFA_INTERPRETER as ::core::ffi::c_int
    {
        if stringnumber > (*(*match_data).code).top_bracket as uint32_t {
            return PCRE2_ERROR_NOSUBSTRING;
        }
        if stringnumber >= (*match_data).oveccount as uint32_t {
            return PCRE2_ERROR_UNAVAILABLE;
        }
        if (*match_data).ovector[stringnumber.wrapping_mul(2 as uint32_t) as usize] == PCRE2_UNSET {
            return PCRE2_ERROR_UNSET;
        }
    } else {
        if stringnumber >= (*match_data).oveccount as uint32_t {
            return PCRE2_ERROR_UNAVAILABLE;
        }
        if count != 0 as ::core::ffi::c_int && stringnumber >= count as uint32_t {
            return PCRE2_ERROR_UNSET;
        }
    }
    left = (*match_data).ovector[stringnumber.wrapping_mul(2 as uint32_t) as usize];
    right = (*match_data).ovector[stringnumber
        .wrapping_mul(2 as uint32_t)
        .wrapping_add(1 as uint32_t) as usize];
    if left > (*match_data).subject_length || right > (*match_data).subject_length {
        return PCRE2_ERROR_INVALIDOFFSET;
    }
    if !sizeptr.is_null() {
        *sizeptr = if left > right {
            0 as size_t
        } else {
            right.wrapping_sub(left)
        };
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_substring_list_get_8(
    mut match_data: *mut pcre2_match_data_8,
    mut listptr: *mut *mut *mut PCRE2_UCHAR8,
    mut lengthsptr: *mut *mut size_t,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0;
    let mut count: ::core::ffi::c_int = 0;
    let mut count2: ::core::ffi::c_int = 0;
    let mut size: size_t = 0;
    let mut lensp: *mut size_t = ::core::ptr::null_mut::<size_t>();
    let mut memp: *mut pcre2_memctl = ::core::ptr::null_mut::<pcre2_memctl>();
    let mut listp: *mut *mut PCRE2_UCHAR8 = ::core::ptr::null_mut::<*mut PCRE2_UCHAR8>();
    let mut sp: *mut PCRE2_UCHAR8 = ::core::ptr::null_mut::<PCRE2_UCHAR8>();
    let mut ovector: *mut size_t = ::core::ptr::null_mut::<size_t>();
    count = (*match_data).rc;
    if count < 0 as ::core::ffi::c_int {
        return count;
    }
    if count == 0 as ::core::ffi::c_int {
        count = (*match_data).oveccount as ::core::ffi::c_int;
    }
    count2 = 2 as ::core::ffi::c_int * count;
    ovector = &raw mut (*match_data).ovector as *mut size_t;
    size = (::core::mem::size_of::<pcre2_memctl>() as usize)
        .wrapping_add(::core::mem::size_of::<*mut PCRE2_UCHAR8>() as usize) as size_t;
    if !lengthsptr.is_null() {
        size = (size as usize).wrapping_add(
            (::core::mem::size_of::<size_t>() as usize).wrapping_mul(count as usize)
                as usize,
        ) as size_t as size_t;
    }
    i = 0 as ::core::ffi::c_int;
    while i < count2 {
        size = (size as usize).wrapping_add(
            (::core::mem::size_of::<*mut PCRE2_UCHAR8>() as usize).wrapping_add(
                (1 as ::core::ffi::c_int * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int))
                    as usize,
            ) as usize,
        ) as size_t as size_t;
        if *ovector.offset((i + 1 as ::core::ffi::c_int) as isize) > *ovector.offset(i as isize) {
            size = size.wrapping_add(
                (*ovector.offset((i + 1 as ::core::ffi::c_int) as isize))
                    .wrapping_sub(*ovector.offset(i as isize))
                    .wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
            );
        }
        i += 2 as ::core::ffi::c_int;
    }
    memp = _pcre2_memctl_malloc_8(size, match_data as *mut pcre2_memctl) as *mut pcre2_memctl;
    if memp.is_null() {
        return PCRE2_ERROR_NOMEMORY;
    }
    listp = (memp as *mut ::core::ffi::c_char)
        .offset(::core::mem::size_of::<pcre2_memctl>() as usize as isize)
        as *mut *mut PCRE2_UCHAR8;
    *listptr = listp;
    lensp = (listp as *mut ::core::ffi::c_char).offset(
        (::core::mem::size_of::<*mut PCRE2_UCHAR8>() as usize)
            .wrapping_mul((count + 1 as ::core::ffi::c_int) as usize) as isize,
    ) as *mut size_t;
    if lengthsptr.is_null() {
        sp = lensp as *mut PCRE2_UCHAR8;
        lensp = ::core::ptr::null_mut::<size_t>();
    } else {
        *lengthsptr = lensp;
        sp = (lensp as *mut ::core::ffi::c_char).offset(
            (::core::mem::size_of::<size_t>() as usize).wrapping_mul(count as usize) as isize,
        ) as *mut PCRE2_UCHAR8;
    }
    i = 0 as ::core::ffi::c_int;
    while i < count2 {
        size = if *ovector.offset((i + 1 as ::core::ffi::c_int) as isize)
            > *ovector.offset(i as isize)
        {
            (*ovector.offset((i + 1 as ::core::ffi::c_int) as isize))
                .wrapping_sub(*ovector.offset(i as isize))
        } else {
            0 as size_t
        };
        if size != 0 as size_t {
            memcpy(
                sp as *mut ::core::ffi::c_void,
                (*match_data)
                    .subject
                    .offset(*ovector.offset(i as isize) as isize)
                    as *const ::core::ffi::c_void,
                size.wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
            );
        }
        let fresh0 = listp;
        listp = listp.offset(1);
        *fresh0 = sp;
        if !lensp.is_null() {
            let fresh1 = lensp;
            lensp = lensp.offset(1);
            *fresh1 = size;
        }
        sp = sp.offset(size as isize);
        let fresh2 = sp;
        sp = sp.offset(1);
        *fresh2 = 0 as PCRE2_UCHAR8;
        i += 2 as ::core::ffi::c_int;
    }
    *listp = ::core::ptr::null_mut::<PCRE2_UCHAR8>();
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_substring_list_free_8(mut list: *mut *mut PCRE2_UCHAR8) {
    if !list.is_null() {
        let mut memctl: *mut pcre2_memctl = (list as *mut ::core::ffi::c_char)
            .offset(-(::core::mem::size_of::<pcre2_memctl>() as usize as isize))
            as *mut pcre2_memctl;
        (*memctl).free.expect("non-null function pointer")(
            memctl as *mut ::core::ffi::c_void,
            (*memctl).memory_data,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_substring_nametable_scan_8(
    mut code: *const pcre2_code_8,
    mut stringname: PCRE2_SPTR8,
    mut firstptr: *mut PCRE2_SPTR8,
    mut lastptr: *mut PCRE2_SPTR8,
) -> ::core::ffi::c_int {
    let mut bot: uint16_t = 0 as uint16_t;
    let mut top: uint16_t = (*code).name_count;
    let mut entrysize: uint16_t = (*code).name_entry_size;
    let mut nametable: PCRE2_SPTR8 = (code as *const ::core::ffi::c_char)
        .offset(::core::mem::size_of::<pcre2_real_code_8>() as usize as isize)
        as PCRE2_SPTR8;
    while top as ::core::ffi::c_int > bot as ::core::ffi::c_int {
        let mut mid: uint16_t = ((top as ::core::ffi::c_int + bot as ::core::ffi::c_int)
            / 2 as ::core::ffi::c_int) as uint16_t;
        let mut entry: PCRE2_SPTR8 = nametable
            .offset((entrysize as ::core::ffi::c_int * mid as ::core::ffi::c_int) as isize);
        let mut c: ::core::ffi::c_int =
            _pcre2_strcmp_8(stringname, entry.offset(IMM2_SIZE as isize));
        if c == 0 as ::core::ffi::c_int {
            let mut first: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
            let mut last: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
            let mut lastentry: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
            lastentry = nametable.offset(
                (entrysize as ::core::ffi::c_int
                    * ((*code).name_count as ::core::ffi::c_int - 1 as ::core::ffi::c_int))
                    as isize,
            );
            last = entry;
            first = last;
            while first > nametable {
                if _pcre2_strcmp_8(
                    stringname,
                    first
                        .offset(-(entrysize as ::core::ffi::c_int as isize))
                        .offset(IMM2_SIZE as isize),
                ) != 0 as ::core::ffi::c_int
                {
                    break;
                }
                first = first.offset(-(entrysize as ::core::ffi::c_int as isize));
            }
            while last < lastentry {
                if _pcre2_strcmp_8(
                    stringname,
                    last.offset(entrysize as ::core::ffi::c_int as isize)
                        .offset(IMM2_SIZE as isize),
                ) != 0 as ::core::ffi::c_int
                {
                    break;
                }
                last = last.offset(entrysize as ::core::ffi::c_int as isize);
            }
            if firstptr.is_null() {
                return if first == last {
                    ((*entry.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *entry
                            .offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint
                        as ::core::ffi::c_int
                } else {
                    PCRE2_ERROR_NOUNIQUESUBSTRING
                };
            }
            *firstptr = first;
            *lastptr = last;
            return entrysize as ::core::ffi::c_int;
        }
        if c > 0 as ::core::ffi::c_int {
            bot = (mid as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as uint16_t;
        } else {
            top = mid;
        }
    }
    return PCRE2_ERROR_NOSUBSTRING;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_substring_number_from_name_8(
    mut code: *const pcre2_code_8,
    mut stringname: PCRE2_SPTR8,
) -> ::core::ffi::c_int {
    return pcre2_substring_nametable_scan_8(
        code,
        stringname,
        ::core::ptr::null_mut::<PCRE2_SPTR8>(),
        ::core::ptr::null_mut::<PCRE2_SPTR8>(),
    );
}
pub const PCRE2_CODE_UNIT_WIDTH: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
