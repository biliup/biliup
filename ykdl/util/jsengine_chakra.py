# -*- coding: utf-8 -*-


"""
This is a Python binding to Microsoft Chakra Javascript engine.
Modified from PyChakra (https://github.com/zhengrenzhe/PyChakra) to
support Win10's built-in Chakra.
"""


import ctypes as _ctypes


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

        self.__runtime = runtime
        self.__context = context
        self.__chakra = chakra

    def __del__(self):
        self.__chakra.JsDisposeRuntime(self.__runtime)

    def eval_js(self, script, source=u""):
        """
            Eval javascript string

            Examples:
                .eval_js("(()=>2)()") // (True, '2')
                .eval_js("(()=>a)()") // (False, "'a' is not defined")

            Parameters:
                script(str): javascript code string
                source(str?): code path (optional)

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
        chakra = self.__chakra
        chakra.JsSetCurrentContext(self.__context)
        # make sure they are unicode string
        if hasattr(script, 'decode'):
            script = script.decode('utf8')
        if hasattr(source, 'decode'):
            source = source.decode('utf8')
        script = _ctypes.c_wchar_p(script)
        source = _ctypes.c_wchar_p(source)
        result = _ctypes.c_void_p()
        err = chakra.JsRunScript(script, 0, source, point(result))

        # no error
        if err == 0:
            return (True, self.__js_value_to_str(result))

        # js exception
        elif err == 196609:
            return (False, self.__get_exception())

        # other error
        else:
            return (False, err)

    def __get_exception(self):
        exception = _ctypes.c_void_p()
        self.__chakra.JsGetAndClearException(point(exception))

        id = _ctypes.c_void_p()
        id_str = "message"
        self.__chakra.JsGetPropertyIdFromName(id_str, point(id))

        value = _ctypes.c_void_p()
        self.__chakra.JsGetProperty(exception, id, point(value))

        return self.__js_value_to_str(value)

    def __js_value_to_str(self, js_value):
        js_value_ref = _ctypes.c_void_p()
        self.__chakra.JsConvertValueToString(js_value, point(js_value_ref))

        str_p = _ctypes.c_wchar_p()
        str_len = _ctypes.c_size_t()
        self.__chakra.JsStringToPointer(js_value_ref, point(str_p), point(str_len))
        return str_p.value


def point(any):
    return _ctypes.byref(any)

