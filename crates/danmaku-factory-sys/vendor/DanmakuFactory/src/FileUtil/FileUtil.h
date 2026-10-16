#ifndef __FILE_UTIL_H__
#define __FILE_UTIL_H__

#include <stdio.h>

#ifdef __cplusplus
extern "C"
{
#endif

    FILE *utf8_fopen(const char *filename, const char *mode);
    int utf8_access(const char *path, int mode);

#ifdef _WIN32
    void GetCommandLineUTF8(int *argc, char ***argv);
    void FreeCommandLineUTF8(int argc, char **argv);
#endif

#ifdef __cplusplus
}
#endif

#endif
