import logging
import os
import re
import time
from threading import Event
import psutil
from common.timer import Timer

logger = logging.getLogger('log01')


class BatchCheckBase:
    def __init__(self, pattern_id, urls):
        self.usr_dict = {}
        self.usr_list = []
        self.pattern_id = pattern_id
        for url in urls:
            self.get_id(url)

    def get_id(self, url):
        m = re.match(self.pattern_id, url)
        if m:
            usr_id = m.group('id')
            self.usr_dict[usr_id.lower()] = url
            self.usr_list.append(usr_id)

    def check(self):
        pass


class Companion(Timer):
    def __init__(self, pid, file_name, size=2.5):
        super().__init__(interval=20)
        self._pid = pid
        self.proc = psutil.Process(self._pid)
        self.file_name = file_name + '.part'
        self._size = size
        self.last_file_size = 0.0

    def kill_child_processes(self):
        file_size = os.path.getsize(self.file_name) / 1024 / 1024 / 1024
        if file_size <= self.last_file_size:
            logger.error('下载卡死' + self.file_name)
            self.proc.terminate()
            time.sleep(1)
            if os.path.isfile(self.file_name):
                return logger.info('卡死下载进程可能未成功退出')
            else:
                self.stop()
                return logger.info('卡死下载进程成功退出')
        self.last_file_size = file_size
        if file_size >= self._size:
            self.proc.terminate()
            self.stop()
            logger.info('分段下载' + self.file_name)

    def run(self):
        retry = 0
        while not self._flag.wait(self.interval):
            if os.path.isfile(self.file_name):
                self.kill_child_processes()
            else:
                logger.info('%s不存在' % self.file_name)
                if retry >= 2:
                    self.proc.terminate()
                    return logger.info('结束进程，找不到%s' % self.file_name)
                retry += 1
                # logger.info('监控<%s>线程退出' % self.file_name)


def match1(text, *patterns):
    if len(patterns) == 1:
        pattern = patterns[0]
        match = re.search(pattern, text)
        if match:
            return match.group(1)
        else:
            return None
    else:
        ret = []
        for pattern in patterns:
            match = re.search(pattern, text)
            if match:
                ret.append(match.group(1))
        return ret
