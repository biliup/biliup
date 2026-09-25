'use client'
import React, { useEffect, useRef } from 'react'
import uPlot from 'uplot'
import 'uplot/dist/uPlot.min.css'
import { formatInUnit, rateUnit } from '@/app/lib/use-live-rates'
import styles from './system-chart.module.scss'

/**
 * 控制台系统状态卡片里的小折线（uPlot）。本模块整体懒加载（见 `SystemStatsPanel`），
 * 与写盘速率图共用 uPlot 的 chunk。
 *
 * - `cpu`：一条线，Y 轴上限随窗口最大值（至少 25%、至多 100%），低负载时曲线不会贴底；
 * - `net`：下行 / 上行两条线，单位按窗口最大值选 KB/s / MB/s。
 *
 * 不画坐标轴，hover 显示时刻与读数。配色每次绘制时从 Semi 的 CSS 变量读，切换深浅色不用重建实例。
 */

export type SystemChartKind = 'cpu' | 'net'

/** CPU 图 Y 轴上限的下限（%） */
const CPU_MIN_TOP = 25

interface Props {
  kind: SystemChartKind
  /** x 为 Unix 秒 */
  xs: number[]
  /** cpu 只用第一条；net 为 [下行, 上行] */
  ys: (number | null)[][]
  windowMs: number
  height: number
}

function themeRgb(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback
  const v = getComputedStyle(document.body).getPropertyValue(name).trim()
  return v || fallback
}
const cpuLine = () => `rgba(${themeRgb('--semi-blue-5', '0,100,250')}, 1)`
const cpuFill = () => `rgba(${themeRgb('--semi-blue-5', '0,100,250')}, 0.14)`
const rxLine = () => `rgba(${themeRgb('--semi-cyan-5', '5,164,182')}, 1)`
const rxFill = () => `rgba(${themeRgb('--semi-cyan-5', '5,164,182')}, 0.12)`
const txLine = () => `rgba(${themeRgb('--semi-violet-5', '114,46,209')}, 1)`

function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n)
}
function hhmmss(sec: number): string {
  const d = new Date(sec * 1000)
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`
}

function seriesMax(u: uPlot): number {
  let max = 0
  for (let i = 1; i < u.data.length; i++) {
    for (const v of u.data[i] ?? []) if (v !== null && v !== undefined && v > max) max = v
  }
  return max
}

function buildOptions(kind: SystemChartKind, width: number, height: number, windowMs: number): uPlot.Options {
  const windowSec = windowMs / 1000
  const series: uPlot.Series[] =
    kind === 'cpu'
      ? [{}, { label: 'CPU', stroke: cpuLine, fill: cpuFill, width: 1.5, spanGaps: false, points: { show: false } }]
      : [
          {},
          { label: '下行', stroke: rxLine, fill: rxFill, width: 1.5, spanGaps: false, points: { show: false } },
          { label: '上行', stroke: txLine, width: 1.5, spanGaps: false, points: { show: false } },
        ]
  return {
    width,
    height,
    legend: { show: false },
    cursor: {
      y: false,
      drag: { x: false, y: false, setScale: false },
      points: { show: true, size: 6 },
    },
    select: { show: false, left: 0, top: 0, width: 0, height: 0 },
    scales: {
      x: {
        time: true,
        range: (u) => {
          const xs = u.data[0]
          const end = xs && xs.length > 0 ? xs[xs.length - 1] : Date.now() / 1000
          return [end - windowSec, end]
        },
      },
      y: {
        range: (u) => {
          const max = seriesMax(u)
          if (kind === 'cpu') return [0, Math.min(100, Math.max(CPU_MIN_TOP, max * 1.15))]
          return [0, max > 0 ? max * 1.15 : 1]
        },
      },
    },
    series,
    axes: [{ show: false }, { show: false }],
  }
}

export default function SystemChart({ kind, xs, ys, windowMs, height }: Props) {
  const hostRef = useRef<HTMLDivElement>(null)
  const readoutRef = useRef<HTMLSpanElement>(null)
  const plotRef = useRef<uPlot | null>(null)
  const dataRef = useRef<uPlot.AlignedData>([xs, ...ys])

  useEffect(() => {
    dataRef.current = [xs, ...ys]
    plotRef.current?.setData(dataRef.current)
  }, [xs, ys])

  useEffect(() => {
    const host = hostRef.current
    const readout = readoutRef.current
    if (!host) return
    const width = Math.max(1, Math.floor(host.clientWidth))
    const opts = buildOptions(kind, width, height, windowMs)
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
          let text: string
          if (kind === 'cpu') {
            const v = u.data[1][idx]
            text = v === null || v === undefined ? '—' : `${v.toFixed(1)}%`
          } else {
            const unit = rateUnit(seriesMax(u))
            const rx = u.data[1][idx]
            const tx = u.data[2][idx]
            const fmt = (v: number | null | undefined) => (v === null || v === undefined ? '—' : formatInUnit(v, unit))
            text = `↓ ${fmt(rx)}  ↑ ${fmt(tx)}`
          }
          readout.textContent = `${hhmmss(x)} · ${text}`
          readout.hidden = false
          const dpr = window.devicePixelRatio || 1
          const maxLeft = Math.max(0, u.bbox.width / dpr - readout.offsetWidth)
          const left = Math.min(Math.max(0, u.cursor.left ?? 0), maxLeft)
          readout.style.transform = `translateX(${Math.round(left)}px)`
        },
      ],
    }
    const plot = new uPlot(opts, dataRef.current, host)
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
  }, [kind, windowMs, height])

  return (
    <div className={styles.host} ref={hostRef} style={{ height }}>
      <span className={styles.readout} ref={readoutRef} hidden aria-hidden="true" />
    </div>
  )
}
