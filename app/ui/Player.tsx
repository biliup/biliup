import React, { useEffect, useRef } from 'react'
import Artplayer from 'artplayer'
import artplayerPluginDanmuku from 'artplayer-plugin-danmuku'
import mpegts from 'mpegts.js'
import type { DanmakuFeed, DanmakuFrame } from '@/app/lib/danmaku-feed'

type VideoPlayer = Artplayer | null

/** mpegts.js 能直接解封装的两种容器；其它扩展名交给浏览器原生 <video> */
export type MpegtsType = 'flv' | 'mpegts'
/** 直播预览支持的容器：flv / mpegts 走 mpegts.js，fmp4 直接 MediaSource 追加 */
export type LiveType = MpegtsType | 'fmp4'

interface PlayerConfig {
  url: string
  height?: string
  width?: string
  /**
   * 容器类型。不传时按 url 扩展名判断（`.flv` → flv，其余原生）。
   * 直播预览的地址没有扩展名，由调用方按后端给的 `preview.format`（与响应 Content-Type 一致）传入；
   * 直播且未传时，先用一次 GET 探测响应的 Content-Type 再起播。
   */
  type?: LiveType
  /** fmp4 的 RFC 6381 编码串（后端从 init segment 解出，如 `avc1.64001f,mp4a.40.2`） */
  codecs?: string | null
  /** 直播：隐藏进度条、开启追帧，播放器随分块到达持续解码 */
  isLive?: boolean
  muted?: boolean
  autoplay?: boolean
  /** 直播流结束 / 被服务端断开（如客户端掉队、录制换直链）时回调，调用方决定是否重连 */
  onEnded?: () => void
  /** mpegts.js 或探测阶段报错（网络 / 解码），附一句可展示的说明 */
  onError?: (message: string) => void
  /**
   * 实时弹幕：`feed` 是页面持有的共享连接（见 `DanmakuFeed`），`id` 是本路直播间；
   * `feed` 为 null 时弹幕层隐藏。传了这个 prop 就装 artplayer-plugin-danmuku，开关切换只
   * 订阅 / 退订与显示 / 隐藏，不重建播放器。
   */
  danmaku?: { id: number; feed: DanmakuFeed | null; fontSize?: number }
}

type DanmukuPlugin = ReturnType<ReturnType<typeof artplayerPluginDanmuku>>

/** 把后端的 RGB 整数转成 CSS 颜色 */
function rgbInt(color: number): string {
  return `#${(color & 0xffffff).toString(16).padStart(6, '0')}`
}

/**
 * mpegts.js 的直播拉流 loader：只做一件事——`abort()` 时**一定**中止 fetch。
 *
 * 自带的 FetchStreamLoader 在 Chrome 上处于 buffering 状态时不调 `AbortController.abort()`，
 * 而是等下一个 `read()` 回来再 `reader.cancel()`；解码出错后读取链已经断了，那一次永远等不到，
 * 拉流连接就留在服务端占着该路的预览许可直到刷新页面（真实 Twitch TS 房间连开 4 次就 429）。
 * 这里每次 `open()` 建一个 AbortController，`abort()` 直接调它，其余行为与原 loader 一致
 * （直播不需要 Range，`needStash` 同样为 true）。
 */
class AbortableFetchLoader extends mpegts.BaseLoader {
  private controller: AbortController | null = null
  private received = 0
  _needStash = true

  constructor(_seekHandler: unknown, _config: unknown) {
    super('abortable-fetch-loader')
  }

  static isSupported() {
    return typeof self.fetch === 'function' && typeof self.ReadableStream === 'function'
  }

  destroy() {
    if (this.isWorking()) this.abort()
    super.destroy()
  }

  open(dataSource: { url: string; withCredentials?: boolean }, range: { from: number; to: number }) {
    const controller = new AbortController()
    this.controller = controller
    this.received = 0
    this._status = mpegts.LoaderStatus.kConnecting
    fetch(dataSource.url, {
      method: 'GET',
      mode: 'cors',
      cache: 'no-store',
      credentials: dataSource.withCredentials ? 'include' : 'same-origin',
      signal: controller.signal,
    })
      .then(async (res) => {
        if (!res.ok || !res.body) {
          this._status = mpegts.LoaderStatus.kError
          // 类型声明里 onError 的第一个参数是 LoaderErrors 接口本身，实际是其中的字符串值
          this.onError?.(mpegts.LoaderErrors.HTTP_STATUS_CODE_INVALID as unknown as mpegts.LoaderErrors, {
            code: res.status,
            msg: res.statusText,
          })
          return
        }
        const reader = res.body.getReader()
        for (;;) {
          const { done, value } = await reader.read()
          if (controller.signal.aborted) return
          if (done) {
            this._status = mpegts.LoaderStatus.kComplete
            this.onComplete?.(range.from, range.from + this.received - 1)
            return
          }
          this._status = mpegts.LoaderStatus.kBuffering
          const chunk = value.buffer.slice(value.byteOffset, value.byteOffset + value.byteLength) as ArrayBuffer
          const byteStart = range.from + this.received
          this.received += chunk.byteLength
          this.onDataArrival?.(chunk, byteStart, this.received)
        }
      })
      .catch((e: Error & { code?: number }) => {
        if (controller.signal.aborted) return
        this._status = mpegts.LoaderStatus.kError
        // 服务端结束响应（掉队断开 / 换直链）走 EARLY_EOF，mpegts.js 会把 MSE 收尾成 ended
        const earlyEof = e.name === 'TypeError' || e.code === 19
        const kind = earlyEof ? mpegts.LoaderErrors.EARLY_EOF : mpegts.LoaderErrors.EXCEPTION
        this.onError?.(kind as unknown as mpegts.LoaderErrors, { code: e.code ?? -1, msg: e.message })
      })
  }

  abort() {
    this._status = mpegts.LoaderStatus.kComplete
    this.controller?.abort()
    this.controller = null
  }
}

/** 销毁 mpegts.js 实例；`destroy()` 内部清 SourceBuffer 在元素出错后可能抛，拉流已由 loader 的 abort 保证断开。 */
function destroyMpegts(player: mpegts.Player) {
  try {
    player.destroy()
  } catch (e) {
    console.warn('[Player] mpegts.destroy 抛出异常（拉流已断开）', e)
  }
}

/** 一条弹幕送进弹幕层：普通弹幕滚动，礼物 / 醒目留言 / 上舰置顶并描边。 */
function emitFrame(plugin: DanmukuPlugin, frame: DanmakuFrame) {
  plugin.emit({
    text: frame.text,
    color: rgbInt(frame.color),
    mode: frame.kind === 'danmaku' ? 0 : 1,
    border: frame.kind !== 'danmaku',
  })
}

/** 按响应头判断直播流容器：`video/x-flv` → flv，`video/mp2t` → mpegts，`video/mp4` → fmp4。 */
export function liveTypeFromContentType(contentType: string | null | undefined): LiveType | null {
  const ct = (contentType ?? '').toLowerCase()
  if (ct.includes('flv')) return 'flv'
  if (ct.includes('mp2t') || ct.includes('mpegts') || ct.includes('mpeg2-ts')) return 'mpegts'
  if (ct.includes('mp4')) return 'fmp4'
  return null
}

/**
 * 用一次 GET 读到响应头就中止，只为拿 Content-Type。
 * 非 2xx 时把服务端的说明（415 / 429 / 503 的正文）原样抛出。
 */
async function probeLiveType(url: string): Promise<LiveType> {
  const controller = new AbortController()
  try {
    const res = await fetch(url, { signal: controller.signal, cache: 'no-store' })
    if (!res.ok) {
      const text = await res.text().catch(() => '')
      throw new Error(text || `HTTP ${res.status}`)
    }
    const type = liveTypeFromContentType(res.headers.get('content-type'))
    if (!type) throw new Error(`无法识别的流类型: ${res.headers.get('content-type') ?? '未知'}`)
    return type
  } finally {
    controller.abort()
  }
}

/** fmp4 预览要求的 MIME；`codecs` 为空时返回 null（Chrome 的 addSourceBuffer 必须带 codecs） */
export function fmp4MimeType(codecs: string | null | undefined): string | null {
  const c = (codecs ?? '').trim()
  return c ? `video/mp4; codecs="${c}"` : null
}

/** 当前浏览器能否用 MSE 播这组 fmp4 编码；SSR / 无 MediaSource 环境返回 false */
export function canPlayFmp4(codecs: string | null | undefined): boolean {
  const mime = fmp4MimeType(codecs)
  if (!mime || typeof window === 'undefined' || typeof MediaSource === 'undefined') return false
  try {
    return MediaSource.isTypeSupported(mime)
  } catch {
    return false
  }
}

/** 直播追帧：缓冲末尾落后超过这么多秒就跳到末尾附近 */
const FMP4_MAX_LATENCY_S = 3
const FMP4_TARGET_REMAIN_S = 0.5
/** 早于当前播放位置这么多秒的缓冲定期清掉，长时间观看不涨内存 */
const FMP4_KEEP_BACKWARD_S = 30

interface Fmp4Options {
  codecs: string | null | undefined
  autoplay: boolean
  onEnded?: () => void
  onError?: (message: string) => void
}

/**
 * Artplayer customType：fMP4 直接交给 MediaSource。
 * 服务端先发 init segment（ftyp + moov）再按 moof + mdat 分片广播，正是 MSE 的原生输入，不需要转封装：
 * fetch 的 ReadableStream 按到达顺序进 appendBuffer 队列（`updateend` 串行），
 * 首个缓冲区出现时把播放位置对齐到缓冲起点，之后落后超过阈值就追到末尾，并定期 remove 旧缓冲。
 */
function playWithMediaSource(
  video: HTMLVideoElement,
  url: string,
  art: Artplayer,
  { codecs, autoplay, onEnded, onError }: Fmp4Options
) {
  const mime = fmp4MimeType(codecs)
  if (!mime) {
    onError?.('后端未能解析出 fMP4 的编码参数，无法起播')
    return
  }
  if (typeof MediaSource === 'undefined' || !MediaSource.isTypeSupported(mime)) {
    onError?.(`浏览器不支持该编码（${codecs}），无法在页面内播放`)
    return
  }

  const controller = new AbortController()
  const mediaSource = new MediaSource()
  const objectUrl = URL.createObjectURL(mediaSource)
  const queue: Uint8Array[] = []
  let sourceBuffer: SourceBuffer | null = null
  let streamEnded = false
  let disposed = false
  let aligned = false
  let chaser: ReturnType<typeof setInterval> | null = null

  const fail = (message: string) => {
    if (disposed) return
    onError?.(message)
  }
  const pump = () => {
    if (disposed || !sourceBuffer || sourceBuffer.updating || mediaSource.readyState !== 'open') return
    const next = queue.shift()
    if (next) {
      try {
        sourceBuffer.appendBuffer(next as BufferSource)
      } catch (error) {
        // 配额满：先清掉早于当前位置的缓冲再重试这一块
        if ((error as DOMException)?.name === 'QuotaExceededError' && video.buffered.length) {
          queue.unshift(next)
          const cut = Math.max(video.buffered.start(0), video.currentTime - 5)
          if (cut > video.buffered.start(0)) sourceBuffer.remove(video.buffered.start(0), cut)
          else fail('播放缓冲已满')
        } else {
          fail(`MSE 追加失败: ${(error as Error)?.message ?? error}`)
        }
      }
    } else if (streamEnded) {
      try {
        mediaSource.endOfStream()
      } catch {
        /* 已经结束 */
      }
    }
  }
  const trim = () => {
    if (!sourceBuffer || sourceBuffer.updating || !video.buffered.length) return
    const start = video.buffered.start(0)
    const cut = video.currentTime - FMP4_KEEP_BACKWARD_S
    if (cut - start > 5) sourceBuffer.remove(start, cut)
  }
  const chase = () => {
    if (disposed || !video.buffered.length) return
    const end = video.buffered.end(video.buffered.length - 1)
    if (!aligned) {
      // 直播分片的时间戳从一个很大的值起步，播放位置得先跳进缓冲区
      aligned = true
      video.currentTime = Math.max(video.buffered.start(0), end - FMP4_TARGET_REMAIN_S)
      if (autoplay) video.play().catch(() => {
        video.muted = true
        video.play().catch(() => {})
      })
      return
    }
    if (end - video.currentTime > FMP4_MAX_LATENCY_S && !video.paused) {
      video.currentTime = end - FMP4_TARGET_REMAIN_S
    }
  }

  const dispose = () => {
    if (disposed) return
    disposed = true
    controller.abort()
    if (chaser) clearInterval(chaser)
    try {
      if (mediaSource.readyState === 'open') mediaSource.endOfStream()
    } catch {
      /* ignore */
    }
    URL.revokeObjectURL(objectUrl)
  }
  art.on('destroy', dispose)

  mediaSource.addEventListener('sourceopen', () => {
    if (disposed) return
    try {
      sourceBuffer = mediaSource.addSourceBuffer(mime)
    } catch (error) {
      fail(`浏览器拒绝该编码（${codecs}）: ${(error as Error)?.message ?? error}`)
      return
    }
    sourceBuffer.mode = 'segments'
    sourceBuffer.addEventListener('updateend', () => {
      trim()
      pump()
    })
    sourceBuffer.addEventListener('error', () => fail('MSE 解码失败'))
    chaser = setInterval(chase, 500)

    fetch(url, { signal: controller.signal, cache: 'no-store' })
      .then(async (res) => {
        if (!res.ok) {
          const text = await res.text().catch(() => '')
          throw new Error(text || `连接失败（HTTP ${res.status}）`)
        }
        if (!res.body) throw new Error('浏览器不支持流式读取响应')
        const reader = res.body.getReader()
        for (;;) {
          const { done, value } = await reader.read()
          if (disposed) return
          if (done) break
          if (value) {
            queue.push(value)
            pump()
          }
        }
        streamEnded = true
        pump()
        // 服务端结束了响应（掉队断开 / 换直链 / 录制结束）
        onEnded?.()
      })
      .catch((error: unknown) => {
        if (disposed || (error as Error)?.name === 'AbortError') return
        fail(error instanceof Error ? error.message : String(error))
      })
  })
  video.src = objectUrl
}

function describeMpegtsError(errorType: string, detail: string, info?: { code?: number; msg?: string }): string {
  if (errorType === mpegts.ErrorTypes.NETWORK_ERROR) {
    if (detail === mpegts.ErrorDetails.NETWORK_STATUS_CODE_INVALID) {
      const code = info?.code
      if (code === 415) return '当前下载器 / 容器不支持预览'
      if (code === 429) return '预览连接数已达上限，请稍后再试'
      if (code === 503) return '录制尚未开始拉流或正在重连'
      return `连接失败（HTTP ${code ?? '?'}）`
    }
    return `网络错误: ${info?.msg ?? detail}`
  }
  if (errorType === mpegts.ErrorTypes.MEDIA_ERROR) {
    if (detail === mpegts.ErrorDetails.MEDIA_CODEC_UNSUPPORTED) return '浏览器不支持该编码（可能为 HEVC）'
    if (detail === mpegts.ErrorDetails.MEDIA_MSE_ERROR) {
      // 典型来源：TS 的 PES 没有按访问单元对齐（如 Twitch），mpegts.js 拆出的帧不完整，浏览器拒绝解码
      return '浏览器解码失败：该直播源的封装方式 mpegts.js 无法处理，录制不受影响'
    }
    return `解码错误: ${info?.msg ?? detail}`
  }
  return `播放错误: ${info?.msg ?? detail}`
}

interface MpegtsOptions {
  type: MpegtsType
  isLive: boolean
  autoplay: boolean
  onEnded?: () => void
  onError?: (message: string) => void
}

/** Artplayer customType：用 mpegts.js 解封装 FLV / MPEG-TS 后喂给 MSE。 */
function playWithMpegts(
  video: HTMLVideoElement,
  url: string,
  art: Artplayer,
  { type, isLive, autoplay, onEnded, onError }: MpegtsOptions
) {
  if (!mpegts.isSupported()) {
    art.notice.show = `当前浏览器不支持 MSE，无法播放 ${type}`
    onError?.('当前浏览器不支持 MSE')
    return
  }
  const artWithMpegts = art as Artplayer & { mpegts?: mpegts.Player | null }
  if (artWithMpegts.mpegts) {
    destroyMpegts(artWithMpegts.mpegts)
    artWithMpegts.mpegts = null
  }

  const player = mpegts.createPlayer(
    { type, url, isLive },
    isLive
      ? {
          // 直播：不攒缓冲、落后就追，源缓冲区用完即清理，长时间观看不涨内存
          enableStashBuffer: false,
          customLoader: AbortableFetchLoader,
          liveBufferLatencyChasing: true,
          liveBufferLatencyMaxLatency: 3,
          liveBufferLatencyMinRemain: 0.5,
          autoCleanupSourceBuffer: true,
          autoCleanupMaxBackwardDuration: 30,
          autoCleanupMinBackwardDuration: 10,
        }
      : {}
  )
  artWithMpegts.mpegts = player
  art.on('destroy', () => {
    if (artWithMpegts.mpegts) {
      destroyMpegts(artWithMpegts.mpegts)
      artWithMpegts.mpegts = null
    }
  })
  player.on(mpegts.Events.ERROR, (errorType: string, detail: string, info?: { code?: number; msg?: string }) => {
    const message = describeMpegtsError(errorType, detail, info)
    art.notice.show = message
    onError?.(message)
  })
  if (isLive) {
    // 服务端结束响应（掉队断开 / 换直链）时 mpegts.js 会把 MSE 收尾成 ended
    video.addEventListener('ended', () => onEnded?.(), { once: true })
  }

  player.attachMediaElement(video)
  player.load()
  if (autoplay) {
    const playing = player.play()
    if (playing && typeof (playing as Promise<void>).catch === 'function') {
      ;(playing as Promise<void>).catch(() => {
        // 自动播放被浏览器拦下时静音重试，直播预览宁可无声也不要卡住
        video.muted = true
        player.play()
      })
    }
  }
}

const Players: React.FC<PlayerConfig> = ({
  url,
  height = '100%',
  width = '100%',
  type,
  codecs,
  isLive = false,
  muted = false,
  autoplay = isLive,
  onEnded,
  onError,
  danmaku,
}) => {
  const containerRef = useRef<HTMLDivElement>(null)
  const playerRef = useRef<VideoPlayer>(null)
  const danmakuCapable = !!danmaku
  const danmakuId = danmaku?.id
  const danmakuFeed = danmaku?.feed ?? null
  const danmakuFontSize = danmaku?.fontSize ?? 22
  // 回调放进 ref：父组件每次渲染传入的新函数不应重建播放器
  const callbacksRef = useRef({ onEnded, onError })
  useEffect(() => {
    callbacksRef.current = { onEnded, onError }
  })

  useEffect(() => {
    if (!containerRef.current) return
    const container = containerRef.current
    let cancelled = false

    const create = (mediaType: LiveType | null) => {
      if (cancelled || !container.isConnected) return
      if (playerRef.current) {
        playerRef.current.destroy()
        playerRef.current = null
      }
      const base = {
        container,
        url,
        autoSize: !isLive,
        fullscreen: true,
        fullscreenWeb: true,
        autoOrientation: true,
        isLive,
        muted,
        autoplay,
        plugins: danmakuCapable
          ? [
              artplayerPluginDanmuku({
                danmuku: [],
                speed: 7,
                fontSize: danmakuFontSize,
                opacity: 0.9,
                antiOverlap: true,
                synchronousPlayback: false,
                emitter: false,
                visible: true,
                margin: [10, '25%'],
              }),
            ]
          : [],
      }
      const onEndedCb = () => callbacksRef.current.onEnded?.()
      const onErrorCb = (message: string) => callbacksRef.current.onError?.(message)
      try {
        if (mediaType === 'fmp4') {
          const options: Fmp4Options = { codecs, autoplay, onEnded: onEndedCb, onError: onErrorCb }
          playerRef.current = new Artplayer({
            ...base,
            type: 'fmp4',
            customType: {
              fmp4: (video: HTMLVideoElement, src: string, art: Artplayer) =>
                playWithMediaSource(video, src, art, options),
            },
          })
        } else if (mediaType) {
          const options: MpegtsOptions = {
            type: mediaType,
            isLive,
            autoplay,
            onEnded: onEndedCb,
            onError: onErrorCb,
          }
          playerRef.current = new Artplayer({
            ...base,
            type: mediaType,
            customType: {
              [mediaType]: (video: HTMLVideoElement, src: string, art: Artplayer) =>
                playWithMpegts(video, src, art, options),
            },
          })
        } else {
          playerRef.current = new Artplayer(base)
        }
      } catch (error) {
        console.error('播放器初始化失败:', error)
        callbacksRef.current.onError?.('播放器初始化失败')
      }
    }

    const declared: LiveType | null = type ?? (url.endsWith('.flv') ? 'flv' : null)
    if (declared || !isLive) {
      create(declared)
    } else {
      probeLiveType(url).then(create, (error: unknown) => {
        if (cancelled) return
        callbacksRef.current.onError?.(error instanceof Error ? error.message : String(error))
      })
    }

    return () => {
      cancelled = true
      if (playerRef.current) {
        playerRef.current.destroy()
        playerRef.current = null
      }
    }
  }, [url, height, width, type, codecs, isLive, muted, autoplay, danmakuCapable, danmakuFontSize])

  // 弹幕开关：有 feed → 订阅本路、显示弹幕层；没有 → 退订、隐藏。不重建播放器。
  // 播放器可能还在探测 Content-Type（异步创建），所以轮询等到实例出现再挂。
  useEffect(() => {
    if (danmakuId === undefined || !danmakuFeed) {
      const plugin = playerRef.current?.plugins?.artplayerPluginDanmuku as DanmukuPlugin | undefined
      plugin?.hide()
      return
    }
    let unsubscribe: (() => void) | null = null
    let cancelled = false
    const attach = () => {
      if (cancelled) return true
      const plugin = playerRef.current?.plugins?.artplayerPluginDanmuku as DanmukuPlugin | undefined
      if (!plugin) return false
      plugin.show()
      unsubscribe = danmakuFeed.subscribe(danmakuId, (frame) => emitFrame(plugin, frame))
      return true
    }
    if (!attach()) {
      const timer = setInterval(() => {
        if (attach()) clearInterval(timer)
      }, 300)
      return () => {
        cancelled = true
        clearInterval(timer)
        unsubscribe?.()
      }
    }
    return () => {
      cancelled = true
      unsubscribe?.()
    }
  }, [danmakuId, danmakuFeed, url, type, codecs])

  return <div ref={containerRef} style={{ width, height }} />
}

export default Players
