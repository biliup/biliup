'use client'
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import dynamic from 'next/dynamic'
import { Button, Modal, Radio, RadioGroup, Switch, Tag, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconChevronDown, IconPlay, IconRefresh } from '@douyinfe/semi-icons'
import type { ButtonProps } from '@douyinfe/semi-ui/lib/es/button'
import { LiveStreamerEntity, revalidateMe } from '@/app/lib/api-streamer'
import { platformName } from '@/app/lib/status'
import {
  canPreview,
  directPlayableUrl,
  fetchLiveUrl,
  formatRate,
  liveImageUrl,
  livePreviewUrl,
  previewDisabledReason,
  previewFormatLabel,
  usePreviewTransport,
} from '@/app/lib/use-dashboard'
import { useBoolPref, useEnumPref } from '@/app/lib/use-local-pref'
import { type DanmakuFeed, useDanmakuFeed } from '@/app/lib/danmaku-feed'
import {
  type LatencyProfile,
  type LiveBufferPolicy,
  RELAY_PROFILES,
  RELAY_PROFILES_SEGMENTED,
  relayPolicy,
  snapshotMsFor,
  type StallInfo,
} from '@/app/lib/live-buffer'
import { LiveRateChart, LiveRateSummary, MODAL_RATE_WINDOW_MS } from './LiveRateChart'
import { LiveMarkBar } from './MarkerControls'
import styles from './live-preview.module.scss'

const Players = dynamic(() => import('@/app/ui/Player'), { ssr: false })

/** 服务端断开（掉队 / 换直链）后自动重连的次数上限；超过后交给用户手动重试 */
const MAX_AUTO_RECONNECT = 3
const RECONNECT_DELAY_MS = 2000
/** 弹层弹幕开关记在本地，默认开；监视器另有自己的开关（默认关） */
const MODAL_DANMAKU_KEY = 'biliup.preview.danmaku'
/** 中转延迟档位（弹层与监视器共用），默认低延迟；卡顿后本次播放自动升到流畅 */
const LATENCY_PROFILE_KEY = 'biliup.preview.latency'
const LATENCY_PROFILES: readonly LatencyProfile[] = ['low', 'smooth']
/** 低延迟档下，稳态里一次卡住这么久、或 60 s 内卡两次，就升到流畅档重连 */
const ESCALATE_STALL_S = 1
const ESCALATE_WINDOW_MS = 60_000

/** 中转延迟档位偏好 */
export function useLatencyProfile(): [LatencyProfile, (v: LatencyProfile) => void] {
  return useEnumPref(LATENCY_PROFILE_KEY, LATENCY_PROFILES, 'low')
}
/** 弹层底部的码率折线默认展开，折叠状态记在本地 */
const MODAL_RATE_CHART_KEY = 'biliup.preview.rateChart'

type Phase = 'connecting' | 'playing' | 'reconnecting' | 'ended' | 'error'

/** 实际在用的取流方式与角标文案 */
type Source =
  | { kind: 'relay'; fallbackReason: null }
  | { kind: 'relay'; fallbackReason: string }
  | {
      kind: 'direct'
      url: string
      format: 'flv' | 'hls' | null
      /** 这条直链是不是失败后重取来的（立刻又失败就不再试直连） */
      refetched: boolean
      /** 开始用这条直链的时刻：播够一阵再断多半是直链到期（斗鱼 5 min、B 站 1 h），可以再取 */
      startedAt: number
    }

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
  danmakuFeed = null,
  onFatal,
}: {
  streamer: LiveStreamerEntity
  muted?: boolean
  /** 监视器小窗：状态文字更简短 */
  compact?: boolean
  /** 页面持有的实时弹幕连接；null 表示弹幕关。仅平台有弹幕客户端时装弹幕层 */
  danmakuFeed?: DanmakuFeed | null
  /** 自动重连耗尽或不可恢复错误（415 / 429 / 解码失败）时通知父组件 */
  onFatal?: (message: string) => void
}) {
  const [nonce, setNonce] = useState(0)
  const [phase, setPhase] = useState<Phase>('connecting')
  const [message, setMessage] = useState<string | null>(null)
  const attemptsRef = useRef(0)
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const transport = usePreviewTransport()
  const directCapable = !!streamer.preview?.direct?.capable
  const directReason = streamer.preview?.direct?.reason ?? null
  // direct 模式且平台能直连：先向后端要直链；否则走中转，direct 模式下把原因挂在角标上。
  // 取流方式随（房间, 配置, 能力）变化而重算：异步结果与失败回落都带着 key，key 不匹配的一律忽略，
  // 这样切换时不需要在 effect 里同步重置 state。
  const wantDirect = transport === 'direct' && directCapable
  const sourceKey = `${streamer.id}:${transport}:${directCapable}`
  const [fetched, setFetched] = useState<{ key: string; source: Source } | null>(null)
  const [override, setOverride] = useState<{ key: string; source: Source } | null>(null)
  useEffect(() => {
    if (!wantDirect) return
    let cancelled = false
    fetchLiveUrl(streamer.id).then(
      (info) => {
        if (cancelled) return
        const source: Source =
          info.direct.capable && info.url
            ? { kind: 'direct', url: directPlayableUrl(info.url), format: info.format, refetched: false, startedAt: Date.now() }
            : { kind: 'relay', fallbackReason: info.direct.reason ?? '该平台不支持直连' }
        setFetched({ key: sourceKey, source })
      },
      (e: unknown) => {
        if (cancelled) return
        setFetched({
          key: sourceKey,
          source: { kind: 'relay', fallbackReason: `获取直链失败：${e instanceof Error ? e.message : String(e)}` },
        })
      }
    )
    return () => {
      cancelled = true
    }
  }, [wantDirect, sourceKey, streamer.id])
  const source = useMemo<Source | null>(() => {
    if (override?.key === sourceKey) return override.source
    if (!wantDirect) {
      return { kind: 'relay', fallbackReason: transport === 'direct' ? (directReason ?? '该平台不支持直连') : null }
    }
    return fetched?.key === sourceKey ? fetched.source : null
  }, [override, fetched, sourceKey, wantDirect, transport, directReason])

  // 中转延迟档位：用户偏好起步；低延迟档在稳态里卡了，本次播放（同房间、同偏好）升到流畅档重连，
  // 用更深的快照起播。升档记 key，房间或偏好一变自然作废，不需要 effect 里重置
  const relayFormat = streamer.preview?.format ?? undefined
  const [profilePref] = useLatencyProfile()
  const levelKey = `${streamer.id}:${profilePref}`
  const [escalated, setEscalated] = useState<string | null>(null)
  const level: LatencyProfile = escalated === levelKey ? 'smooth' : profilePref
  const policy: LiveBufferPolicy = relayPolicy(level, relayFormat)
  const stallsRef = useRef<number[]>([])
  const handleStall = useCallback(
    (info: StallInfo) => {
      if (level !== 'low') return
      const now = Date.now()
      stallsRef.current = [...stallsRef.current.filter((t) => now - t < ESCALATE_WINDOW_MS), now]
      if (info.seconds < ESCALATE_STALL_S && stallsRef.current.length < 2) return
      stallsRef.current = []
      setEscalated(levelKey)
      setPhase('connecting')
      setMessage(null)
      setNonce((n) => n + 1)
    },
    [level, levelKey]
  )

  const codecs = streamer.preview?.codecs ?? null
  const relayUrl = livePreviewUrl(streamer.id, snapshotMsFor(policy))
  const url = source?.kind === 'direct' ? source.url : relayUrl
  const format = source?.kind === 'direct' ? (source.format ?? 'flv') : relayFormat
  // 平台有弹幕客户端才装弹幕层；开关只控制订阅与显示
  const danmakuLayer = streamer.preview?.danmaku
    ? { id: streamer.id, feed: danmakuFeed, fontSize: compact ? 15 : 22 }
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
      // 会话被收回时后端会截断预览流，表现为一次断流：刷新权限点，失效就跳登录页，降级就收起页面
      revalidateMe()
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

  // 直连出问题（CDN 403 / 直链过期 / 断开）：重取一次直链再播；再失败就回落中转，不再试直连
  const handleDirectFailure = useCallback(
    (why: string) => {
      const fallback = (reason: string) => {
        setOverride({ key: sourceKey, source: { kind: 'relay', fallbackReason: reason } })
        attemptsRef.current = 0
        setPhase('connecting')
        setMessage(null)
        setNonce((n) => n + 1)
      }
      // 播够 60 s 再断多半是直链到期（斗鱼 token 5 min、B 站 1 h），照常再取一条；
      // 刚起播就失败才算「直连不行」——重取一次后仍失败就回落中转
      const playedAWhile = source?.kind === 'direct' && Date.now() - source.startedAt > 60_000
      if (source?.kind !== 'direct' || (source.refetched && !playedAWhile)) {
        fallback(`直连失败：${why}`)
        return
      }
      setPhase('reconnecting')
      setMessage('直连中断，重新获取直链…')
      fetchLiveUrl(streamer.id, { fresh: true }).then(
        (info) => {
          if (info.direct.capable && info.url) {
            setOverride({
              key: sourceKey,
              source: {
                kind: 'direct',
                url: directPlayableUrl(info.url),
                format: info.format,
                refetched: true,
                startedAt: Date.now(),
              },
            })
            setPhase('connecting')
            setMessage(null)
            setNonce((n) => n + 1)
          } else {
            fallback(info.direct.reason ?? `直连失败：${why}`)
          }
        },
        () => fallback(`直连失败：${why}，且重取直链失败`)
      )
    },
    [source, sourceKey, streamer.id]
  )

  const handleEnded = useCallback(() => {
    if (source?.kind === 'direct') handleDirectFailure('连接已断开')
    else scheduleReconnect('连接已断开')
  }, [source, handleDirectFailure, scheduleReconnect])
  const handleError = useCallback(
    (text: string) => {
      if (source?.kind === 'direct') {
        handleDirectFailure(text)
        return
      }
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
    [source, handleDirectFailure, scheduleReconnect, onFatal]
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

  const pending = source === null
  const showPlayer = (phase === 'connecting' || phase === 'playing') && !pending
  const badge =
    source?.kind === 'direct'
      ? { text: '直连 CDN', tone: 'direct' as const }
      : source?.kind === 'relay' && source.fallbackReason
        ? { text: `已回落中转：${source.fallbackReason}`, tone: 'fallback' as const }
        : source?.kind === 'relay' && escalated === levelKey
          ? { text: '卡顿，已切到流畅档', tone: 'fallback' as const }
          : null
  return (
    <div
      className={styles.player}
      data-phase={phase}
      data-source={source?.kind ?? 'pending'}
      data-latency={source?.kind === 'relay' ? level : undefined}
    >
      {showPlayer ? (
        <Players
          key={`${source?.kind}-${url}-${nonce}`}
          url={url}
          type={format}
          codecs={codecs}
          isLive
          transport={source?.kind ?? 'relay'}
          buffer={policy}
          muted={muted}
          autoplay
          onEnded={handleEnded}
          onError={handleError}
          onStall={handleStall}
          danmaku={danmakuLayer}
        />
      ) : null}
      {badge ? (
        <Tooltip
          content={
            badge.tone === 'direct'
              ? '浏览器用另取的直链直接向 CDN 拉流，不经 biliup 中转、不影响录制（全局配置 preview_transport = direct）'
              : escalated === levelKey && source?.kind === 'relay' && !source.fallbackReason
                ? `低延迟档缓冲耗尽过，本次播放改用约 ${RELAY_PROFILES.smooth.target} s 缓冲；重新打开恢复低延迟`
                : badge.text
          }
        >
          <span className={styles.badge} data-tone={badge.tone} data-compact={compact || undefined}>
            {badge.text}
          </span>
        </Tooltip>
      ) : null}
      {phase !== 'playing' || pending ? (
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
            {pending
              ? '正在获取直链…'
              : phase === 'connecting'
                ? source.kind === 'direct'
                  ? '正在连接 CDN…'
                  : '正在连接录制流…'
                : message}
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
  const danmakuOn = visible && danmakuAvailable && danmakuPref
  const danmakuFeed = useDanmakuFeed([streamer.id], danmakuOn)
  const [chartOpen, setChartOpen] = useBoolPref(MODAL_RATE_CHART_KEY, true)
  const transport = usePreviewTransport()
  const [latency, setLatency] = useLatencyProfile()
  const notifiedRef = useRef(false)
  const playerRootRef = useRef<HTMLDivElement>(null)
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
                : '这一路没有弹幕客户端：平台不支持，或未开启对应的 *_danmaku 配置（B 站 / 抖音 / 斗鱼 / 虎牙可开）'
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
          {transport === 'relay' ? (
            <Tooltip
              content={`中转缓冲深度，也就是画面延迟。低延迟：FLV 约 ${RELAY_PROFILES.low.target} s、HLS 分片流（fMP4 / TS）约 ${RELAY_PROFILES_SEGMENTED.low.target} s，链路抖动大时可能偶发缓冲，卡了会自动切到流畅；流畅：约 ${RELAY_PROFILES.smooth.target} s。直连 CDN 时不适用`}
            >
              <span className={styles.danmakuSwitch}>
                <RadioGroup
                  type="button"
                  buttonSize="small"
                  value={latency}
                  onChange={(e) => setLatency(e.target.value as LatencyProfile)}
                  aria-label="中转延迟"
                >
                  <Radio value="low">低延迟</Radio>
                  <Radio value="smooth">流畅</Radio>
                </RadioGroup>
              </span>
            </Tooltip>
          ) : null}
        </div>
      }
    >
      {/* 弹层关闭即卸载：不留后台连接 */}
      <div ref={playerRootRef}>
        {visible ? (
          <LivePreviewPlayer streamer={streamer} danmakuFeed={danmakuOn ? danmakuFeed : null} onFatal={handleFatal} />
        ) : null}
      </div>
      <LiveMarkBar streamer={streamer} playerRoot={playerRootRef} active={visible} />
      {/* 写盘速率折线：每秒轮询瘦端点 /v1/live-rates，最近 3 分钟；折叠时不轮询、不加载 uPlot */}
      <section className={styles.rateSection} data-open={chartOpen ? 'true' : 'false'}>
        <button
          type="button"
          className={styles.rateHead}
          onClick={() => setChartOpen(!chartOpen)}
          aria-expanded={chartOpen}
          aria-controls={`live-rate-chart-${streamer.id}`}
        >
          <IconChevronDown className={styles.rateChevron} aria-hidden="true" />
          <span className={styles.rateTitle}>写盘速率 · 最近 3 分钟</span>
          {visible && chartOpen ? <LiveRateSummary id={streamer.id} windowMs={MODAL_RATE_WINDOW_MS} /> : null}
        </button>
        {visible && chartOpen ? (
          <div id={`live-rate-chart-${streamer.id}`} className={styles.rateBody}>
            <LiveRateChart id={streamer.id} windowMs={MODAL_RATE_WINDOW_MS} variant="full" height={150} label={name} />
          </div>
        ) : null}
      </section>
      <div className={styles.modalFoot}>
        <Text type="tertiary" size="small">
          {transport === 'direct'
            ? '直连模式：能直连的平台由浏览器直接向 CDN 拉流，不经 biliup；不能的自动回落到录制流中转（角标标出原因）。关闭弹窗即断开。'
            : '画面来自正在写盘的同一路流，不另外向直播平台拉流；关闭弹窗即断开。'}
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
