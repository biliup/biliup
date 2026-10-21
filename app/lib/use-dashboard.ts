'use client'
import { useSyncExternalStore } from 'react'
import useSWR from 'swr'
import { API_BASE, fetcher, LiveStreamerEntity, StreamerInfo, LiveUrlInfo, PreviewTransport } from './api-streamer'

/** Overview data is bounded on the server; system metrics use their own lightweight endpoint. */

/** 直播中(后端 WorkerStatus Debug 字符串,已验证与 WorkerStatus 枚举一致) */
export const LIVE_STATUS = 'Working'
export const PAUSE_STATUS = 'Pause'

export interface DashboardEvent {
  id: string | number
  /** Unix 毫秒 */
  ts: number
  kind: string
  text: string
  sub: string | null
  session_id: number | null
  entity_id: string | number | null
}

export function formatSize(bytes: number): string {
  if (bytes >= 1 << 30) return `${(bytes / (1 << 30)).toFixed(1)}G`
  if (bytes >= 1 << 20) return `${(bytes / (1 << 20)).toFixed(0)}M`
  if (bytes >= 1 << 10) return `${(bytes / (1 << 10)).toFixed(0)}K`
  return `${bytes}B`
}

export function timeAgo(tsSec: number): string {
  const diff = Math.floor(Date.now() / 1000 - tsSec)
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  return `${Math.floor(diff / 86400)} 天前`
}

/** 已录制时长(秒) → "1h 12min" 样式 */
export function formatDuration(sec: number): string {
  if (sec < 60) return `${Math.max(0, Math.floor(sec))}s`
  if (sec < 3600) return `${Math.floor(sec / 60)}min`
  return `${Math.floor(sec / 3600)}h ${Math.floor((sec % 3600) / 60)}min`
}

/**
 * 「当前时间」(Unix 秒)的共享时钟,供渲染期算「已直播多久」这类相对时长。
 * 渲染函数里直接调 Date.now() 不是纯函数(同一份 props 每次渲染结果都不同,React Compiler 拒绝);
 * 这里把时间放进一个模块级 store,有订阅者时由定时器每秒推进一次,组件用 useSyncExternalStore 读取。
 * 首个订阅者接入时先校准一次,避免长时间无人订阅后拿到过期的值。
 */
const NOW_TICK_MS = 1000
let nowSec = Math.floor(Date.now() / 1000)
let nowTimer: ReturnType<typeof setInterval> | null = null
const nowListeners = new Set<() => void>()
const readNowSec = () => nowSec
const subscribeNow = (onTick: () => void) => {
  if (nowListeners.size === 0) {
    nowSec = Math.floor(Date.now() / 1000)
    nowTimer = setInterval(() => {
      nowSec = Math.floor(Date.now() / 1000)
      nowListeners.forEach((listener) => listener())
    }, NOW_TICK_MS)
  }
  nowListeners.add(onTick)
  return () => {
    nowListeners.delete(onTick)
    if (nowListeners.size === 0 && nowTimer !== null) {
      clearInterval(nowTimer)
      nowTimer = null
    }
  }
}
export function useNowSec(): number {
  return useSyncExternalStore(subscribeNow, readNowSec, readNowSec)
}

/**
 * 写盘速率(字节/秒) → "1.2 MB/s" / "680 KB/s"。
 * 后端没有采样(null / undefined)时返回 null,由调用方显示「—」。
 */
export function formatRate(bytesPerSec: number | null | undefined): string | null {
  if (bytesPerSec === null || bytesPerSec === undefined || !Number.isFinite(bytesPerSec)) return null
  const b = Math.max(0, bytesPerSec)
  if (b >= 1 << 20) return `${(b / (1 << 20)).toFixed(b >= 10 << 20 ? 0 : 1)} MB/s`
  if (b >= 1 << 10) return `${(b / (1 << 10)).toFixed(0)} KB/s`
  return `${Math.round(b)} B/s`
}

/**
 * 直播间的封面 / 头像同源代理地址。封面在未开播时也尝试本地缓存与平台房间信息。
 * 带上原始地址的短哈希作查询参数:封面换了(新一场直播)浏览器就不会沿用旧缓存。
 */
export function liveImageUrl(
  id: number,
  kind: 'cover' | 'avatar',
  sourceUrl: string | null | undefined
): string | null {
  if (!sourceUrl) return kind === 'cover' ? `${API_BASE}/v1/streamers/${id}/cover?v=thumb1-cached` : null
  let h = 5381
  for (let i = 0; i < sourceUrl.length; i++) h = ((h << 5) + h + sourceUrl.charCodeAt(i)) | 0
  return `${API_BASE}/v1/streamers/${id}/${kind}?v=thumb1-${(h >>> 0).toString(36)}`
}

/**
 * 正在录制的那一路流的同源地址（chunked FLV / MPEG-TS / fMP4），供页面内播放器直接拉取。
 * `snapshotMs`：起播快照回溯多少毫秒的已完成 GOP（播放器要维持多深的缓冲就要多深）；不传给服务端的整个保留窗口。
 * `reconnect`：断开后的自动重连。该直播间 / 进程内的预览连接满了时，服务端对它返回 429 而不是挤掉最早的一路，
 * 免得几个真在看的页面互相挤；用户手动打开 / 重试不带，总能挤进来。
 */
export function livePreviewUrl(id: number, snapshotMs?: number, opts: { reconnect?: boolean } = {}): string {
  const params = new URLSearchParams()
  if (snapshotMs !== undefined) params.set('snapshot_ms', String(Math.max(0, Math.round(snapshotMs))))
  if (opts.reconnect) params.set('reconnect', '1')
  const query = params.toString()
  return `${API_BASE}/v1/streamers/${id}/live${query ? `?${query}` : ''}`
}

/**
 * 浏览器直连模式：让后端向平台新取一条 CDN 直链（不复用录制那条）。
 * 后端对同一房间 5 s 内的重复请求复用上一次结果；播放失败后的重取传 `fresh`，一定拿新 token。
 */
export function fetchLiveUrl(id: number, opts: { fresh?: boolean } = {}): Promise<LiveUrlInfo> {
  const query = opts.fresh ? '?fresh=1' : ''
  return fetcher(`/v1/streamers/${id}/live-url${query}`, { cache: 'no-store' })
}

/**
 * CDN 直链在页面里能不能直接用：https 页面拉 http 直链会被当作混合内容拦掉（抖音的直链就是 http），
 * 实测这些 CDN 都支持 https，直接换协议。
 */
export function directPlayableUrl(url: string): string {
  if (typeof window !== 'undefined' && window.location.protocol === 'https:' && url.startsWith('http://')) {
    return 'https://' + url.slice('http://'.length)
  }
  return url
}

/**
 * 全局配置里的预览取流方式。与空间配置页共用同一个 SWR key，改完保存这里立刻拿到；
 * 配置还没加载到时按 relay 处理（默认值）。
 */
export function usePreviewTransport(): PreviewTransport {
  const { data } = useSWR<{ preview_transport?: PreviewTransport | null }>('/v1/configuration', fetcher, {
    refreshInterval: SLOW_REFRESH_MS,
  })
  return data?.preview_transport === 'direct' ? 'direct' : 'relay'
}

/**
 * 卡片 / 监视器能否起播：正在录制、下载器能旁路、容器已确定。
 * 容器未定（刚开始拉流的前几秒）时按钮先禁用，下一次轮询拿到 format 再放开。
 */
export function canPreview(streamer: LiveStreamerEntity): streamer is LiveStreamerEntity & {
  preview: { available: true; format: 'flv' | 'mpegts' | 'fmp4' }
} {
  const format = streamer.preview?.format
  return (
    streamer.status === LIVE_STATUS &&
    !!streamer.preview?.available &&
    (format === 'flv' || format === 'mpegts' || format === 'fmp4')
  )
}

/** 容器短名 → 界面标签 */
export function previewFormatLabel(format: 'flv' | 'mpegts' | 'fmp4' | null | undefined): string | null {
  if (format === 'flv') return 'FLV'
  if (format === 'mpegts') return 'TS'
  if (format === 'fmp4') return 'fMP4'
  return null
}

/** 预览按钮禁用时的提示文案；能预览时返回 null。 */
export function previewDisabledReason(streamer: LiveStreamerEntity): string | null {
  if (streamer.status !== LIVE_STATUS) return '未在录制'
  const preview = streamer.preview
  if (!preview) return '预览信息尚未就绪'
  if (!preview.available) return preview.reason || '当前下载器不支持预览'
  if (!preview.format) return '正在建立预览，请稍候'
  return null
}

export const STREAMERS_REFRESH_MS = 10000
export const SLOW_REFRESH_MS = 60000

interface OverviewData {
  total: number
  recording: number
  pending: number
  uploading: number
  streamers: LiveStreamerEntity[]
  infos: StreamerInfo[]
  version: string
}

export function useDashboard() {
  const { data, error, isLoading } = useSWR<OverviewData>('/v1/overview', fetcher, {
    refreshInterval: STREAMERS_REFRESH_MS,
  })
  const { data: activity, error: activityError } = useSWR<{ events: DashboardEvent[] }>(
    '/v1/activity-events?limit=12', fetcher, { refreshInterval: SLOW_REFRESH_MS }
  )
  return {
    streamers: data?.streamers ?? [],
    version: data?.version,
    total: data?.total ?? 0,
    recording: data?.recording ?? 0,
    pending: data?.pending ?? 0,
    uploading: data?.uploading ?? 0,
    infoByUrl: new Map((data?.infos ?? []).map(info => [info.url, info])),
    events: activity?.events ?? [],
    activityFailed: !!activityError,
    loading: isLoading,
    connectError: !!error && !data,
    stale: !!error && !!data,
  }
}
