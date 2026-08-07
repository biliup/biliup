'use client'
import useSWR from 'swr'
import { fetcher, LiveStreamerEntity, StreamerInfo, FileList } from './api-streamer'

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

export function useDashboard() {
  // 分层轮询:状态秒级刷新,静态信息(标题/文件)30s 足够
  const { data: streamers, error: e1 } = useSWR<LiveStreamerEntity[]>(
    '/v1/streamers',
    fetcher,
    { refreshInterval: 4000, revalidateOnFocus: true }
  )
  const { data: infos, error: e2 } = useSWR<StreamerInfo[]>(
    '/v1/streamer-info',
    fetcher,
    { refreshInterval: 30000 }
  )
  const { data: videos, error: e3 } = useSWR<FileList[]>(
    '/v1/videos',
    fetcher,
    { refreshInterval: 30000 }
  )
  const { data: status, error: e4 } = useSWR(
    '/v1/status',
    fetcher,
    { refreshInterval: 30000 }
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
  const seenUrl = new Set<string>()
  for (const i of infos ?? []) {
    if (!i.url || seenUrl.has(i.url)) continue
    seenUrl.add(i.url)
    if (i.date && now - i.date <= EVENT_WINDOW) {
      events.push({
        ts: i.date,
        kind: 'start',
        text: `${i.name || i.url} 开始录制`,
        sub: i.url ? platformNameOf(i.url) : undefined,
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

  // ---- 错误与可达性(与 PR 首页逻辑一致:空数组 ≠ 不可达) ----
  const hasAnyResponse =
    streamers !== undefined || infos !== undefined || videos !== undefined || status !== undefined
  const loading = !hasAnyResponse && !e1 && !e2 && !e3 && !e4
  const connectError = !hasAnyResponse && (!!e1 || !!e2 || !!e3 || !!e4)
  const streamersFailed = !!e1 && streamers === undefined
  const infosFailed = !!e2 && infos === undefined
  const videosFailed = !!e3 && videos === undefined

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
  }
}

/** url → 平台短名(供事件流使用,避免引 component 依赖) */
function platformNameOf(url: string): string {
  const lower = url.toLowerCase()
  if (lower.includes('bilibili.com') || lower.includes('b23.tv')) return '哔哩哔哩'
  if (lower.includes('huya.com')) return '虎牙'
  if (lower.includes('douyu.com')) return '斗鱼'
  if (lower.includes('youtube.com') || lower.includes('youtu.be')) return 'YouTube'
  if (lower.includes('twitch.tv')) return 'Twitch'
  if (lower.includes('cc.163.com')) return 'CC'
  if (lower.includes('kuaishou.com')) return '快手'
  if (lower.includes('douyin.com')) return '抖音'
  return '直播源'
}
