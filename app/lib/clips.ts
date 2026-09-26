'use client'
import { useState } from 'react'
import useSWR, { mutate, type SWRConfiguration } from 'swr'
import { API_BASE, fetcher, handleResponse } from './api-streamer'
import { ReportedError } from './markers'

export type ClipMode = 'quick' | 'precise'
export type ClipState = 'draft' | 'exporting' | 'ready' | 'failed' | 'published' | 'discarded'

/** 导出中的进度：`ratio` 是 0～1，不知道总量时为 null */
export interface ClipProgress {
  phase: string
  ratio: number | null
}

/** GET /v1/sessions/{id}/clips、/v1/clips/{cid} 返回的切片（后端 `api/clips.rs::ClipView`，这里只列界面用到的字段） */
export interface Clip {
  id: number
  session_id: number
  marker_id: number | null
  /** 选段（场次时间，毫秒） */
  in_ms: number
  out_ms: number
  /** 最近一次导出实际切在哪里；快速剪落在关键帧上，可能比选段略宽 */
  cut_in_ms: number | null
  cut_out_ms: number | null
  mode: ClipMode | null
  title: string
  state: ClipState
  /** 产物文件名（不含目录） */
  file_name: string | null
  output_bytes: number | null
  /** 产物的媒体时长 */
  duration_ms: number | null
  /** 导出失败的原因 */
  error: string | null
  archive_bvid: string | null
  created_by: number | null
  created_at: number
  updated_at: number
  progress: ClipProgress | null
}

/** 与后端一致 */
export const MAX_TITLE_CHARS = 80

/** 「剪下刚才 N 秒」的档位 */
export const LAST_CLIP_OPTIONS = [
  { ms: 30_000, label: '30 秒' },
  { ms: 60_000, label: '1 分钟' },
  { ms: 120_000, label: '2 分钟' },
] as const

export const clipsUrl = (sessionId: number) => `/v1/sessions/${sessionId}/clips`
export const clipUrl = (id: number) => `/v1/clips/${id}`
export const downloadUrl = (id: number, format: 'source' | 'mp4') =>
  `${API_BASE}/v1/clips/${id}/download?format=${format}`

const listRefresh = (data?: { clips: Clip[] }) => (data?.clips.some((c) => c.state === 'exporting') ? 1000 : 0)
const oneRefresh = (clip?: Clip) => (!clip || clip.state === 'exporting' ? 1000 : 0)

/**
 * 按最新数据决定轮询间隔（导出中每秒一次）。不能把函数直接交给 SWR 的 `refreshInterval`：函数返回 0 后
 * SWR 的定时器就不再续期，之后开始的导出不会被轮询到；传数值时数值一变 SWR 会重新计时。
 */
function usePolled<T>(key: string | null, interval: (data?: T) => number, config?: SWRConfiguration<T>) {
  const [ms, setMs] = useState(0)
  const swr = useSWR<T>(key, fetcher, { ...config, refreshInterval: ms })
  const wanted = interval(swr.data)
  if (wanted !== ms) setMs(wanted)
  return swr
}

/** 场次的切片列表；有切片在导出时每秒刷新一次 */
export function useSessionClips(sessionId: number | null) {
  return usePolled(sessionId === null ? null : clipsUrl(sessionId), listRefresh)
}

/** 单个切片；导出中轮询，结束后停 */
export function useClip(id: number) {
  return usePolled(clipUrl(id), oneRefresh, { revalidateOnFocus: false })
}

export function isMp4(clip: Clip): boolean {
  return !!clip.file_name && clip.file_name.toLowerCase().endsWith('.mp4')
}

/** 产物扩展名（`.flv` / `.ts` / `.mp4`）；还没有产物时为空串 */
export function extensionOf(clip: Clip): string {
  const name = clip.file_name ?? ''
  const dot = name.lastIndexOf('.')
  return dot >= 0 ? name.slice(dot).toLowerCase() : ''
}

export function clipModeText(mode: ClipMode | null): string {
  return mode === 'precise' ? '精确剪' : mode === 'quick' ? '快速剪' : ''
}

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

function refresh(clip: Pick<Clip, 'id' | 'session_id'>) {
  void mutate(clipsUrl(clip.session_id))
  void mutate(clipUrl(clip.id))
}

/** 按入点、出点建一个切片；给了 `export` 就建好立即导出，否则是草稿 */
export async function createClip(
  sessionId: number,
  body: { in_ms: number; out_ms: number; title?: string; marker_id?: number | null; export?: ClipMode }
): Promise<Clip> {
  const clip = await sendJson<Clip>(clipsUrl(sessionId), 'POST', {
    ...body,
    in_ms: Math.max(0, Math.round(body.in_ms)),
    out_ms: Math.round(body.out_ms),
  })
  refresh(clip)
  return clip
}

/**
 * 看直播时「剪下刚才 N 秒」：出点是屏幕上正在放的这一帧，换算方式与打标记相同
 * （`pressedAt` 是按下的时刻，`latencyMs` 是播放器缓冲里还没播出的时长），建好立即快速剪。
 */
export async function createLiveClip(
  sessionId: number,
  { lastMs, pressedAt, latencyMs }: { lastMs: number; pressedAt: number; latencyMs: number | null }
): Promise<Clip> {
  const clip = await sendJson<Clip>(clipsUrl(sessionId), 'POST', {
    last_ms: lastMs,
    pressed_at: pressedAt,
    client_now: Date.now(),
    latency_ms: latencyMs,
    export: 'quick',
  })
  refresh(clip)
  return clip
}

export async function updateClip(
  clip: Pick<Clip, 'id' | 'session_id'>,
  patch: { in_ms?: number; out_ms?: number; title?: string }
): Promise<Clip> {
  const updated = await sendJson<Clip>(`${clipsUrl(clip.session_id)}/${clip.id}`, 'PATCH', patch)
  refresh(updated)
  return updated
}

/** 删除切片：正在进行的导出停下，导出的文件一起删掉（切片是文件的唯一归属，不删切片文件就一直留着） */
export async function deleteClip(clip: Pick<Clip, 'id' | 'session_id'>): Promise<void> {
  await send(`${clipsUrl(clip.session_id)}/${clip.id}`, { method: 'DELETE' })
  refresh(clip)
}

/** 开始导出（失败后重试也是它）；导出在后台进行，列表按导出中自动轮询 */
export async function exportClip(clip: Pick<Clip, 'id' | 'session_id'>, mode: ClipMode): Promise<Clip> {
  const exporting = await sendJson<Clip>(`${clipUrl(clip.id)}/export`, 'POST', { mode })
  refresh(exporting)
  return exporting
}

/**
 * 下载：先用 HEAD 让服务端备好文件（要 MP4 而产物不是 MP4 时这一步转封装，可能要几秒），
 * 失败时再用 GET 取服务端的原因（失败的响应体很短）抛出，成功才交给浏览器下载，不把文件读进内存。
 */
export async function downloadClip(clip: Pick<Clip, 'id'>, format: 'source' | 'mp4'): Promise<void> {
  const url = downloadUrl(clip.id, format)
  const head = await fetch(url, { method: 'HEAD' })
  if (!head.ok) {
    const abort = new AbortController()
    const res = await fetch(url, { signal: abort.signal })
    if (!res.ok) {
      try {
        await handleResponse(res)
      } catch (e) {
        if (res.status === 401 || res.status === 403) {
          throw new ReportedError(e instanceof Error ? e.message : String(e))
        }
        throw e
      }
    }
    abort.abort()
  }
  const a = document.createElement('a')
  a.href = url
  a.download = ''
  a.rel = 'noopener'
  document.body.appendChild(a)
  a.click()
  a.remove()
}

interface ToolsStatus {
  ffmpeg: { available: boolean; error: string | null }
}

export type FfmpegState = { ready: boolean; reason: string | null }

/**
 * 精确剪（转码）、非 MP4 产物的 MP4 下载（转封装）要用服务器上的 ffmpeg。`reason` 为 null 表示可用，
 * 否则是给按钮提示用的原因。
 */
export function useFfmpeg(): FfmpegState {
  const { data, error } = useSWR<ToolsStatus>('/v1/tools', fetcher, {
    revalidateOnFocus: true,
    shouldRetryOnError: false,
  })
  if (error) return { ready: false, reason: `读不到服务器的 ffmpeg 状态（${errorText(error)}），精确剪和 MP4 下载暂不可用` }
  if (!data) return { ready: false, reason: '正在检查服务器上的 ffmpeg…' }
  if (data.ffmpeg.available) return { ready: true, reason: null }
  const detail = data.ffmpeg.error ? `：${data.ffmpeg.error}` : ''
  return {
    ready: false,
    reason: `服务器上的 ffmpeg 不可用${detail}。精确剪（转码）和转成 MP4 要用它：安装 ffmpeg，或在配置里把 ffmpeg_path 指向它，然后刷新页面。快速剪和源格式下载不受影响`,
  }
}

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}
