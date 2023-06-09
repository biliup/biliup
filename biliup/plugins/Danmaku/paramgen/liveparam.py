import random
import time
from . import enc
from base64 import urlsafe_b64encode as b64enc
from urllib.parse import quote


def _header(video_id, channel_id) -> str:
    S1_3 = enc.rs(1, video_id)
    S1_5 = enc.rs(1, channel_id) + enc.rs(2, video_id)
    S1 = enc.rs(3, S1_3) + enc.rs(5, S1_5)
    S3 = enc.rs(48687757, enc.rs(1, video_id))
    header_replay = enc.rs(1, S1) + enc.rs(3, S3) + enc.nm(4, 1)
    return b64enc(header_replay)


def _build(video_id, channel_id, ts1, ts2, ts3, ts4, ts5, topchat_only) -> str:
    chattype = 4 if topchat_only else 1

    b1 = enc.nm(1, 0)
    b2 = enc.nm(2, 0)
    b3 = enc.nm(3, 0)
    b4 = enc.nm(4, 0)
    b7 = enc.rs(7, "")
    b8 = enc.nm(8, 0)
    b9 = enc.rs(9, "")
    timestamp2 = enc.nm(10, ts2)
    b11 = enc.nm(11, 3)
    b15 = enc.nm(15, 0)

    header = enc.rs(3, _header(video_id, channel_id))
    timestamp1 = enc.nm(5, ts1)
    s6 = enc.nm(6, 0)
    s7 = enc.nm(7, 0)
    s8 = enc.nm(8, 1)
    body = enc.rs(9, b"".join((b1, b2, b3, b4, b7, b8, b9, timestamp2, b11, b15)))
    timestamp3 = enc.nm(10, ts3)
    timestamp4 = enc.nm(11, ts4)
    s13 = enc.nm(13, chattype)
    chattype = enc.rs(16, enc.nm(1, chattype))
    s17 = enc.nm(17, 0)
    str19 = enc.rs(19, enc.nm(1, 0))
    timestamp5 = enc.nm(20, ts5)
    entity = b"".join(
        (
            header,
            timestamp1,
            s6,
            s7,
            s8,
            body,
            timestamp3,
            timestamp4,
            s13,
            chattype,
            s17,
            str19,
            timestamp5,
        )
    )
    continuation = enc.rs(119693434, entity)
    return quote(b64enc(continuation).decode())


def _times(past_sec):
    n = int(time.time())
    _ts1 = n - random.uniform(0, 1 * 3)
    _ts2 = n - random.uniform(0.01, 0.99)
    _ts3 = n - past_sec + random.uniform(0, 1)
    _ts4 = n - random.uniform(10 * 60, 60 * 60)
    _ts5 = n - random.uniform(0.01, 0.99)
    return list(map(lambda x: int(x * 1000000), [_ts1, _ts2, _ts3, _ts4, _ts5]))


def getparam(video_id, channel_id, past_sec=0, topchat_only=False) -> str:
    """
    Parameter
    ---------
    past_sec : int
        seconds to load past chat data
    topchat_only : bool
        if True, fetch only 'top chat'
    """
    return _build(video_id, channel_id, *_times(past_sec), topchat_only)
