/**
 * 中转直播流的缓冲控制：播放器该维持多深的缓冲、落后了怎么追。
 *
 * 中转流的「缓冲深度」就是用户看到的延迟（服务端收到 → 画面播出），两者只差一个链路传输时间：
 * 服务端按实时节奏转发，播放器攒着不放的那几秒既是抵消抖动的余量，也是延迟。
 * 所以起播时向服务端要多深的快照（`snapshot_ms`）、稳态维持多深的缓冲，是同一个数。
 *
 * 两个档位：`low` 低延迟——够吃掉几百毫秒的到达抖动（模拟 60 ms RTT ± 16 ms、丢包重传停顿的链路上
 * 1 s 都不卡，2 s 留一倍余量）；`smooth` 流畅——链路差（RTT 高、重传多、带宽勉强够码率）时用，
 * 多 3 s 延迟换零 waiting。低延迟档卡了会自动升到流畅档。
 * HLS 来的流（fMP4 / TS）另算：服务端是整个分片到一坨，缓冲至少得盖住一个分片加取片抖动
 * （B 站 hls_fmp4 分片 1–3 s），低延迟档给 4 s。
 * 直连 CDN 的流不走这里（CDN 自带 GOP 缓存，mpegts.js 自己的追帧参数够用）。
 */
export interface LiveBufferPolicy {
  /** 目标缓冲 = 中转延迟，秒 */
  target: number
  /** 落后到这以上用倍速慢慢追回目标（不跳，跳一次会把画面顿一下） */
  max: number
  /** 落后到这以上（长时间暂停、后台标签页）直接跳到目标位置 */
  hard: number
  /** 追帧倍速 */
  rate: number
}

export type LatencyProfile = 'low' | 'smooth'

/** 按 tag 到达的流（FLV）：低延迟 2 s / 流畅 5 s */
export const RELAY_PROFILES: Record<LatencyProfile, LiveBufferPolicy> = {
  low: { target: 2, max: 3.5, hard: 12, rate: 1.1 },
  smooth: { target: 5, max: 7, hard: 20, rate: 1.1 },
}

/** 按 HLS 分片到达的流（fMP4 / TS）：低延迟也得盖住一个分片 */
export const RELAY_PROFILES_SEGMENTED: Record<LatencyProfile, LiveBufferPolicy> = {
  low: { target: 4, max: 6, hard: 14, rate: 1.1 },
  smooth: RELAY_PROFILES.smooth,
}

/** 该档位在该容器下的缓冲策略；容器未定按 FLV */
export function relayPolicy(profile: LatencyProfile, format: string | null | undefined): LiveBufferPolicy {
  return format === 'fmp4' || format === 'mpegts' ? RELAY_PROFILES_SEGMENTED[profile] : RELAY_PROFILES[profile]
}

/** 播放器要维持 target 秒缓冲，起播快照就要 target 秒（服务端给到 target 到 target + 一个 GOP） */
export function snapshotMsFor(policy: LiveBufferPolicy): number {
  return Math.round(policy.target * 1000)
}

/**
 * 起播对齐后再看几秒：快照是一坨突发到达的，落地要一两秒（几 MB 在慢链路上更久），这期间缓冲还在长，
 * 超了就再对齐一次；每对齐一次再多看 2 s，最多看 15 s，之后只做倍速 / 跳的稳态控制
 */
const SETTLE_MS = 3000
const SETTLE_EXTEND_MS = 2000
const SETTLE_MAX_MS = 15000
/** 缓冲比目标多出这么多才值得再对齐一次（起播阶段） */
const ALIGN_SLACK_S = 1
const TICK_MS = 250

export interface StallInfo {
  /** 卡住了多久，秒 */
  seconds: number
  /** 卡住那一刻的缓冲余量，秒（真正的缓冲耗尽接近 0） */
  bufferAhead: number
}

export interface LiveBufferHooks {
  /** 稳态期间（起播对齐并稳定之后）一次 waiting → playing，告诉上层卡了多久 */
  onStall?: (info: StallInfo) => void
}

function liveEdge(video: HTMLVideoElement): { start: number; end: number } | null {
  const b = video.buffered
  if (!b.length) return null
  return { start: b.start(b.length - 1), end: b.end(b.length - 1) }
}

/**
 * 把缓冲控制挂到 `<video>` 上：起播把播放位置对齐到「末尾 − target」，之后每 250 ms 看一次落后量，
 * 超 `max` 倍速、超 `hard` 跳、掉队重对齐留下的缓冲空洞直接跳过。返回卸载函数。
 * mpegts.js 的 `liveSync` / `liveBufferLatencyChasing` 与 fMP4 的 MediaSource 播放器都改用这一套，
 * 两种容器的中转流行为一致，档位也能在播放中途改（`getPolicy` 每次 tick 现取）。
 */
export function attachLiveBufferControl(
  video: HTMLVideoElement,
  getPolicy: () => LiveBufferPolicy,
  hooks: LiveBufferHooks = {}
): () => void {
  let disposed = false
  let aligned = false
  let alignedAt = 0
  let settleUntil = 0
  let lastAlignAt = 0
  let stallStartedAt: number | null = null
  let stallAhead = 0

  const seekTo = (t: number) => {
    lastAlignAt = performance.now()
    video.currentTime = t
  }

  const tick = () => {
    if (disposed) return
    const edge = liveEdge(video)
    if (!edge) return
    const policy = getPolicy()
    const now = performance.now()
    const latency = edge.end - video.currentTime

    if (!aligned) {
      // 起播：跳到「末尾 − target」（fMP4 的时间戳从一个很大的值起步，不跳进缓冲区根本放不起来）；
      // 快照还没到齐就先从有的地方放，接下来几秒里缓冲长过头了再对齐
      aligned = true
      alignedAt = now
      settleUntil = now + SETTLE_MS
      seekTo(Math.max(edge.start, edge.end - policy.target))
      return
    }
    if (now < settleUntil) {
      if (latency > policy.target + ALIGN_SLACK_S && now - lastAlignAt >= 500) {
        seekTo(edge.end - policy.target)
        settleUntil = Math.min(alignedAt + SETTLE_MAX_MS, now + SETTLE_EXTEND_MS)
      }
      return
    }

    // 掉队重对齐后时间戳前跳，缓冲区出现空洞：停在空洞前沿就跳到下一段的起点
    const b = video.buffered
    for (let i = 0; i + 1 < b.length; i++) {
      if (video.currentTime >= b.end(i) - 0.3 && video.currentTime < b.start(i + 1) && video.readyState < 3) {
        seekTo(b.start(i + 1) + 0.05)
        return
      }
    }
    if (video.paused) return
    if (latency > policy.hard) {
      seekTo(edge.end - policy.target)
      if (video.playbackRate !== 1) video.playbackRate = 1
    } else if (latency > policy.max) {
      if (video.playbackRate !== policy.rate) video.playbackRate = policy.rate
    } else if (latency <= policy.target && video.playbackRate !== 1) {
      video.playbackRate = 1
    }
  }

  const onWaiting = () => {
    // 只统计稳态期间真正的缓冲耗尽；对齐 / 跳空洞引起的 seeking 不算
    if (disposed || !aligned || performance.now() < settleUntil || video.seeking) return
    const edge = liveEdge(video)
    stallStartedAt = performance.now()
    stallAhead = edge ? Math.max(0, edge.end - video.currentTime) : 0
  }
  const onPlaying = () => {
    if (stallStartedAt == null) return
    const seconds = (performance.now() - stallStartedAt) / 1000
    stallStartedAt = null
    hooks.onStall?.({ seconds, bufferAhead: stallAhead })
  }
  video.addEventListener('waiting', onWaiting)
  video.addEventListener('playing', onPlaying)
  const timer = setInterval(tick, TICK_MS)
  return () => {
    disposed = true
    clearInterval(timer)
    video.removeEventListener('waiting', onWaiting)
    video.removeEventListener('playing', onPlaying)
  }
}
