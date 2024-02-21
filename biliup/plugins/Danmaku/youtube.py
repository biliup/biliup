import json, re, select, random, traceback, urllib, datetime, base64
import asyncio, aiohttp
from biliup.plugins import random_user_agent

# The core codes for YouTube support are basically from taizan-hokuto/pytchat

headers = {
    'user-agent': random_user_agent(),
}


class Youtube:
    q = None
    url = ""
    vid = ""
    ctn = ""
    client = None
    stop = False

    @classmethod
    async def run(cls, url, q, client, **kargs):
        from .paramgen import liveparam

        cls.q = q
        cls.url = url
        cls.client = client
        cls.stop = False
        cls.key = "eW91dHViZWkvdjEvbGl2ZV9jaGF0L2dldF9saXZlX2NoYXQ/a2V5PUFJemFTeUFPX0ZKMlNscVU4UTRTVEVITEdDaWx3X1k5XzExcWNXOA=="
        await cls.get_url()
        while cls.stop == False:
            try:
                await cls.get_room_info()
                cls.ctn = liveparam.getparam(cls.vid, cls.cid, 1)
                await cls.get_chat()
            except:
                traceback.print_exc()
                await asyncio.sleep(1)

    @classmethod
    async def stop(cls):
        cls.stop = True

    @classmethod
    async def get_url(cls):
        a = re.search(r"youtube.com/channel/([^/?]+)", cls.url)
        try:
            cid = a.group(1)
            cls.cid = cid
            cls.url = f"https://www.youtube.com/channel/{cid}/videos"
        except:
            a = re.search(r"youtube.com/watch\?v=([^/?]+)", cls.url)
            async with cls.client.request(
                "get", f"https://www.youtube.com/embed/{a.group(1)}"
            ) as resp:
                b = re.search(r'\\"channelId\\":\\"(.{24})\\"', await resp.text())
                cls.cid = b.group(1)
                cls.url = f"https://www.youtube.com/channel/{cls.cid}/videos"

    @classmethod
    async def get_room_info(cls):
        async with cls.client.request("get", cls.url) as resp:
            t = re.search(
                r'"gridVideoRenderer"((.(?!"gridVideoRenderer"))(?!"style":"UPCOMING"))+"label":"(LIVE|LIVE NOW|PREMIERING NOW)"([\s\S](?!"style":"UPCOMING"))+?("gridVideoRenderer"|</script>)',
                await resp.text(),
            ).group(0)
            cls.vid = re.search(r'"gridVideoRenderer".+?"videoId":"(.+?)"', t).group(1)
            # print(cls.vid)

    @classmethod
    async def get_chat_single(cls):
        msgs = []
        data = {
            "context": {
                "client": {
                    "visitorData": "",
                    "userAgent": headers["user-agent"],
                    "clientName": "WEB",
                    "clientVersion": "".join(
                        (
                            "2.",
                            (datetime.datetime.today() - datetime.timedelta(days=1)).strftime(
                                "%Y%m%d"
                            ),
                            ".01.00",
                        )
                    ),
                },
            },
            "continuation": cls.ctn,
        }
        u = f'https://www.youtube.com/{base64.b64decode(cls.key).decode("utf-8")}'
        async with cls.client.request("post", u, headers=headers, json=data) as resp:
            # print(await resp.text())
            j = await resp.json()
            j = j["continuationContents"]
            cont = j["liveChatContinuation"]["continuations"][0]
            if cont is None:
                raise Exception("No Continuation")
            metadata = (
                cont.get("invalidationContinuationData")
                or cont.get("timedContinuationData")
                or cont.get("reloadContinuationData")
                or cont.get("liveChatReplayContinuationData")
            )
            cls.ctn = metadata["continuation"]
            # print(j['liveChatContinuation'].get('actions'))
            for action in j["liveChatContinuation"].get("actions", []):
                try:
                    renderer = action["addChatItemAction"]["item"]["liveChatTextMessageRenderer"]
                    msg = {}
                    msg["name"] = renderer["authorName"]["simpleText"]
                    message = ""
                    runs = renderer["message"].get("runs")
                    for r in runs:
                        if r.get("emoji"):
                            message += r["emoji"].get("shortcuts", [""])[0]
                        else:
                            message += r.get("text", "")
                    msg["content"] = message
                    msg["msg_type"] = "danmaku"
                    msg["color"] = "16777215"
                    msgs.append(msg)
                except:
                    pass

        return msgs

    @classmethod
    async def get_chat(cls):
        while cls.stop == False:
            ms = await cls.get_chat_single()
            if len(ms) != 0:
                interval = 1 / len(ms)
            else:
                await asyncio.sleep(1)
            for m in ms:
                await cls.q.put(m)
                await asyncio.sleep(interval)
