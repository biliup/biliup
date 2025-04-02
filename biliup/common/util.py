import asyncio
import json

import httpx
from datetime import datetime, timezone
from biliup.config import config
import logging

try:
    import ssl
    import truststore # type: ignore
except ImportError:
    ssl = None
    truststore = None
    _ssl_context = True
else:
    _ssl_context = truststore.SSLContext(ssl.PROTOCOL_TLS_CLIENT)

# This setup works very well on my Swedish machine, but who knows about others...
DEFAULT_TIMEOUT = httpx.Timeout(timeout=15.0, connect=10.0)
DEFAULT_MAX_RETRIES = 2
DEFAULT_CONNECTION_LIMITS = httpx.Limits(max_connections=100, max_keepalive_connections=100)

client = httpx.AsyncClient(
    http2=True,
    follow_redirects=True,
    timeout=DEFAULT_TIMEOUT,
    limits=DEFAULT_CONNECTION_LIMITS,
    verify=_ssl_context
)
loop = asyncio.get_running_loop()
logger = logging.getLogger('biliup')


def check_timerange(name):
    try:
        time_range_str = config['streamers'].get(name, {}).get('time_range')
        if not time_range_str:
            return True
        time_range = json.loads(time_range_str)
        if not isinstance(time_range, (list, tuple)) or len(time_range) != 2:
            return True

        start = datetime.fromisoformat(time_range[0].replace('Z', '+00:00')).time()
        end   = datetime.fromisoformat(time_range[1].replace('Z', '+00:00')).time()
    except Exception as e:
        logger.error(f'parsing time range {e}')
        return True

    now = datetime.now(timezone.utc).time()

    # Normal interval (e.g. 16:00 → 20:00)
    if start <= end:
        return start <= now <= end

    # Cross‑midnight (e.g. 23:00 → 04:00)
    return now >= start or now <= end
