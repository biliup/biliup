'use client'
import React, { useCallback, useEffect, useRef, useState } from 'react'
import dynamic from 'next/dynamic'
import { Button, Modal, Switch, Tag, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconPlay, IconRefresh } from '@douyinfe/semi-icons'
import type { ButtonProps } from '@douyinfe/semi-ui/lib/es/button'
import { LiveStreamerEntity } from '@/app/lib/api-streamer'
import { platformName } from '@/app/lib/status'
import {
  canPreview,
  formatRate,
  liveDanmakuUrl,
  liveImageUrl,
  livePreviewUrl,
  previewDisabledReason,
  previewFormatLabel,
} from '@/app/lib/use-dashboard'
import { useBoolPref } from '@/app/lib/use-local-pref'
import styles from './live-preview.module.scss'

const Players = dynamic(() => import('@/app/ui/Player'), { ssr: false })

/** 服务端断开（掉队 / 换直链）后自动重连的次数上限；超过后交给用户手动重试 */
const MAX_AUTO_RECONNECT = 3
const RECONNECT_DELAY_MS = 2000
/** 弹层弹幕开关记在本地，默认开；监视器另有自己的开关（默认关） */
const MODAL_DANMAKU_KEY = 'biliup.preview.danmaku'

type Phase = 'connecting' | 'playing' | 'reconnecting' | 'ended' | 'error'

/**
 * 直播预览播放区：复用正在录制的那一路流（`/v1/streamers/{id}/live`）。
 *
 * 播放器（Artplayer + mpegts.js）挂在 `key={nonce}` 上，重连即换 key 重建；
 * 组件卸载时播放器销毁、fetch 中止，服务端随之释放该连接的槽位。
 * 既用于卡片弹层，也用于历史记录页的监视器小窗。
 */
export function LivePreviewPlayer({
  streamer,
  muted = false,
  compact = false,
  danmaku = false,
  onFatal,
}: {
  streamer: LiveStreamerEntity
  muted?: boolean
  /** 监视器小窗：状态文字更简短 */
  compact?: boolean
  /** 是否显示实时弹幕（仅平台有弹幕客户端时生效） */
  danmaku?: boolean
  /** 自动重连耗尽或不可恢复错误（415 / 429 / 解码失败）时通知父组件 */
  onFatal?: (message: string) => void
}) {
  const [nonce, setNonce] = useState(0)
  const [phase, setPhase] = useState<Phase>('connecting')
  const [message, setMessage] = useState<string | null>(null)
  const attemptsRef = useRef(0)
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const format = streamer.preview?.format ?? undefined
  const codecs = streamer.preview?.codecs ?? null
  const url = livePreviewUrl(streamer.id)
  // 平台有弹幕客户端才装弹幕层；开关只控制显示与 SSE 连接
  const danmakuLayer = streamer.preview?.danmaku
    ? { url: liveDanmakuUrl(streamer.id), enabled: danmaku }
    : undefined
  // 出错 / 断开时把封面垫在说明文字后面，而不是一块黑
  const cover = liveImageUrl(streamer.id, 'cover', streamer.live_cover_url)

  useEffect(
    () => () => {
      if (timerRef.current) clearTimeout(timerRef.current)
    },
    []
  )

  const scheduleReconnect = useCallback(
    (why: string) => {
      if (attemptsRef.current >= MAX_AUTO_RECONNECT) {
        setPhase('ended')
        setMessage(`${why}，已重连 ${MAX_AUTO_RECONNECT} 次`)
        onFatal?.(why)
        return
      }
      attemptsRef.current += 1
      setPhase('reconnecting')
      setMessage(`${why}，${RECONNECT_DELAY_MS / 1000} 秒后重连（${attemptsRef.current}/${MAX_AUTO_RECONNECT}）`)
      timerRef.current = setTimeout(() => {
        setPhase('connecting')
        setMessage(null)
        setNonce((n) => n + 1)
      }, RECONNECT_DELAY_MS)
    },
    [onFatal]
  )

  const handleEnded = useCallback(() => scheduleReconnect('连接已断开'), [scheduleReconnect])
  const handleError = useCallback(
    (text: string) => {
      // 415（不支持）/ 429（超限）/ 解码 / 浏览器不支持的编码不会因重试而变化；503 与网络抖动可以再试
      const recoverable = /重连|503|网络|连接失败/.test(text) && !/不支持|拒绝|无法起播/.test(text)
      if (recoverable) {
        scheduleReconnect(text)
        return
      }
      setPhase('error')
      setMessage(text)
      onFatal?.(text)
    },
    [scheduleReconnect, onFatal]
  )
  const retry = () => {
    attemptsRef.current = 0
    setPhase('connecting')
    setMessage(null)
    setNonce((n) => n + 1)
  }

  // 播放器一旦起播就当作在放（mpegts.js 没有可靠的 first-frame 事件；错误会另行回调）
  useEffect(() => {
    if (phase !== 'connecting') return
    const t = setTimeout(() => setPhase((p) => (p === 'connecting' ? 'playing' : p)), 1500)
    return () => clearTimeout(t)
  }, [phase, nonce])

  const showPlayer = phase === 'connecting' || phase === 'playing'
  return (
    <div className={styles.player} data-phase={phase}>
      {showPlayer ? (
        <Players
          key={nonce}
          url={url}
          type={format}
          codecs={codecs}
          isLive
          muted={muted}
          autoplay
          onEnded={handleEnded}
          onError={handleError}
          danmaku={danmakuLayer}
        />
      ) : null}
      {phase !== 'playing' ? (
        <div className={styles.overlay}>
          {cover && (phase === 'error' || phase === 'ended') ? (
            // 封面走同源代理；加载失败只剩底色
            // eslint-disable-next-line @next/next/no-img-element
            <img
              className={styles.overlayCover}
              src={cover}
              alt=""
              aria-hidden="true"
              onError={(e) => {
                e.currentTarget.style.display = 'none'
              }}
            />
          ) : null}
          <div className={styles.overlayText}>
            {phase === 'connecting' ? '正在连接录制流…' : message}
          </div>
          {phase === 'ended' || phase === 'error' ? (
            <Button
              size={compact ? 'small' : 'default'}
              icon={<IconRefresh />}
              theme="light"
              onClick={retry}
            >
              重试
            </Button>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}

/** 卡片里的「预览」弹层：关闭即卸载播放器、断开连接。 */
export function LivePreviewModal({
  streamer,
  visible,
  onClose,
}: {
  streamer: LiveStreamerEntity
  visible: boolean
  onClose: () => void
}) {
  const { Text } = Typography
  const name = streamer.remark || streamer.url
  const rate = formatRate(streamer.live_bytes_per_sec)
  const format = streamer.preview?.format
  const danmakuAvailable = !!streamer.preview?.danmaku
  const [danmakuPref, setDanmakuPref] = useBoolPref(MODAL_DANMAKU_KEY, true)
  const notifiedRef = useRef(false)
  useEffect(() => {
    if (!visible) notifiedRef.current = false
  }, [visible])
  const handleFatal = useCallback(
    (text: string) => {
      if (notifiedRef.current) return
      notifiedRef.current = true
      Toast.error({ content: `预览中断：${text}`, duration: 4 })
    },
    []
  )

  return (
    <Modal
      visible={visible}
      onCancel={onClose}
      closeOnEsc
      footer={null}
      className={styles.modal}
      style={{ width: 'min(960px, 94vw)' }}
      bodyStyle={{ padding: 0 }}
      title={
        <div className={styles.modalTitle}>
          <span className={styles.modalName} title={name}>
            {name}
          </span>
          <Tag size="small" color="green">
            {platformName(streamer.url)}
          </Tag>
          {previewFormatLabel(format) ? (
            <Tag size="small" color="grey">
              {previewFormatLabel(format)}
            </Tag>
          ) : null}
          <Text type="tertiary" size="small">
            {rate ? `写盘 ${rate}` : '写盘速率 —'}
          </Text>
          <Tooltip
            content={
              danmakuAvailable
                ? '显示录制中的实时弹幕（来自本进程的弹幕客户端）'
                : '该平台没有弹幕客户端（目前支持 B 站 / 抖音 / 斗鱼 / 虎牙）'
            }
          >
            <span className={styles.danmakuSwitch}>
              <Text type="tertiary" size="small">
                弹幕
              </Text>
              <Switch
                size="small"
                checked={danmakuAvailable && danmakuPref}
                disabled={!danmakuAvailable}
                onChange={(v) => setDanmakuPref(!!v)}
                aria-label="弹幕"
              />
            </span>
          </Tooltip>
        </div>
      }
    >
      {/* 弹层关闭即卸载：不留后台连接 */}
      {visible ? (
        <LivePreviewPlayer
          streamer={streamer}
          danmaku={danmakuAvailable && danmakuPref}
          onFatal={handleFatal}
        />
      ) : null}
      <div className={styles.modalFoot}>
        <Text type="tertiary" size="small">
          画面来自正在写盘的同一路流，不另外向直播平台拉流；关闭弹窗即断开。
        </Text>
      </div>
    </Modal>
  )
}

/**
 * 「预览」按钮 + 弹层。录制中且下载器能旁路时可点；否则禁用并用 tooltip 说明原因
 * （ffmpeg / streamlink 子进程落盘、fMP4、HEVC、容器尚未确定）。
 */
export function LivePreviewButton({
  streamer,
  size = 'small',
  theme = 'light',
  iconOnly = false,
  className,
}: {
  streamer: LiveStreamerEntity
  size?: ButtonProps['size']
  theme?: ButtonProps['theme']
  iconOnly?: boolean
  className?: string
}) {
  const [open, setOpen] = useState(false)
  const enabled = canPreview(streamer)
  const reason = previewDisabledReason(streamer)
  const button = (
    <Button
      size={size}
      theme={theme}
      type={enabled ? 'primary' : 'tertiary'}
      icon={<IconPlay />}
      disabled={!enabled}
      className={className}
      aria-label={iconOnly ? '预览' : undefined}
      onClick={(e) => {
        e.stopPropagation()
        setOpen(true)
      }}
    >
      {iconOnly ? null : '预览'}
    </Button>
  )
  return (
    <>
      {enabled ? (
        button
      ) : (
        <Tooltip content={reason ?? '暂不可预览'}>
          {/* disabled 按钮不触发鼠标事件，包一层让 tooltip 仍能弹出 */}
          <span className={styles.disabledWrap}>{button}</span>
        </Tooltip>
      )}
      {open ? (
        <LivePreviewModal streamer={streamer} visible={open} onClose={() => setOpen(false)} />
      ) : null}
    </>
  )
}
