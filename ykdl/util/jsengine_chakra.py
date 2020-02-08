# -*- coding: utf-8 -*-


"""
This is a Python binding to Microsoft Chakra Javascript engine.
Modified from PyChakra (https://github.com/zhengrenzhe/PyChakra) to
support Win10's built-in Chakra.
"""


import ctypes as _ctypes
import json


# load win10's built-in chakra binary
try:
    chakra = _ctypes.windll.Chakra
    chakra_available = True
except:
    chakra_available = False


class ChakraHandle():

    def __init__(self):
        # create chakra runtime and context
        runtime = _ctypes.c_void_p()
        chakra.JsCreateRuntime(0, 0, point(runtime))

        context = _ctypes.c_void_p()
        chakra.JsCreateContext(runtime, point(context))
        chakra.JsSetCurrentContext(context)

        self.__runtime = runtime
        self.__context = context
        self.__chakra = chakra

        # get JSON.stringify reference, and create its called arguments array
        stringify = self.eval("JSON.stringify;", raw=True)[1]
        undefined = _ctypes.c_void_p()
        chakra.JsGetUndefinedValue(point(undefined))
        args = (_ctypes.c_void_p * 2)()
        args[0] = undefined

        self.__jsonStringify = stringify
        self.__jsonStringifyArgs = args

    def __del__(self):
        self.__chakra.JsDisposeRuntime(self.__runtime)

    def eval(self, script, raw=False):
        """\
        Eval javascript string

        Examples:
            .eval("(()=>2)()") // (True, 2)
            .eval("(()=>a)()") // (False, "ReferenceError: 'a' is not defined")

        Parameters:
            script(str): javascript code string
            raw(bool?): whether return result as chakra JsValueRef directly
                        (optional, default is False)

        Returns:
            (bool, result)
            bool: indicates whether javascript is running successfully.
            result: if bool is True, result is the javascript running
                        return value.
                    if bool is False and result is string, result is the
                        javascript running exception
                    if bool is False and result is number, result is the
                        chakra internal error code
        """

        # TODO: may need a thread lock, if running multithreading
        self.__chakra.JsSetCurrentContext(self.__context)

        js_source = _ctypes.c_wchar_p("")
        js_script = _ctypes.c_wchar_p(script)

        result = _ctypes.c_void_p()
        err = self.__chakra.JsRunScript(js_script, 0, js_source, point(result))

        # no error
        if err == 0:
            if raw:
                return True, result
            else:
                return self.__js_value_to_py_value(result)

        return self.__get_error(err)

    def __js_value_to_py_value(self, js_value):
        args = self.__jsonStringifyArgs
        args[1] = js_value

        # value => json
        result = _ctypes.c_void_p()
        err = self.__chakra.JsCallFunction(
            self.__jsonStringify, point(args), 2, point(result))

        if err == 0:
            result = self.__js_value_to_str(result)
            if result == "undefined":
                result = None
            else:
                # json => value
                result = json.loads(result)
            return True, result

        return self.__get_error(err)

    def __get_error(self, err):
        # js exception or other error
        if err == 196609:
            err = self.__get_exception()
        return False, err

    def __get_exception(self):
        exception = _ctypes.c_void_p()
        self.__chakra.JsGetAndClearException(point(exception))
        return self.__js_value_to_str(exception)

    def __js_value_to_str(self, js_value):
        js_value_ref = _ctypes.c_void_p()
        self.__chakra.JsConvertValueToString(js_value, point(js_value_ref))

        str_p = _ctypes.c_wchar_p()
        str_l = _ctypes.c_size_t()
        self.__chakra.JsStringToPointer(js_value_ref, point(str_p), point(str_l))
        return str_p.value


def point(any):
    return _ctypes.byref(any)

