import { API_BASE } from './api-streamer'

/** GET /v1/sessions 的一行（后端 `api/sessions.rs::SessionSummary`） */
export interface SessionSummary {
  id: number
  streamer_id: number | null
  streamer_name: string
  title: string
  /** Unix 毫秒 */
  started_at: number
  /** Unix 毫秒；进行中为 null */
  ended_at: number | null
  retain_until: number | null
  /** 本进程正在录这一场 */
  recording: boolean
  /** 时间轴长度（毫秒）。列表里正在写的分段不算；详情按已写入的内容算到最新 */
  duration_ms: number
  segment_count: number
  /** 只算写完的分段 */
  bytes: number
}

export interface SessionPage {
  items: SessionSummary[]
  total: number
  page: number
  page_size: number
}

export type SegmentState = 'recording' | 'finished' | 'missing' | 'deleted' | 'pending_delete'

export interface SegmentView {
  id: number
  container: 'flv' | 'ts' | 'mp4' | 'mkv'
  state: SegmentState
  start_ms: number
  end_ms: number | null
  bytes: number | null
  gap_before_ms: number
  file_name: string
  has_danmaku: boolean
}

export interface Gap {
  from_ms: number
  to_ms: number
}

export interface SessionDetail extends SessionSummary {
  segments: SegmentView[]
  gaps: Gap[]
}

export interface Keyframe {
  t_ms: number
  segment_id: number
}

export interface KeyframeList {
  keyframes: Keyframe[]
  truncated: boolean
}

export const sessionUrl = (id: number) => `/v1/sessions/${id}`
export const keyframesUrl = (id: number, from: number, to: number) =>
  `/v1/sessions/${id}/keyframes?from=${Math.max(0, Math.floor(from))}&to=${Math.max(0, Math.ceil(to))}`
export const mediaUrl = (id: number, from: number) =>
  `${API_BASE}/v1/sessions/${id}/media?from=${Math.max(0, Math.round(from))}`

/**
 * 录制中、已写完的分段能回看和剪；等待清理的也能（文件还在，是被切片等引用住了才没删），与后端一致
 */
export function isReadable(segment: SegmentView): boolean {
  return segment.state === 'recording' || segment.state === 'finished' || segment.state === 'pending_delete'
}

/** 分段的末尾；正在写且还没有关键帧时按起点算 */
export function segmentEnd(segment: SegmentView): number {
  return segment.end_ms ?? segment.start_ms
}

/** B 站 hls_fmp4 这类分片 MP4 录像：DVR 回看接口返回 415 */
export function isFragmentedMp4(segment: SegmentView): boolean {
  return segment.container === 'mp4'
}

/** 场次里有没有能播的内容（至少一个可读分段有长度） */
export function hasPlayableMedia(detail: SessionDetail): boolean {
  return detail.segments.some((s) => isReadable(s) && segmentEnd(s) > s.start_ms)
}

/** `t` 落在哪个分段里（含首尾）；落在缺口里返回 null */
export function segmentAt(segments: SegmentView[], t: number): SegmentView | null {
  for (const s of segments) {
    if (t >= s.start_ms && t <= segmentEnd(s)) return s
  }
  return null
}

/**
 * 从 `from` 回看时响应的封装：服务端从 `from` 所在的可读分段起播，落在缺口或不可读分段里就从下一个可读分段起播。
 * mpegts.js 要在建播放器前知道是 FLV 还是 TS。
 */
export function dvrContainer(segments: SegmentView[], from: number): 'flv' | 'mpegts' {
  const seg =
    segments.find((s) => isReadable(s) && from >= s.start_ms && from <= segmentEnd(s)) ??
    segments.filter((s) => isReadable(s) && s.start_ms > from).sort((a, b) => a.start_ms - b.start_ms)[0]
  return seg?.container === 'ts' ? 'mpegts' : 'flv'
}

/** `[from, to]` 碰到的不可用（已删除 / 丢失 / 待删除）分段 */
export function unavailableIn(segments: SegmentView[], from: number, to: number): SegmentView[] {
  return segments.filter((s) => !isReadable(s) && s.start_ms < to && segmentEnd(s) > from)
}

/** `[from, to]` 跨过的断流缺口数 */
export function gapsIn(gaps: Gap[], from: number, to: number): number {
  return gaps.filter((g) => g.from_ms < to && g.to_ms > from).length
}

/**
 * 可以落刀的位置：关键帧（只来自可读分段，后端已过滤），另加每个可读分段的末尾——
 * 出点落在分段末尾就是剪到这一段结束。按时间升序、去重。
 */
export function cutPoints(keyframes: Keyframe[], segments: SegmentView[], from: number, to: number): number[] {
  const points = keyframes.map((k) => k.t_ms)
  for (const s of segments) {
    const end = segmentEnd(s)
    if (isReadable(s) && end > s.start_ms && end >= from && end <= to) points.push(end)
  }
  points.sort((a, b) => a - b)
  return points.filter((t, i) => i === 0 || t !== points[i - 1])
}

/** 最后一个 ≤ t 的落刀点（入点规则：从 t 之前最近的关键帧开始） */
export function floorPoint(points: number[], t: number): number | null {
  let lo = 0
  let hi = points.length - 1
  let found: number | null = null
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    if (points[mid] <= t) {
      found = points[mid]
      lo = mid + 1
    } else {
      hi = mid - 1
    }
  }
  return found
}

/** 第一个 ≥ t 的落刀点（出点规则：剪到 t 之后最近的关键帧为止） */
export function ceilPoint(points: number[], t: number): number | null {
  let lo = 0
  let hi = points.length - 1
  let found: number | null = null
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    if (points[mid] >= t) {
      found = points[mid]
      hi = mid - 1
    } else {
      lo = mid + 1
    }
  }
  return found
}

/** 离 t 最近的落刀点（拖动手柄时吸附） */
export function nearestPoint(points: number[], t: number): number | null {
  const a = floorPoint(points, t)
  const b = ceilPoint(points, t)
  if (a === null) return b
  if (b === null) return a
  return t - a <= b - t ? a : b
}

/** 场次时间 → `1:02:03.4`（细节条、入出点用，带十分之一秒） */
export function formatPrecise(ms: number): string {
  const tenths = Math.max(0, Math.round(ms / 100))
  const total = Math.floor(tenths / 10)
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  const pad = (n: number) => String(n).padStart(2, '0')
  const frac = tenths % 10
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}.${frac}` : `${m}:${pad(s)}.${frac}`
}

/** 时长 → `1 小时 2 分` / `3 分 4 秒` / `5.2 秒` */
export function formatSpan(ms: number): string {
  if (ms < 60_000) return `${(Math.max(0, ms) / 1000).toFixed(1)} 秒`
  const total = Math.round(ms / 1000)
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  return h > 0 ? `${h} 小时 ${m} 分` : `${m} 分 ${s} 秒`
}
