/*
 * Compatibility C interface to the Rust DanmakuFactory implementation: one
 * XML file in, one ASS file out, the configuration passed explicitly.
 *
 * Ownership
 *   - Every pointer argument is borrowed for the duration of the call only.
 *     Strings are NUL-terminated UTF-8.
 *   - Nothing allocated here is returned: results are written into the
 *     caller's df_result, and the Rust-owned list never leaves the call. The
 *     caller frees nothing.
 *   - df_ffi_version() returns a static string.
 *
 * Global state
 *   - No configuration file is read or written, the working directory and
 *     the locale are not changed, and nothing is printed.
 *   - The historical LC_NUMERIC contract is preserved: it must use '.' or
 *     DF_ERR_LOCALE is returned before writing. Rust itself parses and
 *     formats independently of locale. Callers synchronize locale changes.
 *   - Calls own their state and can run concurrently. biliup retains its
 *     worker process for cancellation and timeout handling.
 *
 * Input
 *   Danmaku times are converted to int milliseconds by the core: times above
 *   about 2,000,000 s (555 hours) are rejected if they overflow. Message
 *   boxes (gift/SC/guard) are not enabled through this compatibility ABI.
 *
 * Paths
 *   Windows paths are decoded from UTF-8; Unix paths preserve their bytes.
 *   Files use Rust's filesystem APIs. The output file is created or
 *   truncated; after a failure it may be left incomplete and must be removed
 *   by the caller.
 */
#ifndef BILIUP_DF_FFI_H
#define BILIUP_DF_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

/* Bumped whenever a struct or the meaning of a field changes. */
#define DF_FFI_ABI_VERSION 1
#define DF_FFI_MESSAGE_LEN 256
/* Includes the terminator, like FONTNAME_LEN in the core. */
#define DF_FFI_FONT_NAME_LEN 128
#define DF_FFI_MAX_RESOLUTION 16384

    enum df_status
    {
        DF_OK = 0,
        /* NULL pointer, struct size mismatch or option out of range */
        DF_ERR_INVALID_ARGUMENT = 1,
        /* LC_NUMERIC does not use '.' as the decimal point */
        DF_ERR_LOCALE = 2,
        /* the input could not be opened; os_error holds errno */
        DF_ERR_INPUT_OPEN = 3,
        /* reading the input failed */
        DF_ERR_INPUT_READ = 4,
        /* the input contains no danmaku the core can read */
        DF_ERR_NO_DANMAKU = 5,
        /* the input contains a danmaku the core cannot write safely */
        DF_ERR_UNSUPPORTED_INPUT = 6,
        DF_ERR_OUT_OF_MEMORY = 7,
        /* the output could not be created; os_error holds errno */
        DF_ERR_OUTPUT_CREATE = 8,
        /* writing or closing the output failed, e.g. the disk is full */
        DF_ERR_OUTPUT_WRITE = 9,
        /* an upstream status this wrapper does not know */
        DF_ERR_INTERNAL = 10
    };

    /* Bits of df_options.block_mode, the core's BLK_* values. */
    enum df_block
    {
        DF_BLOCK_R2L = 1,
        DF_BLOCK_L2R = 2,
        DF_BLOCK_TOP = 4,
        DF_BLOCK_BOTTOM = 8,
        DF_BLOCK_SPECIAL = 16,
        /* everything that is not white */
        DF_BLOCK_COLOR = 32,
        /* the same text while an earlier one is still on screen */
        DF_BLOCK_REPEAT = 64
    };

    /*
     * The upstream CONFIG fields that apply to <d> danmaku, named after the
     * CLI options. df_options_init() fills the CLI defaults. Booleans are 0
     * or 1. Message boxes (gift/SC/guard), statistics, user names and block
     * lists are always off.
     */
    typedef struct df_options
    {
        uint32_t struct_size;     /* sizeof(df_options) */
        int32_t resolution_x;     /* --resolution, 1..DF_FFI_MAX_RESOLUTION */
        int32_t resolution_y;
        float display_area;       /* --displayarea, 0.0001..1 */
        float scroll_area;        /* --scrollarea, 0.0001..1 */
        float scroll_time;        /* --scrolltime, seconds, 0.0001..3600 */
        float fix_time;           /* --fixtime, seconds, 0.0001..3600 */
        int32_t density;          /* --density: -1 no overlap, 0 unlimited */
        int32_t line_spacing;     /* --line-spacing, pixels */
        int32_t top_margin;       /* --top-margin, pixels */
        int32_t bottom_margin;    /* --bottom-margin, pixels */
        int32_t font_size;        /* --fontsize, 1..4096 */
        int32_t font_size_strict; /* --font-size-strict */
        int32_t font_size_norm;   /* --font-size-norm */
        const char *font_name;    /* --fontname: no ',' or control characters */
        int32_t opacity;          /* --opacity, 1..255 */
        float outline;            /* --outline, 0..4 */
        float outline_blur;       /* --outline-blur, 0..100 */
        int32_t outline_opacity;  /* --outline-opacity, 0..255 */
        float shadow;             /* --shadow, 0..4 */
        int32_t bold;             /* --bold */
        int32_t save_blocked;     /* --saveblocked */
        int32_t block_mode;       /* --blockmode, DF_BLOCK_* bits */
    } df_options;

    typedef struct df_result
    {
        uint32_t struct_size;  /* sizeof(df_result), set by the caller */
        int32_t status;        /* enum df_status */
        int32_t upstream_code; /* status of the failing core function, else 0 */
        int32_t os_error;      /* errno of a failed open/write, else 0 */
        int32_t danmaku_count; /* danmaku read from the input */
        int32_t blocked_count; /* of those, blocked by type, density or repetition */
        char message[DF_FFI_MESSAGE_LEN]; /* NUL-terminated, empty on success */
    } df_result;

    uint32_t df_ffi_abi_version(void);

    /* Pinned upstream commit and patch level, e.g. for diagnostics. */
    const char *df_ffi_version(void);

    /* Fills the DanmakuFactory CLI defaults (font_name points to a static string). */
    void df_options_init(df_options *options);

    /*
     * Reads input_path as XML and writes output_path as ASS, following the
     * CLI: readXml, blockByType, normFontSize, sortList, writeAss. Returns
     * result->status (or DF_ERR_INVALID_ARGUMENT when result is unusable).
     */
    int32_t df_convert_xml_to_ass(const char *input_path, const char *output_path, const df_options *options,
                                  df_result *result);

    /* sizeof/offsetof of the structs above, so Rust can verify its mirror. */
    typedef struct df_layout
    {
        uint32_t options_size;
        uint32_t options_align;
        uint32_t options_font_name_offset;
        uint32_t options_block_mode_offset;
        uint32_t result_size;
        uint32_t result_align;
        uint32_t result_message_offset;
    } df_layout;

    void df_ffi_layout(df_layout *layout);

#ifdef __cplusplus
}
#endif

#endif
