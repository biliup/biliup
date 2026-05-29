#!/usr/bin/python3
# coding:utf8
import asyncio
import logging
import logging.config
import shutil

import stream_gears

from biliup import LOG_CONF


class DebugLevelFilter(logging.Filter):
    def filter(self, record):
        return logging.getLogger().isEnabledFor(logging.DEBUG)


def arg_parser():
    logging.config.dictConfig(LOG_CONF)
    logging.getLogger('httpx').addFilter(DebugLevelFilter())

    asyncio.run(main())


async def main():
    # 启动时删除临时文件夹
    shutil.rmtree('./cache/temp', ignore_errors=True)
    loop = asyncio.get_running_loop()
    await loop.run_in_executor(None, stream_gears.main_loop)


if __name__ == '__main__':
    arg_parser()
