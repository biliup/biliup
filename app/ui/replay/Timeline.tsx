'use client'
import React, { useEffect, useRef, useState } from 'react'
import { Popover, Tooltip } from '@douyinfe/semi-ui'
import { IconFlag } from '@douyinfe/semi-icons'
import type { Marker } from '@/app/lib/markers'
import { formatSessionTime, markerClusters } from '@/app/lib/markers'
import {
  ceilPoint,
  floorPoint,
  formatPrecise,
  formatSpan,
  type Gap,
  isReadable,
  nearestPoint,
  type SegmentView,
  segmentAt,
  segmentEnd,
} from '@/app/lib/sessions'
import styles from './replay.module.scss'

export interface Selection {
  in: number | null
  out: number | null
}

const STATE_LABEL: Record<SegmentView['state'], string> = {
  recording: '正在录',
  finished: '已录完',
  missing: '文件不见了',
  deleted: '录像已清理',
  pending_delete: '等待清理（仍可回看）',
}

/** 不吸附时 ← / → 挪多少，也是入点、出点之间的最小距离 */
const FREE_STEP_MS = 100

function pct(t: number, from: number, to: number): number {
  return ((t - from) / Math.max(1, to - from)) * 100
}

function clampPct(t: number, from: number, to: number): number {
  return Math.min(100, Math.max(0, pct(t, from, to)))
}

/** 在 [from, to] 里挑一个刻度间隔，让标签不超过 `maxLabels` 个 */
function tickStep(span: number, maxLabels: number): number {
  const steps = [5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600, 7200, 10800, 21600].map((s) => s * 1000)
  return steps.find((s) => span / s <= maxLabels) ?? steps[steps.length - 1]
}

function Ruler({ from, to, maxLabels }: { from: number; to: number; maxLabels: number }) {
  const step = tickStep(to - from, maxLabels)
  const ticks: number[] = []
  for (let t = Math.ceil(from / step) * step; t <= to; t += step) ticks.push(t)
  return (
    <div className={styles.ruler} aria-hidden="true">
      {ticks.map((t) => (
        <span key={t} className={styles.rulerTick} style={{ left: `${pct(t, from, to)}%` }}>
          {formatSessionTime(t)}
        </span>
      ))}
    </div>
  )
}

function msAt(e: { clientX: number }, el: HTMLElement, from: number, to: number): number {
  const rect = el.getBoundingClientRect()
  const x = Math.min(rect.width, Math.max(0, e.clientX - rect.left))
  return from + (x / Math.max(1, rect.width)) * (to - from)
}

function segmentTitle(s: SegmentView, index: number): string {
  const end = s.end_ms === null ? '…' : formatSessionTime(s.end_ms)
  return `第 ${index + 1} 段（${s.container.toUpperCase()}）${formatSessionTime(s.start_ms)}–${end} · ${STATE_LABEL[s.state]}`
}

function Segments({ segments, from, to }: { segments: SegmentView[]; from: number; to: number }) {
  return (
    <>
      {segments.map((s, i) => {
        const end = Math.max(segmentEnd(s), s.start_ms)
        if (end < from || s.start_ms > to) return null
        const left = clampPct(s.start_ms, from, to)
        const width = Math.max(0.15, clampPct(end, from, to) - left)
        return (
          <div
            key={s.id}
            className={styles.segment}
            data-state={s.state}
            data-readable={isReadable(s) || undefined}
            style={{ left: `${left}%`, width: `${width}%` }}
            title={segmentTitle(s, i)}
          />
        )
      })}
    </>
  )
}

function Gaps({ gaps, from, to }: { gaps: Gap[]; from: number; to: number }) {
  return (
    <>
      {gaps.map((g) => {
        if (g.to_ms < from || g.from_ms > to) return null
        const left = clampPct(g.from_ms, from, to)
        const width = Math.max(0.2, clampPct(g.to_ms, from, to) - left)
        return (
          <div
            key={`${g.from_ms}-${g.to_ms}`}
            className={styles.gap}
            style={{ left: `${left}%`, width: `${width}%` }}
            title={`断流 ${formatSpan(g.to_ms - g.from_ms)}（${formatSessionTime(g.from_ms)}–${formatSessionTime(g.to_ms)}），这段时间没有录到画面`}
          />
        )
      })}
    </>
  )
}

function SelectionBand({ selection, from, to }: { selection: Selection; from: number; to: number }) {
  if (selection.in === null || selection.out === null) return null
  if (selection.out < from || selection.in > to) return null
  const left = clampPct(selection.in, from, to)
  const width = clampPct(selection.out, from, to) - left
  return <div className={styles.selection} style={{ left: `${left}%`, width: `${width}%` }} aria-hidden="true" />
}

function Pins({
  markers,
  from,
  to,
  onPick,
}: {
  markers: Marker[]
  from: number
  to: number
  onPick: (m: Marker) => void
}) {
  const [hovered, setShown] = useState<number | null>(null)
  const [focused, setFocused] = useState<number | null>(null)
  const shown = hovered ?? focused
  const [opened, setOpened] = useState<number | null>(null)
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  useEffect(() => () => { if (hoverTimer.current !== null) clearTimeout(hoverTimer.current) }, [])
  const enterMenu = (id: number) => {
    if (hoverTimer.current !== null) clearTimeout(hoverTimer.current)
    setShown(id)
  }
  const leaveMenu = () => { hoverTimer.current = setTimeout(() => setShown(null), 150) }
  const closeMenu = () => {
    if (hoverTimer.current !== null) clearTimeout(hoverTimer.current)
    setOpened(null); setShown(null); setFocused(null)
  }
  const visible = markerClusters(markers, from, to)
  const description = (m: Marker) => `${formatPrecise(m.at_ms)} · ${m.label || '未命名标记'} · 前 ${formatSpan(m.lookback_ms)}、后 ${formatSpan(m.lookahead_ms)} · 范围 ${formatPrecise(Math.max(0, m.at_ms - m.lookback_ms))} → ${formatPrecise(m.at_ms + m.lookahead_ms)}`
  return (
    <>
      {visible.map((cluster) => {
        const m = cluster[0]
        const position = pct(m.at_ms, from, to)
        const range = `${formatPrecise(Math.max(0, m.at_ms - m.lookback_ms))} → ${formatPrecise(m.at_ms + m.lookahead_ms)}`
        const title = description(m)
        if (cluster.length > 1) return <Popover key={m.id} trigger="custom" position="top" visible={opened === m.id || shown === m.id}
          guardFocus={false} returnFocusOnClose={false} onEscKeyDown={closeMenu}
          onClickOutSide={closeMenu} content={
            <div className={styles.markerClusterList} role="group" aria-label="重合标记" data-marker-cluster-menu={m.id}
              onMouseEnter={() => enterMenu(m.id)} onMouseLeave={leaveMenu}
              onFocusCapture={() => setFocused(m.id)} onBlurCapture={event => {
                if (!(event.relatedTarget instanceof Node) || !event.currentTarget.contains(event.relatedTarget)) setFocused(null)
              }}>
              {cluster.map(marker => <button type="button" key={marker.id} data-marker-id={marker.id}
                className={styles.markerClusterItem} style={marker.color ? { color: marker.color } : undefined}
                onPointerDown={event => event.stopPropagation()}
                onClick={event => { event.stopPropagation(); closeMenu(); onPick(marker) }}>
                <IconFlag size="small" aria-hidden="true" /><span>{description(marker)}</span>
              </button>)}
            </div>
          }>
          <button type="button" className={styles.pin} data-marker-cluster={cluster.map(marker => marker.id).join(',')}
            style={{ left: `${position}%`, ...(m.color ? { color: m.color } : null) }}
            aria-label={`这里有 ${cluster.length} 个标记，打开列表选择。${cluster.map(description).join('；')}`}
            aria-expanded={opened === m.id || shown === m.id}
            onFocus={() => setFocused(m.id)} onBlur={event => {
              if (!(event.relatedTarget instanceof Element) || !event.relatedTarget.closest(`[data-marker-cluster-menu="${m.id}"]`)) setFocused(null)
            }}
            onMouseEnter={() => enterMenu(m.id)} onMouseLeave={leaveMenu}
            onPointerDown={event => event.stopPropagation()} onClick={event => { event.stopPropagation(); setOpened(opened === m.id ? null : m.id) }}
            onKeyDown={event => {
              if (!['Enter', ' ', 'ArrowDown'].includes(event.key)) return
              event.preventDefault(); event.stopPropagation(); setOpened(m.id)
              setTimeout(() => document.querySelector<HTMLButtonElement>(`[data-marker-cluster-menu="${m.id}"] button`)?.focus(), 0)
            }}>
            <IconFlag size="small" aria-hidden="true" /><span className={styles.markerCount}>{cluster.length}</span>
          </button>
        </Popover>
        return (
          <Tooltip key={m.id} trigger="custom" visible={shown === m.id} position="top" content={
            <div className={styles.markerTip}>
              <strong>{formatPrecise(m.at_ms)} · {m.label || '未命名标记'}</strong>
              <span>前 {formatSpan(m.lookback_ms)} · 后 {formatSpan(m.lookahead_ms)}</span>
              <span>{range}{m.color ? ` · ${m.color}` : ''}</span>
            </div>
          }>
            <button
              type="button"
              className={styles.pin}
              data-marker-id={m.id}
              style={{ left: `${position}%`, ...(m.color ? { color: m.color } : null) }}
              aria-label={title}
              onFocus={() => setFocused(m.id)}
              onBlur={() => setFocused(null)}
              onMouseEnter={() => setShown(m.id)}
              onMouseLeave={() => setShown(null)}
              onPointerDown={e => e.stopPropagation()}
              onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') e.stopPropagation() }}
              onClick={(e) => {
                e.stopPropagation()
                onPick(m)
              }}
            >
              <IconFlag size="small" aria-hidden="true" />
            </button>
          </Tooltip>
        )
      })}
    </>
  )
}

/** 概览条上的候选色带 */
export interface SuggestionBand {
  id: number
  from: number
  to: number
  label: string
  expired: boolean
}

function Bands({
  bands,
  to,
  current,
  onPick,
}: {
  bands: SuggestionBand[]
  to: number
  current: number | null
  onPick: (id: number) => void
}) {
  return (
    <>
      {bands.map((b) => {
        const left = clampPct(b.from, 0, to)
        const width = Math.max(0.3, clampPct(b.to, 0, to) - left)
        const title = `候选 ${formatSessionTime(b.from)}–${formatSessionTime(b.to)} ${b.label}${b.expired ? '（已过期）' : ''}`
        return (
          <button
            type="button"
            key={b.id}
            className={styles.suggestionBand}
            data-suggestion-band={b.id}
            data-expired={b.expired || undefined}
            data-current={current === b.id || undefined}
            style={{ left: `${left}%`, width: `${width}%` }}
            title={title}
            aria-label={title}
            onClick={(e) => {
              e.stopPropagation()
              onPick(b.id)
            }}
          />
        )
      })}
    </>
  )
}

/**
 * 概览条：整场时间轴。分段色块（已清理的画灰）、断流缺口、标记针、选段、播放头和细节条的范围；
 * 点哪里就跳到哪里（已清理的录像不能回看）。有候选时下沿画出候选色带，点色带跳到那条候选。
 */
export function OverviewBar({
  duration,
  segments,
  gaps,
  markers,
  selection,
  playhead,
  live,
  detailWindow,
  onSeek,
  onBlocked,
  onPickMarker,
  bands = [],
  currentBand = null,
  onPickBand,
}: {
  duration: number
  segments: SegmentView[]
  gaps: Gap[]
  markers: Marker[]
  selection: Selection
  playhead: number | null
  live: boolean
  detailWindow: [number, number] | null
  onSeek: (ms: number) => void
  onBlocked: (segment: SegmentView) => void
  onPickMarker: (m: Marker) => void
  bands?: SuggestionBand[]
  currentBand?: number | null
  onPickBand?: (id: number) => void
}) {
  const trackRef = useRef<HTMLDivElement>(null)
  const [hover, setHover] = useState<number | null>(null)
  const to = Math.max(1000, duration)
  const seekValue = Math.max(0, Math.min(to, playhead ?? 0))
  const seekTo = (ms: number) => {
    const target = Math.max(0, Math.min(to, ms))
    const seg = segmentAt(segments, target)
    if (seg && !isReadable(seg)) {
      onBlocked(seg)
      return
    }
    onSeek(target)
  }
  const pick = (e: React.MouseEvent) => {
    if (!trackRef.current) return
    seekTo(msAt(e, trackRef.current, 0, to))
  }
  return (
    <div className={styles.overview}>
      <div
        ref={trackRef}
        className={styles.overviewTrack}
        onClick={pick}
        onMouseMove={(e) => trackRef.current && setHover(msAt(e, trackRef.current, 0, to))}
        onMouseLeave={() => setHover(null)}
      >
        <Segments segments={segments} from={0} to={to} />
        <Gaps gaps={gaps} from={0} to={to} />
        {detailWindow ? (
          <div
            className={styles.window}
            style={{
              left: `${clampPct(detailWindow[0], 0, to)}%`,
              width: `${clampPct(detailWindow[1], 0, to) - clampPct(detailWindow[0], 0, to)}%`,
            }}
            aria-hidden="true"
          />
        ) : null}
        <SelectionBand selection={selection} from={0} to={to} />
        {onPickBand && bands.length ? <Bands bands={bands} to={to} current={currentBand} onPick={onPickBand} /> : null}
        <Pins markers={markers} from={0} to={to} onPick={onPickMarker} />
        {playhead !== null ? (
          <div
            className={styles.playhead}
            data-live={live || undefined}
            style={{ left: `${clampPct(playhead, 0, to)}%` }}
            aria-hidden="true"
          />
        ) : null}
        {hover !== null ? (
          <span className={styles.hoverTime} style={{ left: `${clampPct(hover, 0, to)}%` }} aria-hidden="true">
            {formatSessionTime(hover)}
          </span>
        ) : null}
        <input
          className={styles.overviewSeekInput}
          type="range"
          min={0}
          max={to}
          step={1000}
          value={seekValue}
          aria-label="场次播放位置"
          aria-valuetext={formatSessionTime(seekValue)}
          onChange={(e) => seekTo(Number(e.currentTarget.value))}
        />
      </div>
      <Ruler from={0} to={to} maxLabels={8} />
    </div>
  )
}

/**
 * 细节条：当前位置前后各 5 分钟。关键帧刻度、入点 / 出点手柄（拖动时吸附到最近的落刀点，
 * 获得焦点后可以用 ← / → 挪到上一个 / 下一个落刀点；关掉吸附时拖到哪里是哪里，← / → 挪 0.1 秒），
 * 点刻度区域跳到那一刻。
 */
export function DetailBar({
  from,
  to,
  segments,
  gaps,
  points,
  markers,
  selection,
  playhead,
  loading,
  truncated,
  snap,
  onSeek,
  onBlocked,
  onChange,
  onPickMarker,
}: {
  from: number
  to: number
  segments: SegmentView[]
  gaps: Gap[]
  /** 这个范围里的落刀点（关键帧 + 分段末尾） */
  points: number[]
  markers: Marker[]
  selection: Selection
  playhead: number | null
  loading: boolean
  truncated: boolean
  /** 入点、出点吸附到落刀点 */
  snap: boolean
  onSeek: (ms: number) => void
  onBlocked: (segment: SegmentView) => void
  onChange: (selection: Selection) => void
  onPickMarker: (m: Marker) => void
}) {
  const trackRef = useRef<HTMLDivElement>(null)
  const [dragging, setDragging] = useState<'in' | 'out' | null>(null)

  const candidates = (which: 'in' | 'out') =>
    which === 'in'
      ? points.filter((p) => selection.out === null || p < selection.out)
      : points.filter((p) => selection.in === null || p > selection.in)

  /** 不吸附时的取值：不越过另一端，不出细节条 */
  const free = (which: 'in' | 'out', ms: number): number | null => {
    const lo = which === 'out' && selection.in !== null ? selection.in + FREE_STEP_MS : from
    const hi = which === 'in' && selection.out !== null ? selection.out - FREE_STEP_MS : to
    return lo > hi ? null : Math.round(Math.min(Math.max(ms, lo, 0), hi))
  }

  const moveTo = (which: 'in' | 'out', ms: number) => {
    const next = snap ? nearestPoint(candidates(which), ms) : free(which, ms)
    if (next === null || next === selection[which]) return
    onChange({ ...selection, [which]: next })
  }

  const step = (which: 'in' | 'out', dir: -1 | 1) => {
    const current = selection[which]
    if (current === null) return
    const list = candidates(which)
    const next = !snap
      ? free(which, current + dir * FREE_STEP_MS)
      : dir < 0
        ? floorPoint(list, current - 1)
        : ceilPoint(list, current + 1)
    if (next !== null) onChange({ ...selection, [which]: next })
  }

  const handle = (which: 'in' | 'out') => {
    const value = selection[which]
    if (value === null || value < from || value > to) return null
    const label = which === 'in' ? '入点' : '出点'
    return (
      <div
        className={styles.handle}
        data-which={which}
        data-dragging={dragging === which || undefined}
        style={{ left: `${pct(value, from, to)}%` }}
        role="slider"
        tabIndex={0}
        aria-label={snap ? `${label}（按关键帧吸附）` : label}
        aria-valuemin={Math.round(from)}
        aria-valuemax={Math.round(to)}
        aria-valuenow={Math.round(value)}
        aria-valuetext={formatPrecise(value)}
        title={
          snap
            ? `${label} ${formatPrecise(value)}：拖动时吸附到最近的关键帧；选中后按 ← / → 挪一格`
            : `${label} ${formatPrecise(value)}：拖到哪里是哪里；选中后按 ← / → 挪 0.1 秒`
        }
        onPointerDown={(e) => {
          e.stopPropagation()
          e.currentTarget.setPointerCapture(e.pointerId)
          setDragging(which)
        }}
        onPointerMove={(e) => {
          if (dragging !== which || !trackRef.current) return
          moveTo(which, msAt(e, trackRef.current, from, to))
        }}
        onPointerUp={(e) => {
          e.currentTarget.releasePointerCapture(e.pointerId)
          setDragging(null)
        }}
        onPointerCancel={() => setDragging(null)}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return
          e.preventDefault()
          e.stopPropagation()
          step(which, e.key === 'ArrowLeft' ? -1 : 1)
        }}
      >
        <span className={styles.handleLabel}>
          {which === 'in' ? 'I' : 'O'} {formatPrecise(value)}
        </span>
      </div>
    )
  }

  const span = Math.max(1, to - from)
  return (
    <div className={styles.detail} data-loading={loading || undefined}>
      <Ruler from={from} to={to} maxLabels={10} />
      <div
        ref={trackRef}
        className={styles.detailTrack}
        role="presentation"
        onClick={(e) => {
          if (!trackRef.current) return
          const ms = msAt(e, trackRef.current, from, to)
          const seg = segmentAt(segments, ms)
          if (seg && !isReadable(seg)) {
            onBlocked(seg)
            return
          }
          onSeek(ms)
        }}
      >
        <Segments segments={segments} from={from} to={to} />
        <Gaps gaps={gaps} from={from} to={to} />
        <svg className={styles.ticks} viewBox={`0 0 ${span} 1`} preserveAspectRatio="none" aria-hidden="true">
          {points.map((p) => (
            <line key={p} x1={p - from} x2={p - from} y1={0.35} y2={1} vectorEffect="non-scaling-stroke" />
          ))}
        </svg>
        <SelectionBand selection={selection} from={from} to={to} />
        <Pins markers={markers} from={from} to={to} onPick={onPickMarker} />
        {playhead !== null && playhead >= from && playhead <= to ? (
          <div className={styles.playhead} style={{ left: `${pct(playhead, from, to)}%` }} aria-hidden="true" />
        ) : null}
        {handle('in')}
        {handle('out')}
      </div>
      <div className={styles.detailFoot}>
        <span>
          {formatSessionTime(from)} – {formatSessionTime(to)} · 竖线是关键帧（{points.length} 个可落刀位置）
        </span>
        {truncated ? <span className={styles.warn}>关键帧太多，只显示了一部分</span> : null}
      </div>
    </div>
  )
}
