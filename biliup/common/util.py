import asyncio

import httpx

# Set up for non-mainland China networks
HTTP_TIMEOUT = 15

client = httpx.AsyncClient(http2=True, follow_redirects=True, timeout=HTTP_TIMEOUT)
loop = asyncio.get_running_loop()
