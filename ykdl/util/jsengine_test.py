#!/usr/bin/env python
#-*- coding: UTF-8 -*-

import unittest
import platform
import jsengine
from jsengine import *


print_source_code = False

def skip(func):
    def newfunc(*args, **kwargs):
        global ctx
        if ctx:
            try:
                func(*args, **kwargs)
            except:
                ctx = JSEngine()
                raise
        else:
            raise unittest.SkipTest('init failed')
    return newfunc

class JSEngineES6Tests(unittest.TestCase):

    expected_exceptions = ProgramError, RuntimeError

    def test_00_init(self):
        global ctx
        ctx = JSEngine()

    @skip
    def test_01_keyword_let(self):
        _ctx = JSEngine()
        with self.assertRaises(self.expected_exceptions):
            _ctx.eval('''
            let vlet;
            let vlet;''')
        _ctx = JSEngine()
        with self.assertRaises(self.expected_exceptions):
            _ctx.eval('''
            var vlet = 1;
            let vlet;''')
        self.assertEqual(ctx.eval('''
        var vlet = 1;
        function flet() {
            let vlet = 2;
        }
        flet(), vlet'''), 1)

    @skip
    def test_02_keyword_const(self):
        _ctx = JSEngine()
        with self.assertRaises(self.expected_exceptions):
            _ctx.eval('const vconst')
        _ctx = JSEngine()
        with self.assertRaises(self.expected_exceptions):
            _ctx.eval('''
            const vconst = 1;
            const vconst = 1''')
        _ctx = JSEngine()
        with self.assertRaises(self.expected_exceptions):
            _ctx.eval('''
            const vconst = 1;
            vconst = 1;''')
        _ctx = JSEngine()
        with self.assertRaises(self.expected_exceptions):
            _ctx.eval('''
            var vconst;
            const vconst = 1;''')
        self.assertEqual(ctx.eval('''
        var vconst = 1;
        function fconst() {
            const vconst = 2;
        }
        fconst(), vconst'''), 1)

    @skip
    def test_03_keyword_for_of(self):
        ctx.eval('for (n of []) {}')

    @skip
    def test_04_operator_power(self):
        self.assertEqual(ctx.eval('3 ** 3'), 27)

    @skip
    def test_05_operator_spread_rest(self):
        self.assertEqual(ctx.eval('''
        function foo(first, ...args) {return [first, args]}
        foo(...[1, 2, 3])'''), [1, [2, 3]])

    @skip
    def test_06_function_arrow(self):
        self.assertEqual(ctx.eval('(arg => arg)(1)'), 1)
        _ctx = JSEngine()
        with self.assertRaises(self.expected_exceptions):
            _ctx.eval('''
            foo = arg => arg;
            obj = new foo()''')

    @skip
    def test_07_function_default_arguments(self):
        ctx.eval('function foo(agr = "default") {}')

    @skip
    def test_08_class_super(self):
        ctx.eval('''
        class C1 {
            constructor() {this.id = 1;}
            method() {return 2;}
        }
        var c1 = new C1()''')
        ctx.append('''
        class C2 extends C1 {
            constructor() {
                super()
                this.id += super.method();}
        }
        ''')
        self.assertEqual(ctx.eval('new C2().id'), 3)
        _ctx = JSEngine()
        with self.assertRaises(self.expected_exceptions):
            _ctx.eval('''
            var c3 = new C3();
            class C3 {}''')

    @skip
    def test_09_type_symbol(self):
        ctx.eval('''
        var sym1 = Symbol('foo');
        var sym2 = Symbol('foo')''')
        self.assertFalse(ctx.eval('sym1 === sym2'))
        _ctx = JSEngine()
        with self.assertRaises(self.expected_exceptions):
            _ctx.eval('new Symbol()')

    @skip
    def test_10_type_set_map(self):
        ctx.eval('''
        new Set();
        new Map();''')

    @skip
    def test_11_type_typedarray(self):
        ctx.eval('''
        new Int8Array(2);
        new Uint8Array(2);
        new Uint8ClampedArray(2);
        new Int16Array(2);
        new Uint16Array(2);
        new Int32Array(2);
        new Uint32Array(2);
        new Float32Array(2);
        new Float64Array(2);''')

    @skip
    def test_12_deconstruction(self):
        ctx.eval('''
        var arr = [1, 2, 3];
        var [n1, n2, n3] = arr;''')
        ctx.eval('''
        var obj = {id1: 1, id2: 2, id3: 3}
        var {id1, id2, id3} = obj;''')
        ctx.eval('''
        var [n1, n2, n3, n4] = arr;
        var n1 = undefined;
        var [n1] = arr;''')
        ctx.eval('''
        var {id1, id2, id3, id4} = obj;
        var id1 = undefined;
        var {id1} = obj;''')
        self.assertEqual(ctx.eval('n1'), 1)
        self.assertEqual(ctx.eval('n4'), None)
        self.assertEqual(ctx.eval('id1'), 1)
        self.assertEqual(ctx.eval('id4'), None)

    @skip
    def test_13_string_template(self):
        self.assertEqual(ctx.eval('''
`123
456`'''), '123\n456')
        self.assertEqual(ctx.eval('`123456${3 + 4}`'), '1234567')

    @skip
    def test_14_literal(self):
        self.assertEqual(ctx.eval('0b11'), 3)
        self.assertEqual(ctx.eval('0B11'), 3)
        self.assertEqual(ctx.eval('0o11'), 9)
        self.assertEqual(ctx.eval('0O11'), 9)

    @skip
    def test_98_engine_in_out_string(self):
        ss = 'αβγ'        
        rs = '"αβγ"'
        us = jsengine.to_unicode(ss)
        self.assertEqual(ctx.eval(rs), us)
        self.assertEqual(ctx.eval(jsengine.to_unicode(rs)), us)
        self.assertEqual(ctx.eval(jsengine.to_bytes(rs)), us)
        ctx.append('''
        function ping(s1, s2, s3) {
            return [s1, s2, s3]
        }''')
        # Mixed string types input
        self.assertEqual(ctx.call('ping',
                ss, jsengine.to_unicode(ss), jsengine.to_bytes(ss)), [us] * 3)

    @skip
    def test_99_engine_get_source(self):
        if print_source_code:
            print('\nSOURCE CODE:')
            print(ctx.source)
            print('SOURCE CODE END\n')
        else:
            ctx.source


def test_engine(engine):
    global JSEngine, ctx
    print('\nStart test %s' % engine.__name__)
    if engine is ExternalJSEngine:
        print('Used external interpreter: %r' % jsengine.external_interpreter)
    JSEngine = engine
    ctx = None
    unittest.TestProgram(exit=False)
    print('End test %s\n' % engine.__name__)
    
def test_main(external_interpreters):
    print('Default JSEngine is %r' % jsengine.JSEngine)
    print('Default external_interpreter is %r' % jsengine.external_interpreter)

    for JSEngine in (ChakraJSEngine, QuickJSEngine):
        test_engine(JSEngine)

    for external_interpreter in external_interpreters:
        if set_external_interpreter(external_interpreter):
            test_engine(ExternalJSEngine)

    if platform.system() == 'Windows':
        import msvcrt
        print('Press any key to continue ...')
        msvcrt.getch()

default_external_interpreters = [
    # test passed
    'gjs',          # Gjs
    'cjs',          # CJS
    'jsc',          # JavaScriptCore
    'qjs',          # QuickJS
    'node',         # Node.js
    'nodejs',       # Node.js
    'spidermonkey', # SpiderMonkey
    'chakra',       # ChakraCore

    # test passed, but unceremonious names
    #'d8',          # V8
    #'js',          # SpiderMonkey
    #'ch',          # ChakraCore

    # test failed
    'hermes',       # Hermes
    'duk',          # Duktape
]


if __name__ == '__main__':
    import sys
    try:
        sys.argv.remove('-psc')
    except ValueError:
        pass
    else:
        print_source_code = True
    external_interpreters = sys.argv[1:] or default_external_interpreters
    del sys.argv[1:] # clear arguments
    test_main(external_interpreters)
