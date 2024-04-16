import json, re, select, random, traceback, urllib, datetime, base64
import asyncio, aiohttp
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
    async def get_ws_info(url):
        from biliup.plugins.twitcasting import TwitcastingUtils
        async with aiohttp.ClientSession() as session:
            async with session.get(url, headers=Twitcasting.fake_headers) as response:
                html_text = await response.text()
            broadcasterInfo = TwitcastingUtils._getBroadcaster(html_text)
            if broadcasterInfo['MovieID']:
                data = aiohttp.FormData()
                data.add_field('movie_id', broadcasterInfo['MovieID'])
                async with session.post(
                    url="https://twitcasting.tv/eventpubsuburl.php",
                    data=data,
                    headers=Twitcasting.fake_headers,
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