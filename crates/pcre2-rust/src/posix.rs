use crate::bindings::{
    pcre2_real_general_context_8, pcre2_real_compile_context_8,
    pcre2_real_match_context_8, pcre2_real_code_8, pcre2_real_match_data_8,
};
extern "C" {
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn pcre2_match_8(
        _: *const pcre2_code_8,
        _: PCRE2_SPTR8,
        _: size_t,
        _: size_t,
        _: uint32_t,
        _: *mut pcre2_match_data_8,
        _: *mut pcre2_match_context_8,
    ) -> ::core::ffi::c_int;
    fn pcre2_match_data_create_8(
        _: uint32_t,
        _: *mut pcre2_general_context_8,
    ) -> *mut pcre2_match_data_8;
    fn pcre2_match_data_free_8(_: *mut pcre2_match_data_8);
    fn pcre2_get_ovector_pointer_8(_: *mut pcre2_match_data_8) -> *mut size_t;
    fn pcre2_compile_8(
        _: PCRE2_SPTR8,
        _: size_t,
        _: uint32_t,
        _: *mut ::core::ffi::c_int,
        _: *mut size_t,
        _: *mut pcre2_compile_context_8,
    ) -> *mut pcre2_code_8;
    fn pcre2_code_free_8(_: *mut pcre2_code_8);
    fn pcre2_pattern_info_8(
        _: *const pcre2_code_8,
        _: uint32_t,
        _: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
}
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type size_t = usize;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR8 = uint8_t;
pub type PCRE2_SPTR8 = *const PCRE2_UCHAR8;
pub type pcre2_general_context_8 = pcre2_real_general_context_8;
pub type pcre2_compile_context_8 = pcre2_real_compile_context_8;
pub type pcre2_match_context_8 = pcre2_real_match_context_8;
pub type pcre2_code_8 = pcre2_real_code_8;
pub type pcre2_match_data_8 = pcre2_real_match_data_8;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const REG_NOMATCH: C2RustUnnamed = 17;
pub const REG_INVARG: C2RustUnnamed = 16;
pub const REG_ESUBREG: C2RustUnnamed = 15;
pub const REG_ESPACE: C2RustUnnamed = 14;
pub const REG_ESIZE: C2RustUnnamed = 13;
pub const REG_ERANGE: C2RustUnnamed = 12;
pub const REG_EPAREN: C2RustUnnamed = 11;
pub const REG_EMPTY: C2RustUnnamed = 10;
pub const REG_EESCAPE: C2RustUnnamed = 9;
pub const REG_ECTYPE: C2RustUnnamed = 8;
pub const REG_ECOLLATE: C2RustUnnamed = 7;
pub const REG_EBRACK: C2RustUnnamed = 6;
pub const REG_EBRACE: C2RustUnnamed = 5;
pub const REG_BADRPT: C2RustUnnamed = 4;
pub const REG_BADPAT: C2RustUnnamed = 3;
pub const REG_BADBR: C2RustUnnamed = 2;
pub const REG_ASSERT: C2RustUnnamed = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct regex_t {
    pub re_pcre2_code: *mut ::core::ffi::c_void,
    pub re_match_data: *mut ::core::ffi::c_void,
    pub re_endp: *const ::core::ffi::c_char,
    pub re_nsub: size_t,
    pub re_erroffset: size_t,
    pub re_cflags: ::core::ffi::c_int,
}
pub type regoff_t = ::core::ffi::c_int;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct regmatch_t {
    pub rm_so: regoff_t,
    pub rm_eo: regoff_t,
}
pub const COMPILE_ERROR_BASE: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const PCRE2_CASELESS: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PCRE2_DOTALL: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PCRE2_MULTILINE: ::core::ffi::c_uint = 0x400 as ::core::ffi::c_uint;
pub const PCRE2_UCP: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PCRE2_UNGREEDY: ::core::ffi::c_uint = 0x40000 as ::core::ffi::c_uint;
pub const PCRE2_UTF: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PCRE2_LITERAL: ::core::ffi::c_uint = 0x2000000 as ::core::ffi::c_uint;
pub const PCRE2_NOTBOL: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PCRE2_NOTEOL: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PCRE2_NOTEMPTY: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PCRE2_ERROR_NOMATCH: ::core::ffi::c_int = -1;
pub const PCRE2_ERROR_UTF8_ERR1: ::core::ffi::c_int = -(3 as ::core::ffi::c_int);
pub const PCRE2_ERROR_UTF8_ERR21: ::core::ffi::c_int = -(23 as ::core::ffi::c_int);
pub const PCRE2_ERROR_BADMAGIC: ::core::ffi::c_int = -31;
pub const PCRE2_ERROR_BADMODE: ::core::ffi::c_int = -32;
pub const PCRE2_ERROR_BADOPTION: ::core::ffi::c_int = -34;
pub const PCRE2_ERROR_BADUTFOFFSET: ::core::ffi::c_int = -36;
pub const PCRE2_ERROR_MATCHLIMIT: ::core::ffi::c_int = -47;
pub const PCRE2_ERROR_NOMEMORY: ::core::ffi::c_int = -48;
pub const PCRE2_ERROR_NULL: ::core::ffi::c_int = -51;
pub const PCRE2_ERROR_HEAPLIMIT: ::core::ffi::c_int = -63;
pub const PCRE2_INFO_CAPTURECOUNT: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PCRE2_ZERO_TERMINATED: size_t = !(0 as ::core::ffi::c_int as size_t);
pub const PCRE2_UNSET: size_t = !(0 as ::core::ffi::c_int as size_t);
pub const REG_ICASE: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const REG_NEWLINE: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const REG_NOTBOL: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const REG_NOTEOL: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const REG_DOTALL: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const REG_NOSUB: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
pub const REG_UTF: ::core::ffi::c_int = 0x40 as ::core::ffi::c_int;
pub const REG_STARTEND: ::core::ffi::c_int = 0x80 as ::core::ffi::c_int;
pub const REG_NOTEMPTY: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const REG_UNGREEDY: ::core::ffi::c_int = 0x200 as ::core::ffi::c_int;
pub const REG_UCP: ::core::ffi::c_int = 0x400 as ::core::ffi::c_int;
pub const REG_PEND: ::core::ffi::c_int = 0x800 as ::core::ffi::c_int;
pub const REG_NOSPEC: ::core::ffi::c_int = 0x1000 as ::core::ffi::c_int;
static mut eint1: [::core::ffi::c_int; 24] = [
    0 as ::core::ffi::c_int,
    REG_EESCAPE as ::core::ffi::c_int,
    REG_EESCAPE as ::core::ffi::c_int,
    REG_EESCAPE as ::core::ffi::c_int,
    REG_BADBR as ::core::ffi::c_int,
    REG_BADBR as ::core::ffi::c_int,
    REG_EBRACK as ::core::ffi::c_int,
    REG_ECTYPE as ::core::ffi::c_int,
    REG_ERANGE as ::core::ffi::c_int,
    REG_BADRPT as ::core::ffi::c_int,
    REG_ASSERT as ::core::ffi::c_int,
    REG_BADPAT as ::core::ffi::c_int,
    REG_BADPAT as ::core::ffi::c_int,
    REG_BADPAT as ::core::ffi::c_int,
    REG_EPAREN as ::core::ffi::c_int,
    REG_ESUBREG as ::core::ffi::c_int,
    REG_INVARG as ::core::ffi::c_int,
    REG_INVARG as ::core::ffi::c_int,
    REG_EPAREN as ::core::ffi::c_int,
    REG_ESIZE as ::core::ffi::c_int,
    REG_ESIZE as ::core::ffi::c_int,
    REG_ESPACE as ::core::ffi::c_int,
    REG_EPAREN as ::core::ffi::c_int,
    REG_ASSERT as ::core::ffi::c_int,
];
static mut eint2: [::core::ffi::c_int; 16] = [
    30 as ::core::ffi::c_int,
    REG_ECTYPE as ::core::ffi::c_int,
    32 as ::core::ffi::c_int,
    REG_INVARG as ::core::ffi::c_int,
    37 as ::core::ffi::c_int,
    REG_EESCAPE as ::core::ffi::c_int,
    56 as ::core::ffi::c_int,
    REG_INVARG as ::core::ffi::c_int,
    92 as ::core::ffi::c_int,
    REG_INVARG as ::core::ffi::c_int,
    98 as ::core::ffi::c_int,
    REG_EESCAPE as ::core::ffi::c_int,
    99 as ::core::ffi::c_int,
    REG_EESCAPE as ::core::ffi::c_int,
    102 as ::core::ffi::c_int,
    REG_EESCAPE as ::core::ffi::c_int,
];
static mut pstring: [*const ::core::ffi::c_char; 18] = [
    b"\0" as *const u8 as *const ::core::ffi::c_char,
    b"internal error\0" as *const u8 as *const ::core::ffi::c_char,
    b"invalid repeat counts in {}\0" as *const u8 as *const ::core::ffi::c_char,
    b"pattern error\0" as *const u8 as *const ::core::ffi::c_char,
    b"? * + invalid\0" as *const u8 as *const ::core::ffi::c_char,
    b"unbalanced {}\0" as *const u8 as *const ::core::ffi::c_char,
    b"unbalanced []\0" as *const u8 as *const ::core::ffi::c_char,
    b"collation error - not relevant\0" as *const u8 as *const ::core::ffi::c_char,
    b"bad class\0" as *const u8 as *const ::core::ffi::c_char,
    b"bad escape sequence\0" as *const u8 as *const ::core::ffi::c_char,
    b"empty expression\0" as *const u8 as *const ::core::ffi::c_char,
    b"unbalanced ()\0" as *const u8 as *const ::core::ffi::c_char,
    b"bad range inside []\0" as *const u8 as *const ::core::ffi::c_char,
    b"expression too big\0" as *const u8 as *const ::core::ffi::c_char,
    b"failed to get memory\0" as *const u8 as *const ::core::ffi::c_char,
    b"bad back reference\0" as *const u8 as *const ::core::ffi::c_char,
    b"bad argument\0" as *const u8 as *const ::core::ffi::c_char,
    b"match failed\0" as *const u8 as *const ::core::ffi::c_char,
];
#[no_mangle]
pub unsafe extern "C" fn pcre2_regerror(
    mut errcode: ::core::ffi::c_int,
    mut preg: *const regex_t,
    mut errbuf: *mut ::core::ffi::c_char,
    mut errbuf_size: size_t,
) -> size_t {
    let mut message: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut offset_buf: [::core::ffi::c_char; 23] = [0; 23];
    let mut snprintf_rc: ::core::ffi::c_int = 0;
    let mut have_offset: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: size_t = 0;
    message = if errcode <= 0 as ::core::ffi::c_int
        || errcode
            >= (::core::mem::size_of::<[*const ::core::ffi::c_char; 18]>() as usize)
                .wrapping_div(::core::mem::size_of::<*mut ::core::ffi::c_char>() as usize)
                as ::core::ffi::c_int
    {
        b"unknown error code\0" as *const u8 as *const ::core::ffi::c_char
    } else {
        pstring[errcode as usize]
    };
    if !preg.is_null()
        && (*preg).re_erroffset != -(1 as ::core::ffi::c_int) as size_t
        && {
            snprintf_rc = snprintf(
                &raw mut offset_buf as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 23]>() as size_t,
                b" at offset %d\0" as *const u8 as *const ::core::ffi::c_char,
                (*preg).re_erroffset as ::core::ffi::c_int,
            );
            snprintf_rc > 0 as ::core::ffi::c_int
        }
        && snprintf_rc < ::core::mem::size_of::<[::core::ffi::c_char; 23]>() as ::core::ffi::c_int
    {
        have_offset = 1 as ::core::ffi::c_int;
        offset_buf[(::core::mem::size_of::<[::core::ffi::c_char; 23]>() as usize)
            .wrapping_sub(1 as usize) as usize] = 0 as ::core::ffi::c_char;
    }
    i = 0 as size_t;
    while *message as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        if i.wrapping_add(1 as size_t) < errbuf_size {
            *errbuf.offset(i as isize) = *message;
        }
        i = i.wrapping_add(1);
        message = message.offset(1);
    }
    if have_offset != 0 {
        message = &raw mut offset_buf as *mut ::core::ffi::c_char;
        while *message as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
            if i.wrapping_add(1 as size_t) < errbuf_size {
                *errbuf.offset(i as isize) = *message;
            }
            i = i.wrapping_add(1);
            message = message.offset(1);
        }
    }
    if errbuf_size > 0 as size_t {
        *errbuf.offset(
            (if i < errbuf_size {
                i
            } else {
                errbuf_size.wrapping_sub(1 as size_t)
            }) as isize,
        ) = 0 as ::core::ffi::c_char;
    }
    i = i.wrapping_add(1);
    return i as ::core::ffi::c_int as size_t;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_regfree(mut preg: *mut regex_t) {
    pcre2_match_data_free_8((*preg).re_match_data as *mut pcre2_match_data_8);
    pcre2_code_free_8((*preg).re_pcre2_code as *mut pcre2_code_8);
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_regcomp(
    mut preg: *mut regex_t,
    mut pattern: *const ::core::ffi::c_char,
    mut cflags: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut erroffset: size_t = 0;
    let mut patlen: size_t = 0;
    let mut errorcode: ::core::ffi::c_int = 0;
    let mut options: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut re_nsub: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    (*preg).re_match_data = NULL;
    (*preg).re_pcre2_code = NULL;
    patlen = if cflags & REG_PEND != 0 as ::core::ffi::c_int {
        (*preg).re_endp.offset_from(pattern) as size_t
    } else {
        PCRE2_ZERO_TERMINATED
    };
    if cflags & REG_ICASE != 0 as ::core::ffi::c_int {
        options = (options as ::core::ffi::c_uint | PCRE2_CASELESS) as ::core::ffi::c_int;
    }
    if cflags & REG_NEWLINE != 0 as ::core::ffi::c_int {
        options = (options as ::core::ffi::c_uint | PCRE2_MULTILINE) as ::core::ffi::c_int;
    }
    if cflags & REG_DOTALL != 0 as ::core::ffi::c_int {
        options = (options as ::core::ffi::c_uint | PCRE2_DOTALL) as ::core::ffi::c_int;
    }
    if cflags & REG_NOSPEC != 0 as ::core::ffi::c_int {
        options = (options as ::core::ffi::c_uint | PCRE2_LITERAL) as ::core::ffi::c_int;
    }
    if cflags & REG_UTF != 0 as ::core::ffi::c_int {
        options = (options as ::core::ffi::c_uint | PCRE2_UTF) as ::core::ffi::c_int;
    }
    if cflags & REG_UCP != 0 as ::core::ffi::c_int {
        options = (options as ::core::ffi::c_uint | PCRE2_UCP) as ::core::ffi::c_int;
    }
    if cflags & REG_UNGREEDY != 0 as ::core::ffi::c_int {
        options = (options as ::core::ffi::c_uint | PCRE2_UNGREEDY) as ::core::ffi::c_int;
    }
    (*preg).re_cflags = cflags;
    (*preg).re_pcre2_code = pcre2_compile_8(
        pattern as PCRE2_SPTR8,
        patlen,
        options as uint32_t,
        &raw mut errorcode,
        &raw mut erroffset,
        ::core::ptr::null_mut::<pcre2_compile_context_8>(),
    ) as *mut ::core::ffi::c_void;
    (*preg).re_erroffset = erroffset;
    if (*preg).re_pcre2_code.is_null() {
        let mut i: ::core::ffi::c_uint = 0;
        if errorcode < COMPILE_ERROR_BASE {
            return REG_BADPAT as ::core::ffi::c_int;
        }
        errorcode -= COMPILE_ERROR_BASE;
        if errorcode
            < (::core::mem::size_of::<[::core::ffi::c_int; 24]>() as usize)
                .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as usize)
                as ::core::ffi::c_int
        {
            return eint1[errorcode as usize];
        }
        i = 0 as ::core::ffi::c_uint;
        while (i as usize)
            < (::core::mem::size_of::<[::core::ffi::c_int; 16]>() as usize)
                .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as usize)
        {
            if errorcode == eint2[i as usize] {
                return eint2[i.wrapping_add(1 as ::core::ffi::c_uint) as usize];
            }
            i = i.wrapping_add(2 as ::core::ffi::c_uint);
        }
        return REG_BADPAT as ::core::ffi::c_int;
    }
    pcre2_pattern_info_8(
        (*preg).re_pcre2_code as *const pcre2_code_8,
        PCRE2_INFO_CAPTURECOUNT as uint32_t,
        &raw mut re_nsub as *mut ::core::ffi::c_void,
    );
    (*preg).re_nsub = re_nsub as size_t;
    (*preg).re_match_data = pcre2_match_data_create_8(
        (re_nsub + 1 as ::core::ffi::c_int) as uint32_t,
        ::core::ptr::null_mut::<pcre2_general_context_8>(),
    ) as *mut ::core::ffi::c_void;
    (*preg).re_erroffset = -(1 as ::core::ffi::c_int) as size_t;
    if (*preg).re_match_data.is_null() {
        pcre2_code_free_8((*preg).re_pcre2_code as *mut pcre2_code_8);
        (*preg).re_pcre2_code = NULL;
        return REG_ESPACE as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn pcre2_regexec(
    mut preg: *const regex_t,
    mut string: *const ::core::ffi::c_char,
    mut nmatch: size_t,
    mut pmatch: *mut regmatch_t,
    mut eflags: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut rc: ::core::ffi::c_int = 0;
    let mut so: ::core::ffi::c_int = 0;
    let mut eo: ::core::ffi::c_int = 0;
    let mut options: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut md: *mut pcre2_match_data_8 = (*preg).re_match_data as *mut pcre2_match_data_8;
    if string.is_null() {
        return REG_INVARG as ::core::ffi::c_int;
    }
    if eflags & REG_NOTBOL != 0 as ::core::ffi::c_int {
        options = (options as ::core::ffi::c_uint | PCRE2_NOTBOL) as ::core::ffi::c_int;
    }
    if eflags & REG_NOTEOL != 0 as ::core::ffi::c_int {
        options = (options as ::core::ffi::c_uint | PCRE2_NOTEOL) as ::core::ffi::c_int;
    }
    if eflags & REG_NOTEMPTY != 0 as ::core::ffi::c_int {
        options = (options as ::core::ffi::c_uint | PCRE2_NOTEMPTY) as ::core::ffi::c_int;
    }
    if (*preg).re_cflags & REG_NOSUB != 0 as ::core::ffi::c_int || pmatch.is_null() {
        nmatch = 0 as size_t;
    }
    if eflags & REG_STARTEND != 0 as ::core::ffi::c_int {
        if pmatch.is_null() {
            return REG_INVARG as ::core::ffi::c_int;
        }
        so = (*pmatch.offset(0 as ::core::ffi::c_int as isize)).rm_so as ::core::ffi::c_int;
        eo = (*pmatch.offset(0 as ::core::ffi::c_int as isize)).rm_eo as ::core::ffi::c_int;
    } else {
        so = 0 as ::core::ffi::c_int;
        eo = strlen(string) as ::core::ffi::c_int;
    }
    rc = pcre2_match_8(
        (*preg).re_pcre2_code as *const pcre2_code_8,
        (string as PCRE2_SPTR8).offset(so as isize),
        (eo - so) as size_t,
        0 as size_t,
        options as uint32_t,
        md,
        ::core::ptr::null_mut::<pcre2_match_context_8>(),
    );
    if rc >= 0 as ::core::ffi::c_int {
        let mut i: size_t = 0;
        let mut ovector: *mut size_t = pcre2_get_ovector_pointer_8(md);
        if rc as size_t > nmatch {
            rc = nmatch as ::core::ffi::c_int;
        }
        i = 0 as size_t;
        while i < rc as size_t {
            (*pmatch.offset(i as isize)).rm_so =
                (if *ovector.offset(i.wrapping_mul(2 as size_t) as isize) == PCRE2_UNSET {
                    -(1 as ::core::ffi::c_int)
                } else {
                    (*ovector.offset(i.wrapping_mul(2 as size_t) as isize))
                        .wrapping_add(so as size_t) as ::core::ffi::c_int
                }) as regoff_t;
            (*pmatch.offset(i as isize)).rm_eo = (if *ovector
                .offset(i.wrapping_mul(2 as size_t).wrapping_add(1 as size_t) as isize)
                == PCRE2_UNSET
            {
                -(1 as ::core::ffi::c_int)
            } else {
                (*ovector.offset(i.wrapping_mul(2 as size_t).wrapping_add(1 as size_t) as isize))
                    .wrapping_add(so as size_t) as ::core::ffi::c_int
            }) as regoff_t;
            i = i.wrapping_add(1);
        }
        while i < nmatch {
            let ref mut fresh0 = (*pmatch.offset(i as isize)).rm_eo;
            *fresh0 = -(1 as ::core::ffi::c_int) as regoff_t;
            (*pmatch.offset(i as isize)).rm_so = *fresh0;
            i = i.wrapping_add(1);
        }
        return 0 as ::core::ffi::c_int;
    }
    if rc <= PCRE2_ERROR_UTF8_ERR1 && rc >= PCRE2_ERROR_UTF8_ERR21 {
        return REG_INVARG as ::core::ffi::c_int;
    }
    match rc {
        PCRE2_ERROR_HEAPLIMIT => return REG_ESPACE as ::core::ffi::c_int,
        PCRE2_ERROR_NOMATCH => return REG_NOMATCH as ::core::ffi::c_int,
        PCRE2_ERROR_BADMODE => return REG_INVARG as ::core::ffi::c_int,
        PCRE2_ERROR_BADMAGIC => return REG_INVARG as ::core::ffi::c_int,
        PCRE2_ERROR_BADOPTION => return REG_INVARG as ::core::ffi::c_int,
        PCRE2_ERROR_BADUTFOFFSET => return REG_INVARG as ::core::ffi::c_int,
        PCRE2_ERROR_MATCHLIMIT => return REG_ESPACE as ::core::ffi::c_int,
        PCRE2_ERROR_NOMEMORY => return REG_ESPACE as ::core::ffi::c_int,
        PCRE2_ERROR_NULL => return REG_INVARG as ::core::ffi::c_int,
        _ => return REG_ASSERT as ::core::ffi::c_int,
    };
}
