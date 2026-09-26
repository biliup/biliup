'use client'
import React, { useMemo, useState } from 'react'
import dynamic from 'next/dynamic'
import { Button, Checkbox, Popconfirm, Progress, Tag, Tooltip, Typography } from '@douyinfe/semi-ui'
import { diskPercent, formatBytes, memoryPercent, toSeries } from '@/app/lib/use-system-stats'
import { formatRate, timeAgo } from '@/app/lib/use-dashboard'
import { formatVersion } from '@/app/lib/status'
import { humDate } from '@/app/lib/utils'
import { nodeOutdated, type FleetNode, type PoolUsage } from '@/app/lib/use-fleet'
import styles from './page.module.scss'
import NodeConfigStatus from './NodeConfigStatus'

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
  controllerVersion,
  onEditConfig,
}: {
  node: FleetNode
  canManage: boolean
  onRevoke: (node: FleetNode, reassign: boolean) => void
  controllerVersion?: string
  onEditConfig?: (node: FleetNode) => void
}) {
  const [reassign, setReassign] = useState(false)
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
        {nodeOutdated(node) ? (
          <Tooltip content="这台节点的 biliup 版本太旧，收不了控制面分派的房间，请先升级">
            <Tag size="small" color="red">
              版本旧
            </Tag>
          </Tooltip>
        ) : node.synced === false ? (
          <Tooltip content="已下发最新的房间与模板，节点还没确认">
            <Tag size="small" color="blue">
              同步中
            </Tag>
          </Tooltip>
        ) : null}
        {node.allow_hooks ? (
          <Tooltip content="加入时带了 --allow-hooks：处理器里带 run 命令（能执行任意命令）的房间可以派到这台机器；rm、mv 等文件操作不受这个限制">
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
        <Tooltip content="控制面分派给它的房间（含迁移中、还没交给它的）；它自己添加的直播间不算">
          <span className={styles.pool}>
            分派
            <b>{node.assigned_rooms}</b>
          </span>
        </Tooltip>
      </div>

      <div className={styles.nodeAccounts}>
        {node.accounts.length ? (
          <>
            <span className={styles.nodeAccountsLabel}>B 站账号</span>
            {node.accounts.map((account) => (
              <Tooltip key={account.mid} content={`mid ${account.mid}`}>
                <Tag size="small" color="white">
                  {account.uname || account.mid}
                </Tag>
              </Tooltip>
            ))}
          </>
        ) : (
          <Text type="tertiary" size="small">
            未登记 B 站账号
          </Text>
        )}
        {node.tools && !node.tools.ffmpeg.available ? (
          <Tag size="small" color="orange">
            没有 ffmpeg
          </Tag>
        ) : null}
      </div>

      <NodeConfigStatus
        node={node}
        controllerVersion={controllerVersion}
        onOpen={onEditConfig ? () => onEditConfig(node) : undefined}
      />

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
            content={
              node.assigned_rooms > 0 ? (
                <div className={styles.revokeBody}>
                  <span>
                    立即断开，它的身份作废；分派给它的 {node.assigned_rooms}{' '}
                    个房间变为未分派，它本机转为自己管理、继续录这些房间
                  </span>
                  <Checkbox checked={reassign} onChange={(e) => setReassign(Boolean(e.target.checked))}>
                    把这些房间按负载自动改派到其他节点
                  </Checkbox>
                  {reassign ? (
                    <span className={styles.revokeWarn}>
                      {node.name} 如果还在运行，会和新节点同时录这些房间（重复录制与投稿），直到在它本机删掉
                    </span>
                  ) : null}
                </div>
              ) : (
                '立即断开，它的身份作废；要再加入得重新生成票据'
              )
            }
            okType="danger"
            onVisibleChange={(visible) => {
              if (visible) setReassign(false)
            }}
            onConfirm={() => onRevoke(node, reassign && node.assigned_rooms > 0)}
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
