from datetime import datetime, timedelta, timezone
import logging.config
import time

# logging.config.fileConfig('Engine/configlog.ini', )

__all__ = ['download', 'upload', 'work', 'links_id', 'Enginebase', 'logger']

logger = logging.getLogger('log01')
links_id = {
    '星际2Innovation吕布卫星人族天梯第一视角': {'Twitch': 'innovation_s2', 'Panda': '1160340'},
    '星际2soO输本虫族天梯第一视角': {'Twitch': 'sc2soo', 'Panda': '1150595'},
    '星际2sOs狗哥神族天梯第一视角': {'Panda': '1160930'},
    '星际2Stats拔本神族天梯第一视角': {'Twitch': 'kimdaeyeob3'},
    '星际2Dark暗本虫族天梯第一视角': {'Twitch': 'qkrfuddn0'},
    '星际2Scarlett噶姐虫族天梯第一视角': {'Twitch': 'scarlettm'},
    '星际2GuMiho砸本人族天梯第一视角': {'Twitch': 'gumiho'},
    '星际2Maru人族天梯第一视角': {'Twitch': 'maru072'},
    '星际2TY全教主全太阳人族天梯第一视角': {'Twitch': 'sc2tyty'},
    '星际2ByuN武圣人族天梯第一视角': {'Twitch': 'byunprime'},
    '星际2小herO神族天梯第一视角': {'Twitch': 'dmadkr0818'},
    '星际2Zest神族天梯第一视角': {'Twitch': 'sc2_zest'},
    '星际2PartinG跳跳胖丁神族天梯第一视角': {'Twitch': 'partingthebigboy'},
    '星际2Rogue脑虫虫族天梯第一视角': {'Twitch': 'roguejinair'}
    # 'test':{'Twitch':'roguejinair', 'Panda':'439695'},
    # 'test1':{'Twitch':'rotterdam08', 'Panda':'1160340'}
}

root_url = {
    'Twitch': 'https://www.twitch.tv/',
    'Panda': 'https://www.panda.tv/',
    'Twitch_check': 'https://api.twitch.tv/kraken/streams/'}


class Enginebase(object):
    def __init__(self, dictionary, key, suffix):
        self.dic = dictionary
        self.key = key
        self.urlpath = self.dic[key]
        self.url = self.join_url()
        self.suffix = suffix
        # self.file_name = self.join_filename(suffix)

    def join_url(self):
        url = {}
        for n in self.urlpath:
            u = root_url[n] + self.urlpath[n]
            cu = root_url.get(n + '_check')
            if cu:
                url[n + '_check'] = cu + self.urlpath[n]
            url[n] = u
        # print(url)
        return url

    @property
    def file_name(self):
        now = Enginebase.time_now()
        if self.suffix == 'mp4':
            file_name = '%s%s%s.mp4' % (self.key, now, str(time.time())[:10])
        elif self.suffix == 'flv':
            file_name = '%s%s%s.flv' % (self.key, now, str(time.time())[:10])
        else:
            file_name = '%s%s' % (self.key, now)
        return file_name

    @staticmethod
    def time_now():
        utc_dt = datetime.utcnow().replace(tzinfo=timezone.utc)
        bj_dt = utc_dt.astimezone(timezone(timedelta(hours=8)))
        now = bj_dt.strftime('%Y{0}%m{1}%d{2}').format(*'年月日')
        return now


if __name__ == '__main__':
    for k in links_id.copy():
        pd = Enginebase(dictionary=links_id, key=k, suffix='mp4')
        # pd.is_recursion(None)
        # if k == '星际2Scarlett噶姐虫族天梯第一视角':
        #     break
    print(links_id)
