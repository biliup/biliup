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


class Monitoring(Timer):
    def __init__(self, parent_pid, file_name):
        super().__init__(func=self.kill_child_processes)
        self.parent = self.children = self.numc = None
        self.parent_pid = parent_pid
        self.file_name = file_name + '.part'
        self.last_file_size = 0.0
        self.flag = Event()

    def terminate(self):
        if self.numc == 0:
            logger.error("ChildrenProcess doesn't exist")
        else:
            for process in self.children:
                process.terminate()
            # logger.info('下载卡死' + self.file_name)

    def get_process(self, parent_pid):
        try:
            parent = psutil.Process(parent_pid)
        except psutil.NoSuchProcess:
            self.stop()
            return logger.error("Process doesn't exist")
        children = parent.children(recursive=True)
        numc = len(children)
        return parent, children, numc

    def kill_child_processes(self):
        if self.flag.is_set():
            self.stop()
            return
        file_size = os.path.getsize(self.file_name) / 1024 / 1024 / 1024
        if file_size <= self.last_file_size:
            logger.error('下载卡死' + self.file_name)
            if self.numc == 0:
                self.parent.terminate()
            else:
                self.terminate()
            time.sleep(1)
            if os.path.isfile(self.file_name):
                return logger.info('卡死下载进程可能未成功退出')
            else:
                self.stop()
                return logger.info('卡死下载进程成功退出')
        self.last_file_size = file_size
        if file_size >= 2.5:
            self.flag.set()
            self.terminate()
            logger.info('分段下载' + self.file_name)

    def __timer(self):
        logger.debug('获取到{0}，{1}'.format(self.parent_pid, self.file_name))
        retry = 0
        while not self._flag.wait(self.interval):
            self.parent, self.children, self.numc = self.get_process(self.parent_pid)
            if os.path.isfile(self.file_name):
                self._func(*self._args, **self._kwargs)
            else:
                logger.info('%s不存在' % self.file_name)
                if retry >= 2:
                    self.terminate()
                    return logger.info('结束进程，找不到%s' % self.file_name)
                retry += 1
                # logger.info('监控<%s>线程退出' % self.file_name)

    def run(self):
        try:
            self.__timer()
        finally:
            logger.debug('退出监控<%s>线程' % self.file_name)


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
