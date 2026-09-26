'use client'
import { API_BASE } from './api-streamer'

/**
 * 中转预览的租约（服务端见 `live_preview::lease`）。
 *
 * 服务端从 TCP 层看不出页面是否还在：浏览器经过会替它读完上游的代理 / 隧道时，关掉播放器连接也不断。
 * 所以页面自己证明还在看：
 *
 * - 会话号：每个页面一个随机串。码率 WebSocket（`use-live-rates.ts`）带着它连上即持有租约，服务端
 *   每 15 s Ping、浏览器自动回 Pong；WebSocket 断开（页面关了）或 45 s 没 Pong，这个页面的中转预览全部结束。
 *   WebSocket 连不上时退回每秒轮询，轮询同样带会话号续约。
 * - 连接号：每次建播放器一个（`<会话号>.<序号>`），拼进 `/live` 的地址。播放器销毁（关弹层、切档位、重连）时
 *   `DELETE /v1/streamers/{id}/live?conn=` 立即释放；页面关闭时用 `sendBeacon` 发同一路径的 POST 兜底。
 * - 开着的连接一变，码率 WebSocket 就把整份集合发给服务端（{@link openPreviewsMessage}），
 *   不在集合里的连接由服务端结束——哪次 DELETE 丢了也不会留下幽灵连接。只在变化时发，不靠定时器。
 */

let session: string | null = null
let seq = 0
/** 还没释放的中转预览：连接号 → 直播间 id，页面关闭时逐个 sendBeacon */
const open = new Map<string, number>()
let pageHideBound = false
const changeListeners = new Set<() => void>()

function changed() {
  changeListeners.forEach((l) => l())
}

/** 开着的中转预览集合变化时回调（码率 WebSocket 据此把集合发给服务端） */
export function onOpenPreviewsChange(listener: () => void): () => void {
  changeListeners.add(listener)
  return () => changeListeners.delete(listener)
}

/** 发给服务端的「本页开着的中转预览」：序号不大于 `seq` 又不在 `conns` 里的连接会被结束 */
export function openPreviewsMessage(): string {
  return JSON.stringify({ conns: [...open.keys()], seq })
}

/** 本页面的中转预览会话号（非安全上下文也能用 getRandomValues） */
export function previewSessionId(): string {
  if (!session) {
    const bytes = new Uint8Array(12)
    crypto.getRandomValues(bytes)
    session = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
  }
  return session
}

function releaseUrl(id: number, conn: string): string {
  return `${API_BASE}/v1/streamers/${id}/live?conn=${encodeURIComponent(conn)}`
}

/** 立即释放一条中转预览；已经结束的也无妨（服务端回 204） */
export function releasePreview(id: number, conn: string) {
  if (open.delete(conn)) changed()
  fetch(releaseUrl(id, conn), { method: 'DELETE', keepalive: true, cache: 'no-store' }).catch(() => {})
}

function onPageHide() {
  for (const [conn, id] of open) navigator.sendBeacon?.(releaseUrl(id, conn))
  open.clear()
}

/** 播放器每建一次拿一个连接号：返回拼好会话号 / 连接号的地址，以及销毁时调用的释放函数 */
export interface PreviewLease {
  attach(url: string): { url: string; release: () => void }
}

/** 某个直播间的中转预览租约；`attach` 每调用一次就是一条新连接 */
export function relayLease(id: number): PreviewLease {
  return {
    attach(url) {
      if (!pageHideBound && typeof window !== 'undefined') {
        window.addEventListener('pagehide', onPageHide)
        pageHideBound = true
      }
      seq += 1
      const conn = `${previewSessionId()}.${seq}`
      open.set(conn, id)
      changed()
      const sep = url.includes('?') ? '&' : '?'
      return {
        url: `${url}${sep}session=${previewSessionId()}&conn=${conn}`,
        release: () => releasePreview(id, conn),
      }
    },
  }
}
