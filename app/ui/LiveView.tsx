'use client'
import React, { useCallback, useRef, useState } from 'react'
import useSWR from 'swr'
import { useRouter } from 'next/navigation'
import { Button, Empty, Radio, RadioGroup, Select, Spin, Switch, Tag, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconArrowLeft, IconChevronDown, IconHistory, IconLive, IconRefresh } from '@douyinfe/semi-icons'
import { fetcher, LiveStreamerEntity, StreamerInfo } from '@/app/lib/api-streamer'
import { LIVE_STATUS, platformName } from '@/app/lib/status'
import {
  canPreview,
  formatRate,
  liveImageUrl,
  previewDisabledReason,
  previewFormatLabel,
  SLOW_REFRESH_MS,
  STREAMERS_REFRESH_MS,
  usePreviewTransport,
} from '@/app/lib/use-dashboard'
import { useBoolPref } from '@/app/lib/use-local-pref'
import { useMe } from '@/app/lib/use-me'
import { useDanmakuFeed } from '@/app/lib/danmaku-feed'
import { type LatencyProfile, RELAY_PROFILES, RELAY_PROFILES_SEGMENTED } from '@/app/lib/live-buffer'
import PageHeader from '@/app/(app)/components/PageHeader'
import { LivePreviewPlayer, livePageHref, useLatencyProfile } from './LivePreview'
import { LiveRateChart, LiveRateSummary, MODAL_RATE_WINDOW_MS } from './LiveRateChart'
import { LiveMarkBar } from './MarkerControls'
import { PublishQueueBanner } from './publish/JobStatus'
import styles from './live-view.module.scss'

/** 预览页的弹幕开关记在本地，默认开；监视器另有自己的开关（默认关）。键名沿用弹层时代，老用户的选择不丢 */
const DANMAKU_KEY = 'biliup.preview.danmaku'
/** 播放器下方的码率折线默认展开，折叠状态记在本地（键名同上沿用） */
const RATE_CHART_KEY = 'biliup.preview.rateChart'

/** 录像回看页的地址（见 `app/(app)/replay/page.tsx`） */
function replayHref(sessionId: number): string {
  return `/replay?session=${sessionId}`
}

/**
 * 直播预览页 `/live?streamer=<id>` 的主体。
 *
 * 播放器随页面挂载 / 卸载：离开页面（站内跳转、浏览器后退、关标签页）时播放器销毁，
 * 中转连接当场释放（`DELETE …/live?conn=`，关标签页走 `sendBeacon`），租约心跳随之放下。
 * 同一页面里换直播间时整块按直播间 id 重新挂载，旧连接先释放、新连接后建立。
 */
export default function LiveView({ streamerId }: { streamerId: number }) {
  const router = useRouter()
  const { data: streamers, error, isLoading, mutate } = useSWR<LiveStreamerEntity[]>('/v1/streamers', fetcher, {
    refreshInterval: STREAMERS_REFRESH_MS,
  })
  const { data: infos } = useSWR<StreamerInfo[]>('/v1/streamer-info', fetcher, {
    refreshInterval: SLOW_REFRESH_MS,
  })
  const streamer = streamers?.find((s) => s.id === streamerId)
  const info = streamer ? latestInfo(infos, streamer.url) : undefined
  const back = () => {
    if (window.history.length > 1) router.back()
    else router.push('/')
  }

  const name = streamer ? streamer.remark || streamer.url : null
  const rate = streamer ? formatRate(streamer.live_bytes_per_sec) : null
  const format = previewFormatLabel(streamer?.preview?.format)
  const header = (
    <PageHeader
      icon={<IconLive size="large" />}
      title={
        <span className={styles.headTitle} title={name ?? undefined}>
          {name ? `直播预览 · ${name}` : '直播预览'}
        </span>
      }
      description={
        streamer ? (
          <span className={styles.headMeta}>
            <Tag size="small" color="green">
              {platformName(streamer.url)}
            </Tag>
            {format ? (
              <Tag size="small" color="grey">
                {format}
              </Tag>
            ) : null}
            <span>{rate ? `写盘 ${rate}` : '写盘速率 —'}</span>
            {info?.title ? (
              <span className={styles.headLiveTitle} title={info.title}>
                {info.title}
              </span>
            ) : null}
          </span>
        ) : undefined
      }
      actions={
        <Button icon={<IconArrowLeft />} onClick={back} aria-label="返回">
          <span className={styles.hideNarrow}>返回</span>
        </Button>
      }
    />
  )

  let body: React.ReactNode
  if (!streamers && (isLoading || !error)) {
    body = (
      <div className={styles.center}>
        <Spin size="large" tip="正在加载直播间…" />
      </div>
    )
  } else if (!streamers) {
    body = (
      <div className={styles.center}>
        <Empty title="加载失败" description="无法获取主播列表，请检查后端连接">
          <Button icon={<IconRefresh />} onClick={() => mutate()}>
            重试
          </Button>
        </Empty>
      </div>
    )
  } else if (!streamer) {
    body = (
      <div className={styles.center}>
        <Empty title={`找不到直播间 #${streamerId}`} description="这个直播间可能已经被删除">
          <Button theme="solid" onClick={() => router.push('/')}>
            去控制台
          </Button>
        </Empty>
      </div>
    )
  } else {
    body = <LiveStage key={streamer.id} streamer={streamer} streamers={streamers} />
  }

  return (
    <>
      {header}
      <PublishQueueBanner />
      {body}
    </>
  )
}

function latestInfo(infos: StreamerInfo[] | undefined, url: string): StreamerInfo | undefined {
  let best: StreamerInfo | undefined
  for (const i of infos ?? []) {
    if (i.url === url && (!best || i.date > best.date)) best = i
  }
  return best
}

/** 一个直播间的播放区 + 标记 + 码率图；按直播间 id 挂载，换直播间即整块重建 */
function LiveStage({ streamer, streamers }: { streamer: LiveStreamerEntity; streamers: LiveStreamerEntity[] }) {
  const { Text } = Typography
  const router = useRouter()
  const { can } = useMe()
  const name = streamer.remark || streamer.url
  const live = streamer.status === LIVE_STATUS
  const available = live && !!streamer.preview?.available
  const previewable = canPreview(streamer)
  // 起播要等容器定下来（canPreview）；起播之后录制端换代时容器会短暂回到「未定」，
  // 这时播放器自己在重连，不因为列表里的一次快照就卸掉它
  const [started, setStarted] = useState(false)
  if (previewable && !started) setStarted(true)
  const showPlayer = available && (previewable || started)

  const danmakuAvailable = !!streamer.preview?.danmaku
  const [danmakuPref, setDanmakuPref] = useBoolPref(DANMAKU_KEY, true)
  const danmakuOn = showPlayer && danmakuAvailable && danmakuPref
  const danmakuFeed = useDanmakuFeed([streamer.id], danmakuOn)
  const [chartOpen, setChartOpen] = useBoolPref(RATE_CHART_KEY, true)
  const transport = usePreviewTransport()
  const [latency, setLatency] = useLatencyProfile()
  const subscribers = streamer.preview?.subscribers
  const maxSubscribers = streamer.preview?.max_subscribers
  const notifiedRef = useRef(false)
  const playerRootRef = useRef<HTMLDivElement>(null)
  const handleFatal = useCallback((text: string) => {
    if (notifiedRef.current) return
    notifiedRef.current = true
    Toast.error({ content: `预览中断：${text}`, duration: 4 })
  }, [])

  // 换直播间：只列正在录、能预览的，当前这一路总在里面。用 replace 不叠历史，
  // 「返回」和浏览器后退回到进预览页之前的页面，而不是上一个直播间
  const switchable = streamers.filter((s) => s.id === streamer.id || canPreview(s))
  const switchTo = (id: number) => {
    if (id !== streamer.id) router.replace(livePageHref(id))
  }

  const sessionId = streamer.session_id ?? null
  const replayReason = !can('file.view')
    ? '当前角色不能查看录像（需要 file.view 权限）'
    : sessionId === null
      ? '这一场还没有写出第一个分段，画面开始写盘后才能回看'
      : null
  const replayButton = (
    <Button
      icon={<IconHistory />}
      disabled={replayReason !== null}
      onClick={() => {
        if (sessionId !== null) router.push(replayHref(sessionId))
      }}
    >
      回看本场
    </Button>
  )

  const cover = live ? liveImageUrl(streamer.id, 'cover', streamer.live_cover_url) : null
  const idleText = !live
    ? '这个直播间现在没有在录制，预览已断开'
    : (previewDisabledReason(streamer) ?? '暂不可预览')

  return (
    <div className={styles.page}>
      <section className={styles.card}>
        <div className={styles.toolbar}>
          {switchable.length > 1 ? (
            <span className={styles.control}>
              <Text type="tertiary" size="small" className={styles.hideNarrow}>
                直播间
              </Text>
              <Select
                size="small"
                value={streamer.id}
                onChange={(v) => switchTo(Number(v))}
                optionList={switchable.map((s) => ({ value: s.id, label: s.remark || s.url }))}
                className={styles.switcher}
                aria-label="切换直播间"
                filter
              />
            </span>
          ) : null}
          <span className={styles.controls}>
            <Tooltip
              content={
                danmakuAvailable
                  ? '显示录制中的实时弹幕（来自本进程的弹幕客户端）'
                  : '这一路没有弹幕客户端：平台不支持，或未开启对应的 *_danmaku 配置（B 站 / 抖音 / 斗鱼 / 虎牙可开）'
              }
            >
              <span className={styles.control}>
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
                <span className={styles.control}>
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
          </span>
        </div>

        <div className={styles.stage}>
          <div ref={playerRootRef} className={styles.screen}>
            {showPlayer ? (
              <LivePreviewPlayer
                streamer={streamer}
                danmakuFeed={danmakuOn ? danmakuFeed : null}
                onFatal={handleFatal}
              />
            ) : (
              <div className={styles.idle} data-state={live ? 'unavailable' : 'offline'}>
                {cover ? (
                  // 封面走同源代理；加载失败只剩底色
                  // eslint-disable-next-line @next/next/no-img-element
                  <img
                    className={styles.idleCover}
                    src={cover}
                    alt=""
                    aria-hidden="true"
                    onError={(e) => {
                      e.currentTarget.style.display = 'none'
                    }}
                  />
                ) : null}
                <span className={styles.idleText}>{idleText}</span>
              </div>
            )}
          </div>
        </div>

        <div className={styles.markRow}>
          <div className={styles.markBar}>
            <LiveMarkBar streamer={streamer} playerRoot={playerRootRef} active />
          </div>
          <span className={styles.replay}>
            {replayReason === null ? (
              replayButton
            ) : (
              <Tooltip content={replayReason}>
                {/* disabled 按钮不触发鼠标事件，包一层让 tooltip 仍能弹出 */}
                <span className={styles.disabledWrap}>{replayButton}</span>
              </Tooltip>
            )}
          </span>
        </div>

        {/* 写盘速率折线：最近 3 分钟；折叠时不加载 uPlot */}
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
            {chartOpen && live ? <LiveRateSummary id={streamer.id} windowMs={MODAL_RATE_WINDOW_MS} /> : null}
          </button>
          {chartOpen && live ? (
            <div id={`live-rate-chart-${streamer.id}`} className={styles.rateBody}>
              <LiveRateChart id={streamer.id} windowMs={MODAL_RATE_WINDOW_MS} variant="full" height={150} label={name} />
            </div>
          ) : null}
        </section>

        <div className={styles.foot}>
          <Text type="tertiary" size="small">
            {transport === 'direct'
              ? '直连模式：能直连的平台由浏览器直接向 CDN 拉流，不经 biliup；不能的自动回落到录制流中转（角标标出原因）。离开这个页面即断开。'
              : '画面来自正在写盘的同一路流，不另外向直播平台拉流；离开这个页面即断开。'}
            {subscribers !== undefined && maxSubscribers ? (
              <>
                {' '}
                这一路当前 {subscribers}/{maxSubscribers} 路中转预览，满了再打开会接替最早的一路。
              </>
            ) : null}
          </Text>
        </div>
      </section>
    </div>
  )
}
