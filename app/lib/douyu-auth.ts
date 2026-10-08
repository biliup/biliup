import { apiFetch, handleResponse, revalidateMe } from './api-streamer'

export type DouyuLoginState = 'unknown' | 'valid' | 'invalid' | 'anonymous'
export type DouyuRefreshState =
  | 'disabled'
  | 'missing_credentials'
  | 'scheduled'
  | 'refreshing'
  | 'retry'
  | 'credentials_invalid'

/** Safe status only: the API never returns Cookie, LTP0 or device credentials. Times are Unix seconds. */
export interface DouyuAuthStatus {
  streamer_id: number | null
  enabled: boolean
  has_cookie: boolean
  has_ltp0: boolean
  has_device_id: boolean
  login_state: DouyuLoginState
  refresh_state: DouyuRefreshState
  account_id: string | null
  last_success_at: number | null
  last_checked_at: number | null
  next_refresh_at: number | null
  failure_count: number
  needs_login: boolean
}

const LOGIN_STATES: readonly string[] = ['unknown', 'valid', 'invalid', 'anonymous']
const REFRESH_STATES: readonly string[] = [
  'disabled', 'missing_credentials', 'scheduled', 'refreshing', 'retry', 'credentials_invalid',
]

const timestamp = (value: unknown): number | null =>
  typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : null

export function parseDouyuAuthStatus(value: unknown): DouyuAuthStatus {
  if (!value || typeof value !== 'object') throw new Error('斗鱼登录状态响应无效')
  const data = value as Record<string, unknown>
  if (
    !LOGIN_STATES.includes(data.login_state as string) ||
    !REFRESH_STATES.includes(data.refresh_state as string) ||
    typeof data.enabled !== 'boolean' ||
    typeof data.has_cookie !== 'boolean' ||
    typeof data.has_ltp0 !== 'boolean' ||
    typeof data.has_device_id !== 'boolean' ||
    typeof data.needs_login !== 'boolean'
  ) throw new Error('斗鱼登录状态响应无效')
  // Pick only the known safe fields, including when a newer server sends additional fields.
  return {
    streamer_id: typeof data.streamer_id === 'number' && Number.isInteger(data.streamer_id) ? data.streamer_id : null,
    enabled: data.enabled,
    has_cookie: data.has_cookie,
    has_ltp0: data.has_ltp0,
    has_device_id: data.has_device_id,
    login_state: data.login_state as DouyuLoginState,
    refresh_state: data.refresh_state as DouyuRefreshState,
    account_id: typeof data.account_id === 'string' && /^[1-9]\d*$/.test(data.account_id) ? data.account_id : null,
    last_success_at: timestamp(data.last_success_at),
    last_checked_at: timestamp(data.last_checked_at),
    next_refresh_at: timestamp(data.next_refresh_at),
    failure_count: typeof data.failure_count === 'number' && Number.isInteger(data.failure_count) && data.failure_count >= 0
      ? data.failure_count : 0,
    needs_login: data.needs_login,
  }
}

export const douyuAuthStatusKey = (streamerId?: number) =>
  `/v1/douyu/auth/status${streamerId === undefined ? '' : `?streamer_id=${streamerId}`}`

/** Do not surface upstream response bodies or exception messages from credential requests. */
async function authRequest(path: string, init?: RequestInit): Promise<unknown> {
  const response = await apiFetch(path, init)
  if (response.status === 401) await handleResponse(response)
  if (response.status === 403) {
    revalidateMe()
    throw new Error('没有权限管理斗鱼登录凭据')
  }
  if (!response.ok) throw new Error('斗鱼登录请求失败，请稍后重试')
  return response.json()
}

export async function fetchDouyuAuthStatus(path: string): Promise<DouyuAuthStatus> {
  return parseDouyuAuthStatus(await authRequest(path))
}

export async function refreshDouyuAuth(streamerId?: number): Promise<DouyuAuthStatus> {
  return parseDouyuAuthStatus(await authRequest('/v1/douyu/auth/refresh', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(streamerId === undefined ? {} : { streamer_id: streamerId }),
  }))
}

export async function testDouyuCookie(cookie: string): Promise<boolean> {
  const value = await authRequest('/v1/douyu/validate-cookie', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ cookie }),
  })
  if (!value || typeof value !== 'object' || !('valid' in value) || typeof value.valid !== 'boolean') {
    throw new Error('斗鱼 Cookie 验证响应无效')
  }
  return value.valid
}

export const DOUYU_AUTH_FIELDS = [
  'douyu_cookie', 'douyu_ltp0', 'douyu_refresh_device_id', 'douyu_auto_refresh',
] as const

export type DouyuCookieMode = 'inherit' | 'custom' | 'anonymous'

/** Room overrides use null/absence for inheritance and an explicit empty string for anonymity. */
export function douyuCookieOverrideMode(cookie: unknown): DouyuCookieMode {
  if (typeof cookie !== 'string') return 'inherit'
  return cookie.trim() === '' ? 'anonymous' : 'custom'
}

/** A blank independent Cookie input must not accidentally disable the space's saved login. */
export function douyuRoomCookieValue(cookie: unknown, mode: DouyuCookieMode): string | null {
  if (mode === 'anonymous') return ''
  if (mode === 'inherit' || typeof cookie !== 'string' || cookie.trim() === '') return null
  return cookie
}

/** Empty field representations are equivalent; an unset switch uses the server's enabled default. */
export function douyuAuthFieldsEqual(a: Record<string, unknown>, b: Record<string, unknown>, roomOverride = false): boolean {
  return DOUYU_AUTH_FIELDS.every((key) => {
    if (roomOverride && key === 'douyu_cookie') {
      return (a[key] ?? null) === (b[key] ?? null)
    }
    const normalize = (value: unknown) => key === 'douyu_auto_refresh'
      ? value ?? true
      : value === undefined || value === null || value === '' ? '' : value
    return normalize(a[key]) === normalize(b[key])
  })
}

export function douyuAuthDate(value: number | null): string {
  if (value === null) return '尚无记录'
  const date = new Date(value * 1000)
  return Number.isNaN(date.getTime()) ? '尚无记录' : date.toLocaleString('zh-CN', { hour12: false })
}

/** An old Cookie can remain valid after a failed or deduplicated exchange. */
export function douyuManualRefreshMessage(previous: DouyuAuthStatus, next: DouyuAuthStatus): string {
  if (next.needs_login || next.refresh_state === 'credentials_invalid' || next.login_state === 'invalid') {
    return '续期未完成，请重新登录斗鱼并更新同一账号的来源凭据。'
  }
  if (next.refresh_state === 'retry' || next.failure_count > 0) {
    return '本次续期未完成，已安排退避重试，现有已保存 Cookie 会保留。'
  }
  if (next.refresh_state === 'missing_credentials') {
    return '续期未执行，请先保存同一次登录取得的 LTP0 和 dy_did。'
  }
  if (next.refresh_state === 'refreshing') {
    return '已有续期任务正在进行，请等待状态更新。'
  }
  const completed = next.last_success_at !== null
    && (previous.last_success_at === null || next.last_success_at > previous.last_success_at)
  if (completed && next.login_state === 'valid') {
    return '登录 Cookie 已续期，运行中的请求已更新。'
  }
  return '本次没有新的续期成功记录，可能处于短暂冷却期；请查看状态或稍后重试。'
}

export const DOUYU_LOGIN_LABELS: Record<DouyuLoginState, string> = {
  unknown: '等待登录校验', valid: '登录有效', invalid: '登录已失效', anonymous: '未配置登录 Cookie',
}

export const DOUYU_REFRESH_LABELS: Record<DouyuRefreshState, string> = {
  disabled: '自动续期已关闭',
  missing_credentials: '缺少续期凭据',
  scheduled: '等待下次续期',
  refreshing: '正在续期',
  retry: '续期失败，等待重试',
  credentials_invalid: '续期凭据已失效',
}
