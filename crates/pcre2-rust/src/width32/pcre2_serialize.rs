// Translated from PCRE2 10.49 pcre2_serialize.c; code unit width 32.
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
// /* This module contains functions for serializing and deserializing
// a sequence of compiled codes. */
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
    static mut _pcre2_default_compile_context_32: pcre2_compile_context_32;
    fn _pcre2_memctl_malloc_32(_: size_t, _: *mut pcre2_memctl) -> *mut ::core::ffi::c_void;
}
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type size_t = usize;
pub type int32_t = __int32_t;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR32 = uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_real_general_context_32 {
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
pub type pcre2_general_context_32 = pcre2_real_general_context_32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_real_compile_context_32 {
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
pub type pcre2_compile_context_32 = pcre2_real_compile_context_32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_real_code_32 {
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
pub type pcre2_code_32 = pcre2_real_code_32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_serialized_data {
    pub magic: uint32_t,
    pub version: uint32_t,
    pub config: uint32_t,
    pub number_of_codes: int32_t,
}
pub const MAX_NAME_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const MAX_NAME_COUNT: ::core::ffi::c_int = 10000 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const PCRE2_MAJOR: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const PCRE2_MINOR: ::core::ffi::c_int = 49 as ::core::ffi::c_int;
pub const PCRE2_ERROR_BADDATA: ::core::ffi::c_int = -(29 as ::core::ffi::c_int);
pub const PCRE2_ERROR_MIXEDTABLES: ::core::ffi::c_int = -(30 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADMAGIC: ::core::ffi::c_int = -(31 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADMODE: ::core::ffi::c_int = -(32 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NOMEMORY: ::core::ffi::c_int = -(48 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NULL: ::core::ffi::c_int = -(51 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADSERIALIZEDDATA: ::core::ffi::c_int = -(62 as ::core::ffi::c_int);
pub const PCRE2_DEREF_TABLES: ::core::ffi::c_uint = 0x40000 as ::core::ffi::c_uint;
pub const MAGIC_NUMBER: usize = 0x50435245 as usize;
pub const cbit_length: ::core::ffi::c_int = 320 as ::core::ffi::c_int;
pub const cbits_offset: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
pub const ctypes_offset: ::core::ffi::c_int = cbits_offset + cbit_length;
pub const TABLES_LENGTH: ::core::ffi::c_int = ctypes_offset + 256 as ::core::ffi::c_int;
pub const IMM2_SIZE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SERIALIZED_DATA_MAGIC: ::core::ffi::c_uint = 0x50523253 as ::core::ffi::c_uint;
pub const SERIALIZED_DATA_VERSION: ::core::ffi::c_int =
    10 as ::core::ffi::c_int | (49 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int;
pub const SERIALIZED_DATA_CONFIG: usize = ::core::mem::size_of::<PCRE2_UCHAR32>() as usize
    | (::core::mem::size_of::<*mut ::core::ffi::c_void>() as usize) << 8 as ::core::ffi::c_int
    | (::core::mem::size_of::<size_t>() as usize) << 16 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn pcre2_serialize_encode_32(
    mut codes: *mut *const pcre2_code_32,
    mut number_of_codes: int32_t,
    mut serialized_bytes: *mut *mut uint8_t,
    mut serialized_size: *mut size_t,
    mut gcontext: *mut pcre2_general_context_32,
) -> int32_t {
    let mut bytes: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut dst_bytes: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut i: int32_t = 0;
    let mut total_size: size_t = 0;
    let mut re: *const pcre2_real_code_32 = ::core::ptr::null::<pcre2_real_code_32>();
    let mut tables: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut data: *mut pcre2_serialized_data = ::core::ptr::null_mut::<pcre2_serialized_data>();
    let mut memctl: *const pcre2_memctl = if !gcontext.is_null() {
        &raw mut (*gcontext).memctl
    } else {
        &raw mut _pcre2_default_compile_context_32.memctl
    };
    if codes.is_null() || serialized_bytes.is_null() || serialized_size.is_null() {
        return PCRE2_ERROR_NULL as int32_t;
    }
    if number_of_codes <= 0 as int32_t {
        return PCRE2_ERROR_BADDATA as int32_t;
    }
    total_size = (::core::mem::size_of::<pcre2_serialized_data>() as usize)
        .wrapping_add(TABLES_LENGTH as usize) as size_t;
    tables = ::core::ptr::null::<uint8_t>();
    i = 0 as ::core::ffi::c_int as int32_t;
    while i < number_of_codes {
        if (*codes.offset(i as isize)).is_null() {
            return PCRE2_ERROR_NULL as int32_t;
        }
        re = *codes.offset(i as isize) as *const pcre2_real_code_32;
        if (*re).magic_number as usize != MAGIC_NUMBER {
            return PCRE2_ERROR_BADMAGIC as int32_t;
        }
        if tables.is_null() {
            tables = (*re).tables;
        } else if tables != (*re).tables {
            return PCRE2_ERROR_MIXEDTABLES as int32_t;
        }
        total_size = total_size.wrapping_add((*re).blocksize);
        i += 1;
    }
    bytes = (*memctl).malloc.expect("non-null function pointer")(
        total_size.wrapping_add(::core::mem::size_of::<pcre2_memctl>() as size_t),
        (*memctl).memory_data,
    ) as *mut uint8_t;
    if bytes.is_null() {
        return PCRE2_ERROR_NOMEMORY as int32_t;
    }
    memcpy(
        bytes as *mut ::core::ffi::c_void,
        memctl as *const ::core::ffi::c_void,
        ::core::mem::size_of::<pcre2_memctl>() as size_t,
    );
    bytes = bytes.offset(::core::mem::size_of::<pcre2_memctl>() as usize as isize);
    data = bytes as *mut pcre2_serialized_data;
    (*data).magic = SERIALIZED_DATA_MAGIC as uint32_t;
    (*data).version = SERIALIZED_DATA_VERSION as uint32_t;
    (*data).config = SERIALIZED_DATA_CONFIG as uint32_t;
    (*data).number_of_codes = number_of_codes;
    dst_bytes = bytes.offset(::core::mem::size_of::<pcre2_serialized_data>() as usize as isize);
    memcpy(
        dst_bytes as *mut ::core::ffi::c_void,
        tables as *const ::core::ffi::c_void,
        TABLES_LENGTH as size_t,
    );
    dst_bytes = dst_bytes.offset(TABLES_LENGTH as isize);
    i = 0 as ::core::ffi::c_int as int32_t;
    while i < number_of_codes {
        re = *codes.offset(i as isize) as *const pcre2_real_code_32;
        memcpy(
            dst_bytes as *mut ::core::ffi::c_void,
            re as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            (*re).blocksize,
        );
        memset(
            dst_bytes.offset(0 as usize as isize) as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<pcre2_memctl>() as size_t,
        );
        memset(
            dst_bytes.offset(24 as usize as isize) as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<*mut ::core::ffi::c_void>() as size_t,
        );
        memset(
            dst_bytes.offset(32 as usize as isize) as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<*mut ::core::ffi::c_void>() as size_t,
        );
        dst_bytes = dst_bytes.offset((*re).blocksize as isize);
        i += 1;
    }
    *serialized_bytes = bytes;
    *serialized_size = total_size;
    return number_of_codes;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_serialize_decode_32(
    mut codes: *mut *mut pcre2_code_32,
    mut number_of_codes: int32_t,
    mut bytes: *const uint8_t,
    mut gcontext: *mut pcre2_general_context_32,
) -> int32_t {
    let mut current_block: u64;
    let mut data: *const pcre2_serialized_data = bytes as *const pcre2_serialized_data;
    let mut memctl: *const pcre2_memctl = if !gcontext.is_null() {
        &raw mut (*gcontext).memctl
    } else {
        &raw mut _pcre2_default_compile_context_32.memctl
    };
    let mut src_bytes: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut dst_re: *mut pcre2_real_code_32 = ::core::ptr::null_mut::<pcre2_real_code_32>();
    let mut tables: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut i: int32_t = 0;
    let mut j: int32_t = 0;
    let mut error: int32_t = 0;
    if data.is_null() || codes.is_null() {
        return PCRE2_ERROR_NULL as int32_t;
    }
    if number_of_codes <= 0 as int32_t {
        return PCRE2_ERROR_BADDATA as int32_t;
    }
    if (*data).number_of_codes <= 0 as int32_t {
        return PCRE2_ERROR_BADSERIALIZEDDATA as int32_t;
    }
    if (*data).magic != SERIALIZED_DATA_MAGIC as uint32_t {
        return PCRE2_ERROR_BADMAGIC as int32_t;
    }
    if (*data).version != SERIALIZED_DATA_VERSION as uint32_t {
        return PCRE2_ERROR_BADMODE as int32_t;
    }
    if (*data).config as usize != SERIALIZED_DATA_CONFIG {
        return PCRE2_ERROR_BADMODE as int32_t;
    }
    if number_of_codes > (*data).number_of_codes {
        number_of_codes = (*data).number_of_codes;
    }
    src_bytes = bytes.offset(::core::mem::size_of::<pcre2_serialized_data>() as usize as isize);
    tables = (*memctl).malloc.expect("non-null function pointer")(
        (TABLES_LENGTH as size_t).wrapping_add(::core::mem::size_of::<size_t>() as size_t),
        (*memctl).memory_data,
    ) as *mut uint8_t;
    if tables.is_null() {
        return PCRE2_ERROR_NOMEMORY as int32_t;
    }
    memcpy(
        tables as *mut ::core::ffi::c_void,
        src_bytes as *const ::core::ffi::c_void,
        TABLES_LENGTH as size_t,
    );
    *(tables.offset(TABLES_LENGTH as isize) as *mut size_t) = number_of_codes as size_t;
    src_bytes = src_bytes.offset(TABLES_LENGTH as isize);
    i = 0 as ::core::ffi::c_int as int32_t;
    loop {
        if !(i < number_of_codes) {
            current_block = 17281240262373992796;
            break;
        }
        let mut blocksize: size_t = 0;
        memcpy(
            &raw mut blocksize as *mut ::core::ffi::c_void,
            src_bytes.offset(72 as usize as isize) as *const ::core::ffi::c_void,
            ::core::mem::size_of::<size_t>() as size_t,
        );
        if blocksize <= ::core::mem::size_of::<pcre2_real_code_32>() as usize {
            error = PCRE2_ERROR_BADSERIALIZEDDATA as int32_t;
            current_block = 11879365840902183751;
            break;
        } else {
            dst_re = _pcre2_memctl_malloc_32(blocksize, gcontext as *mut pcre2_memctl)
                as *mut pcre2_real_code_32;
            if dst_re.is_null() {
                error = PCRE2_ERROR_NOMEMORY as int32_t;
                current_block = 11879365840902183751;
                break;
            } else {
                memcpy(
                    (dst_re as *mut uint8_t)
                        .offset(::core::mem::size_of::<pcre2_memctl>() as usize as isize)
                        as *mut ::core::ffi::c_void,
                    src_bytes.offset(::core::mem::size_of::<pcre2_memctl>() as usize as isize)
                        as *const ::core::ffi::c_void,
                    blocksize.wrapping_sub(::core::mem::size_of::<pcre2_memctl>() as size_t),
                );
                if (*dst_re).magic_number as usize != MAGIC_NUMBER
                    || (*dst_re).name_entry_size as ::core::ffi::c_int
                        > MAX_NAME_SIZE + IMM2_SIZE + 1 as ::core::ffi::c_int
                    || (*dst_re).name_count as ::core::ffi::c_int > MAX_NAME_COUNT
                {
                    error = PCRE2_ERROR_BADSERIALIZEDDATA as int32_t;
                    current_block = 11879365840902183751;
                    break;
                } else {
                    (*dst_re).tables = tables;
                    (*dst_re).executable_jit = NULL;
                    (*dst_re).flags =
                        ((*dst_re).flags as ::core::ffi::c_uint | PCRE2_DEREF_TABLES) as uint32_t;
                    let ref mut fresh0 = *codes.offset(i as isize);
                    *fresh0 = dst_re as *mut pcre2_code_32;
                    dst_re = ::core::ptr::null_mut::<pcre2_real_code_32>();
                    src_bytes = src_bytes.offset(blocksize as isize);
                    i += 1;
                }
            }
        }
    }
    match current_block {
        17281240262373992796 => return number_of_codes,
        _ => {
            if !dst_re.is_null() {
                (*memctl).free.expect("non-null function pointer")(
                    dst_re as *mut ::core::ffi::c_void,
                    (*memctl).memory_data,
                );
            }
            (*memctl).free.expect("non-null function pointer")(
                tables as *mut ::core::ffi::c_void,
                (*memctl).memory_data,
            );
            j = 0 as ::core::ffi::c_int as int32_t;
            while j < i {
                (*memctl).free.expect("non-null function pointer")(
                    *codes.offset(j as isize) as *mut ::core::ffi::c_void,
                    (*memctl).memory_data,
                );
                let ref mut fresh1 = *codes.offset(j as isize);
                *fresh1 = ::core::ptr::null_mut::<pcre2_code_32>();
                j += 1;
            }
            return error;
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_serialize_get_number_of_codes_32(
    mut bytes: *const uint8_t,
) -> int32_t {
    let mut data: *const pcre2_serialized_data = bytes as *const pcre2_serialized_data;
    if data.is_null() {
        return PCRE2_ERROR_NULL as int32_t;
    }
    if (*data).magic != SERIALIZED_DATA_MAGIC as uint32_t {
        return PCRE2_ERROR_BADMAGIC as int32_t;
    }
    if (*data).version != SERIALIZED_DATA_VERSION as uint32_t {
        return PCRE2_ERROR_BADMODE as int32_t;
    }
    if (*data).config as usize != SERIALIZED_DATA_CONFIG {
        return PCRE2_ERROR_BADMODE as int32_t;
    }
    return (*data).number_of_codes;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_serialize_free_32(mut bytes: *mut uint8_t) {
    if !bytes.is_null() {
        let mut memctl: *mut pcre2_memctl = bytes
            .offset(-(::core::mem::size_of::<pcre2_memctl>() as usize as isize))
            as *mut pcre2_memctl;
        (*memctl).free.expect("non-null function pointer")(
            memctl as *mut ::core::ffi::c_void,
            (*memctl).memory_data,
        );
    }
}
