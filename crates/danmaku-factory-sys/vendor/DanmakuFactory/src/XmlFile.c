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

#include "CDanmakuFactory.h"
#include "FileUtil/FileUtil.h"

#define LABEL_LEN 10240

static char *xmlUnescape(char *const str);
static void errorExit(FILE *ipF, DANMAKU *head, DANMAKU *ptr);

/* 弹幕类型重定义规范 */
/* +-------------+---------+ */
/* |  类型名称   |  编号   | */
/* +-------------+---------+ */
/* |  右左滚动   |    1    | */
/* +-------------+---------+ */
/* |  左右滚动   |    2    | */
/* +-------------+---------+ */
/* |  上方固定   |    3    | */
/* +-------------+---------+ */
/* |  下方固定   |    4    | */
/* +-------------+---------+ */
/* | B站特殊弹幕 |    5    | */
/* +-------------+---------+ */

/*
** 是否包含子串
** 参数：
** 文件指针/存储字符串的指针/字符串长度
** 返回值：
** TRUE 包含
** FALSE 读取失败/不包含
*/
BOOL findSubstr(FILE *file, const char *substr, int maxlen)
{
    // 保存当前文件指针位置
    long currentPos = ftell(file);
    if (currentPos == -1)
    {
        return FALSE; // 获取文件指针位置失败
    }

    char buffer[1024];
    BOOL isFound = FALSE;
    while (maxlen - 1 > 0 &&
           fgets(buffer, maxlen < SIZE_NUM(char, buffer) ? maxlen : SIZE_NUM(char, buffer), file) != NULL)
    {
        if (strstr(buffer, substr) != NULL)
        {
            isFound = TRUE;
            break;
        }
        maxlen -= (int)strlen(buffer);
    }

    // 恢复文件指针位置
    fseek(file, currentPos, SEEK_SET);

    return isFound;
}

/*
 * 读取xml文件加入弹幕池
 * 参数：
 * 文件名/链表头/读取模式（"n"清空新建 / "a"尾部追加）/时轴偏移量
 * 返回值：
 * 0 正常退出
 * 1 打开文件失败
 * 2 3 4 读取文件发生错误
 * 5 6 7 内存空间申请失败
 * 8 文件未能按正确格式读入
 */
int readXml(const char *const ipFile, DANMAKU **head, const char *mode, const float timeShift, STATUS *const status)
{
    FILE *ipF;
    DANMAKU *tailNode = NULL;
    int cnt, ch;

    /* 刷新status */
    if (status != NULL)
    {
        status->function = (void *)readXml;
        status->completedNum = 0;
        status->isDone = FALSE;
    }

    /* 打开文件 */
    if ((ipF = utf8_fopen(ipFile, "r")) == NULL)
    {
        return 1; /* 文件打开失败 */
    }

    // 检查文件是否为录播姬生成的文件
    BOOL isBililiveRecorder = findSubstr(ipF, "<BililiveRecorder", 1024);

    /* 判断读入方式 */
    if (*head == NULL || *mode == 'n')
    { /* 新建模式 */
        freeList(*head);
        *head = NULL;
    }
    else if (*mode == 'a')
    { /* 追加模式 */
        tailNode = *head;
        while (tailNode->next != NULL)
        {
            tailNode = tailNode->next;
        }
    }

    /* 读取xml文件 */
    float time;
    short type;
    short fontSize = 0;
    int color = 0;
    int messageType;
    char *text;
    char tempText[MAX_TEXT_LENGTH];
    char *textPtr;

    char label[LABEL_LEN];
    char key[MAX_TEXT_LENGTH];
    char raw[LABEL_LEN];

    DANMAKU *danmakuNode;
    USERPART *userNode;
    GIFTPART *giftNode;
    SPPART *specialNode;

    char *labelPtr;

    USERPART user;
    GIFTPART gift;

    BOOL isDanmaku;
    BOOL hasUserInfo;
    BOOL hasGiftInfo;

    while (!feof(ipF))
    {
        type = 0;
        isDanmaku = FALSE;
        hasUserInfo = FALSE;
        hasGiftInfo = FALSE;

        text = NULL;

        gift.price = -1;
        gift.count = -1;
        gift.name[0] = '\0';
        gift.duration = 0;
        user.level = -1;
        user.medalLevel = -1;
        user.uid = 0;
        user.medalName[0] = '\0';
        user.name[0] = '\0';

        /* 读取标签内容 */
        while (fgetc(ipF) != '<')
        {
            if (feof(ipF))
            {
                goto ENDREAD;
            }
            if (ferror(ipF))
            {
                errorExit(ipF, *head, tailNode);
                return 2; /* 读文件发生错误 */
            }
        }

        labelPtr = label;
        while ((ch = fgetc(ipF)) != '>')
        {
            if (feof(ipF))
            {
                goto ENDREAD;
            }
            if (ferror(ipF))
            {
                errorExit(ipF, *head, tailNode);
                return 2; /* 读文件发生错误 */
            }

            if (labelPtr - label < LABEL_LEN - 1)
            { /* label长度限制，保留结尾 '\0' 的位置 */
                *labelPtr = ch;
                labelPtr++;
            }
        }
        *labelPtr = '\0';

        /* 解析标签内容 */
        labelPtr = label;
        getNextWord(&labelPtr, tempText, MAX_TEXT_LENGTH, ' ', TRUE);
        if (strcmp(tempText, "d") == 0)
        { /* 普通弹幕 */
            messageType = UNKNOW_TYPE_DANMAKU;
        }
        else if (strcmp(tempText, "gift") == 0)
        { /* 录播姬 - 普通礼物 */
            messageType = MSG_GIFT;
            hasGiftInfo = TRUE;
        }
        else if (strcmp(tempText, "sc") == 0)
        { /* 录播姬 - SuperChat */
            messageType = MSG_SUPER_CHAT;
            hasGiftInfo = TRUE;
        }
        else if (strcmp(tempText, "guard") == 0)
        { /* 录播姬 - 舰长 */
            messageType = MSG_GUARD;
            hasGiftInfo = TRUE;
        }
        else
        { /* 无效标签 */
            continue;
        }

        while (*labelPtr != '\0')
        {
            strGetLeftPart(key, &labelPtr, '=', MAX_TEXT_LENGTH);
            trim(key);

            if (strcmp(key, "p") == 0)
            {
                strGetLeftPart(tempText, &labelPtr, ',', MAX_TEXT_LENGTH);
                time = atof(deQuotMarks(tempText));
                strGetLeftPart(tempText, &labelPtr, ',', MAX_TEXT_LENGTH);
                type = (short)atoi(deQuotMarks(tempText));
                strGetLeftPart(tempText, &labelPtr, ',', MAX_TEXT_LENGTH);
                fontSize = (short)atoi(deQuotMarks(tempText));

                strGetLeftPart(tempText, &labelPtr, ',', MAX_TEXT_LENGTH);
                color = atoi(deQuotMarks(tempText));

                /* 跳过后续无价值参数 */
                strGetLeftPart(NULL, &labelPtr, '\"', MAX_TEXT_LENGTH);
            }
            else if (strcmp(key, "ts") == 0)
            {
                getNextWord(&labelPtr, tempText, MAX_TEXT_LENGTH, ' ', TRUE);
                time = atof(deQuotMarks(tempText));
            }
            else if (strcmp(key, "user") == 0)
            {
                strGetLeftPart(NULL, &labelPtr, '\"', MAX_TEXT_LENGTH);
                strGetLeftPart(user.name, &labelPtr, '\"', USER_NAME_LEN);
                hasUserInfo = TRUE;
            }
            else if (strcmp(key, "uid") == 0)
            {
                getNextWord(&labelPtr, tempText, MAX_TEXT_LENGTH, ' ', TRUE);
                user.uid = strtoull(deQuotMarks(tempText), NULL, 10);
            }
            else if (strcmp(key, "giftname") == 0)
            {
                getNextWord(&labelPtr, gift.name, GIFT_NAME_LEN, ' ', TRUE);
                deQuotMarks(gift.name);
            }
            else if (strcmp(key, "giftcount") == 0 || strcmp(key, "count") == 0)
            {
                getNextWord(&labelPtr, tempText, MAX_TEXT_LENGTH, ' ', TRUE);
                gift.count = atoi(deQuotMarks(tempText));
            }
            else if (strcmp(key, "price") == 0)
            {
                getNextWord(&labelPtr, tempText, MAX_TEXT_LENGTH, ' ', TRUE);
                // 如果是录播姬的 SC 金额，需要乘以 1000
                if (isBililiveRecorder && messageType == MSG_SUPER_CHAT)
                {
                    gift.price = atoi(deQuotMarks(tempText)) * 1000;
                }
                else
                {
                    gift.price = atoi(deQuotMarks(tempText));
                }
            }
            else if (strcmp(key, "time") == 0)
            {
                // sc node
                getNextWord(&labelPtr, tempText, MAX_TEXT_LENGTH, ' ', TRUE);
                gift.duration = GET_MS_FLT(atof(deQuotMarks(tempText)));
            }
            else if (strcmp(key, "level") == 0)
            {
                getNextWord(&labelPtr, tempText, MAX_TEXT_LENGTH, ' ', TRUE);
                switch (atoi(deQuotMarks(tempText)))
                {
                case 1:
                    // 总督
                    gift.duration = gift.price = 19998000;
                    break;
                case 2:
                    // 提督
                    gift.duration = gift.price = 1998000;
                    break;
                case 3:
                    // 舰长
                    gift.duration = gift.price = 198000;
                    break;
                default:
                    // 未知
                    gift.duration = gift.price = 18000;
                    break;
                }
            }
            // BililiveRecorder，开启记录raw
            else if (strcmp(key, "raw") == 0)
            {
                if (hasGiftInfo == TRUE)
                {
                    strGetLeftPart(NULL, &labelPtr, '\"', LABEL_LEN);
                    strGetLeftPart(raw, &labelPtr, '\"', LABEL_LEN);

                    /* 解析raw部分 */
                    char rawKey[KEY_LEN];
                    char rawValue[VALUE_LEN];
                    char *rawPtr = raw;

                    char coinTypeValue[VALUE_LEN] = {0};
                    xmlUnescape(raw);
                    strGetLeftPart(NULL, &rawPtr, '{', LABEL_LEN);

                    while (*rawPtr != '\0')
                    {
                        // TODO: fix json parse.
                        strGetLeftPart(rawKey, &rawPtr, ':', KEY_LEN);
                        strGetLeftPart(rawValue, &rawPtr, ',', VALUE_LEN);
                        deQuotMarks(rawKey);
                        deQuotMarks(rawValue);
                        if (strcmp(rawKey, "gift_name") == 0)
                        {
                            strSafeCopy(gift.name, rawValue, GIFT_NAME_LEN);
                        }
                        else if (strcmp(rawKey, "coin_type") == 0)
                        {
                            strSafeCopy(coinTypeValue, rawValue, GIFT_NAME_LEN);
                        }
                        else if (strcmp(rawKey, "uid") == 0)
                        {
                            if (user.uid == 0)
                            {
                                user.uid = strtoull(rawValue, NULL, 10);
                            }
                        }
                        else if (strcmp(rawKey, "price") == 0)
                        {
                            if (gift.price == -1)
                            {
                                gift.price = atoi(rawValue);
                            }
                        }
                        else if (strcmp(rawKey, "combo_stay_time") == 0)
                        {
                            if (gift.duration == 0)
                            {
                                gift.duration = GET_MS_FLT(atof(rawValue));
                            }
                        }
                    }

                    if (strcmp(coinTypeValue, "silver") == 0)
                    {
                        gift.price = 0;
                    }
                }
            }
            // blrec的礼物，不包含sc和guard
            else if (strcmp(key, "cointype") == 0)
            {
                char coinTypeValue[VALUE_LEN];
                getNextWord(&labelPtr, coinTypeValue, GIFT_NAME_LEN, ' ', TRUE);
                deQuotMarks(coinTypeValue);
                // 银瓜子 —— 免费礼物
                if (strcmp(coinTypeValue, "\xe9\x93\xb6\xe7\x93\x9c\xe5\xad\x90") == 0)
                {
                    gift.price = 0;
                }
            }
            else
            {
                getNextWord(&labelPtr, NULL, MAX_TEXT_LENGTH, ' ', TRUE);
            }
        }

        if (messageType == MSG_GIFT && gift.duration == 0)
        {
            // issues#111
            gift.duration = 5 * 1000;
        }

        if (messageType == UNKNOW_TYPE_DANMAKU || messageType == MSG_SUPER_CHAT)
        {
            /* 读取普通弹幕 或 SC 的文本部分 */
            cnt = 0;
            while ((ch = fgetc(ipF)) != '<' && cnt < MAX_TEXT_LENGTH - 1)
            {
                tempText[cnt] = ch;
                if (feof(ipF))
                {
                    break;
                }
                if (ferror(ipF))
                {
                    errorExit(ipF, *head, tailNode);
                    return 4; /* 读文件发生错误 */
                }
                cnt++;
            }
            tempText[cnt] = '\0';

            /* 申请文本部分空间 */
            if ((text = (char *)malloc((strlen(tempText) + 1) * sizeof(char))) == NULL)
            {
                errorExit(ipF, *head, danmakuNode);
                return 6; /* 申请内存空间失败 */
            }

            strcpy(text, tempText);
        }

        /* 申请一个节点的空间 */
        {
            if ((danmakuNode = (DANMAKU *)malloc(sizeof(DANMAKU))) == NULL)
            {
                errorExit(ipF, *head, tailNode);
                return 5; /* 申请内存空间失败 */
            }

            /* 类型转换 */
            if (messageType == UNKNOW_TYPE_DANMAKU)
            { /* 将xml定义的类型编号转换为程序统一定义的类型编号 */
                switch (type)
                {
                case 1: {
                    type = R2L;
                    break;
                }
                case 6: {
                    type = L2R;
                    break;
                }
                case 5: {
                    type = TOP;
                    break;
                }
                case 4: {
                    type = BOTTOM;
                    break;
                }
                case 7: {
                    type = SPECIAL;
                    break;
                }
                default: {
                    type = type;
                    break;
                }
                }
            }
            else
            {
                type = messageType;
            }

            /* 计算时轴偏移量 */
            time += timeShift;
            if (time < EPS)
            {
                time = 0.0f; /* 如果时间加偏移量是负数则置 0 */
            }

            /* 数据部分赋值 */
            xmlUnescape(text); /* 文本内容xml反转义 */
            danmakuNode->text = text;
            danmakuNode->type = type;
            danmakuNode->time = GET_MS_FLT(time);
            danmakuNode->fontSize = fontSize;
            danmakuNode->color = color;
            danmakuNode->special = NULL;

            danmakuNode->gift = NULL;
            danmakuNode->user = NULL;
            danmakuNode->special = NULL;
            danmakuNode->next = NULL;
        }

        /* 申请用户信息部分空间 */
        if (hasUserInfo == TRUE)
        {
            if ((userNode = (USERPART *)malloc(sizeof(USERPART))) == NULL)
            {
                errorExit(ipF, *head, tailNode);
                return 5; /* 申请内存空间失败 */
            }

            strSafeCopy(userNode->name, user.name, USER_NAME_LEN);
            userNode->uid = user.uid;
            danmakuNode->user = userNode;
        }

        /* 申请礼物信息部分空间 */
        if (hasGiftInfo == TRUE)
        {
            if ((giftNode = (GIFTPART *)malloc(sizeof(GIFTPART))) == NULL)
            {
                errorExit(ipF, *head, tailNode);
                return 5; /* 申请内存空间失败 */
            }

            strSafeCopy(giftNode->name, gift.name, GIFT_NAME_LEN);
            giftNode->count = gift.count;
            giftNode->price = gift.price;
            giftNode->duration = gift.duration;
            danmakuNode->gift = giftNode;
        }

        /* 特殊弹幕解析 */
        if (type == SPECIAL)
        {
            char textPart[MAX_TEXT_LENGTH];

            /* 申请特殊弹幕部分的空间 */
            if ((specialNode = (SPPART *)malloc(sizeof(SPPART))) == NULL)
            {
                errorExit(ipF, *head, danmakuNode);
                return 7; /* 申请内存空间失败 */
            }
            textPtr = text;
            /* [0,0.17,"1-1",7,"文本部分内容",0,0,0,0.17,500,0,true,"微软雅黑",1] */
            strGetLeftPart(NULL, &textPtr, '[', MAX_TEXT_LENGTH);
            specialNode->startX = atof(deQuotMarks(strGetLeftPart(tempText, &textPtr, ',', MAX_TEXT_LENGTH)));
            specialNode->startY = atof(deQuotMarks(strGetLeftPart(tempText, &textPtr, ',', MAX_TEXT_LENGTH)));
            specialNode->fadeStart =
                (int)((1 - atof(deQuotMarks(strGetLeftPart(tempText, &textPtr, '-', MAX_TEXT_LENGTH)))) * 255);
            specialNode->fadeEnd =
                (int)((1 - atof(deQuotMarks(strGetLeftPart(tempText, &textPtr, ',', MAX_TEXT_LENGTH)))) * 255);
            specialNode->existTime =
                GET_MS_FLT(atof(deQuotMarks(strGetLeftPart(tempText, &textPtr, ',', MAX_TEXT_LENGTH))));

            /* 文本部分 */
            strGetLeftPart(NULL, &textPtr, '\"', MAX_TEXT_LENGTH);
            strGetLeftPart(tempText, &textPtr, '\"', MAX_TEXT_LENGTH);
            strrpl(tempText, textPart, "/n", "\n", MAX_TEXT_LENGTH); // 远古弹幕转义换行符
            strGetLeftPart(NULL, &textPtr, ',', MAX_TEXT_LENGTH);
            strcpy(text, textPart);

            specialNode->frZ = atoi(deQuotMarks(strGetLeftPart(tempText, &textPtr, ',', MAX_TEXT_LENGTH)));
            specialNode->frY = atoi(deQuotMarks(strGetLeftPart(tempText, &textPtr, ',', MAX_TEXT_LENGTH)));
            specialNode->endX = atof(deQuotMarks(strGetLeftPart(tempText, &textPtr, ',', MAX_TEXT_LENGTH)));
            specialNode->endY = atof(deQuotMarks(strGetLeftPart(tempText, &textPtr, ',', MAX_TEXT_LENGTH)));
            specialNode->moveTime = atoi(deQuotMarks(strGetLeftPart(tempText, &textPtr, ',', MAX_TEXT_LENGTH)));
            specialNode->pauseTime = atoi(deQuotMarks(strGetLeftPart(tempText, &textPtr, ',', MAX_TEXT_LENGTH)));

            /* 字体部分 */
            strGetLeftPart(NULL, &textPtr, '\"', MAX_TEXT_LENGTH);
            strGetLeftPart(specialNode->fontName, &textPtr, '\"', MAX_TEXT_LENGTH);
            deQuotMarks(specialNode->fontName);

            danmakuNode->special = specialNode;
        }

        /* 链表连接 */
        if (*head == NULL)
        {
            *head = tailNode = danmakuNode;
        }
        else
        {
            tailNode->next = danmakuNode;
            tailNode = danmakuNode;
        }

        /* 更新状态 */
        if (status != NULL)
        {
            (status->totalNum)++;
            (status->completedNum)++;
        }
    } /* 结束 while */
ENDREAD:
    if (*head == NULL)
    {
        fclose(ipF);
        return 8; /* 文件不能按正确格式读入 */
    }

    fclose(ipF);

    /* 刷新status */
    if (status != NULL)
    {
        status->isDone = TRUE;
    }

    return 0;
}

/*
 * 写xml文件
 * 参数：文件名/弹幕池/状态
 * 返回值：
 * 0 正常退出
 * 1 弹幕池为空
 * 2 创建文件失败
 * 3 写文件发生错误
 */
int writeXml(char const *const fileName, DANMAKU *danmakuHead, STATUS *const status)
{
    /* 刷新status */
    if (status != NULL)
    {
        status->function = (void *)writeXml;
        (status->completedNum) = 0;
        status->isDone = FALSE;
    }

    if (danmakuHead == NULL)
    {
        return 1;
    }

    FILE *opF;
    DANMAKU *ptr = danmakuHead;

    char tempText[64];
    int typeInXml;

    if ((opF = utf8_fopen(fileName, "w")) == NULL)
    {
        return 2;
    }
    fprintf(opF, "<?xml version=\"1.0\"?>"
                 "\n<i>");

    fprintf(opF, "\n    <chatserver></chatserver>"
                 "\n    <chatid></chatid>"
                 "\n    <mission></mission>"
                 "\n    <maxlimit></maxlimit>"
                 "\n    <state></state>"
                 "\n    <real_name></real_name>"
                 "\n    <source></source>");

    while (ptr != NULL)
    {
        /* +----------+-----------+-----------+ */
        /* | 类型名称 | 原类型编号| 新类型编号| */
        /* +----------+-----------+-----------+ */
        /* | 右左滚动 |     1     |     1     | */
        /* +----------+-----------+-----------+ */
        /* | 左右滚动 |     6     |     2     | */
        /* +----------+-----------+-----------+ */
        /* | 上方固定 |     5     |     3     | */
        /* +----------+-----------+-----------+ */
        /* | 下方固定 |     4     |     4     | */
        /* +----------+-----------+-----------+ */
        /* | 特殊弹幕 |     7     |     5     | */
        /* +----------+-----------+-----------+ */
        if (IS_R2L(ptr))
        {
            typeInXml = 1;
        }
        else if (IS_L2R(ptr))
        {
            typeInXml = 6;
        }
        else if (IS_TOP(ptr))
        {
            typeInXml = 5;
        }
        else if (IS_BTM(ptr))
        {
            typeInXml = 4;
        }
        else if (IS_SPECIAL(ptr))
        {
            typeInXml = 7;
        }
        else
        {
            ptr = ptr->next;
            continue;
        }
        fprintf(opF, "\n    <d p=\"%s,%d,%d,%d,0,0,NULL,0\">", intTimeToStr(tempText, ptr->time, 3), typeInXml,
                ptr->fontSize, ptr->color);

        if (IS_SPECIAL(ptr) == FALSE)
        {
            fprintf(opF, "%s", ptr->text);
        }
        else
        {
            fprintf(opF, "[%s,", floatToStr(tempText, ptr->special->startX, 2));
            fprintf(opF, "%s,", floatToStr(tempText, ptr->special->startY, 2));
            fprintf(opF, "\"%s", floatToStr(tempText, 1 - (ptr->special->fadeStart / 255.00), 2));
            fprintf(opF, "-%s\",", floatToStr(tempText, 1 - (ptr->special->fadeEnd / 255.00), 2));
            fprintf(opF, "%s,", intTimeToStr(tempText, ptr->special->existTime, 1));
            fprintf(opF, "\"%s\",", ptr->text);
            fprintf(opF, "%d,%d,", ptr->special->frZ, ptr->special->frY);
            fprintf(opF, "%s,", floatToStr(tempText, ptr->special->endX, 2));
            fprintf(opF, "%s,", floatToStr(tempText, ptr->special->endY, 2));
            fprintf(opF, "%d,%d,", ptr->special->moveTime, ptr->special->pauseTime);
            fprintf(opF, "true,\"%s\",1]", ptr->special->fontName);
        }
        fprintf(opF, "</d>");

        ptr = ptr->next;

        if (ferror(opF))
        {
            fclose(opF);
            return 3;
        }

        /* 刷新status */
        if (status != NULL)
        {
            (status->completedNum)++;
        }
    }

    fprintf(opF, "\n</i>");

    fclose(opF);

    /* 刷新status */
    if (status != NULL)
    {
        status->isDone = TRUE;
    }
    return 0;
}

/*
 * xml转义字符反转义
 *
 * 对照：
 *     原      反转义
 *    &lt;        <
 *    &gt;        >
 *    &amp;       &
 *    &apos;      '
 *    &quot;      "
 *    &#34;       "
 */
static char *xmlUnescape(char *const str)
{
    if (str == NULL)
    {
        return NULL;
    }

    char *srcPtr = str; // 源字符串指针
    char *dstPtr = str; // 目标字符串指针，也用于就地修改字符串

    while (*srcPtr != '\0')
    {
        if (*srcPtr != '&')
        {
            // 如果当前字符不是 '&'，就原样复制
            *dstPtr++ = *srcPtr++;
            continue;
        }

        if (strncmp(srcPtr, "&amp;", 5) == 0)
        {
            *dstPtr++ = '&';
            srcPtr += 5;
        }
        else if (strncmp(srcPtr, "&apos;", 6) == 0)
        {
            *dstPtr++ = '\'';
            srcPtr += 6;
        }
        else if (strncmp(srcPtr, "&gt;", 4) == 0)
        {
            *dstPtr++ = '>';
            srcPtr += 4;
        }
        else if (strncmp(srcPtr, "&lt;", 4) == 0)
        {
            *dstPtr++ = '<';
            srcPtr += 4;
        }
        else if (strncmp(srcPtr, "&quot;", 6) == 0)
        {
            *dstPtr++ = '\"';
            srcPtr += 6;
        }
        else if (strncmp(srcPtr, "&#34;", 5) == 0)
        {
            *dstPtr++ = '\"';
            srcPtr += 5;
        }
        else
        {
            // 如果不是已知的转义序列，就原样复制
            *dstPtr++ = *srcPtr++;
        }
    }

    // 字符串结束
    *dstPtr = '\0';

    return str;
}

/*
 * 出错后程序退出前的处理
 * 参数：
 * 文件指针/链表头指针/最后一个未结尾节点指针/
 */
static void errorExit(FILE *ipF, DANMAKU *head, DANMAKU *ptr)
{
    fclose(ipF);
    if (head != NULL)
    {
        ptr->next = NULL;
        freeList(head);
    }
    return;
}
