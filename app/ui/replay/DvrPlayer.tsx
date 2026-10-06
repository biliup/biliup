'use client'
import React, { forwardRef, useEffect, useImperativeHandle, useRef } from 'react'
import mpegts from 'mpegts.js'
import { describeMpegtsError, destroyMpegts, type LoaderHooks, mpegtsLiveConfig } from '@/app/ui/Player'
import { mediaUrl } from '@/app/lib/sessions'
import styles from './replay.module.scss'

export type DvrPhase = 'connecting' | 'playing' | 'paused' | 'waiting' | 'ended' | 'error'

export interface DvrHandle {
  play(): void
  pause(): void
  /** 目标在已缓冲的范围里就地跳过去并返回 true；否则返回 false，由调用方用新的 `from` 重开 */
  seekWithin(ms: number): boolean
  /** 当前画面的场次时间；还没起播时是请求的 `from` */
  position(): number
  paused(): boolean
  setMuted(muted: boolean): void
}

/** 缓冲区判断的容差（秒）：MSE 的 buffered 边界和帧时间有几十毫秒的出入 */
const EDGE_S = 0.08

interface Timing {
  /** 起播关键帧的场次时间（`X-Dvr-Start-Ms`） */
  startMs: number | null
  /** 起播关键帧在播放器时间线上的位置 */
  anchor: number | null
  /** 还没跳到 `from` 本身 */
  pendingSeek: boolean
}

function positionOf(video: HTMLVideoElement | null, t: Timing, from: number): number {
  if (!video || t.startMs === null || t.anchor === null || t.pendingSeek) return from
  return t.startMs + (video.currentTime - t.anchor) * 1000
}

/**
 * DVR 回看播放器：`GET /v1/sessions/{id}/media?from=` 的 chunked FLV / TS 交给 mpegts.js（与直播预览同一套
 * loader 和直播配置，不追帧），画在一个不带自带控件的 <video> 上，控件由回看页自己画。
 *
 * 场次时间怎么算：服务端从不晚于 `from` 的最近关键帧起播，`X-Dvr-Start-Ms` 是那个关键帧的场次时间；
 * mpegts.js 会把第一帧的时间戳归到 0 附近，所以取第一次有缓冲时的 `buffered.start(0)` 作为那个关键帧在
 * 播放器里的位置，之后「场次时间 = 起播关键帧 + (currentTime − 该位置)」。起播后先就地跳到 `from` 本身。
 *
 * 一次打开只放一条响应：遇到断流缺口、已清理 / 缺失的分段、编码参数变化或场次结束，服务端结束响应，这里报 `ended`，
 * 由回看页决定从哪里重开。换位置同样由调用方换 `key` 重建，卸载时中止拉流。
 */
const DvrPlayer = forwardRef<
  DvrHandle,
  {
    sessionId: number
    from: number
    /** 起播分段的封装，决定 mpegts.js 用哪个解复用器 */
    type: 'flv' | 'mpegts'
    muted: boolean
    onPosition: (ms: number) => void
    /** `status` 是服务端拒绝时的 HTTP 状态码；连不上或连接中途断掉为 0 */
    onPhase: (phase: DvrPhase, message?: string, status?: number) => void
    onEnded: (lastMs: number) => void
    onMutedChange: (muted: boolean) => void
  }
>(function DvrPlayer({ sessionId, from, type, muted, onPosition, onPhase, onEnded, onMutedChange }, ref) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const callbacks = useRef({ onPosition, onPhase, onEnded, onMutedChange })
  useEffect(() => {
    callbacks.current = { onPosition, onPhase, onEnded, onMutedChange }
  })
  // 只在本次打开里有意义
  const timing = useRef<Timing>({ startMs: null, anchor: null, pendingSeek: true })
  const initialMuted = useRef(muted)

  useImperativeHandle(ref, () => ({
    play() {
      const video = videoRef.current
      if (!video) return
      video.play().catch(() => {
        video.muted = true
        video.play().catch(() => callbacks.current.onPhase('paused'))
      })
    },
    pause() {
      videoRef.current?.pause()
    },
    seekWithin(ms: number) {
      const video = videoRef.current
      const { startMs, anchor } = timing.current
      if (!video || startMs === null || anchor === null) return false
      const target = anchor + (ms - startMs) / 1000
      for (let i = 0; i < video.buffered.length; i++) {
        if (target >= video.buffered.start(i) - EDGE_S && target <= video.buffered.end(i) - EDGE_S) {
          timing.current.pendingSeek = false
          video.currentTime = Math.max(video.buffered.start(i), target)
          return true
        }
      }
      return false
    },
    position() {
      return positionOf(videoRef.current, timing.current, from)
    },
    paused() {
      return videoRef.current?.paused ?? true
    },
    setMuted(value: boolean) {
      if (videoRef.current) videoRef.current.muted = value
    },
  }))

  useEffect(() => {
    const video = videoRef.current
    if (!video) return
    const t = timing.current
    t.startMs = null
    t.anchor = null
    t.pendingSeek = true
    video.muted = initialMuted.current
    let serverError = false
    let disposed = false
    const cb = () => callbacks.current

    if (!mpegts.isSupported()) {
      cb().onPhase('error', '当前浏览器不支持 MSE，无法回看')
      return
    }

    const hooks: LoaderHooks = {
      onResponse(res) {
        if (res.ok) {
          const header = Number(res.headers.get('X-Dvr-Start-Ms'))
          t.startMs = Number.isFinite(header) && res.headers.has('X-Dvr-Start-Ms') ? header : from
          if (t.startMs >= from) t.pendingSeek = false
          return
        }
        serverError = true
        const status = res.status
        res
          .clone()
          .text()
          .catch(() => '')
          .then((text) => {
            if (disposed) return
            cb().onPhase('error', text.trim() || `回看失败（HTTP ${status}）`, status)
          })
      },
    }
    const player = mpegts.createPlayer(
      { type, url: mediaUrl(sessionId, from), isLive: true, cors: true, withCredentials: false },
      { ...mpegtsLiveConfig('relay'), ...hooks } as mpegts.Config
    )
    player.on(mpegts.Events.ERROR, (errorType: string, detail: string, info?: { code?: number; msg?: string }) => {
      if (serverError || disposed) return
      const network = errorType === mpegts.ErrorTypes.NETWORK_ERROR
      cb().onPhase('error', describeMpegtsError(errorType, detail, info), network ? 0 : undefined)
    })

    const alignToFrom = () => {
      if (!video.buffered.length || t.startMs === null) return
      if (t.anchor === null) t.anchor = video.buffered.start(0)
      if (!t.pendingSeek) return
      const target = t.anchor + (from - t.startMs) / 1000
      if (video.buffered.end(video.buffered.length - 1) >= target + EDGE_S) {
        t.pendingSeek = false
        video.currentTime = target
      }
    }
    const onProgress = () => alignToFrom()
    const onTime = () => {
      alignToFrom()
      cb().onPosition(positionOf(video, t, from))
    }
    const onPlaying = () => cb().onPhase('playing')
    const onPause = () => {
      if (!video.ended) cb().onPhase('paused')
    }
    const onWaiting = () => cb().onPhase('waiting')
    const onEnded = () => {
      cb().onPhase('ended')
      cb().onEnded(positionOf(video, t, from))
    }
    const onVolume = () => cb().onMutedChange(video.muted)
    video.addEventListener('progress', onProgress)
    video.addEventListener('loadeddata', onProgress)
    video.addEventListener('timeupdate', onTime)
    video.addEventListener('playing', onPlaying)
    video.addEventListener('pause', onPause)
    video.addEventListener('waiting', onWaiting)
    video.addEventListener('ended', onEnded)
    video.addEventListener('volumechange', onVolume)

    cb().onPhase('connecting')
    player.attachMediaElement(video)
    player.load()
    const playing = player.play() as Promise<void> | void
    if (playing && typeof playing.catch === 'function') {
      playing.catch(() => {
        // 浏览器拦下有声自动播放时静音再试；实例已经卸载就不要再碰它
        if (disposed) return
        video.muted = true
        const retry = video.play()
        retry?.catch(() => {
          if (!disposed) cb().onPhase('paused')
        })
      })
    }

    return () => {
      disposed = true
      video.removeEventListener('progress', onProgress)
      video.removeEventListener('loadeddata', onProgress)
      video.removeEventListener('timeupdate', onTime)
      video.removeEventListener('playing', onPlaying)
      video.removeEventListener('pause', onPause)
      video.removeEventListener('waiting', onWaiting)
      video.removeEventListener('ended', onEnded)
      video.removeEventListener('volumechange', onVolume)
      destroyMpegts(player)
    }
  }, [sessionId, from, type])

  useEffect(() => {
    const video = videoRef.current
    if (video && video.muted !== muted) video.muted = muted
  }, [muted])

  return <video ref={videoRef} className={styles.video} playsInline data-dvr-session={sessionId} />
})

export default DvrPlayer
