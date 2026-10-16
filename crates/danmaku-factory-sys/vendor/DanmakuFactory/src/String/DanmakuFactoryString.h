/* MIT License
 *
 * Copyright (c) 2022 hkm
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

#ifndef __DANMAKUFACTORY_STRING_H__
#define __DANMAKUFACTORY_STRING_H__

#ifdef __cplusplus
extern "C"
{
#endif

#include "../Define/DanmakuDef.h"
#include <stddef.h>

#define CHAR_IS_BLANK(ch) (ch == ' ' || ch == '\n' || ch == '\t' || ch == '\r')
#define STR_IS_EMPTY(str) (str[0] == '\0')
#define STR_IS_BLANK(str) (isDesignatedStr(str, " \n\t\r"))
#define TO_STR_END(ptr) (ptr += strlen(ptr))
#define FLOAT_IS_EQUAL(floatNum1, floatNum2) (fabs(floatNum1 - floatNum2) < EPS)

/*移动字符指针向后跳到距离为字符串prefix的长度的位置*/
#define STR_PTR_JUMP(strPtr, prefix)                                                                                   \
    {                                                                                                                  \
        strPtr += strlen(prefix) * sizeof(char);                                                                       \
    }

    extern int getStrHei(const unsigned char *str, const int fontSizeSet, const int fontSizeInFile,
                         const char *fontName);
    extern int getStrLen(const unsigned char *str, const int fontSizeSet, const int fontSizeInFile,
                         const char *fontName);
    extern char *intTimeToStr(char *const outputStr, const int input, const int accuracy);
    extern char *floatToStr(char *const outputStr, const double input, const int accuracy);
    extern char *strGetLeftPart(char *const opStr, char **const ipStr, const char cutCh, const int maxLen);
    extern char *trim(char *const str);
    extern char *deQuotMarks(char *const str);
    extern char *toLower(char *output, char *input);
    extern int lastChr(const char *const str, const char chr);
    extern BOOL isDesignatedChar(char ch, const char *designatedChar);
    extern BOOL isDesignatedStr(char *str, char *designatedChar);
    extern BOOL isUtf8(char const *const str);
    extern char *strSafeCopy(char *destination, const char *const source, size_t count);
    extern int strSafeCat(char *dstBuf, int dstSize, const char *srcBuf);
    extern int match(char *mainStr, char *patternStr);
    extern char *strrpl(char *inStr, char *outStr, char *srcStr, char *dstStr, int outBuffSize);
    extern BOOL isStartWith(const char *const mainStr, const char *const prefixStr);
    extern char *getNextWord(char **ptr, char *buf, int maxLen, char endBefore, BOOL isMovePtr);
    extern char *filenameGetFormat(char *format, const char *const fileName, int maxLen);
    extern char *filenameGetPath(char *path, const char *const fileName, int maxLen);
    extern BOOL strToBool(const char *const str);
    extern char *boolToStr(char *opStr, BOOL boolValue);
    extern void unicode_to_utf8(unsigned int codepoint, char *utf8_buffer);

#ifdef __cplusplus
}
#endif

#endif