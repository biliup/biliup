import json

import stream_gears
if __name__ == '__main__':
    res = stream_gears.get_qrcode()
    print(res)
    res = stream_gears.login_by_qrcode(res)
    with open(f'{json.loads(res)["token_info"]["mid"]}.json', 'w', encoding='utf-8') as file:
        file.write(res)
    print(res)
