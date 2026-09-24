'use client'
import { useSyncExternalStore } from 'react'
import { API_BASE, handleResponse, revalidateMe } from './api-streamer'

/**
 * 录制中各房间的写盘速率采样：页面内唯一的一份数据源，供弹层折线图、卡片与监视器的 sparkline 共用。
 *
 * - 有订阅者时连一条 WebSocket `GET /v1/ws/live-rates`，服务端每秒推一帧（只含录制中房间的
 *   `{id, bytes_per_sec, ts}`，只读内存）；WebSocket 连不上（反代没放行 upgrade）就回退到每秒
 *   `GET /v1/live-rates` 轮询。没有订阅者就断开，标签页切到后台也断，回前台立刻重连。**不调快 `/v1/streamers`**。
 *   选 WebSocket 而不是 SSE：WebSocket 不占浏览器对同一主机 HTTP/1.1 的六个并发连接，
 *   监视器 4 路视频 + 1 条弹幕 SSE 已经用掉五个。
 * - 历史放在模块级的环形缓冲里（每房间最近 3 分钟），组件卸载不清：切走再切回来曲线还在，
 *   中间没采样的那段是空档（null），画成断口。
 * - 用 useSyncExternalStore 订阅：快照只在收到新一帧时换引用，无采样变化时组件不重渲染。
 */

/** 与后端 `LiveRateFrame` 一致 */
export interface LiveRateFrame {
  id: number
  /** 字节/秒；尚无采样 / 下载器不经本进程写盘时为 null */
  bytes_per_sec: number | null
  /** 服务端读数时刻，Unix 毫秒 */
  ts: number
}

/** 回退轮询的间隔（毫秒）；WebSocket 推送也是 1 s 一帧。后端 RateMeter 本身 1 s 采一次，再快没有新信息。 */
export const LIVE_RATES_POLL_MS = 1000
/** 浏览器里保留的历史长度（毫秒） */
export const LIVE_RATES_HISTORY_MS = 3 * 60 * 1000
/** 每房间环形缓冲容量：3 分钟 × 1 Hz，再留些余量给略快于 1 s 的轮询 */
const RING_CAPACITY = 240
/** 拉取失败 / WebSocket 断开后的退避基数 */
const RETRY_MS = 3000

/** 一段用于画图的序列：x 为 Unix 秒（uPlot 时间轴单位），y 为字节/秒，null 是断口 */
export interface RateSeries {
  xs: number[]
  ys: (number | null)[]
  /** 窗口内最新一个非 null 的值；没有则 null */
  latest: number | null
  /** 窗口内的最大值（用于选单位 / Y 轴上限）；没有则 0 */
  max: number
}

/** 固定容量的环形缓冲，只追加、按时间窗口读出。 */
class RateRing {
  private readonly ts = new Float64Array(RING_CAPACITY)
  private readonly bps = new Float64Array(RING_CAPACITY)
  private start = 0
  private len = 0

  push(ts: number, value: number | null) {
    const i = (this.start + this.len) % RING_CAPACITY
    this.ts[i] = ts
    this.bps[i] = value === null ? NaN : value
    if (this.len < RING_CAPACITY) this.len += 1
    else this.start = (this.start + 1) % RING_CAPACITY
  }

  /** 最后一次写入的时刻（毫秒）；空缓冲为 0 */
  lastTs(): number {
    if (this.len === 0) return 0
    return this.ts[(this.start + this.len - 1) % RING_CAPACITY]
  }

  /** 读出 `[sinceMs, +∞)` 的样本 */
  read(sinceMs: number): RateSeries {
    const xs: number[] = []
    const ys: (number | null)[] = []
    let latest: number | null = null
    let max = 0
    for (let k = 0; k < this.len; k++) {
      const i = (this.start + k) % RING_CAPACITY
      const t = this.ts[i]
      if (t < sinceMs) continue
      const v = this.bps[i]
      xs.push(t / 1000)
      if (Number.isNaN(v)) {
        ys.push(null)
      } else {
        ys.push(v)
        latest = v
        if (v > max) max = v
      }
    }
    return { xs, ys, latest, max }
  }
}

/** 最近一帧的摘要；引用只在收到新帧 / 出错 / 换传输方式时更换 */
export interface LiveRatesSnapshot {
  /** 每收到一帧加一，组件用它决定何时重画 */
  version: number
  /** 最近一帧的服务端时刻（毫秒）；尚未收到为 0 */
  ts: number
  /** 最近一帧到达浏览器的时刻（毫秒，浏览器时钟）；窗口裁剪按它算。尚未收到为 0 */
  receivedAt: number
  /** 最近一帧里各录制中房间的速率 */
  latest: ReadonlyMap<number, number | null>
  /** 最近一次连接 / 拉取是否失败（网络 / 401 等）；成功后清空 */
  error: string | null
  /** 当前的数据来源：WebSocket 推送（首选）或每秒 HTTP 轮询（WebSocket 连不上时的回退）；尚未开始为 null */
  transport: 'ws' | 'poll' | null
}

const rings = new Map<number, RateRing>()
const listeners = new Set<() => void>()
let snapshot: LiveRatesSnapshot = { version: 0, ts: 0, receivedAt: 0, latest: new Map(), error: null, transport: null }
const SERVER_SNAPSHOT: LiveRatesSnapshot = snapshot
let timer: ReturnType<typeof setTimeout> | null = null
let inflight: AbortController | null = null
let socket: WebSocket | null = null
/** WebSocket 从未成功建立过（反代没放行 upgrade、老代理）→ 本页面余下时间用轮询 */
let wsUnusable = false
/** WebSocket 连续失败次数，决定重连退避 */
let wsFailures = 0
let visibilityBound = false

function emit() {
  listeners.forEach((l) => l())
}

/**
 * 把一帧写进各房间的缓冲；帧里没有的房间（停录 / 下播）补一个 null，曲线就此断开。
 * 横轴统一用浏览器时钟（收到的时刻）：窗口裁剪也按同一时钟算，服务端与浏览器时钟有偏差时曲线不漂。
 */
export function ingestFrames(frames: LiveRateFrame[], receivedAt = Date.now()) {
  const latest = new Map<number, number | null>()
  for (const f of frames) {
    let ring = rings.get(f.id)
    if (!ring) {
      ring = new RateRing()
      rings.set(f.id, ring)
    }
    ring.push(receivedAt, f.bytes_per_sec)
    latest.set(f.id, f.bytes_per_sec)
  }
  for (const [id, ring] of rings) {
    if (latest.has(id)) continue
    if (receivedAt - ring.lastTs() > LIVE_RATES_HISTORY_MS) {
      rings.delete(id)
    } else {
      ring.push(receivedAt, null)
    }
  }
  const ts = frames.length > 0 ? frames[0].ts : receivedAt
  snapshot = { ...snapshot, version: snapshot.version + 1, ts, receivedAt, latest, error: null }
  emit()
}

function markError(message: string) {
  if (snapshot.error === message) return
  snapshot = { ...snapshot, version: snapshot.version + 1, error: message }
  emit()
}

function setTransport(transport: LiveRatesSnapshot['transport']) {
  if (snapshot.transport === transport) return
  snapshot = { ...snapshot, version: snapshot.version + 1, transport }
  emit()
}

/** 与日志页同一套推导：生产同源，开发模式指向 NEXT_PUBLIC_API_SERVER */
function liveRatesSocketUrl(): string {
  if (API_BASE) return `${API_BASE.replace(/^http/, 'ws')}/v1/ws/live-rates`
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${protocol}//${window.location.host}/v1/ws/live-rates`
}

function active(): boolean {
  return listeners.size > 0 && !(typeof document !== 'undefined' && document.visibilityState === 'hidden')
}

/** 首选：一条 WebSocket，服务端每秒推一帧。 */
function connectSocket() {
  timer = null
  if (socket || !active()) return
  let ws: WebSocket
  try {
    ws = new WebSocket(liveRatesSocketUrl())
  } catch (e) {
    wsUnusable = true
    markError(e instanceof Error ? e.message : String(e))
    schedule(0)
    return
  }
  socket = ws
  let gotFrame = false
  ws.onmessage = (event: MessageEvent<string>) => {
    let frames: LiveRateFrame[]
    try {
      frames = JSON.parse(event.data)
    } catch {
      return
    }
    if (!Array.isArray(frames)) return
    gotFrame = true
    wsFailures = 0
    if (snapshot.transport !== 'ws') setTransport('ws')
    ingestFrames(frames)
  }
  ws.onclose = () => {
    if (socket !== ws) return
    socket = null
    if (!active()) return
    if (!gotFrame) {
      // 一帧都没收到就断了：反代不支持 upgrade、401、403……这个页面里不再试 WebSocket
      wsUnusable = true
      markError('WebSocket 不可用，改为每秒轮询')
      schedule(0)
      return
    }
    // 中途断开（服务重启 / 网络抖动 / 会话被收回）：退避重连，期间没有新帧，曲线出现断口。
    // 会话失效时后端也会主动断开，刷新一次权限点：失效就跳登录页，降级就收起页面
    revalidateMe()
    wsFailures += 1
    timer = setTimeout(connectSocket, Math.min(RETRY_MS * wsFailures, 10_000))
  }
  ws.onerror = () => {
    /* 紧接着会触发 onclose，在那里统一处理 */
  }
}

/** 回退：每秒 GET 一次。 */
async function pollOnce() {
  inflight?.abort()
  const controller = new AbortController()
  inflight = controller
  let ok = false
  try {
    const res = await fetch(`${API_BASE}/v1/live-rates`, { cache: 'no-store', signal: controller.signal })
    // 与其它接口同一套处理：会话失效（401）跳登录页，失去权限（403）刷新权限点、页面随之收起
    await handleResponse(res)
    const frames = (await res.json()) as LiveRateFrame[]
    if (controller.signal.aborted) return
    if (snapshot.transport !== 'poll') setTransport('poll')
    ingestFrames(Array.isArray(frames) ? frames : [])
    ok = true
  } catch (e) {
    if (controller.signal.aborted) return
    markError(e instanceof Error ? e.message : String(e))
  } finally {
    if (inflight === controller) inflight = null
  }
  schedule(ok ? LIVE_RATES_POLL_MS : RETRY_MS)
}

/** 有订阅者且页面可见时，按当前传输方式开始 / 继续取数。 */
function schedule(delay: number) {
  if (timer) clearTimeout(timer)
  timer = null
  if (!active()) return
  if (!wsUnusable && typeof WebSocket !== 'undefined') {
    timer = setTimeout(connectSocket, delay)
  } else {
    timer = setTimeout(pollOnce, delay)
  }
}

function stop() {
  if (timer) clearTimeout(timer)
  timer = null
  inflight?.abort()
  inflight = null
  if (socket) {
    const ws = socket
    socket = null
    ws.onclose = null
    ws.close()
  }
}

function onVisibilityChange() {
  if (document.visibilityState === 'hidden') stop()
  else if (listeners.size > 0 && !timer && !inflight && !socket) schedule(0)
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  if (typeof document !== 'undefined' && !visibilityBound) {
    document.addEventListener('visibilitychange', onVisibilityChange)
    visibilityBound = true
  }
  // 首个订阅者：下一拍再连（StrictMode 的订阅 → 退订 → 再订阅只会连一次）
  if (listeners.size === 1 && !timer && !inflight && !socket) schedule(0)
  return () => {
    listeners.delete(listener)
    if (listeners.size === 0) stop()
  }
}

const getSnapshot = () => snapshot
const getServerSnapshot = () => SERVER_SNAPSHOT

/**
 * 订阅实时速率。挂载即开始（或加入）取数——首选一条 WebSocket（服务端每秒推一帧），
 * 连不上时回退到每秒 HTTP 轮询；全部卸载即断开 / 停止。返回最近一帧的摘要，
 * 序列本身用 {@link readRateSeries} 按窗口读取。
 */
export function useLiveRates(): LiveRatesSnapshot {
  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot)
}

/**
 * 读某房间最近 `windowMs` 的序列（含组件卸载期间留下的历史）。
 * `now` 传快照的 `receivedAt`：同一版本的快照读出的序列相同，渲染期调用也是纯的。
 */
export function readRateSeries(id: number, windowMs: number, now: number): RateSeries {
  const ring = rings.get(id)
  if (!ring) return { xs: [], ys: [], latest: null, max: 0 }
  return ring.read(now - windowMs)
}

/** 测试时清空全部历史（生产代码不调用）。 */
export function resetLiveRates() {
  stop()
  rings.clear()
  wsUnusable = false
  wsFailures = 0
  snapshot = { version: 0, ts: 0, receivedAt: 0, latest: new Map(), error: null, transport: null }
}

/**
 * 按窗口内的最大值选单位：≥ 1 MiB/s 用 MB/s，否则 KB/s。
 * 返回除数与标签，供 Y 轴刻度与 hover 数值共用，同一张图里单位不跳。
 */
export function rateUnit(maxBytesPerSec: number): { div: number; label: string } {
  if (maxBytesPerSec >= 1 << 20) return { div: 1 << 20, label: 'MB/s' }
  return { div: 1 << 10, label: 'KB/s' }
}

/** 用给定单位格式化一个值（刻度 / hover 用），小于 10 保留一位小数。 */
export function formatInUnit(bytesPerSec: number, unit: { div: number; label: string }): string {
  const v = bytesPerSec / unit.div
  return `${v >= 10 || v === 0 ? v.toFixed(0) : v.toFixed(1)} ${unit.label}`
}
