import { mutate } from 'swr'
import { API_BASE, handleResponse } from './api-streamer'

/** GET / POST /v1/sessions/{id}/markers 返回的标记 */
export interface Marker {
  id: number
  session_id: number
  /** 场次时间轴上的位置（毫秒） */
  at_ms: number
  label: string
  color: string | null
  /** 打标记的 Web 用户；未开 --auth 时为 null */
  created_by: number | null
  /** Unix 毫秒 */
  created_at: number
  lookback_ms: number
  lookahead_ms: number
}

const markersUrl = (sessionId: number) => `/v1/sessions/${sessionId}/markers`

/** 401 / 403：`handleResponse` 已经跳登录页或提示过，调用方不必再弹一次 */
export class ReportedError extends Error {}

async function send(url: string, init: RequestInit): Promise<Response> {
  const res = await fetch(API_BASE + url, init)
  try {
    return await handleResponse(res)
  } catch (e) {
    if (res.status === 401 || res.status === 403) {
      throw new ReportedError(e instanceof Error ? e.message : String(e))
    }
    throw e
  }
}

async function sendJson<T>(url: string, method: string, body: unknown): Promise<T> {
  const res = await send(url, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  return res.json()
}

/** 卡片、监视器上的本场标记数来自 /v1/streamers，标记增删后刷新一次 */
function refreshMarkerCounts() {
  void mutate('/v1/streamers')
}

/**
 * 按当前画面打一个标记：`pressedAt` 是按下的时刻（`Date.now()`），`latencyMs` 是播放器缓冲里还没播出的时长。
 * 服务端只用 `client_now − pressed_at` 这个差值，不要求本机时钟准。
 */
export async function createLiveMarker(
  sessionId: number,
  { pressedAt, latencyMs }: { pressedAt: number; latencyMs: number | null }
): Promise<Marker> {
  const marker = await sendJson<Marker>(markersUrl(sessionId), 'POST', {
    pressed_at: pressedAt,
    client_now: Date.now(),
    latency_ms: latencyMs,
  })
  refreshMarkerCounts()
  return marker
}

export async function renameMarker(sessionId: number, id: number, label: string): Promise<Marker> {
  return sendJson<Marker>(`${markersUrl(sessionId)}/${id}`, 'PATCH', { label })
}

export async function deleteMarker(sessionId: number, id: number): Promise<void> {
  await send(`${markersUrl(sessionId)}/${id}`, { method: 'DELETE' })
  refreshMarkerCounts()
}

/**
 * 播放器的延迟：缓冲里还没播出的时长（毫秒），即屏幕上这一帧落后于服务端收到它的时间。
 * 中转时就是缓冲深度（见 live-buffer.ts）；直连时浏览器与录制各连 CDN，两边落后 CDN 的程度相近，同样按缓冲深度算。
 * 还没有画面时返回 null。
 */
export function playerLatencyMs(root: HTMLElement | null): number | null {
  const video = root?.querySelector('video')
  if (!video || !video.buffered.length) return null
  const end = video.buffered.end(video.buffered.length - 1)
  const ahead = end - video.currentTime
  return Number.isFinite(ahead) && ahead >= 0 ? Math.round(ahead * 1000) : null
}

/** 场次时间 → `1:02:03` / `2:03` */
export function formatSessionTime(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000))
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  const pad = (n: number) => String(n).padStart(2, '0')
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`
}
