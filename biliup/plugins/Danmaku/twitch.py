import random
import re


class Twitch:
    heartbeat = "PING"
    heartbeatInterval = 40

    @staticmethod
    async def get_ws_info(url):
        reg_datas = []
        room_id = re.search(r"/([^/?]+)[^/]*$", url).group(1)

        reg_datas.append("CAP REQ :twitch.tv/tags twitch.tv/commands twitch.tv/membership")
        reg_datas.append("PASS SCHMOOPIIE")
        nick = f"justinfan{int(8e4 * random.random() + 1e3)}"
        reg_datas.append(f"NICK {nick}")
        reg_datas.append(f"USER {nick} 8 * :{nick}")
        reg_datas.append(f"JOIN #{room_id}")

        return "wss://irc-ws.chat.twitch.tv", reg_datas

    @staticmethod
    def decode_msg(data):
        msgs = []
        if data is not None:
            for d in data.splitlines():
                msgt = {}
                try:
                    msgt["content"] = re.search(r"PRIVMSG [^:]+:(.+)", d).group(1)
                    msgt["name"] = re.search(r"display-name=([^;]+);", d).group(1)
                    # if msgt["content"][0] == '@': continue # 丢掉表情符号
                    c = re.search(r"color=#([a-zA-Z0-9]{6});", d).group(1)
                    msgt["color"] = int(c, 16)
                    msgt["msg_type"] = "danmaku"
                    # print(msgt)
                    msgs.append(msgt)
                except Exception as Error:
                    # print(f"twitch：decode_msg：捕获到异常：{Error}")
                    # traceback.print_exc()
                    pass
        return msgs
