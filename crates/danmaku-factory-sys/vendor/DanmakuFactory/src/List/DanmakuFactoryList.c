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
#define PCRE2_CODE_UNIT_WIDTH 8
#include "DanmakuFactoryList.h"
#include "../Config/Config.h"
#include "../Define/DanmakuDef.h"
#include <pcre2.h>
#include <stdio.h>

/*
 * 排序整个链表（桶排序）
 * 参数：要排序的链表头
 * 返回值：
 * 0 正常退出
 * 1 弹幕池为空
 * 2 桶空间申请失败
 */
int sortList(DANMAKU **listHead, STATUS *const status)
{
    /* 刷新status */
    if (status != NULL)
    {
        status->function = (void *)&sortList;
        (status->completedNum) = 0;
        status->isDone = FALSE;
    }

    if (*listHead == NULL)
    {
#if PRINT_ERR == TRUE
        printf("\n[X] 弹幕池为空");
#endif
        return 1;
    }

    DANMAKU *now = (*listHead)->next;
    BOOL isSorted = TRUE;

    /* 统计弹幕数量并找出最大最小值 */
    int danmakuNum = 1; // 弹幕条数
    int max;            // 最大值
    int min;            // 最小值
    for (max = min = (*listHead)->time; now != NULL; now = now->next, ++danmakuNum)
    {
        if (now->time > max)
        {
            max = now->time;
        }
        else if (now->time < min)
        {
            min = now->time;
        }

        if (isSorted == TRUE && now->next != NULL && now->time > now->next->time)
        {
            isSorted = FALSE;
        }
    }

    /* 如果本来就排序好了直接退出 */
    if (isSorted == TRUE)
    {
        return 0;
    }

    /* 申请桶空间并清0 */
    int bucketNum = danmakuNum / 128 + 1;
    DANMAKU **bucket = NULL;
    if ((bucket = (DANMAKU **)malloc(sizeof(DANMAKU *) * bucketNum)) == NULL)
    {
#if PRINT_ERR == TRUE
        printf("\n[X] 申请内存空间失败");
#endif
        return 2;
    }
    memset(bucket, 0, sizeof(DANMAKU *) * bucketNum);

    /* 入桶 */
    int index; // 桶号
    double denominator = max - min + 1;
    DANMAKU *ptr;
    DANMAKU *last;
    do
    {
        now = *listHead;
        index = (int)(bucketNum * ((now->time - min) / denominator));
        if (index >= bucketNum || index < 0)
        {
            /* 溢出非法索引处理 */
            index = bucketNum - 1;
        }

        *listHead = (*listHead)->next;
        if (bucket[index] == NULL)
        { /* 如果该桶为空则将新节点指针填入 */
            bucket[index] = now;
            now->next = NULL;
        }
        else
        {
            ptr = last = bucket[index];
            if (now->time <= ptr->time)
            { /* 判断是否为该桶最小值 */
                bucket[index] = now;
            }
            else
            {
                while ((ptr = ptr->next) != NULL && now->time > ptr->time)
                {
                    last = ptr;
                }
                last->next = now;
            }
            now->next = ptr;
        }

        /* 刷新status */
        if (status != NULL)
        {
            (status->completedNum)++;
        }
    } while (*listHead != NULL);

    /* 出桶 */
    now = *listHead = bucket[0];
    while (now->next != NULL)
    {
        now = now->next;
    }
    for (int cnt = 1; cnt < bucketNum; cnt++)
    {
        ptr = bucket[cnt];
        if (ptr != NULL)
        {
            now->next = ptr;
            while (ptr->next != NULL)
            {
                ptr = ptr->next;
            }
            now = ptr;
        }
    }
    free(bucket);

    /* 刷新status */
    if (status != NULL)
    {
        status->isDone = TRUE;
    }
    return 0;
}

/*
 * 弹幕按类型屏蔽
 * 参数：弹幕链表头/屏蔽模式/屏蔽关键字串集/关键词屏蔽是否启用正则表达式
 * 返回值：空
 * 附屏蔽模式：
 * BLK_R2L         屏蔽右左滚动
 * BLK_L2R         屏蔽左右滚动
 * BLK_TOP         屏蔽顶端固定
 * BLK_BOTTOM      屏蔽底端固定
 * BLK_SPECIAL     屏蔽特殊弹幕
 * BLK_COLOR       屏蔽非白色弹幕
 */
void blockByType(DANMAKU *const danmakuHead, const int mode, char **keyStrings, BOOL blocklistRegexEnabled)
{
    if (mode == 0 && keyStrings == NULL)
    {
        return;
    }

    // 先编译所有正则表达式
    pcre2_code **regCodes = NULL;
    int regCodeCount = 0;

    if (blocklistRegexEnabled && keyStrings != NULL)
    {
        for (int i = 0; keyStrings[i] != NULL; i++)
        {
            int regexErrorCode;
            PCRE2_SIZE regexErrorOffset;
            pcre2_code *code = pcre2_compile((PCRE2_SPTR)keyStrings[i], PCRE2_ZERO_TERMINATED, PCRE2_UTF,
                                             &regexErrorCode, &regexErrorOffset, NULL);
            if (code == NULL)
            {
                PCRE2_UCHAR errorMsg[256];
                pcre2_get_error_message(regexErrorCode, errorMsg, sizeof(errorMsg));
                fprintf(stderr,
                        "\nERROR"
                        "\nFailed to compile regex expression %d '%s': %s\n",
                        i, keyStrings[i], errorMsg);
            }
            else
            {
                pcre2_code **newRegCodes = (pcre2_code **)realloc(regCodes, sizeof(pcre2_code *) * (regCodeCount + 1));
                if (newRegCodes != NULL)
                {
                    regCodes = newRegCodes;
                    regCodes[regCodeCount] = code;
                    regCodeCount++;
                }
                else
                {
                    // 内存分配失败，释放当前编译的正则表达式
                    pcre2_code_free(code);
                    fprintf(stderr, "\nERROR: Failed to allocate memory for regex code\n");
                }
            }
        }
    }

    DANMAKU *ptr = (DANMAKU *)danmakuHead;
    while (ptr != NULL)
    {
        if ((mode & BLK_COLOR) && !IS_SPECIAL(ptr) && ptr->color != 0xFFFFFF)
        {
            if (ptr->type > 0)
            {
                ptr->type *= -1;
            }
        }
        else if ((mode & BLK_R2L) && IS_R2L(ptr))
        {
            if (ptr->type > 0)
            {
                ptr->type *= -1;
            }
        }
        else if ((mode & BLK_L2R) && IS_L2R(ptr))
        {
            if (ptr->type > 0)
            {
                ptr->type *= -1;
            }
        }
        else if ((mode & BLK_TOP) && IS_TOP(ptr))
        {
            if (ptr->type > 0)
            {
                ptr->type *= -1;
            }
        }
        else if ((mode & BLK_BOTTOM) && IS_BTM(ptr))
        {
            if (ptr->type > 0)
            {
                ptr->type *= -1;
            }
        }
        else if ((mode & BLK_SPECIAL) && IS_SPECIAL(ptr))
        {
            if (ptr->type > 0)
            {
                ptr->type *= -1;
            }
        }
        // 如果有关键字串集
        if (keyStrings != NULL && ptr->text != NULL)
        {
            // 逐个检查关键字串
            for (int i = 0; keyStrings[i] != NULL; i++)
            {
                if (blocklistRegexEnabled && i < regCodeCount)
                {
                    // 使用正则表达式匹配
                    pcre2_match_data *matchData = pcre2_match_data_create_from_pattern(regCodes[i], NULL);
                    int matchNum =
                        pcre2_match(regCodes[i], (PCRE2_SPTR)ptr->text, PCRE2_ZERO_TERMINATED, 0, 0, matchData, NULL);

                    if (matchNum < 0)
                    {
                        switch (matchNum)
                        {
                        case PCRE2_ERROR_NOMATCH:
                            // 未能匹配到，无需处理
                            break;
                        default: {
                            // 其他错误
                            PCRE2_UCHAR errorMsg[256];
                            pcre2_get_error_message(matchNum, errorMsg, sizeof(errorMsg));
                            fprintf(stderr,
                                    "\nERROR"
                                    "\nRegex match failed for '%s': %s\n",
                                    keyStrings[i], errorMsg);
                        }
                        }
                        pcre2_match_data_free(matchData);
                    }
                    else
                    {
                        // 匹配成功
                        PCRE2_SIZE *ovector = pcre2_get_ovector_pointer(matchData);
                        if (ptr->type > 0)
                        {
                            ptr->type *= -1;
                        }
                        pcre2_match_data_free(matchData);
                        break;
                    }
                }
                else
                {
                    const char *key = keyStrings[i];
                    size_t keyLen = strlen(key);

                    // 如果黑名单文本被<>包裹，认为是uid，使用ptr -> user -> uid进行完整匹配
                    if (keyLen >= 3 && keyStrings[i][0] == '<' && keyStrings[i][keyLen - 1] == '>')
                    {
                        if (ptr->user != NULL && ptr->user->uid != 0)
                        {
                            char *uidStr = (char *)malloc(keyLen - 1);
                            if (uidStr != NULL)
                            {
                                strncpy(uidStr, keyStrings[i] + 1, keyLen - 2);
                                uidStr[keyLen - 2] = '\0';
                                if (ptr->user->uid == atoi(uidStr))
                                {
                                    if (ptr->type > 0)
                                    {
                                        ptr->type *= -1;
                                    }
                                    free(uidStr);
                                    break;
                                }
                                free(uidStr);
                            }
                        }
                    }
                    // 如果黑名单文本被[]包裹，认为是user，使用ptr -> user -> name进行完整匹配
                    else if (keyLen >= 3 && keyStrings[i][0] == '[' && keyStrings[i][keyLen - 1] == ']')
                    {
                        if (ptr->user != NULL && ptr->user->name[0] != '\0')
                        {
                            char *nameStr = (char *)malloc(keyLen - 1);
                            if (nameStr != NULL)
                            {
                                strncpy(nameStr, keyStrings[i] + 1, keyLen - 2);
                                nameStr[keyLen - 2] = '\0';
                                if (strcmp(ptr->user->name, nameStr) == 0)
                                {
                                    if (ptr->type > 0)
                                    {
                                        ptr->type *= -1;
                                    }
                                    free(nameStr);
                                    break;
                                }
                                free(nameStr);
                            }
                        }
                    }
                    // 使用简单的字符串匹配
                    else if (strstr(ptr->text, keyStrings[i]) != NULL)
                    {
                        if (ptr->type > 0)
                        {
                            ptr->type *= -1;
                        }
                        break;
                    }
                }
            }
        }
        ptr = ptr->next;
    }

    // 释放编译的正则表达式
    if (blocklistRegexEnabled && regCodes != NULL)
    {
        for (int i = 0; i < regCodeCount; i++)
        {
            pcre2_code_free(regCodes[i]);
        }
        free(regCodes);
    }

    if (keyStrings != NULL && !blocklistRegexEnabled)
    {
        for (int i = 0; keyStrings[i] != NULL; ++i)
        {
            free(keyStrings[i]);
        }
    }
}

/*
 * 释放整个链表
 * 参数：
 * 要释放的链表头
 */
void freeList(DANMAKU *listHead)
{
    DANMAKU *ptr = listHead;
    while (ptr != NULL)
    {
        listHead = ptr->next;
        free(ptr->text); /* 释放文本部分的空间 */
        free(ptr->user);
        free(ptr->gift);
        free(ptr->special);
        free(ptr);
        ptr = listHead;
    }
}

void normFontSize(DANMAKU *const danmakuHead, const CONFIG config)
{
    BOOL doNormalize = FALSE;

    if (config.fontSizeNorm)
    {
        for (DANMAKU *ptr = (DANMAKU *)danmakuHead; ptr != NULL; ptr = ptr->next)
        {
            if (ptr->type == SPECIAL)
            {
                continue;
            }

            if (ptr->fontSize <= 0 || ptr->fontSize >= config.resolution.y)
            {
                doNormalize = TRUE;
                break;
            }
        }
    }

    if (config.fontSizeStrict || doNormalize)
    {
        for (DANMAKU *ptr = (DANMAKU *)danmakuHead; ptr != NULL; ptr = ptr->next)
        {
            if (ptr->type == SPECIAL)
            {
                continue;
            }

            ptr->fontSize = 25;
        }
    }
}
