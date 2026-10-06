import type React from 'react'
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

/**
 * 空间配置「平台设置」左栏的展示顺序与文案。
 * 各插件组件接受 `bare` 属性：为 true 时只渲染字段（空间配置页），否则渲染成 Collapse.Panel（配置覆写弹窗）。
 * 新增插件时在这里登记一行即可出现在 UI 里。
 */
export const PlatformPanels: { key: string; name: string; Component: React.FC<any> }[] = [
  { key: 'bilibili', name: '哔哩哔哩', Component: Bilibili },
  { key: 'cc', name: 'CC', Component: CC },
  { key: 'douyin', name: '抖音', Component: Douyin },
  { key: 'douyu', name: '斗鱼', Component: Douyu },
  { key: 'huya', name: '虎牙', Component: Huya },
  { key: 'kilakila', name: '克拉克拉', Component: Kilakila },
  { key: 'twitcasting', name: 'TwitCasting', Component: Twitcasting },
  { key: 'twitch', name: 'Twitch', Component: Twitch },
  { key: 'youtube', name: 'YouTube', Component: Youtube },
  { key: 'user', name: '用户 Cookie', Component: Cookie },
]

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
