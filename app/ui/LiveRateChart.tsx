'use client'
import React, { useMemo } from 'react'
import dynamic from 'next/dynamic'
import { Switch, Tooltip, Typography } from '@douyinfe/semi-ui'
import type { RateChartVariant } from './RateChart'
import { formatInUnit, rateUnit, readRateSeries, useLiveRates } from '@/app/lib/use-live-rates'
import { useBoolPref } from '@/app/lib/use-local-pref'
import styles from './live-rate-chart.module.scss'

/**
 * 码率折线的懒加载入口。uPlot（约 22 KB gzip）与 `RateChart` 打在独立 chunk 里，
 * 只有页面上真的出现了一张图才下载；首屏包不含它。
 */
const RateChart = dynamic(() => import('./RateChart'), { ssr: false, loading: () => null })

/** 弹层折线的窗口：与浏览器内保留的历史一致 */
export const MODAL_RATE_WINDOW_MS = 3 * 60 * 1000
/** 卡片 / 监视器 sparkline 的窗口 */
export const SPARK_RATE_WINDOW_MS = 60 * 1000

/** 控制台 / 直播管理页卡片 sparkline 的开关（默认关），两页共用一个偏好 */
export const CARD_RATE_CHART_KEY = 'biliup.cards.rateChart'

export function useCardRateChart(): [boolean, (v: boolean) => void] {
  return useBoolPref(CARD_RATE_CHART_KEY, false)
}

/** 工具栏里的「码率图」开关：开启后录制中卡片的封面上叠一条最近 60 秒的写盘速率折线。 */
export function CardRateSwitch({ className }: { className?: string }) {
  const [on, setOn] = useCardRateChart()
  const { Text } = Typography
  return (
    <Tooltip content="开启后，录制中卡片的封面上叠加最近 60 秒的写盘速率折线（一条 WebSocket 每秒推一帧，所有卡片共用）">
      <span className={[styles.switchRow, className].filter(Boolean).join(' ')}>
        <Text type="tertiary" size="small">
          码率图
        </Text>
        <Switch size="small" checked={on} onChange={(v) => setOn(!!v)} aria-label="码率图" />
      </span>
    </Tooltip>
  )
}

export function LiveRateChart({
  id,
  windowMs,
  variant = 'full',
  height,
  label,
  className,
  overlay = false,
}: {
  id: number
  windowMs: number
  variant?: RateChartVariant
  height?: number
  label?: string
  className?: string
  /** 叠在封面 / 画面上：读数条放在图区内部而不是上方 */
  overlay?: boolean
}) {
  const h = height ?? (variant === 'sparkline' ? 36 : 160)
  // 外层定高：chunk 到达前后布局不跳
  return (
    <div
      className={[styles.slot, className].filter(Boolean).join(' ')}
      style={{ height: h }}
      data-variant={variant}
      data-overlay={overlay || undefined}
    >
      <RateChart id={id} windowMs={windowMs} variant={variant} height={h} label={label} overlay={overlay} />
    </div>
  )
}

/** 弹层折线标题里的「当前 / 峰值 / 均值」，与图共用同一份采样与同一个单位。 */
export function LiveRateSummary({ id, windowMs }: { id: number; windowMs: number }) {
  const snap = useLiveRates()
  const series = useMemo(() => readRateSeries(id, windowMs, snap.receivedAt), [id, windowMs, snap])
  const current = snap.latest.get(id) ?? null
  if (!snap.latest.has(id) || series.latest === null) {
    return (
      <span className={styles.summary} data-state="empty">
        —
      </span>
    )
  }
  const unit = rateUnit(series.max)
  let sum = 0
  let n = 0
  for (const v of series.ys) {
    if (v !== null) {
      sum += v
      n += 1
    }
  }
  const avg = n > 0 ? sum / n : 0
  return (
    <span className={styles.summary} data-state="ready">
      <span>
        当前 <b>{current === null ? '—' : formatInUnit(current, unit)}</b>
      </span>
      <span>
        峰值 <b>{formatInUnit(series.max, unit)}</b>
      </span>
      <span>
        均值 <b>{formatInUnit(avg, unit)}</b>
      </span>
    </span>
  )
}
