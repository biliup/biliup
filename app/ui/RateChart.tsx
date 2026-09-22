'use client'
import React, { useEffect, useMemo, useRef } from 'react'
import uPlot from 'uplot'
import 'uplot/dist/uPlot.min.css'
import { formatInUnit, rateUnit, readRateSeries, useLiveRates, type RateSeries } from '@/app/lib/use-live-rates'
import styles from './rate-chart.module.scss'

/**
 * 写盘速率折线（uPlot）。本模块整体懒加载（消费方用 `next/dynamic` 引入），uPlot 与它的 CSS
 * 只在第一次真正要画图时才下载，不进首屏。
 *
 * 两种形态：
 * - `full`：弹层底部的折线，带时间轴与 Y 轴（单位按窗口最大值自动选 KB/s / MB/s），hover 读数；
 * - `sparkline`：卡片 / 监视器小窗里的一条线，不画坐标轴，hover 显示数值与时刻。
 *
 * 数据来自 `useLiveRates()` 共享的采样历史；uPlot 实例在 effect 里创建 / 销毁，渲染期不碰 ref。
 * 配色在每次绘制时从 Semi 的 CSS 变量读取，切换深浅色不用重建实例。uPlot 本身没有动画，
 * 因此 `prefers-reduced-motion` 下没有需要关的东西；样式里也不加过渡。
 * 没有任何采样（未录制 / 断流 / 下载器不计字节）时只显示「—」占位，不画空图。
 */

export type RateChartVariant = 'full' | 'sparkline'

interface Props {
  /** 直播间 id */
  id: number
  /** 展示的时间窗口（毫秒） */
  windowMs: number
  variant?: RateChartVariant
  /** 图高（CSS px）；宽随容器 */
  height?: number
  className?: string
  /** 可访问名称前缀 */
  label?: string
}

const FONT = '11px system-ui, -apple-system, "Segoe UI", sans-serif'

/** 读 Semi 主题变量。`--semi-green-5` 是 `r,g,b` 三元组，方便拼半透明填充。 */
function themeColor(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback
  const v = getComputedStyle(document.body).getPropertyValue(name).trim()
  return v || fallback
}
const lineColor = () => `rgba(${themeColor('--semi-green-5', '59,179,70')}, 1)`
const fillColor = () => `rgba(${themeColor('--semi-green-5', '59,179,70')}, 0.16)`
const gridColor = () => themeColor('--semi-color-border', 'rgba(28,31,35,0.08)')
const axisColor = () => themeColor('--semi-color-text-2', 'rgba(28,31,35,0.62)')

function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n)
}
function hhmmss(sec: number): string {
  const d = new Date(sec * 1000)
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`
}

function seriesMax(u: uPlot): number {
  let max = 0
  for (const v of u.data[1] ?? []) if (v !== null && v !== undefined && v > max) max = v
  return max
}

/** Y 轴从 0 起，顶上留 15% 余量；全 0 时给 1 免得刻度退化 */
function yRange(u: uPlot): uPlot.Range.MinMax {
  const max = seriesMax(u)
  return [0, max > 0 ? max * 1.15 : 1]
}

/** 横轴始终是「最新一点往前 windowSec 秒」：只有几个点时曲线从右端往左长，不会先铺满再压缩 */
function xRange(windowSec: number) {
  return (u: uPlot): uPlot.Range.MinMax => {
    const xs = u.data[0]
    const end = xs && xs.length > 0 ? xs[xs.length - 1] : Date.now() / 1000
    return [end - windowSec, end]
  }
}

function buildOptions(variant: RateChartVariant, width: number, height: number, windowMs: number): uPlot.Options {
  const spark = variant === 'sparkline'
  return {
    width,
    height,
    // 图例用自己的读数条替代；uPlot 的表格图例会撑开高度
    legend: { show: false },
    // 只允许 hover 读数，不做框选缩放（缩放对实时滚动窗口没有意义）
    cursor: {
      x: !spark,
      y: false,
      drag: { x: false, y: false, setScale: false },
      points: { show: true, size: spark ? 6 : 8, fill: lineColor, stroke: lineColor },
      lock: false,
    },
    select: { show: false, left: 0, top: 0, width: 0, height: 0 },
    scales: {
      x: { time: true, range: xRange(windowMs / 1000) },
      y: { range: yRange },
    },
    series: [
      {},
      {
        label: '写盘速率',
        stroke: lineColor,
        fill: spark ? fillColor : undefined,
        width: spark ? 1.5 : 2,
        // 断口（null）不连线：录制中断的那段就是空白
        spanGaps: false,
        points: { show: false },
      },
    ],
    axes: spark
      ? [{ show: false }, { show: false }]
      : [
          {
            stroke: axisColor,
            grid: { stroke: gridColor, width: 1 },
            ticks: { stroke: gridColor, width: 1, size: 4 },
            font: FONT,
            size: 22,
            gap: 4,
            values: (_u, splits) => splits.map(hhmmss),
          },
          {
            stroke: axisColor,
            grid: { stroke: gridColor, width: 1 },
            ticks: { show: false },
            font: FONT,
            size: 58,
            gap: 6,
            // 四格等分、0 在底
            splits: (_u, _axisIdx, _scaleMin, scaleMax) => [0, 1, 2, 3, 4].map((k) => (scaleMax / 4) * k),
            values: (u, splits) => {
              const unit = rateUnit(seriesMax(u))
              return splits.map((s) => (s === 0 ? '0' : formatInUnit(s, unit)))
            },
          },
        ],
  }
}

/**
 * 真正持有 uPlot 实例的部分；外层已确认窗口内有采样才挂它。
 * 创建 / 销毁与数据更新分成两个 effect：StrictMode 双次执行时第一份实例在清理里销毁，
 * 不会留下多余的 canvas。
 */
function Plot({
  id,
  windowMs,
  variant,
  height,
  series,
}: {
  id: number
  windowMs: number
  variant: RateChartVariant
  height: number
  series: RateSeries
}) {
  const hostRef = useRef<HTMLDivElement>(null)
  const readoutRef = useRef<HTMLSpanElement>(null)
  const plotRef = useRef<uPlot | null>(null)

  useEffect(() => {
    const host = hostRef.current
    const readout = readoutRef.current
    if (!host) return
    const width = Math.max(1, Math.floor(host.clientWidth))
    const opts = buildOptions(variant, width, height, windowMs)
    // hover 读数：直接写 DOM，不走 React 状态（mousemove 频率高，不值得重渲染）
    opts.hooks = {
      setCursor: [
        (u) => {
          if (!readout) return
          const idx = u.cursor.idx
          if (idx === null || idx === undefined) {
            readout.hidden = true
            return
          }
          const x = u.data[0][idx]
          const y = u.data[1][idx]
          const text = y === null || y === undefined ? '—' : formatInUnit(y, rateUnit(seriesMax(u)))
          readout.textContent = `${hhmmss(x)} · ${text}`
          readout.hidden = false
          // 跟着光标左右走，不出图区
          const plotLeft = u.bbox.left / (window.devicePixelRatio || 1)
          const left = plotLeft + Math.min(Math.max(0, u.cursor.left ?? 0), Math.max(0, u.bbox.width / (window.devicePixelRatio || 1) - readout.offsetWidth))
          readout.style.transform = `translateX(${Math.round(left)}px)`
        },
      ],
    }
    // 初始数据直接从共享历史读（import 异步、props 可能已经过时），随后的更新走下面那个 effect
    const initial = readRateSeries(id, windowMs, Date.now())
    const plot = new uPlot(opts, [initial.xs, initial.ys], host)
    plotRef.current = plot
    const ro = new ResizeObserver(() => {
      const w = Math.floor(host.clientWidth)
      if (w > 0 && w !== plot.width) plot.setSize({ width: w, height })
    })
    ro.observe(host)
    return () => {
      ro.disconnect()
      plot.destroy()
      plotRef.current = null
      if (readout) readout.hidden = true
    }
  }, [id, windowMs, variant, height])

  // 新一帧到达：只换数据，不重建实例
  useEffect(() => {
    plotRef.current?.setData([series.xs, series.ys])
  }, [series])

  return (
    <div className={styles.plotHost} ref={hostRef} data-variant={variant} style={{ height }}>
      <span className={styles.readout} ref={readoutRef} hidden aria-hidden="true" />
    </div>
  )
}

export default function RateChart({ id, windowMs, variant = 'full', height, className, label }: Props) {
  const snap = useLiveRates()
  // 环形缓冲只随快照版本变化，快照引用就是重新读序列的触发键；`receivedAt` 让渲染期的读取保持纯粹
  const series = useMemo(() => readRateSeries(id, windowMs, snap.receivedAt), [id, windowMs, snap])
  const h = height ?? (variant === 'sparkline' ? 36 : 160)
  const unit = rateUnit(series.max)
  const wrapCls = [styles.chart, className].filter(Boolean).join(' ')

  // 房间不在最近一帧里（停录 / 下播 / 接口不可用）或窗口内从没有过采样（yt-dlp 等不计字节）→ 只给「—」
  const recording = snap.latest.has(id)
  if (!recording || series.latest === null) {
    return (
      <div
        className={wrapCls}
        data-variant={variant}
        data-state={snap.error ? 'error' : 'empty'}
        style={{ height: h }}
        role="img"
        aria-label={label ? `${label}：暂无采样` : '写盘速率暂无采样'}
        title={snap.error ? `速率接口不可用：${snap.error}` : recording ? '尚无写盘速率采样' : '未在录制'}
      >
        <span className={styles.placeholder}>—</span>
      </div>
    )
  }

  const current = formatInUnit(series.latest, unit)
  return (
    <div
      className={wrapCls}
      data-variant={variant}
      data-state="ready"
      role="img"
      aria-label={label ? `${label}：当前 ${current}` : `写盘速率 ${current}`}
    >
      <Plot id={id} windowMs={windowMs} variant={variant} height={h} series={series} />
    </div>
  )
}
