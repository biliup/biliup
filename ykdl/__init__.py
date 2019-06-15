#!/usr/bin/env python
# -*- coding: utf-8 -*

import sys

if sys.platform.startswith('win'):
    # hack sys.stdout in Windows cmd shell
    if sys.version_info[0] < 3:
        class filewrapper(object):
            def __init__(self, file, encoding=None, errors='ignore'):
                object.__setattr__(self, 'wrappedfile', file)
                object.__setattr__(self, 'encoding', encoding or file.encoding)
                object.__setattr__(self, 'errors', errors)

            def __getattr__(self, name):
                return getattr(self.wrappedfile, name)

            def __setattr__(self, name, value):
                setattr(self.wrappedfile, name, value)

            def write(self, s):
                if isinstance(s, unicode):
                    s = s.encode(encoding=self.encoding, errors=self.errors)
                self.wrappedfile.write(s)

        sys.stdout = filewrapper(sys.stdout)

    elif sys.version_info[1] < 6:
        import io
        sys.stdout = io.TextIOWrapper(sys.stdout.detach(),
                                      encoding=sys.stdout.encoding,
                                      errors='ignore',
                                      line_buffering=True)


from .util.log import ColorHandler
import logging

if sys.version_info[0] < 3:
    logging.root.addHandler(ColorHandler())
else:
    logging.basicConfig(handlers=[ColorHandler()])
