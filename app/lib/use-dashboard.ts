'use client'
import useSWR from 'swr'
import { fetcher, LiveStreamerEntity, StreamerInfo, FileList } from './api-streamer'
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
 * 设计说明:后端没有"错误/磁盘占用"等指标,因此控制台不展示虚构数据,
 * KPI 全部来自上述接口的真实聚合。
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
