'use client'
import { useEffect, useMemo } from 'react'
import { API_BASE } from './api-streamer'

/** SSE 里一条弹幕的形状，与后端 `DanmakuFrame` 一致 */
export interface DanmakuFrame {
  /** 直播间 id：一条 SSE 可能复用给多个直播间 */
  id: number
  kind: 'danmaku' | 'gift' | 'super_chat' | 'guard_buy'
  text: string
  name: string | null
  /** RGB 整数（白 16777215） */
  color: number
  /** Unix 毫秒 */
  ts: number
}

const KINDS: DanmakuFrame['kind'][] = ['danmaku', 'gift', 'super_chat', 'guard_buy']

/**
 * 若干直播间共用的一条实时弹幕连接。
 *
 * 浏览器对同一主机的 HTTP/1.1 并发连接只有 6 个，监视器同屏 N 路视频已经占了 N 个，
 * 弹幕不能再每路一条——所以由页面持有一个 feed（`/v1/danmaku?ids=…`，一条 SSE），
 * 各播放器按 id 订阅自己那一路。单路弹层同样走这里（ids 只有一个）。
 */
export class DanmakuFeed {
  private source: EventSource | null = null
  private active = false
  private listeners = new Map<number, Set<(frame: DanmakuFrame) => void>>()

  /** 构造不产生副作用；`open()` 之后、有订阅者时才真正连接（StrictMode 下重复构造无害） */
  constructor(readonly ids: number[]) {}

  /** 允许连接；已有订阅者就立刻连。 */
  open() {
    this.active = true
    this.connectIfNeeded()
  }

  /** 断开并禁止再连；订阅者保留，`open()` 后可恢复。 */
  close() {
    this.active = false
    this.source?.close()
    this.source = null
  }

  /** 订阅某个直播间的弹幕；返回退订函数。 */
  subscribe(id: number, listener: (frame: DanmakuFrame) => void): () => void {
    let set = this.listeners.get(id)
    if (!set) {
      set = new Set()
      this.listeners.set(id, set)
    }
    set.add(listener)
    this.connectIfNeeded()
    return () => {
      set?.delete(listener)
    }
  }

  private connectIfNeeded() {
    if (!this.active || this.source || this.ids.length === 0 || typeof window === 'undefined') return
    if (![...this.listeners.values()].some((set) => set.size > 0)) return
    const source = new EventSource(`${API_BASE}/v1/danmaku?ids=${this.ids.join(',')}`)
    const handle = (e: MessageEvent<string>) => {
      let frame: DanmakuFrame
      try {
        frame = JSON.parse(e.data)
      } catch {
        return
      }
      if (!frame.text) return
      this.listeners.get(frame.id)?.forEach((l) => l(frame))
    }
    for (const kind of KINDS) source.addEventListener(kind, handle as EventListener)
    this.source = source
  }
}

/** 同一组 id 的稳定键，供 effect 依赖：顺序无关。 */
export function feedKey(ids: number[]): string {
  return [...ids].sort((a, b) => a - b).join(',')
}

/**
 * 按需持有一个 feed：`enabled` 且 ids 非空时连接，ids 变化（监视器换路）时换一条，
 * 关闭 / 卸载时断开。返回值给各播放器的 `danmaku.feed`。
 */
export function useDanmakuFeed(ids: number[], enabled: boolean): DanmakuFeed | null {
  const key = enabled ? feedKey(ids) : ''
  // 构造无副作用，连接在 effect 里 open() 之后才发生
  const feed = useMemo(() => (key ? new DanmakuFeed(key.split(',').map(Number)) : null), [key])
  useEffect(() => {
    feed?.open()
    return () => feed?.close()
  }, [feed])
  return feed
}
