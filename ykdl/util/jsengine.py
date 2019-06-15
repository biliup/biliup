#!/usr/bin/env python

'''
    Simple Javascript engines' wrapper
    
    Description:
        This library wraps the system's built-in Javascript interpreter to python.

    Platform:
        macOS:   Use JavascriptCore
        Linux:   Use gjs on Gnome, cjs on Cinnamon or NodeJS if installed
        Windows: Use Win10's built-in Chakra, if not available use NodeJS
    
    Usage:
    
        from jsengine import JSEngine, javascript_is_supported
        
        if not javascript_is_supported:  # always check this first!
            ......
        
        ctx = JSEngine()
        ctx.eval('1 + 1')  # => 2
    
        ctx2 = JSEngine("""
            function add(x, y) {
            return x + y;
        }
        """)
        ctx2.call("add", 1, 2)  # => 3
'''

from __future__ import print_function
from distutils.spawn import find_executable
from subprocess import Popen, PIPE
import io
import json
import os
import platform
import re
import sys
import tempfile

### Before using this library, check this variable first!!!
javascript_is_supported = True

# Exceptions
class ProgramError(Exception):
    pass

class RuntimeError(Exception):
    pass


use_chakra = False

# Choose javascript interpreter
# macOS: use built-in JavaScriptCore
if platform.system() == 'Darwin':
    interpreter = ['/System/Library/Frameworks/JavaScriptCore.framework/Versions/A/Resources/jsc']

# Windows: Try Chakra, if fails, load Node.js
elif platform.system() == 'Windows':
    from .jsengine_chakra import ChakraHandle, chakra_available
    if chakra_available:
        use_chakra = True
    elif find_executable('node') is not None:
        interpreter = ['node']
    else:
        print('Please install Node.js!', file=sys.stderr)
        javascript_is_supported = False

# Linux: use gjs on Gnome, cjs on Cinnamon or JavaScriptCore/NodeJS if installed
elif platform.system() == 'Linux':
    if find_executable('gjs') is not None:
        interpreter = ['gjs']
    elif find_executable('cjs') is not None:
        interpreter = ['cjs']
    elif find_executable('jsc') is not None:
        interpreter = ['jsc']
    elif find_executable('nodejs') is not None:
        interpreter = ['nodejs']
    elif find_executable('node') is not None:
        interpreter = ['node']
    else:
        print('Please install at least one of the following Javascript interpreter: gjs, cjs, nodejs', file=sys.stderr)
        javascript_is_supported = False
else:
    print('Sorry, the Javascript engine is currently not supported on your system', file=sys.stderr)
    javascript_is_supported = False



# Inject to the script to let it return jsonlized value to python
injected_script = r'''
var exports = undefined;
(function(program, execJS) { execJS(program) })(
function() {
    return eval(#{encoded_source});
},
function(program) {
    var print = (this.print === undefined) ? console.log : this.print;
    try {
        result = program();
        print("");
        if (typeof result == 'undefined' && result !== null) {
            print('["ok"]');
        }
        else {
            try {
                print(JSON.stringify(['ok', result]));
            }
            catch (err) {
                print('["err", "Script returns a value with an unknown type"]');
            }
        }
    }
    catch (err) {
        print(JSON.stringify(['err', '' + err]));
    }
});
'''



class AbstractJSEngine:
    def __init__(self, source=''):
        self._source = source
    
    def call(self, identifier, *args):
        args = json.dumps(args)
        code = '{identifier}.apply(this,{args})'.format(identifier=identifier, args=args)
        return self._eval(code)
        
    def eval(self, code=''):
        return self._eval(code)


class ChakraJSEngine(AbstractJSEngine):
    def __init__(self, source=''):
        AbstractJSEngine.__init__(self, source)
        self.chakra = ChakraHandle()
        if source:
            self.chakra.eval_js(source)
            
    def _eval(self, code):
        if not code.strip():
            return None
        data =  json.dumps(code, ensure_ascii=True)
        code = 'JSON.stringify([eval({data})]);'.format(data=data);
        ok, result = self.chakra.eval_js(code)
        if ok:
            return json.loads(result)[0]
        else:
            raise ProgramError(str(result))


class ExternalJSEngine(AbstractJSEngine):
    def __init__(self, source=''):
        AbstractJSEngine.__init__(self, source)
        self._last_code = ''

    def _eval(self, code):
        # TODO: may need a thread lock, if running multithreading
        if not code.strip():
            return None
        if self._last_code:
            self._source += '\n' + self._last_code
        self._last_code = code
        data = json.dumps(code, ensure_ascii=True)
        code = 'return eval({data});'.format(data=data)
        return self._exec(code)
    
    def _exec(self, code):
        if self._source:
            code = self._source + '\n' + code
        code = self._inject_script(code)
        output = self._run_interpreter_with_tempfile(code)
        output = output.replace('\r\n', '\n').replace('\r', '\n')
        last_line = output.split('\n')[-2]
        ret = json.loads(last_line)
        if len(ret) == 1:
            return None
        status, value = ret
        if status == 'ok':
            return value
        else:
            raise ProgramError(value)
        
    def _run_interpreter_with_tempfile(self, code):
        (fd, filename) = tempfile.mkstemp(prefix='execjs', suffix='.js')
        os.close(fd)
        try:
            # decoding in python2
            if hasattr(code, 'decode'):
                code = code.decode('utf8')
            with io.open(filename, 'w', encoding='utf8') as fp:
                fp.write(code)
            
            cmd = interpreter + [filename]
            p = None
            try:
                p = Popen(cmd, stdout=PIPE, universal_newlines=True)
                stdoutdata, stderrdata = p.communicate()
                ret = p.wait()
            finally:
                del p
            if ret != 0:
                raise RuntimeError('Javascript interpreter returns non-zero value!')
            return stdoutdata
        finally:
            os.remove(filename)

    def _inject_script(self, source):
        encoded_source = \
            '(function(){ ' + \
            self._encode_unicode_codepoints(source) + \
            ' })()'
        return injected_script.replace('#{encoded_source}', json.dumps(encoded_source))

    def _encode_unicode_codepoints(self, str):
        codepoint_format = '\\u{0:04x}'.format
        def codepoint(m):
            return codepoint_format(ord(m.group(0)))
        return re.sub('[^\x00-\x7f]', codepoint, str)


if use_chakra:
    class JSEngine(ChakraJSEngine):
        pass
else:
    class JSEngine(ExternalJSEngine):
        pass
