import json

import aiohttp

from biliup.plugins import random_user_agent


class Twitcasting:
    heartbeat = None
    fake_headers = {
        "Accept": "*/*",
        "Accept-Encoding": "gzip, deflate, br",
        "Cache-Control": "no-cache",
        "Pragma": "no-cache",
        "Referer": "https://twitcasting.tv/",
        "User-Agent": random_user_agent()
    }

    @staticmethod
    async def get_ws_info(url, context):
        async with aiohttp.ClientSession(headers=Twitcasting.fake_headers) as session:
            async with session.post(
                    url="https://twitcasting.tv/eventpubsuburl.php",
                    data={
                        'movie_id': context['movie_id'],
                        'password': context['password']
                    },
                    timeout=5
            ) as resp:
                r_obj = await resp.json()
                url = r_obj['url']
                return url, []

    @staticmethod
    def decode_msg(data):
        msgs = []
        if data is not None:
            for d in data.splitlines():
                if len(d) == 0:
                    continue
                try:
                    d = json.loads(d)[0]
                    msg = {
                        "content": d['message'],
                        "msg_type": "danmaku"
                    }
                    msgs.append(msg)
                except:
                    pass
        return msgs
