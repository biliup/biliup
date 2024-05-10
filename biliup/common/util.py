import asyncio

import httpx

client = httpx.AsyncClient(http2=True)
loop = asyncio.get_running_loop()
