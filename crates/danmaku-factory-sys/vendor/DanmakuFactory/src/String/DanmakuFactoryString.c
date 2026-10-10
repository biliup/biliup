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

#include "DanmakuFactoryString.h"
#include <stdio.h>
#include <string.h>

/*
 * 计算字符串高度
 * 参数：
 * 字符串指针/字号/字体
 * 返回值：
 * 高度（像素）
 */
int getStrHei(const unsigned char *str, const int fontSizeSet, const int fontSizeInFile, const char *fontName)
{
    /* 非法判断 */
    if (str == NULL)
    {
        return -1;
    }

    // 字号映射转换
    return fontSizeSet + (fontSizeInFile - 25);
}

/*
 * 计算字符串宽度
 * 参数：
 * 字符串指针/字号/字体
 * 返回值：
 * 宽度（像素）
 */
int getStrLen(const unsigned char *str, const int fontSizeSet, const int fontSizeInFile, const char *fontName)
{

    int cnt = 0;
    /* 非法判断 */
    if (str == NULL)
    {
        return -1;
    }

    /* 判断字符串编码 */
    if (isUtf8(str) == TRUE)
    {
        /*
         * 目前仅支持utf-8
         * 首字节           空间占用           起始
         * 0--- ----        1 byte            0x00
         * 110- ----        2 byte            0xC0
         * 1110 ----        3 byte            0xE0
         * 1111 0---        4 byte            0xF0
         * 1111 10--        5 byte            0xF8
         * 1111 110-        6 byte            0xFC
         */
        while (*str != '\0')
        {
            if (*str >= 0xC0)
            {
                cnt++;
            }
            else if (*str < 0x80)
            {
                cnt++;
            }
            str++;
        }
    }
    else
    {
        cnt = (int)strlen(str);
    }

    int len = cnt * (fontSizeSet + (fontSizeInFile - 25)) / 1.2;

    return len;
}

/*
 * 以指定精度的最小数位将整数转换为字符串
 * 参数：输出字符串(秒)/输入整数(毫秒)/精度
 */
char *intTimeToStr(char *const outputStr, const int input, const int accuracy)
{
    int seconds = input / 1000;
    int milliseconds = input - seconds * 1000;

    /* 整数 */
    if (milliseconds == 0)
    {
        sprintf(outputStr, "%d", seconds);
        return outputStr;
    }

    char tempStr[32], *tempPtr;
    sprintf(tempStr, "%d.%03d", seconds, milliseconds);
    tempPtr = strchr(tempStr, '.');

    /* 小数 */
    for (int cnt = 0; cnt <= accuracy; cnt++)
    {
        tempPtr++;
        if (*tempPtr == '\0')
        {
            break;
        }
    }

    /* 寻找末尾第一位非 0 位 */
    while (*(tempPtr - 1) == '0')
    {
        tempPtr--;
    }

    /* 去除小数点 */
    if (*(tempPtr - 1) == '.')
    {
        tempPtr--;
    }
    *tempPtr = '\0';

    strcpy(outputStr, tempStr);
    return outputStr;
}

/*
 * 以指定精度的最小数位将浮点数转换为字符串
 * 参数：输出字符数组/输入浮点数/精度
 */
char *floatToStr(char *const outputStr, const double input, const int accuracy)
{
    char tempStr[32], *tempPtr;
    sprintf(tempStr, "%lf", input);
    tempPtr = strchr(tempStr, '.');

    /* 整数 */
    if (tempPtr == NULL)
    {
        strcpy(outputStr, tempStr);
        return outputStr;
    }

    /* 小数 */
    for (int cnt = 0; cnt <= accuracy; cnt++)
    {
        tempPtr++;
        if (*tempPtr == '\0')
        {
            break;
        }
    }

    /* 寻找末尾第一位非 0 位 */
    while (*(tempPtr - 1) == '0')
    {
        tempPtr--;
    }

    /* 去除小数点 */
    if (*(tempPtr - 1) == '.')
    {
        tempPtr--;
    }
    *tempPtr = '\0';

    strcpy(outputStr, tempStr);
    return outputStr;
}

/*
 * 得到字符串左边部分，并移动指针至截断位置后一个字符
 * 参数：输出字符串/输入字符串指针地址/截断位置/
 * 返回值：结果字符串地址
 * 输出字符串为NULL时 将单纯移动指针
 */
char *strGetLeftPart(char *const opStr, char **const ipStr, const char cutCh, const int maxLen)
{
    int copiedLen = 0;
    char *leftPtr, *rightPtr;
    char *opPtr = (char *)opStr;
    leftPtr = rightPtr = (char *)*ipStr;

    /* 移动指针到指定字符位置 */
    while (*rightPtr != '\0' && *rightPtr != cutCh && copiedLen < maxLen - 1)
    {
        copiedLen++;
        rightPtr++;
    }

    /* 字符串中无指定字符 */
    if (*rightPtr == '\0')
    {
        leftPtr = rightPtr;
    }

    /* 执行拷贝 */
    if (opStr != NULL)
    {
        while (leftPtr < rightPtr)
        {
            *opPtr = *leftPtr;
            opPtr++;
            leftPtr++;
        }

        *opPtr = '\0';
    }

    /* 移动指针到下一个位置 */
    if (*rightPtr != '\0')
    {
        rightPtr++;
    }
    *ipStr = rightPtr;

    return opStr;
}

/*
 * 除去字符串左右空白字符
 * 参数：
 * 要去除空白字符的字符串/
 * 返回值：
 * 结果字符串/
 */
char *trim(char *const str)
{
    /* 非法检查 */
    if (str == NULL)
    {
        return NULL;
    }

    char *leftPtr, *rightPtr, *reWritePtr;
    leftPtr = str;
    /* 寻找左边的第一个非空字符 */
    while (*leftPtr == ' ' || *leftPtr == '\n' || *leftPtr == '\t' || *leftPtr == '\r')
    {
        if (*leftPtr == '\0')
        {
            break;
        }
        leftPtr++;
    }

    /* 寻找右边的最后一个非空字符 */
    rightPtr = leftPtr;
    while (*rightPtr != '\0')
    {
        rightPtr++;
    }

    while (*rightPtr == ' ' || *rightPtr == '\n' || *rightPtr == '\t' || *rightPtr == '\r' || *rightPtr == '\0')
    {
        if (rightPtr == leftPtr)
        {
            break;
        }
        rightPtr--;
    }

    /* 拷贝左右指针之间的内容 */
    reWritePtr = (char *)str;

    while (leftPtr <= rightPtr)
    {
        *reWritePtr = *leftPtr;
        reWritePtr++;
        leftPtr++;
    }

    *reWritePtr = '\0';

    return str;
}

/*
 * 去字符串一重左右引号(单双引号)，没有的话则不处理
 * 返回值：输入的指针
 */
char *deQuotMarks(char *const str)
{
    char *strPtr = (char *)str;

    /* 非法检查 */
    if (str == NULL)
    {
        return NULL;
    }

    trim(str); /* 去空白字符 */
    /* 去前引号 */
    if (*strPtr == '\"' || *strPtr == '\'')
    {
        while (*(strPtr + 1) != '\0')
        {
            *strPtr = *(strPtr + 1);
            strPtr++;
        }
    }
    else
    {
        while (*strPtr != '\0')
        {
            strPtr++;
        }
    }

    /* 去后引号 */
    if (strPtr != (char *)str && (*(strPtr - 1) == '\"' || *(strPtr - 1) == '\''))
    {
        strPtr--;
    }
    *strPtr = '\0';

    return str;
}

/*
 * 转换成小写字母
 * 参数：
 * 输出字符串地址/输入字符串地址/
 * 返回值：
 * 结果字符串地址
 * 若输出字符串地址为NULL，则将直接修改输入字符串
 */
char *toLower(char *output, char *input)
{
    char *ptr;
    if (output == NULL)
    {
        ptr = input;
    }
    else
    {
        strcpy(output, input);
        ptr = output;
    }

    while (*ptr != '\0')
    {
        if (*ptr >= 'A' && *ptr <= 'Z')
        {
            *ptr += 'a' - 'A';
        }
        ptr++;
    }

    if (output == NULL)
    {
        return input;
    }
    return output;
}

/*
 * 获取字符在字符串最后出现的位置
 * 参数：被查找的字符串/目标字符
 * 返回值：从0开始的索引
 * -2 字符串超长
 * -1 未找到
 */
int lastChr(const char *const str, const char chr)
{
    if (strlen(str) > MAX_TEXT_LENGTH)
    { /* 字符串长度合法性检查 */
        return -2;
    }

    char *ptr = (char *)(str + strlen(str));
    while (*ptr != chr && ptr >= str)
    {
        ptr--;
    }

    if (ptr < str)
    {
        return -1;
    }
    else
    {
        return ptr - str;
    }
}

/*
 * 判断字符是否是指定字符之一
 * 参数：输入字符/包含指定字符的字符串
 * 返回值：是否是指定字符之一
 */
BOOL isDesignatedChar(char ch, const char *designatedChar)
{
    char *ptr = (char *)designatedChar;
    while (*ptr != '\0')
    {
        if (*ptr == ch)
        {
            return TRUE;
        }
        ptr++;
    }

    return FALSE;
}

/*
 * 判断字符串中字符是否都是指定字符
 * 参数：输入字符串/包含指定字符的字符串
 * 返回值：是否是指定字符串
 */
BOOL isDesignatedStr(char *str, char *designatedChar)
{
    int cnt = 0;
    int i, j;
    for (i = 0; i < (int)strlen(str); i++)
    {
        for (j = 0; j < (int)strlen(designatedChar); j++)
        {
            if (str[i] == designatedChar[j])
            {
                cnt++;
            }
        }
    }

    if (cnt == (int)strlen(str))
    {
        return TRUE;
    }
    else
    {
        return FALSE;
    }
}

/*
 * 判断一个字符串是否是utf-8编码
 * 参数:要检测的字符串
 * 返回值:是否是utf-8编码
 */
BOOL isUtf8(char const *const str)
{
    /* 非法检查 */
    if (str == NULL)
    {
        return FALSE;
    }

    int i, len;
    const unsigned char *ptr = str;
    while ((*ptr) != '\0')
    {
        if (((*ptr) & 0x80) == 0x00)
        { // 0000 0000 - 0000 007F    0xxxxxxx
            ptr++;
            continue;
        }
        else if (((*ptr) & 0xE0) == 0xC0)
        { // 0000 0080 - 0000 07FF    110xxxxx 10xxxxxx
            len = 1;
        }
        else if (((*ptr) & 0xF0) == 0xE0)
        { // 0000 0800 - 0000 FFFF    1110xxxx 10xxxxxx 10xxxxxx
            len = 2;
        }
        else if (((*ptr) & 0xF8) == 0xF0)
        { // 0001 0000 - 0010 FFFF    11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
            len = 3;
        }
        else if (((*ptr) & 0xFC) == 0xF8)
        { // 0020 0000 - 03FF FFFF    111110xx 10xxxxxx 10xxxxxx 10xxxxxx 10xxxxxx
            len = 4;
        }
        else if (((*ptr) & 0xFE) == 0xFC)
        { // 0400 0000 - 7FFF FFFF    1111110x 10xxxxxx 10xxxxxx 10xxxxxx 10xxxxxx 10xxxxxx
            len = 5;
        }
        else
        {
            return FALSE;
        }

        for (i = 0; i < len; i++)
        {
            ptr++;
            if (*ptr == '\0' || ((*ptr) & 0xC0) != 0x80)
            {
                return FALSE;
            }
        }
        ptr++;
    }

    return TRUE;
}

/*
 * 安全的字符串复制
 * 参数：目的字符串/源字符串/最大长度
 * 返回值：目的字符串指针
 */
char *strSafeCopy(char *destination, const char *const source, size_t count)
{
    char *returnPtr;
    returnPtr = strncpy(destination, source, count);
    destination[count - 1] = '\0';
    return returnPtr;
}

/*
 * 安全的字符串连接
 * 参数：目的字符串/目的字符串最大长度/源字符串
 * 返回值：目的字符串最终长度
 */
int strSafeCat(char *dstBuf, int dstSize, const char *srcBuf)
{
    /* 合法性检查 */
    if (dstBuf == NULL || srcBuf == NULL || dstSize <= 0)
    {
        return 0;
    }

    int length = 0;
    char *dstPtr;
    const char *srcPtr;
    dstPtr = dstBuf;
    srcPtr = srcBuf;

    /* 遍历目标容器 */
    while (*dstPtr != '\0')
    {
        length++;
        if (length >= dstSize)
        {
            *dstPtr = '\0';
            return length;
        }

        dstPtr++;
    }

    /* 连接字符串 */
    while (*srcPtr != '\0')
    {
        length++;
        if (length >= dstSize)
        {
            break;
        }

        *dstPtr = *srcPtr;

        dstPtr++;
        srcPtr++;
    }

    *dstPtr = '\0';
    return length;
}

/*
 * 获取匹配次数
 * 参数：主串/模式串
 * 返回值：模式串在主串中出现的次数/错误返回-1
 * 两次匹配不能有重复字符，如 主aaa 模式aa 返回匹配次数是1次
 * 因为在本项目中模式串长度是1或者2，故直接暴力匹配
 */
int match(char *mainStr, char *patternStr)
{
    /* 合法性检查 */
    if (mainStr == NULL || patternStr == NULL || patternStr[0] == '\0')
    {
        return -1;
    }

    char *mainPtr, *mainCmpPtr, *patternPtr;
    mainPtr = mainCmpPtr = mainStr;
    patternPtr = patternStr;

    int matchTimes = 0;
    while (*mainPtr != '\0')
    {
        patternPtr = patternStr;
        while (*patternPtr != '\0')
        {
            if (*mainCmpPtr != *patternPtr)
            {
                break;
            }
            mainCmpPtr++;
            patternPtr++;
        }

        if (*patternPtr == '\0')
        {
            matchTimes++;
            mainPtr = mainCmpPtr;
        }
        else
        {
            mainCmpPtr = ++mainPtr;
        }
    }

    return matchTimes;
}

/*
 * 字符串替换
 * 参数：输入串/输出串/待替换串/需替换成串
 * 返回值：输出串指针/错误返回NULL
 * 两次匹配不能有重复字符，如 主aaa 模式aa 返回匹配次数是1次
 * 因为在本项目中模式串长度是1或者2，故直接暴力匹配
 */
char *strrpl(char *inStr, char *outStr, char *srcStr, char *dstStr, int outBuffSize)
{
    /* 合法性检查 */
    if (inStr == NULL || outStr == NULL || srcStr == NULL || srcStr[0] == '\0')
    {
        return NULL;
    }

    int cnt = 0;
    char *inPtr, *inCmpPtr, *inCopyPtr, *outPtr, *srcPtr, *dstPtr;
    inPtr = inCmpPtr = inCopyPtr = inStr;
    outPtr = outStr;
    srcPtr = srcStr;

    int matchTimes = 0;
    while (*inPtr != '\0')
    {
        srcPtr = srcStr;
        inCopyPtr = inCmpPtr;
        while (*srcPtr != '\0')
        {
            if (*inCmpPtr != *srcPtr)
            {
                break;
            }
            inCmpPtr++;
            srcPtr++;
        }

        if (*srcPtr == '\0')
        {
            matchTimes++;
            inPtr = inCmpPtr;

            /* 复制替换内容 */
            dstPtr = dstStr;
            while (cnt < outBuffSize && *dstPtr != '\0')
            {
                *outPtr = *dstPtr;
                cnt++;
                outPtr++;
                dstPtr++;
            }
        }
        else
        {
            inCmpPtr = ++inPtr;
            /* 复制主串内容 */
            if (cnt < outBuffSize)
            {
                *outPtr = *inCopyPtr;
                cnt++;
                outPtr++;
            }
        }
    }

    *outPtr = '\0';
    return outStr;
}

/*
 * 字符串是否是以指定子字符串开头
 * 参数：待判断字符串/指定字符串
 * 返回值：布尔值
 */
BOOL isStartWith(const char *const mainStr, const char *const prefixStr)
{
    int index = 0;
    while (mainStr[index] != '\0' && prefixStr[index] != '\0')
    {
        if (mainStr[index] != prefixStr[index])
        {
            return FALSE;
        }
        index++;
    }

    if (prefixStr[index] == '\0')
    {
        return TRUE;
    }
    else
    {
        return FALSE;
    }
}

/*
 * 从下一个非空白字符开始读取直到下一个空白字符或指定字符结束
 * 参数：字符串指针/输出字符串/输出字符串最大长度/提前结束字符（不含，'\0'表示任意空白字符结束）\是否移动指针到结束位置的下一个字符
 * 返回值：输出字符串
 * 当输出字符串长度达到 maxLen-1 时结束并补'\0'
 * 即便是指定了结束字符，遇到空白字符仍然会结束
 * buf 为 NULL 时，仅移动指针
 */
char *getNextWord(char **ptr, char *buf, int maxLen, char endBefore, BOOL isMovePtr)
{
    int cnt = 1;
    char *ipPtr = *ptr, *opPtr = buf;

    /* 参数合法性检查 */
    if (*ptr == NULL || maxLen < 2)
    {
        return NULL;
    }

    /* 跳过前面的空白字符 */
    while (*ipPtr != '\0' && CHAR_IS_BLANK(*ipPtr))
    {
        ipPtr++;
    }

    /* 拷贝字符串 */
    while (*ipPtr != '\0')
    {
        if (opPtr != NULL)
        {
            *opPtr = *ipPtr;
            opPtr++;
        }

        ipPtr++;
        cnt++;

        /* 检查结束条件 */
        if (cnt == maxLen)
        { /* 字串超过容器最大长度 */
            break;
        }
        else if (*ipPtr == endBefore || CHAR_IS_BLANK(*ipPtr))
        { /* 检查是否为结束字符 */
            break;
        }
    }

    if (opPtr != NULL)
    {
        *opPtr = '\0';
        buf[maxLen - 1] = '\0';
    }

    /* 是否移动指针 */
    if (isMovePtr == TRUE)
    {
        *ptr = ipPtr;
    }
    return buf;
}

/*
 * 从文件名获取文件格式
 * 参数：输出格式字符串/输入文件名字符串/输出字符串最大长度
 * 返回值：输出格式字符串
 */
char *filenameGetFormat(char *format, const char *const fileName, int maxLen)
{
    char *ptr;
    ptr = (char *)&fileName[strlen(fileName)];

    while (*ptr != '.')
    {
        if (ptr < fileName)
        {
            strcpy(format, "");
            return format;
        }
        ptr--;
    }
    ptr++;

    strSafeCopy(format, ptr, maxLen);

    return format;
}

/*
 * 获取文件路径部分（包含最后一个斜杠）
 * 参数：输出文件路径字符串/完整文件名字符串/输出字符串最大长度
 * 返回值：输出文件路径字符串
 */
char *filenameGetPath(char *path, const char *const fileName, int maxLen)
{
    int pathLen;
    char *inPtr = (char *)fileName;
    char *outPtr = path;

    /* 移动指针到末尾 */
    while (*inPtr != '\0')
    {
        inPtr++;
    }

    /* 向前寻找斜杠 */
    while (inPtr != fileName)
    {
        if (*inPtr == '\\' || *inPtr == '/')
        {
            break;
        }
        inPtr--;
    }

    if (inPtr == fileName)
    {
        path[0] = '\0';
        return path;
    }

    pathLen = inPtr - fileName + 2; /* 包含斜杠与结束符 */

    strSafeCopy(outPtr, fileName, maxLen > pathLen ? pathLen : maxLen);

    return path;
}

/* 字符串转布尔类型 */
BOOL strToBool(const char *const str)
{
    char tempStr[MAX_TEXT_LENGTH];
    toLower(tempStr, (char *)str);
    trim(tempStr);
    if (strcmp("false", tempStr) == 0)
    {
        return FALSE;
    }

    return TRUE;
}

/* 布尔类型转字符串 */
char *boolToStr(char *opStr, BOOL boolValue)
{
    if (boolValue == TRUE)
    {
        strcpy(opStr, "true");
    }
    else
    {
        strcpy(opStr, "false");
    }

    return opStr;
}

void unicode_to_utf8(unsigned int codepoint, char *utf8_buffer)
{
    if (codepoint <= 0x7F)
    {
        utf8_buffer[0] = (char)codepoint;
        utf8_buffer[1] = '\0';
    }
    else if (codepoint <= 0x7FF)
    {
        utf8_buffer[0] = (char)(0xC0 | (codepoint >> 6));
        utf8_buffer[1] = (char)(0x80 | (codepoint & 0x3F));
        utf8_buffer[2] = '\0';
    }
    else if (codepoint <= 0xFFFF && (codepoint < 0xD800 || codepoint > 0xDFFF))
    { /* U+D800-U+DFFF 是 UTF-16 代理项，不是字符，编码后不是合法的 UTF-8 */
        utf8_buffer[0] = (char)(0xE0 | (codepoint >> 12));
        utf8_buffer[1] = (char)(0x80 | ((codepoint >> 6) & 0x3F));
        utf8_buffer[2] = (char)(0x80 | (codepoint & 0x3F));
        utf8_buffer[3] = '\0';
    }
    else if (codepoint > 0xFFFF && codepoint <= 0x10FFFF)
    {
        utf8_buffer[0] = (char)(0xF0 | (codepoint >> 18));
        utf8_buffer[1] = (char)(0x80 | ((codepoint >> 12) & 0x3F));
        utf8_buffer[2] = (char)(0x80 | ((codepoint >> 6) & 0x3F));
        utf8_buffer[3] = (char)(0x80 | (codepoint & 0x3F));
        utf8_buffer[4] = '\0';
    }
    else
    {
        // Invalid code point
        utf8_buffer[0] = '\0';
    }
}
