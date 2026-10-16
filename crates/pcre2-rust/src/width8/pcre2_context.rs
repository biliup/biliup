// Translated from PCRE2 10.49 pcre2_context.c; code unit width 8.
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
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
    static _pcre2_default_tables_8: [uint8_t; 0];
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
pub struct pcre2_real_general_context_8 {
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
pub type pcre2_general_context_8 = pcre2_real_general_context_8;
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
pub type pcre2_compile_context_8 = pcre2_real_compile_context_8;
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
pub struct pcre2_real_convert_context_8 {
    pub memctl: pcre2_memctl,
    pub glob_separator: uint32_t,
    pub glob_escape: uint32_t,
}
pub type pcre2_convert_context_8 = pcre2_real_convert_context_8;
pub const HEAP_LIMIT: ::core::ffi::c_int = 20000000 as ::core::ffi::c_int;
pub const MATCH_LIMIT: ::core::ffi::c_int = 10000000 as ::core::ffi::c_int;
pub const MATCH_LIMIT_DEPTH: ::core::ffi::c_int = MATCH_LIMIT;
pub const MAX_VARLOOKBEHIND: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const NEWLINE_DEFAULT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PARENS_NEST_LIMIT: ::core::ffi::c_int = 250 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const PCRE2_NEWLINE_CR: uint32_t = 1 as uint32_t;
pub const PCRE2_NEWLINE_LF: uint32_t = 2 as uint32_t;
pub const PCRE2_NEWLINE_CRLF: uint32_t = 3 as uint32_t;
pub const PCRE2_NEWLINE_ANY: uint32_t = 4 as uint32_t;
pub const PCRE2_NEWLINE_ANYCRLF: uint32_t = 5 as uint32_t;
pub const PCRE2_NEWLINE_NUL: uint32_t = 6 as uint32_t;
pub const PCRE2_BSR_UNICODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PCRE2_BSR_ANYCRLF: uint32_t = 2 as uint32_t;
pub const PCRE2_ERROR_BADDATA: ::core::ffi::c_int = -(29 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADOPTION: ::core::ffi::c_int = -(34 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NULL: ::core::ffi::c_int = -(51 as ::core::ffi::c_int);
pub const PCRE2_OPTIMIZATION_NONE: uint32_t = 0 as uint32_t;
pub const PCRE2_OPTIMIZATION_FULL: uint32_t = 1 as uint32_t;
pub const PCRE2_AUTO_POSSESS: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const PCRE2_START_OPTIMIZE_OFF: ::core::ffi::c_int = 69 as ::core::ffi::c_int;
pub const PCRE2_UNSET: size_t = !(0 as ::core::ffi::c_int as size_t);
pub const BSR_DEFAULT: ::core::ffi::c_int = PCRE2_BSR_UNICODE;
pub const PCRE2_OPTIMIZATION_ALL: ::core::ffi::c_uint = 0x7 as ::core::ffi::c_uint;
pub const CHAR_DOT: ::core::ffi::c_int = '.' as i32;
pub const CHAR_SLASH: ::core::ffi::c_int = '/' as i32;
pub const CHAR_BACKSLASH: ::core::ffi::c_int = '\\' as i32;
unsafe extern "C" fn default_malloc(
    mut size: size_t,
    mut data: *mut ::core::ffi::c_void,
) -> *mut ::core::ffi::c_void {
    return malloc(size);
}
unsafe extern "C" fn default_free(
    mut block: *mut ::core::ffi::c_void,
    mut data: *mut ::core::ffi::c_void,
) {
    free(block);
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_memctl_malloc_8(
    mut size: size_t,
    mut memctl: *mut pcre2_memctl,
) -> *mut ::core::ffi::c_void {
    let mut newmemctl: *mut pcre2_memctl = ::core::ptr::null_mut::<pcre2_memctl>();
    let mut yield_0: *mut ::core::ffi::c_void = if memctl.is_null() {
        malloc(size)
    } else {
        (*memctl).malloc.expect("non-null function pointer")(size, (*memctl).memory_data)
    };
    if yield_0.is_null() {
        return NULL;
    }
    newmemctl = yield_0 as *mut pcre2_memctl;
    if memctl.is_null() {
        (*newmemctl).malloc = Some(
            default_malloc
                as unsafe extern "C" fn(
                    size_t,
                    *mut ::core::ffi::c_void,
                ) -> *mut ::core::ffi::c_void,
        )
            as Option<
                unsafe extern "C" fn(size_t, *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void,
            >;
        (*newmemctl).free = Some(
            default_free
                as unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
        )
            as Option<
                unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
            >;
        (*newmemctl).memory_data = NULL;
    } else {
        *newmemctl = *memctl;
    }
    return yield_0;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_general_context_create_8(
    mut private_malloc: Option<
        unsafe extern "C" fn(size_t, *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void,
    >,
    mut private_free: Option<
        unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
    >,
    mut memory_data: *mut ::core::ffi::c_void,
) -> *mut pcre2_general_context_8 {
    let mut gcontext: *mut pcre2_general_context_8 =
        ::core::ptr::null_mut::<pcre2_general_context_8>();
    if private_malloc.is_none() {
        private_malloc = Some(
            default_malloc
                as unsafe extern "C" fn(
                    size_t,
                    *mut ::core::ffi::c_void,
                ) -> *mut ::core::ffi::c_void,
        )
            as Option<
                unsafe extern "C" fn(size_t, *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void,
            >;
    }
    if private_free.is_none() {
        private_free = Some(
            default_free
                as unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
        )
            as Option<
                unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
            >;
    }
    gcontext = private_malloc.expect("non-null function pointer")(
        ::core::mem::size_of::<pcre2_real_general_context_8>() as size_t,
        memory_data,
    ) as *mut pcre2_general_context_8;
    if gcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_general_context_8>();
    }
    (*gcontext).memctl.malloc = private_malloc;
    (*gcontext).memctl.free = private_free;
    (*gcontext).memctl.memory_data = memory_data;
    return gcontext;
}
#[no_mangle]
pub static mut _pcre2_default_compile_context_8: pcre2_compile_context_8 = unsafe {
    pcre2_real_compile_context_8 {
        memctl: pcre2_memctl {
            malloc: Some(
                default_malloc
                    as unsafe extern "C" fn(
                        size_t,
                        *mut ::core::ffi::c_void,
                    ) -> *mut ::core::ffi::c_void,
            ),
            free: Some(
                default_free
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_void,
                        *mut ::core::ffi::c_void,
                    ) -> (),
            ),
            memory_data: NULL,
        },
        stack_guard: None,
        stack_guard_data: NULL,
        tables: &raw const _pcre2_default_tables_8 as *const uint8_t,
        max_pattern_length: PCRE2_UNSET,
        max_pattern_compiled_length: PCRE2_UNSET,
        bsr_convention: BSR_DEFAULT as uint16_t,
        newline_convention: NEWLINE_DEFAULT as uint16_t,
        parens_nest_limit: PARENS_NEST_LIMIT as uint32_t,
        extra_options: 0 as uint32_t,
        max_varlookbehind: MAX_VARLOOKBEHIND as uint32_t,
        optimization_flags: PCRE2_OPTIMIZATION_ALL as uint32_t,
    }
};
#[no_mangle]
pub unsafe extern "C" fn pcre2_compile_context_create_8(
    mut gcontext: *mut pcre2_general_context_8,
) -> *mut pcre2_compile_context_8 {
    let mut ccontext: *mut pcre2_compile_context_8 = _pcre2_memctl_malloc_8(
        ::core::mem::size_of::<pcre2_real_compile_context_8>() as size_t,
        gcontext as *mut pcre2_memctl,
    ) as *mut pcre2_compile_context_8;
    if ccontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_compile_context_8>();
    }
    *ccontext = _pcre2_default_compile_context_8;
    if !gcontext.is_null() {
        *(ccontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
    }
    return ccontext;
}
#[no_mangle]
pub static mut _pcre2_default_match_context_8: pcre2_match_context_8 = unsafe {
    pcre2_real_match_context_8 {
        memctl: pcre2_memctl {
            malloc: Some(
                default_malloc
                    as unsafe extern "C" fn(
                        size_t,
                        *mut ::core::ffi::c_void,
                    ) -> *mut ::core::ffi::c_void,
            ),
            free: Some(
                default_free
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_void,
                        *mut ::core::ffi::c_void,
                    ) -> (),
            ),
            memory_data: NULL,
        },
        jit_callback: None,
        jit_callback_data: NULL,
        callout: None,
        callout_data: NULL,
        substitute_callout: None,
        substitute_callout_data: NULL,
        substitute_case_callout: None,
        substitute_case_callout_data: NULL,
        offset_limit: PCRE2_UNSET,
        heap_limit: HEAP_LIMIT as uint32_t,
        match_limit: MATCH_LIMIT as uint32_t,
        depth_limit: MATCH_LIMIT_DEPTH as uint32_t,
    }
};
#[no_mangle]
pub unsafe extern "C" fn pcre2_match_context_create_8(
    mut gcontext: *mut pcre2_general_context_8,
) -> *mut pcre2_match_context_8 {
    let mut mcontext: *mut pcre2_match_context_8 = _pcre2_memctl_malloc_8(
        ::core::mem::size_of::<pcre2_real_match_context_8>() as size_t,
        gcontext as *mut pcre2_memctl,
    ) as *mut pcre2_match_context_8;
    if mcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_match_context_8>();
    }
    *mcontext = _pcre2_default_match_context_8;
    if !gcontext.is_null() {
        *(mcontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
    }
    return mcontext;
}
#[no_mangle]
pub static mut _pcre2_default_convert_context_8: pcre2_convert_context_8 = unsafe {
    pcre2_real_convert_context_8 {
        memctl: pcre2_memctl {
            malloc: Some(
                default_malloc
                    as unsafe extern "C" fn(
                        size_t,
                        *mut ::core::ffi::c_void,
                    ) -> *mut ::core::ffi::c_void,
            ),
            free: Some(
                default_free
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_void,
                        *mut ::core::ffi::c_void,
                    ) -> (),
            ),
            memory_data: NULL,
        },
        glob_separator: CHAR_SLASH as uint32_t,
        glob_escape: CHAR_BACKSLASH as uint32_t,
    }
};
#[no_mangle]
pub unsafe extern "C" fn pcre2_convert_context_create_8(
    mut gcontext: *mut pcre2_general_context_8,
) -> *mut pcre2_convert_context_8 {
    let mut ccontext: *mut pcre2_convert_context_8 = _pcre2_memctl_malloc_8(
        ::core::mem::size_of::<pcre2_real_convert_context_8>() as size_t,
        gcontext as *mut pcre2_memctl,
    ) as *mut pcre2_convert_context_8;
    if ccontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_convert_context_8>();
    }
    *ccontext = _pcre2_default_convert_context_8;
    if !gcontext.is_null() {
        *(ccontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
    }
    return ccontext;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_general_context_copy_8(
    mut gcontext: *mut pcre2_general_context_8,
) -> *mut pcre2_general_context_8 {
    let mut newcontext: *mut pcre2_general_context_8 = (*gcontext)
        .memctl
        .malloc
        .expect("non-null function pointer")(
        ::core::mem::size_of::<pcre2_real_general_context_8>() as size_t,
        (*gcontext).memctl.memory_data,
    ) as *mut pcre2_general_context_8;
    if newcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_general_context_8>();
    }
    memcpy(
        newcontext as *mut ::core::ffi::c_void,
        gcontext as *const ::core::ffi::c_void,
        ::core::mem::size_of::<pcre2_real_general_context_8>() as size_t,
    );
    return newcontext;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_compile_context_copy_8(
    mut ccontext: *mut pcre2_compile_context_8,
) -> *mut pcre2_compile_context_8 {
    let mut newcontext: *mut pcre2_compile_context_8 = (*ccontext)
        .memctl
        .malloc
        .expect("non-null function pointer")(
        ::core::mem::size_of::<pcre2_real_compile_context_8>() as size_t,
        (*ccontext).memctl.memory_data,
    ) as *mut pcre2_compile_context_8;
    if newcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_compile_context_8>();
    }
    memcpy(
        newcontext as *mut ::core::ffi::c_void,
        ccontext as *const ::core::ffi::c_void,
        ::core::mem::size_of::<pcre2_real_compile_context_8>() as size_t,
    );
    return newcontext;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_match_context_copy_8(
    mut mcontext: *mut pcre2_match_context_8,
) -> *mut pcre2_match_context_8 {
    let mut newcontext: *mut pcre2_match_context_8 = (*mcontext)
        .memctl
        .malloc
        .expect("non-null function pointer")(
        ::core::mem::size_of::<pcre2_real_match_context_8>() as size_t,
        (*mcontext).memctl.memory_data,
    ) as *mut pcre2_match_context_8;
    if newcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_match_context_8>();
    }
    memcpy(
        newcontext as *mut ::core::ffi::c_void,
        mcontext as *const ::core::ffi::c_void,
        ::core::mem::size_of::<pcre2_real_match_context_8>() as size_t,
    );
    return newcontext;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_convert_context_copy_8(
    mut ccontext: *mut pcre2_convert_context_8,
) -> *mut pcre2_convert_context_8 {
    let mut newcontext: *mut pcre2_convert_context_8 = (*ccontext)
        .memctl
        .malloc
        .expect("non-null function pointer")(
        ::core::mem::size_of::<pcre2_real_convert_context_8>() as size_t,
        (*ccontext).memctl.memory_data,
    ) as *mut pcre2_convert_context_8;
    if newcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_convert_context_8>();
    }
    memcpy(
        newcontext as *mut ::core::ffi::c_void,
        ccontext as *const ::core::ffi::c_void,
        ::core::mem::size_of::<pcre2_real_convert_context_8>() as size_t,
    );
    return newcontext;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_general_context_free_8(mut gcontext: *mut pcre2_general_context_8) {
    if !gcontext.is_null() {
        (*gcontext).memctl.free.expect("non-null function pointer")(
            gcontext as *mut ::core::ffi::c_void,
            (*gcontext).memctl.memory_data,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_compile_context_free_8(mut ccontext: *mut pcre2_compile_context_8) {
    if !ccontext.is_null() {
        (*ccontext).memctl.free.expect("non-null function pointer")(
            ccontext as *mut ::core::ffi::c_void,
            (*ccontext).memctl.memory_data,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_match_context_free_8(mut mcontext: *mut pcre2_match_context_8) {
    if !mcontext.is_null() {
        (*mcontext).memctl.free.expect("non-null function pointer")(
            mcontext as *mut ::core::ffi::c_void,
            (*mcontext).memctl.memory_data,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_convert_context_free_8(mut ccontext: *mut pcre2_convert_context_8) {
    if !ccontext.is_null() {
        (*ccontext).memctl.free.expect("non-null function pointer")(
            ccontext as *mut ::core::ffi::c_void,
            (*ccontext).memctl.memory_data,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_character_tables_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut tables: *const uint8_t,
) -> ::core::ffi::c_int {
    (*ccontext).tables = tables;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_bsr_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut value: uint32_t,
) -> ::core::ffi::c_int {
    match value {
        2 | 1 => {
            (*ccontext).bsr_convention = value as uint16_t;
            return 0 as ::core::ffi::c_int;
        }
        _ => return PCRE2_ERROR_BADDATA,
    };
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_max_pattern_length_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut length: size_t,
) -> ::core::ffi::c_int {
    (*ccontext).max_pattern_length = length;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_max_pattern_compiled_length_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut length: size_t,
) -> ::core::ffi::c_int {
    (*ccontext).max_pattern_compiled_length = length;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_newline_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut newline: uint32_t,
) -> ::core::ffi::c_int {
    match newline {
        1 | 2 | 3 | 4 | 5 | 6 => {
            (*ccontext).newline_convention = newline as uint16_t;
            return 0 as ::core::ffi::c_int;
        }
        _ => return PCRE2_ERROR_BADDATA,
    };
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_max_varlookbehind_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    (*ccontext).max_varlookbehind = limit;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_parens_nest_limit_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    (*ccontext).parens_nest_limit = limit;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_compile_extra_options_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut options: uint32_t,
) -> ::core::ffi::c_int {
    (*ccontext).extra_options = options;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_compile_recursion_guard_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut guard: Option<
        unsafe extern "C" fn(uint32_t, *mut ::core::ffi::c_void) -> ::core::ffi::c_int,
    >,
    mut user_data: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    (*ccontext).stack_guard = guard;
    (*ccontext).stack_guard_data = user_data;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_optimize_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut directive: uint32_t,
) -> ::core::ffi::c_int {
    if ccontext.is_null() {
        return PCRE2_ERROR_NULL;
    }
    match directive {
        0 => {
            (*ccontext).optimization_flags = 0 as uint32_t;
        }
        1 => {
            (*ccontext).optimization_flags = PCRE2_OPTIMIZATION_ALL as uint32_t;
        }
        _ => {
            if directive >= PCRE2_AUTO_POSSESS as uint32_t
                && directive <= PCRE2_START_OPTIMIZE_OFF as uint32_t
            {
                if directive & 1 as uint32_t != 0 as uint32_t {
                    (*ccontext).optimization_flags = ((*ccontext).optimization_flags
                        as ::core::ffi::c_uint
                        & !((1 as ::core::ffi::c_uint)
                            << (directive >> 1 as ::core::ffi::c_int).wrapping_sub(32 as uint32_t)))
                        as uint32_t;
                } else {
                    (*ccontext).optimization_flags = ((*ccontext).optimization_flags
                        as ::core::ffi::c_uint
                        | (1 as ::core::ffi::c_uint)
                            << (directive >> 1 as ::core::ffi::c_int).wrapping_sub(32 as uint32_t))
                        as uint32_t;
                }
                return 0 as ::core::ffi::c_int;
            }
            return PCRE2_ERROR_BADOPTION;
        }
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_callout_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut callout: Option<
        unsafe extern "C" fn(
            *mut pcre2_callout_block_8,
            *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
    mut callout_data: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    (*mcontext).callout = callout;
    (*mcontext).callout_data = callout_data;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_substitute_callout_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut substitute_callout: Option<
        unsafe extern "C" fn(
            *mut pcre2_substitute_callout_block_8,
            *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
    mut substitute_callout_data: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    (*mcontext).substitute_callout = substitute_callout;
    (*mcontext).substitute_callout_data = substitute_callout_data;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_substitute_case_callout_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut substitute_case_callout: Option<
        unsafe extern "C" fn(
            PCRE2_SPTR8,
            size_t,
            *mut PCRE2_UCHAR8,
            size_t,
            ::core::ffi::c_int,
            *mut ::core::ffi::c_void,
        ) -> size_t,
    >,
    mut substitute_case_callout_data: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    (*mcontext).substitute_case_callout = substitute_case_callout;
    (*mcontext).substitute_case_callout_data = substitute_case_callout_data;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_heap_limit_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    (*mcontext).heap_limit = limit;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_match_limit_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    (*mcontext).match_limit = limit;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_depth_limit_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    (*mcontext).depth_limit = limit;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_offset_limit_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut limit: size_t,
) -> ::core::ffi::c_int {
    (*mcontext).offset_limit = limit;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_recursion_limit_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    return pcre2_set_depth_limit_8(mcontext, limit);
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_recursion_memory_management_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut mymalloc: Option<
        unsafe extern "C" fn(size_t, *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void,
    >,
    mut myfree: Option<
        unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
    >,
    mut mydata: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_glob_separator_8(
    mut ccontext: *mut pcre2_convert_context_8,
    mut separator: uint32_t,
) -> ::core::ffi::c_int {
    if separator != CHAR_SLASH as uint32_t
        && separator != CHAR_BACKSLASH as uint32_t
        && separator != CHAR_DOT as uint32_t
    {
        return PCRE2_ERROR_BADDATA;
    }
    (*ccontext).glob_separator = separator;
    return 0 as ::core::ffi::c_int;
}
static mut globpunct: *const ::core::ffi::c_char =
    b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~\0" as *const u8 as *const ::core::ffi::c_char;
#[no_mangle]
pub unsafe extern "C" fn pcre2_set_glob_escape_8(
    mut ccontext: *mut pcre2_convert_context_8,
    mut escape: uint32_t,
) -> ::core::ffi::c_int {
    if escape > 255 as uint32_t
        || escape != 0 as uint32_t
            && (strchr(globpunct, escape as ::core::ffi::c_int) as *const ::core::ffi::c_char)
                .is_null()
    {
        return PCRE2_ERROR_BADDATA;
    }
    (*ccontext).glob_escape = escape;
    return 0 as ::core::ffi::c_int;
}
