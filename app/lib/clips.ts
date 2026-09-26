'use client'
import { useState } from 'react'
import useSWR, { mutate } from 'swr'
import { API_BASE, fetcher, handleResponse } from './api-streamer'
import { ReportedError } from './markers'

export type ClipMode = 'quick' | 'precise'
export type ClipState = 'draft' | 'exporting' | 'ready' | 'failed' | 'published' | 'discarded'

/** 导出中的进度：`ratio` 是 0～1，不知道总量时为 null */
export interface ClipProgress {
  phase: string
  ratio: number | null
}

/** GET /v1/sessions/{id}/clips 返回的切片（后端 `api/clips.rs::ClipView`，这里只列回看页用到的字段） */
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

export const clipsUrl = (sessionId: number) => `/v1/sessions/${sessionId}/clips`

const listRefresh = (data?: { clips: Clip[] }) => (data?.clips.some((c) => c.state === 'exporting') ? 1000 : 0)

/**
 * 场次的切片列表；有切片在导出时每秒刷新一次。不能把函数直接交给 SWR 的 `refreshInterval`：函数返回 0 后
 * SWR 的定时器就不再续期，之后开始的导出不会被轮询到；传数值时数值一变 SWR 会重新计时。
 */
export function useSessionClips(sessionId: number) {
  const [ms, setMs] = useState(0)
  const swr = useSWR<{ clips: Clip[] }>(clipsUrl(sessionId), fetcher, { refreshInterval: ms })
  const wanted = listRefresh(swr.data)
  if (wanted !== ms) setMs(wanted)
  return swr
}

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

/** 按入点、出点建一个切片（草稿，不导出） */
export async function createClip(
  sessionId: number,
  body: { in_ms: number; out_ms: number; title?: string; marker_id?: number | null }
): Promise<Clip> {
  const clip = await sendJson<Clip>(clipsUrl(sessionId), 'POST', {
    ...body,
    in_ms: Math.max(0, Math.round(body.in_ms)),
    out_ms: Math.round(body.out_ms),
  })
  void mutate(clipsUrl(sessionId))
  return clip
}

export async function updateClip(
  clip: Pick<Clip, 'id' | 'session_id'>,
  patch: { in_ms?: number; out_ms?: number; title?: string }
): Promise<Clip> {
  const updated = await sendJson<Clip>(`${clipsUrl(clip.session_id)}/${clip.id}`, 'PATCH', patch)
  void mutate(clipsUrl(clip.session_id))
  return updated
}

export async function deleteClip(clip: Pick<Clip, 'id' | 'session_id'>): Promise<void> {
  await send(`${clipsUrl(clip.session_id)}/${clip.id}`, { method: 'DELETE' })
  void mutate(clipsUrl(clip.session_id))
}
