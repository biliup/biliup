import re
from datetime import datetime, timezone, timedelta
from common import logger


def time_now():
    utc_dt = datetime.utcnow().replace(tzinfo=timezone.utc)
    bj_dt = utc_dt.astimezone(timezone(timedelta(hours=8)))
    # now = bj_dt.strftime('%Y{0}%m{1}%d{2}').format(*'...')
    now = bj_dt.strftime('%Y{0}%m{1}%d').format(*'..')
    return now


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


def new_hook(t, v, tb):
    logger.error("Uncaught exception：", exc_info=(t, v, tb))


# class SafeRotatingFileHandler(TimedRotatingFileHandler):
#     def __init__(self, filename, when='h', interval=1, backupCount=0, encoding=None, delay=False, utc=False):
#         TimedRotatingFileHandler.__init__(self, filename, when, interval, backupCount, encoding, delay, utc)
#
#     """
#     Override doRollover
#     lines commanded by "##" is changed by cc
#     """
#
#     def doRollover(self):
#         """
#         do a rollover; in this case, a date/time stamp is appended to the filename
#         when the rollover happens.  However, you want the file to be named for the
#         start of the interval, not the current time.  If there is a backup count,
#         then we have to get a list of matching filenames, sort them and remove
#         the one with the oldest suffix.
#
#         Override,   1. if dfn not exist then do rename
#                     2. _open with "a" model
#         """
#         if self.stream:
#             self.stream.close()
#             self.stream = None
#         # get the time that this sequence started at and make it a TimeTuple
#         currentTime = int(time.time())
#         dstNow = time.localtime(currentTime)[-1]
#         t = self.rolloverAt - self.interval
#         if self.utc:
#             timeTuple = time.gmtime(t)
#         else:
#             timeTuple = time.localtime(t)
#             dstThen = timeTuple[-1]
#             if dstNow != dstThen:
#                 if dstNow:
#                     addend = 3600
#                 else:
#                     addend = -3600
#                 timeTuple = time.localtime(t + addend)
#         dfn = self.baseFilename + "." + time.strftime(self.suffix, timeTuple)
#         ##        if os.path.exists(dfn):
#         ##            os.remove(dfn)
#
#         # Issue 18940: A file may not have been created if delay is True.
#         ##        if os.path.exists(self.baseFilename):
#         if not os.path.exists(dfn) and os.path.exists(self.baseFilename):
#             os.rename(self.baseFilename, dfn)
#         if self.backupCount > 0:
#             for s in self.getFilesToDelete():
#                 os.remove(s)
#         if not self.delay:
#             self.mode = "a"
#             self.stream = self._open()
#         newRolloverAt = self.computeRollover(currentTime)
#         while newRolloverAt <= currentTime:
#             newRolloverAt = newRolloverAt + self.interval
#         # If DST changes and midnight or weekly rollover, adjust for this.
#         if (self.when == 'MIDNIGHT' or self.when.startswith('W')) and not self.utc:
#             dstAtRollover = time.localtime(newRolloverAt)[-1]
#             if dstNow != dstAtRollover:
#                 if not dstNow:  # DST kicks in before next rollover, so we need to deduct an hour
#                     addend = -3600
#                 else:  # DST bows out before next rollover, so we need to add an hour
#                     addend = 3600
#                 newRolloverAt += addend
#         self.rolloverAt = newRolloverAt
