import asyncio
import logging
import subprocess
import sys
import os

logger = logging.getLogger('biliup')

global global_reloader


def has_extension(fname_list, *extension):
    for fname in fname_list:
        result = list(map(fname.endswith, extension))
        if True in result:
            return True
    return False


def is_docker():
    path = '/proc/self/cgroup'
    return (
            os.path.exists('/.dockerenv') or
            os.path.isfile(path) and any('docker' in line for line in open(path))
    )
