// Translated from PCRE2 10.49 pcre2_script_run.c; code unit width 8.
// /*************************************************
// *      Perl-Compatible Regular Expressions       *
// *************************************************/
//
// /* PCRE is a library of functions to support regular expressions whose syntax
// and semantics are as close as possible to those of the Perl 5 language.
//
//                        Written by Philip Hazel
//      Original API code Copyright (c) 1997-2012 University of Cambridge
//           New API code Copyright (c) 2016-2021 University of Cambridge
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
// /* This module contains the function for checking a script run. */
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
    static _pcre2_ucd_digit_sets_8: [uint32_t; 0];
    static _pcre2_ucd_script_sets_8: [uint32_t; 0];
    static _pcre2_ucd_records_8: [ucd_record; 0];
    static _pcre2_ucd_stage1_8: [uint16_t; 0];
    static _pcre2_ucd_stage2_8: [uint16_t; 0];
}
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type size_t = usize;
pub type int32_t = __int32_t;
pub type BOOL = ::core::ffi::c_int;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR8 = uint8_t;
pub type PCRE2_SPTR8 = *const PCRE2_UCHAR8;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const ucp_Zs: C2RustUnnamed = 29;
pub const ucp_Zp: C2RustUnnamed = 28;
pub const ucp_Zl: C2RustUnnamed = 27;
pub const ucp_So: C2RustUnnamed = 26;
pub const ucp_Sm: C2RustUnnamed = 25;
pub const ucp_Sk: C2RustUnnamed = 24;
pub const ucp_Sc: C2RustUnnamed = 23;
pub const ucp_Ps: C2RustUnnamed = 22;
pub const ucp_Po: C2RustUnnamed = 21;
pub const ucp_Pi: C2RustUnnamed = 20;
pub const ucp_Pf: C2RustUnnamed = 19;
pub const ucp_Pe: C2RustUnnamed = 18;
pub const ucp_Pd: C2RustUnnamed = 17;
pub const ucp_Pc: C2RustUnnamed = 16;
pub const ucp_No: C2RustUnnamed = 15;
pub const ucp_Nl: C2RustUnnamed = 14;
pub const ucp_Nd: C2RustUnnamed = 13;
pub const ucp_Mn: C2RustUnnamed = 12;
pub const ucp_Me: C2RustUnnamed = 11;
pub const ucp_Mc: C2RustUnnamed = 10;
pub const ucp_Lu: C2RustUnnamed = 9;
pub const ucp_Lt: C2RustUnnamed = 8;
pub const ucp_Lo: C2RustUnnamed = 7;
pub const ucp_Lm: C2RustUnnamed = 6;
pub const ucp_Ll: C2RustUnnamed = 5;
pub const ucp_Cs: C2RustUnnamed = 4;
pub const ucp_Co: C2RustUnnamed = 3;
pub const ucp_Cn: C2RustUnnamed = 2;
pub const ucp_Cf: C2RustUnnamed = 1;
pub const ucp_Cc: C2RustUnnamed = 0;
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const ucp_Script_Count: C2RustUnnamed_0 = 175;
pub const ucp_Beria_Erfe: C2RustUnnamed_0 = 174;
pub const ucp_Tolong_Siki: C2RustUnnamed_0 = 173;
pub const ucp_Tai_Yo: C2RustUnnamed_0 = 172;
pub const ucp_Sidetic: C2RustUnnamed_0 = 171;
pub const ucp_Kirat_Rai: C2RustUnnamed_0 = 170;
pub const ucp_Nag_Mundari: C2RustUnnamed_0 = 169;
pub const ucp_Kawi: C2RustUnnamed_0 = 168;
pub const ucp_Vithkuqi: C2RustUnnamed_0 = 167;
pub const ucp_Tangsa: C2RustUnnamed_0 = 166;
pub const ucp_Khitan_Small_Script: C2RustUnnamed_0 = 165;
pub const ucp_Dives_Akuru: C2RustUnnamed_0 = 164;
pub const ucp_Chorasmian: C2RustUnnamed_0 = 163;
pub const ucp_Wancho: C2RustUnnamed_0 = 162;
pub const ucp_Nyiakeng_Puachue_Hmong: C2RustUnnamed_0 = 161;
pub const ucp_Elymaic: C2RustUnnamed_0 = 160;
pub const ucp_Old_Sogdian: C2RustUnnamed_0 = 159;
pub const ucp_Medefaidrin: C2RustUnnamed_0 = 158;
pub const ucp_Makasar: C2RustUnnamed_0 = 157;
pub const ucp_Zanabazar_Square: C2RustUnnamed_0 = 156;
pub const ucp_Soyombo: C2RustUnnamed_0 = 155;
pub const ucp_Nushu: C2RustUnnamed_0 = 154;
pub const ucp_Marchen: C2RustUnnamed_0 = 153;
pub const ucp_Bhaiksuki: C2RustUnnamed_0 = 152;
pub const ucp_SignWriting: C2RustUnnamed_0 = 151;
pub const ucp_Hatran: C2RustUnnamed_0 = 150;
pub const ucp_Anatolian_Hieroglyphs: C2RustUnnamed_0 = 149;
pub const ucp_Ahom: C2RustUnnamed_0 = 148;
pub const ucp_Warang_Citi: C2RustUnnamed_0 = 147;
pub const ucp_Siddham: C2RustUnnamed_0 = 146;
pub const ucp_Pau_Cin_Hau: C2RustUnnamed_0 = 145;
pub const ucp_Palmyrene: C2RustUnnamed_0 = 144;
pub const ucp_Nabataean: C2RustUnnamed_0 = 143;
pub const ucp_Old_North_Arabian: C2RustUnnamed_0 = 142;
pub const ucp_Mro: C2RustUnnamed_0 = 141;
pub const ucp_Mende_Kikakui: C2RustUnnamed_0 = 140;
pub const ucp_Pahawh_Hmong: C2RustUnnamed_0 = 139;
pub const ucp_Bassa_Vah: C2RustUnnamed_0 = 138;
pub const ucp_Sora_Sompeng: C2RustUnnamed_0 = 137;
pub const ucp_Miao: C2RustUnnamed_0 = 136;
pub const ucp_Meroitic_Cursive: C2RustUnnamed_0 = 135;
pub const ucp_Brahmi: C2RustUnnamed_0 = 134;
pub const ucp_Batak: C2RustUnnamed_0 = 133;
pub const ucp_Inscriptional_Pahlavi: C2RustUnnamed_0 = 132;
pub const ucp_Inscriptional_Parthian: C2RustUnnamed_0 = 131;
pub const ucp_Old_South_Arabian: C2RustUnnamed_0 = 130;
pub const ucp_Imperial_Aramaic: C2RustUnnamed_0 = 129;
pub const ucp_Meetei_Mayek: C2RustUnnamed_0 = 128;
pub const ucp_Bamum: C2RustUnnamed_0 = 127;
pub const ucp_Egyptian_Hieroglyphs: C2RustUnnamed_0 = 126;
pub const ucp_Tai_Viet: C2RustUnnamed_0 = 125;
pub const ucp_Tai_Tham: C2RustUnnamed_0 = 124;
pub const ucp_Cham: C2RustUnnamed_0 = 123;
pub const ucp_Rejang: C2RustUnnamed_0 = 122;
pub const ucp_Saurashtra: C2RustUnnamed_0 = 121;
pub const ucp_Vai: C2RustUnnamed_0 = 120;
pub const ucp_Ol_Chiki: C2RustUnnamed_0 = 119;
pub const ucp_Lepcha: C2RustUnnamed_0 = 118;
pub const ucp_Sundanese: C2RustUnnamed_0 = 117;
pub const ucp_Phoenician: C2RustUnnamed_0 = 116;
pub const ucp_Cuneiform: C2RustUnnamed_0 = 115;
pub const ucp_Balinese: C2RustUnnamed_0 = 114;
pub const ucp_Kharoshthi: C2RustUnnamed_0 = 113;
pub const ucp_Old_Persian: C2RustUnnamed_0 = 112;
pub const ucp_New_Tai_Lue: C2RustUnnamed_0 = 111;
pub const ucp_Braille: C2RustUnnamed_0 = 110;
pub const ucp_Osmanya: C2RustUnnamed_0 = 109;
pub const ucp_Ugaritic: C2RustUnnamed_0 = 108;
pub const ucp_Inherited: C2RustUnnamed_0 = 107;
pub const ucp_Deseret: C2RustUnnamed_0 = 106;
pub const ucp_Old_Italic: C2RustUnnamed_0 = 105;
pub const ucp_Khmer: C2RustUnnamed_0 = 104;
pub const ucp_Ogham: C2RustUnnamed_0 = 103;
pub const ucp_Canadian_Aboriginal: C2RustUnnamed_0 = 102;
pub const ucp_Lao: C2RustUnnamed_0 = 101;
pub const ucp_Common: C2RustUnnamed_0 = 100;
pub const ucp_Unknown: C2RustUnnamed_0 = 99;
pub const ucp_Tulu_Tigalari: C2RustUnnamed_0 = 98;
pub const ucp_Todhri: C2RustUnnamed_0 = 97;
pub const ucp_Sunuwar: C2RustUnnamed_0 = 96;
pub const ucp_Ol_Onal: C2RustUnnamed_0 = 95;
pub const ucp_Gurung_Khema: C2RustUnnamed_0 = 94;
pub const ucp_Garay: C2RustUnnamed_0 = 93;
pub const ucp_Toto: C2RustUnnamed_0 = 92;
pub const ucp_Old_Uyghur: C2RustUnnamed_0 = 91;
pub const ucp_Cypro_Minoan: C2RustUnnamed_0 = 90;
pub const ucp_Yezidi: C2RustUnnamed_0 = 89;
pub const ucp_Nandinagari: C2RustUnnamed_0 = 88;
pub const ucp_Sogdian: C2RustUnnamed_0 = 87;
pub const ucp_Hanifi_Rohingya: C2RustUnnamed_0 = 86;
pub const ucp_Gunjala_Gondi: C2RustUnnamed_0 = 85;
pub const ucp_Dogra: C2RustUnnamed_0 = 84;
pub const ucp_Masaram_Gondi: C2RustUnnamed_0 = 83;
pub const ucp_Tangut: C2RustUnnamed_0 = 82;
pub const ucp_Osage: C2RustUnnamed_0 = 81;
pub const ucp_Newa: C2RustUnnamed_0 = 80;
pub const ucp_Adlam: C2RustUnnamed_0 = 79;
pub const ucp_Old_Hungarian: C2RustUnnamed_0 = 78;
pub const ucp_Multani: C2RustUnnamed_0 = 77;
pub const ucp_Tirhuta: C2RustUnnamed_0 = 76;
pub const ucp_Khudawadi: C2RustUnnamed_0 = 75;
pub const ucp_Psalter_Pahlavi: C2RustUnnamed_0 = 74;
pub const ucp_Old_Permic: C2RustUnnamed_0 = 73;
pub const ucp_Modi: C2RustUnnamed_0 = 72;
pub const ucp_Manichaean: C2RustUnnamed_0 = 71;
pub const ucp_Mahajani: C2RustUnnamed_0 = 70;
pub const ucp_Linear_A: C2RustUnnamed_0 = 69;
pub const ucp_Khojki: C2RustUnnamed_0 = 68;
pub const ucp_Grantha: C2RustUnnamed_0 = 67;
pub const ucp_Elbasan: C2RustUnnamed_0 = 66;
pub const ucp_Duployan: C2RustUnnamed_0 = 65;
pub const ucp_Caucasian_Albanian: C2RustUnnamed_0 = 64;
pub const ucp_Takri: C2RustUnnamed_0 = 63;
pub const ucp_Sharada: C2RustUnnamed_0 = 62;
pub const ucp_Meroitic_Hieroglyphs: C2RustUnnamed_0 = 61;
pub const ucp_Chakma: C2RustUnnamed_0 = 60;
pub const ucp_Mandaic: C2RustUnnamed_0 = 59;
pub const ucp_Kaithi: C2RustUnnamed_0 = 58;
pub const ucp_Old_Turkic: C2RustUnnamed_0 = 57;
pub const ucp_Javanese: C2RustUnnamed_0 = 56;
pub const ucp_Lisu: C2RustUnnamed_0 = 55;
pub const ucp_Samaritan: C2RustUnnamed_0 = 54;
pub const ucp_Avestan: C2RustUnnamed_0 = 53;
pub const ucp_Lydian: C2RustUnnamed_0 = 52;
pub const ucp_Carian: C2RustUnnamed_0 = 51;
pub const ucp_Lycian: C2RustUnnamed_0 = 50;
pub const ucp_Kayah_Li: C2RustUnnamed_0 = 49;
pub const ucp_Nko: C2RustUnnamed_0 = 48;
pub const ucp_Phags_Pa: C2RustUnnamed_0 = 47;
pub const ucp_Syloti_Nagri: C2RustUnnamed_0 = 46;
pub const ucp_Tifinagh: C2RustUnnamed_0 = 45;
pub const ucp_Glagolitic: C2RustUnnamed_0 = 44;
pub const ucp_Coptic: C2RustUnnamed_0 = 43;
pub const ucp_Buginese: C2RustUnnamed_0 = 42;
pub const ucp_Cypriot: C2RustUnnamed_0 = 41;
pub const ucp_Shavian: C2RustUnnamed_0 = 40;
pub const ucp_Linear_B: C2RustUnnamed_0 = 39;
pub const ucp_Tai_Le: C2RustUnnamed_0 = 38;
pub const ucp_Limbu: C2RustUnnamed_0 = 37;
pub const ucp_Tagbanwa: C2RustUnnamed_0 = 36;
pub const ucp_Buhid: C2RustUnnamed_0 = 35;
pub const ucp_Hanunoo: C2RustUnnamed_0 = 34;
pub const ucp_Tagalog: C2RustUnnamed_0 = 33;
pub const ucp_Gothic: C2RustUnnamed_0 = 32;
pub const ucp_Yi: C2RustUnnamed_0 = 31;
pub const ucp_Han: C2RustUnnamed_0 = 30;
pub const ucp_Bopomofo: C2RustUnnamed_0 = 29;
pub const ucp_Katakana: C2RustUnnamed_0 = 28;
pub const ucp_Hiragana: C2RustUnnamed_0 = 27;
pub const ucp_Mongolian: C2RustUnnamed_0 = 26;
pub const ucp_Runic: C2RustUnnamed_0 = 25;
pub const ucp_Cherokee: C2RustUnnamed_0 = 24;
pub const ucp_Ethiopic: C2RustUnnamed_0 = 23;
pub const ucp_Hangul: C2RustUnnamed_0 = 22;
pub const ucp_Georgian: C2RustUnnamed_0 = 21;
pub const ucp_Myanmar: C2RustUnnamed_0 = 20;
pub const ucp_Tibetan: C2RustUnnamed_0 = 19;
pub const ucp_Thai: C2RustUnnamed_0 = 18;
pub const ucp_Sinhala: C2RustUnnamed_0 = 17;
pub const ucp_Malayalam: C2RustUnnamed_0 = 16;
pub const ucp_Kannada: C2RustUnnamed_0 = 15;
pub const ucp_Telugu: C2RustUnnamed_0 = 14;
pub const ucp_Tamil: C2RustUnnamed_0 = 13;
pub const ucp_Oriya: C2RustUnnamed_0 = 12;
pub const ucp_Gujarati: C2RustUnnamed_0 = 11;
pub const ucp_Gurmukhi: C2RustUnnamed_0 = 10;
pub const ucp_Bengali: C2RustUnnamed_0 = 9;
pub const ucp_Devanagari: C2RustUnnamed_0 = 8;
pub const ucp_Thaana: C2RustUnnamed_0 = 7;
pub const ucp_Syriac: C2RustUnnamed_0 = 6;
pub const ucp_Arabic: C2RustUnnamed_0 = 5;
pub const ucp_Hebrew: C2RustUnnamed_0 = 4;
pub const ucp_Armenian: C2RustUnnamed_0 = 3;
pub const ucp_Cyrillic: C2RustUnnamed_0 = 2;
pub const ucp_Greek: C2RustUnnamed_0 = 1;
pub const ucp_Latin: C2RustUnnamed_0 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ucd_record {
    pub script: uint8_t,
    pub chartype: uint8_t,
    pub gbprop: uint8_t,
    pub caseset: uint8_t,
    pub other_case: int32_t,
    pub scriptx_bidiclass: uint16_t,
    pub bprops: uint16_t,
}
pub const SCRIPT_HANHANGUL: C2RustUnnamed_1 = 5;
pub const SCRIPT_UNSET: C2RustUnnamed_1 = 0;
pub const SCRIPT_HANBOPOMOFO: C2RustUnnamed_1 = 4;
pub const SCRIPT_HANHIRAKATA: C2RustUnnamed_1 = 3;
pub const SCRIPT_HANPENDING: C2RustUnnamed_1 = 2;
pub const SCRIPT_MAP: C2RustUnnamed_1 = 1;
pub type C2RustUnnamed_1 = ::core::ffi::c_uint;
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const UCD_SCRIPTX_MASK: ::core::ffi::c_int = 0x3ff as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn _pcre2_script_run_8(
    mut ptr: PCRE2_SPTR8,
    mut endptr: PCRE2_SPTR8,
    mut utf: BOOL,
) -> BOOL {
    let mut require_state: uint32_t = SCRIPT_UNSET as ::core::ffi::c_int as uint32_t;
    let mut require_map: [uint32_t; 6] = [0; 6];
    let mut map: [uint32_t; 6] = [0; 6];
    let mut require_digitset: uint32_t = 0 as uint32_t;
    let mut c: uint32_t = 0;
    if ptr >= endptr {
        return TRUE;
    }
    let fresh0 = ptr;
    ptr = ptr.offset(1);
    c = *fresh0 as uint32_t;
    if utf != 0 && c >= 0xc0 as uint32_t {
        if c & 0x20 as uint32_t == 0 as uint32_t {
            let fresh1 = ptr;
            ptr = ptr.offset(1);
            c = (c & 0x1f as uint32_t) << 6 as ::core::ffi::c_int
                | *fresh1 as uint32_t & 0x3f as uint32_t;
        } else if c & 0x10 as uint32_t == 0 as uint32_t {
            c = (c & 0xf as uint32_t) << 12 as ::core::ffi::c_int
                | (*ptr as uint32_t & 0x3f as uint32_t) << 6 as ::core::ffi::c_int
                | *ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
            ptr = ptr.offset(2 as ::core::ffi::c_int as isize);
        } else if c & 0x8 as uint32_t == 0 as uint32_t {
            c = (c & 0x7 as uint32_t) << 18 as ::core::ffi::c_int
                | (*ptr as uint32_t & 0x3f as uint32_t) << 12 as ::core::ffi::c_int
                | (*ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                    << 6 as ::core::ffi::c_int
                | *ptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
            ptr = ptr.offset(3 as ::core::ffi::c_int as isize);
        } else if c & 0x4 as uint32_t == 0 as uint32_t {
            c = (c & 0x3 as uint32_t) << 24 as ::core::ffi::c_int
                | (*ptr as uint32_t & 0x3f as uint32_t) << 18 as ::core::ffi::c_int
                | (*ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                    << 12 as ::core::ffi::c_int
                | (*ptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                    << 6 as ::core::ffi::c_int
                | *ptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
            ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
        } else {
            c = (c & 0x1 as uint32_t) << 30 as ::core::ffi::c_int
                | (*ptr as uint32_t & 0x3f as uint32_t) << 24 as ::core::ffi::c_int
                | (*ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                    << 18 as ::core::ffi::c_int
                | (*ptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                    << 12 as ::core::ffi::c_int
                | (*ptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t)
                    << 6 as ::core::ffi::c_int
                | *ptr.offset(4 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
            ptr = ptr.offset(5 as ::core::ffi::c_int as isize);
        }
    }
    if ptr >= endptr {
        return TRUE;
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < ucp_Script_Count as ::core::ffi::c_int / 32 as ::core::ffi::c_int
        + 1 as ::core::ffi::c_int
    {
        require_map[i as usize] = 0 as uint32_t;
        i += 1;
    }
    loop {
        let mut ucd: *const ucd_record = (&raw const _pcre2_ucd_records_8 as *const ucd_record)
            .offset(
                *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t).offset(
                    (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                        .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                        as ::core::ffi::c_int
                        * UCD_BLOCK_SIZE
                        + c as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                ) as ::core::ffi::c_int as isize,
            );
        let mut script: uint32_t = (*ucd).script as uint32_t;
        if script == ucp_Unknown as ::core::ffi::c_int as uint32_t {
            return FALSE;
        }
        if (*ucd).scriptx_bidiclass as ::core::ffi::c_int & UCD_SCRIPTX_MASK
            != 0 as ::core::ffi::c_int
            || script != ucp_Inherited as ::core::ffi::c_int as uint32_t
                && script != ucp_Common as ::core::ffi::c_int as uint32_t
        {
            let mut OK: BOOL = 0;
            memcpy(
                &raw mut map as *mut uint32_t as *mut ::core::ffi::c_void,
                (&raw const _pcre2_ucd_script_sets_8 as *const uint32_t).offset(
                    ((*ucd).scriptx_bidiclass as ::core::ffi::c_int & UCD_SCRIPTX_MASK) as isize,
                ) as *const ::core::ffi::c_void,
                ((ucp_Unknown as ::core::ffi::c_int / 32 as ::core::ffi::c_int
                    + 1 as ::core::ffi::c_int) as size_t)
                    .wrapping_mul(::core::mem::size_of::<uint32_t>() as size_t),
            );
            memset(
                (&raw mut map as *mut uint32_t).offset(
                    (ucp_Unknown as ::core::ffi::c_int / 32 as ::core::ffi::c_int
                        + 1 as ::core::ffi::c_int) as isize,
                ) as *mut ::core::ffi::c_void,
                0 as ::core::ffi::c_int,
                ((ucp_Script_Count as ::core::ffi::c_int / 32 as ::core::ffi::c_int
                    + 1 as ::core::ffi::c_int
                    - (ucp_Unknown as ::core::ffi::c_int / 32 as ::core::ffi::c_int
                        + 1 as ::core::ffi::c_int)) as size_t)
                    .wrapping_mul(::core::mem::size_of::<uint32_t>() as size_t),
            );
            if script != ucp_Common as ::core::ffi::c_int as uint32_t
                && script != ucp_Inherited as ::core::ffi::c_int as uint32_t
            {
                map[script.wrapping_div(32 as uint32_t) as usize] =
                    (map[script.wrapping_div(32 as uint32_t) as usize] as ::core::ffi::c_uint
                        | (1 as ::core::ffi::c_uint) << script.wrapping_rem(32 as uint32_t))
                        as uint32_t;
            }
            match require_state {
                0 => match script {
                    30 => {
                        require_state = SCRIPT_HANPENDING as ::core::ffi::c_int as uint32_t;
                    }
                    27 | 28 => {
                        require_state = SCRIPT_HANHIRAKATA as ::core::ffi::c_int as uint32_t;
                    }
                    29 => {
                        require_state = SCRIPT_HANBOPOMOFO as ::core::ffi::c_int as uint32_t;
                    }
                    22 => {
                        require_state = SCRIPT_HANHANGUL as ::core::ffi::c_int as uint32_t;
                    }
                    _ => {
                        memcpy(
                            &raw mut require_map as *mut uint32_t as *mut ::core::ffi::c_void,
                            &raw mut map as *mut uint32_t as *const ::core::ffi::c_void,
                            ((ucp_Script_Count as ::core::ffi::c_int / 32 as ::core::ffi::c_int
                                + 1 as ::core::ffi::c_int) as size_t)
                                .wrapping_mul(::core::mem::size_of::<uint32_t>() as size_t),
                        );
                        require_state = SCRIPT_MAP as ::core::ffi::c_int as uint32_t;
                    }
                },
                2 => {
                    if script != ucp_Han as ::core::ffi::c_int as uint32_t {
                        let mut chspecial: uint32_t = 0 as uint32_t;
                        if map[(ucp_Bopomofo as ::core::ffi::c_int / 32 as ::core::ffi::c_int)
                            as usize]
                            & (1 as uint32_t)
                                << ucp_Bopomofo as ::core::ffi::c_int % 32 as ::core::ffi::c_int
                            != 0 as uint32_t
                        {
                            chspecial |= FOUND_BOPOMOFO as uint32_t;
                        }
                        if map[(ucp_Hiragana as ::core::ffi::c_int / 32 as ::core::ffi::c_int)
                            as usize]
                            & (1 as uint32_t)
                                << ucp_Hiragana as ::core::ffi::c_int % 32 as ::core::ffi::c_int
                            != 0 as uint32_t
                        {
                            chspecial |= FOUND_HIRAGANA as uint32_t;
                        }
                        if map[(ucp_Katakana as ::core::ffi::c_int / 32 as ::core::ffi::c_int)
                            as usize]
                            & (1 as uint32_t)
                                << ucp_Katakana as ::core::ffi::c_int % 32 as ::core::ffi::c_int
                            != 0 as uint32_t
                        {
                            chspecial |= FOUND_KATAKANA as uint32_t;
                        }
                        if map
                            [(ucp_Hangul as ::core::ffi::c_int / 32 as ::core::ffi::c_int) as usize]
                            & (1 as uint32_t)
                                << ucp_Hangul as ::core::ffi::c_int % 32 as ::core::ffi::c_int
                            != 0 as uint32_t
                        {
                            chspecial |= FOUND_HANGUL as uint32_t;
                        }
                        if chspecial == 0 as uint32_t {
                            return FALSE;
                        }
                        if chspecial == FOUND_BOPOMOFO as uint32_t {
                            require_state = SCRIPT_HANBOPOMOFO as ::core::ffi::c_int as uint32_t;
                        } else if chspecial == (FOUND_HIRAGANA | FOUND_KATAKANA) as uint32_t {
                            require_state = SCRIPT_HANHIRAKATA as ::core::ffi::c_int as uint32_t;
                        }
                    }
                }
                3 => {
                    if (map[(ucp_Han as ::core::ffi::c_int / 32 as ::core::ffi::c_int) as usize]
                        & (1 as uint32_t)
                            << ucp_Han as ::core::ffi::c_int % 32 as ::core::ffi::c_int)
                        .wrapping_add(
                            map[(ucp_Hiragana as ::core::ffi::c_int / 32 as ::core::ffi::c_int)
                                as usize]
                                & (1 as uint32_t)
                                    << ucp_Hiragana as ::core::ffi::c_int
                                        % 32 as ::core::ffi::c_int,
                        )
                        .wrapping_add(
                            map[(ucp_Katakana as ::core::ffi::c_int / 32 as ::core::ffi::c_int)
                                as usize]
                                & (1 as uint32_t)
                                    << ucp_Katakana as ::core::ffi::c_int
                                        % 32 as ::core::ffi::c_int,
                        )
                        == 0 as uint32_t
                    {
                        return FALSE;
                    }
                }
                4 => {
                    if (map[(ucp_Han as ::core::ffi::c_int / 32 as ::core::ffi::c_int) as usize]
                        & (1 as uint32_t)
                            << ucp_Han as ::core::ffi::c_int % 32 as ::core::ffi::c_int)
                        .wrapping_add(
                            map[(ucp_Bopomofo as ::core::ffi::c_int / 32 as ::core::ffi::c_int)
                                as usize]
                                & (1 as uint32_t)
                                    << ucp_Bopomofo as ::core::ffi::c_int
                                        % 32 as ::core::ffi::c_int,
                        )
                        == 0 as uint32_t
                    {
                        return FALSE;
                    }
                }
                5 => {
                    if (map[(ucp_Han as ::core::ffi::c_int / 32 as ::core::ffi::c_int) as usize]
                        & (1 as uint32_t)
                            << ucp_Han as ::core::ffi::c_int % 32 as ::core::ffi::c_int)
                        .wrapping_add(
                            map[(ucp_Hangul as ::core::ffi::c_int / 32 as ::core::ffi::c_int)
                                as usize]
                                & (1 as uint32_t)
                                    << ucp_Hangul as ::core::ffi::c_int % 32 as ::core::ffi::c_int,
                        )
                        == 0 as uint32_t
                    {
                        return FALSE;
                    }
                }
                1 => {
                    OK = FALSE as BOOL;
                    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                    while i_0
                        < ucp_Script_Count as ::core::ffi::c_int / 32 as ::core::ffi::c_int
                            + 1 as ::core::ffi::c_int
                    {
                        if require_map[i_0 as usize] & map[i_0 as usize] != 0 as uint32_t {
                            OK = TRUE as BOOL;
                            break;
                        } else {
                            i_0 += 1;
                        }
                    }
                    if OK == 0 {
                        return FALSE;
                    }
                    match script {
                        30 => {
                            require_state = SCRIPT_HANPENDING as ::core::ffi::c_int as uint32_t;
                        }
                        27 | 28 => {
                            require_state = SCRIPT_HANHIRAKATA as ::core::ffi::c_int as uint32_t;
                        }
                        29 => {
                            require_state = SCRIPT_HANBOPOMOFO as ::core::ffi::c_int as uint32_t;
                        }
                        22 => {
                            require_state = SCRIPT_HANHANGUL as ::core::ffi::c_int as uint32_t;
                        }
                        _ => {
                            let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                            while i_1
                                < ucp_Script_Count as ::core::ffi::c_int / 32 as ::core::ffi::c_int
                                    + 1 as ::core::ffi::c_int
                            {
                                require_map[i_1 as usize] &= map[i_1 as usize];
                                i_1 += 1;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if (*ucd).chartype as ::core::ffi::c_int == ucp_Nd as ::core::ffi::c_int {
            let mut digitset: uint32_t = 0;
            if c <= *(&raw const _pcre2_ucd_digit_sets_8 as *const uint32_t)
                .offset(1 as ::core::ffi::c_int as isize)
            {
                digitset = 1 as uint32_t;
            } else {
                let mut mid: ::core::ffi::c_int = 0;
                let mut bot: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
                let mut top: ::core::ffi::c_int = *(&raw const _pcre2_ucd_digit_sets_8
                    as *const uint32_t)
                    .offset(0 as ::core::ffi::c_int as isize)
                    as ::core::ffi::c_int;
                loop {
                    if top <= bot + 1 as ::core::ffi::c_int {
                        digitset = top as uint32_t;
                        break;
                    } else {
                        mid = (top + bot) / 2 as ::core::ffi::c_int;
                        if c <= *(&raw const _pcre2_ucd_digit_sets_8 as *const uint32_t)
                            .offset(mid as isize)
                        {
                            top = mid;
                        } else {
                            bot = mid;
                        }
                    }
                }
            }
            if require_digitset == 0 as uint32_t {
                require_digitset = digitset;
            } else if digitset != require_digitset {
                return FALSE;
            }
        }
        if ptr >= endptr {
            return TRUE;
        }
        let fresh2 = ptr;
        ptr = ptr.offset(1);
        c = *fresh2 as uint32_t;
        if utf != 0 && c >= 0xc0 as uint32_t {
            if c & 0x20 as uint32_t == 0 as uint32_t {
                let fresh3 = ptr;
                ptr = ptr.offset(1);
                c = (c & 0x1f as uint32_t) << 6 as ::core::ffi::c_int
                    | *fresh3 as uint32_t & 0x3f as uint32_t;
            } else if c & 0x10 as uint32_t == 0 as uint32_t {
                c = (c & 0xf as uint32_t) << 12 as ::core::ffi::c_int
                    | (*ptr as uint32_t & 0x3f as uint32_t) << 6 as ::core::ffi::c_int
                    | *ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
                ptr = ptr.offset(2 as ::core::ffi::c_int as isize);
            } else if c & 0x8 as uint32_t == 0 as uint32_t {
                c = (c & 0x7 as uint32_t) << 18 as ::core::ffi::c_int
                    | (*ptr as uint32_t & 0x3f as uint32_t) << 12 as ::core::ffi::c_int
                    | (*ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                        & 0x3f as uint32_t)
                        << 6 as ::core::ffi::c_int
                    | *ptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
                ptr = ptr.offset(3 as ::core::ffi::c_int as isize);
            } else if c & 0x4 as uint32_t == 0 as uint32_t {
                c = (c & 0x3 as uint32_t) << 24 as ::core::ffi::c_int
                    | (*ptr as uint32_t & 0x3f as uint32_t) << 18 as ::core::ffi::c_int
                    | (*ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                        & 0x3f as uint32_t)
                        << 12 as ::core::ffi::c_int
                    | (*ptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                        & 0x3f as uint32_t)
                        << 6 as ::core::ffi::c_int
                    | *ptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
                ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
            } else {
                c = (c & 0x1 as uint32_t) << 30 as ::core::ffi::c_int
                    | (*ptr as uint32_t & 0x3f as uint32_t) << 24 as ::core::ffi::c_int
                    | (*ptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                        & 0x3f as uint32_t)
                        << 18 as ::core::ffi::c_int
                    | (*ptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                        & 0x3f as uint32_t)
                        << 12 as ::core::ffi::c_int
                    | (*ptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                        & 0x3f as uint32_t)
                        << 6 as ::core::ffi::c_int
                    | *ptr.offset(4 as ::core::ffi::c_int as isize) as uint32_t & 0x3f as uint32_t;
                ptr = ptr.offset(5 as ::core::ffi::c_int as isize);
            }
        }
    }
}
pub const FOUND_BOPOMOFO: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const FOUND_HIRAGANA: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const FOUND_KATAKANA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const FOUND_HANGUL: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
