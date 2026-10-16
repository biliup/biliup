#include "FileUtil.h"
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
// clang-format off
#include <windows.h>
#include <shellapi.h>
// clang-format on
#include <io.h>

FILE *utf8_fopen(const char *filename, const char *mode)
{
    FILE *f = NULL;
    int fn_len, m_len;
    wchar_t *wfilename, *wmode;

    if (!filename || !mode)
        return NULL;

    fn_len = MultiByteToWideChar(CP_UTF8, 0, filename, -1, NULL, 0);
    m_len = MultiByteToWideChar(CP_UTF8, 0, mode, -1, NULL, 0);

    if (fn_len == 0 || m_len == 0)
        return NULL;

    wfilename = (wchar_t *)malloc(fn_len * sizeof(wchar_t));
    wmode = (wchar_t *)malloc(m_len * sizeof(wchar_t));

    if (!wfilename || !wmode)
    {
        if (wfilename)
            free(wfilename);
        if (wmode)
            free(wmode);
        return NULL;
    }

    MultiByteToWideChar(CP_UTF8, 0, filename, -1, wfilename, fn_len);
    MultiByteToWideChar(CP_UTF8, 0, mode, -1, wmode, m_len);

    f = _wfopen(wfilename, wmode);

    free(wfilename);
    free(wmode);

    return f;
}

int utf8_access(const char *path, int mode)
{
    int ret = -1;
    int len;
    wchar_t *wpath;

    if (!path)
        return -1;

    len = MultiByteToWideChar(CP_UTF8, 0, path, -1, NULL, 0);
    if (len == 0)
        return -1;

    wpath = (wchar_t *)malloc(len * sizeof(wchar_t));
    if (!wpath)
        return -1;

    MultiByteToWideChar(CP_UTF8, 0, path, -1, wpath, len);

    ret = _waccess(wpath, mode);

    free(wpath);
    return ret;
}

void GetCommandLineUTF8(int *argc, char ***argv)
{
    int nArgs;
    LPWSTR *szArglist = CommandLineToArgvW(GetCommandLineW(), &nArgs);
    if (NULL == szArglist)
    {
        return;
    }

    char **utf8_argv = (char **)malloc(sizeof(char *) * nArgs);
    if (!utf8_argv)
    {
        LocalFree(szArglist);
        return;
    }

    for (int i = 0; i < nArgs; i++)
    {
        int len = WideCharToMultiByte(CP_UTF8, 0, szArglist[i], -1, NULL, 0, NULL, NULL);
        utf8_argv[i] = (char *)malloc(len);
        if (utf8_argv[i])
        {
            WideCharToMultiByte(CP_UTF8, 0, szArglist[i], -1, utf8_argv[i], len, NULL, NULL);
        }
    }

    LocalFree(szArglist);
    *argc = nArgs;
    *argv = utf8_argv;
}

void FreeCommandLineUTF8(int argc, char **argv)
{
    if (argv)
    {
        for (int i = 0; i < argc; i++)
        {
            free(argv[i]);
        }
        free(argv);
    }
}

#else

#include <unistd.h>

FILE *utf8_fopen(const char *filename, const char *mode)
{
    return fopen(filename, mode);
}

int utf8_access(const char *path, int mode)
{
    return access(path, mode);
}

#endif
