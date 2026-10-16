// Translated from PCRE2 10.49 pcre2_compile_cgroup.c; code unit width 16.
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
    fn memmove(
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
    fn _pcre2_strncmp_16(_: PCRE2_SPTR16, _: PCRE2_SPTR16, _: size_t) -> ::core::ffi::c_int;
}
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type size_t = usize;
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
pub struct pcre2_real_compile_context_16 {
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct named_group_16 {
    pub name: PCRE2_SPTR16,
    pub number: uint32_t,
    pub length: uint16_t,
    pub hash_dup: uint16_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct compile_data {
    pub next: *mut compile_data,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct recurse_arguments {
    pub header: compile_data,
    pub size: size_t,
    pub skip_size: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union class_bits_storage {
    pub classbits: [uint8_t; 32],
    pub classwords: [uint32_t; 8],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct compile_block_16 {
    pub cx: *mut pcre2_real_compile_context_16,
    pub lcc: *const uint8_t,
    pub fcc: *const uint8_t,
    pub cbits: *const uint8_t,
    pub ctypes: *const uint8_t,
    pub start_workspace: *mut PCRE2_UCHAR16,
    pub start_code: *mut PCRE2_UCHAR16,
    pub start_pattern: PCRE2_SPTR16,
    pub end_pattern: PCRE2_SPTR16,
    pub name_table: *mut PCRE2_UCHAR16,
    pub workspace_size: size_t,
    pub small_ref_offset: [size_t; 10],
    pub erroroffset: size_t,
    pub classbits: class_bits_storage,
    pub names_found: uint16_t,
    pub name_entry_size: uint16_t,
    pub parens_depth: uint16_t,
    pub assert_depth: uint16_t,
    pub named_groups: *mut named_group_16,
    pub named_group_list_size: uint32_t,
    pub external_options: uint32_t,
    pub external_flags: uint32_t,
    pub bracount: uint32_t,
    pub lastcapture: uint32_t,
    pub parsed_pattern: *mut uint32_t,
    pub parsed_pattern_end: *mut uint32_t,
    pub groupinfo: *mut uint32_t,
    pub top_backref: uint32_t,
    pub backref_map: uint32_t,
    pub nltype: uint32_t,
    pub nllen: uint32_t,
    pub nl: [PCRE2_UCHAR16; 4],
    pub class_op_used: [uint8_t; 15],
    pub req_varyopt: uint32_t,
    pub max_varlookbehind: uint32_t,
    pub max_lookbehind: ::core::ffi::c_int,
    pub had_accept: BOOL,
    pub had_pruneorskip: BOOL,
    pub had_recurse: BOOL,
    pub dupnames: BOOL,
    pub first_data: *mut compile_data,
    pub last_data: *mut compile_data,
    pub char_lists_size: size_t,
}
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const ERR120: C2RustUnnamed = 220;
pub const ERR119: C2RustUnnamed = 219;
pub const ERR118: C2RustUnnamed = 218;
pub const ERR117: C2RustUnnamed = 217;
pub const ERR116: C2RustUnnamed = 216;
pub const ERR115: C2RustUnnamed = 215;
pub const ERR114: C2RustUnnamed = 214;
pub const ERR113: C2RustUnnamed = 213;
pub const ERR112: C2RustUnnamed = 212;
pub const ERR111: C2RustUnnamed = 211;
pub const ERR110: C2RustUnnamed = 210;
pub const ERR109: C2RustUnnamed = 209;
pub const ERR108: C2RustUnnamed = 208;
pub const ERR107: C2RustUnnamed = 207;
pub const ERR106: C2RustUnnamed = 206;
pub const ERR105: C2RustUnnamed = 205;
pub const ERR104: C2RustUnnamed = 204;
pub const ERR103: C2RustUnnamed = 203;
pub const ERR102: C2RustUnnamed = 202;
pub const ERR101: C2RustUnnamed = 201;
pub const ERR100: C2RustUnnamed = 200;
pub const ERR99: C2RustUnnamed = 199;
pub const ERR98: C2RustUnnamed = 198;
pub const ERR97: C2RustUnnamed = 197;
pub const ERR96: C2RustUnnamed = 196;
pub const ERR95: C2RustUnnamed = 195;
pub const ERR94: C2RustUnnamed = 194;
pub const ERR93: C2RustUnnamed = 193;
pub const ERR92: C2RustUnnamed = 192;
pub const ERR91: C2RustUnnamed = 191;
pub const ERR90: C2RustUnnamed = 190;
pub const ERR89: C2RustUnnamed = 189;
pub const ERR88: C2RustUnnamed = 188;
pub const ERR87: C2RustUnnamed = 187;
pub const ERR86: C2RustUnnamed = 186;
pub const ERR85: C2RustUnnamed = 185;
pub const ERR84: C2RustUnnamed = 184;
pub const ERR83: C2RustUnnamed = 183;
pub const ERR82: C2RustUnnamed = 182;
pub const ERR81: C2RustUnnamed = 181;
pub const ERR80: C2RustUnnamed = 180;
pub const ERR79: C2RustUnnamed = 179;
pub const ERR78: C2RustUnnamed = 178;
pub const ERR77: C2RustUnnamed = 177;
pub const ERR76: C2RustUnnamed = 176;
pub const ERR75: C2RustUnnamed = 175;
pub const ERR74: C2RustUnnamed = 174;
pub const ERR73: C2RustUnnamed = 173;
pub const ERR72: C2RustUnnamed = 172;
pub const ERR71: C2RustUnnamed = 171;
pub const ERR70: C2RustUnnamed = 170;
pub const ERR69: C2RustUnnamed = 169;
pub const ERR68: C2RustUnnamed = 168;
pub const ERR67: C2RustUnnamed = 167;
pub const ERR66: C2RustUnnamed = 166;
pub const ERR65: C2RustUnnamed = 165;
pub const ERR64: C2RustUnnamed = 164;
pub const ERR63: C2RustUnnamed = 163;
pub const ERR62: C2RustUnnamed = 162;
pub const ERR61: C2RustUnnamed = 161;
pub const ERR60: C2RustUnnamed = 160;
pub const ERR59: C2RustUnnamed = 159;
pub const ERR58: C2RustUnnamed = 158;
pub const ERR57: C2RustUnnamed = 157;
pub const ERR56: C2RustUnnamed = 156;
pub const ERR55: C2RustUnnamed = 155;
pub const ERR54: C2RustUnnamed = 154;
pub const ERR53: C2RustUnnamed = 153;
pub const ERR52: C2RustUnnamed = 152;
pub const ERR51: C2RustUnnamed = 151;
pub const ERR50: C2RustUnnamed = 150;
pub const ERR49: C2RustUnnamed = 149;
pub const ERR48: C2RustUnnamed = 148;
pub const ERR47: C2RustUnnamed = 147;
pub const ERR46: C2RustUnnamed = 146;
pub const ERR45: C2RustUnnamed = 145;
pub const ERR44: C2RustUnnamed = 144;
pub const ERR43: C2RustUnnamed = 143;
pub const ERR42: C2RustUnnamed = 142;
pub const ERR41: C2RustUnnamed = 141;
pub const ERR40: C2RustUnnamed = 140;
pub const ERR39: C2RustUnnamed = 139;
pub const ERR38: C2RustUnnamed = 138;
pub const ERR37: C2RustUnnamed = 137;
pub const ERR36: C2RustUnnamed = 136;
pub const ERR35: C2RustUnnamed = 135;
pub const ERR34: C2RustUnnamed = 134;
pub const ERR33: C2RustUnnamed = 133;
pub const ERR32: C2RustUnnamed = 132;
pub const ERR31: C2RustUnnamed = 131;
pub const ERR30: C2RustUnnamed = 130;
pub const ERR29: C2RustUnnamed = 129;
pub const ERR28: C2RustUnnamed = 128;
pub const ERR27: C2RustUnnamed = 127;
pub const ERR26: C2RustUnnamed = 126;
pub const ERR25: C2RustUnnamed = 125;
pub const ERR24: C2RustUnnamed = 124;
pub const ERR23: C2RustUnnamed = 123;
pub const ERR22: C2RustUnnamed = 122;
pub const ERR21: C2RustUnnamed = 121;
pub const ERR20: C2RustUnnamed = 120;
pub const ERR19: C2RustUnnamed = 119;
pub const ERR18: C2RustUnnamed = 118;
pub const ERR17: C2RustUnnamed = 117;
pub const ERR16: C2RustUnnamed = 116;
pub const ERR15: C2RustUnnamed = 115;
pub const ERR14: C2RustUnnamed = 114;
pub const ERR13: C2RustUnnamed = 113;
pub const ERR12: C2RustUnnamed = 112;
pub const ERR11: C2RustUnnamed = 111;
pub const ERR10: C2RustUnnamed = 110;
pub const ERR9: C2RustUnnamed = 109;
pub const ERR8: C2RustUnnamed = 108;
pub const ERR7: C2RustUnnamed = 107;
pub const ERR6: C2RustUnnamed = 106;
pub const ERR5: C2RustUnnamed = 105;
pub const ERR4: C2RustUnnamed = 104;
pub const ERR3: C2RustUnnamed = 103;
pub const ERR2: C2RustUnnamed = 102;
pub const ERR1: C2RustUnnamed = 101;
pub const ERR0: C2RustUnnamed = 100;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const IMM2_SIZE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const META_OFFSET: ::core::ffi::c_uint = 2148925440;
pub const META_CAPTURE_NAME: ::core::ffi::c_uint = 2149056512;
pub const META_CAPTURE_NUMBER: ::core::ffi::c_uint = 2149122048;
pub const NAMED_GROUP_HASH_MASK: uint16_t = 0x7fff as ::core::ffi::c_int as uint16_t;
pub const NAMED_GROUP_IS_DUPNAME: uint16_t = 0x8000 as ::core::ffi::c_int as uint16_t;
#[no_mangle]
pub unsafe extern "C" fn _pcre2_compile_get_hash_from_name16(
    mut name: PCRE2_SPTR16,
    mut length: uint32_t,
) -> uint16_t {
    let mut hash: uint16_t = 0;
    hash = (*name.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        & 0x7f as ::core::ffi::c_int
        | (*name.offset(length.wrapping_sub(1 as uint32_t) as isize) as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_int)
            << 7 as ::core::ffi::c_int) as uint16_t;
    return hash;
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_compile_find_named_group16(
    mut name: PCRE2_SPTR16,
    mut length: uint32_t,
    mut cb: *mut compile_block_16,
) -> *mut named_group_16 {
    let mut hash: uint16_t = _pcre2_compile_get_hash_from_name16(name, length);
    let mut ng: *mut named_group_16 = ::core::ptr::null_mut::<named_group_16>();
    let mut end: *mut named_group_16 = (*cb)
        .named_groups
        .offset((*cb).names_found as ::core::ffi::c_int as isize);
    ng = (*cb).named_groups;
    while ng < end {
        if length == (*ng).length as uint32_t
            && hash as ::core::ffi::c_int
                == (*ng).hash_dup as ::core::ffi::c_int
                    & NAMED_GROUP_HASH_MASK as ::core::ffi::c_int
            && _pcre2_strncmp_16(name, (*ng).name, length as size_t) == 0 as ::core::ffi::c_int
        {
            return ng;
        }
        ng = ng.offset(1);
    }
    return ::core::ptr::null_mut::<named_group_16>();
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_compile_add_name_to_table16(
    mut cb: *mut compile_block_16,
    mut ng: *mut named_group_16,
    mut tablecount: uint32_t,
) -> uint32_t {
    let mut i: uint32_t = 0;
    let mut name: PCRE2_SPTR16 = (*ng).name;
    let mut length: ::core::ffi::c_int = (*ng).length as ::core::ffi::c_int;
    let mut duplicate_count: uint32_t = 1 as uint32_t;
    let mut slot: *mut PCRE2_UCHAR16 = (*cb).name_table;
    if (*ng).hash_dup as ::core::ffi::c_int & NAMED_GROUP_IS_DUPNAME as ::core::ffi::c_int
        != 0 as ::core::ffi::c_int
    {
        let mut ng_it: *mut named_group_16 = ::core::ptr::null_mut::<named_group_16>();
        let mut end: *mut named_group_16 = (*cb)
            .named_groups
            .offset((*cb).names_found as ::core::ffi::c_int as isize);
        ng_it = ng.offset(1 as ::core::ffi::c_int as isize);
        while ng_it < end {
            if (*ng_it).name == name {
                duplicate_count = duplicate_count.wrapping_add(1);
            }
            ng_it = ng_it.offset(1);
        }
    }
    i = 0 as uint32_t;
    while i < tablecount {
        let mut crc: ::core::ffi::c_int = memcmp(
            name as *const ::core::ffi::c_void,
            slot.offset(IMM2_SIZE as isize) as *const ::core::ffi::c_void,
            (length * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int)) as size_t,
        );
        if crc == 0 as ::core::ffi::c_int
            && *slot.offset((IMM2_SIZE + length) as isize) as ::core::ffi::c_int
                != 0 as ::core::ffi::c_int
        {
            crc = -(1 as ::core::ffi::c_int);
        }
        if crc < 0 as ::core::ffi::c_int {
            memmove(
                slot.offset(
                    ((*cb).name_entry_size as uint32_t).wrapping_mul(duplicate_count) as isize,
                ) as *mut ::core::ffi::c_void,
                slot as *const ::core::ffi::c_void,
                tablecount
                    .wrapping_sub(i)
                    .wrapping_mul((*cb).name_entry_size as uint32_t)
                    .wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as uint32_t)
                    as size_t,
            );
            break;
        } else {
            slot = slot.offset((*cb).name_entry_size as ::core::ffi::c_int as isize);
            i = i.wrapping_add(1);
        }
    }
    tablecount = tablecount.wrapping_add(duplicate_count);
    loop {
        *slot.offset(0 as ::core::ffi::c_int as isize) = (*ng).number as PCRE2_UCHAR16;
        memcpy(
            slot.offset(IMM2_SIZE as isize) as *mut ::core::ffi::c_void,
            name as *const ::core::ffi::c_void,
            (length * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int)) as size_t,
        );
        memset(
            slot.offset(IMM2_SIZE as isize).offset(length as isize) as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            (((*cb).name_entry_size as ::core::ffi::c_int - length - 1 as ::core::ffi::c_int)
                * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int)) as size_t,
        );
        duplicate_count = duplicate_count.wrapping_sub(1);
        if duplicate_count == 0 as uint32_t {
            break;
        }
        loop {
            ng = ng.offset(1);
            if (*ng).name == name {
                break;
            }
        }
        slot = slot.offset((*cb).name_entry_size as ::core::ffi::c_int as isize);
    }
    return tablecount;
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_compile_find_dupname_details16(
    mut name: PCRE2_SPTR16,
    mut length: uint32_t,
    mut indexptr: *mut ::core::ffi::c_int,
    mut countptr: *mut ::core::ffi::c_int,
    mut errorcodeptr: *mut ::core::ffi::c_int,
    mut cb: *mut compile_block_16,
) -> BOOL {
    let mut i: uint32_t = 0;
    let mut groupnumber: uint32_t = 0;
    let mut count: ::core::ffi::c_int = 0;
    let mut slot: *mut PCRE2_UCHAR16 = (*cb).name_table;
    i = 0 as uint32_t;
    while i < (*cb).names_found as uint32_t {
        if _pcre2_strncmp_16(
            name,
            slot.offset(IMM2_SIZE as isize) as PCRE2_SPTR16,
            length as size_t,
        ) == 0 as ::core::ffi::c_int
            && *slot.offset((IMM2_SIZE as uint32_t).wrapping_add(length) as isize)
                as ::core::ffi::c_int
                == 0 as ::core::ffi::c_int
        {
            break;
        }
        slot = slot.offset((*cb).name_entry_size as ::core::ffi::c_int as isize);
        i = i.wrapping_add(1);
    }
    if i >= (*cb).names_found as uint32_t {
        *errorcodeptr = ERR53 as ::core::ffi::c_int;
        (*cb).erroroffset = name.offset_from((*cb).start_pattern) as isize as size_t;
        return FALSE;
    }
    *indexptr = i as ::core::ffi::c_int;
    count = 0 as ::core::ffi::c_int;
    loop {
        count += 1;
        groupnumber = *slot.offset(0 as ::core::ffi::c_int as isize) as uint32_t;
        (*cb).backref_map = ((*cb).backref_map as ::core::ffi::c_uint
            | if groupnumber < 32 as uint32_t {
                (1 as ::core::ffi::c_uint) << groupnumber
            } else {
                1 as ::core::ffi::c_uint
            }) as uint32_t;
        if groupnumber > (*cb).top_backref {
            (*cb).top_backref = groupnumber;
        }
        i = i.wrapping_add(1);
        if i >= (*cb).names_found as uint32_t {
            break;
        }
        slot = slot.offset((*cb).name_entry_size as ::core::ffi::c_int as isize);
        if _pcre2_strncmp_16(
            name,
            slot.offset(IMM2_SIZE as isize) as PCRE2_SPTR16,
            length as size_t,
        ) != 0 as ::core::ffi::c_int
            || *slot.offset(IMM2_SIZE as isize).offset(length as isize) as ::core::ffi::c_int
                != 0 as ::core::ffi::c_int
        {
            break;
        }
    }
    *countptr = count;
    return TRUE;
}
unsafe extern "C" fn _pcre2_compile_process_capture_list(
    mut pptr: *mut uint32_t,
    mut offset: size_t,
    mut errorcodeptr: *mut ::core::ffi::c_int,
    mut cb: *mut compile_block_16,
) -> size_t {
    let mut i: size_t = 0;
    let mut size: size_t = 0 as size_t;
    let mut ng: *mut named_group_16 = ::core::ptr::null_mut::<named_group_16>();
    let mut name: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut length: uint32_t = 0;
    let mut end: *mut named_group_16 = (*cb)
        .named_groups
        .offset((*cb).names_found as ::core::ffi::c_int as isize);
    loop {
        pptr = pptr.offset(1);
        match *pptr & 0xffff0000 as uint32_t {
            META_OFFSET => {
                offset = (*pptr.offset(1 as ::core::ffi::c_int as isize) as size_t)
                    << 32 as ::core::ffi::c_int
                    | *pptr.offset(2 as ::core::ffi::c_int as isize) as size_t;
                pptr = pptr.offset(2 as ::core::ffi::c_int as isize);
            }
            META_CAPTURE_NAME => {
                offset = offset.wrapping_add((*pptr & 0xffff as uint32_t) as size_t);
                pptr = pptr.offset(1);
                length = *pptr;
                name = (*cb).start_pattern.offset(offset as isize);
                ng = _pcre2_compile_find_named_group16(name, length, cb);
                if ng.is_null() {
                    *errorcodeptr = ERR15 as ::core::ffi::c_int;
                    (*cb).erroroffset = offset;
                    return 0 as size_t;
                }
                if (*ng).hash_dup as ::core::ffi::c_int
                    & NAMED_GROUP_IS_DUPNAME as ::core::ffi::c_int
                    == 0 as ::core::ffi::c_int
                {
                    *pptr.offset(-(1 as ::core::ffi::c_int) as isize) =
                        META_CAPTURE_NUMBER as uint32_t;
                    *pptr.offset(0 as ::core::ffi::c_int as isize) = (*ng).number;
                    size = size.wrapping_add(1);
                } else {
                    *pptr.offset(-(1 as ::core::ffi::c_int) as isize) =
                        META_CAPTURE_NAME as uint32_t;
                    *pptr.offset(0 as ::core::ffi::c_int as isize) =
                        ng.offset_from((*cb).named_groups) as isize as uint32_t;
                    size = size.wrapping_add(1);
                    name = (*ng).name;
                    loop {
                        ng = ng.offset(1);
                        if !(ng < end) {
                            break;
                        }
                        if (*ng).name == name {
                            size = size.wrapping_add(1);
                        }
                    }
                }
            }
            META_CAPTURE_NUMBER => {
                offset = offset.wrapping_add((*pptr & 0xffff as uint32_t) as size_t);
                pptr = pptr.offset(1);
                i = *pptr as size_t;
                if i > (*cb).bracount as size_t {
                    *errorcodeptr = ERR15 as ::core::ffi::c_int;
                    (*cb).erroroffset = offset;
                    return 0 as size_t;
                }
                if i > (*cb).top_backref as size_t {
                    (*cb).top_backref = i as uint16_t as uint32_t;
                }
                size = size.wrapping_add(1);
            }
            _ => return size,
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_compile_parse_scan_substr_args16(
    mut pptr: *mut uint32_t,
    mut errorcodeptr: *mut ::core::ffi::c_int,
    mut cb: *mut compile_block_16,
    mut lengthptr: *mut size_t,
) -> *mut uint32_t {
    let mut captures: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut capture_ptr: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut bit: uint8_t = 0;
    let mut name: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut ng: *mut named_group_16 = ::core::ptr::null_mut::<named_group_16>();
    let mut end: *mut named_group_16 = (*cb)
        .named_groups
        .offset((*cb).names_found as ::core::ffi::c_int as isize);
    let mut all_found: BOOL = 0;
    let mut size: size_t = 0;
    if _pcre2_compile_process_capture_list(
        pptr.offset(-(1 as ::core::ffi::c_int as isize)),
        0 as size_t,
        errorcodeptr,
        cb,
    ) == 0 as size_t
    {
        return ::core::ptr::null_mut::<uint32_t>();
    }
    size = ((*cb)
        .bracount
        .wrapping_add(1 as uint32_t)
        .wrapping_add(7 as uint32_t)
        >> 3 as ::core::ffi::c_int) as size_t;
    captures = (*(*cb).cx)
        .memctl
        .malloc
        .expect("non-null function pointer")(size, (*(*cb).cx).memctl.memory_data)
        as *mut uint8_t;
    if captures.is_null() {
        *errorcodeptr = ERR21 as ::core::ffi::c_int;
        (*cb).erroroffset = (*pptr.offset(1 as ::core::ffi::c_int as isize) as size_t)
            << 32 as ::core::ffi::c_int
            | *pptr.offset(2 as ::core::ffi::c_int as isize) as size_t;
        return ::core::ptr::null_mut::<uint32_t>();
    }
    memset(
        captures as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        size,
    );
    loop {
        match *pptr & 0xffff0000 as uint32_t {
            META_OFFSET => {
                pptr = pptr.offset(1);
                pptr = pptr.offset(2 as ::core::ffi::c_int as isize);
            }
            META_CAPTURE_NAME => {
                ng = (*cb)
                    .named_groups
                    .offset(*pptr.offset(1 as ::core::ffi::c_int as isize) as isize);
                pptr = pptr.offset(2 as ::core::ffi::c_int as isize);
                name = (*ng).name;
                all_found = TRUE as BOOL;
                loop {
                    if !((*ng).name != name) {
                        capture_ptr =
                            captures.offset(((*ng).number >> 3 as ::core::ffi::c_int) as isize);
                        bit = ((1 as ::core::ffi::c_int) << ((*ng).number & 0x7 as uint32_t))
                            as uint8_t;
                        if *capture_ptr as ::core::ffi::c_int & bit as ::core::ffi::c_int
                            == 0 as ::core::ffi::c_int
                        {
                            *capture_ptr = (*capture_ptr as ::core::ffi::c_int
                                | bit as ::core::ffi::c_int)
                                as uint8_t;
                            all_found = FALSE as BOOL;
                        }
                    }
                    ng = ng.offset(1);
                    if !(ng < end) {
                        break;
                    }
                }
                if all_found == 0 {
                    *lengthptr = (*lengthptr).wrapping_add(
                        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE) as size_t,
                    );
                } else {
                    *pptr.offset(-(2 as ::core::ffi::c_int) as isize) =
                        META_CAPTURE_NUMBER as uint32_t;
                    *pptr.offset(-(1 as ::core::ffi::c_int) as isize) = 0 as uint32_t;
                }
            }
            META_CAPTURE_NUMBER => {
                pptr = pptr.offset(2 as ::core::ffi::c_int as isize);
                capture_ptr = captures.offset(
                    (*pptr.offset(-(1 as ::core::ffi::c_int) as isize) >> 3 as ::core::ffi::c_int)
                        as isize,
                );
                bit = ((1 as ::core::ffi::c_int)
                    << (*pptr.offset(-(1 as ::core::ffi::c_int) as isize) & 0x7 as uint32_t))
                    as uint8_t;
                if *capture_ptr as ::core::ffi::c_int & bit as ::core::ffi::c_int
                    != 0 as ::core::ffi::c_int
                {
                    *pptr.offset(-(1 as ::core::ffi::c_int) as isize) = 0 as uint32_t;
                } else {
                    *capture_ptr =
                        (*capture_ptr as ::core::ffi::c_int | bit as ::core::ffi::c_int) as uint8_t;
                    *lengthptr =
                        (*lengthptr).wrapping_add((1 as ::core::ffi::c_int + IMM2_SIZE) as size_t);
                }
            }
            _ => {
                break;
            }
        }
    }
    (*(*cb).cx).memctl.free.expect("non-null function pointer")(
        captures as *mut ::core::ffi::c_void,
        (*(*cb).cx).memctl.memory_data,
    );
    return pptr.offset(-(1 as ::core::ffi::c_int as isize));
}
unsafe extern "C" fn do_heapify_u16(mut captures: *mut uint16_t, mut size: size_t, mut i: size_t) {
    let mut max: size_t = 0;
    let mut left: size_t = 0;
    let mut right: size_t = 0;
    let mut tmp: uint16_t = 0;
    loop {
        max = i;
        left = (i << 1 as ::core::ffi::c_int).wrapping_add(1 as size_t);
        right = left.wrapping_add(1 as size_t);
        if left < size
            && *captures.offset(left as isize) as ::core::ffi::c_int
                > *captures.offset(max as isize) as ::core::ffi::c_int
        {
            max = left;
        }
        if right < size
            && *captures.offset(right as isize) as ::core::ffi::c_int
                > *captures.offset(max as isize) as ::core::ffi::c_int
        {
            max = right;
        }
        if i == max {
            return;
        }
        tmp = *captures.offset(i as isize);
        *captures.offset(i as isize) = *captures.offset(max as isize);
        *captures.offset(max as isize) = tmp;
        i = max;
    }
}
#[no_mangle]
pub unsafe extern "C" fn _pcre2_compile_parse_recurse_args16(
    mut pptr_start: *mut uint32_t,
    mut offset: size_t,
    mut errorcodeptr: *mut ::core::ffi::c_int,
    mut cb: *mut compile_block_16,
) -> BOOL {
    let mut pptr: *mut uint32_t = pptr_start;
    let mut i: size_t = 0;
    let mut size: size_t = 0;
    let mut name: PCRE2_SPTR16 = ::core::ptr::null::<PCRE2_UCHAR16>();
    let mut ng: *mut named_group_16 = ::core::ptr::null_mut::<named_group_16>();
    let mut end: *mut named_group_16 = (*cb)
        .named_groups
        .offset((*cb).names_found as ::core::ffi::c_int as isize);
    let mut args: *mut recurse_arguments = ::core::ptr::null_mut::<recurse_arguments>();
    let mut captures: *mut uint16_t = ::core::ptr::null_mut::<uint16_t>();
    let mut current: *mut uint16_t = ::core::ptr::null_mut::<uint16_t>();
    let mut captures_end: *mut uint16_t = ::core::ptr::null_mut::<uint16_t>();
    let mut tmp: uint16_t = 0;
    size = _pcre2_compile_process_capture_list(pptr, offset, errorcodeptr, cb);
    if size == 0 as size_t {
        return FALSE;
    }
    args = (*(*cb).cx)
        .memctl
        .malloc
        .expect("non-null function pointer")(
        (::core::mem::size_of::<recurse_arguments>() as size_t)
            .wrapping_add(size.wrapping_mul(::core::mem::size_of::<uint16_t>() as size_t)),
        (*(*cb).cx).memctl.memory_data,
    ) as *mut recurse_arguments;
    if args.is_null() {
        *errorcodeptr = ERR21 as ::core::ffi::c_int;
        (*cb).erroroffset = offset;
        return FALSE;
    }
    (*args).header.next = ::core::ptr::null_mut::<compile_data>();
    (*args).size = size;
    if !(*cb).last_data.is_null() {
        (*(*cb).last_data).next = &raw mut (*args).header as *mut compile_data;
    } else {
        (*cb).first_data = &raw mut (*args).header;
    }
    (*cb).last_data = &raw mut (*args).header;
    captures = args.offset(1 as ::core::ffi::c_int as isize) as *mut uint16_t;
    loop {
        pptr = pptr.offset(1);
        match *pptr & 0xffff0000 as uint32_t {
            META_OFFSET => {
                pptr = pptr.offset(2 as ::core::ffi::c_int as isize);
            }
            META_CAPTURE_NAME => {
                pptr = pptr.offset(1);
                ng = (*cb).named_groups.offset(*pptr as isize);
                let fresh0 = captures;
                captures = captures.offset(1);
                *fresh0 = (*ng).number as uint16_t;
                name = (*ng).name;
                loop {
                    ng = ng.offset(1);
                    if !(ng < end) {
                        break;
                    }
                    if (*ng).name == name {
                        let fresh1 = captures;
                        captures = captures.offset(1);
                        *fresh1 = (*ng).number as uint16_t;
                    }
                }
            }
            META_CAPTURE_NUMBER => {
                pptr = pptr.offset(1);
                let fresh2 = captures;
                captures = captures.offset(1);
                *fresh2 = *pptr as uint16_t;
            }
            _ => {
                break;
            }
        }
    }
    (*args).skip_size =
        (pptr.offset_from(pptr_start) as isize as size_t).wrapping_sub(1 as size_t);
    if size == 1 as size_t {
        return TRUE;
    }
    captures = args.offset(1 as ::core::ffi::c_int as isize) as *mut uint16_t;
    i = (size >> 1 as ::core::ffi::c_int).wrapping_sub(1 as size_t);
    loop {
        do_heapify_u16(captures, size, i);
        if i == 0 as size_t {
            break;
        }
        i = i.wrapping_sub(1);
    }
    i = size.wrapping_sub(1 as size_t);
    while i > 0 as size_t {
        tmp = *captures.offset(0 as ::core::ffi::c_int as isize);
        *captures.offset(0 as ::core::ffi::c_int as isize) = *captures.offset(i as isize);
        *captures.offset(i as isize) = tmp;
        do_heapify_u16(captures, i, 0 as size_t);
        i = i.wrapping_sub(1);
    }
    captures_end = captures.offset(size as isize);
    let fresh3 = captures;
    captures = captures.offset(1);
    tmp = *fresh3;
    current = captures;
    while current < captures_end {
        if *current as ::core::ffi::c_int != tmp as ::core::ffi::c_int {
            tmp = *current;
            let fresh4 = captures;
            captures = captures.offset(1);
            *fresh4 = tmp;
        }
        current = current.offset(1);
    }
    (*args).size = captures
        .offset_from(args.offset(1 as ::core::ffi::c_int as isize) as *mut uint16_t)
        as isize as size_t;
    return TRUE;
}
pub const PCRE2_CODE_UNIT_WIDTH: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
