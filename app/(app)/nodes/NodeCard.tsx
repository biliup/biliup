'use client'
import React, { useMemo } from 'react'
import dynamic from 'next/dynamic'
import { Button, Popconfirm, Progress, Tag, Tooltip, Typography } from '@douyinfe/semi-ui'
import { diskPercent, formatBytes, memoryPercent, toSeries } from '@/app/lib/use-system-stats'
import { formatRate, timeAgo } from '@/app/lib/use-dashboard'
import { formatVersion } from '@/app/lib/status'
import { humDate } from '@/app/lib/utils'
import type { FleetNode, PoolUsage } from '@/app/lib/use-fleet'
import styles from './page.module.scss'

const SystemChart = dynamic(() => import('@/app/ui/SystemChart'), { ssr: false, loading: () => null })

const CHART_HEIGHT = 30
const WINDOW_MS = 5 * 60 * 1000
const DEFAULT_INTERVAL_MS = 2000
const { Text } = Typography

function levelColor(percent: number): string {
  if (percent >= 90) return 'var(--semi-color-danger)'
  if (percent >= 80) return 'var(--semi-color-warning)'
  return 'var(--semi-color-primary)'
}

function Metric({
  title,
  value,
  foot,
  children,
}: {
  title: string
  value: React.ReactNode
  foot?: React.ReactNode
  children?: React.ReactNode
}) {
  return (
    <div className={styles.metric} role="group" aria-label={title}>
      <div className={styles.metricHead}>
        <span className={styles.metricTitle}>{title}</span>
        <span className={styles.metricValue}>{value}</span>
      </div>
      <div className={styles.metricBody} style={{ height: CHART_HEIGHT }}>
        {children}
      </div>
      {foot ? <div className={styles.metricFoot}>{foot}</div> : null}
    </div>
  )
}

function Pool({ label, usage }: { label: string; usage: PoolUsage | undefined }) {
  if (!usage) return null
  const full = usage.capacity > 0 && usage.occupied >= usage.capacity
  return (
    <span className={styles.pool}>
      {label}
      <b className={full ? styles.poolFull : undefined}>
        {usage.occupied}/{usage.capacity}
      </b>
    </span>
  )
}

const dash = <span className={styles.placeholder}>—</span>

export default function NodeCard({
  node,
  canManage,
  onRevoke,
}: {
  node: FleetNode
  canManage: boolean
  onRevoke: (node: FleetNode) => void
}) {
  const summary = node.summary
  const intervalMs = node.interval_ms || DEFAULT_INTERVAL_MS
  const series = useMemo(() => toSeries(node.samples, intervalMs), [node.samples, intervalMs])
  const cpuYs = useMemo(() => [series.cpu], [series])
  const netYs = useMemo(() => [series.rx, series.tx], [series])
  const last = node.samples.length > 0 ? node.samples[node.samples.length - 1] : null
  const hasCurve = node.online && series.xs.length >= 2
  const memory = summary?.memory ?? null
  const disk = summary?.disk ?? null
  const memPct = memory ? memoryPercent(memory) : 0
  const diskPct = disk ? diskPercent(disk) : 0
  const idle = node.online ? (
    <Text type="tertiary" size="small">
      正在积累采样…
    </Text>
  ) : null

  const seen = node.last_seen_at ? Math.floor(node.last_seen_at / 1000) : null
  const statusLine = node.online
    ? node.connected_at
      ? `已连接 ${timeAgo(Math.floor(node.connected_at / 1000)).replace(/前$/, '')}`
      : '在线'
    : seen
      ? `最后在线 ${timeAgo(seen)}`
      : '从未上线'

  return (
    <article className={`${styles.node} ${node.online ? '' : styles.offline}`} aria-label={`节点 ${node.name}`}>
      <header className={styles.nodeHead}>
        <span className={`${styles.dot} ${node.online ? styles.dotOn : ''}`} aria-hidden="true" />
        <Text strong ellipsis={{ showTooltip: true }} className={styles.nodeName}>
          {node.name}
        </Text>
        <Tag size="small" color={node.online ? 'green' : 'grey'}>
          {node.online ? '在线' : '离线'}
        </Tag>
        {node.online && node.path ? (
          <Tooltip
            content={
              node.path === 'relay'
                ? '经控制面内嵌的 relay 转发（TCP）'
                : '已打洞直连（UDP），不再经过 relay'
            }
          >
            <Tag size="small" color={node.path === 'relay' ? 'orange' : 'cyan'}>
              {node.path === 'relay' ? '中继' : '直连'}
            </Tag>
          </Tooltip>
        ) : null}
        {node.allow_hooks ? (
          <Tooltip content="加入时带了 --allow-hooks：以后下发的配置可以在这台机器上执行命令钩子">
            <Tag size="small" color="red">
              允许钩子
            </Tag>
          </Tooltip>
        ) : null}
        <span className={styles.nodeVersion} title={node.version ?? undefined}>
          {node.version ? `v${formatVersion(node.version)}` : ''}
        </span>
      </header>

      <div className={styles.nodeStats}>
        <span className={styles.pool}>
          录制中
          <b className={summary && summary.recording > 0 ? styles.recording : undefined}>
            {summary ? summary.recording : '—'}
          </b>
          {summary ? <span className={styles.poolOf}>/ {summary.rooms} 个直播间</span> : null}
        </span>
        <Pool label="下载池" usage={summary?.pools.download} />
        <Pool label="上传池" usage={summary?.pools.upload} />
      </div>

      {!node.online && summary ? (
        <div className={styles.stale}>离线，下面是最后一次上报的数据</div>
      ) : null}

      <div className={styles.metrics}>
        <Metric title="CPU" value={node.online && last ? `${last.cpu.toFixed(0)}%` : dash}>
          {hasCurve ? (
            <SystemChart kind="cpu" xs={series.xs} ys={cpuYs} windowMs={WINDOW_MS} height={CHART_HEIGHT} />
          ) : (
            idle
          )}
        </Metric>
        <Metric
          title="内存"
          value={memory ? `${memPct.toFixed(0)}%` : dash}
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
        </Metric>
        <Metric
          title="磁盘"
          value={disk ? `${diskPct.toFixed(0)}%` : dash}
          foot={disk ? `剩余 ${formatBytes(disk.available)}` : null}
        >
          {disk ? (
            <Tooltip content={`录制目录：${disk.path}`}>
              <div className={styles.bar}>
                <Progress
                  percent={diskPct}
                  stroke={levelColor(diskPct)}
                  aria-label={`录制目录磁盘已用 ${diskPct.toFixed(0)}%`}
                />
              </div>
            </Tooltip>
          ) : null}
        </Metric>
        <Metric
          title="网络"
          value={node.online && last ? formatRate(last.rx + last.tx) : dash}
          foot={
            node.online && last ? (
              <span className={styles.net}>
                <span className={styles.rx}>↓ {formatRate(last.rx)}</span>
                <span className={styles.tx}>↑ {formatRate(last.tx)}</span>
              </span>
            ) : null
          }
        >
          {hasCurve ? (
            <SystemChart kind="net" xs={series.xs} ys={netYs} windowMs={WINDOW_MS} height={CHART_HEIGHT} />
          ) : (
            idle
          )}
        </Metric>
      </div>

      <footer className={styles.nodeFoot}>
        <span className={styles.nodeMeta}>
          <span title={seen ? humDate(seen) : undefined}>{statusLine}</span>
          <Tooltip content={`节点 ID：${node.endpoint_id}`}>
            <span className={styles.endpoint}>{node.endpoint_id.slice(0, 10)}</span>
          </Tooltip>
        </span>
        {canManage ? (
          <Popconfirm
            title={`移除节点 ${node.name}？`}
            content="立即断开，它的身份作废；要再加入得重新生成票据"
            okType="danger"
            onConfirm={() => onRevoke(node)}
          >
            <Button size="small" theme="borderless" type="danger">
              移除
            </Button>
          </Popconfirm>
        ) : null}
      </footer>
    </article>
  )
}
