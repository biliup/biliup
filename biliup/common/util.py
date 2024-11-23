import asyncio

import httpx
from datetime import datetime, time, timezone, timedelta
from biliup.config import config
import logging


# This setup works very well on my Swedish machine, but who knows about others...
DEFAULT_TIMEOUT = httpx.Timeout(timeout=15.0, connect=10.0)
DEFAULT_MAX_RETRIES = 2
DEFAULT_CONNECTION_LIMITS = httpx.Limits(max_connections=100, max_keepalive_connections=100)

client = httpx.AsyncClient(http2=True, follow_redirects=True, timeout=DEFAULT_TIMEOUT, limits=DEFAULT_CONNECTION_LIMITS)
loop = asyncio.get_running_loop()
logger = logging.getLogger('biliup')


def check_timerange(name):
    time_range = config['streamers'].get(name, {}).get('time_range')
    now = datetime.now(tz=timezone(timedelta(hours=8))).time()
    logger.debug(f"{name}: 校验时间范围 {time_range} 当前时间 {now.strftime('%H:%M:%S')}")

    if not time_range or '-' not in time_range:
        return True

    try:
        start_time, end_time = map(time_string_to_time, time_range.split('-'))
    except (ValueError, IndexError) as e:
        logger.exception(f"Invalid time range format: {e}")
        return True

    if start_time > end_time:
        is_in_range = now >= start_time or now <= end_time
    else:
        is_in_range = start_time <= now <= end_time
    return is_in_range


def time_string_to_time(time_string):
    h, m, s = map(int, time_string.split(':'))
    return time(h, m, s)
