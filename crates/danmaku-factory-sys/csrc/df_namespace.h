/*
 * Force-included into every vendored DanmakuFactory file and into df_ffi.c.
 * The core exports generic names such as match(), trim() and toLower(); the
 * prefix keeps them from clashing with other C code linked into biliup.
 * Generated from the core's headers and `nm` of its objects; update it when
 * the vendored sources change (a missing name only means an unprefixed
 * symbol, which the symbol check in tests/c/check.sh reports).
 */
#ifndef BILIUP_DF_NAMESPACE_H
#define BILIUP_DF_NAMESPACE_H

#define arrset biliup_df_arrset
#define assEscape biliup_df_assEscape
#define assEscapeList biliup_df_assEscapeList
#define assFileToDanmaku biliup_df_assFileToDanmaku
#define blockByType biliup_df_blockByType
#define boolToStr biliup_df_boolToStr
#define deQuotMarks biliup_df_deQuotMarks
#define deStyleNamePrefix biliup_df_deStyleNamePrefix
#define filenameGetFormat biliup_df_filenameGetFormat
#define filenameGetPath biliup_df_filenameGetPath
#define findSubstr biliup_df_findSubstr
#define floatToStr biliup_df_floatToStr
#define freeAssFile biliup_df_freeAssFile
#define FreeCommandLineUTF8 biliup_df_FreeCommandLineUTF8
#define freeList biliup_df_freeList
#define GetCommandLineUTF8 biliup_df_GetCommandLineUTF8
#define getNextWord biliup_df_getNextWord
#define getStrHei biliup_df_getStrHei
#define getStrLen biliup_df_getStrLen
#define intTimeToStr biliup_df_intTimeToStr
#define isDesignatedChar biliup_df_isDesignatedChar
#define isDesignatedStr biliup_df_isDesignatedStr
#define isStartWith biliup_df_isStartWith
#define isUtf8 biliup_df_isUtf8
#define lastChr biliup_df_lastChr
#define match biliup_df_match
#define normFontSize biliup_df_normFontSize
#define printConfig biliup_df_printConfig
#define readAss biliup_df_readAss
#define readAssFile biliup_df_readAssFile
#define readConfig biliup_df_readConfig
#define readJson biliup_df_readJson
#define readTemplateFile biliup_df_readTemplateFile
#define readXml biliup_df_readXml
#define sortList biliup_df_sortList
#define strGetLeftPart biliup_df_strGetLeftPart
#define strrpl biliup_df_strrpl
#define strSafeCat biliup_df_strSafeCat
#define strSafeCopy biliup_df_strSafeCopy
#define strToBool biliup_df_strToBool
#define timeToInt biliup_df_timeToInt
#define toDecColor biliup_df_toDecColor
#define toHexColor biliup_df_toHexColor
#define toHexOpacity biliup_df_toHexOpacity
#define toLower biliup_df_toLower
#define trim biliup_df_trim
#define unicode_to_utf8 biliup_df_unicode_to_utf8
#define utf8_access biliup_df_utf8_access
#define utf8_fopen biliup_df_utf8_fopen
#define writeAss biliup_df_writeAss
#define writeAssDanmakuPart biliup_df_writeAssDanmakuPart
#define writeAssStatPart biliup_df_writeAssStatPart
#define writeAssStylesPart biliup_df_writeAssStylesPart
#define writeConfig biliup_df_writeConfig
#define writeJson biliup_df_writeJson
#define writeXml biliup_df_writeXml

#endif
