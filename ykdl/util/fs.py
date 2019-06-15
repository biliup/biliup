#!/usr/bin/env python
# -*- coding: utf-8 -*-

import platform

def legitimize(text, os=platform.system()):
    """Converts a string to a valid filename.
    """

    # POSIX systems
    text = text.translate({
        0: None,
        ord('/'): u'-',
    })

    if os == 'Windows':
        # Windows (non-POSIX namespace)
        text = text.translate({
            # Reserved in Windows VFAT and NTFS
            ord(':'): u'-',
            ord('*'): u'-',
            ord('?'): u'-',
            ord('\\'): u'-',
            ord('|'): u'-',
            ord('\"'): u'\'',
            ord('\n'): u'_',
            # Reserved in Windows VFAT
            ord('+'): u'-',
            ord('<'): u'-',
            ord('>'): u'-',
            ord('['): u'(',
            ord(']'): u')',
        })
    else:
        # *nix
        if os == 'Darwin':
            # Mac OS HFS+
            text = text.translate({
                ord(':'): u'-',
            })

        # Remove leading .
        if text.startswith("."):
            text = text[1:]

    text = text[:82] # Trim to 82 Unicode characters long
    return text
