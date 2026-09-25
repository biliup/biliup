'use client'
import { useSyncExternalStore } from 'react'
import useSWR from 'swr'
import { API_BASE, fetcher, LiveStreamerEntity, StreamerInfo, FileList, LiveUrlInfo, PreviewTransport } from './api-streamer'
import { platformName } from './status'

/**
 * 控制台数据聚合 hook —— 纯前端实现,零后端改动。
 *
 * 数据源(全部为现有后端接口):
 *  - /v1/streamers      主播与实时状态(轮询快:4s)
 *  - /v1/streamer-info  直播标题 / 开始时间(轮询慢:30s)
 *  - /v1/videos         录制文件列表(轮询慢:30s,用于文件总量 / 今日新增 / 事件流)
 *  - /v1/status         服务版本等(轮询慢:30s,仅用于可达性)
 *
 * 设计说明:控制台不展示虚构数据,KPI 全部来自上述接口的真实聚合;
 * CPU / 内存 / 磁盘 / 网速由 use-system-stats 单独轮询 /v1/system-stats。
 */

/** 直播中(后端 WorkerStatus Debug 字符串,已验证与 WorkerStatus 枚举一致) */
export const LIVE_STATUS = 'Working'
export const PAUSE_STATUS = 'Pause'

export interface DashboardEvent {
  /** Unix 秒 */
  ts: number
  kind: 'start' | 'file'
  /** 主文本 */
  text: string
  /** 右侧小标签(平台 / 大小) */
  sub?: string
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
 * 录制中直播间的封面 / 头像同源代理地址。
 * 带上原始地址的短哈希作查询参数:封面换了(新一场直播)浏览器就不会沿用旧缓存。
 */
export function liveImageUrl(
  id: number,
  kind: 'cover' | 'avatar',
  sourceUrl: string | null | undefined
): string | null {
  if (!sourceUrl) return null
  let h = 5381
  for (let i = 0; i < sourceUrl.length; i++) h = ((h << 5) + h + sourceUrl.charCodeAt(i)) | 0
  return `${API_BASE}/v1/streamers/${id}/${kind}?v=${(h >>> 0).toString(36)}`
}

/**
 * 正在录制的那一路流的同源地址（chunked FLV / MPEG-TS / fMP4），供页面内播放器直接拉取。
 * `snapshotMs`：起播快照回溯多少毫秒的已完成 GOP（播放器要维持多深的缓冲就要多深）；不传给服务端的整个保留窗口。
 */
export function livePreviewUrl(id: number, snapshotMs?: number): string {
  const base = `${API_BASE}/v1/streamers/${id}/live`
  return snapshotMs === undefined ? base : `${base}?snapshot_ms=${Math.max(0, Math.round(snapshotMs))}`
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

const DAY = 86400
const EVENT_WINDOW = DAY // 事件流只展示最近 24h

/**
 * 轮询间隔。/v1/streamer-info 是全表查询、/v1/videos 每次扫一遍工作目录，
 * 长期运行的实例上这两个接口不便宜，所以只有主播状态走较快的节奏。
 */
export const STREAMERS_REFRESH_MS = 10000
export const SLOW_REFRESH_MS = 60000

export function useDashboard() {
  const { data: streamers, error: e1 } = useSWR<LiveStreamerEntity[]>(
    '/v1/streamers',
    fetcher,
    { refreshInterval: STREAMERS_REFRESH_MS, revalidateOnFocus: true }
  )
  const { data: infos, error: e2 } = useSWR<StreamerInfo[]>(
    '/v1/streamer-info',
    fetcher,
    { refreshInterval: SLOW_REFRESH_MS }
  )
  const { data: videos, error: e3 } = useSWR<FileList[]>(
    '/v1/videos',
    fetcher,
    { refreshInterval: SLOW_REFRESH_MS }
  )
  const { data: status, error: e4 } = useSWR(
    '/v1/status',
    fetcher,
    { refreshInterval: SLOW_REFRESH_MS }
  )

  // ---- KPI:主播与任务状态 ----
  const list = streamers ?? []
  const total = list.length
  const recording = list.filter((s) => s.status === LIVE_STATUS).length
  const pending = list.filter((s) => s.upload_status === 'Pending').length
  const uploading = list.filter((s) => s.upload_status === LIVE_STATUS).length

  // ---- KPI:录制文件聚合(来自 /v1/videos) ----
  // "今日"按本地时区的自然日 0 点起算
  const nowLocal = new Date()
  const todayStart = Math.floor(
    new Date(nowLocal.getFullYear(), nowLocal.getMonth(), nowLocal.getDate()).getTime() / 1000
  )
  let totalSize = 0
  let todaySize = 0
  for (const v of videos ?? []) {
    totalSize += v.size || 0
    if ((v.updateTime || 0) >= todayStart) todaySize += v.size || 0
  }

  // ---- url → 最新 StreamerInfo(标题 / 最近开始时间) ----
  const infoByUrl = new Map<string, StreamerInfo>()
  for (const i of infos ?? []) {
    if (!i.url) continue
    const cur = infoByUrl.get(i.url)
    if (!cur || i.date > cur.date) infoByUrl.set(i.url, i)
  }

  // ---- 事件流:前端合成,零后端改动 ----
  // 1) 开始录制:streamer-info 里 24h 内的直播开始时间(按 url 合并取最新)
  // 2) 文件生成:/v1/videos 里 24h 内生成的录制文件
  const events: DashboardEvent[] = []
  const now = Math.floor(Date.now() / 1000)
  for (const i of Array.from(infoByUrl.values())) {
    if (i.date && now - i.date <= EVENT_WINDOW) {
      events.push({
        ts: i.date,
        kind: 'start',
        text: `${i.name || i.url} 开始录制`,
        sub: i.url ? platformName(i.url) : undefined,
      })
    }
  }
  for (const v of videos ?? []) {
    const t = v.updateTime || 0
    if (t && now - t <= EVENT_WINDOW) {
      events.push({
        ts: t,
        kind: 'file',
        text: v.name || '录制文件',
        sub: formatSize(v.size || 0),
      })
    }
  }
  events.sort((a, b) => b.ts - a.ts)

  // ---- 错误与可达性 ----
  // 只有四个接口全部失败才判定为「后端不可达」；单个接口失败（尤其是序列化整份配置的
  // /v1/status）只降级对应区块，不能把主播卡片整页替换成断连提示。
  const hasAnyResponse =
    streamers !== undefined || infos !== undefined || videos !== undefined || status !== undefined
  const hasAnyError = !!e1 || !!e2 || !!e3 || !!e4
  const allFailed = !!e1 && !!e2 && !!e3 && !!e4
  const loading = !hasAnyResponse && !hasAnyError
  const connectError = allFailed || (!hasAnyResponse && hasAnyError)
  const streamersFailed = !!e1
  const infosFailed = !!e2
  const videosFailed = !!e3
  const statusFailed = !!e4

  return {
    streamers,
    infos,
    videos,
    version: (status as { version?: string } | undefined)?.version,
    // KPI
    total,
    recording,
    pending,
    uploading,
    totalSize,
    todaySize,
    infoByUrl,
    events: events.slice(0, 12),
    // 状态
    loading,
    connectError,
    streamersFailed,
    infosFailed,
    videosFailed,
    statusFailed,
  }
}
