"""扫码登录，把凭据写到 `<mid>.json`（或第一个参数指定的路径）。

用法：python login.py [输出文件]
"""
import json
import sys

import stream_gears
if __name__ == '__main__':
    res = stream_gears.get_qrcode(proxy=None)
    print(res)
    res = stream_gears.login_by_qrcode(res, proxy=None)
    out = sys.argv[1] if len(sys.argv) > 1 else f'{json.loads(res)["token_info"]["mid"]}.json'
    with open(out, 'w', encoding='utf-8') as file:
        file.write(res)
    print(res)
