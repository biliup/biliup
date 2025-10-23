#!/usr/bin/python3
# coding:utf8
import argparse
import asyncio
import logging.config
import shutil

import stream_gears

import biliup.common.reload
# from biliup.config import config
from biliup import __version__, IS_FROZEN, LOG_CONF
from biliup.common.Daemon import Daemon
from biliup.common.log import DebugLevelFilter


def arg_parser():
    logging.config.dictConfig(LOG_CONF)
    logging.getLogger('httpx').addFilter(DebugLevelFilter())

    asyncio.run(main())


async def main():
    # from biliup.app import event_manager

    # event_manager.start()

    # 启动时删除临时文件夹
    shutil.rmtree('./cache/temp', ignore_errors=True)
    from biliup.common.util import loop

    await loop.run_in_executor(None, stream_gears.main_loop)



class GracefulExit(SystemExit):
    code = 1


if __name__ == '__main__':
    arg_parser()
