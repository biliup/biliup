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

#ifndef __ASS_STRING_PROCESSING_H__
#define __ASS_STRING_PROCESSING_H__

#ifdef __cplusplus
extern "C"
{
#endif

#include "../CDanmakuFactory.h"

// TODO: sc框文本宽度补偿值 有准确测长方法请移除
#define SCBOX_TXT_LEN_COMPENSATION 1.2

#define ASS_ESCAPE 0
#define ASS_UNESCAPE 1

    struct AssEscapeListNode
    {
        char *originalText;
        char *assEscapedText;
    };

    extern char *toHexOpacity(int decOpacity, char *hexOpacity);
    extern char *toHexColor(int decColor, char *hexColor);
    extern int toDecColor(char *hexColor);
    extern char *deStyleNamePrefix(char *str);
    extern int timeToInt(const char *ipStr);
    extern void arrset(int *array, const int value, const int numberOfMember);
    char *assEscape(char *dstStr, char *srcStr, int dstStrLen, int mode);

#ifdef __cplusplus
}
#endif

#endif