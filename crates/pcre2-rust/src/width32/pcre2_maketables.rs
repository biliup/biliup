// Translated from PCRE2 10.49 pcre2_maketables.c; code unit width 32.
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
// /* This module contains the external function pcre2_maketables(), which builds
// character tables for PCRE2 in the current locale. The file is compiled on its
// own as part of the PCRE2 library. It is also included in the compilation of
// pcre2_dftables.c as a freestanding program, in which case the macro
// PCRE2_DFTABLES is defined. */
extern "C" {
    fn tolower(__c: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn toupper(__c: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type __uint8_t = u8;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const _ISalnum: C2RustUnnamed = 8;
pub const _ISpunct: C2RustUnnamed = 4;
pub const _IScntrl: C2RustUnnamed = 2;
pub const _ISblank: C2RustUnnamed = 1;
pub const _ISgraph: C2RustUnnamed = 32768;
pub const _ISprint: C2RustUnnamed = 16384;
pub const _ISspace: C2RustUnnamed = 8192;
pub const _ISxdigit: C2RustUnnamed = 4096;
pub const _ISdigit: C2RustUnnamed = 2048;
pub const _ISalpha: C2RustUnnamed = 1024;
pub const _ISlower: C2RustUnnamed = 512;
pub const _ISupper: C2RustUnnamed = 256;
pub type size_t = usize;
pub type uint8_t = __uint8_t;
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
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const cbit_space: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const cbit_xdigit: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const cbit_digit: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const cbit_upper: ::core::ffi::c_int = 96 as ::core::ffi::c_int;
pub const cbit_lower: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const cbit_word: ::core::ffi::c_int = 160 as ::core::ffi::c_int;
pub const cbit_graph: ::core::ffi::c_int = 192 as ::core::ffi::c_int;
pub const cbit_print: ::core::ffi::c_int = 224 as ::core::ffi::c_int;
pub const cbit_punct: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const cbit_cntrl: ::core::ffi::c_int = 288 as ::core::ffi::c_int;
pub const cbit_length: ::core::ffi::c_int = 320 as ::core::ffi::c_int;
pub const ctype_space: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const ctype_letter: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const ctype_lcletter: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const ctype_digit: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const ctype_word: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const cbits_offset: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
pub const ctypes_offset: ::core::ffi::c_int = cbits_offset + cbit_length;
pub const TABLES_LENGTH: ::core::ffi::c_int = ctypes_offset + 256 as ::core::ffi::c_int;
pub const CHAR_UNDERSCORE: ::core::ffi::c_int = '_' as i32;
#[no_mangle]
pub unsafe extern "C" fn pcre2_maketables_32(
    mut gcontext: *mut pcre2_general_context_32,
) -> *const uint8_t {
    let mut yield_0: *mut uint8_t = (if !gcontext.is_null() {
        (*gcontext)
            .memctl
            .malloc
            .expect("non-null function pointer")(
            TABLES_LENGTH as size_t,
            (*gcontext).memctl.memory_data,
        )
    } else {
        malloc(TABLES_LENGTH as size_t)
    }) as *mut uint8_t;
    let mut i: ::core::ffi::c_int = 0;
    let mut p: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    if yield_0.is_null() {
        return ::core::ptr::null::<uint8_t>();
    }
    p = yield_0;
    i = 0 as ::core::ffi::c_int;
    while i < 256 as ::core::ffi::c_int {
        let mut c: ::core::ffi::c_int = tolower(i);
        let fresh0 = p;
        p = p.offset(1);
        *fresh0 = (if c < 256 as ::core::ffi::c_int { c } else { i }) as uint8_t;
        i += 1;
    }
    i = 0 as ::core::ffi::c_int;
    while i < 256 as ::core::ffi::c_int {
        let mut c_0: ::core::ffi::c_int = if libc::islower(i)
            != 0
        {
            toupper(i)
        } else {
            tolower(i)
        };
        let fresh1 = p;
        p = p.offset(1);
        *fresh1 = (if c_0 < 256 as ::core::ffi::c_int {
            c_0
        } else {
            i
        }) as uint8_t;
        i += 1;
    }
    memset(
        p as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        cbit_length as size_t,
    );
    i = 0 as ::core::ffi::c_int;
    while i < 256 as ::core::ffi::c_int {
        if libc::isdigit(i)
            != 0
        {
            let ref mut fresh2 = *p.offset((cbit_digit + i / 8 as ::core::ffi::c_int) as isize);
            *fresh2 = (*fresh2 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if libc::isupper(i)
            != 0
        {
            let ref mut fresh3 = *p.offset((cbit_upper + i / 8 as ::core::ffi::c_int) as isize);
            *fresh3 = (*fresh3 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if libc::islower(i)
            != 0
        {
            let ref mut fresh4 = *p.offset((cbit_lower + i / 8 as ::core::ffi::c_int) as isize);
            *fresh4 = (*fresh4 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if libc::isalnum(i)
            != 0
        {
            let ref mut fresh5 = *p.offset((cbit_word + i / 8 as ::core::ffi::c_int) as isize);
            *fresh5 = (*fresh5 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if i == CHAR_UNDERSCORE {
            let ref mut fresh6 = *p.offset((cbit_word + i / 8 as ::core::ffi::c_int) as isize);
            *fresh6 = (*fresh6 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if libc::isspace(i)
            != 0
        {
            let ref mut fresh7 = *p.offset((cbit_space + i / 8 as ::core::ffi::c_int) as isize);
            *fresh7 = (*fresh7 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if libc::isxdigit(i)
            != 0
        {
            let ref mut fresh8 = *p.offset((cbit_xdigit + i / 8 as ::core::ffi::c_int) as isize);
            *fresh8 = (*fresh8 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if libc::isgraph(i)
            != 0
        {
            let ref mut fresh9 = *p.offset((cbit_graph + i / 8 as ::core::ffi::c_int) as isize);
            *fresh9 = (*fresh9 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if libc::isprint(i)
            != 0
        {
            let ref mut fresh10 = *p.offset((cbit_print + i / 8 as ::core::ffi::c_int) as isize);
            *fresh10 = (*fresh10 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if libc::ispunct(i)
            != 0
        {
            let ref mut fresh11 = *p.offset((cbit_punct + i / 8 as ::core::ffi::c_int) as isize);
            *fresh11 = (*fresh11 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if libc::iscntrl(i)
            != 0
        {
            let ref mut fresh12 = *p.offset((cbit_cntrl + i / 8 as ::core::ffi::c_int) as isize);
            *fresh12 = (*fresh12 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        i += 1;
    }
    p = p.offset(cbit_length as isize);
    i = 0 as ::core::ffi::c_int;
    while i < 256 as ::core::ffi::c_int {
        let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        if libc::isspace(i)
            != 0
        {
            x += ctype_space;
        }
        if libc::isalpha(i)
            != 0
        {
            x += ctype_letter;
        }
        if libc::islower(i)
            != 0
        {
            x += ctype_lcletter;
        }
        if libc::isdigit(i)
            != 0
        {
            x += ctype_digit;
        }
        if libc::isalnum(i)
            != 0
            || i == CHAR_UNDERSCORE
        {
            x += ctype_word;
        }
        let fresh13 = p;
        p = p.offset(1);
        *fresh13 = x as uint8_t;
        i += 1;
    }
    return yield_0;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_maketables_free_32(
    mut gcontext: *mut pcre2_general_context_32,
    mut tables: *const uint8_t,
) {
    if !gcontext.is_null() {
        (*gcontext).memctl.free.expect("non-null function pointer")(
            tables as *mut ::core::ffi::c_void,
            (*gcontext).memctl.memory_data,
        );
    } else {
        free(tables as *mut ::core::ffi::c_void);
    };
}
