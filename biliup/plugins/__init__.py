import logging
import re
import hashlib
import time
import json
from urllib.parse import urlencode, quote
from typing import (
    Any,
    Callable,
    Dict,
    List,
    Optional,
    Tuple,
    Union,
)

logger = logging.getLogger('biliup')


def match1(text, *patterns):
    if len(patterns) == 1:
        pattern = patterns[0]
        match = re.search(pattern, text)
        if match:
            return match.group(1)
        else:
            return None
    else:
        ret = []
        for pattern in patterns:
            match = re.search(pattern, text)
            if match:
                ret.append(match.group(1))
        return ret


def random_user_agent(device: str = 'desktop') -> str:
    import random
    chrome_version = random.randint(100, 120)
    if device == 'mobile':
        android_version = random.randint(9, 14)
        mobile = random.choice([
            'SM-G981B', 'SM-G9910', 'SM-S9080', 'SM-S9110', 'SM-S921B',
            'Pixel 5', 'Pixel 6', 'Pixel 7', 'Pixel 7 Pro', 'Pixel 8',
        ])
        return f'Mozilla/5.0 (Linux; Android {android_version}; {mobile}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{chrome_version}.0.0.0 Mobile Safari/537.36'
    return f'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{chrome_version}.0.0.0 Safari/537.36'


def json_loads(text: Union[str, None]) -> Dict[str, Any]:
    if not text:
        raise ValueError("Invalid JSON: None")
    try:
        return json.loads(text)
    except json.JSONDecodeError as e:
        raise ValueError(f"Invalid JSON: {text}") from e


class Wbi:
    WTS = "wts"
    W_RID = "w_rid"
    UPDATE_INTERVAL = 2 * 60 * 60

    KEY_MAP = [
        46, 47, 18, 2,  53, 8,  23, 32, 15, 50, 10, 31, 58, 3,  45, 35,
        27, 43, 5,  49, 33, 9,  42, 19, 29, 28, 14, 39, 12, 38, 41, 13,
        37, 48, 7,  16, 24, 55, 40, 61, 26, 17, 0,  1,  60, 51, 30, 4,
        22, 25, 54, 21, 56, 59, 6,  63, 57, 62, 11, 36, 20, 34, 44, 52,
    ]

    def __init__(self):
        self.key = None
        self.last_update = 0

    def update_key(self, img, sub):
        """
        更新 key，基于 img 和 sub 的组合。
        """
        KEY_LENGTH = 32
        full = img + sub
        key = [full[self.KEY_MAP[i]] for i in range(KEY_LENGTH)]
        self.key = ''.join(key)
        self.last_update = int(time.time())
        logger.info(f"Updated wbi key successfully")

    def sign(self, query: dict, ts: int = None):
        """
        生成签名。
        :param query: 请求参数
        :param ts: 时间戳（可选），默认为当前时间戳。
        """
        if self.key is None:
            raise ValueError("Key is not set.")

        ts = ts or int(time.time())
        ts_str = str(ts)

        sanitized_query = {
            k: ''.join(c for c in v if c not in "!\'()*")
            for k, v in query.items()
        }
        sanitized_query[self.WTS] = ts_str
        sorted_query = dict(sorted(sanitized_query.items()))
        content_string = urlencode(sorted_query, quote_via=quote)

        md5 = hashlib.md5()
        md5.update((content_string + self.key).encode('utf-8'))
        sign = md5.hexdigest()

        query[self.W_RID] = sign
        query[self.WTS] = ts_str

wbi = Wbi()