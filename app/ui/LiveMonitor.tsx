'use client'
import React, { useState } from 'react'
import useSWR from 'swr'
import { Button, Empty, Select, Switch, Tag, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconPlay, IconStop } from '@douyinfe/semi-icons'
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
import { useBoolPref, useChoicePref } from '@/app/lib/use-local-pref'
import { useDanmakuFeed } from '@/app/lib/danmaku-feed'
import { LivePreviewPlayer, useLatencyProfile } from './LivePreview'
import { type LatencyProfile, RELAY_PROFILES, RELAY_PROFILES_SEGMENTED } from '@/app/lib/live-buffer'
import { LiveRateChart, SPARK_RATE_WINDOW_MS } from './LiveRateChart'
import { MarkerCount } from './MarkerControls'
import styles from './live-monitor.module.scss'

/**
 * 监视器默认同屏路数。每路小窗占用对应直播间的 1 个预览连接
 * （服务端每间上限见 `PREVIEW_MAX_SUBSCRIBERS_PER_ROOM`，进程上限 16），改这里即可换默认值。
 */
export const DEFAULT_MONITOR_TILES = 4
/**
 * 浏览器对同一主机的 HTTP/1.1 并发连接只有 6 个：每路小窗占 1 个长连接，弹幕复用 1 个，
 * 列表轮询还要留 1 个，所以 HTTP/1.1 下最多 4 路；页面走 HTTP/2、HTTP/3（TLS 反代）时不受此限，
 * 放开 6 / 9 路。
 */
export const MONITOR_TILE_OPTIONS_H1 = [1, 2, 4]
export const MONITOR_TILE_OPTIONS_H2 = [1, 2, 4, 6, 9]

function tileOptions(): number[] {
  if (typeof window === 'undefined') return MONITOR_TILE_OPTIONS_H1
  const nav = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming | undefined
  const proto = nav?.nextHopProtocol ?? ''
  return proto === 'h2' || proto === 'h3' ? MONITOR_TILE_OPTIONS_H2 : MONITOR_TILE_OPTIONS_H1
}
const TILES_KEY = 'biliup.monitor.maxTiles'
/** 多路同屏默认不开弹幕（每路一条 SSE、多路叠加太吵），开关记在本地 */
const DANMAKU_KEY = 'biliup.monitor.danmaku'
/** 每路小窗下的 60 s 码率 sparkline，默认关（多路同时画 + 每秒轮询），开关记在本地 */
const RATE_CHART_KEY = 'biliup.monitor.rateChart'

/** 用户的选择：手动激活的顺序（先进先出）与手动停掉的房间；其余由数据推导。 */
interface Selection {
  order: number[]
  stopped: number[]
}

/**
 * 由「用户选择 + 当前可预览房间 + 路数上限」推导正在播放的房间列表（纯函数，无副作用）：
 * 先保留用户激活且仍可预览的，再按列表顺序自动补满，手动停掉的不自动补。
 */
export function resolveActive(
  selection: Selection,
  previewableIds: number[],
  maxTiles: number
): number[] {
  const previewable = new Set(previewableIds)
  const stopped = new Set(selection.stopped)
  const active: number[] = []
  for (const id of selection.order) {
    if (previewable.has(id) && !active.includes(id)) active.push(id)
  }
  for (const id of previewableIds) {
    if (active.length >= maxTiles) break
    if (!active.includes(id) && !stopped.has(id)) active.push(id)
  }
  return active.slice(Math.max(0, active.length - maxTiles))
}

/**
 * 实时监视：所有正在录制的直播间的小窗网格。
 * 可预览且在路数上限内的直接播放（默认静音）；超出的显示封面，点击替换最早开始的一路；
 * 下载器不能旁路的显示封面与原因。独立组件，可整体挪到别的页面。
 */
export default function LiveMonitor() {
  const { Text } = Typography
  const { data: streamers, error, isLoading } = useSWR<LiveStreamerEntity[]>(
    '/v1/streamers',
    fetcher,
    { refreshInterval: STREAMERS_REFRESH_MS }
  )
  const { data: infos } = useSWR<StreamerInfo[]>('/v1/streamer-info', fetcher, {
    refreshInterval: SLOW_REFRESH_MS,
  })
  const options = tileOptions()
  const [maxTiles, setMaxTiles] = useChoicePref(TILES_KEY, options, DEFAULT_MONITOR_TILES)
  const [danmaku, setDanmaku] = useBoolPref(DANMAKU_KEY, false)
  const [latency, setLatency] = useLatencyProfile()
  const transport = usePreviewTransport()
  const [rateChart, setRateChart] = useBoolPref(RATE_CHART_KEY, false)
  const [selection, setSelection] = useState<Selection>({ order: [], stopped: [] })

  const live = (streamers ?? []).filter((s) => s.status === LIVE_STATUS)
  const previewable = live.filter(canPreview)
  const previewableIds = previewable.map((s) => s.id)
  const active = resolveActive(selection, previewableIds, maxTiles)
  const activeSet = new Set(active)
  // 正在播且平台有弹幕客户端的那几路共用一条弹幕连接
  const danmakuIds = previewable.filter((s) => activeSet.has(s.id) && s.preview?.danmaku).map((s) => s.id)
  const danmakuFeed = useDanmakuFeed(danmakuIds, danmaku)

  const infoByUrl = new Map<string, StreamerInfo>()
  for (const i of infos ?? []) {
    const cur = infoByUrl.get(i.url)
    if (!cur || i.date > cur.date) infoByUrl.set(i.url, i)
  }

  // 激活：满了就替换最早开始的一路（先进先出）
  const activate = (id: number) => {
    setSelection((prev) => {
      const current = resolveActive(prev, previewableIds, maxTiles)
      const kept = current.length >= maxTiles ? current.slice(1) : current
      return {
        order: [...kept.filter((x) => x !== id), id],
        stopped: prev.stopped.filter((x) => x !== id),
      }
    })
  }
  const stop = (id: number) => {
    setSelection((prev) => ({
      order: prev.order.filter((x) => x !== id),
      stopped: prev.stopped.includes(id) ? prev.stopped : [...prev.stopped, id],
    }))
  }

  if (error) {
    return (
      <div className={styles.center}>
        <Empty title="加载失败" description="无法获取主播列表，请检查后端连接" />
      </div>
    )
  }
  if (!isLoading && live.length === 0) {
    return (
      <div className={styles.center}>
        <Empty
          title="当前没有正在录制的直播间"
          description="开播并开始录制后，这里会以小窗形式同屏显示各路画面（stream-gears / mesio 下载器）"
        />
      </div>
    )
  }

  const full = active.length >= maxTiles
  return (
    <section className={styles.monitor}>
      <div className={styles.toolbar}>
        <div className={styles.summary}>
          <Text strong>
            正在监视 {active.length} / {maxTiles} 路
          </Text>
          <Text type="tertiary" size="small">
            录制中 {live.length} 间 · 可预览 {previewable.length} 间
          </Text>
        </div>
        <div className={styles.controls}>
          <Tooltip content="所有小窗一起开关；只有平台实现了弹幕客户端的房间会显示">
            <span className={styles.switchRow}>
              <Text type="tertiary" size="small">
                弹幕
              </Text>
              <Switch size="small" checked={danmaku} onChange={(v) => setDanmaku(!!v)} aria-label="弹幕" />
            </span>
          </Tooltip>
          {transport === 'relay' ? (
            <>
              <Tooltip
                content={`中转缓冲深度，也就是画面延迟。低延迟：FLV 约 ${RELAY_PROFILES.low.target} s、HLS 分片流约 ${RELAY_PROFILES_SEGMENTED.low.target} s，卡了的小窗自动切到流畅；流畅：约 ${RELAY_PROFILES.smooth.target} s`}
              >
                <Text type="tertiary" size="small">
                  延迟
                </Text>
              </Tooltip>
              <Select
                size="small"
                value={latency}
                onChange={(v) => setLatency(v as LatencyProfile)}
                optionList={[
                  { value: 'low', label: '低延迟' },
                  { value: 'smooth', label: '流畅' },
                ]}
                style={{ width: 96 }}
                aria-label="中转延迟"
              />
            </>
          ) : null}
          <Tooltip content="每路小窗下方显示最近 60 秒的写盘速率折线（每秒向后端拉一次速率，所有小窗共用一次请求）">
            <span className={styles.switchRow}>
              <Text type="tertiary" size="small">
                码率图
              </Text>
              <Switch size="small" checked={rateChart} onChange={(v) => setRateChart(!!v)} aria-label="码率图" />
            </span>
          </Tooltip>
          <Text type="tertiary" size="small">
            同屏路数
          </Text>
          <Select
            size="small"
            value={maxTiles}
            onChange={(v) => setMaxTiles(Number(v))}
            optionList={options.map((n) => ({ value: n, label: `${n} 路` }))}
            style={{ width: 88 }}
            aria-label="同屏路数"
          />
        </div>
      </div>
      <Text type="tertiary" size="small" className={styles.hint}>
        画面来自各直播间正在写盘的同一路流，默认静音；每路小窗占用该直播间的 1 个预览连接。
        超出路数的直播间显示封面，点击可替换最早开始的一路。
      </Text>
      <div className={styles.grid}>
        {live.map((s) => {
          const info = infoByUrl.get(s.url)
          const playing = activeSet.has(s.id)
          const can = canPreview(s)
          const reason = can ? null : previewDisabledReason(s)
          const cover = liveImageUrl(s.id, 'cover', s.live_cover_url)
          const rate = formatRate(s.live_bytes_per_sec)
          const name = s.remark || s.url
          return (
            <article
              key={s.id}
              className={`${styles.tile} ${playing ? styles.tilePlaying : ''}`}
              data-id={s.id}
              data-state={playing ? 'playing' : can ? 'idle' : 'unavailable'}
            >
              <header className={styles.tileHead}>
                <span className={styles.tileName} title={name}>
                  {name}
                </span>
                <Tag size="small" color="grey">
                  {platformName(s.url)}
                </Tag>
                {previewFormatLabel(s.preview?.format) ? (
                  <Tag size="small" color="green">
                    {previewFormatLabel(s.preview?.format)}
                  </Tag>
                ) : null}
                <MarkerCount count={s.marker_count} compact />
                <span className={styles.tileRate}>{rate ?? '—'}</span>
                {playing ? (
                  <Tooltip content="停止这一路">
                    <Button
                      size="small"
                      theme="borderless"
                      type="tertiary"
                      icon={<IconStop />}
                      aria-label={`停止 ${name}`}
                      onClick={() => stop(s.id)}
                    />
                  </Tooltip>
                ) : null}
              </header>
              {playing ? (
                <LivePreviewPlayer streamer={s} muted compact danmakuFeed={danmaku ? danmakuFeed : null} />
              ) : (
                <button
                  type="button"
                  className={styles.idle}
                  disabled={!can}
                  onClick={() => activate(s.id)}
                  aria-label={
                    can ? (full ? `播放 ${name}（替换最早的一路）` : `播放 ${name}`) : `${name}：${reason}`
                  }
                >
                  {cover ? (
                    // 封面走同源代理；失败时隐藏，只剩底色，说明文字仍在
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
                  <span className={styles.idleText}>
                    {can ? (
                      <>
                        <IconPlay />
                        {full ? '点击替换最早的一路' : '点击开始预览'}
                      </>
                    ) : (
                      reason
                    )}
                  </span>
                </button>
              )}
              {rateChart ? (
                <LiveRateChart
                  id={s.id}
                  windowMs={SPARK_RATE_WINDOW_MS}
                  variant="sparkline"
                  height={32}
                  label={`${name} 写盘速率`}
                  className={styles.tileSpark}
                />
              ) : null}
              <footer className={styles.tileFoot} title={info?.title || ''}>
                {info?.title || '\u00a0'}
              </footer>
            </article>
          )
        })}
      </div>
    </section>
  )
}
