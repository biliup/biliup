'use client'
import { useSyncExternalStore } from 'react'
import { API_BASE, handleResponse } from './api-streamer'

/**
 * 控制台首页的系统状态（CPU / 内存 / 录制目录磁盘 / 网速），数据来自 `GET /v1/system-stats`。
 *
 * - 服务端常驻采样（默认每 2 s 一次，保留最近 5 分钟）；这里有订阅者时按服务端给的 `interval_ms`
 *   轮询，第一次拿全部历史，之后带 `since=<最后一个采样的 ts>` 只拿增量。
 * - 没有订阅者或标签页切到后台就停，回前台立刻补拉（服务端留着历史，曲线不断）。
 * - 横轴用服务端的采样时刻：它严格递增，与浏览器时钟无关。
 * - 用 useSyncExternalStore 订阅，快照只在收到新数据或出错时换引用。
 */

/** 与后端 `Sample` 一致 */
export interface SystemSample {
  /** 采样时刻，Unix 毫秒 */
  ts: number
  /** 整机 CPU 占用，0–100 */
  cpu: number
  /** 下行字节/秒 */
  rx: number
  /** 上行字节/秒 */
  tx: number
}

export interface SystemMemory {
  used: number
  total: number
  /** 报的是容器（cgroup）上限而不是整机内存 */
  limited: boolean
}

export interface SystemDisk {
  /** 录制目录（服务进程的工作目录） */
  path: string
  total: number
  used: number
  /** 本进程还能写入的空间 */
  available: number
}

export interface SystemCpu {
  logical: number
  physical: number | null
}

/** 与后端 `SystemStats` 一致 */
export interface SystemStatsResponse {
  ts: number
  interval_ms: number
  history_ms: number
  cpu: SystemCpu | null
  memory: SystemMemory | null
  disk: SystemDisk | null
  interfaces: string[]
  samples: SystemSample[]
}

export interface SystemStatsSnapshot {
  /** 每次更新加一 */
  version: number
  /** 最近一次成功响应（不含 samples）；尚未拿到为 null */
  latest: Omit<SystemStatsResponse, 'samples'> | null
  /** 最近 `history_ms` 内的采样，按时间升序；每次更新换新数组 */
  samples: readonly SystemSample[]
  /** 服务端最新采样已落后当前时刻好几个周期（采样任务停了） */
  stalled: boolean
  /** 最近一次拉取失败的原因；成功后清空 */
  error: string | null
}

/** 服务端没给间隔时的默认值，与后端 `SAMPLE_INTERVAL` 一致 */
const DEFAULT_INTERVAL_MS = 2000
const DEFAULT_HISTORY_MS = 5 * 60 * 1000
/** 拉取失败后的重试间隔 */
const RETRY_MS = 5000
/** 最新采样落后服务端时刻超过这么多个周期，就当采样已停 */
const STALL_PERIODS = 5

const EMPTY: SystemStatsSnapshot = { version: 0, latest: null, samples: [], stalled: false, error: null }
let snapshot: SystemStatsSnapshot = EMPTY
const listeners = new Set<() => void>()
let timer: ReturnType<typeof setTimeout> | null = null
let inflight: AbortController | null = null
let visibilityBound = false

function emit() {
  listeners.forEach((l) => l())
}

/** 合并一次响应：按 ts 去重追加，裁到服务端保留的时长。 */
export function ingestSystemStats(res: SystemStatsResponse) {
  const { samples: incoming, ...latest } = res
  const last = snapshot.samples.length > 0 ? snapshot.samples[snapshot.samples.length - 1].ts : -Infinity
  const merged = snapshot.samples.concat(incoming.filter((s) => s.ts > last))
  const newest = merged.length > 0 ? merged[merged.length - 1].ts : 0
  const keepFrom = newest - (latest.history_ms || DEFAULT_HISTORY_MS)
  const samples = merged.filter((s) => s.ts >= keepFrom)
  const interval = latest.interval_ms || DEFAULT_INTERVAL_MS
  const stalled = newest > 0 && latest.ts - newest > interval * STALL_PERIODS
  snapshot = { version: snapshot.version + 1, latest, samples, stalled, error: null }
  emit()
}

function markError(message: string) {
  if (snapshot.error === message) return
  snapshot = { ...snapshot, version: snapshot.version + 1, error: message }
  emit()
}

function active(): boolean {
  return listeners.size > 0 && !(typeof document !== 'undefined' && document.visibilityState === 'hidden')
}

async function pollOnce() {
  timer = null
  inflight?.abort()
  const controller = new AbortController()
  inflight = controller
  const samples = snapshot.samples
  const since = samples.length > 0 ? `?since=${samples[samples.length - 1].ts}` : ''
  let ok = false
  try {
    const res = await fetch(`${API_BASE}/v1/system-stats${since}`, { cache: 'no-store', signal: controller.signal })
    await handleResponse(res)
    const body = (await res.json()) as SystemStatsResponse
    if (controller.signal.aborted) return
    ingestSystemStats(body)
    ok = true
  } catch (e) {
    if (controller.signal.aborted) return
    markError(e instanceof Error ? e.message : String(e))
  } finally {
    if (inflight === controller) inflight = null
  }
  schedule(ok ? snapshot.latest?.interval_ms || DEFAULT_INTERVAL_MS : RETRY_MS)
}

function schedule(delay: number) {
  if (timer) clearTimeout(timer)
  timer = null
  if (!active()) return
  timer = setTimeout(pollOnce, delay)
}

function stop() {
  if (timer) clearTimeout(timer)
  timer = null
  inflight?.abort()
  inflight = null
}

function onVisibilityChange() {
  if (document.visibilityState === 'hidden') stop()
  else if (listeners.size > 0 && !timer && !inflight) schedule(0)
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  if (typeof document !== 'undefined' && !visibilityBound) {
    document.addEventListener('visibilitychange', onVisibilityChange)
    visibilityBound = true
  }
  // 首个订阅者：下一拍再拉（StrictMode 的订阅 → 退订 → 再订阅只会拉一次）
  if (listeners.size === 1 && !timer && !inflight) schedule(0)
  return () => {
    listeners.delete(listener)
    if (listeners.size === 0) stop()
  }
}

const getSnapshot = () => snapshot
const getServerSnapshot = () => EMPTY

export function useSystemStats(): SystemStatsSnapshot {
  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot)
}

/** 画图用的序列：x 为 Unix 秒；相邻采样间隔超过 2.5 个周期处插一个 null，画成断口 */
export interface SystemSeries {
  xs: number[]
  cpu: (number | null)[]
  rx: (number | null)[]
  tx: (number | null)[]
}

export function toSeries(samples: readonly SystemSample[], intervalMs: number): SystemSeries {
  const out: SystemSeries = { xs: [], cpu: [], rx: [], tx: [] }
  const gap = intervalMs * 2.5
  let prev: number | null = null
  for (const s of samples) {
    if (prev !== null && s.ts - prev > gap) {
      out.xs.push((prev + intervalMs) / 1000)
      out.cpu.push(null)
      out.rx.push(null)
      out.tx.push(null)
    }
    out.xs.push(s.ts / 1000)
    out.cpu.push(s.cpu)
    out.rx.push(s.rx)
    out.tx.push(s.tx)
    prev = s.ts
  }
  return out
}

/** 字节数 → "12.3 GB" / "512 MB"（1024 进制，与 df -h 一致） */
export function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB']
  let v = Math.max(0, bytes)
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i += 1
  }
  return `${i === 0 || v >= 100 ? v.toFixed(0) : v.toFixed(1)} ${units[i]}`
}

/** 已用比例（0–100）；磁盘按 used / (used + available)，与 df 同口径 */
export function diskPercent(disk: SystemDisk): number {
  const denom = disk.used + disk.available
  return denom > 0 ? (disk.used / denom) * 100 : 0
}

export function memoryPercent(memory: SystemMemory): number {
  return memory.total > 0 ? (memory.used / memory.total) * 100 : 0
}
