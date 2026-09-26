'use client'
import { API_BASE, handleResponse } from './api-streamer'

/**
 * Fleet 分层配置（只有控制面有这些接口）：全局配置 ⊕ 节点覆盖 ⊕ 节点本地密钥。
 * 控制面只存、只下发白名单里的键；Cookie、密码留在各节点本机。
 */

/**
 * 能随 Fleet 下发的键，与后端 `redact::VISIBLE_CONFIG_KEYS` 同序。
 * 后端测试 `frontend_key_lists_match` 会读这个文件核对，两边增删必须同步。
 */
export const DELIVERABLE_KEYS = [
  'downloader',
  'sync_save_dir',
  'ffmpeg_path',
  'file_size',
  'segment_time',
  'filtering_threshold',
  'filename_prefix',
  'segment_processor_parallel',
  'uploader',
  'submit_api',
  'lines',
  'threads',
  'delay',
  'event_loop_interval',
  'checker_sleep',
  'pool1_size',
  'pool2_size',
  'live_merge_minutes',
  'retention_hours',
  'min_free_space',
  'preview_transport',
  'preview_max_minutes',
  'use_live_cover',
  'douyu_cdn',
  'douyu_force_hs',
  'douyu_danmaku',
  'douyu_rate',
  'douyu_codec',
  'douyu_disable_interactive_game',
  'huya_cdn',
  'huya_cdn_fallback',
  'huya_danmaku',
  'huya_max_ratio',
  'huya_protocol',
  'huya_imgplus',
  'huya_mobile_api',
  'huya_codec',
  'douyin_danmaku',
  'douyin_quality',
  'douyin_protocol',
  'douyin_double_screen',
  'douyin_true_origin',
  'cc_protocol',
  'kila_protocol',
  'bilibili_danmaku',
  'bilibili_danmaku_detail',
  'bilibili_danmaku_raw',
  'bili_protocol',
  'bili_cdn',
  'bili_force_source',
  'bili_liveapi',
  'bili_fallback_api',
  'bili_cdn_fallback',
  'bili_hls_transcode_timeout',
  'bili_replace_cn01',
  'bili_qn',
  'bili_anonymous_origin',
  'youtube_prefer_vcodec',
  'youtube_prefer_acodec',
  'youtube_max_resolution',
  'youtube_max_videosize',
  'youtube_after_date',
  'youtube_before_date',
  'youtube_enable_download_live',
  'youtube_enable_download_playback',
  'youtube_danmaku',
  'ytb_danmaku',
  'twitch_danmaku',
  'twitch_disable_ads',
  'twitcasting_danmaku',
  'twitcasting_quality',
  'loggers_level',
]

/** 跟机器走的键，与后端 `layers::PER_NODE_KEYS` 同序：全局配置不管，只能写进节点覆盖 */
export const PER_NODE_KEYS = [
  'pool1_size',
  'pool2_size',
  'ffmpeg_path',
  'sync_save_dir',
  'min_free_space',
  'loggers_level',
]

export const isPerNode = (key: string) => PER_NODE_KEYS.includes(key)
export const SHARED_KEYS = DELIVERABLE_KEYS.filter((key) => !isPerNode(key))

/** 空间配置页里只在本机填写、不随 Fleet 下发的字段（x-field-id） */
export const LOCAL_SECRET_FIELDS = [
  'user.bili_cookie',
  'user.bili_cookie_file',
  'user.douyin_cookie',
  'user.twitch_cookie',
  'user.youtube_cookie',
  'user.twitcasting_cookie',
  'user.afreecatv_username',
  'user.afreecatv_password',
  'user.niconico-email',
  'user.niconico-password',
  'user.niconico-user-session',
  'user.niconico-purge-credentials',
  'kuaishou_cookie',
  'douyu_deviceId',
  'twitcasting_password',
]

export type ConfigValues = Record<string, unknown>

/** `GET /v1/fleet/configuration`；`PUT` 的响应多了 `changed` 与 `ignored` */
export interface FleetConfig {
  /** 还没保存过为 0 */
  version: number
  saved: boolean
  updated_at: number | null
  updated_by: number | null
  /** 全部共享键；还没保存过时是默认值，此时节点不收全局配置 */
  config: ConfigValues
  per_node_keys: string[]
  changed?: boolean
  ignored?: string[]
}

export interface ConfigVersion {
  version: number
  config: ConfigValues
  updated_at: number
  updated_by: number | null
}

export interface ConfigHistory {
  keep: number
  /** 新的在前 */
  versions: ConfigVersion[]
}

/** 与后端 `NodeConfigState` 一致 */
export interface NodeConfigState {
  /** 离线为 null；`unsupported` 是节点版本太旧，只收房间不收配置 */
  sync: 'unsupported' | 'pending' | 'applied' | 'failed' | null
  error: string | null
  /** 节点的 biliup 版本或协议次版本比控制面旧 */
  outdated: boolean
  override_keys: string[]
}

/** `GET /v1/fleet/nodes/{id}/config` */
export interface NodeConfig {
  node_id: number
  override: ConfigValues
  /** 节点应收到的：全局共享键 ⊕ 覆盖 */
  delivered: ConfigValues
  global: FleetConfig
  state: NodeConfigState
}

export const FLEET_CONFIG_KEY = '/v1/fleet/configuration'
export const FLEET_CONFIG_HISTORY_KEY = '/v1/fleet/configuration/history'
export const nodeConfigKey = (id: number) => `/v1/fleet/nodes/${id}/config`

async function putJson<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(API_BASE + path, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  await handleResponse(res)
  return res.json()
}

export function saveFleetConfig(config: ConfigValues) {
  return putJson<FleetConfig>(FLEET_CONFIG_KEY, config)
}

export function saveNodeOverride(id: number, override: ConfigValues) {
  return putJson<{ node_id: number; override: ConfigValues; ignored: string[] }>(nodeConfigKey(id), override)
}

/** 表单清空后的值：Semi 清空输入框会给 undefined 或空串，接口里一律是 null */
const blank = (value: unknown) => (value === undefined || value === '' ? null : value)
const same = (a: unknown, b: unknown) => JSON.stringify(blank(a)) === JSON.stringify(blank(b))

/**
 * 全局配置的请求体：键集合永远是上次保存的那一份（共享键全集），键数不随表单变。
 * 表单里没动过的字段（与挂载后的快照相同）原样回传保存值，避免字段自带的 initValue 或格式换算造成「没改也变」。
 */
export function globalPayload(saved: ConfigValues, initial: ConfigValues, values: ConfigValues): ConfigValues {
  const body: ConfigValues = {}
  for (const key of Object.keys(saved)) {
    body[key] = same(values[key], initial[key]) ? saved[key] : blank(values[key])
  }
  return body
}

/**
 * 节点覆盖的请求体：只含这台节点与全局不同的键。
 * 没动过的字段保留原有覆盖；改动后清空的字段、改成与全局相同的共享字段都撤掉覆盖。
 * `global` 为 null 表示还没保存过全局配置（节点不收共享键），这时填了什么就覆盖什么。
 */
export function overridePayload(
  current: ConfigValues,
  initial: ConfigValues,
  values: ConfigValues,
  global: ConfigValues | null,
): ConfigValues {
  const body: ConfigValues = {}
  for (const key of DELIVERABLE_KEYS) {
    if (same(values[key], initial[key])) {
      if (key in current) body[key] = current[key]
      continue
    }
    const value = blank(values[key])
    if (value === null) continue
    if (global && !isPerNode(key) && same(value, global[key])) continue
    body[key] = value
  }
  return body
}

/**
 * 受控节点保存本机密钥的请求体：以服务端当前配置为底，只换掉改动过的密钥字段。
 * 其余字段原样回传，白名单投影不变，本机的只读拦截（409）不会误伤。
 */
export function secretsPayload(entity: ConfigValues, initial: ConfigValues, values: ConfigValues): ConfigValues {
  const body: ConfigValues = { ...entity }
  const user: ConfigValues = { ...((entity.user as ConfigValues | null) ?? {}) }
  let userChanged = false
  const pick = (source: ConfigValues, path: string[]) =>
    path.length === 1 ? source[path[0]] : ((source.user as ConfigValues | undefined) ?? {})[path[1]]
  for (const field of LOCAL_SECRET_FIELDS) {
    const path = field.startsWith('user.') ? ['user', field.slice('user.'.length)] : [field]
    const value = pick(values, path)
    if (same(value, pick(initial, path))) continue
    if (path.length === 1) {
      body[field] = blank(value)
    } else {
      user[path[1]] = blank(value)
      userChanged = true
    }
  }
  if (userChanged) body.user = user
  return body
}

const BADGE_TONES = {
  primary: ['var(--semi-color-primary-light-default)', 'var(--semi-color-primary)'],
  grey: ['var(--semi-color-fill-1)', 'var(--semi-color-text-2)'],
} as const

export interface FieldMarks {
  /** 只显示这些字段（其余带 x-field-id 的字段隐藏，Form.Slot 之类的非字段元素不受影响） */
  show: string[]
  /** 显示但不能改 */
  lock?: string[]
  badges?: { keys: string[]; text: string; tone: keyof typeof BADGE_TONES }[]
}

/** 全宽的配置抽屉要压住移动端固定在左上角的菜单按钮（z-index 1001），否则标题被挡 */
export const SHEET_Z_INDEX = 1002

const fieldSelector = (keys: string[]) => keys.map((key) => `[x-field-id="${key}"]`).join(',')

/**
 * 复用空间配置的表单组件时，按字段 id 生成限定在 `[data-field-scope=scope]` 里的样式：
 * 隐藏不相关的字段、锁住或给字段标签加标记。组件本身不用改。
 */
export function fieldMarksCss(scope: string, marks: FieldMarks): string {
  const root = `[data-field-scope="${scope}"] .semi-form-field`
  const rules = [`${root}[x-field-id]:not(${fieldSelector(marks.show)}){display:none}`]
  if (marks.lock?.length) {
    rules.push(`${root}:is(${fieldSelector(marks.lock)}){opacity:.55;pointer-events:none}`)
  }
  for (const badge of marks.badges ?? []) {
    if (!badge.keys.length) continue
    const [background, color] = BADGE_TONES[badge.tone]
    rules.push(
      `${root}:is(${fieldSelector(badge.keys)}) .semi-form-field-label-text::after{content:'${badge.text}';` +
        `display:inline-block;margin-left:6px;padding:0 6px;border-radius:4px;font-size:11px;font-weight:500;` +
        `line-height:18px;vertical-align:1px;background:${background};color:${color}}`,
    )
  }
  return rules.join('\n')
}

/** 相邻两版之间取值不同的键，按白名单顺序 */
export function changedKeys(a: ConfigValues, b: ConfigValues): string[] {
  return DELIVERABLE_KEYS.filter((key) => (key in a || key in b) && !same(a[key], b[key]))
}
