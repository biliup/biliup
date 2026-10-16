// Translated from PCRE2 10.49 pcre2_tables.c; code unit width 8.
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
// /* This module contains some fixed tables that are used by more than one of the
// PCRE2 code modules. The tables are also #included by the pcre2test program,
// which uses macros to change their names from _pcre2_xxx to xxxx, thereby
// avoiding name clashes with the library. In this case, PCRE2_PCRE2TEST is
// defined. */
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type size_t = usize;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type PCRE2_UCHAR8 = uint8_t;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const ucp_Z: C2RustUnnamed = 6;
pub const ucp_S: C2RustUnnamed = 5;
pub const ucp_P: C2RustUnnamed = 4;
pub const ucp_N: C2RustUnnamed = 3;
pub const ucp_M: C2RustUnnamed = 2;
pub const ucp_L: C2RustUnnamed = 1;
pub const ucp_C: C2RustUnnamed = 0;
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const ucp_Zs: C2RustUnnamed_0 = 29;
pub const ucp_Zp: C2RustUnnamed_0 = 28;
pub const ucp_Zl: C2RustUnnamed_0 = 27;
pub const ucp_So: C2RustUnnamed_0 = 26;
pub const ucp_Sm: C2RustUnnamed_0 = 25;
pub const ucp_Sk: C2RustUnnamed_0 = 24;
pub const ucp_Sc: C2RustUnnamed_0 = 23;
pub const ucp_Ps: C2RustUnnamed_0 = 22;
pub const ucp_Po: C2RustUnnamed_0 = 21;
pub const ucp_Pi: C2RustUnnamed_0 = 20;
pub const ucp_Pf: C2RustUnnamed_0 = 19;
pub const ucp_Pe: C2RustUnnamed_0 = 18;
pub const ucp_Pd: C2RustUnnamed_0 = 17;
pub const ucp_Pc: C2RustUnnamed_0 = 16;
pub const ucp_No: C2RustUnnamed_0 = 15;
pub const ucp_Nl: C2RustUnnamed_0 = 14;
pub const ucp_Nd: C2RustUnnamed_0 = 13;
pub const ucp_Mn: C2RustUnnamed_0 = 12;
pub const ucp_Me: C2RustUnnamed_0 = 11;
pub const ucp_Mc: C2RustUnnamed_0 = 10;
pub const ucp_Lu: C2RustUnnamed_0 = 9;
pub const ucp_Lt: C2RustUnnamed_0 = 8;
pub const ucp_Lo: C2RustUnnamed_0 = 7;
pub const ucp_Lm: C2RustUnnamed_0 = 6;
pub const ucp_Ll: C2RustUnnamed_0 = 5;
pub const ucp_Cs: C2RustUnnamed_0 = 4;
pub const ucp_Co: C2RustUnnamed_0 = 3;
pub const ucp_Cn: C2RustUnnamed_0 = 2;
pub const ucp_Cf: C2RustUnnamed_0 = 1;
pub const ucp_Cc: C2RustUnnamed_0 = 0;
pub type C2RustUnnamed_1 = ::core::ffi::c_uint;
pub const ucp_Bprop_Count: C2RustUnnamed_1 = 57;
pub const ucp_XID_Start: C2RustUnnamed_1 = 56;
pub const ucp_XID_Continue: C2RustUnnamed_1 = 55;
pub const ucp_White_Space: C2RustUnnamed_1 = 54;
pub const ucp_Variation_Selector: C2RustUnnamed_1 = 53;
pub const ucp_Uppercase: C2RustUnnamed_1 = 52;
pub const ucp_Unified_Ideograph: C2RustUnnamed_1 = 51;
pub const ucp_Terminal_Punctuation: C2RustUnnamed_1 = 50;
pub const ucp_Soft_Dotted: C2RustUnnamed_1 = 49;
pub const ucp_Sentence_Terminal: C2RustUnnamed_1 = 48;
pub const ucp_Regional_Indicator: C2RustUnnamed_1 = 47;
pub const ucp_Radical: C2RustUnnamed_1 = 46;
pub const ucp_Quotation_Mark: C2RustUnnamed_1 = 45;
pub const ucp_Prepended_Concatenation_Mark: C2RustUnnamed_1 = 44;
pub const ucp_Pattern_White_Space: C2RustUnnamed_1 = 43;
pub const ucp_Pattern_Syntax: C2RustUnnamed_1 = 42;
pub const ucp_Noncharacter_Code_Point: C2RustUnnamed_1 = 41;
pub const ucp_Modifier_Combining_Mark: C2RustUnnamed_1 = 40;
pub const ucp_Math: C2RustUnnamed_1 = 39;
pub const ucp_Lowercase: C2RustUnnamed_1 = 38;
pub const ucp_Logical_Order_Exception: C2RustUnnamed_1 = 37;
pub const ucp_Join_Control: C2RustUnnamed_1 = 36;
pub const ucp_InCB: C2RustUnnamed_1 = 35;
pub const ucp_Ideographic: C2RustUnnamed_1 = 34;
pub const ucp_ID_Start: C2RustUnnamed_1 = 33;
pub const ucp_ID_Continue: C2RustUnnamed_1 = 32;
pub const ucp_ID_Compat_Math_Start: C2RustUnnamed_1 = 31;
pub const ucp_ID_Compat_Math_Continue: C2RustUnnamed_1 = 30;
pub const ucp_IDS_Unary_Operator: C2RustUnnamed_1 = 29;
pub const ucp_IDS_Trinary_Operator: C2RustUnnamed_1 = 28;
pub const ucp_IDS_Binary_Operator: C2RustUnnamed_1 = 27;
pub const ucp_Hex_Digit: C2RustUnnamed_1 = 26;
pub const ucp_Grapheme_Link: C2RustUnnamed_1 = 25;
pub const ucp_Grapheme_Extend: C2RustUnnamed_1 = 24;
pub const ucp_Grapheme_Base: C2RustUnnamed_1 = 23;
pub const ucp_Extender: C2RustUnnamed_1 = 22;
pub const ucp_Extended_Pictographic: C2RustUnnamed_1 = 21;
pub const ucp_Emoji_Presentation: C2RustUnnamed_1 = 20;
pub const ucp_Emoji_Modifier_Base: C2RustUnnamed_1 = 19;
pub const ucp_Emoji_Modifier: C2RustUnnamed_1 = 18;
pub const ucp_Emoji_Component: C2RustUnnamed_1 = 17;
pub const ucp_Emoji: C2RustUnnamed_1 = 16;
pub const ucp_Diacritic: C2RustUnnamed_1 = 15;
pub const ucp_Deprecated: C2RustUnnamed_1 = 14;
pub const ucp_Default_Ignorable_Code_Point: C2RustUnnamed_1 = 13;
pub const ucp_Dash: C2RustUnnamed_1 = 12;
pub const ucp_Changes_When_Uppercased: C2RustUnnamed_1 = 11;
pub const ucp_Changes_When_Titlecased: C2RustUnnamed_1 = 10;
pub const ucp_Changes_When_Lowercased: C2RustUnnamed_1 = 9;
pub const ucp_Changes_When_Casemapped: C2RustUnnamed_1 = 8;
pub const ucp_Changes_When_Casefolded: C2RustUnnamed_1 = 7;
pub const ucp_Cased: C2RustUnnamed_1 = 6;
pub const ucp_Case_Ignorable: C2RustUnnamed_1 = 5;
pub const ucp_Bidi_Mirrored: C2RustUnnamed_1 = 4;
pub const ucp_Bidi_Control: C2RustUnnamed_1 = 3;
pub const ucp_Alphabetic: C2RustUnnamed_1 = 2;
pub const ucp_ASCII_Hex_Digit: C2RustUnnamed_1 = 1;
pub const ucp_ASCII: C2RustUnnamed_1 = 0;
pub type C2RustUnnamed_2 = ::core::ffi::c_uint;
pub const ucp_bidiWS: C2RustUnnamed_2 = 22;
pub const ucp_bidiS: C2RustUnnamed_2 = 21;
pub const ucp_bidiRLO: C2RustUnnamed_2 = 20;
pub const ucp_bidiRLI: C2RustUnnamed_2 = 19;
pub const ucp_bidiRLE: C2RustUnnamed_2 = 18;
pub const ucp_bidiR: C2RustUnnamed_2 = 17;
pub const ucp_bidiPDI: C2RustUnnamed_2 = 16;
pub const ucp_bidiPDF: C2RustUnnamed_2 = 15;
pub const ucp_bidiON: C2RustUnnamed_2 = 14;
pub const ucp_bidiNSM: C2RustUnnamed_2 = 13;
pub const ucp_bidiLRO: C2RustUnnamed_2 = 12;
pub const ucp_bidiLRI: C2RustUnnamed_2 = 11;
pub const ucp_bidiLRE: C2RustUnnamed_2 = 10;
pub const ucp_bidiL: C2RustUnnamed_2 = 9;
pub const ucp_bidiFSI: C2RustUnnamed_2 = 8;
pub const ucp_bidiET: C2RustUnnamed_2 = 7;
pub const ucp_bidiES: C2RustUnnamed_2 = 6;
pub const ucp_bidiEN: C2RustUnnamed_2 = 5;
pub const ucp_bidiCS: C2RustUnnamed_2 = 4;
pub const ucp_bidiBN: C2RustUnnamed_2 = 3;
pub const ucp_bidiB: C2RustUnnamed_2 = 2;
pub const ucp_bidiAN: C2RustUnnamed_2 = 1;
pub const ucp_bidiAL: C2RustUnnamed_2 = 0;
pub type C2RustUnnamed_3 = ::core::ffi::c_uint;
pub const ucp_gbExtended_Pictographic: C2RustUnnamed_3 = 14;
pub const ucp_gbZWJ: C2RustUnnamed_3 = 13;
pub const ucp_gbOther: C2RustUnnamed_3 = 12;
pub const ucp_gbRegional_Indicator: C2RustUnnamed_3 = 11;
pub const ucp_gbLVT: C2RustUnnamed_3 = 10;
pub const ucp_gbLV: C2RustUnnamed_3 = 9;
pub const ucp_gbT: C2RustUnnamed_3 = 8;
pub const ucp_gbV: C2RustUnnamed_3 = 7;
pub const ucp_gbL: C2RustUnnamed_3 = 6;
pub const ucp_gbSpacingMark: C2RustUnnamed_3 = 5;
pub const ucp_gbPrepend: C2RustUnnamed_3 = 4;
pub const ucp_gbExtend: C2RustUnnamed_3 = 3;
pub const ucp_gbControl: C2RustUnnamed_3 = 2;
pub const ucp_gbLF: C2RustUnnamed_3 = 1;
pub const ucp_gbCR: C2RustUnnamed_3 = 0;
pub type C2RustUnnamed_4 = ::core::ffi::c_uint;
pub const ucp_Script_Count: C2RustUnnamed_4 = 175;
pub const ucp_Beria_Erfe: C2RustUnnamed_4 = 174;
pub const ucp_Tolong_Siki: C2RustUnnamed_4 = 173;
pub const ucp_Tai_Yo: C2RustUnnamed_4 = 172;
pub const ucp_Sidetic: C2RustUnnamed_4 = 171;
pub const ucp_Kirat_Rai: C2RustUnnamed_4 = 170;
pub const ucp_Nag_Mundari: C2RustUnnamed_4 = 169;
pub const ucp_Kawi: C2RustUnnamed_4 = 168;
pub const ucp_Vithkuqi: C2RustUnnamed_4 = 167;
pub const ucp_Tangsa: C2RustUnnamed_4 = 166;
pub const ucp_Khitan_Small_Script: C2RustUnnamed_4 = 165;
pub const ucp_Dives_Akuru: C2RustUnnamed_4 = 164;
pub const ucp_Chorasmian: C2RustUnnamed_4 = 163;
pub const ucp_Wancho: C2RustUnnamed_4 = 162;
pub const ucp_Nyiakeng_Puachue_Hmong: C2RustUnnamed_4 = 161;
pub const ucp_Elymaic: C2RustUnnamed_4 = 160;
pub const ucp_Old_Sogdian: C2RustUnnamed_4 = 159;
pub const ucp_Medefaidrin: C2RustUnnamed_4 = 158;
pub const ucp_Makasar: C2RustUnnamed_4 = 157;
pub const ucp_Zanabazar_Square: C2RustUnnamed_4 = 156;
pub const ucp_Soyombo: C2RustUnnamed_4 = 155;
pub const ucp_Nushu: C2RustUnnamed_4 = 154;
pub const ucp_Marchen: C2RustUnnamed_4 = 153;
pub const ucp_Bhaiksuki: C2RustUnnamed_4 = 152;
pub const ucp_SignWriting: C2RustUnnamed_4 = 151;
pub const ucp_Hatran: C2RustUnnamed_4 = 150;
pub const ucp_Anatolian_Hieroglyphs: C2RustUnnamed_4 = 149;
pub const ucp_Ahom: C2RustUnnamed_4 = 148;
pub const ucp_Warang_Citi: C2RustUnnamed_4 = 147;
pub const ucp_Siddham: C2RustUnnamed_4 = 146;
pub const ucp_Pau_Cin_Hau: C2RustUnnamed_4 = 145;
pub const ucp_Palmyrene: C2RustUnnamed_4 = 144;
pub const ucp_Nabataean: C2RustUnnamed_4 = 143;
pub const ucp_Old_North_Arabian: C2RustUnnamed_4 = 142;
pub const ucp_Mro: C2RustUnnamed_4 = 141;
pub const ucp_Mende_Kikakui: C2RustUnnamed_4 = 140;
pub const ucp_Pahawh_Hmong: C2RustUnnamed_4 = 139;
pub const ucp_Bassa_Vah: C2RustUnnamed_4 = 138;
pub const ucp_Sora_Sompeng: C2RustUnnamed_4 = 137;
pub const ucp_Miao: C2RustUnnamed_4 = 136;
pub const ucp_Meroitic_Cursive: C2RustUnnamed_4 = 135;
pub const ucp_Brahmi: C2RustUnnamed_4 = 134;
pub const ucp_Batak: C2RustUnnamed_4 = 133;
pub const ucp_Inscriptional_Pahlavi: C2RustUnnamed_4 = 132;
pub const ucp_Inscriptional_Parthian: C2RustUnnamed_4 = 131;
pub const ucp_Old_South_Arabian: C2RustUnnamed_4 = 130;
pub const ucp_Imperial_Aramaic: C2RustUnnamed_4 = 129;
pub const ucp_Meetei_Mayek: C2RustUnnamed_4 = 128;
pub const ucp_Bamum: C2RustUnnamed_4 = 127;
pub const ucp_Egyptian_Hieroglyphs: C2RustUnnamed_4 = 126;
pub const ucp_Tai_Viet: C2RustUnnamed_4 = 125;
pub const ucp_Tai_Tham: C2RustUnnamed_4 = 124;
pub const ucp_Cham: C2RustUnnamed_4 = 123;
pub const ucp_Rejang: C2RustUnnamed_4 = 122;
pub const ucp_Saurashtra: C2RustUnnamed_4 = 121;
pub const ucp_Vai: C2RustUnnamed_4 = 120;
pub const ucp_Ol_Chiki: C2RustUnnamed_4 = 119;
pub const ucp_Lepcha: C2RustUnnamed_4 = 118;
pub const ucp_Sundanese: C2RustUnnamed_4 = 117;
pub const ucp_Phoenician: C2RustUnnamed_4 = 116;
pub const ucp_Cuneiform: C2RustUnnamed_4 = 115;
pub const ucp_Balinese: C2RustUnnamed_4 = 114;
pub const ucp_Kharoshthi: C2RustUnnamed_4 = 113;
pub const ucp_Old_Persian: C2RustUnnamed_4 = 112;
pub const ucp_New_Tai_Lue: C2RustUnnamed_4 = 111;
pub const ucp_Braille: C2RustUnnamed_4 = 110;
pub const ucp_Osmanya: C2RustUnnamed_4 = 109;
pub const ucp_Ugaritic: C2RustUnnamed_4 = 108;
pub const ucp_Inherited: C2RustUnnamed_4 = 107;
pub const ucp_Deseret: C2RustUnnamed_4 = 106;
pub const ucp_Old_Italic: C2RustUnnamed_4 = 105;
pub const ucp_Khmer: C2RustUnnamed_4 = 104;
pub const ucp_Ogham: C2RustUnnamed_4 = 103;
pub const ucp_Canadian_Aboriginal: C2RustUnnamed_4 = 102;
pub const ucp_Lao: C2RustUnnamed_4 = 101;
pub const ucp_Common: C2RustUnnamed_4 = 100;
pub const ucp_Unknown: C2RustUnnamed_4 = 99;
pub const ucp_Tulu_Tigalari: C2RustUnnamed_4 = 98;
pub const ucp_Todhri: C2RustUnnamed_4 = 97;
pub const ucp_Sunuwar: C2RustUnnamed_4 = 96;
pub const ucp_Ol_Onal: C2RustUnnamed_4 = 95;
pub const ucp_Gurung_Khema: C2RustUnnamed_4 = 94;
pub const ucp_Garay: C2RustUnnamed_4 = 93;
pub const ucp_Toto: C2RustUnnamed_4 = 92;
pub const ucp_Old_Uyghur: C2RustUnnamed_4 = 91;
pub const ucp_Cypro_Minoan: C2RustUnnamed_4 = 90;
pub const ucp_Yezidi: C2RustUnnamed_4 = 89;
pub const ucp_Nandinagari: C2RustUnnamed_4 = 88;
pub const ucp_Sogdian: C2RustUnnamed_4 = 87;
pub const ucp_Hanifi_Rohingya: C2RustUnnamed_4 = 86;
pub const ucp_Gunjala_Gondi: C2RustUnnamed_4 = 85;
pub const ucp_Dogra: C2RustUnnamed_4 = 84;
pub const ucp_Masaram_Gondi: C2RustUnnamed_4 = 83;
pub const ucp_Tangut: C2RustUnnamed_4 = 82;
pub const ucp_Osage: C2RustUnnamed_4 = 81;
pub const ucp_Newa: C2RustUnnamed_4 = 80;
pub const ucp_Adlam: C2RustUnnamed_4 = 79;
pub const ucp_Old_Hungarian: C2RustUnnamed_4 = 78;
pub const ucp_Multani: C2RustUnnamed_4 = 77;
pub const ucp_Tirhuta: C2RustUnnamed_4 = 76;
pub const ucp_Khudawadi: C2RustUnnamed_4 = 75;
pub const ucp_Psalter_Pahlavi: C2RustUnnamed_4 = 74;
pub const ucp_Old_Permic: C2RustUnnamed_4 = 73;
pub const ucp_Modi: C2RustUnnamed_4 = 72;
pub const ucp_Manichaean: C2RustUnnamed_4 = 71;
pub const ucp_Mahajani: C2RustUnnamed_4 = 70;
pub const ucp_Linear_A: C2RustUnnamed_4 = 69;
pub const ucp_Khojki: C2RustUnnamed_4 = 68;
pub const ucp_Grantha: C2RustUnnamed_4 = 67;
pub const ucp_Elbasan: C2RustUnnamed_4 = 66;
pub const ucp_Duployan: C2RustUnnamed_4 = 65;
pub const ucp_Caucasian_Albanian: C2RustUnnamed_4 = 64;
pub const ucp_Takri: C2RustUnnamed_4 = 63;
pub const ucp_Sharada: C2RustUnnamed_4 = 62;
pub const ucp_Meroitic_Hieroglyphs: C2RustUnnamed_4 = 61;
pub const ucp_Chakma: C2RustUnnamed_4 = 60;
pub const ucp_Mandaic: C2RustUnnamed_4 = 59;
pub const ucp_Kaithi: C2RustUnnamed_4 = 58;
pub const ucp_Old_Turkic: C2RustUnnamed_4 = 57;
pub const ucp_Javanese: C2RustUnnamed_4 = 56;
pub const ucp_Lisu: C2RustUnnamed_4 = 55;
pub const ucp_Samaritan: C2RustUnnamed_4 = 54;
pub const ucp_Avestan: C2RustUnnamed_4 = 53;
pub const ucp_Lydian: C2RustUnnamed_4 = 52;
pub const ucp_Carian: C2RustUnnamed_4 = 51;
pub const ucp_Lycian: C2RustUnnamed_4 = 50;
pub const ucp_Kayah_Li: C2RustUnnamed_4 = 49;
pub const ucp_Nko: C2RustUnnamed_4 = 48;
pub const ucp_Phags_Pa: C2RustUnnamed_4 = 47;
pub const ucp_Syloti_Nagri: C2RustUnnamed_4 = 46;
pub const ucp_Tifinagh: C2RustUnnamed_4 = 45;
pub const ucp_Glagolitic: C2RustUnnamed_4 = 44;
pub const ucp_Coptic: C2RustUnnamed_4 = 43;
pub const ucp_Buginese: C2RustUnnamed_4 = 42;
pub const ucp_Cypriot: C2RustUnnamed_4 = 41;
pub const ucp_Shavian: C2RustUnnamed_4 = 40;
pub const ucp_Linear_B: C2RustUnnamed_4 = 39;
pub const ucp_Tai_Le: C2RustUnnamed_4 = 38;
pub const ucp_Limbu: C2RustUnnamed_4 = 37;
pub const ucp_Tagbanwa: C2RustUnnamed_4 = 36;
pub const ucp_Buhid: C2RustUnnamed_4 = 35;
pub const ucp_Hanunoo: C2RustUnnamed_4 = 34;
pub const ucp_Tagalog: C2RustUnnamed_4 = 33;
pub const ucp_Gothic: C2RustUnnamed_4 = 32;
pub const ucp_Yi: C2RustUnnamed_4 = 31;
pub const ucp_Han: C2RustUnnamed_4 = 30;
pub const ucp_Bopomofo: C2RustUnnamed_4 = 29;
pub const ucp_Katakana: C2RustUnnamed_4 = 28;
pub const ucp_Hiragana: C2RustUnnamed_4 = 27;
pub const ucp_Mongolian: C2RustUnnamed_4 = 26;
pub const ucp_Runic: C2RustUnnamed_4 = 25;
pub const ucp_Cherokee: C2RustUnnamed_4 = 24;
pub const ucp_Ethiopic: C2RustUnnamed_4 = 23;
pub const ucp_Hangul: C2RustUnnamed_4 = 22;
pub const ucp_Georgian: C2RustUnnamed_4 = 21;
pub const ucp_Myanmar: C2RustUnnamed_4 = 20;
pub const ucp_Tibetan: C2RustUnnamed_4 = 19;
pub const ucp_Thai: C2RustUnnamed_4 = 18;
pub const ucp_Sinhala: C2RustUnnamed_4 = 17;
pub const ucp_Malayalam: C2RustUnnamed_4 = 16;
pub const ucp_Kannada: C2RustUnnamed_4 = 15;
pub const ucp_Telugu: C2RustUnnamed_4 = 14;
pub const ucp_Tamil: C2RustUnnamed_4 = 13;
pub const ucp_Oriya: C2RustUnnamed_4 = 12;
pub const ucp_Gujarati: C2RustUnnamed_4 = 11;
pub const ucp_Gurmukhi: C2RustUnnamed_4 = 10;
pub const ucp_Bengali: C2RustUnnamed_4 = 9;
pub const ucp_Devanagari: C2RustUnnamed_4 = 8;
pub const ucp_Thaana: C2RustUnnamed_4 = 7;
pub const ucp_Syriac: C2RustUnnamed_4 = 6;
pub const ucp_Arabic: C2RustUnnamed_4 = 5;
pub const ucp_Hebrew: C2RustUnnamed_4 = 4;
pub const ucp_Armenian: C2RustUnnamed_4 = 3;
pub const ucp_Cyrillic: C2RustUnnamed_4 = 2;
pub const ucp_Greek: C2RustUnnamed_4 = 1;
pub const ucp_Latin: C2RustUnnamed_4 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ucp_type_table {
    pub name_offset: uint16_t,
    pub type_0: uint16_t,
    pub value: uint16_t,
}
pub const LINK_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const NOTACHAR: ::core::ffi::c_uint = 0xffffffff as ::core::ffi::c_uint;
pub const CHAR_HT: ::core::ffi::c_int = '\t' as i32;
pub const CHAR_VT: ::core::ffi::c_int = '\u{b}' as i32;
pub const CHAR_FF: ::core::ffi::c_int = '\u{c}' as i32;
pub const CHAR_CR: ::core::ffi::c_int = '\r' as i32;
pub const CHAR_LF: ::core::ffi::c_int = '\n' as i32;
pub const CHAR_NEL: ::core::ffi::c_uchar = -123i32 as ::core::ffi::c_uchar;
pub const CHAR_SPACE: ::core::ffi::c_int = ' ' as i32;
pub const CHAR_QUOTATION_MARK: ::core::ffi::c_int = '"' as i32;
pub const CHAR_NUMBER_SIGN: ::core::ffi::c_int = '#' as i32;
pub const CHAR_DOLLAR_SIGN: ::core::ffi::c_int = '$' as i32;
pub const CHAR_PERCENT_SIGN: ::core::ffi::c_int = '%' as i32;
pub const CHAR_APOSTROPHE: ::core::ffi::c_int = '\'' as i32;
pub const CHAR_CIRCUMFLEX_ACCENT: ::core::ffi::c_int = '^' as i32;
pub const CHAR_GRAVE_ACCENT: ::core::ffi::c_int = '`' as i32;
pub const CHAR_LEFT_CURLY_BRACKET: ::core::ffi::c_int = '{' as i32;
pub const CHAR_RIGHT_CURLY_BRACKET: ::core::ffi::c_int = '}' as i32;
pub const CHAR_NBSP: ::core::ffi::c_uchar = -96i32 as ::core::ffi::c_uchar;
pub const PT_LAMP: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PT_GC: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PT_PC: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PT_SC: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const PT_SCX: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PT_ALNUM: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const PT_SPACE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const PT_PXSPACE: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const PT_WORD: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const PT_UCNC: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const PT_BIDICL: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const PT_BOOL: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const PT_ANY: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const IMM2_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
#[no_mangle]
pub static mut _pcre2_OP_lengths_8: [uint8_t; 173] = [0; 173];
#[no_mangle]
pub static mut _pcre2_hspace_list_8: [uint32_t; 20] = [
    CHAR_HT as uint32_t,
    CHAR_SPACE as uint32_t,
    CHAR_NBSP as uint32_t,
    0x1680 as ::core::ffi::c_int as uint32_t,
    0x180e as ::core::ffi::c_int as uint32_t,
    0x2000 as ::core::ffi::c_int as uint32_t,
    0x2001 as ::core::ffi::c_int as uint32_t,
    0x2002 as ::core::ffi::c_int as uint32_t,
    0x2003 as ::core::ffi::c_int as uint32_t,
    0x2004 as ::core::ffi::c_int as uint32_t,
    0x2005 as ::core::ffi::c_int as uint32_t,
    0x2006 as ::core::ffi::c_int as uint32_t,
    0x2007 as ::core::ffi::c_int as uint32_t,
    0x2008 as ::core::ffi::c_int as uint32_t,
    0x2009 as ::core::ffi::c_int as uint32_t,
    0x200a as ::core::ffi::c_int as uint32_t,
    0x202f as ::core::ffi::c_int as uint32_t,
    0x205f as ::core::ffi::c_int as uint32_t,
    0x3000 as ::core::ffi::c_int as uint32_t,
    NOTACHAR,
];
#[no_mangle]
pub static mut _pcre2_vspace_list_8: [uint32_t; 8] = [
    CHAR_LF as uint32_t,
    CHAR_VT as uint32_t,
    CHAR_FF as uint32_t,
    CHAR_CR as uint32_t,
    CHAR_NEL as uint32_t,
    0x2028 as ::core::ffi::c_int as uint32_t,
    0x2029 as ::core::ffi::c_int as uint32_t,
    NOTACHAR,
];
#[no_mangle]
pub static mut _pcre2_callout_start_delims_8: [uint32_t; 9] = [
    CHAR_GRAVE_ACCENT as uint32_t,
    CHAR_APOSTROPHE as uint32_t,
    CHAR_QUOTATION_MARK as uint32_t,
    CHAR_CIRCUMFLEX_ACCENT as uint32_t,
    CHAR_PERCENT_SIGN as uint32_t,
    CHAR_NUMBER_SIGN as uint32_t,
    CHAR_DOLLAR_SIGN as uint32_t,
    CHAR_LEFT_CURLY_BRACKET as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
];
#[no_mangle]
pub static mut _pcre2_callout_end_delims_8: [uint32_t; 9] = [
    CHAR_GRAVE_ACCENT as uint32_t,
    CHAR_APOSTROPHE as uint32_t,
    CHAR_QUOTATION_MARK as uint32_t,
    CHAR_CIRCUMFLEX_ACCENT as uint32_t,
    CHAR_PERCENT_SIGN as uint32_t,
    CHAR_NUMBER_SIGN as uint32_t,
    CHAR_DOLLAR_SIGN as uint32_t,
    CHAR_RIGHT_CURLY_BRACKET as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
];
#[no_mangle]
pub static mut _pcre2_utf8_table1: [::core::ffi::c_int; 6] = [
    0x7f as ::core::ffi::c_int,
    0x7ff as ::core::ffi::c_int,
    0xffff as ::core::ffi::c_int,
    0x1fffff as ::core::ffi::c_int,
    0x3ffffff as ::core::ffi::c_int,
    0x7fffffff as ::core::ffi::c_int,
];
#[no_mangle]
pub static mut _pcre2_utf8_table1_size: ::core::ffi::c_uint = 0;
#[no_mangle]
pub static mut _pcre2_utf8_table2: [::core::ffi::c_int; 6] = [
    0 as ::core::ffi::c_int,
    0xc0 as ::core::ffi::c_int,
    0xe0 as ::core::ffi::c_int,
    0xf0 as ::core::ffi::c_int,
    0xf8 as ::core::ffi::c_int,
    0xfc as ::core::ffi::c_int,
];
#[no_mangle]
pub static mut _pcre2_utf8_table3: [::core::ffi::c_int; 6] = [
    0xff as ::core::ffi::c_int,
    0x1f as ::core::ffi::c_int,
    0xf as ::core::ffi::c_int,
    0x7 as ::core::ffi::c_int,
    0x3 as ::core::ffi::c_int,
    0x1 as ::core::ffi::c_int,
];
#[no_mangle]
pub static mut _pcre2_utf8_table4: [uint8_t; 64] = [
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
];
#[no_mangle]
pub static mut _pcre2_ucp_gentype_8: [uint32_t; 30] = [
    ucp_C as ::core::ffi::c_int as uint32_t,
    ucp_C as ::core::ffi::c_int as uint32_t,
    ucp_C as ::core::ffi::c_int as uint32_t,
    ucp_C as ::core::ffi::c_int as uint32_t,
    ucp_C as ::core::ffi::c_int as uint32_t,
    ucp_L as ::core::ffi::c_int as uint32_t,
    ucp_L as ::core::ffi::c_int as uint32_t,
    ucp_L as ::core::ffi::c_int as uint32_t,
    ucp_L as ::core::ffi::c_int as uint32_t,
    ucp_L as ::core::ffi::c_int as uint32_t,
    ucp_M as ::core::ffi::c_int as uint32_t,
    ucp_M as ::core::ffi::c_int as uint32_t,
    ucp_M as ::core::ffi::c_int as uint32_t,
    ucp_N as ::core::ffi::c_int as uint32_t,
    ucp_N as ::core::ffi::c_int as uint32_t,
    ucp_N as ::core::ffi::c_int as uint32_t,
    ucp_P as ::core::ffi::c_int as uint32_t,
    ucp_P as ::core::ffi::c_int as uint32_t,
    ucp_P as ::core::ffi::c_int as uint32_t,
    ucp_P as ::core::ffi::c_int as uint32_t,
    ucp_P as ::core::ffi::c_int as uint32_t,
    ucp_P as ::core::ffi::c_int as uint32_t,
    ucp_P as ::core::ffi::c_int as uint32_t,
    ucp_S as ::core::ffi::c_int as uint32_t,
    ucp_S as ::core::ffi::c_int as uint32_t,
    ucp_S as ::core::ffi::c_int as uint32_t,
    ucp_S as ::core::ffi::c_int as uint32_t,
    ucp_Z as ::core::ffi::c_int as uint32_t,
    ucp_Z as ::core::ffi::c_int as uint32_t,
    ucp_Z as ::core::ffi::c_int as uint32_t,
];
#[no_mangle]
pub static mut _pcre2_ucp_gbtable_8: [uint32_t; 15] = [
    (1 as ::core::ffi::c_uint) << ucp_gbLF as ::core::ffi::c_int,
    0 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
    ((1 as ::core::ffi::c_int) << ucp_gbExtend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbSpacingMark as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbZWJ as ::core::ffi::c_int) as uint32_t,
    ((1 as ::core::ffi::c_int) << ucp_gbExtend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbSpacingMark as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbZWJ as ::core::ffi::c_int) as ::core::ffi::c_uint
        | (1 as ::core::ffi::c_uint) << ucp_gbPrepend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbL as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbV as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbT as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbLV as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbLVT as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbOther as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbRegional_Indicator as ::core::ffi::c_int,
    ((1 as ::core::ffi::c_int) << ucp_gbExtend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbSpacingMark as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbZWJ as ::core::ffi::c_int) as uint32_t,
    ((1 as ::core::ffi::c_int) << ucp_gbExtend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbSpacingMark as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbZWJ as ::core::ffi::c_int) as ::core::ffi::c_uint
        | (1 as ::core::ffi::c_uint) << ucp_gbL as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbV as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbLV as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbLVT as ::core::ffi::c_int,
    ((1 as ::core::ffi::c_int) << ucp_gbExtend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbSpacingMark as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbZWJ as ::core::ffi::c_int) as ::core::ffi::c_uint
        | (1 as ::core::ffi::c_uint) << ucp_gbV as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbT as ::core::ffi::c_int,
    ((1 as ::core::ffi::c_int) << ucp_gbExtend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbSpacingMark as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbZWJ as ::core::ffi::c_int) as ::core::ffi::c_uint
        | (1 as ::core::ffi::c_uint) << ucp_gbT as ::core::ffi::c_int,
    ((1 as ::core::ffi::c_int) << ucp_gbExtend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbSpacingMark as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbZWJ as ::core::ffi::c_int) as ::core::ffi::c_uint
        | (1 as ::core::ffi::c_uint) << ucp_gbV as ::core::ffi::c_int
        | (1 as ::core::ffi::c_uint) << ucp_gbT as ::core::ffi::c_int,
    ((1 as ::core::ffi::c_int) << ucp_gbExtend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbSpacingMark as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbZWJ as ::core::ffi::c_int) as ::core::ffi::c_uint
        | (1 as ::core::ffi::c_uint) << ucp_gbT as ::core::ffi::c_int,
    (1 as ::core::ffi::c_uint) << ucp_gbRegional_Indicator as ::core::ffi::c_int,
    ((1 as ::core::ffi::c_int) << ucp_gbExtend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbSpacingMark as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbZWJ as ::core::ffi::c_int) as uint32_t,
    ((1 as ::core::ffi::c_int) << ucp_gbExtend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbSpacingMark as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbZWJ as ::core::ffi::c_int) as ::core::ffi::c_uint
        | (1 as ::core::ffi::c_uint) << ucp_gbExtended_Pictographic as ::core::ffi::c_int,
    ((1 as ::core::ffi::c_int) << ucp_gbExtend as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbSpacingMark as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << ucp_gbZWJ as ::core::ffi::c_int) as uint32_t,
];
#[no_mangle]
pub static mut _pcre2_ucp_typerange_8: [::core::ffi::c_int; 14] = [
    ucp_Cc as ::core::ffi::c_int,
    ucp_Cs as ::core::ffi::c_int,
    ucp_Ll as ::core::ffi::c_int,
    ucp_Lu as ::core::ffi::c_int,
    ucp_Mc as ::core::ffi::c_int,
    ucp_Mn as ::core::ffi::c_int,
    ucp_Nd as ::core::ffi::c_int,
    ucp_No as ::core::ffi::c_int,
    ucp_Pc as ::core::ffi::c_int,
    ucp_Ps as ::core::ffi::c_int,
    ucp_Sc as ::core::ffi::c_int,
    ucp_So as ::core::ffi::c_int,
    ucp_Zl as ::core::ffi::c_int,
    ucp_Zs as ::core::ffi::c_int,
];
#[no_mangle]
pub static mut _pcre2_utt_names_8: [::core::ffi::c_char; 3834] = unsafe {
    ::core::mem::transmute::<
        [u8; 3834],
        [::core::ffi::c_char; 3834],
    >(
        *b"adlam\0adlm\0aghb\0ahex\0ahom\0alpha\0alphabetic\0anatolianhieroglyphs\0any\0arab\0arabic\0armenian\0armi\0armn\0ascii\0asciihexdigit\0avestan\0avst\0bali\0balinese\0bamu\0bamum\0bass\0bassavah\0batak\0batk\0beng\0bengali\0berf\0beriaerfe\0bhaiksuki\0bhks\0bidial\0bidian\0bidib\0bidibn\0bidic\0bidicontrol\0bidics\0bidien\0bidies\0bidiet\0bidifsi\0bidil\0bidilre\0bidilri\0bidilro\0bidim\0bidimirrored\0bidinsm\0bidion\0bidipdf\0bidipdi\0bidir\0bidirle\0bidirli\0bidirlo\0bidis\0bidiws\0bopo\0bopomofo\0brah\0brahmi\0brai\0braille\0bugi\0buginese\0buhd\0buhid\0c\0cakm\0canadianaboriginal\0cans\0cari\0carian\0cased\0caseignorable\0caucasianalbanian\0cc\0cf\0chakma\0cham\0changeswhencasefolded\0changeswhencasemapped\0changeswhenlowercased\0changeswhentitlecased\0changeswhenuppercased\0cher\0cherokee\0chorasmian\0chrs\0ci\0cn\0co\0common\0copt\0coptic\0cpmn\0cprt\0cs\0cuneiform\0cwcf\0cwcm\0cwl\0cwt\0cwu\0cypriot\0cyprominoan\0cyrillic\0cyrl\0dash\0defaultignorablecodepoint\0dep\0deprecated\0deseret\0deva\0devanagari\0di\0dia\0diacritic\0diak\0divesakuru\0dogr\0dogra\0dsrt\0dupl\0duployan\0ebase\0ecomp\0egyp\0egyptianhieroglyphs\0elba\0elbasan\0elym\0elymaic\0emod\0emoji\0emojicomponent\0emojimodifier\0emojimodifierbase\0emojipresentation\0epres\0ethi\0ethiopic\0ext\0extendedpictographic\0extender\0extpict\0gara\0garay\0geor\0georgian\0glag\0glagolitic\0gong\0gonm\0goth\0gothic\0gran\0grantha\0graphemebase\0graphemeextend\0graphemelink\0grbase\0greek\0grek\0grext\0grlink\0gujarati\0gujr\0gukh\0gunjalagondi\0gurmukhi\0guru\0gurungkhema\0han\0hang\0hangul\0hani\0hanifirohingya\0hano\0hanunoo\0hatr\0hatran\0hebr\0hebrew\0hex\0hexdigit\0hira\0hiragana\0hluw\0hmng\0hmnp\0hung\0idc\0idcompatmathcontinue\0idcompatmathstart\0idcontinue\0ideo\0ideographic\0ids\0idsb\0idsbinaryoperator\0idst\0idstart\0idstrinaryoperator\0idsu\0idsunaryoperator\0imperialaramaic\0incb\0inherited\0inscriptionalpahlavi\0inscriptionalparthian\0ital\0java\0javanese\0joinc\0joincontrol\0kaithi\0kali\0kana\0kannada\0katakana\0kawi\0kayahli\0khar\0kharoshthi\0khitansmallscript\0khmer\0khmr\0khoj\0khojki\0khudawadi\0kiratrai\0kits\0knda\0krai\0kthi\0l\0l&\0lana\0lao\0laoo\0latin\0latn\0lc\0lepc\0lepcha\0limb\0limbu\0lina\0linb\0lineara\0linearb\0lisu\0ll\0lm\0lo\0loe\0logicalorderexception\0lower\0lowercase\0lt\0lu\0lyci\0lycian\0lydi\0lydian\0m\0mahajani\0mahj\0maka\0makasar\0malayalam\0mand\0mandaic\0mani\0manichaean\0marc\0marchen\0masaramgondi\0math\0mc\0mcm\0me\0medefaidrin\0medf\0meeteimayek\0mend\0mendekikakui\0merc\0mero\0meroiticcursive\0meroitichieroglyphs\0miao\0mlym\0mn\0modi\0modifiercombiningmark\0mong\0mongolian\0mro\0mroo\0mtei\0mult\0multani\0myanmar\0mymr\0n\0nabataean\0nagm\0nagmundari\0nand\0nandinagari\0narb\0nbat\0nchar\0nd\0newa\0newtailue\0nko\0nkoo\0nl\0no\0noncharactercodepoint\0nshu\0nushu\0nyiakengpuachuehmong\0ogam\0ogham\0olchiki\0olck\0oldhungarian\0olditalic\0oldnortharabian\0oldpermic\0oldpersian\0oldsogdian\0oldsoutharabian\0oldturkic\0olduyghur\0olonal\0onao\0oriya\0orkh\0orya\0osage\0osge\0osma\0osmanya\0ougr\0p\0pahawhhmong\0palm\0palmyrene\0patsyn\0patternsyntax\0patternwhitespace\0patws\0pauc\0paucinhau\0pc\0pcm\0pd\0pe\0perm\0pf\0phag\0phagspa\0phli\0phlp\0phnx\0phoenician\0pi\0plrd\0po\0prependedconcatenationmark\0prti\0ps\0psalterpahlavi\0qaac\0qaai\0qmark\0quotationmark\0radical\0regionalindicator\0rejang\0ri\0rjng\0rohg\0runic\0runr\0s\0samaritan\0samr\0sarb\0saur\0saurashtra\0sc\0sd\0sentenceterminal\0sgnw\0sharada\0shavian\0shaw\0shrd\0sidd\0siddham\0sidetic\0sidt\0signwriting\0sind\0sinh\0sinhala\0sk\0sm\0so\0softdotted\0sogd\0sogdian\0sogo\0sora\0sorasompeng\0soyo\0soyombo\0space\0sterm\0sund\0sundanese\0sunu\0sunuwar\0sylo\0sylotinagri\0syrc\0syriac\0tagalog\0tagb\0tagbanwa\0taile\0taitham\0taiviet\0taiyo\0takr\0takri\0tale\0talu\0tamil\0taml\0tang\0tangsa\0tangut\0tavt\0tayo\0telu\0telugu\0term\0terminalpunctuation\0tfng\0tglg\0thaa\0thaana\0thai\0tibetan\0tibt\0tifinagh\0tirh\0tirhuta\0tnsa\0todhri\0todr\0tolongsiki\0tols\0toto\0tulutigalari\0tutg\0ugar\0ugaritic\0uideo\0unifiedideograph\0unknown\0upper\0uppercase\0vai\0vaii\0variationselector\0vith\0vithkuqi\0vs\0wancho\0wara\0warangciti\0wcho\0whitespace\0wspace\0xan\0xidc\0xidcontinue\0xids\0xidstart\0xpeo\0xps\0xsp\0xsux\0xuc\0xwd\0yezi\0yezidi\0yi\0yiii\0z\0zanabazarsquare\0zanb\0zinh\0zl\0zp\0zs\0zyyy\0zzzz\0\0",
    )
};
#[no_mangle]
pub static mut _pcre2_utt_8: [ucp_type_table; 518] = [
    ucp_type_table {
        name_offset: 0 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Adlam as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 6 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Adlam as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 11 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Caucasian_Albanian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 16 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_ASCII_Hex_Digit as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 21 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Ahom as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 26 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Alphabetic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 32 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Alphabetic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 43 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Anatolian_Hieroglyphs as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 64 as uint16_t,
        type_0: PT_ANY as uint16_t,
        value: 0 as uint16_t,
    },
    ucp_type_table {
        name_offset: 68 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Arabic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 73 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Arabic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 80 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Armenian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 89 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Imperial_Aramaic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 94 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Armenian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 99 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_ASCII as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 105 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_ASCII_Hex_Digit as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 119 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Avestan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 127 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Avestan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 132 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Balinese as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 137 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Balinese as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 146 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Bamum as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 151 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Bamum as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 157 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Bassa_Vah as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 162 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Bassa_Vah as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 171 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Batak as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 177 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Batak as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 182 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Bengali as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 187 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Bengali as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 195 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Beria_Erfe as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 200 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Beria_Erfe as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 210 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Bhaiksuki as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 220 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Bhaiksuki as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 225 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiAL as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 232 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiAN as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 239 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiB as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 245 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiBN as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 252 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Bidi_Control as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 258 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Bidi_Control as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 270 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiCS as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 277 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiEN as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 284 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiES as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 291 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiET as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 298 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiFSI as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 306 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiL as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 312 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiLRE as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 320 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiLRI as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 328 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiLRO as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 336 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Bidi_Mirrored as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 342 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Bidi_Mirrored as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 355 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiNSM as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 363 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiON as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 370 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiPDF as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 378 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiPDI as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 386 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiR as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 392 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiRLE as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 400 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiRLI as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 408 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiRLO as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 416 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiS as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 422 as uint16_t,
        type_0: PT_BIDICL as uint16_t,
        value: ucp_bidiWS as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 429 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Bopomofo as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 434 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Bopomofo as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 443 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Brahmi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 448 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Brahmi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 455 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Braille as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 460 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Braille as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 468 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Buginese as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 473 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Buginese as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 482 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Buhid as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 487 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Buhid as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 493 as uint16_t,
        type_0: PT_GC as uint16_t,
        value: ucp_C as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 495 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Chakma as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 500 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Canadian_Aboriginal as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 519 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Canadian_Aboriginal as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 524 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Carian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 529 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Carian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 536 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Cased as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 542 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Case_Ignorable as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 556 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Caucasian_Albanian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 574 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Cc as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 577 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Cf as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 580 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Chakma as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 587 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Cham as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 592 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Changes_When_Casefolded as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 614 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Changes_When_Casemapped as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 636 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Changes_When_Lowercased as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 658 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Changes_When_Titlecased as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 680 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Changes_When_Uppercased as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 702 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Cherokee as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 707 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Cherokee as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 716 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Chorasmian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 727 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Chorasmian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 732 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Case_Ignorable as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 735 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Cn as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 738 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Co as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 741 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Common as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 748 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Coptic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 753 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Coptic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 760 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Cypro_Minoan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 765 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Cypriot as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 770 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Cs as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 773 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Cuneiform as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 783 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Changes_When_Casefolded as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 788 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Changes_When_Casemapped as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 793 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Changes_When_Lowercased as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 797 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Changes_When_Titlecased as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 801 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Changes_When_Uppercased as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 805 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Cypriot as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 813 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Cypro_Minoan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 825 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Cyrillic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 834 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Cyrillic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 839 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Dash as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 844 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Default_Ignorable_Code_Point as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 870 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Deprecated as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 874 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Deprecated as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 885 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Deseret as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 893 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Devanagari as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 898 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Devanagari as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 909 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Default_Ignorable_Code_Point as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 912 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Diacritic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 916 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Diacritic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 926 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Dives_Akuru as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 931 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Dives_Akuru as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 942 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Dogra as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 947 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Dogra as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 953 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Deseret as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 958 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Duployan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 963 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Duployan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 972 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Emoji_Modifier_Base as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 978 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Emoji_Component as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 984 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Egyptian_Hieroglyphs as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 989 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Egyptian_Hieroglyphs as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1009 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Elbasan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1014 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Elbasan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1022 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Elymaic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1027 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Elymaic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1035 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Emoji_Modifier as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1040 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Emoji as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1046 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Emoji_Component as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1061 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Emoji_Modifier as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1075 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Emoji_Modifier_Base as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1093 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Emoji_Presentation as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1111 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Emoji_Presentation as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1117 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Ethiopic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1122 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Ethiopic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1131 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Extender as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1135 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Extended_Pictographic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1156 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Extender as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1165 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Extended_Pictographic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1173 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Garay as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1178 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Garay as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1184 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Georgian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1189 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Georgian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1198 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Glagolitic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1203 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Glagolitic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1214 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Gunjala_Gondi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1219 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Masaram_Gondi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1224 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Gothic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1229 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Gothic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1236 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Grantha as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1241 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Grantha as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1249 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Grapheme_Base as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1262 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Grapheme_Extend as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1277 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Grapheme_Link as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1290 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Grapheme_Base as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1297 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Greek as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1303 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Greek as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1308 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Grapheme_Extend as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1314 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Grapheme_Link as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1321 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Gujarati as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1330 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Gujarati as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1335 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Gurung_Khema as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1340 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Gunjala_Gondi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1353 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Gurmukhi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1362 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Gurmukhi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1367 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Gurung_Khema as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1379 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Han as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1383 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Hangul as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1388 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Hangul as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1395 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Han as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1400 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Hanifi_Rohingya as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1415 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Hanunoo as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1420 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Hanunoo as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1428 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Hatran as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1433 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Hatran as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1440 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Hebrew as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1445 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Hebrew as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1452 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Hex_Digit as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1456 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Hex_Digit as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1465 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Hiragana as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1470 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Hiragana as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1479 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Anatolian_Hieroglyphs as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1484 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Pahawh_Hmong as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1489 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Nyiakeng_Puachue_Hmong as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1494 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Old_Hungarian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1499 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_ID_Continue as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1503 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_ID_Compat_Math_Continue as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1524 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_ID_Compat_Math_Start as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1542 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_ID_Continue as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1553 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Ideographic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1558 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Ideographic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1570 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_ID_Start as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1574 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_IDS_Binary_Operator as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1579 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_IDS_Binary_Operator as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1597 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_IDS_Trinary_Operator as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1602 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_ID_Start as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1610 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_IDS_Trinary_Operator as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1629 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_IDS_Unary_Operator as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1634 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_IDS_Unary_Operator as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1651 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Imperial_Aramaic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1667 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_InCB as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1672 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Inherited as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1682 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Inscriptional_Pahlavi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1703 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Inscriptional_Parthian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1725 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Old_Italic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1730 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Javanese as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1735 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Javanese as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1744 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Join_Control as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1750 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Join_Control as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1762 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Kaithi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1769 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Kayah_Li as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1774 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Katakana as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1779 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Kannada as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1787 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Katakana as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1796 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Kawi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1801 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Kayah_Li as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1809 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Kharoshthi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1814 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Kharoshthi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1825 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Khitan_Small_Script as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1843 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Khmer as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1849 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Khmer as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1854 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Khojki as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1859 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Khojki as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1866 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Khudawadi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1876 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Kirat_Rai as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1885 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Khitan_Small_Script as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1890 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Kannada as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1895 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Kirat_Rai as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1900 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Kaithi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1905 as uint16_t,
        type_0: PT_GC as uint16_t,
        value: ucp_L as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1907 as uint16_t,
        type_0: PT_LAMP as uint16_t,
        value: 0 as uint16_t,
    },
    ucp_type_table {
        name_offset: 1910 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Tai_Tham as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1915 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Lao as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1919 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Lao as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1924 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Latin as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1930 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Latin as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1935 as uint16_t,
        type_0: PT_LAMP as uint16_t,
        value: 0 as uint16_t,
    },
    ucp_type_table {
        name_offset: 1938 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Lepcha as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1943 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Lepcha as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1950 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Limbu as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1955 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Limbu as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1961 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Linear_A as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1966 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Linear_B as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1971 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Linear_A as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1979 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Linear_B as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1987 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Lisu as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1992 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Ll as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1995 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Lm as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 1998 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Lo as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2001 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Logical_Order_Exception as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2005 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Logical_Order_Exception as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2027 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Lowercase as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2033 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Lowercase as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2043 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Lt as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2046 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Lu as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2049 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Lycian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2054 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Lycian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2061 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Lydian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2066 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Lydian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2073 as uint16_t,
        type_0: PT_GC as uint16_t,
        value: ucp_M as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2075 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Mahajani as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2084 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Mahajani as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2089 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Makasar as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2094 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Makasar as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2102 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Malayalam as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2112 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Mandaic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2117 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Mandaic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2125 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Manichaean as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2130 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Manichaean as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2141 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Marchen as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2146 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Marchen as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2154 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Masaram_Gondi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2167 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Math as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2172 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Mc as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2175 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Modifier_Combining_Mark as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2179 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Me as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2182 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Medefaidrin as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2194 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Medefaidrin as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2199 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Meetei_Mayek as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2211 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Mende_Kikakui as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2216 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Mende_Kikakui as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2229 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Meroitic_Cursive as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2234 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Meroitic_Hieroglyphs as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2239 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Meroitic_Cursive as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2255 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Meroitic_Hieroglyphs as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2275 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Miao as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2280 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Malayalam as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2285 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Mn as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2288 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Modi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2293 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Modifier_Combining_Mark as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2315 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Mongolian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2320 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Mongolian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2330 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Mro as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2334 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Mro as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2339 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Meetei_Mayek as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2344 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Multani as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2349 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Multani as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2357 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Myanmar as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2365 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Myanmar as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2370 as uint16_t,
        type_0: PT_GC as uint16_t,
        value: ucp_N as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2372 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Nabataean as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2382 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Nag_Mundari as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2387 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Nag_Mundari as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2398 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Nandinagari as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2403 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Nandinagari as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2415 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Old_North_Arabian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2420 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Nabataean as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2425 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Noncharacter_Code_Point as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2431 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Nd as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2434 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Newa as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2439 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_New_Tai_Lue as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2449 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Nko as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2453 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Nko as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2458 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Nl as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2461 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_No as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2464 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Noncharacter_Code_Point as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2486 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Nushu as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2491 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Nushu as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2497 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Nyiakeng_Puachue_Hmong as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2518 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Ogham as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2523 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Ogham as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2529 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Ol_Chiki as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2537 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Ol_Chiki as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2542 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Old_Hungarian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2555 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Old_Italic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2565 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Old_North_Arabian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2581 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Old_Permic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2591 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Old_Persian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2602 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Old_Sogdian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2613 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Old_South_Arabian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2629 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Old_Turkic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2639 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Old_Uyghur as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2649 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Ol_Onal as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2656 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Ol_Onal as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2661 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Oriya as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2667 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Old_Turkic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2672 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Oriya as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2677 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Osage as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2683 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Osage as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2688 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Osmanya as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2693 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Osmanya as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2701 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Old_Uyghur as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2706 as uint16_t,
        type_0: PT_GC as uint16_t,
        value: ucp_P as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2708 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Pahawh_Hmong as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2720 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Palmyrene as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2725 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Palmyrene as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2735 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Pattern_Syntax as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2742 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Pattern_Syntax as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2756 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Pattern_White_Space as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2774 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Pattern_White_Space as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2780 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Pau_Cin_Hau as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2785 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Pau_Cin_Hau as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2795 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Pc as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2798 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Prepended_Concatenation_Mark as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2802 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Pd as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2805 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Pe as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2808 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Old_Permic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2813 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Pf as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2816 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Phags_Pa as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2821 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Phags_Pa as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2829 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Inscriptional_Pahlavi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2834 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Psalter_Pahlavi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2839 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Phoenician as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2844 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Phoenician as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2855 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Pi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2858 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Miao as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2863 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Po as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2866 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Prepended_Concatenation_Mark as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2893 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Inscriptional_Parthian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2898 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Ps as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2901 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Psalter_Pahlavi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2916 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Coptic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2921 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Inherited as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2926 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Quotation_Mark as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2932 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Quotation_Mark as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2946 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Radical as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2954 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Regional_Indicator as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2972 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Rejang as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2979 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Regional_Indicator as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2982 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Rejang as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2987 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Hanifi_Rohingya as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2992 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Runic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 2998 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Runic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3003 as uint16_t,
        type_0: PT_GC as uint16_t,
        value: ucp_S as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3005 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Samaritan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3015 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Samaritan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3020 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Old_South_Arabian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3025 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Saurashtra as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3030 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Saurashtra as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3041 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Sc as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3044 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Soft_Dotted as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3047 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Sentence_Terminal as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3064 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_SignWriting as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3069 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Sharada as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3077 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Shavian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3085 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Shavian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3090 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Sharada as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3095 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Siddham as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3100 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Siddham as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3108 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Sidetic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3116 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Sidetic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3121 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_SignWriting as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3133 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Khudawadi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3138 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Sinhala as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3143 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Sinhala as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3151 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Sk as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3154 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Sm as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3157 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_So as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3160 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Soft_Dotted as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3171 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Sogdian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3176 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Sogdian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3184 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Old_Sogdian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3189 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Sora_Sompeng as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3194 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Sora_Sompeng as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3206 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Soyombo as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3211 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Soyombo as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3219 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_White_Space as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3225 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Sentence_Terminal as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3231 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Sundanese as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3236 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Sundanese as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3246 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Sunuwar as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3251 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Sunuwar as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3259 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Syloti_Nagri as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3264 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Syloti_Nagri as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3276 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Syriac as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3281 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Syriac as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3288 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tagalog as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3296 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tagbanwa as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3301 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tagbanwa as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3310 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tai_Le as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3316 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Tai_Tham as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3324 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Tai_Viet as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3332 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Tai_Yo as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3338 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Takri as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3343 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Takri as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3349 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tai_Le as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3354 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_New_Tai_Lue as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3359 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tamil as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3365 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tamil as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3370 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tangut as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3375 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Tangsa as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3382 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tangut as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3389 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Tai_Viet as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3394 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Tai_Yo as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3399 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Telugu as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3404 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Telugu as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3411 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Terminal_Punctuation as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3416 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Terminal_Punctuation as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3436 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tifinagh as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3441 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tagalog as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3446 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Thaana as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3451 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Thaana as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3458 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Thai as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3463 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tibetan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3471 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tibetan as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3476 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tifinagh as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3485 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tirhuta as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3490 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tirhuta as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3498 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Tangsa as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3503 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Todhri as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3510 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Todhri as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3515 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Tolong_Siki as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3526 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Tolong_Siki as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3531 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Toto as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3536 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tulu_Tigalari as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3549 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Tulu_Tigalari as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3554 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Ugaritic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3559 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Ugaritic as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3568 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Unified_Ideograph as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3574 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Unified_Ideograph as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3591 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Unknown as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3599 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Uppercase as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3605 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Uppercase as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3615 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Vai as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3619 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Vai as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3624 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Variation_Selector as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3642 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Vithkuqi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3647 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Vithkuqi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3656 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_Variation_Selector as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3659 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Wancho as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3666 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Warang_Citi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3671 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Warang_Citi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3682 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Wancho as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3687 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_White_Space as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3698 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_White_Space as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3705 as uint16_t,
        type_0: PT_ALNUM as uint16_t,
        value: 0 as uint16_t,
    },
    ucp_type_table {
        name_offset: 3709 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_XID_Continue as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3714 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_XID_Continue as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3726 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_XID_Start as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3731 as uint16_t,
        type_0: PT_BOOL as uint16_t,
        value: ucp_XID_Start as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3740 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Old_Persian as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3745 as uint16_t,
        type_0: PT_PXSPACE as uint16_t,
        value: 0 as uint16_t,
    },
    ucp_type_table {
        name_offset: 3749 as uint16_t,
        type_0: PT_SPACE as uint16_t,
        value: 0 as uint16_t,
    },
    ucp_type_table {
        name_offset: 3753 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Cuneiform as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3758 as uint16_t,
        type_0: PT_UCNC as uint16_t,
        value: 0 as uint16_t,
    },
    ucp_type_table {
        name_offset: 3762 as uint16_t,
        type_0: PT_WORD as uint16_t,
        value: 0 as uint16_t,
    },
    ucp_type_table {
        name_offset: 3766 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Yezidi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3771 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Yezidi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3778 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Yi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3781 as uint16_t,
        type_0: PT_SCX as uint16_t,
        value: ucp_Yi as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3786 as uint16_t,
        type_0: PT_GC as uint16_t,
        value: ucp_Z as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3788 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Zanabazar_Square as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3804 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Zanabazar_Square as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3809 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Inherited as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3814 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Zl as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3817 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Zp as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3820 as uint16_t,
        type_0: PT_PC as uint16_t,
        value: ucp_Zs as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3823 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Common as ::core::ffi::c_int as uint16_t,
    },
    ucp_type_table {
        name_offset: 3828 as uint16_t,
        type_0: PT_SC as uint16_t,
        value: ucp_Unknown as ::core::ffi::c_int as uint16_t,
    },
];
#[no_mangle]
pub static mut _pcre2_utt_size_8: size_t = 0;
unsafe extern "C" fn run_static_initializers() {
    _pcre2_utf8_table1_size = (::core::mem::size_of::<[::core::ffi::c_int; 6]>() as usize)
        .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as usize)
        as ::core::ffi::c_uint;
    _pcre2_OP_lengths_8 = [
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        2 as ::core::ffi::c_int as uint8_t,
        (2 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE) as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE) as uint8_t,
        (1 as usize).wrapping_add(
            (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize),
        ) as uint8_t,
        (1 as usize).wrapping_add(
            (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize),
        ) as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + IMM2_SIZE + 1 as ::core::ffi::c_int) as uint8_t,
        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE + 1 as ::core::ffi::c_int)
            as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * LINK_SIZE + 1 as ::core::ffi::c_int)
            as uint8_t,
        0 as ::core::ffi::c_int as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE + IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE + IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE + IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE + IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + LINK_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE) as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        3 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        (1 as ::core::ffi::c_int + IMM2_SIZE) as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
        1 as ::core::ffi::c_int as uint8_t,
    ];
    _pcre2_utt_size_8 = (::core::mem::size_of::<[ucp_type_table; 518]>() as size_t)
        .wrapping_div(::core::mem::size_of::<ucp_type_table>() as size_t);
}
#[used]
#[cfg_attr(any(target_os = "linux", target_os = "android"), link_section = ".init_array")]
#[cfg_attr(target_os = "windows", link_section = ".CRT$XCU")]
#[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
static INIT_ARRAY: [unsafe extern "C" fn(); 1] = [run_static_initializers];
