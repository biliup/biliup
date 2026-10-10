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

#ifndef __DANMAKUDEF_H__
#define __DANMAKUDEF_H__

#ifdef __cplusplus
extern "C"
{
#endif

#define MAX_ASS_MS_INT 2147483640
#define NUM_OF_TYPE 5
#define EPS 1e-4
#define MAX_TEXT_LENGTH 1024
#define FONTNAME_LEN 128
#define USER_NAME_LEN 128
#define MEDAL_NAME_LEN 32
#define GIFT_NAME_LEN 64

#define TRUE 1
#define FALSE 0
#define BOOL_UNDETERMINED -1

#define PRINT_ERR FALSE

#define UNKNOW_TYPE_DANMAKU 0

#define R2L 1
#define L2R 2
#define TOP 3
#define BOTTOM 4
#define SPECIAL 5

#define MSG_SUPER_CHAT 11
#define MSG_GUARD 12
#define MSG_GIFT 13

#define BLK_R2L 1
#define BLK_L2R 2
#define BLK_TOP 4
#define BLK_BOTTOM 8
#define BLK_SPECIAL 16
#define BLK_COLOR 32
#define BLK_REPEAT 64

#define TABLE 1
#define HISTOGRAM 2

#define IS_GIFT(ptr) (ptr->type == GIFT || ptr->type == -GIFT)
#define IS_R2L(ptr) (ptr->type == R2L || ptr->type == -R2L)
#define IS_L2R(ptr) (ptr->type == L2R || ptr->type == -L2R)
#define IS_TOP(ptr) (ptr->type == TOP || ptr->type == -TOP)
#define IS_BTM(ptr) (ptr->type == BOTTOM || ptr->type == -BOTTOM)
#define IS_SPECIAL(ptr) (ptr->type == SPECIAL || ptr->type == -SPECIAL)

#define IS_MSG_SUPER_CHAT(ptr) (ptr->type == MSG_SUPER_CHAT || ptr->type == -MSG_SUPER_CHAT)
#define IS_MSG_GUARD(ptr) (ptr->type == MSG_GUARD || ptr->type == -MSG_GUARD)
#define IS_MSG_GIFT(ptr) (ptr->type == MSG_GIFT || ptr->type == -MSG_GIFT)

#define IS_NORMAL(ptr) (IS_R2L(ptr) || IS_L2R(ptr) || IS_TOP(ptr) || IS_BTM(ptr))
#define IS_MSG(ptr) (IS_MSG_SUPER_CHAT(ptr) || IS_MSG_GUARD(ptr) || IS_MSG_GIFT(ptr))

#define GET_MS_FLT(sec) ((int)((sec) * 1000.0f + 0.5f))
#define GET_ASS_MS_FLT(sec) ((int)((sec) * 100.0f + 0.55f) * 10)
#define GET_ASS_MS_INT(ms) ((ms + 5) / 10 * 10)

#define SIZE_NUM(type, name) (sizeof(name) / sizeof(type))

    struct SingleDanmaku
    {                                       /* 弹幕节点定义 */
        int time;                           /* 开始时间(毫秒) */
        short type;                         /* 弹幕类型 */
        short fontSize;                     /* 字体大小 25是基准 */
        int color;                          /* 文字颜色 */
        char *text;                         /* 文本内容 */
        struct UserPart *user;              /* 用户信息部分 */
        struct GiftPart *gift;              /* 礼物信息部分 */
        struct SpecialDanmakuPart *special; /* 特殊弹幕部分 */
        struct SingleDanmaku *next;
    };

    /* 用户信息部分 */
    struct UserPart
    {
        unsigned long long uid;
        int level;
        char name[USER_NAME_LEN];
        char medalName[MEDAL_NAME_LEN];
        int medalLevel;
    };

    /* 礼物信息部分 */
    struct GiftPart
    {
        char name[GIFT_NAME_LEN];
        int price; // 礼物金额(厘)
        int count;
        int duration; // 持续时间(毫秒)
    };

    /* 特殊弹幕部分 */
    struct SpecialDanmakuPart
    {
        int existTime;               /* 生存时间  ms */
        int moveTime;                /* 移动时间  ms */
        int pauseTime;               /* 暂停时间  ms */
        float startX;                /* 开始X坐标 小数0-1表比例 */
        float startY;                /* 开始Y坐标 小数0-1表比例 */
        float endX;                  /* 结束X坐标 小数0-1表比例 */
        float endY;                  /* 结束Y坐标 小数0-1表比例 */
        int fadeStart;               /* 渐变开始不透明度 0-255 越大越透明 */
        int fadeEnd;                 /* 渐变结束不透明度 0-255 越大越透明 */
        int frY;                     /* Y轴旋转 */
        int frZ;                     /* Z轴旋转 */
        char fontName[FONTNAME_LEN]; /* 字体名 */
    };

    typedef int BOOL;
    typedef struct SingleDanmaku DANMAKU;
    typedef struct SpecialDanmakuPart SPPART;
    typedef struct UserPart USERPART;
    typedef struct GiftPart GIFTPART;

#ifdef __cplusplus
}
#endif

#endif
