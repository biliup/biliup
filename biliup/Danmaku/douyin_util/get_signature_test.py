DOUYIN_USER_AGENT = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.159 Safari/537.36'

def load_webmssdk(js_file: str):
    with open(js_file, 'r', encoding='utf-8') as f:
        return f.read()

def get_signature(X_MS_STUB: str):
    try:
        import jsengine
        ctx = jsengine.jsengine()
        js_dom = f"""
document = {{}}
window = {{}}
navigator = {{
'userAgent': '{DOUYIN_USER_AGENT}'
}}
""".strip()
        js_enc = load_webmssdk('biliup\Danmaku\douyin_util\webmssdk.js')
        final_js = js_dom + js_enc
        ctx.eval(final_js)
        function_caller = f"get_sign('{X_MS_STUB}')"
        signature = ctx.eval(function_caller)
        print("signature: ", signature)
        return signature
    except:
        raise
    return "00000000"

get_signature("69a78110dbe05a916c750237d701907e")