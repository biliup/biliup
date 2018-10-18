def singleton(cls):
    instance = cls()
    cls.__call__ = lambda self: instance
    return instance


# def wait_child(signum, frame):
#     logger.debug('receive SIGCHLD')
#     try:
#         while True:
#             # -1 表示任意子进程
#             # os.WNOHANG 表示如果没有可用的需要 wait 退出状态的子进程，立即返回不阻塞
#             cpid, status = os.waitpid(-1, os.WNOHANG)
#             if cpid == 0:
#                 logger.debug('no child process was immediately available')
#                 break
#             exitcode = status >> 8
#             logger.debug('child process %s exit with exitcode %s', cpid, exitcode)
#     except OSError as e:
#         if e.errno == errno.ECHILD:
#             logger.error('current process has no existing unwaited-for child processes.')
#         else:
#             raise
#     logger.debug('handle SIGCHLD end')


# def signal_handler(signum, frame):
#     logger.info('收到Terminate信号')
#     raise youtube_dl.utils.DownloadError(signum)


# def monitoring(q):
#     # signal.signal(signal.SIGCHLD, wait_child)
#     while True:
#         # print('开始监测')
#         pid, file_name = q.get()
#         time.sleep(5)
#         logger.info('获取到{0}，{1}'.format(pid, file_name))
#         t = Thread(target=kill_child_processes, args=(pid, file_name))
#         t.start()
