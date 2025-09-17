import asyncio

import stream_gears

async def main():
    # from biliup.common.util import client, loop
    # print(loop)
    config = stream_gears.config_bindings()
    print(config)
    print(config.get("twitcasting_password", "313213"))
    print(config.get("file_size"))
    print(config.get("file_size"))
    print(config.get("streamers"))
    # await loop.run_in_executor(None, stream_gears.main_loop)
    # stream_gears.main_loop()


if __name__ == '__main__':
    asyncio.run(main())