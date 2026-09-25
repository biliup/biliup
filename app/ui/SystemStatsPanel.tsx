'use client'
import React, { useMemo } from 'react'
import dynamic from 'next/dynamic'
import { Progress, Tag, Tooltip, Typography } from '@douyinfe/semi-ui'
import {
  diskPercent,
  formatBytes,
  memoryPercent,
  toSeries,
  useSystemStats,
  type SystemSeries,
} from '@/app/lib/use-system-stats'
import { formatRate } from '@/app/lib/use-dashboard'
import styles from './system-stats.module.scss'

/**
 * 控制台首页的系统状态：CPU、内存、录制目录所在磁盘、网速四张小卡片。
 * 数据与口径见 `use-system-stats.ts` 和后端 `server/common/system_stats.rs`。
 */

const SystemChart = dynamic(() => import('./SystemChart'), { ssr: false, loading: () => null })

const CHART_HEIGHT = 36
const DEFAULT_HISTORY_MS = 5 * 60 * 1000
const DEFAULT_INTERVAL_MS = 2000
const { Text } = Typography

/** 占用比例 → 进度条颜色：≥ 90% 红，≥ 80% 橙 */
function levelColor(percent: number): string {
  if (percent >= 90) return 'var(--semi-color-danger)'
  if (percent >= 80) return 'var(--semi-color-warning)'
  return 'var(--semi-color-primary)'
}

function Tile({
  title,
  meta,
  value,
  children,
  foot,
  label,
}: {
  title: string
  meta?: React.ReactNode
  value: React.ReactNode
  children?: React.ReactNode
  foot?: React.ReactNode
  label: string
}) {
  return (
    <div className={styles.tile} role="group" aria-label={label}>
      <div className={styles.tileHead}>
        <span className={styles.tileTitle}>{title}</span>
        {meta ? <span className={styles.tileMeta}>{meta}</span> : null}
      </div>
      <div className={styles.tileValue}>{value}</div>
      <div className={styles.tileBody} style={{ height: CHART_HEIGHT }}>
        {children}
      </div>
      {foot ? <div className={styles.tileFoot}>{foot}</div> : null}
    </div>
  )
}

function Placeholder() {
  return <span className={styles.placeholder}>—</span>
}

export default function SystemStatsPanel() {
  const snap = useSystemStats()
  const latest = snap.latest
  const intervalMs = latest?.interval_ms || DEFAULT_INTERVAL_MS
  const windowMs = latest?.history_ms || DEFAULT_HISTORY_MS
  const series: SystemSeries = useMemo(() => toSeries(snap.samples, intervalMs), [snap.samples, intervalMs])
  const cpuYs = useMemo(() => [series.cpu], [series])
  const netYs = useMemo(() => [series.rx, series.tx], [series])
  const last = snap.samples.length > 0 ? snap.samples[snap.samples.length - 1] : null
  const hasCurve = series.xs.length >= 2

  const cpu = latest?.cpu ?? null
  const memory = latest?.memory ?? null
  const disk = latest?.disk ?? null
  const memPct = memory ? memoryPercent(memory) : 0
  const diskPct = disk ? diskPercent(disk) : 0

  const notice = !latest
    ? snap.error
      ? `系统状态加载失败：${snap.error}`
      : null
    : snap.error
      ? `连接中断，下面是最后一次拿到的数据（${snap.error}）`
      : snap.stalled
        ? '服务端采样已停止，数值可能已过期，请查看服务日志'
        : null

  const minutes = Math.round(windowMs / 60000)

  return (
    <section className={styles.panel} aria-label="系统状态">
      <div className={styles.head}>
        <span className={styles.label}>系统状态</span>
        <span className={styles.note}>
          最近 {minutes} 分钟 · 每 {Math.round(intervalMs / 1000)} 秒采样
        </span>
      </div>
      {notice ? (
        <div className={styles.notice} role="status">
          {notice}
        </div>
      ) : null}
      <div className={styles.grid}>
        <Tile
          title="CPU"
          label="CPU 占用"
          meta={
            cpu ? (
              <Tooltip content="整机所有逻辑核的平均占用；在容器里读到的是宿主机的数">
                <span>
                  {cpu.logical} 线程{cpu.physical ? ` / ${cpu.physical} 核` : ''}
                </span>
              </Tooltip>
            ) : null
          }
          value={last ? `${last.cpu.toFixed(1)}%` : <Placeholder />}
        >
          {hasCurve ? (
            <SystemChart kind="cpu" xs={series.xs} ys={cpuYs} windowMs={windowMs} height={CHART_HEIGHT} />
          ) : (
            <Text type="tertiary" size="small">
              {latest ? '正在积累采样…' : '加载中…'}
            </Text>
          )}
        </Tile>

        <Tile
          title="内存"
          label="内存占用"
          meta={
            memory?.limited ? (
              <Tooltip content="服务运行在设了内存上限的容器里，这里报的是容器上限与匿名内存（不含可回收的页缓存）">
                <Tag size="small" color="blue">
                  容器上限
                </Tag>
              </Tooltip>
            ) : null
          }
          value={memory ? `${memPct.toFixed(0)}%` : <Placeholder />}
          foot={memory ? `${formatBytes(memory.used)} / ${formatBytes(memory.total)}` : null}
        >
          {memory ? (
            <Progress
              percent={memPct}
              stroke={levelColor(memPct)}
              aria-label={`内存已用 ${memPct.toFixed(0)}%`}
              className={styles.bar}
            />
          ) : null}
        </Tile>

        <Tile
          title="磁盘"
          label="录制目录所在磁盘"
          meta={
            disk ? (
              <Tooltip content={`录制目录：${disk.path}`}>
                <span className={styles.path}>{disk.path}</span>
              </Tooltip>
            ) : null
          }
          value={disk ? `${diskPct.toFixed(0)}%` : <Placeholder />}
          foot={
            disk
              ? `剩余 ${formatBytes(disk.available)} / 共 ${formatBytes(disk.total)}`
              : latest
                ? '读取录制目录容量失败或超时'
                : null
          }
        >
          {disk ? (
            <Progress
              percent={diskPct}
              stroke={levelColor(diskPct)}
              aria-label={`录制目录磁盘已用 ${diskPct.toFixed(0)}%`}
              className={styles.bar}
            />
          ) : null}
        </Tile>

        <Tile
          title="网络"
          label="网络速率"
          meta={
            latest && latest.interfaces.length > 0 ? (
              <Tooltip content={`参与统计的网卡：${latest.interfaces.join('、')}（只统计物理网卡，避免 docker0、veth 等重复计数）`}>
                <span className={styles.path}>{latest.interfaces.join(', ')}</span>
              </Tooltip>
            ) : null
          }
          value={
            last ? (
              <span className={styles.net}>
                <span className={styles.rx}>↓ {formatRate(last.rx)}</span>
                <span className={styles.tx}>↑ {formatRate(last.tx)}</span>
              </span>
            ) : (
              <Placeholder />
            )
          }
        >
          {hasCurve ? (
            <SystemChart kind="net" xs={series.xs} ys={netYs} windowMs={windowMs} height={CHART_HEIGHT} />
          ) : (
            <Text type="tertiary" size="small">
              {latest ? '正在积累采样…' : '加载中…'}
            </Text>
          )}
        </Tile>
      </div>
    </section>
  )
}
