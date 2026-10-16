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

#ifndef __ASSFILE_H__
#define __ASSFILE_H__

#ifdef __cplusplus
extern "C"
{
#endif

#include "../CDanmakuFactory.h"
#include "AssStringProcessing.h"

#define ASS_MAX_LINE_LEN 4096
#define ASS_TITLE_LEN 128
#define ASS_SCRIPT_TYPE_LEN 16
#define ASS_COLLISIONS_LEN 16

#define TEMPBUF_SIZE 128
#define MARKSTR_SIZE 24

    /*
     * ASS文件结构
     */
    struct AssSubtitleFile
    {
        char title[ASS_TITLE_LEN];            // 标题
        char scriptType[ASS_SCRIPT_TYPE_LEN]; // 版本号
        char collisions[ASS_COLLISIONS_LEN];  // 字幕冲撞时的决策 Normal 或 Reverse
        int playResX;                         // 分辨率X
        int playResY;                         // 分辨率Y
        float timer;                          // 播放速度
        int stylesNum;                        // 样式条数
        struct AssSubtitleFileStyles *styles; // 样式表
        struct AssSubtitleFileEvents *events; // 事件链表
    };

    /*
     * Styles部分 23个字段
     * Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour,
     *         OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut,
     *         ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow,
     *         Alignment, MarginL, MarginR, MarginV, Encoding
     */

#define ASS_STYLE_NAME_LEN 64
#define ASS_STYLE_FONTNAME_LEN 128
#define ASS_COLOR_LEN 16

    struct AssSubtitleFileStyles
    {
        char name[ASS_STYLE_NAME_LEN];         // 名称
        char fontname[ASS_STYLE_FONTNAME_LEN]; // 字体
        int fontsize;                          // 字号
        char primaryColour[ASS_COLOR_LEN];     // 主体颜色
        char secondaryColour[ASS_COLOR_LEN];   // 次要颜色
        char outlineColor[ASS_COLOR_LEN];      // 边框颜色
        char backColour[ASS_COLOR_LEN];        // 背景颜色
        int bold;                              // 粗体 -1开启 0关闭
        int italic;                            // 斜体 -1开启 0关闭
        int underline;                         // 下划线 -1开启 0关闭
        int strikeout;                         // 删除线 -1开启 0关闭
        int scaleX;                            // 横向缩放(%) 100正常
        int scaleY;                            // 纵向缩放(%) 100正常
        float spacing;                         // 字间距(px)
        float angle;                           // 旋转角度(度)
        int borderStyle;                       // 边框样式 1边框阴影 3边框
        float outline;                         // 边框深度(0-4)
        float shadow;                          // 阴影深度(0-4)
        int alignment;                         // 对齐方式
        int marginL;                           // 左边距
        int marginR;                           // 右边距
        int marginV;                           // 垂直边距
        int encoding;                          // 编码方式 0ANSI 1默认 128日文 134简中 136繁中
    };

#define ASS_EVENT_TYPE_LEN 16
#define ASS_EVENT_STYLE_LEN 64
#define ASS_EVENT_NAME_LEN 64
#define ASS_EVENT_EFFECT_LEN 128
    /*
     * Event 部分
     */
    struct AssSubtitleFileEvents
    {
        char event[ASS_EVENT_TYPE_LEN];    // 事件名称(Dialogue/Comment)
        int layer;                         // 字幕图层
        int start;                         // 开始时间(毫秒)
        int end;                           // 结束时间(毫秒)
        char style[ASS_EVENT_STYLE_LEN];   // 样式
        char name[ASS_EVENT_NAME_LEN];     // 角色名称
        int marginL;                       // 左边距 四位整数
        int marginR;                       // 右边距 四位整数
        int marginV;                       // 垂直边距 四位整数
        char effect[ASS_EVENT_EFFECT_LEN]; // 过渡效果
        char *text;                        // 文本部分
        struct AssSubtitleFileEvents *next;
    };

    /*
     * Danmaku 队列
     */
    // typedef struct DanmakuList
    // {
    //     DANMAKU *message;
    //     int height;
    //     int posY;

    //     struct DanmakuList *nextNode;
    // } DMKLIST;

    /*
     * Message 队列
     */
    struct MessageList
    {
        struct ComboList *comboListHead;
        struct ComboList *comboListTail;

        BOOL isShown;
        BOOL isUpMoved; // 该消息是否已完成向上滚动
        int height;
        // int width;
        int posY;

        struct DialogueList *dlgListHead;
        struct DialogueList *dlgListTail;

        struct MessageList *lastNode;
        struct MessageList *nextNode;
    };

    struct MessageListPtrList
    {
        struct MessageList *msgListPtr;

        int thisMsgEndTime; // 精度为 10 毫秒
        int height;
        int posY;

        struct MessageListPtrList *nextNode;
    };

    struct ComboList
    {
        DANMAKU *message;

        struct ComboList *nextNode;
    };

    struct DialogueList
    {
        int startPosX;
        int startPosY;
        int endPosX;
        int endPosY;

        int startTime;
        int endTime;

        char *effect;

        struct DialogueList *nextNode;
    };

    typedef struct AssSubtitleFile ASSFILE;
    typedef struct AssSubtitleFileStyles STYLE;
    typedef struct AssSubtitleFileEvents EVENT;
    typedef struct MessageList MSGLIST;
    typedef struct MessageListPtrList MSGPTRLIST;
    typedef struct ComboList COMBOLIST;
    typedef struct DialogueList DLGLIST;

    /* ass */
    extern int readAss(const char *const fileName, DANMAKU **danmakuHead, const char *mode, ASSFILE *assSub,
                       const float timeShift, STATUS *const status);
    extern int writeAss(const char *const fileName, DANMAKU *danmakuHead, const CONFIG config,
                        const ASSFILE *const subPart, STATUS *const status);

    /* 不常用的ass操作函数 */
    extern int readAssFile(ASSFILE *assFile, const char *const fileName);
    extern int assFileToDanmaku(ASSFILE *inputSub, DANMAKU **danmakuHead, const char *mode, ASSFILE *outputSub,
                                const float timeShift, STATUS *const status);
    extern void writeAssStylesPart(FILE *opF, const int numOfStyles, STYLE *const styles);
    extern int writeAssDanmakuPart(FILE *opF, DANMAKU *head, CONFIG config, STATUS *const status);
    extern int writeAssStatPart(FILE *opF, DANMAKU *head, int mode, const int rollTime, const int holdTime,
                                const int density, const int blockMode);

    extern void freeAssFile(ASSFILE *assFile);

#ifdef __cplusplus
}
#endif

#endif
