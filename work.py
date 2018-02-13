import logging
import signal
from logging import FileHandler
from logging.handlers import TimedRotatingFileHandler
from multiprocessing import Process
import time
import os
import errno
import psutil

# log_fmt = '%(asctime)s %(filename)s[line:%(lineno)d] %(levelname)s %(message)s'
# formatter = logging.Formatter(log_fmt)
# log_file_handler = TimedRotatingFileHandler(filename="ds_update.log", when="D", interval=1, backupCount=2)
# # log_file_handler.suffix = "%Y-%m-%d.log"
# # log_file_handler.extMatch = re.compile(r"^\d{4}-\d{2}-\d{2}.log$")
# log_file_handler.setFormatter(formatter)
# logging.basicConfig(level=logging.INFO)
# logger = logging.getLogger(__name__)
# logger.addHandler(log_file_handler)
logger = logging.getLogger('log01')


def wait_child(signum, frame):
    logger.debug('receive SIGCHLD')
    try:
        while True:
            # -1 表示任意子进程
            # os.WNOHANG 表示如果没有可用的需要 wait 退出状态的子进程，立即返回不阻塞
            cpid, status = os.waitpid(-1, os.WNOHANG)
            if cpid == 0:
                logger.debug('no child process was immediately available')
                break
            exitcode = status >> 8
            logger.debug('child process %s exit with exitcode %s', cpid, exitcode)
    except OSError as e:
        if e.errno == errno.ECHILD:
            logger.error('current process has no existing unwaited-for child processes.')
        else:
            raise
    logger.debug('handle SIGCHLD end')


def kill_child_processes(parent_pid, file_name_, sig=signal.SIGINT):
    file_name_ = file_name_ + '.part'
    last_file_size = 0.0
    while True:
        time.sleep(10)
        if os.path.isfile(file_name_):
            file_size = os.path.getsize(file_name_) / 1024 / 1024 / 1024
            file_sizes = os.path.getsize(file_name_)
            if float(file_sizes) == last_file_size:
                try:
                    parent = psutil.Process(parent_pid)
                except psutil.NoSuchProcess:
                    return
                children = parent.children(recursive=True)
                if len(children) == 0:
                    parent.send_signal(sig)
                    logger.info('下载卡死pandaTV' + file_name_)
                else:
                    for process in children:
                        # print(process)
                        process.send_signal(sig)
                    logger.info('下载卡死' + file_name_)
                time.sleep(2)
                if os.path.isfile(file_name_):
                    logger.info('卡死下载进程可能未成功退出')
                    break
                else:
                    logger.info('卡死下载进程成功退出')
                    break

            last_file_size = file_sizes

            if float(file_size) >= 3.9:
                try:
                    parent = psutil.Process(parent_pid)
                except psutil.NoSuchProcess:
                    return
                children = parent.children(recursive=True)
                if len(children) == 0:
                    parent.send_signal(sig)
                    logger.info('分段下载pandatv' + file_name_)
                else:
                    for process in children:
                        # print(process)
                        process.send_signal(sig)
                    print('分段下载twitch')
                    logger.info('分段下载twitch' + file_name_)
                break
        else:
            return
            # os._exit(0)


def monitoring(q):
    signal.signal(signal.SIGCHLD, wait_child)
    while True:
        # print('开始监测')
        pid, file_name = q.get()
        time.sleep(5)
        # print('获取到{0}，{1}'.format(pid,file_name))
        p = Process(target=kill_child_processes, args=(pid, file_name))
        p.start()


class SafeRotatingFileHandler(TimedRotatingFileHandler):
    def __init__(self, filename, when='h', interval=1, backupCount=0, encoding=None, delay=False, utc=False):
        TimedRotatingFileHandler.__init__(self, filename, when, interval, backupCount, encoding, delay, utc)

    """
    Override doRollover
    lines commanded by "##" is changed by cc
    """

    def doRollover(self):
        """
        do a rollover; in this case, a date/time stamp is appended to the filename
        when the rollover happens.  However, you want the file to be named for the
        start of the interval, not the current time.  If there is a backup count,
        then we have to get a list of matching filenames, sort them and remove
        the one with the oldest suffix.

        Override,   1. if dfn not exist then do rename
                    2. _open with "a" model
        """
        if self.stream:
            self.stream.close()
            self.stream = None
        # get the time that this sequence started at and make it a TimeTuple
        currentTime = int(time.time())
        dstNow = time.localtime(currentTime)[-1]
        t = self.rolloverAt - self.interval
        if self.utc:
            timeTuple = time.gmtime(t)
        else:
            timeTuple = time.localtime(t)
            dstThen = timeTuple[-1]
            if dstNow != dstThen:
                if dstNow:
                    addend = 3600
                else:
                    addend = -3600
                timeTuple = time.localtime(t + addend)
        dfn = self.baseFilename + "." + time.strftime(self.suffix, timeTuple)
        ##        if os.path.exists(dfn):
        ##            os.remove(dfn)

        # Issue 18940: A file may not have been created if delay is True.
        ##        if os.path.exists(self.baseFilename):
        if not os.path.exists(dfn) and os.path.exists(self.baseFilename):
            os.rename(self.baseFilename, dfn)
        if self.backupCount > 0:
            for s in self.getFilesToDelete():
                os.remove(s)
        if not self.delay:
            self.mode = "a"
            self.stream = self._open()
        newRolloverAt = self.computeRollover(currentTime)
        while newRolloverAt <= currentTime:
            newRolloverAt = newRolloverAt + self.interval
        # If DST changes and midnight or weekly rollover, adjust for this.
        if (self.when == 'MIDNIGHT' or self.when.startswith('W')) and not self.utc:
            dstAtRollover = time.localtime(newRolloverAt)[-1]
            if dstNow != dstAtRollover:
                if not dstNow:  # DST kicks in before next rollover, so we need to deduct an hour
                    addend = -3600
                else:  # DST bows out before next rollover, so we need to add an hour
                    addend = 3600
                newRolloverAt += addend
        self.rolloverAt = newRolloverAt

