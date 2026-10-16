// Translated from PCRE2 10.49 pcre2_find_bracket.c; code unit width 8.
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
// /* This module contains a single function that scans through a compiled pattern
// until it finds a capturing bracket with the given number, or, if the number is
// negative, an instance of OP_REVERSE or OP_VREVERSE for a lookbehind. The
// function is called from pcre2_compile.c and also from pcre2_study.c when
// finding the minimum matching length. */
extern "C" {
    static _pcre2_utf8_table4: [uint8_t; 0];
    static _pcre2_OP_lengths_8: [uint8_t; 0];
}
pub type __uint8_t = u8;
pub type BOOL = ::core::ffi::c_int;
pub type uint8_t = __uint8_t;
pub type PCRE2_UCHAR8 = uint8_t;
pub type PCRE2_SPTR8 = *const PCRE2_UCHAR8;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const OP_TABLE_LENGTH: C2RustUnnamed = 173;
pub const OP_UCP_WORD_BOUNDARY: C2RustUnnamed = 172;
pub const OP_NOT_UCP_WORD_BOUNDARY: C2RustUnnamed = 171;
pub const OP_DEFINE: C2RustUnnamed = 170;
pub const OP_SKIPZERO: C2RustUnnamed = 169;
pub const OP_CLOSE: C2RustUnnamed = 168;
pub const OP_ASSERT_ACCEPT: C2RustUnnamed = 167;
pub const OP_ACCEPT: C2RustUnnamed = 166;
pub const OP_FAIL: C2RustUnnamed = 165;
pub const OP_COMMIT_ARG: C2RustUnnamed = 164;
pub const OP_COMMIT: C2RustUnnamed = 163;
pub const OP_THEN_ARG: C2RustUnnamed = 162;
pub const OP_THEN: C2RustUnnamed = 161;
pub const OP_SKIP_ARG: C2RustUnnamed = 160;
pub const OP_SKIP: C2RustUnnamed = 159;
pub const OP_PRUNE_ARG: C2RustUnnamed = 158;
pub const OP_PRUNE: C2RustUnnamed = 157;
pub const OP_MARK: C2RustUnnamed = 156;
pub const OP_BRAPOSZERO: C2RustUnnamed = 155;
pub const OP_BRAMINZERO: C2RustUnnamed = 154;
pub const OP_BRAZERO: C2RustUnnamed = 153;
pub const OP_TRUE: C2RustUnnamed = 152;
pub const OP_FALSE: C2RustUnnamed = 151;
pub const OP_DNRREF: C2RustUnnamed = 150;
pub const OP_RREF: C2RustUnnamed = 149;
pub const OP_DNCREF: C2RustUnnamed = 148;
pub const OP_CREF: C2RustUnnamed = 147;
pub const OP_SCOND: C2RustUnnamed = 146;
pub const OP_SCBRAPOS: C2RustUnnamed = 145;
pub const OP_SCBRA: C2RustUnnamed = 144;
pub const OP_SBRAPOS: C2RustUnnamed = 143;
pub const OP_SBRA: C2RustUnnamed = 142;
pub const OP_COND: C2RustUnnamed = 141;
pub const OP_CBRAPOS: C2RustUnnamed = 140;
pub const OP_CBRA: C2RustUnnamed = 139;
pub const OP_BRAPOS: C2RustUnnamed = 138;
pub const OP_BRA: C2RustUnnamed = 137;
pub const OP_SCRIPT_RUN: C2RustUnnamed = 136;
pub const OP_ONCE: C2RustUnnamed = 135;
pub const OP_ASSERT_SCS: C2RustUnnamed = 134;
pub const OP_ASSERTBACK_NA: C2RustUnnamed = 133;
pub const OP_ASSERT_NA: C2RustUnnamed = 132;
pub const OP_ASSERTBACK_NOT: C2RustUnnamed = 131;
pub const OP_ASSERTBACK: C2RustUnnamed = 130;
pub const OP_ASSERT_NOT: C2RustUnnamed = 129;
pub const OP_ASSERT: C2RustUnnamed = 128;
pub const OP_VREVERSE: C2RustUnnamed = 127;
pub const OP_REVERSE: C2RustUnnamed = 126;
pub const OP_KETRPOS: C2RustUnnamed = 125;
pub const OP_KETRMIN: C2RustUnnamed = 124;
pub const OP_KETRMAX: C2RustUnnamed = 123;
pub const OP_KET: C2RustUnnamed = 122;
pub const OP_ALT: C2RustUnnamed = 121;
pub const OP_CALLOUT_STR: C2RustUnnamed = 120;
pub const OP_CALLOUT: C2RustUnnamed = 119;
pub const OP_RECURSE: C2RustUnnamed = 118;
pub const OP_DNREFI: C2RustUnnamed = 117;
pub const OP_DNREF: C2RustUnnamed = 116;
pub const OP_REFI: C2RustUnnamed = 115;
pub const OP_REF: C2RustUnnamed = 114;
pub const OP_ECLASS: C2RustUnnamed = 113;
pub const OP_XCLASS: C2RustUnnamed = 112;
pub const OP_NCLASS: C2RustUnnamed = 111;
pub const OP_CLASS: C2RustUnnamed = 110;
pub const OP_CRPOSRANGE: C2RustUnnamed = 109;
pub const OP_CRPOSQUERY: C2RustUnnamed = 108;
pub const OP_CRPOSPLUS: C2RustUnnamed = 107;
pub const OP_CRPOSSTAR: C2RustUnnamed = 106;
pub const OP_CRMINRANGE: C2RustUnnamed = 105;
pub const OP_CRRANGE: C2RustUnnamed = 104;
pub const OP_CRMINQUERY: C2RustUnnamed = 103;
pub const OP_CRQUERY: C2RustUnnamed = 102;
pub const OP_CRMINPLUS: C2RustUnnamed = 101;
pub const OP_CRPLUS: C2RustUnnamed = 100;
pub const OP_CRMINSTAR: C2RustUnnamed = 99;
pub const OP_CRSTAR: C2RustUnnamed = 98;
pub const OP_TYPEPOSUPTO: C2RustUnnamed = 97;
pub const OP_TYPEPOSQUERY: C2RustUnnamed = 96;
pub const OP_TYPEPOSPLUS: C2RustUnnamed = 95;
pub const OP_TYPEPOSSTAR: C2RustUnnamed = 94;
pub const OP_TYPEEXACT: C2RustUnnamed = 93;
pub const OP_TYPEMINUPTO: C2RustUnnamed = 92;
pub const OP_TYPEUPTO: C2RustUnnamed = 91;
pub const OP_TYPEMINQUERY: C2RustUnnamed = 90;
pub const OP_TYPEQUERY: C2RustUnnamed = 89;
pub const OP_TYPEMINPLUS: C2RustUnnamed = 88;
pub const OP_TYPEPLUS: C2RustUnnamed = 87;
pub const OP_TYPEMINSTAR: C2RustUnnamed = 86;
pub const OP_TYPESTAR: C2RustUnnamed = 85;
pub const OP_NOTPOSUPTOI: C2RustUnnamed = 84;
pub const OP_NOTPOSQUERYI: C2RustUnnamed = 83;
pub const OP_NOTPOSPLUSI: C2RustUnnamed = 82;
pub const OP_NOTPOSSTARI: C2RustUnnamed = 81;
pub const OP_NOTEXACTI: C2RustUnnamed = 80;
pub const OP_NOTMINUPTOI: C2RustUnnamed = 79;
pub const OP_NOTUPTOI: C2RustUnnamed = 78;
pub const OP_NOTMINQUERYI: C2RustUnnamed = 77;
pub const OP_NOTQUERYI: C2RustUnnamed = 76;
pub const OP_NOTMINPLUSI: C2RustUnnamed = 75;
pub const OP_NOTPLUSI: C2RustUnnamed = 74;
pub const OP_NOTMINSTARI: C2RustUnnamed = 73;
pub const OP_NOTSTARI: C2RustUnnamed = 72;
pub const OP_NOTPOSUPTO: C2RustUnnamed = 71;
pub const OP_NOTPOSQUERY: C2RustUnnamed = 70;
pub const OP_NOTPOSPLUS: C2RustUnnamed = 69;
pub const OP_NOTPOSSTAR: C2RustUnnamed = 68;
pub const OP_NOTEXACT: C2RustUnnamed = 67;
pub const OP_NOTMINUPTO: C2RustUnnamed = 66;
pub const OP_NOTUPTO: C2RustUnnamed = 65;
pub const OP_NOTMINQUERY: C2RustUnnamed = 64;
pub const OP_NOTQUERY: C2RustUnnamed = 63;
pub const OP_NOTMINPLUS: C2RustUnnamed = 62;
pub const OP_NOTPLUS: C2RustUnnamed = 61;
pub const OP_NOTMINSTAR: C2RustUnnamed = 60;
pub const OP_NOTSTAR: C2RustUnnamed = 59;
pub const OP_POSUPTOI: C2RustUnnamed = 58;
pub const OP_POSQUERYI: C2RustUnnamed = 57;
pub const OP_POSPLUSI: C2RustUnnamed = 56;
pub const OP_POSSTARI: C2RustUnnamed = 55;
pub const OP_EXACTI: C2RustUnnamed = 54;
pub const OP_MINUPTOI: C2RustUnnamed = 53;
pub const OP_UPTOI: C2RustUnnamed = 52;
pub const OP_MINQUERYI: C2RustUnnamed = 51;
pub const OP_QUERYI: C2RustUnnamed = 50;
pub const OP_MINPLUSI: C2RustUnnamed = 49;
pub const OP_PLUSI: C2RustUnnamed = 48;
pub const OP_MINSTARI: C2RustUnnamed = 47;
pub const OP_STARI: C2RustUnnamed = 46;
pub const OP_POSUPTO: C2RustUnnamed = 45;
pub const OP_POSQUERY: C2RustUnnamed = 44;
pub const OP_POSPLUS: C2RustUnnamed = 43;
pub const OP_POSSTAR: C2RustUnnamed = 42;
pub const OP_EXACT: C2RustUnnamed = 41;
pub const OP_MINUPTO: C2RustUnnamed = 40;
pub const OP_UPTO: C2RustUnnamed = 39;
pub const OP_MINQUERY: C2RustUnnamed = 38;
pub const OP_QUERY: C2RustUnnamed = 37;
pub const OP_MINPLUS: C2RustUnnamed = 36;
pub const OP_PLUS: C2RustUnnamed = 35;
pub const OP_MINSTAR: C2RustUnnamed = 34;
pub const OP_STAR: C2RustUnnamed = 33;
pub const OP_NOTI: C2RustUnnamed = 32;
pub const OP_NOT: C2RustUnnamed = 31;
pub const OP_CHARI: C2RustUnnamed = 30;
pub const OP_CHAR: C2RustUnnamed = 29;
pub const OP_CIRCM: C2RustUnnamed = 28;
pub const OP_CIRC: C2RustUnnamed = 27;
pub const OP_DOLLM: C2RustUnnamed = 26;
pub const OP_DOLL: C2RustUnnamed = 25;
pub const OP_EOD: C2RustUnnamed = 24;
pub const OP_EODN: C2RustUnnamed = 23;
pub const OP_EXTUNI: C2RustUnnamed = 22;
pub const OP_VSPACE: C2RustUnnamed = 21;
pub const OP_NOT_VSPACE: C2RustUnnamed = 20;
pub const OP_HSPACE: C2RustUnnamed = 19;
pub const OP_NOT_HSPACE: C2RustUnnamed = 18;
pub const OP_ANYNL: C2RustUnnamed = 17;
pub const OP_PROP: C2RustUnnamed = 16;
pub const OP_NOTPROP: C2RustUnnamed = 15;
pub const OP_ANYBYTE: C2RustUnnamed = 14;
pub const OP_ALLANY: C2RustUnnamed = 13;
pub const OP_ANY: C2RustUnnamed = 12;
pub const OP_WORDCHAR: C2RustUnnamed = 11;
pub const OP_NOT_WORDCHAR: C2RustUnnamed = 10;
pub const OP_WHITESPACE: C2RustUnnamed = 9;
pub const OP_NOT_WHITESPACE: C2RustUnnamed = 8;
pub const OP_DIGIT: C2RustUnnamed = 7;
pub const OP_NOT_DIGIT: C2RustUnnamed = 6;
pub const OP_WORD_BOUNDARY: C2RustUnnamed = 5;
pub const OP_NOT_WORD_BOUNDARY: C2RustUnnamed = 4;
pub const OP_SET_SOM: C2RustUnnamed = 3;
pub const OP_SOM: C2RustUnnamed = 2;
pub const OP_SOD: C2RustUnnamed = 1;
pub const OP_END: C2RustUnnamed = 0;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const IMM2_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn _pcre2_find_bracket_8(
    mut code: PCRE2_SPTR8,
    mut utf: BOOL,
    mut number: ::core::ffi::c_int,
) -> PCRE2_SPTR8 {
    loop {
        let mut c: PCRE2_UCHAR8 = *code;
        if c as ::core::ffi::c_int == OP_END as ::core::ffi::c_int {
            return ::core::ptr::null::<PCRE2_UCHAR8>();
        }
        if c as ::core::ffi::c_int == OP_XCLASS as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_ECLASS as ::core::ffi::c_int
        {
            code = code.offset(
                ((*code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *code.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                        as ::core::ffi::c_int) as ::core::ffi::c_uint as isize,
            );
        } else if c as ::core::ffi::c_int == OP_CALLOUT_STR as ::core::ffi::c_int {
            code = code.offset(
                ((*code.offset(
                    (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                        as isize,
                ) as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *code.offset(
                        (1 as ::core::ffi::c_int
                            + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int
                            + 1 as ::core::ffi::c_int) as isize,
                    ) as ::core::ffi::c_int) as ::core::ffi::c_uint as isize,
            );
        } else if c as ::core::ffi::c_int == OP_REVERSE as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_VREVERSE as ::core::ffi::c_int
        {
            if number < 0 as ::core::ffi::c_int {
                return code;
            }
            code = code.offset(
                *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(c as isize)
                    as ::core::ffi::c_int as isize,
            );
        } else if c as ::core::ffi::c_int == OP_CBRA as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_SCBRA as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_CBRAPOS as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_SCBRAPOS as ::core::ffi::c_int
        {
            let mut n: ::core::ffi::c_int = ((*code
                .offset((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int)
                << 8 as ::core::ffi::c_int
                | *code.offset(
                    (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                        as isize,
                ) as ::core::ffi::c_int)
                as ::core::ffi::c_uint
                as ::core::ffi::c_int;
            if n == number {
                return code;
            }
            code = code.offset(
                *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(c as isize)
                    as ::core::ffi::c_int as isize,
            );
        } else {
            match c as ::core::ffi::c_int {
                85 | 86 | 87 | 88 | 89 | 90 | 94 | 95 | 96 => {
                    if *code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == OP_PROP as ::core::ffi::c_int
                        || *code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            == OP_NOTPROP as ::core::ffi::c_int
                    {
                        code = code.offset(2 as ::core::ffi::c_int as isize);
                    }
                }
                91 | 92 | 93 | 97 => {
                    if *code.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize)
                        as ::core::ffi::c_int
                        == OP_PROP as ::core::ffi::c_int
                        || *code.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize)
                            as ::core::ffi::c_int
                            == OP_NOTPROP as ::core::ffi::c_int
                    {
                        code = code.offset(2 as ::core::ffi::c_int as isize);
                    }
                }
                156 | 164 | 158 | 160 | 162 => {
                    code = code.offset(*code.offset(1 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int as isize);
                }
                _ => {}
            }
            code = code.offset(
                *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(c as isize)
                    as ::core::ffi::c_int as isize,
            );
            if utf != 0 {
                match c as ::core::ffi::c_int {
                    29 | 30 | 31 | 32 | 41 | 54 | 67 | 80 | 39 | 52 | 65 | 78 | 40 | 53 | 66
                    | 79 | 45 | 58 | 71 | 84 | 33 | 46 | 59 | 72 | 34 | 47 | 60 | 73 | 42 | 55
                    | 68 | 81 | 35 | 48 | 61 | 74 | 36 | 49 | 62 | 75 | 43 | 56 | 69 | 82 | 37
                    | 50 | 63 | 76 | 38 | 51 | 64 | 77 | 44 | 57 | 70 | 83 => {
                        if *code.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                            >= 0xc0 as ::core::ffi::c_int
                        {
                            code = code.offset(
                                *(&raw const _pcre2_utf8_table4 as *const uint8_t).offset(
                                    (*code.offset(-(1 as ::core::ffi::c_int) as isize)
                                        as ::core::ffi::c_uint
                                        & 0x3f as ::core::ffi::c_uint)
                                        as isize,
                                ) as ::core::ffi::c_int as isize,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
