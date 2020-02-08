#!/usr/bin/env python
# -*- coding: utf-8 -*-

'''
  A simple Javascript engines' wrapper.

  Description:
    This library wraps the system's built-in Javascript interpreter to python.
    It also support PyChakra, QuickJS, Node.js, etc.

    If your want use a special external Javascript interpreter, please call
    `set_external_interpreter` with binary's path after imported this module:
      
      from jsengine import *

      if set_external_interpreter(binary_path):  # setting OK
          ctx = ExternalJSEngine()

  Platform:
    macOS:   Use JavascriptCore
    Linux:   Use Gjs on Gnome, CJS on Cinnamon
    Windows: Use Chakra

    PyChakra and Node.js can run in all the above.

  Usage:

    from jsengine import JSEngine
    
    if JSEngine is None:  # always check this first!
      ......

    ctx = JSEngine()
    ctx.eval('1 + 1')  # => 2

    ctx2 = JSEngine("""
    function add(x, y) {
            return x + y;
        }
        """)
    ctx2.call("add", 1, 2)  # => 3

    ctx.append("""
        function square(x) {
            return x ** 2;
        }
        """)
    ctx.call("square", 9)  # => 81
'''

from __future__ import print_function
from subprocess import Popen, PIPE
import io
import json
import os
import platform
import sys
import tempfile

try:
    from shutil import which
except ImportError:
    from distutils.spawn import find_executable as which


### Before using this library, check JSEngine first!!!
__all__ = ['ProgramError', 'ChakraJSEngine', 'QuickJSEngine', 'ExternalJSEngine',
           'JSEngine', 'set_external_interpreter']



# Exceptions
class ProgramError(Exception):
    pass


### Detect javascript interpreters
chakra_available = False
quickjs_available = False
external_interpreter = None
external_interpreter_tempfile = False

# PyChakra
try:
    from PyChakra import Runtime as ChakraHandle, get_lib_path
    if not os.path.exists(get_lib_path()):
        raise RuntimeError
except (ImportError, RuntimeError):
    pass
else:
    chakra_available = True

# PyQuickJS
try:
    import quickjs
except ImportError:
    pass
else:
    quickjs_available = True

# macOS: built-in JavaScriptCore
if platform.system() == 'Darwin':
    external_interpreter = '/System/Library/Frameworks/JavaScriptCore.framework/Versions/A/Resources/jsc'

# Windows: built-in Chakra, Node.js if installed
elif platform.system() == 'Windows':
    if not chakra_available:
        try:
            from jsengine_chakra import ChakraHandle, chakra_available
        except ImportError:
            from .jsengine_chakra import ChakraHandle, chakra_available

    external_interpreter = which('node')

    if not chakra_available and not quickjs_available and external_interpreter is None:
        print('Please install PyChakra or Node.js!', file=sys.stderr)

# Linux: Gjs on Gnome, CJS on Cinnamon or JavaScriptCore/Node.js if installed
elif platform.system() == 'Linux':
    for interpreter in ('gjs', 'cjs', 'jsc', 'qjs', 'nodejs', 'node'):
        external_interpreter = which(interpreter)
        if external_interpreter:
            break

    if not chakra_available and not quickjs_available and external_interpreter is None:
        print('''\
Please install at least one of the following Javascript interpreter.'
python packages: PyChakra, quickjs
applications: Gjs, CJS, QuickJS, JavaScriptCore, Node.js.''', file=sys.stderr)

else:
    print('Sorry, the Javascript engine is currently not supported on your system.',
          file=sys.stderr)


# Inject to the script to let it return jsonlized value to python
# The additional code run only once, it does not require isolation processing
injected_script = u'''\
{source}
try {{
    var result = eval({data}), status = true;
}}
catch (err) {{
    var result = '' + err, status = false;
}}
try {{
    print('\\n' + JSON.stringify(["result", status, result]));
}}
catch (err) {{
    print('\\n["result", false, "Script returns a value with an unsupported type"]');
}}
'''


# Some simple compatibility processing
init_print_script = u'''\
if (typeof print === 'undefined' && typeof console === 'object') {
    print = console.log;
}
'''
init_global_script = u'''\
if (typeof global === 'undefined') {
    if (typeof Proxy === 'function') {
        global = new Proxy(this, {});
    } else {
        global = this;
    }
}
'''
init_del_gobject_script = u'''\
if (typeof {gobject} === 'object') {{
    {gobject} = undefined;
}}
'''
init_del_gobjects = ['exports']

end_split_char = set(u',;\\{}([')

if sys.version_info > (3,):
    unicode = str

def to_unicode(s):
    if not isinstance(s, unicode):
        s = s.decode('utf8')
    return s

def to_bytes(s):
    if isinstance(s, unicode):
        s = s.encode('utf8')
    return s

def json_encoder_fallback(o):
    # Allow bytes (python3)
    if isinstance(o, bytes):
        return to_unicode(o)
    return json.JSONEncoder.default(json_encoder, o)

json_encoder = json.JSONEncoder(
    skipkeys=True,
    ensure_ascii=False,
    check_circular=True,
    allow_nan=True,
    indent=None,
    separators=None,
    default=json_encoder_fallback,
)


class AbstractJSEngine:
    def __init__(self, source=u'', init_global=True, init_del_gobjects=init_del_gobjects):
        self._source = []
        init_script = [init_print_script]
        if init_global:
            init_script.append(init_global_script)
        if init_del_gobjects:
            for gobject in init_del_gobjects:
                if gobject == 'print' and hasattr(self, '_tempfile'):
                    continue
                init_script.append(init_del_gobject_script.format(gobject=gobject))
        init_script = u''.join(init_script)
        self.append(init_script)
        self.append(source)

    @property
    def source(self):
        '''All the inputted Javascript code.'''
        return self._get_source()

    def _append_source(self, code):
        if code:
            self._source.append(code)

    def _check_code(self, code):
        # Input unicode
        code = to_unicode(code)
        last_c = code.rstrip()[-1:]
        if last_c:
            # Simple end-split check
            if last_c not in end_split_char:
                code += u';'
            return code

    def append(self, code):
        '''Run Javascript code and return none.'''
        code = self._check_code(code)
        if code:
            self._append(code)

    def eval(self, code):
        '''Run Javascript code and return result.'''
        code = self._check_code(code)
        if code:
            return self._eval(code)

    def call(self, identifier, *args):
        '''Use name string and arguments to call Javascript function.'''
        chunks = json_encoder.iterencode(args, _one_shot=True)
        chunks = [to_unicode(chunk) for chunk in chunks]
        args = u''.join(chunks)[1:-1]
        code = u'{identifier}({args});'.format(identifier=identifier, args=args)
        return self._eval(code)

class InternalJSEngine(AbstractJSEngine):
    '''Wrappered for Internal(DLL) Javascript interpreter.'''

    def _get_source(self):
        return u'\n'.join(self._source)

    def _append(self, code):
        self._context.eval(code, eval=False, raw=True)

    def _eval(self, code):
        return self._context.eval(code)


class ChakraJSEngine(InternalJSEngine):
    '''Wrappered for system's built-in Chakra or PyChakra(ChakraCore).'''

    def __init__(self, *args, **kwargs):
        if not chakra_available:
            msg = 'No supported Chakra binary found on your system!'
            if quickjs_available:
                msg += ' Please install PyChakra or use QuickJSEngine.'
            elif external_interpreter:
                msg += ' Please install PyChakra or use ExternalJSEngine.'
            else:
                msg += ' Please install PyChakra.'
            raise RuntimeError(msg)
        self._context = self.Context(self)
        InternalJSEngine.__init__(self, *args, **kwargs)

    class Context:
        def __init__(self, engine):
            self._engine = engine
            self._context = ChakraHandle()

        def eval(self, code, eval=True, raw=False):
            self._engine._append_source(code)
            ok, result = self._context.eval(code, raw=raw)
            if ok:
                if eval:
                    return result
            else:
                raise ProgramError(str(result))


class QuickJSEngine(InternalJSEngine):
    '''Wrappered for QuickJS python binding quickjs.'''

    def __init__(self, *args, **kwargs):
        if not quickjs_available:
            msg = 'No supported QuickJS package found on custom python environment!'
            if chakra_available:
                msg += ' Please install python package quickjs or use ChakraJSEngine.'
            elif external_interpreter:
                msg += ' Please install python package quickjs or use ExternalJSEngine.'
            else:
                msg += ' Please install python package quickjs.'
            raise RuntimeError(msg)
        self._context = self.Context(self)
        InternalJSEngine.__init__(self, *args, **kwargs)

    class Context:
        def __init__(self, engine):
            self._engine = engine
            self._context = quickjs.Context()

        def eval(self, code, eval=True, raw=False):
            self._engine._append_source(code)
            try:
                result = self._context.eval(code)
            except quickjs.JSException as e:
                raise ProgramError(*e.args)
            else:
                if eval:
                    if raw or not isinstance(result, quickjs.Object):
                        return result
                    else:
                        return json.loads(result.json())


class ExternalJSEngine(AbstractJSEngine):
    '''Wrappered for external Javascript interpreter.'''

    def __init__(self, *args, **kwargs):
        if not external_interpreter:
            msg = 'No supported external Javascript interpreter found on your system!'
            if chakra_available:
                msg += ' Please install one or use ChakraJSEngine.'
            elif quickjs_available:
                msg += ' Please install one or use QuickJSEngine.'
            else:
                msg += ' Please install one.'
            raise RuntimeError(msg)
        self._last_code = u''
        self._tempfile = external_interpreter_tempfile
        AbstractJSEngine.__init__(self, *args, **kwargs)

    def _get_source(self, last_code=True):
        if last_code and self._last_code:
            source = self._source + [self._last_code]
        else:
            source = self._source
        return u'\n'.join(source)

    def _append(self, code):
        self._append_source(self._last_code)
        self._last_code = code

    def _eval(self, code):
        self._append(code)
        code = self._inject_script(code)
        if not self._tempfile:
            try:
                output = self._run_interpreter_with_pipe(code)
            except RuntimeError:
                self._tempfile = True
        if self._tempfile:
            output = self._run_interpreter_with_tempfile(code)

        output = output.replace(u'\r\n', u'\n').replace(u'\r', u'\n')
        # Search result in the last 5 lines of output
        for result_line in output.split(u'\n')[-5:]:
            if result_line[:9] == u'["result"':
                break
        _, ok, result = json.loads(result_line)
        if ok:
            return result
        else:
            raise ProgramError(result)

    def _run_interpreter(self, cmd, input=None):
        p = None
        stdin = PIPE if input else None
        try:
            p = Popen(cmd, stdin=stdin, stdout=PIPE, stderr=PIPE)
            stdout_data, stderr_data = p.communicate(input=input)
            retcode = p.wait()
        finally:
            del p
        if retcode != 0:
            raise RuntimeError('Javascript interpreter returns non-zero value! '
                               'Error msg: %s' % stderr_data.decode('utf8'))
        # Output unicode
        return stdout_data.decode('utf8')

    def _run_interpreter_with_pipe(self, code):
        cmd = [external_interpreter]
        # Input bytes
        code = to_bytes(code)
        return self._run_interpreter(cmd, input=code)

    def _run_interpreter_with_tempfile(self, code):
        fd, filename = tempfile.mkstemp(prefix='execjs', suffix='.js')
        os.close(fd)
        try:
            # Write bytes
            code = to_bytes(code)
            with io.open(filename, 'wb') as fp:
                fp.write(code)

            cmd = [external_interpreter, filename]
            return self._run_interpreter(cmd)
        finally:
            os.remove(filename)

    def _inject_script(self, code):
        source = self._get_source(last_code=False)
        data = json_encoder.encode(code)
        return injected_script.format(source=source, data=data)


def set_external_interpreter(interpreter):
    global external_interpreter
    external_interpreter = which(interpreter)
    res = external_interpreter is not None
    if res:
        _set_external_interpreter_tempfile()
    else:
        print("Can not find the given interpreter's path: %r" % interpreter, file=sys.stderr)
    return res


def _set_external_interpreter_tempfile():
    global external_interpreter_tempfile
    interpreter_name = os.path.basename(external_interpreter).split('.')[0]
    external_interpreter_tempfile = interpreter_name in ('qjs', 'qjsbn', 'd8')


if external_interpreter:
    _set_external_interpreter_tempfile()


# Prefer InternalJSEngine (via dynamic library loading)
if chakra_available:
    JSEngine = ChakraJSEngine
elif quickjs_available:
    JSEngine = QuickJSEngine
elif external_interpreter:
    JSEngine = ExternalJSEngine
else:
    JSEngine = None


if __name__ == '__main__':
    # run test
    import subprocess
    cmds = [sys.executable, 'jsengine_test.py']
    subprocess.Popen(cmds)
