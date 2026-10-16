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

#include "AssStringProcessing.h"

const struct AssEscapeListNode assEscapeList[] = {/* 源字符串 转义后字符串 */
                                                  // {"\\n", "\\{}n"},
                                                  // {"\\N", "\\{}N"},
                                                  // {"\\h", "\\{}h"},
                                                  {"\\n", "\\\xe2\x80\x8bn"},
                                                  {"\\N", "\\\xe2\x80\x8bN"},
                                                  {"\\h", "\\​h"},
                                                  {"{", "\\{"}, // compatible with libass
                                                  {"}", "\\}"}, // compatible with libass
                                                  {" ", "\\h"},
                                                  {"\n", "\\N"},
                                                  {"&lt;", "<"},
                                                  {"&gt;", ">"},
                                                  {"&amp;", "&"},
                                                  {"&apos;", "\'"},
                                                  {"&quot;", "\""},
                                                  {"&#", ""}, // emoji Unicode 码位占位符
                                                  {NULL, NULL}};

/*
 * 转换十进制rgb颜色为十六进制颜色
 * 参数：
 * 十进制颜色/记录十六进制的数组（7个字节以上）
 * 返回值：
 * 十六进制数组的地址
 */
char *toHexColor(int decColor, char *hexColor)
{ /* 先转换为16进制再两个为一对倒序排列 */
    int i, j;
    /* 使用两个for实现位置交叉 */
    for (i = 0; i < 3; i++)
    {
        for (j = 1; j >= 0; j--)
        {
            if (decColor % 16 < 10)
            {
                hexColor[2 * i + j] = decColor % 16 + '0';
            }
            else
            {
                hexColor[2 * i + j] = decColor % 16 - 10 + 'A';
            }
            decColor /= 16;
        }
    }
    hexColor[6] = '\0';
    return hexColor;
}

/*
 * 十进制透明度转十六进制透明度（0-255）
 */
char *toHexOpacity(int decOpacity, char *hexOpacity)
{
    int cnt;
    for (cnt = 1; cnt >= 0; cnt--)
    {
        if (decOpacity % 16 < 10)
        {
            hexOpacity[cnt] = decOpacity % 16 + '0';
        }
        else
        {
            hexOpacity[cnt] = decOpacity % 16 - 10 + 'A';
        }
        decOpacity /= 16;
    }
    hexOpacity[2] = '\0';
    return hexOpacity;
}

/*
 * 转换十六进制rgb颜色为十进制颜色
 */
int toDecColor(char *hexColor)
{
    int i, j;
    int color = 0;

    for (i = 2; i >= 0; i--)
    {
        for (j = 0; j < 2; j++)
        {
            if (hexColor[2 * i + j] >= '0' && hexColor[2 * i + j] <= '9')
            {
                color *= 16;
                color += hexColor[2 * i + j] - '0';
            }
            else if (hexColor[2 * i + j] >= 'a' && hexColor[2 * i + j] <= 'f')
            {
                color *= 16;
                color += hexColor[2 * i + j] - 'a' + 10;
            }
            else if (hexColor[2 * i + j] >= 'A' && hexColor[2 * i + j] <= 'F')
            {
                color *= 16;
                color += hexColor[2 * i + j] - 'A' + 10;
            }
        }
    }

    return color;
}

/*  去样式名前缀
 *
 *   0         1         2
 *   012345678901234567890123456789
 *   danmakuFactory_ext_sub00_name     ->     name
 */
char *deStyleNamePrefix(char *str)
{
    /* 合法性检查 避免偏移导致的非法访问 */
    if (strlen(str) < 22)
    {
        return NULL;
    }

    char *leftPtr, *rightPtr;
    leftPtr = str;
    rightPtr = str + 21; /* 从22个字节开始寻找字符 '_' 下划线 */

    while (*rightPtr != '\0' && *(rightPtr - 1) != '_')
    { /* 寻找下划线 */
        rightPtr++;
    }

    while (*(rightPtr - 1) != '\0')
    { /* 内容拷贝 */
        *leftPtr = *rightPtr;
        leftPtr++;
        rightPtr++;
    }

    return str;
}

/*
  以ass标准读取时间
  输出单位：毫秒
 */
int timeToInt(const char *ipStr)
{
    /* 0:00:01.60 */
    int num = 0;
    int time = 0;
    char *ptr = (char *)ipStr;

    /* 小时部分 */
    while (*ptr != ':' && *ptr != '.' && *ptr != '\0')
    {
        if (*ptr == ' ')
        {
            ptr++;
            continue;
        }
        num *= 10;
        num += *ptr - '0';
        ptr++;
    }
    time = num * 3600;
    num = 0;
    ptr++;

    /* 分钟部分 */
    while (*ptr != ':' && *ptr != '.' && *ptr != '\0')
    {
        if (*ptr == ' ')
        {
            ptr++;
            continue;
        }
        num *= 10;
        num += *ptr - '0';
        ptr++;
    }
    time += num * 60;
    num = 0;
    ptr++;

    /* 秒部分 */
    while (*ptr != '.' && *ptr != ':' && *ptr != '\0')
    {
        if (*ptr == ' ')
        {
            ptr++;
            continue;
        }
        num *= 10;
        num += *ptr - '0';
        ptr++;
    }
    time += num;
    num = 0;
    ptr++;

    /* 毫秒部分 */
    while (*ptr != '.' && *ptr != ':' && *ptr != '\0')
    {
        if (*ptr == ' ')
        {
            ptr++;
            continue;
        }
        if (++num > 3)
        { // 最多 3 位有效数字
            if ((*ptr - '0') >= 5)
            {
                time++; // 四舍五入
            }
            break;
        }
        time *= 10;
        time += *ptr - '0';
        ptr++;
    }
    for (int i = 0; i < 3 - num; i++)
    {
        time *= 10;
    }

    return time;
}

/*
 * 数组清空
 * 参数：
 * 数组首地址/要初始化成的值/成员数量/
 * 返回值：
 * ferror函数的返回值
 */
void arrset(int *array, const int value, const int numberOfMember)
{
    int cnt;
    for (cnt = 0; cnt < numberOfMember; cnt++)
    {
        *(array + cnt) = value;
    }
    return;
}

/*
 * ASS 转义
 * 参数：
 * 目标字符串/源字符串/工作模式（ASS_ESCAPE/ASS_UNESCAPE）
 * 返回值：
 * 目标字符串
 */

char *assEscape(char *dstStr, char *srcStr, int dstStrLen, int mode)
{
    if (srcStr == NULL || dstStr == NULL)
    {
        return NULL;
    }

    int lenCnt = 0;
    char *srcPtr, *dstPtr;
    char *originalTextPtr, *assEscapedTextPtr;
    char utf8Hex[5];
    srcPtr = srcStr;
    dstPtr = dstStr;

    while (*srcPtr != '\0' && lenCnt < dstStrLen - 1)
    {
        int listCnt = 0;
        while (lenCnt < dstStrLen - 1)
        {
            if (mode == ASS_ESCAPE)
            {
                originalTextPtr = assEscapeList[listCnt].originalText;
                assEscapedTextPtr = assEscapeList[listCnt].assEscapedText;
            }
            else
            {
                originalTextPtr = assEscapeList[listCnt].assEscapedText;
                assEscapedTextPtr = assEscapeList[listCnt].originalText;
            }

            if (originalTextPtr == NULL || assEscapedTextPtr == NULL || *originalTextPtr == '\0')
            { /* 匹配全部失败，直接拷贝文本 */
                *dstPtr = *srcPtr;
                dstPtr++;
                lenCnt++;
                break;
            }

            char *srcCmpPtr = srcPtr;
            char *originalTextCmpPtr = originalTextPtr;
            while (*originalTextCmpPtr != '\0' && *srcCmpPtr != '\0')
            {
                if (*originalTextCmpPtr != *srcCmpPtr)
                {
                    break;
                }
                originalTextCmpPtr++;
                srcCmpPtr++;
            }

            if (*originalTextCmpPtr == '\0')
            {
                if (strcmp(originalTextPtr, "&#") == 0)
                {
                    char emojiUnicodeText[8];
                    strGetLeftPart(emojiUnicodeText, &srcCmpPtr, ';', 8);
                    unicode_to_utf8(atoi(emojiUnicodeText), utf8Hex);
                    assEscapedTextPtr = utf8Hex;
                    // "128514" + ";". A trailing "&#" has nothing after it to skip:
                    // stepping over its terminator would read past the string.
                    srcPtr += strlen(emojiUnicodeText) + (srcPtr[2] != '\0');
                }

                /* 拷贝转义后字串 */
                while (*assEscapedTextPtr != '\0')
                {
                    if (lenCnt >= dstStrLen - 1)
                    { /* 目标容器超长 */
                        break;
                    }

                    *dstPtr = *assEscapedTextPtr;
                    dstPtr++;
                    assEscapedTextPtr++;
                    lenCnt++;
                }

                srcPtr += strlen(originalTextPtr) - 1; // "- 1" is for "srcPtr++" later.
                break;
            }

            listCnt++;
        }

        srcPtr++;
    }

    *dstPtr = '\0';

    return dstStr;
}