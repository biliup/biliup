/*
 * XML -> ASS through the vendored DanmakuFactory core, following the CLI's
 * main() step by step (src/main.c at the pinned commit):
 *
 *   readXml -> blockByType -> normFontSize -> sortList -> writeAss
 *
 * What main() does around these calls is replaced:
 *   - options come from df_options instead of DanmakuFactoryConfig.json next
 *     to the executable and argv; the range checks are main()'s, plus checks
 *     for NaN, int overflow and the font name;
 *   - errors are returned instead of printed, with prompts never waiting on
 *     stdin (main() runs with --ignore-warnings --force);
 *   - the danmaku list is freed (main() exits without freeing it).
 *
 * Audit of the core on this path (vendored sources, see ../patches):
 *   - Static data: assEscapeList is a const table. The only mutable statics
 *     (debug_cnt/debug_total) are in assFileToDanmaku(), which is not called.
 *   - Output: printf/fprintf to stdout/stderr only happen under
 *     PRINT_ERR == TRUE (it is FALSE), in printConfig() and in the block-list
 *     regex path; none is reached. There is no exit() or abort().
 *   - Process state: no chdir, setlocale, getenv, rand or strtok. atof() and
 *     printf("%f") depend on LC_NUMERIC, which is checked below.
 *   - Message boxes (gift/SC/guard) are disabled: that code calls
 *     getMsgBoxHeight(), which has no return statement, and dereferences
 *     user parts that may be NULL. They are drawn as "unknown type" comments.
 *   - Statistics boxes and block lists are not exposed. PCRE2 is linked for
 *     blockByType() but only runs for regex block lists, so never here.
 * The fixes for the memory errors found during the audit are in
 * ../patches (out-of-bounds line placement, a trailing "&#", long tags,
 * double fclose on write errors, leaked file handles and list parts).
 */
#include "df_ffi.h"

#include "CDanmakuFactory.h"

#include <errno.h>
#include <locale.h>
#include <math.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

#define DF_FFI_VERSION                                                                                                 \
    "DanmakuFactory 3813e93b13b087e95901d1822baf9c3540e3f3f6 with biliup patches 0001-0007, PCRE2 10.49"

/* The CLI's defaultConfig in main.c. */
#define DEFAULT_FONT_NAME "Microsoft YaHei"
/* Message box settings only appear in the "MSG" style line. */
#define DEFAULT_MSGBOX_WIDTH 500
#define DEFAULT_MSGBOX_HEIGHT 1080
#define DEFAULT_MSGBOX_X 20
#define DEFAULT_MSGBOX_Y 0
#define DEFAULT_MSGBOX_FONT_SIZE 38

#define MAX_TIME_SECONDS 3600.0f
#define MAX_FONT_SIZE 4096
#define MAX_OUTLINE_BLUR 100.0f
#define ALL_BLOCK_BITS                                                                                                 \
    (DF_BLOCK_R2L | DF_BLOCK_L2R | DF_BLOCK_TOP | DF_BLOCK_BOTTOM | DF_BLOCK_SPECIAL | DF_BLOCK_COLOR | DF_BLOCK_REPEAT)

uint32_t df_ffi_abi_version(void)
{
    return DF_FFI_ABI_VERSION;
}

const char *df_ffi_version(void)
{
    return DF_FFI_VERSION;
}

void df_options_init(df_options *options)
{
    if (options == NULL)
    {
        return;
    }
    memset(options, 0, sizeof(*options));
    options->struct_size = (uint32_t)sizeof(*options);
    options->resolution_x = 1920;
    options->resolution_y = 1080;
    options->display_area = 1.0f;
    options->scroll_area = 1.0f;
    options->scroll_time = 12.0f;
    options->fix_time = 5.0f;
    options->density = 0;
    options->line_spacing = 0;
    options->top_margin = 0;
    options->bottom_margin = 0;
    options->font_size = 38;
    options->font_size_strict = 0;
    options->font_size_norm = 0;
    options->font_name = DEFAULT_FONT_NAME;
    options->opacity = 180;
    options->outline = 0.0f;
    options->outline_blur = 0.0f;
    options->outline_opacity = 255;
    options->shadow = 1.0f;
    options->bold = 0;
    options->save_blocked = 1;
    options->block_mode = 0;
}

/* The offset of a member that follows a char is the member's alignment. */
struct options_alignment
{
    char c;
    df_options options;
};
struct result_alignment
{
    char c;
    df_result result;
};

void df_ffi_layout(df_layout *layout)
{
    if (layout == NULL)
    {
        return;
    }
    layout->options_size = (uint32_t)sizeof(df_options);
    layout->options_align = (uint32_t)offsetof(struct options_alignment, options);
    layout->options_font_name_offset = (uint32_t)offsetof(df_options, font_name);
    layout->options_block_mode_offset = (uint32_t)offsetof(df_options, block_mode);
    layout->result_size = (uint32_t)sizeof(df_result);
    layout->result_align = (uint32_t)offsetof(struct result_alignment, result);
    layout->result_message_offset = (uint32_t)offsetof(df_result, message);
}

static int32_t fail(df_result *result, int32_t status, int32_t upstream_code, int32_t os_error, const char *format,
                    ...)
{
    va_list args;
    result->status = status;
    result->upstream_code = upstream_code;
    result->os_error = os_error;
    va_start(args, format);
    vsnprintf(result->message, sizeof(result->message), format, args);
    va_end(args);
    result->message[sizeof(result->message) - 1] = '\0';
    return status;
}

static int is_bool(int32_t value)
{
    return value == 0 || value == 1;
}

static int float_between(float value, float low, float high)
{
    return isfinite(value) && value >= low && value <= high;
}

/* Strict UTF-8: no overlong forms, surrogates or code points above U+10FFFF. */
static int is_valid_utf8(const unsigned char *s)
{
    while (*s != '\0')
    {
        unsigned char c = *s;
        int extra;
        unsigned long code;
        if (c < 0x80)
        {
            s++;
            continue;
        }
        else if (c >= 0xC2 && c <= 0xDF)
        {
            extra = 1;
            code = c & 0x1F;
        }
        else if (c >= 0xE0 && c <= 0xEF)
        {
            extra = 2;
            code = c & 0x0F;
        }
        else if (c >= 0xF0 && c <= 0xF4)
        {
            extra = 3;
            code = c & 0x07;
        }
        else
        {
            return 0;
        }
        for (int i = 1; i <= extra; i++)
        {
            if ((s[i] & 0xC0) != 0x80)
            {
                return 0;
            }
            code = (code << 6) | (s[i] & 0x3F);
        }
        if ((extra == 2 && (code < 0x800 || (code >= 0xD800 && code <= 0xDFFF))) ||
            (extra == 3 && (code < 0x10000 || code > 0x10FFFF)))
        {
            return 0;
        }
        s += extra + 1;
    }
    return 1;
}

/* NULL when valid, otherwise which option is wrong. The ranges are main()'s. */
static const char *check_options(const df_options *o)
{
    if (o->resolution_x < 1 || o->resolution_x > DF_FFI_MAX_RESOLUTION || o->resolution_y < 1 ||
        o->resolution_y > DF_FFI_MAX_RESOLUTION)
    {
        return "resolution";
    }
    if (!float_between(o->display_area, (float)EPS, (float)(1.0 + EPS)))
    {
        return "display_area";
    }
    if (!float_between(o->scroll_area, (float)EPS, (float)(1.0 + EPS)))
    {
        return "scroll_area";
    }
    if (!float_between(o->scroll_time, (float)EPS, MAX_TIME_SECONDS))
    {
        return "scroll_time";
    }
    if (!float_between(o->fix_time, (float)EPS, MAX_TIME_SECONDS))
    {
        return "fix_time";
    }
    if (o->density < -1)
    {
        return "density";
    }
    if (o->line_spacing < -DF_FFI_MAX_RESOLUTION || o->line_spacing > DF_FFI_MAX_RESOLUTION)
    {
        return "line_spacing";
    }
    if (o->top_margin < -DF_FFI_MAX_RESOLUTION || o->top_margin > DF_FFI_MAX_RESOLUTION ||
        o->bottom_margin < -DF_FFI_MAX_RESOLUTION || o->bottom_margin > DF_FFI_MAX_RESOLUTION)
    {
        return "margin";
    }
    if (o->font_size < 1 || o->font_size > MAX_FONT_SIZE)
    {
        return "font_size";
    }
    if (!is_bool(o->font_size_strict) || !is_bool(o->font_size_norm) || !is_bool(o->bold) ||
        !is_bool(o->save_blocked))
    {
        return "boolean option";
    }
    if (o->font_name == NULL || o->font_name[0] == '\0' || strlen(o->font_name) >= DF_FFI_FONT_NAME_LEN ||
        !is_valid_utf8((const unsigned char *)o->font_name))
    {
        return "font_name";
    }
    for (const unsigned char *c = (const unsigned char *)o->font_name; *c != '\0'; c++)
    {
        /* ',' separates the fields of an ASS style line */
        if (*c < 0x20 || *c == 0x7F || *c == ',')
        {
            return "font_name";
        }
    }
    if (o->opacity < 1 || o->opacity > 255)
    {
        return "opacity";
    }
    if (!float_between(o->outline, 0.0f, 4.0f))
    {
        return "outline";
    }
    if (!float_between(o->outline_blur, 0.0f, MAX_OUTLINE_BLUR))
    {
        return "outline_blur";
    }
    if (o->outline_opacity < 0 || o->outline_opacity > 255)
    {
        return "outline_opacity";
    }
    if (!float_between(o->shadow, 0.0f, 4.0f))
    {
        return "shadow";
    }
    if ((o->block_mode & ~ALL_BLOCK_BITS) != 0)
    {
        return "block_mode";
    }
    return NULL;
}

static void to_config(const df_options *o, CONFIG *config)
{
    memset(config, 0, sizeof(*config));
    config->resolution.x = o->resolution_x;
    config->resolution.y = o->resolution_y;
    config->displayarea = o->display_area;
    config->scrollarea = o->scroll_area;
    config->scrolltime = o->scroll_time;
    config->fixtime = o->fix_time;
    config->density = o->density;
    config->lineSpacing = o->line_spacing;
    config->topMargin = o->top_margin;
    config->bottomMargin = o->bottom_margin;
    config->fontsize = o->font_size;
    config->fontSizeStrict = o->font_size_strict;
    config->fontSizeNorm = o->font_size_norm;
    /* length checked against DF_FFI_FONT_NAME_LEN == FONTNAME_LEN */
    memcpy(config->fontname, o->font_name, strlen(o->font_name) + 1);
    config->opacity = o->opacity;
    config->outline = o->outline;
    config->outlineBlur = o->outline_blur;
    config->outlineOpacity = o->outline_opacity;
    config->shadow = o->shadow;
    config->bold = o->bold;
    config->saveBlockedPart = o->save_blocked;
    config->showUserNames = FALSE;
    config->showMsgBox = FALSE;
    config->msgBoxSize.x = DEFAULT_MSGBOX_WIDTH;
    config->msgBoxSize.y = DEFAULT_MSGBOX_HEIGHT;
    config->msgBoxPos.x = DEFAULT_MSGBOX_X;
    config->msgBoxPos.y = DEFAULT_MSGBOX_Y;
    config->msgboxFontsize = DEFAULT_MSGBOX_FONT_SIZE;
    config->msgboxDuration = 0.0f;
    config->giftMinPrice = 0.0f;
    config->blockmode = o->block_mode;
    config->statmode = 0;
    config->blocklist = NULL;
    config->blocklistRegexEnabled = FALSE;
}

int32_t df_convert_xml_to_ass(const char *input_path, const char *output_path, const df_options *options,
                              df_result *result)
{
    if (result == NULL || result->struct_size != sizeof(df_result))
    {
        return DF_ERR_INVALID_ARGUMENT;
    }
    result->status = DF_OK;
    result->upstream_code = 0;
    result->os_error = 0;
    result->danmaku_count = 0;
    result->blocked_count = 0;
    result->message[0] = '\0';

    if (input_path == NULL || output_path == NULL || options == NULL)
    {
        return fail(result, DF_ERR_INVALID_ARGUMENT, 0, 0, "missing input path, output path or options");
    }
    if (options->struct_size != sizeof(df_options))
    {
        return fail(result, DF_ERR_INVALID_ARGUMENT, 0, 0, "df_options size %u, expected %u",
                    (unsigned)options->struct_size, (unsigned)sizeof(df_options));
    }
    const char *invalid = check_options(options);
    if (invalid != NULL)
    {
        return fail(result, DF_ERR_INVALID_ARGUMENT, 0, 0, "invalid %s", invalid);
    }
    const struct lconv *numeric = localeconv();
    if (numeric == NULL || numeric->decimal_point == NULL || strcmp(numeric->decimal_point, ".") != 0)
    {
        return fail(result, DF_ERR_LOCALE, 0, 0, "LC_NUMERIC must use '.' as the decimal point");
    }

    CONFIG config;
    to_config(options, &config);

    DANMAKU *pool = NULL;
    errno = 0;
    int code = readXml(input_path, &pool, "a", 0.0f, NULL);
    if (code != 0)
    {
        int err = errno;
        /* On failure readXml has released the list itself (errorExit) and
         * leaves a stale head pointer behind; it must not be freed again. */
        pool = NULL;
        switch (code)
        {
        case 1:
            return fail(result, DF_ERR_INPUT_OPEN, code, err, "cannot open the input file");
        case 2:
        case 3:
        case 4:
            return fail(result, DF_ERR_INPUT_READ, code, err, "cannot read the input file");
        case 5:
        case 6:
        case 7:
            return fail(result, DF_ERR_OUT_OF_MEMORY, code, 0, "out of memory while reading the input");
        case 8:
            return fail(result, DF_ERR_NO_DANMAKU, code, 0, "the input contains no danmaku");
        default:
            return fail(result, DF_ERR_INTERNAL, code, 0, "readXml returned %d", code);
        }
    }

    int32_t count = 0;
    for (DANMAKU *ptr = pool; ptr != NULL; ptr = ptr->next)
    {
        /* readXml allocates the special part only for type 7; a negative
         * type such as -5 still counts as special and would crash writeAss */
        if (IS_SPECIAL(ptr) && ptr->special == NULL)
        {
            freeList(pool);
            return fail(result, DF_ERR_UNSUPPORTED_INPUT, 0, 0, "danmaku %d is a special danmaku without data",
                        (int)count + 1);
        }
        count++;
    }
    result->danmaku_count = count;

    blockByType(pool, config.blockmode, NULL, config.blocklistRegexEnabled);
    normFontSize(pool, config);

    code = sortList(&pool, NULL);
    if (code != 0)
    {
        /* sortList fails before touching the list */
        freeList(pool);
        if (code == 2)
        {
            return fail(result, DF_ERR_OUT_OF_MEMORY, code, 0, "out of memory while sorting");
        }
        return fail(result, DF_ERR_INTERNAL, code, 0, "sortList returned %d", code);
    }

    errno = 0;
    code = writeAss(output_path, pool, config, NULL, NULL);
    int err = errno;

    int32_t blocked = 0;
    for (DANMAKU *ptr = pool; ptr != NULL; ptr = ptr->next)
    {
        if (ptr->type < 0)
        {
            blocked++;
        }
    }
    result->blocked_count = blocked;
    freeList(pool);

    /* hundreds: the file (1 create, 2 write/close); tens: writeAssDanmakuPart;
     * units: writeAssStatPart, which returns 0 with statistics off */
    int file_part = code / 100;
    int danmaku_part = code / 10 % 10;
    if (danmaku_part >= 3 && danmaku_part <= 8)
    {
        return fail(result, DF_ERR_OUT_OF_MEMORY, code, 0, "out of memory while writing");
    }
    if (file_part == 2 || danmaku_part == 9)
    {
        return fail(result, DF_ERR_OUTPUT_WRITE, code, err, "cannot write the output file");
    }
    if (file_part == 1)
    {
        return fail(result, DF_ERR_OUTPUT_CREATE, code, err, "cannot create the output file");
    }
    if (code != 0)
    {
        return fail(result, DF_ERR_INTERNAL, code, 0, "writeAss returned %d", code);
    }
    return DF_OK;
}
