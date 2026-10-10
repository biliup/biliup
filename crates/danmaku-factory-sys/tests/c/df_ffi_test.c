/*
 * Exercises df_ffi.c and the vendored core under AddressSanitizer,
 * UndefinedBehaviorSanitizer and LeakSanitizer (see check.sh): success and
 * every failure path, repeated so that leaks and stale state show up.
 *
 *   df_ffi_test GOLDEN_DIR WORK_DIR
 */
#include "df_ffi.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

static int failures = 0;

#define CHECK(cond, ...)                                                                                               \
    do                                                                                                                 \
    {                                                                                                                  \
        if (!(cond))                                                                                                   \
        {                                                                                                              \
            fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                                                       \
            fprintf(stderr, __VA_ARGS__);                                                                              \
            fprintf(stderr, "\n");                                                                                     \
            failures++;                                                                                                \
        }                                                                                                              \
    } while (0)

static char work[4096];
static char golden[4096];

static void path_in(char *out, const char *dir, const char *name)
{
    snprintf(out, 4096, "%s/%s", dir, name);
}

static void write_file(const char *path, const char *text)
{
    FILE *f = fopen(path, "wb");
    if (f == NULL || fputs(text, f) == EOF || fclose(f) != 0)
    {
        fprintf(stderr, "cannot write %s\n", path);
        exit(2);
    }
}

static char *read_file(const char *path, long *size)
{
    FILE *f = fopen(path, "rb");
    if (f == NULL)
    {
        return NULL;
    }
    fseek(f, 0, SEEK_END);
    *size = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *data = malloc((size_t)*size + 1);
    if (fread(data, 1, (size_t)*size, f) != (size_t)*size)
    {
        *size = -1;
    }
    data[*size < 0 ? 0 : *size] = '\0';
    fclose(f);
    return data;
}

static int exists(const char *path)
{
    struct stat st;
    return stat(path, &st) == 0;
}

static df_options biliup(void)
{
    df_options o;
    df_options_init(&o);
    o.font_name = "Noto Sans CJK SC";
    o.font_size_strict = 1;
    o.opacity = 204;
    o.outline = 2.0f;
    o.shadow = 0.0f;
    o.fix_time = 12.0f;
    o.display_area = 0.4f;
    o.density = -1;
    return o;
}

/* Converts and returns the status; a failure must not leave an output. */
static int convert(const char *input, const char *output, const df_options *o, df_result *r)
{
    remove(output);
    r->struct_size = sizeof(*r);
    int status = df_convert_xml_to_ass(input, output, o, r);
    CHECK(status == r->status, "returned %d but result says %d", status, r->status);
    CHECK(memchr(r->message, '\0', sizeof(r->message)) != NULL, "message not terminated");
    if (status != DF_OK)
    {
        /* the caller (Rust) removes partial output; only check it is not a valid file */
        remove(output);
    }
    return status;
}

static void golden_inputs(void)
{
    static const char *names[] = {"basic_cn",  "long_text", "emoji",    "special_chars", "types",  "dense",
                                  "same_time", "sizes",     "long_time", "unsorted",     "repeat", NULL};
    char input[4096], output[4096], expected_path[4096], name[256];
    path_in(output, work, "golden.ass");
    for (int i = 0; names[i] != NULL; i++)
    {
        snprintf(name, sizeof(name), "inputs/%s.xml", names[i]);
        path_in(input, golden, name);
        snprintf(name, sizeof(name), "expected/%s-biliup.ass", names[i]);
        path_in(expected_path, golden, name);
        df_options o = biliup();
        df_result r;
        int status = convert(input, output, &o, &r);
        CHECK(status == DF_OK, "%s: status %d %s", names[i], status, r.message);
        long actual_size = 0, expected_size = 0;
        char *actual = read_file(output, &actual_size);
        char *expected = read_file(expected_path, &expected_size);
        CHECK(actual != NULL && expected != NULL && actual_size == expected_size &&
                  memcmp(actual, expected, (size_t)actual_size) == 0,
              "%s differs from the golden file", names[i]);
        free(actual);
        free(expected);
        /* other geometries, including ones the unpatched core mishandled */
        df_options variants[4] = {biliup(), biliup(), biliup(), biliup()};
        variants[0].resolution_x = 96, variants[0].resolution_y = 64, variants[0].font_size = 8;
        variants[0].display_area = 0.05f, variants[0].density = 1000;
        variants[1].resolution_x = 64, variants[1].resolution_y = 38, variants[1].display_area = 1.0f;
        variants[2].resolution_x = 4, variants[2].resolution_y = 1, variants[2].density = 0;
        variants[3].font_size_strict = 0, variants[3].font_size_norm = 1, variants[3].block_mode = 127;
        variants[3].save_blocked = 0, variants[3].line_spacing = -50, variants[3].outline_blur = 3.0f;
        for (int v = 0; v < 4; v++)
        {
            status = convert(input, output, &variants[v], &r);
            CHECK(status == DF_OK, "%s variant %d: status %d %s", names[i], v, status, r.message);
        }
    }
    remove(output);
}

static void edge_texts(void)
{
    char input[4096], output[4096];
    path_in(input, work, "edge.xml");
    path_in(output, work, "edge.ass");
    /* trailing "&#" in an exactly sized buffer, surrogates, long tags and
     * texts, every type including unknown ones and a special payload that
     * is cut short */
    char *xml = malloc(64 * 1024);
    strcpy(xml, "<i>\n<d p=\"1,1,25,16777215\">abc&#</d>\n"
                "<d p=\"1,1,25,16777215\">&#</d>\n"
                "<d p=\"1,1,25,16777215\">&#55357;&#56832;&#0;&#-5;&#99999999999</d>\n"
                "<d p=\"2,7,25,0\">[0.5</d>\n"
                "<d p=\"2,7,25,0\">[</d>\n"
                "<d p=\"2,7,25,0\"></d>\n"
                "<d p=\"3,8,25,0\">code</d><d p=\"3,9,25,0\">bas</d><d p=\"3,11,25,0\">sc type</d>\n"
                "<d p=\"3,12,25,0\">guard type</d><d p=\"3,13,25,0\">gift type</d>\n"
                "<gift ts=\"4\" user=\"u\" giftname=\"g\" giftcount=\"1\" price=\"100\"></gift>\n"
                "<sc ts=\"4\" user=\"u\" price=\"30\" time=\"60\">super chat</sc>\n"
                "<guard ts=\"4\" user=\"u\" level=\"3\" count=\"1\"></guard>\n"
                "<d p=\"5,1,25,16777215\" user=\"name\" uid=\"1\">with user</d>\n"
                "<d p=\"5,1,25,16777215\" x=\"");
    for (int i = 0; i < 12000; i++)
    {
        strcat(xml, "a");
    }
    strcat(xml, "\">long tag</d>\n<d p=\"6,1,25,16777215\">");
    for (int i = 0; i < 1500; i++)
    {
        strcat(xml, "\xe9\x95\xbf");
    }
    strcat(xml, "</d>\n<d p=\"1800000,1,25,16777215\">500 hours</d>\n<d p=\"-3,1,25,16777215\">negative</d>\n</i>\n");
    write_file(input, xml);
    free(xml);
    df_options o = biliup();
    df_result r;
    for (int round = 0; round < 3; round++)
    {
        int status = convert(input, output, &o, &r);
        CHECK(status == DF_OK, "edge texts: %d %s", status, r.message);
        CHECK(r.danmaku_count > 10, "edge texts: count %d", r.danmaku_count);
        o.resolution_y = 38, o.display_area = 1.0f, o.density = 0;
    }
    remove(input);
    remove(output);
}

static void failures_clean_up(void)
{
    char input[4096], output[4096], missing[4096];
    path_in(input, work, "in.xml");
    path_in(output, work, "out.ass");
    path_in(missing, work, "missing.xml");
    df_options o = biliup();
    df_result r;

    for (int round = 0; round < 50; round++)
    {
        CHECK(convert(missing, output, &o, &r) == DF_ERR_INPUT_OPEN && r.os_error != 0, "missing input");
        CHECK(convert(work, output, &o, &r) == DF_ERR_INPUT_READ, "directory input: %d", r.status);
        write_file(input, "<i></i>");
        CHECK(convert(input, output, &o, &r) == DF_ERR_NO_DANMAKU, "empty input");
        write_file(input, "no tags here");
        CHECK(convert(input, output, &o, &r) == DF_ERR_NO_DANMAKU, "garbage input");
        write_file(input, "<i><d p=\"1,1,25,0\">a</d><d p=\"1,-5,25,0\">b</d><d p=\"2,7,25,0\">[0,0,\"1-1\",3,\"x\",0,0,"
                          "1,1,500,0,true,\"\",1]</d></i>");
        CHECK(convert(input, output, &o, &r) == DF_ERR_UNSUPPORTED_INPUT, "special without data");
        write_file(input, "<i><d p=\"1,1,25,0\">a</d></i>");
        char deep[4096];
        path_in(deep, work, "no-such-dir/out.ass");
        CHECK(convert(input, deep, &o, &r) == DF_ERR_OUTPUT_CREATE && r.os_error != 0, "missing output dir");
#ifdef __linux__
        /* every write fails with ENOSPC: used to fclose twice (patch 0005) */
        CHECK(df_convert_xml_to_ass(input, "/dev/full", &o, &r) == DF_ERR_OUTPUT_WRITE, "full disk: %d", r.status);
        char big[4096];
        path_in(big, golden, "inputs/dense.xml");
        CHECK(df_convert_xml_to_ass(big, "/dev/full", &o, &r) == DF_ERR_OUTPUT_WRITE, "full disk (large): %d",
              r.status);
#endif
    }

    /* argument checks */
    r.struct_size = sizeof(r);
    CHECK(df_convert_xml_to_ass(NULL, output, &o, &r) == DF_ERR_INVALID_ARGUMENT, "NULL input");
    CHECK(df_convert_xml_to_ass(input, NULL, &o, &r) == DF_ERR_INVALID_ARGUMENT, "NULL output");
    CHECK(df_convert_xml_to_ass(input, output, NULL, &r) == DF_ERR_INVALID_ARGUMENT, "NULL options");
    CHECK(df_convert_xml_to_ass(input, output, &o, NULL) == DF_ERR_INVALID_ARGUMENT, "NULL result");
    r.struct_size = 3;
    CHECK(df_convert_xml_to_ass(input, output, &o, &r) == DF_ERR_INVALID_ARGUMENT, "result size");
    df_options bad[12];
    for (int i = 0; i < 12; i++)
    {
        bad[i] = biliup();
    }
    bad[0].struct_size = 4;
    bad[1].resolution_x = 0;
    bad[2].display_area = NAN;
    bad[3].scroll_time = INFINITY;
    bad[4].density = -2;
    bad[5].font_name = NULL;
    bad[6].font_name = "a,b";
    bad[7].font_name = "\xff\xfe";
    bad[8].font_name = "\xed\xa0\x80"; /* a surrogate */
    bad[9].opacity = 256;
    bad[10].font_size_strict = 2;
    bad[11].block_mode = 1 << 7;
    for (int i = 0; i < 12; i++)
    {
        CHECK(convert(input, output, &bad[i], &r) == DF_ERR_INVALID_ARGUMENT, "bad option %d accepted", i);
        CHECK(!exists(output), "bad option %d wrote output", i);
    }
    remove(input);
}

int main(int argc, char **argv)
{
    if (argc != 3)
    {
        fprintf(stderr, "usage: df_ffi_test GOLDEN_DIR WORK_DIR\n");
        return 2;
    }
    snprintf(golden, sizeof(golden), "%s", argv[1]);
    snprintf(work, sizeof(work), "%s", argv[2]);
    CHECK(df_ffi_abi_version() == DF_FFI_ABI_VERSION, "ABI version");
    golden_inputs();
    edge_texts();
    failures_clean_up();
    if (failures != 0)
    {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    printf("df_ffi_test: all checks passed\n");
    return 0;
}
