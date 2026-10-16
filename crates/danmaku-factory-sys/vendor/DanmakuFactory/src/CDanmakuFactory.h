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

#ifndef __C_DANMAKU_FACTORY_H__
#define __C_DANMAKU_FACTORY_H__

#ifdef __cplusplus
extern "C"
{
#endif

#include "Config/Config.h"
#include "Define/DanmakuDef.h"
#include "Define/Status.h"
#include "List/DanmakuFactoryList.h"
#include "String/DanmakuFactoryString.h"
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "AssFile/AssFile.h"
#include "TemplateFile/TemplateFile.h"

    /* xml */
    extern int readXml(const char *const ipFile, DANMAKU **head, const char *mode, const float timeShift,
                       STATUS *const status);
    extern int writeXml(char const *const fileName, DANMAKU *danmakuHead, STATUS *const status);

    /* json */
    extern int readJson(const char *const ipFile, DANMAKU **head, const char *mode, const float timeShift,
                        STATUS *const status);
    extern int writeJson(const char *const fileName, DANMAKU *danmakuHead, STATUS *const status);

#ifdef __cplusplus
}
#endif

#endif
