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

#ifndef __TEMPLATE_FILE_H__
#define __TEMPLATE_FILE_H__

#ifdef __cplusplus
extern "C"
{
#endif

#include "../Define/DanmakuDef.h"
#include "../Define/Status.h"
#include "../String/DanmakuFactoryString.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define LINE_MAX_LEN 4096
#define FORMAT_TABLE_MAX_LEN 256
#define STR_TABLE_MAX_LEN 128
#define READ_BUFFER_SIZE LINE_MAX_LEN * 5

#define TAR_NULL 0
#define TAR_FORMAT 1
#define TAR_SET_TIME 2
#define TAR_SET_TYPE 3
#define TAR_SET_COLOR 4
#define TAR_SET_FONTSIZE 5

#define FORMAT_TIME -1
#define FORMAT_TYPE -2
#define FORMAT_COLOR -3
#define FORMAT_FONTSIZE -4
#define FORMAT_TEXT -5
#define FORMAT_ANYNUM -6
#define FORMAT_ANYSTR -7

#define HAVE_NOT_BEEN_SET -1

#define FORMAT_IS_VAR(tableValue) (tableValue < 0)

    struct formatOfDanmaku
    {
        char *strTable[STR_TABLE_MAX_LEN];     /* 字串表 */
        int formatTable[FORMAT_TABLE_MAX_LEN]; /* 格式表 */
        int strTableLen;
        int formatTableLen;
        /*
         * 格式表表示方法： 按顺序用整形存储每一个部分，宏定义 FORMAT_XXX
         * 表示读取某类型值，正整数表示字串表下标，对于某一字符串
         */
    };

    struct timeDefineSetting
    {
        float ratioToSeconds;
    };

    struct typeDefineSetting
    {
        int rightToLeft;
        int leftToRight;
        int topFix;
        int bottomFix;
        int defaultType;
    };

    struct colorDefineSetting
    {
        int defaultColor;
    };

    struct fontsizeDefineSetting
    {
        int defaultFontsize;
    };

    typedef struct formatOfDanmaku FORMAT;
    typedef struct timeDefineSetting SET_TIME;
    typedef struct typeDefineSetting SET_TYPE;
    typedef struct colorDefineSetting SET_COLOR;
    typedef struct fontsizeDefineSetting SET_FONTSIZE;

    extern int readTemplateFile(const char *const ipFile, const char *const templateFile, DANMAKU **danmakuHead,
                                const char *mode, const float timeShift, STATUS *const status, char *errMsg,
                                int msgLen);

#ifdef __cplusplus
}
#endif

#endif
