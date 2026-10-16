// Translated from PCRE2 10.49 pcre2_convert.c; code unit width 32.
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
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
    static mut _pcre2_default_convert_context_32: pcre2_convert_context_32;
    static _pcre2_default_tables_32: [uint8_t; 0];
    fn _pcre2_memctl_malloc_32(_: size_t, _: *mut pcre2_memctl) -> *mut ::core::ffi::c_void;
    fn _pcre2_strlen_32(_: PCRE2_SPTR32) -> size_t;
    fn _pcre2_valid_utf_32(_: PCRE2_SPTR32, _: size_t, _: *mut size_t) -> ::core::ffi::c_int;
}
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type size_t = usize;
pub type BOOL = ::core::ffi::c_int;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR32 = uint32_t;
pub type PCRE2_SPTR32 = *const PCRE2_UCHAR32;
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
pub struct pcre2_real_convert_context_32 {
    pub memctl: pcre2_memctl,
    pub glob_separator: uint32_t,
    pub glob_escape: uint32_t,
}
pub type pcre2_convert_context_32 = pcre2_real_convert_context_32;
pub const POSIX_CLASS_NOT_STARTED: C2RustUnnamed = 3;
pub const POSIX_START_REGEX: C2RustUnnamed = 0;
pub const POSIX_NOT_BRACKET: C2RustUnnamed = 2;
pub const POSIX_ANCHORED: C2RustUnnamed = 1;
pub const POSIX_CLASS_STARTED: C2RustUnnamed = 5;
pub const POSIX_CLASS_STARTING: C2RustUnnamed = 4;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pcre2_output_context {
    pub output: *mut PCRE2_UCHAR32,
    pub output_end: PCRE2_SPTR32,
    pub output_size: size_t,
    pub out_str: [uint8_t; 8],
}
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SIZE_MAX: usize = usize::MAX;
pub const PCRE2_CONVERT_UTF: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PCRE2_CONVERT_NO_UTF_CHECK: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PCRE2_CONVERT_POSIX_BASIC: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PCRE2_CONVERT_POSIX_EXTENDED: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PCRE2_CONVERT_GLOB: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR: ::core::ffi::c_uint = 0x30 as ::core::ffi::c_uint;
pub const PCRE2_CONVERT_GLOB_NO_STARSTAR: ::core::ffi::c_uint = 0x50 as ::core::ffi::c_uint;
pub const PCRE2_ERROR_END_BACKSLASH: ::core::ffi::c_int = 101 as ::core::ffi::c_int;
pub const PCRE2_ERROR_MISSING_SQUARE_BRACKET: ::core::ffi::c_int = 106 as ::core::ffi::c_int;
pub const PCRE2_ERROR_BADOPTION: ::core::ffi::c_int = -(34 as ::core::ffi::c_int);
pub const PCRE2_ERROR_INTERNAL: ::core::ffi::c_int = -(44 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NOMEMORY: ::core::ffi::c_int = -(48 as ::core::ffi::c_int);
pub const PCRE2_ERROR_NULL: ::core::ffi::c_int = -(51 as ::core::ffi::c_int);
pub const PCRE2_ERROR_CONVERT_SYNTAX: ::core::ffi::c_int = -(64 as ::core::ffi::c_int);
pub const PCRE2_SIZE_MAX: usize = SIZE_MAX;
pub const PCRE2_ZERO_TERMINATED: size_t = !(0 as ::core::ffi::c_int as size_t);
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
pub const cbits_offset: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
pub const CHAR_VT: ::core::ffi::c_int = '\u{b}' as i32;
pub const CHAR_FF: ::core::ffi::c_int = '\u{c}' as i32;
pub const CHAR_CR: ::core::ffi::c_int = '\r' as i32;
pub const CHAR_LF: ::core::ffi::c_int = '\n' as i32;
pub const CHAR_NUL: ::core::ffi::c_int = '\0' as i32;
pub const CHAR_EXCLAMATION_MARK: ::core::ffi::c_int = '!' as i32;
pub const CHAR_DOLLAR_SIGN: uint32_t = 36 as uint32_t;
pub const CHAR_LEFT_PARENTHESIS: ::core::ffi::c_int = '(' as i32;
pub const CHAR_RIGHT_PARENTHESIS: ::core::ffi::c_int = ')' as i32;
pub const CHAR_ASTERISK: ::core::ffi::c_int = '*' as i32;
pub const CHAR_PLUS: ::core::ffi::c_int = '+' as i32;
pub const CHAR_MINUS: ::core::ffi::c_int = '-' as i32;
pub const CHAR_DOT: ::core::ffi::c_int = '.' as i32;
pub const CHAR_0: ::core::ffi::c_int = '0' as i32;
pub const CHAR_9: ::core::ffi::c_int = '9' as i32;
pub const CHAR_COLON: ::core::ffi::c_int = ':' as i32;
pub const CHAR_LESS_THAN_SIGN: ::core::ffi::c_int = '<' as i32;
pub const CHAR_GREATER_THAN_SIGN: ::core::ffi::c_int = '>' as i32;
pub const CHAR_QUESTION_MARK: ::core::ffi::c_int = '?' as i32;
pub const CHAR_A: ::core::ffi::c_int = 'A' as i32;
pub const CHAR_C: ::core::ffi::c_int = 'C' as i32;
pub const CHAR_I: ::core::ffi::c_int = 'I' as i32;
pub const CHAR_M: ::core::ffi::c_int = 'M' as i32;
pub const CHAR_O: ::core::ffi::c_int = 'O' as i32;
pub const CHAR_T: ::core::ffi::c_int = 'T' as i32;
pub const CHAR_LEFT_SQUARE_BRACKET: ::core::ffi::c_int = '[' as i32;
pub const CHAR_BACKSLASH: ::core::ffi::c_int = '\\' as i32;
pub const CHAR_RIGHT_SQUARE_BRACKET: ::core::ffi::c_int = ']' as i32;
pub const CHAR_CIRCUMFLEX_ACCENT: ::core::ffi::c_int = '^' as i32;
pub const CHAR_UNDERSCORE: ::core::ffi::c_int = '_' as i32;
pub const CHAR_a: ::core::ffi::c_int = 'a' as i32;
pub const CHAR_s: ::core::ffi::c_int = 's' as i32;
pub const CHAR_z: ::core::ffi::c_int = 'z' as i32;
pub const CHAR_LEFT_CURLY_BRACKET: uint32_t = 123 as uint32_t;
pub const CHAR_VERTICAL_LINE: ::core::ffi::c_int = '|' as i32;
pub const CHAR_RIGHT_CURLY_BRACKET: uint32_t = 125 as uint32_t;
pub const TYPE_OPTIONS: ::core::ffi::c_uint =
    PCRE2_CONVERT_GLOB | PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_POSIX_EXTENDED;
pub const ALL_OPTIONS: ::core::ffi::c_uint = PCRE2_CONVERT_UTF
    | PCRE2_CONVERT_NO_UTF_CHECK
    | PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR
    | PCRE2_CONVERT_GLOB_NO_STARSTAR
    | TYPE_OPTIONS;
pub const DUMMY_BUFFER_SIZE: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
static mut pcre2_escaped_literals: *const ::core::ffi::c_char =
    b"\\?*+|.^${}[]()\0" as *const u8 as *const ::core::ffi::c_char;
static mut posix_meta_escapes: *const ::core::ffi::c_char =
    b"(){}123456789\0" as *const u8 as *const ::core::ffi::c_char;
static mut posix_classes: *const ::core::ffi::c_char =
    b"alpha:lower:upper:alnum:ascii:blank:cntrl:digit:graph:print:punct:space:word:xdigit:\0"
        as *const u8 as *const ::core::ffi::c_char;
unsafe extern "C" fn convert_posix(
    mut pattype: uint32_t,
    mut pattern: PCRE2_SPTR32,
    mut plength: size_t,
    mut utf: BOOL,
    mut use_buffer: *mut PCRE2_UCHAR32,
    mut use_length: size_t,
    mut bufflenptr: *mut size_t,
    mut dummyrun: BOOL,
    mut ccontext: *mut pcre2_convert_context_32,
) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut posix: PCRE2_SPTR32 = pattern;
    let mut p: *mut PCRE2_UCHAR32 = use_buffer;
    let mut pp: *mut PCRE2_UCHAR32 = p;
    let mut endp: *mut PCRE2_UCHAR32 = p
        .offset(use_length as isize)
        .offset(-(1 as ::core::ffi::c_int as isize));
    let mut convlength: size_t = 0 as size_t;
    let mut bracount: uint32_t = 0 as uint32_t;
    let mut posix_state: uint32_t = POSIX_START_REGEX as ::core::ffi::c_int as uint32_t;
    let mut lastspecial: uint32_t = 0 as uint32_t;
    let mut extended: BOOL =
        (pattype & PCRE2_CONVERT_POSIX_EXTENDED as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    let mut nextisliteral: BOOL = FALSE;
    *bufflenptr = plength;
    let mut s: *const ::core::ffi::c_char = b"(*NUL)\0" as *const u8 as *const ::core::ffi::c_char;
    while *s as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        if p >= endp {
            return PCRE2_ERROR_NOMEMORY;
        }
        let fresh0 = p;
        p = p.offset(1);
        *fresh0 = *s as PCRE2_UCHAR32;
        s = s.offset(1);
    }
    while plength > 0 as size_t {
        let mut c: uint32_t = 0;
        let mut sc: uint32_t = 0;
        let mut clength: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
        convlength = convlength.wrapping_add(p.offset_from(pp) as isize as size_t);
        if dummyrun != 0 {
            p = use_buffer;
        }
        pp = p;
        c = *posix as uint32_t;
        posix = posix.offset(clength as isize);
        plength = plength.wrapping_sub(clength as size_t);
        sc = if nextisliteral != 0 { 0 as uint32_t } else { c };
        nextisliteral = FALSE as BOOL;
        if posix_state >= POSIX_CLASS_NOT_STARTED as ::core::ffi::c_int as uint32_t {
            if c == CHAR_RIGHT_SQUARE_BRACKET as uint32_t {
                let mut s_0: *const ::core::ffi::c_char =
                    b"]\0" as *const u8 as *const ::core::ffi::c_char;
                while *s_0 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                    if p >= endp {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    let fresh1 = p;
                    p = p.offset(1);
                    *fresh1 = *s_0 as PCRE2_UCHAR32;
                    s_0 = s_0.offset(1);
                }
                posix_state = POSIX_NOT_BRACKET as ::core::ffi::c_int as uint32_t;
            } else {
                match posix_state {
                    5 => {
                        if c >= CHAR_a as uint32_t && c <= CHAR_z as uint32_t {
                            current_block = 2122094917359643297;
                        } else {
                            posix_state = POSIX_CLASS_NOT_STARTED as ::core::ffi::c_int as uint32_t;
                            if c == CHAR_COLON as uint32_t
                                && plength > 0 as size_t
                                && *posix == CHAR_RIGHT_SQUARE_BRACKET as PCRE2_UCHAR32
                            {
                                let mut s_1: *const ::core::ffi::c_char =
                                    b":]\0" as *const u8 as *const ::core::ffi::c_char;
                                while *s_1 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                                    if p >= endp {
                                        return PCRE2_ERROR_NOMEMORY;
                                    }
                                    let fresh2 = p;
                                    p = p.offset(1);
                                    *fresh2 = *s_1 as PCRE2_UCHAR32;
                                    s_1 = s_1.offset(1);
                                }
                                plength = plength.wrapping_sub(1);
                                posix = posix.offset(1);
                                continue;
                            } else {
                                current_block = 11071765304959134860;
                            }
                        }
                    }
                    3 => {
                        current_block = 11071765304959134860;
                    }
                    4 => {
                        if c == CHAR_COLON as uint32_t {
                            posix_state = POSIX_CLASS_STARTED as ::core::ffi::c_int as uint32_t;
                        }
                        current_block = 2122094917359643297;
                    }
                    _ => {
                        current_block = 2122094917359643297;
                    }
                }
                match current_block {
                    11071765304959134860 => {
                        if c == CHAR_LEFT_SQUARE_BRACKET as uint32_t {
                            posix_state = POSIX_CLASS_STARTING as ::core::ffi::c_int as uint32_t;
                        }
                    }
                    _ => {}
                }
                if c == CHAR_BACKSLASH as uint32_t {
                    let mut s_2: *const ::core::ffi::c_char =
                        b"\\\0" as *const u8 as *const ::core::ffi::c_char;
                    while *s_2 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                        if p >= endp {
                            return PCRE2_ERROR_NOMEMORY;
                        }
                        let fresh3 = p;
                        p = p.offset(1);
                        *fresh3 = *s_2 as PCRE2_UCHAR32;
                        s_2 = s_2.offset(1);
                    }
                }
                if p.offset(clength as isize) > endp {
                    return PCRE2_ERROR_NOMEMORY;
                }
                memcpy(
                    p as *mut ::core::ffi::c_void,
                    posix.offset(-(clength as isize)) as *const ::core::ffi::c_void,
                    (clength * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int)) as size_t,
                );
                p = p.offset(clength as isize);
            }
        } else {
            match sc {
                91 => {
                    let mut s_3: *const ::core::ffi::c_char =
                        b"[\0" as *const u8 as *const ::core::ffi::c_char;
                    while *s_3 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                        if p >= endp {
                            return PCRE2_ERROR_NOMEMORY;
                        }
                        let fresh4 = p;
                        p = p.offset(1);
                        *fresh4 = *s_3 as PCRE2_UCHAR32;
                        s_3 = s_3.offset(1);
                    }
                    posix_state = POSIX_CLASS_NOT_STARTED as ::core::ffi::c_int as uint32_t;
                    if plength > 0 as size_t {
                        if *posix == CHAR_CIRCUMFLEX_ACCENT as PCRE2_UCHAR32 {
                            posix = posix.offset(1);
                            plength = plength.wrapping_sub(1);
                            let mut s_4: *const ::core::ffi::c_char =
                                b"^\0" as *const u8 as *const ::core::ffi::c_char;
                            while *s_4 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                                if p >= endp {
                                    return PCRE2_ERROR_NOMEMORY;
                                }
                                let fresh5 = p;
                                p = p.offset(1);
                                *fresh5 = *s_4 as PCRE2_UCHAR32;
                                s_4 = s_4.offset(1);
                            }
                        }
                        if plength > 0 as size_t
                            && *posix == CHAR_RIGHT_SQUARE_BRACKET as PCRE2_UCHAR32
                        {
                            posix = posix.offset(1);
                            plength = plength.wrapping_sub(1);
                            let mut s_5: *const ::core::ffi::c_char =
                                b"]\0" as *const u8 as *const ::core::ffi::c_char;
                            while *s_5 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                                if p >= endp {
                                    return PCRE2_ERROR_NOMEMORY;
                                }
                                let fresh6 = p;
                                p = p.offset(1);
                                *fresh6 = *s_5 as PCRE2_UCHAR32;
                                s_5 = s_5.offset(1);
                            }
                        }
                    }
                    continue;
                }
                92 => {
                    if plength == 0 as size_t {
                        return PCRE2_ERROR_END_BACKSLASH;
                    }
                    if extended != 0 {
                        nextisliteral = TRUE as BOOL;
                    } else if *posix < 255 as PCRE2_UCHAR32
                        && !(strchr(posix_meta_escapes, *posix as ::core::ffi::c_int)
                            as *const ::core::ffi::c_char)
                            .is_null()
                    {
                        if *posix >= CHAR_0 as PCRE2_UCHAR32 && *posix <= CHAR_9 as PCRE2_UCHAR32 {
                            let mut s_6: *const ::core::ffi::c_char =
                                b"\\\0" as *const u8 as *const ::core::ffi::c_char;
                            while *s_6 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                                if p >= endp {
                                    return PCRE2_ERROR_NOMEMORY;
                                }
                                let fresh7 = p;
                                p = p.offset(1);
                                *fresh7 = *s_6 as PCRE2_UCHAR32;
                                s_6 = s_6.offset(1);
                            }
                        }
                        if p.offset(1 as ::core::ffi::c_int as isize) > endp {
                            return PCRE2_ERROR_NOMEMORY;
                        }
                        let fresh8 = posix;
                        posix = posix.offset(1);
                        let fresh9 = p;
                        p = p.offset(1);
                        *fresh9 = *fresh8;
                        lastspecial = *fresh9 as uint32_t;
                        plength = plength.wrapping_sub(1);
                    } else {
                        nextisliteral = TRUE as BOOL;
                    }
                    continue;
                }
                41 => {
                    if extended == 0 || bracount == 0 as uint32_t {
                        current_block = 3487612082277595709;
                    } else {
                        bracount = bracount.wrapping_sub(1);
                        current_block = 17926094401822700770;
                    }
                }
                40 => {
                    bracount = bracount.wrapping_add(1);
                    current_block = 2869672365253668134;
                }
                63 | 43 | 123 | 125 | 124 => {
                    current_block = 2869672365253668134;
                }
                46 | 36 => {
                    current_block = 14733463508963758080;
                }
                42 => {
                    if !(lastspecial != CHAR_ASTERISK as uint32_t) {
                        continue;
                    }
                    if extended == 0
                        && (posix_state < POSIX_NOT_BRACKET as ::core::ffi::c_int as uint32_t
                            || lastspecial == CHAR_LEFT_PARENTHESIS as uint32_t)
                    {
                        current_block = 3487612082277595709;
                    } else {
                        current_block = 17926094401822700770;
                    }
                }
                94 => {
                    if extended != 0 {
                        current_block = 17926094401822700770;
                    } else if posix_state == POSIX_START_REGEX as ::core::ffi::c_int as uint32_t
                        || lastspecial == CHAR_LEFT_PARENTHESIS as uint32_t
                    {
                        posix_state = POSIX_ANCHORED as ::core::ffi::c_int as uint32_t;
                        current_block = 17926094401822700770;
                    } else {
                        current_block = 11306844946966188313;
                    }
                }
                _ => {
                    current_block = 11306844946966188313;
                }
            }
            match current_block {
                2869672365253668134 => {
                    if extended == 0 {
                        current_block = 3487612082277595709;
                    } else {
                        current_block = 14733463508963758080;
                    }
                }
                11306844946966188313 => {
                    if c < 255 as uint32_t
                        && !(strchr(pcre2_escaped_literals, c as ::core::ffi::c_int)
                            as *const ::core::ffi::c_char)
                            .is_null()
                    {
                        current_block = 3487612082277595709;
                    } else {
                        current_block = 12625034345703806147;
                    }
                }
                _ => {}
            }
            match current_block {
                14733463508963758080 => {
                    posix_state = POSIX_NOT_BRACKET as ::core::ffi::c_int as uint32_t;
                    current_block = 17926094401822700770;
                }
                3487612082277595709 => {
                    let mut s_7: *const ::core::ffi::c_char =
                        b"\\\0" as *const u8 as *const ::core::ffi::c_char;
                    while *s_7 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                        if p >= endp {
                            return PCRE2_ERROR_NOMEMORY;
                        }
                        let fresh11 = p;
                        p = p.offset(1);
                        *fresh11 = *s_7 as PCRE2_UCHAR32;
                        s_7 = s_7.offset(1);
                    }
                    current_block = 12625034345703806147;
                }
                _ => {}
            }
            match current_block {
                12625034345703806147 => {
                    lastspecial = 0xff as uint32_t;
                    if p.offset(clength as isize) > endp {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    memcpy(
                        p as *mut ::core::ffi::c_void,
                        posix.offset(-(clength as isize)) as *const ::core::ffi::c_void,
                        (clength * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int)) as size_t,
                    );
                    p = p.offset(clength as isize);
                    posix_state = POSIX_NOT_BRACKET as ::core::ffi::c_int as uint32_t;
                }
                _ => {
                    lastspecial = c;
                    if p.offset(1 as ::core::ffi::c_int as isize) > endp {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    let fresh10 = p;
                    p = p.offset(1);
                    *fresh10 = c as PCRE2_UCHAR32;
                }
            }
        }
    }
    if posix_state >= POSIX_CLASS_NOT_STARTED as ::core::ffi::c_int as uint32_t {
        return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
    }
    convlength = convlength.wrapping_add(p.offset_from(pp) as isize as size_t);
    *bufflenptr = convlength;
    let fresh12 = p;
    p = p.offset(1);
    *fresh12 = 0 as PCRE2_UCHAR32;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn convert_glob_write(
    mut out: *mut pcre2_output_context,
    mut chr: PCRE2_UCHAR32,
) {
    (*out).output_size = (*out).output_size.wrapping_add(1);
    if (*out).output < (*out).output_end as *mut PCRE2_UCHAR32 {
        let fresh15 = (*out).output;
        (*out).output = (*out).output.offset(1);
        *fresh15 = chr;
    }
}
unsafe extern "C" fn convert_glob_write_str(
    mut out: *mut pcre2_output_context,
    mut length: size_t,
) {
    let mut out_str: *mut uint8_t = &raw mut (*out).out_str as *mut uint8_t;
    let mut output: *mut PCRE2_UCHAR32 = (*out).output;
    let mut output_end: PCRE2_SPTR32 = (*out).output_end;
    let mut output_size: size_t = (*out).output_size;
    loop {
        output_size = output_size.wrapping_add(1);
        if output < output_end as *mut PCRE2_UCHAR32 {
            let fresh16 = out_str;
            out_str = out_str.offset(1);
            let fresh17 = output;
            output = output.offset(1);
            *fresh17 = *fresh16 as PCRE2_UCHAR32;
        }
        length = length.wrapping_sub(1);
        if !(length != 0 as size_t) {
            break;
        }
    }
    (*out).output = output;
    (*out).output_size = output_size;
}
unsafe extern "C" fn convert_glob_print_separator(
    mut out: *mut pcre2_output_context,
    mut separator: PCRE2_UCHAR32,
    mut with_escape: BOOL,
) {
    if with_escape != 0 {
        convert_glob_write(out, CHAR_BACKSLASH as PCRE2_UCHAR32);
    }
    convert_glob_write(out, separator);
}
unsafe extern "C" fn convert_glob_print_wildcard(
    mut out: *mut pcre2_output_context,
    mut separator: PCRE2_UCHAR32,
    mut with_escape: BOOL,
) {
    (*out).out_str[0 as ::core::ffi::c_int as usize] = CHAR_LEFT_SQUARE_BRACKET as uint8_t;
    (*out).out_str[1 as ::core::ffi::c_int as usize] = CHAR_CIRCUMFLEX_ACCENT as uint8_t;
    convert_glob_write_str(out, 2 as size_t);
    convert_glob_print_separator(out, separator, with_escape);
    convert_glob_write(out, CHAR_RIGHT_SQUARE_BRACKET as PCRE2_UCHAR32);
}
unsafe extern "C" fn convert_glob_parse_class(
    mut from: *mut PCRE2_SPTR32,
    mut pattern_end: PCRE2_SPTR32,
    mut out: *mut pcre2_output_context,
) -> ::core::ffi::c_int {
    let mut start: PCRE2_SPTR32 = (*from).offset(1 as ::core::ffi::c_int as isize);
    let mut pattern: PCRE2_SPTR32 = start;
    let mut class_ptr: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut c: PCRE2_UCHAR32 = 0;
    let mut class_index: ::core::ffi::c_int = 0;
    loop {
        if pattern >= pattern_end {
            return 0 as ::core::ffi::c_int;
        }
        let fresh23 = pattern;
        pattern = pattern.offset(1);
        c = *fresh23;
        if c < CHAR_a as PCRE2_UCHAR32 || c > CHAR_z as PCRE2_UCHAR32 {
            break;
        }
    }
    if c != CHAR_COLON as PCRE2_UCHAR32
        || pattern >= pattern_end
        || *pattern != CHAR_RIGHT_SQUARE_BRACKET as PCRE2_UCHAR32
    {
        return 0 as ::core::ffi::c_int;
    }
    class_ptr = posix_classes;
    class_index = 1 as ::core::ffi::c_int;
    loop {
        if *class_ptr as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
            return 0 as ::core::ffi::c_int;
        }
        pattern = start;
        while *pattern == *class_ptr as PCRE2_UCHAR32 {
            if *pattern == CHAR_COLON as PCRE2_UCHAR32 {
                pattern = pattern.offset(2 as ::core::ffi::c_int as isize);
                start = start.offset(-(2 as ::core::ffi::c_int as isize));
                loop {
                    let fresh24 = start;
                    start = start.offset(1);
                    convert_glob_write(out, *fresh24);
                    if !(start < pattern) {
                        break;
                    }
                }
                *from = pattern;
                return class_index;
            }
            pattern = pattern.offset(1);
            class_ptr = class_ptr.offset(1);
        }
        while *class_ptr as ::core::ffi::c_int != CHAR_COLON {
            class_ptr = class_ptr.offset(1);
        }
        class_ptr = class_ptr.offset(1);
        class_index += 1;
    }
}
unsafe extern "C" fn convert_glob_char_in_class(
    mut class_index: ::core::ffi::c_int,
    mut c: PCRE2_UCHAR32,
) -> BOOL {
    let mut cbits: *const uint8_t =
        (&raw const _pcre2_default_tables_32 as *const uint8_t).offset(cbits_offset as isize);
    let mut cbit: ::core::ffi::c_int = 0;
    if c > 0xff as PCRE2_UCHAR32 {
        return FALSE;
    }
    match class_index {
        1 => {
            if c == CHAR_UNDERSCORE as PCRE2_UCHAR32 {
                return FALSE;
            }
            if *cbits
                .offset(cbit_digit as isize)
                .offset(c.wrapping_div(8 as PCRE2_UCHAR32) as isize)
                as ::core::ffi::c_uint
                & (1 as ::core::ffi::c_uint) << (c & 7 as PCRE2_UCHAR32)
                != 0 as ::core::ffi::c_uint
            {
                return FALSE;
            }
            cbit = cbit_word;
        }
        2 => {
            cbit = cbit_lower;
        }
        3 => {
            cbit = cbit_upper;
        }
        4 => {
            if c == CHAR_UNDERSCORE as PCRE2_UCHAR32 {
                return FALSE;
            }
            cbit = cbit_word;
        }
        5 => {
            if *cbits
                .offset(cbit_cntrl as isize)
                .offset(c.wrapping_div(8 as PCRE2_UCHAR32) as isize)
                as ::core::ffi::c_uint
                & (1 as ::core::ffi::c_uint) << (c & 7 as PCRE2_UCHAR32)
                != 0 as ::core::ffi::c_uint
            {
                return TRUE;
            }
            cbit = cbit_print;
        }
        6 => {
            if c == CHAR_LF as PCRE2_UCHAR32
                || c == CHAR_VT as PCRE2_UCHAR32
                || c == CHAR_FF as PCRE2_UCHAR32
                || c == CHAR_CR as PCRE2_UCHAR32
            {
                return FALSE;
            }
            cbit = cbit_space;
        }
        7 => {
            cbit = cbit_cntrl;
        }
        8 => {
            cbit = cbit_digit;
        }
        9 => {
            cbit = cbit_graph;
        }
        10 => {
            cbit = cbit_print;
        }
        11 => {
            cbit = cbit_punct;
        }
        12 => {
            cbit = cbit_space;
        }
        13 => {
            cbit = cbit_word;
        }
        14 => {
            cbit = cbit_xdigit;
        }
        _ => return FALSE,
    }
    return (*cbits
        .offset(cbit as isize)
        .offset(c.wrapping_div(8 as PCRE2_UCHAR32) as isize) as ::core::ffi::c_uint
        & (1 as ::core::ffi::c_uint) << (c & 7 as PCRE2_UCHAR32)
        != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int;
}
unsafe extern "C" fn convert_glob_parse_range(
    mut from: *mut PCRE2_SPTR32,
    mut pattern_end: PCRE2_SPTR32,
    mut out: *mut pcre2_output_context,
    mut utf: BOOL,
    mut separator: PCRE2_UCHAR32,
    mut with_escape: BOOL,
    mut escape: PCRE2_UCHAR32,
    mut no_wildsep: BOOL,
) -> ::core::ffi::c_int {
    let mut is_negative: BOOL = FALSE;
    let mut separator_seen: BOOL = FALSE;
    let mut has_prev_c: BOOL = 0;
    let mut pattern: PCRE2_SPTR32 = *from;
    let mut char_start: PCRE2_SPTR32 = ::core::ptr::null::<PCRE2_UCHAR32>();
    let mut c: uint32_t = 0;
    let mut prev_c: uint32_t = 0;
    let mut len: ::core::ffi::c_int = 0;
    let mut class_index: ::core::ffi::c_int = 0;
    if pattern >= pattern_end {
        *from = pattern;
        return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
    }
    if *pattern == CHAR_EXCLAMATION_MARK as PCRE2_UCHAR32
        || *pattern == CHAR_CIRCUMFLEX_ACCENT as PCRE2_UCHAR32
    {
        pattern = pattern.offset(1);
        if pattern >= pattern_end {
            *from = pattern;
            return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
        }
        is_negative = TRUE as BOOL;
        (*out).out_str[0 as ::core::ffi::c_int as usize] = CHAR_LEFT_SQUARE_BRACKET as uint8_t;
        (*out).out_str[1 as ::core::ffi::c_int as usize] = CHAR_CIRCUMFLEX_ACCENT as uint8_t;
        len = 2 as ::core::ffi::c_int;
        if no_wildsep == 0 {
            if with_escape != 0 {
                (*out).out_str[len as usize] = CHAR_BACKSLASH as uint8_t;
                len += 1;
            }
            (*out).out_str[len as usize] = separator as uint8_t;
        }
        convert_glob_write_str(out, (len + 1 as ::core::ffi::c_int) as size_t);
    } else {
        convert_glob_write(out, CHAR_LEFT_SQUARE_BRACKET as PCRE2_UCHAR32);
    }
    has_prev_c = FALSE as BOOL;
    prev_c = 0 as uint32_t;
    if *pattern == CHAR_RIGHT_SQUARE_BRACKET as PCRE2_UCHAR32 {
        (*out).out_str[0 as ::core::ffi::c_int as usize] = CHAR_BACKSLASH as uint8_t;
        (*out).out_str[1 as ::core::ffi::c_int as usize] = CHAR_RIGHT_SQUARE_BRACKET as uint8_t;
        convert_glob_write_str(out, 2 as size_t);
        has_prev_c = TRUE as BOOL;
        prev_c = CHAR_RIGHT_SQUARE_BRACKET as uint32_t;
        pattern = pattern.offset(1);
    }
    while pattern < pattern_end {
        char_start = pattern;
        let fresh18 = pattern;
        pattern = pattern.offset(1);
        c = *fresh18 as uint32_t;
        if c == CHAR_RIGHT_SQUARE_BRACKET as uint32_t {
            convert_glob_write(out, c as PCRE2_UCHAR32);
            if is_negative == 0 && no_wildsep == 0 && separator_seen != 0 {
                (*out).out_str[0 as ::core::ffi::c_int as usize] = CHAR_LEFT_PARENTHESIS as uint8_t;
                (*out).out_str[1 as ::core::ffi::c_int as usize] = CHAR_QUESTION_MARK as uint8_t;
                (*out).out_str[2 as ::core::ffi::c_int as usize] = CHAR_LESS_THAN_SIGN as uint8_t;
                (*out).out_str[3 as ::core::ffi::c_int as usize] = CHAR_EXCLAMATION_MARK as uint8_t;
                convert_glob_write_str(out, 4 as size_t);
                convert_glob_print_separator(out, separator, with_escape);
                convert_glob_write(out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR32);
            }
            *from = pattern;
            return 0 as ::core::ffi::c_int;
        }
        if pattern >= pattern_end {
            break;
        }
        if c == CHAR_LEFT_SQUARE_BRACKET as uint32_t && *pattern == CHAR_COLON as PCRE2_UCHAR32 {
            *from = pattern;
            class_index = convert_glob_parse_class(from, pattern_end, out);
            if class_index != 0 as ::core::ffi::c_int {
                pattern = *from;
                has_prev_c = FALSE as BOOL;
                prev_c = 0 as uint32_t;
                if is_negative == 0 && convert_glob_char_in_class(class_index, separator) != 0 {
                    separator_seen = TRUE as BOOL;
                }
                continue;
            }
        } else if c == CHAR_MINUS as uint32_t
            && has_prev_c != 0
            && *pattern != CHAR_RIGHT_SQUARE_BRACKET as PCRE2_UCHAR32
        {
            convert_glob_write(out, CHAR_MINUS as PCRE2_UCHAR32);
            char_start = pattern;
            let fresh19 = pattern;
            pattern = pattern.offset(1);
            c = *fresh19 as uint32_t;
            if pattern >= pattern_end {
                break;
            }
            if escape != 0 as PCRE2_UCHAR32 && c == escape {
                char_start = pattern;
                let fresh20 = pattern;
                pattern = pattern.offset(1);
                c = *fresh20 as uint32_t;
            } else if c == CHAR_LEFT_SQUARE_BRACKET as uint32_t
                && *pattern == CHAR_COLON as PCRE2_UCHAR32
            {
                *from = pattern;
                return PCRE2_ERROR_CONVERT_SYNTAX;
            }
            if prev_c > c {
                *from = pattern;
                return PCRE2_ERROR_CONVERT_SYNTAX;
            }
            if prev_c < separator && separator < c {
                separator_seen = TRUE as BOOL;
            }
            has_prev_c = FALSE as BOOL;
            prev_c = 0 as uint32_t;
        } else {
            if escape != 0 as PCRE2_UCHAR32 && c == escape {
                char_start = pattern;
                let fresh21 = pattern;
                pattern = pattern.offset(1);
                c = *fresh21 as uint32_t;
                if pattern >= pattern_end {
                    break;
                }
            }
            has_prev_c = TRUE as BOOL;
            prev_c = c;
        }
        if c == CHAR_LEFT_SQUARE_BRACKET as uint32_t
            || c == CHAR_RIGHT_SQUARE_BRACKET as uint32_t
            || c == CHAR_BACKSLASH as uint32_t
            || c == CHAR_MINUS as uint32_t
        {
            convert_glob_write(out, CHAR_BACKSLASH as PCRE2_UCHAR32);
        }
        if c == separator {
            separator_seen = TRUE as BOOL;
        }
        loop {
            let fresh22 = char_start;
            char_start = char_start.offset(1);
            convert_glob_write(out, *fresh22);
            if !(char_start < pattern) {
                break;
            }
        }
    }
    *from = pattern;
    return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
}
unsafe extern "C" fn convert_glob_print_commit(mut out: *mut pcre2_output_context) {
    (*out).out_str[0 as ::core::ffi::c_int as usize] = CHAR_LEFT_PARENTHESIS as uint8_t;
    (*out).out_str[1 as ::core::ffi::c_int as usize] = CHAR_ASTERISK as uint8_t;
    (*out).out_str[2 as ::core::ffi::c_int as usize] = CHAR_C as uint8_t;
    (*out).out_str[3 as ::core::ffi::c_int as usize] = CHAR_O as uint8_t;
    (*out).out_str[4 as ::core::ffi::c_int as usize] = CHAR_M as uint8_t;
    (*out).out_str[5 as ::core::ffi::c_int as usize] = CHAR_M as uint8_t;
    (*out).out_str[6 as ::core::ffi::c_int as usize] = CHAR_I as uint8_t;
    (*out).out_str[7 as ::core::ffi::c_int as usize] = CHAR_T as uint8_t;
    convert_glob_write_str(out, 8 as size_t);
    convert_glob_write(out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR32);
}
unsafe extern "C" fn convert_glob(
    mut options: uint32_t,
    mut pattern: PCRE2_SPTR32,
    mut plength: size_t,
    mut utf: BOOL,
    mut use_buffer: *mut PCRE2_UCHAR32,
    mut use_length: size_t,
    mut bufflenptr: *mut size_t,
    mut dummyrun: BOOL,
    mut ccontext: *mut pcre2_convert_context_32,
) -> ::core::ffi::c_int {
    let mut out: pcre2_output_context = pcre2_output_context {
        output: ::core::ptr::null_mut::<PCRE2_UCHAR32>(),
        output_end: ::core::ptr::null::<PCRE2_UCHAR32>(),
        output_size: 0,
        out_str: [0; 8],
    };
    let mut pattern_start: PCRE2_SPTR32 = pattern;
    let mut pattern_end: PCRE2_SPTR32 = pattern.offset(plength as isize);
    let mut separator: PCRE2_UCHAR32 = (*ccontext).glob_separator as PCRE2_UCHAR32;
    let mut escape: PCRE2_UCHAR32 = (*ccontext).glob_escape as PCRE2_UCHAR32;
    let mut c: PCRE2_UCHAR32 = 0;
    let mut no_wildsep: BOOL = (options & PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR as uint32_t
        != 0 as uint32_t) as ::core::ffi::c_int;
    let mut no_starstar: BOOL = (options & PCRE2_CONVERT_GLOB_NO_STARSTAR as uint32_t
        != 0 as uint32_t) as ::core::ffi::c_int;
    let mut in_atomic: BOOL = FALSE;
    let mut after_starstar: BOOL = FALSE;
    let mut no_slash_z: BOOL = FALSE;
    let mut with_escape: BOOL = 0;
    let mut is_start: BOOL = 0;
    let mut after_separator: BOOL = 0;
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if utf != 0 && (separator >= 128 as PCRE2_UCHAR32 || escape >= 128 as PCRE2_UCHAR32) {
        *bufflenptr = 0 as size_t;
        return PCRE2_ERROR_CONVERT_SYNTAX;
    }
    with_escape = (strchr(pcre2_escaped_literals, separator as ::core::ffi::c_int)
        as *const ::core::ffi::c_char
        != NULL as *const ::core::ffi::c_char) as ::core::ffi::c_int as BOOL;
    out.output = use_buffer;
    out.output_end = use_buffer.offset(use_length as isize) as PCRE2_SPTR32;
    out.output_size = 0 as size_t;
    out.out_str[0 as ::core::ffi::c_int as usize] = CHAR_LEFT_PARENTHESIS as uint8_t;
    out.out_str[1 as ::core::ffi::c_int as usize] = CHAR_QUESTION_MARK as uint8_t;
    out.out_str[2 as ::core::ffi::c_int as usize] = CHAR_s as uint8_t;
    out.out_str[3 as ::core::ffi::c_int as usize] = CHAR_RIGHT_PARENTHESIS as uint8_t;
    convert_glob_write_str(&raw mut out, 4 as size_t);
    is_start = TRUE as BOOL;
    if pattern < pattern_end
        && *pattern.offset(0 as ::core::ffi::c_int as isize) == CHAR_ASTERISK as PCRE2_UCHAR32
    {
        if no_wildsep != 0 {
            is_start = FALSE as BOOL;
        } else if no_starstar == 0
            && pattern.offset(1 as ::core::ffi::c_int as isize) < pattern_end
            && *pattern.offset(1 as ::core::ffi::c_int as isize) == CHAR_ASTERISK as PCRE2_UCHAR32
        {
            is_start = FALSE as BOOL;
        }
    }
    if is_start != 0 {
        out.out_str[0 as ::core::ffi::c_int as usize] = CHAR_BACKSLASH as uint8_t;
        out.out_str[1 as ::core::ffi::c_int as usize] = CHAR_A as uint8_t;
        convert_glob_write_str(&raw mut out, 2 as size_t);
    }
    while pattern < pattern_end {
        let fresh13 = pattern;
        pattern = pattern.offset(1);
        c = *fresh13;
        if c == CHAR_ASTERISK as PCRE2_UCHAR32 {
            is_start = (pattern == pattern_start.offset(1 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_int as BOOL;
            if in_atomic != 0 {
                convert_glob_write(&raw mut out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR32);
                in_atomic = FALSE as BOOL;
            }
            if no_starstar == 0
                && pattern < pattern_end
                && *pattern == CHAR_ASTERISK as PCRE2_UCHAR32
            {
                after_separator = (is_start != 0
                    || *pattern.offset(-(2 as ::core::ffi::c_int) as isize) == separator)
                    as ::core::ffi::c_int as BOOL;
                loop {
                    pattern = pattern.offset(1);
                    if !(pattern < pattern_end && *pattern == CHAR_ASTERISK as PCRE2_UCHAR32) {
                        break;
                    }
                }
                if pattern >= pattern_end {
                    no_slash_z = TRUE as BOOL;
                    break;
                } else {
                    after_starstar = TRUE as BOOL;
                    if after_separator != 0
                        && escape != 0 as PCRE2_UCHAR32
                        && *pattern == escape
                        && pattern.offset(1 as ::core::ffi::c_int as isize) < pattern_end
                        && *pattern.offset(1 as ::core::ffi::c_int as isize) == separator
                    {
                        pattern = pattern.offset(1);
                    }
                    if is_start != 0 {
                        if *pattern != separator {
                            continue;
                        }
                        out.out_str[0 as ::core::ffi::c_int as usize] =
                            CHAR_LEFT_PARENTHESIS as uint8_t;
                        out.out_str[1 as ::core::ffi::c_int as usize] =
                            CHAR_QUESTION_MARK as uint8_t;
                        out.out_str[2 as ::core::ffi::c_int as usize] = CHAR_COLON as uint8_t;
                        out.out_str[3 as ::core::ffi::c_int as usize] = CHAR_BACKSLASH as uint8_t;
                        out.out_str[4 as ::core::ffi::c_int as usize] = CHAR_A as uint8_t;
                        out.out_str[5 as ::core::ffi::c_int as usize] =
                            CHAR_VERTICAL_LINE as uint8_t;
                        convert_glob_write_str(&raw mut out, 6 as size_t);
                        convert_glob_print_separator(&raw mut out, separator, with_escape);
                        convert_glob_write(&raw mut out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR32);
                        pattern = pattern.offset(1);
                    } else {
                        convert_glob_print_commit(&raw mut out);
                        if after_separator == 0 || *pattern != separator {
                            out.out_str[0 as ::core::ffi::c_int as usize] = CHAR_DOT as uint8_t;
                            out.out_str[1 as ::core::ffi::c_int as usize] =
                                CHAR_ASTERISK as uint8_t;
                            out.out_str[2 as ::core::ffi::c_int as usize] =
                                CHAR_QUESTION_MARK as uint8_t;
                            convert_glob_write_str(&raw mut out, 3 as size_t);
                        } else {
                            out.out_str[0 as ::core::ffi::c_int as usize] =
                                CHAR_LEFT_PARENTHESIS as uint8_t;
                            out.out_str[1 as ::core::ffi::c_int as usize] =
                                CHAR_QUESTION_MARK as uint8_t;
                            out.out_str[2 as ::core::ffi::c_int as usize] = CHAR_COLON as uint8_t;
                            out.out_str[3 as ::core::ffi::c_int as usize] = CHAR_DOT as uint8_t;
                            out.out_str[4 as ::core::ffi::c_int as usize] =
                                CHAR_ASTERISK as uint8_t;
                            out.out_str[5 as ::core::ffi::c_int as usize] =
                                CHAR_QUESTION_MARK as uint8_t;
                            convert_glob_write_str(&raw mut out, 6 as size_t);
                            convert_glob_print_separator(&raw mut out, separator, with_escape);
                            out.out_str[0 as ::core::ffi::c_int as usize] =
                                CHAR_RIGHT_PARENTHESIS as uint8_t;
                            out.out_str[1 as ::core::ffi::c_int as usize] =
                                CHAR_QUESTION_MARK as uint8_t;
                            out.out_str[2 as ::core::ffi::c_int as usize] =
                                CHAR_QUESTION_MARK as uint8_t;
                            convert_glob_write_str(&raw mut out, 3 as size_t);
                            pattern = pattern.offset(1);
                        }
                    }
                }
            } else {
                if pattern < pattern_end && *pattern == CHAR_ASTERISK as PCRE2_UCHAR32 {
                    loop {
                        pattern = pattern.offset(1);
                        if !(pattern < pattern_end && *pattern == CHAR_ASTERISK as PCRE2_UCHAR32) {
                            break;
                        }
                    }
                }
                if no_wildsep != 0 {
                    if pattern >= pattern_end {
                        no_slash_z = TRUE as BOOL;
                        break;
                    } else if is_start != 0 {
                        continue;
                    }
                }
                if is_start == 0 {
                    if after_starstar != 0 {
                        out.out_str[0 as ::core::ffi::c_int as usize] =
                            CHAR_LEFT_PARENTHESIS as uint8_t;
                        out.out_str[1 as ::core::ffi::c_int as usize] =
                            CHAR_QUESTION_MARK as uint8_t;
                        out.out_str[2 as ::core::ffi::c_int as usize] =
                            CHAR_GREATER_THAN_SIGN as uint8_t;
                        convert_glob_write_str(&raw mut out, 3 as size_t);
                        in_atomic = TRUE as BOOL;
                    } else {
                        convert_glob_print_commit(&raw mut out);
                    }
                }
                if no_wildsep != 0 {
                    convert_glob_write(&raw mut out, CHAR_DOT as PCRE2_UCHAR32);
                } else {
                    convert_glob_print_wildcard(&raw mut out, separator, with_escape);
                }
                out.out_str[0 as ::core::ffi::c_int as usize] = CHAR_ASTERISK as uint8_t;
                out.out_str[1 as ::core::ffi::c_int as usize] = CHAR_QUESTION_MARK as uint8_t;
                if pattern >= pattern_end {
                    out.out_str[1 as ::core::ffi::c_int as usize] = CHAR_PLUS as uint8_t;
                }
                convert_glob_write_str(&raw mut out, 2 as size_t);
            }
        } else if c == CHAR_QUESTION_MARK as PCRE2_UCHAR32 {
            if no_wildsep != 0 {
                convert_glob_write(&raw mut out, CHAR_DOT as PCRE2_UCHAR32);
            } else {
                convert_glob_print_wildcard(&raw mut out, separator, with_escape);
            }
        } else if c == CHAR_LEFT_SQUARE_BRACKET as PCRE2_UCHAR32 {
            result = convert_glob_parse_range(
                &raw mut pattern,
                pattern_end,
                &raw mut out,
                utf,
                separator,
                with_escape,
                escape,
                no_wildsep,
            );
            if result != 0 as ::core::ffi::c_int {
                break;
            }
        } else {
            if escape != 0 as PCRE2_UCHAR32 && c == escape {
                if pattern >= pattern_end {
                    result = PCRE2_ERROR_CONVERT_SYNTAX;
                    break;
                } else {
                    let fresh14 = pattern;
                    pattern = pattern.offset(1);
                    c = *fresh14;
                }
            }
            if c < 255 as PCRE2_UCHAR32
                && !(strchr(pcre2_escaped_literals, c as ::core::ffi::c_int)
                    as *const ::core::ffi::c_char)
                    .is_null()
            {
                convert_glob_write(&raw mut out, CHAR_BACKSLASH as PCRE2_UCHAR32);
            }
            convert_glob_write(&raw mut out, c);
        }
    }
    if result == 0 as ::core::ffi::c_int {
        if no_slash_z == 0 {
            out.out_str[0 as ::core::ffi::c_int as usize] = CHAR_BACKSLASH as uint8_t;
            out.out_str[1 as ::core::ffi::c_int as usize] = CHAR_z as uint8_t;
            convert_glob_write_str(&raw mut out, 2 as size_t);
        }
        if in_atomic != 0 {
            convert_glob_write(&raw mut out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR32);
        }
        convert_glob_write(&raw mut out, CHAR_NUL as PCRE2_UCHAR32);
        if dummyrun == 0
            && out.output_size
                != out.output.offset_from(use_buffer) as isize as size_t
        {
            result = PCRE2_ERROR_NOMEMORY;
        }
    }
    if result != 0 as ::core::ffi::c_int {
        *bufflenptr = pattern.offset_from(pattern_start) as isize as size_t;
        return result;
    }
    *bufflenptr = out.output_size.wrapping_sub(1 as size_t);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_pattern_convert_32(
    mut pattern: PCRE2_SPTR32,
    mut plength: size_t,
    mut options: uint32_t,
    mut buffptr: *mut *mut PCRE2_UCHAR32,
    mut bufflenptr: *mut size_t,
    mut ccontext: *mut pcre2_convert_context_32,
) -> ::core::ffi::c_int {
    let mut rc: ::core::ffi::c_int = 0;
    let mut null_str: [PCRE2_UCHAR32; 1] = [0xcd as ::core::ffi::c_int as PCRE2_UCHAR32];
    let mut dummy_buffer: [PCRE2_UCHAR32; 100] = [0; 100];
    let mut use_buffer: *mut PCRE2_UCHAR32 = &raw mut dummy_buffer as *mut PCRE2_UCHAR32;
    let mut use_length: size_t = DUMMY_BUFFER_SIZE as size_t;
    let mut utf: BOOL =
        (options & PCRE2_CONVERT_UTF as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    let mut pattype: uint32_t = options & TYPE_OPTIONS as uint32_t;
    if pattern.is_null() && plength == 0 as size_t {
        pattern = &raw mut null_str as *mut PCRE2_UCHAR32 as PCRE2_SPTR32;
    }
    if pattern.is_null() || bufflenptr.is_null() {
        if !bufflenptr.is_null() {
            *bufflenptr = 0 as size_t;
        }
        return PCRE2_ERROR_NULL;
    }
    if options & !(ALL_OPTIONS as uint32_t) != 0 as uint32_t
        || pattype & (!pattype).wrapping_add(1 as uint32_t) != pattype
        || pattype == 0 as uint32_t
    {
        *bufflenptr = 0 as size_t;
        return PCRE2_ERROR_BADOPTION;
    }
    if plength == PCRE2_ZERO_TERMINATED {
        plength = _pcre2_strlen_32(pattern);
    }
    if ccontext.is_null() {
        ccontext = &raw mut _pcre2_default_convert_context_32;
    }
    if utf != 0 && options & PCRE2_CONVERT_NO_UTF_CHECK as uint32_t == 0 as uint32_t {
        let mut erroroffset: size_t = 0;
        rc = _pcre2_valid_utf_32(pattern, plength, &raw mut erroroffset);
        if rc != 0 as ::core::ffi::c_int {
            *bufflenptr = erroroffset;
            return rc;
        }
    }
    if !buffptr.is_null() && !(*buffptr).is_null() {
        use_buffer = *buffptr;
        use_length = *bufflenptr;
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 2 as ::core::ffi::c_int {
        let mut allocated: *mut PCRE2_UCHAR32 = ::core::ptr::null_mut::<PCRE2_UCHAR32>();
        let mut dummyrun: BOOL = (buffptr.is_null() || (*buffptr).is_null()) as ::core::ffi::c_int;
        match pattype {
            PCRE2_CONVERT_GLOB => {
                rc = convert_glob(
                    options & !(PCRE2_CONVERT_GLOB as uint32_t),
                    pattern,
                    plength,
                    utf,
                    use_buffer,
                    use_length,
                    bufflenptr,
                    dummyrun,
                    ccontext,
                );
            }
            PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_POSIX_EXTENDED => {
                rc = convert_posix(
                    pattype, pattern, plength, utf, use_buffer, use_length, bufflenptr, dummyrun,
                    ccontext,
                );
            }
            _ => {
                *bufflenptr = 0 as size_t;
                return PCRE2_ERROR_INTERNAL;
            }
        }
        if rc != 0 as ::core::ffi::c_int || buffptr.is_null() || !(*buffptr).is_null() {
            return rc;
        }
        if *bufflenptr
            > (PCRE2_SIZE_MAX as usize)
                .wrapping_sub(::core::mem::size_of::<pcre2_memctl>() as usize)
                .wrapping_div(
                    (1 as ::core::ffi::c_int * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int))
                        as usize,
                )
                .wrapping_sub(1 as usize)
            || {
                allocated = _pcre2_memctl_malloc_32(
                    (::core::mem::size_of::<pcre2_memctl>() as size_t).wrapping_add(
                        (*bufflenptr).wrapping_add(1 as size_t).wrapping_mul(
                            (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t,
                        ),
                    ),
                    ccontext as *mut pcre2_memctl,
                ) as *mut PCRE2_UCHAR32;
                allocated.is_null()
            }
        {
            *bufflenptr = 0 as size_t;
            return PCRE2_ERROR_NOMEMORY;
        }
        *buffptr = (allocated as *mut ::core::ffi::c_char)
            .offset(::core::mem::size_of::<pcre2_memctl>() as usize as isize)
            as *mut PCRE2_UCHAR32;
        use_buffer = *buffptr;
        use_length = (*bufflenptr).wrapping_add(1 as size_t);
        i += 1;
    }
    *bufflenptr = 0 as size_t;
    return PCRE2_ERROR_INTERNAL;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_converted_pattern_free_32(mut converted: *mut PCRE2_UCHAR32) {
    if !converted.is_null() {
        let mut memctl: *mut pcre2_memctl = (converted as *mut ::core::ffi::c_char)
            .offset(-(::core::mem::size_of::<pcre2_memctl>() as usize as isize))
            as *mut pcre2_memctl;
        (*memctl).free.expect("non-null function pointer")(
            memctl as *mut ::core::ffi::c_void,
            (*memctl).memory_data,
        );
    }
}
pub const PCRE2_CODE_UNIT_WIDTH: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
