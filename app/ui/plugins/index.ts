import Bilibili from './bilibili'
import CC from './cc'
import Cookie from './cookie'
import Douyin from './douyin'
import Douyu from './douyu'
import Huya from './huya'
import Kilakila from './kilakila'
import Twitcasting from './twitcasting'
import Twitch from './twitch'
import Youtube from './youtube'

export {
  Bilibili,
  CC,
  Cookie,
  Douyin,
  Douyu,
  Huya,
  Kilakila,
  Twitcasting,
  Twitch,
  Youtube,
  SupportedPlatforms,
}

// 导出所有插件
const plugins = {
  Bilibili,
  CC,
  Cookie,
  Douyin,
  Douyu,
  Huya,
  Kilakila,
  Twitcasting,
  Twitch,
  Youtube,
}

const SupportedPlatforms = {
  'https?:\/\/(b23\.tv|live\.bilibili\.com)': Bilibili,
  'https?:\/\/(cc\.163\.com)': CC,
  'https?:\/\/(?:(?:www|m|live|v)\.)?douyin\.com': Douyin,
  'https?:\/\/(?:(?:www|m)\.)?douyu\.com': Douyu,
  'https?:\/\/(?:(?:www|m)\.)?huya\.com': Huya,
  'https?:\/\/(live\.kilakila\.cn|www\.hongdoufm\.com)': Kilakila,
  'https?:\/\/twitcasting\.tv': Twitcasting,
  'https?:\/\/(?:(?:www|go|m)\.)?twitch\.tv': Twitch,
  'https?:\/\/(?:(?:www|m)\.)?youtube\.com': Youtube,
}

export default plugins
