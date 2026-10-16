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

#ifndef __CONFIG_H__
#define __CONFIG_H__

#ifdef __cplusplus
extern "C"
{
#endif

#include "../Define/DanmakuDef.h"

#define SIZE_OF_BUF 4096
#define KEY_LEN 128
#define VALUE_LEN 512
#define TYPENAME_LEN 64

    struct Coordinate
    {
        int x;
        int y;
    };

    struct Configurations
    {
        struct Coordinate resolution; /* 分辨率 */
        float displayarea;            /* 显示区域 */
        float scrollarea;             /* 滚动区域 */
        float scrolltime;             /* 滚动时间 */
        float fixtime;                /* 固定时间 */

        int density;                 /* 密度 */
        int lineSpacing;             /* 行间距(像素) */
        int topMargin;               /* 顶部距离(像素) */
        int bottomMargin;            /* 底部距离(像素) */
        int fontsize;                /* 字号 */
        BOOL fontSizeStrict;         /* 是否严格保持指定的字号大小 */
        BOOL fontSizeNorm;           /* 是否修正字号 */
        char fontname[FONTNAME_LEN]; /* 字体 */
        int opacity;                 /* 不透明度 */
        float outline;               /* 描边 */
        float outlineBlur;           /* 描边模糊半径 */
        int outlineOpacity;          /* 描边不透明度 */
        float shadow;                /* 阴影 */
        BOOL bold;                   /* 是否加粗 */

        BOOL saveBlockedPart; /* 是否保存屏蔽部分 */
        BOOL showUserNames;   /* 是否显示用户名 */
        BOOL showMsgBox;      /* 是否显示消息框 */

        struct Coordinate msgBoxSize; /* 消息框大小 */
        struct Coordinate msgBoxPos;  /* 消息框位置 */
        int msgboxFontsize;           /* 消息框内文字大小 */
        float msgboxDuration;         /* 消息框持续时长(秒数) */
        float giftMinPrice;           /* 消息框礼物最低价格限制(元) */

        int blockmode;              /* 屏蔽模式 */
        int statmode;               /* 统计模式 */
        char **blocklist;           /* 弹幕黑名单文件，文件内以换行分隔 */
        BOOL blocklistRegexEnabled; /* 黑名单是否启用正则表达式匹配 */
    };

    typedef struct Coordinate COORDIN;
    typedef struct Configurations CONFIG;

    extern CONFIG readConfig(const char *const configFileName, const CONFIG defaultConfig);
    extern BOOL writeConfig(const char *const configFileName, const CONFIG newConfig);
    extern void printConfig(CONFIG config);

#ifdef __cplusplus
}
#endif

#endif
