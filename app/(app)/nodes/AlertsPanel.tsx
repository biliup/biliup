'use client'
import React, { useMemo, useRef, useState } from 'react'
import { mutate as revalidate } from 'swr'
import { Button, Empty, Popconfirm, Select, Spin, Tag, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconAlertTriangle, IconTickCircle } from '@douyinfe/semi-icons'
import { humDate } from '@/app/lib/utils'
import { errorMessage, type FleetNode } from '@/app/lib/use-fleet'
import {
  ALERT_KINDS,
  FLEET_SUMMARY_KEY,
  clearAlert,
  clearAlerts,
  useFleetAlerts,
  type FleetAlert,
} from '@/app/lib/fleet-alerts'
import pageStyles from './page.module.scss'
import styles from './alerts.module.scss'

const { Text } = Typography

const STATUS_OPEN = 'open'
const STATUS_ALL = 'all'
const NODE_ALL = 'all'

/** 按控制面的时钟算「多久以前」，避免浏览器与控制面时钟不一致 */
function ago(at: number, now: number): string {
  const diff = Math.floor((now - at) / 1000)
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  return `${Math.floor(diff / 86400)} 天前`
}

export default function AlertsPanel({
  nodes,
  canManage,
  nodeFilter,
  onNodeFilter,
}: {
  nodes: FleetNode[]
  canManage: boolean
  /** 地址栏 `?node=` 带过来的节点 id；null 为全部 */
  nodeFilter: number | null
  onNodeFilter: (id: number | null) => void
}) {
  const { data, error, isLoading, mutate } = useFleetAlerts(true)
  const [status, setStatus] = useState<string>(STATUS_ALL)
  const [clearing, setClearing] = useState<number | 'all' | null>(null)
  const busy = useRef(false)

  const alerts = useMemo(() => data?.alerts ?? [], [data])
  const now = data?.now ?? 0
  const eventsSince = data?.events_since ?? 3
  const legacy = nodes.filter((n) => n.online && (n.proto ?? 0) < eventsSince)
  const filtered = useMemo(
    () =>
      alerts.filter(
        (a) =>
          (status === STATUS_ALL || a.resolved_at === null) && (nodeFilter === null || a.node_id === nodeFilter),
      ),
    [alerts, status, nodeFilter],
  )
  const openCount = alerts.filter((a) => a.resolved_at === null).length
  const byNode = useMemo(() => {
    const counts = new Map<number, number>()
    for (const a of alerts) counts.set(a.node_id, (counts.get(a.node_id) ?? 0) + 1)
    return counts
  }, [alerts])
  const narrowed = status !== STATUS_ALL || nodeFilter !== null

  const refresh = () => {
    mutate().catch(() => undefined)
    revalidate(FLEET_SUMMARY_KEY).catch(() => undefined)
  }

  const acknowledge = async (alert: FleetAlert) => {
    if (busy.current) return
    busy.current = true
    setClearing(alert.id)
    try {
      await clearAlert(alert.id)
      Toast.success('已知道')
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
    } finally {
      busy.current = false
      setClearing(null)
      refresh()
    }
  }

  const acknowledgeShown = async () => {
    if (busy.current) return
    busy.current = true
    setClearing('all')
    try {
      if (narrowed) {
        const results = await Promise.allSettled(filtered.map((a) => clearAlert(a.id)))
        const failed = results.filter((r) => r.status === 'rejected').length
        if (failed) Toast.warning(`清除了 ${results.length - failed} 条，${failed} 条已不存在`)
        else Toast.success(`已清除 ${results.length} 条告警`)
      } else {
        const cleared = await clearAlerts()
        Toast.success(`已清除 ${cleared} 条告警`)
      }
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
    } finally {
      busy.current = false
      setClearing(null)
      refresh()
    }
  }

  let body: React.ReactNode
  if (!data && isLoading) {
    body = (
      <div className={pageStyles.center}>
        <Spin size="large" />
      </div>
    )
  } else if (!data) {
    body = (
      <div className={pageStyles.center}>
        <Empty title="加载失败" description={errorMessage(error) || '无法获取告警'} />
        <Button onClick={refresh} style={{ marginTop: 12 }}>
          重试
        </Button>
      </div>
    )
  } else if (alerts.length === 0) {
    body = (
      <div className={pageStyles.center}>
        <Empty
          image={<IconTickCircle size="extra-large" style={{ color: 'var(--semi-color-success)' }} />}
          title="没有告警"
          description="节点离线、磁盘不足、配置应用失败、房间落地失败、录制出错与投稿失败都会显示在这里"
        />
      </div>
    )
  } else if (filtered.length === 0) {
    body = (
      <div className={pageStyles.center}>
        <Empty title="没有匹配的告警" description="调整筛选条件试试" />
      </div>
    )
  } else {
    body = (
      <ul className={styles.list} aria-label="告警列表">
        {filtered.map((alert) => (
          <AlertItem
            key={alert.id}
            alert={alert}
            now={now}
            canManage={canManage}
            clearing={clearing === alert.id || clearing === 'all'}
            onAcknowledge={() => acknowledge(alert)}
          />
        ))}
      </ul>
    )
  }

  const nodeOptions = [
    { value: NODE_ALL, label: `全部节点（${alerts.length}）` },
    ...nodes.map((n) => ({ value: String(n.id), label: `${n.name}（${byNode.get(n.id) ?? 0}）` })),
  ]

  return (
    <>
      <div className={styles.memo} role="note">
        告警只保存在控制面内存里（最多 {data?.capacity ?? 200} 条），控制面重启后清空，也不会发外部通知。
        「知道了」只是把它从列表里拿掉；状态类告警要先恢复、再出现才会重新提醒。
      </div>
      {legacy.length ? (
        <div className={pageStyles.notice} role="status">
          {legacy.map((n) => n.name).join('、')} 的 Fleet 协议版本低于 {eventsSince}
          ，不会上报录制出错与投稿失败；离线、磁盘、配置与落地失败照常告警。升级 biliup 后就有
        </div>
      ) : null}
      {data && error ? (
        <div className={pageStyles.notice} role="status">
          连接中断，下面是最后一次拿到的数据（{errorMessage(error)}）
        </div>
      ) : null}
      {alerts.length > 0 ? (
        <div className={styles.toolbar}>
          <Select
            value={status}
            onChange={(v) => setStatus(String(v))}
            optionList={[
              { value: STATUS_ALL, label: `全部（${alerts.length}）` },
              { value: STATUS_OPEN, label: `未恢复（${openCount}）` },
            ]}
            className={styles.status}
            aria-label="按状态筛选"
          />
          <Select
            value={nodeFilter === null ? NODE_ALL : String(nodeFilter)}
            onChange={(v) => onNodeFilter(v === NODE_ALL ? null : Number(v))}
            optionList={nodeOptions}
            className={styles.node}
            aria-label="按节点筛选"
          />
          {canManage && filtered.length > 0 ? (
            <Popconfirm
              title={narrowed ? `把筛出的 ${filtered.length} 条告警都标为知道了？` : '把全部告警都标为知道了？'}
              content="清掉后不会再提醒，除非情况恢复后再次出现"
              onConfirm={acknowledgeShown}
            >
              <Button className={styles.clearAll} loading={clearing === 'all'}>
                {narrowed ? `这 ${filtered.length} 条知道了` : '全部知道了'}
              </Button>
            </Popconfirm>
          ) : null}
        </div>
      ) : null}
      {body}
    </>
  )
}

/** 节点卡片名字旁的告警标记：有还没恢复的为红色，只剩已恢复未清除的为灰色；点开是这台节点的告警 */
export function NodeAlertBadge({ alerts, onOpen }: { alerts: FleetAlert[]; onOpen: () => void }) {
  if (alerts.length === 0) return null
  const open = alerts.filter((a) => a.resolved_at === null)
  const kinds = [...new Set(open.map((a) => ALERT_KINDS[a.kind].label))]
  const hint = open.length
    ? `${open.length} 条未恢复：${kinds.join('、')}。点击查看`
    : `${alerts.length} 条已恢复、还没清除的告警。点击查看`
  return (
    <Tooltip content={hint}>
      <button type="button" className={styles.badge} onClick={onOpen} aria-label={`告警 ${alerts.length} 条，查看`}>
        <Tag size="small" color={open.length ? 'red' : 'grey'} prefixIcon={<IconAlertTriangle />}>
          {open.length || alerts.length}
        </Tag>
      </button>
    </Tooltip>
  )
}

/** `label`：换掉告警种类的标签与说明（一主一备面板按内容认出的四类） */
export function AlertItem({
  alert,
  now,
  canManage,
  clearing,
  onAcknowledge,
  label,
}: {
  alert: FleetAlert
  now: number
  canManage: boolean
  clearing: boolean
  onAcknowledge: () => void
  label?: (typeof ALERT_KINDS)[keyof typeof ALERT_KINDS]
}) {
  const kind = label ?? ALERT_KINDS[alert.kind]
  const resolved = alert.resolved_at !== null
  const subject = alert.room || alert.url
  return (
    <li className={`${styles.item} ${resolved ? styles.resolved : ''}`} aria-label={`${kind.label}：${alert.node_name}`}>
      <div className={styles.head}>
        {resolved ? (
          <IconTickCircle className={styles.iconOk} aria-hidden="true" />
        ) : (
          <IconAlertTriangle className={styles.iconBad} aria-hidden="true" />
        )}
        <Tooltip content={kind.hint}>
          <Tag size="small" color={kind.color}>
            {kind.label}
          </Tag>
        </Tooltip>
        <Text strong className={styles.nodeName} ellipsis={{ showTooltip: true }}>
          {alert.node_name}
        </Text>
        {subject ? (
          <Text className={styles.subject} ellipsis={{ showTooltip: true }} type="secondary">
            {subject}
          </Text>
        ) : null}
        {resolved ? (
          <Tag size="small" color="green">
            {ago(alert.resolved_at as number, now)}恢复
          </Tag>
        ) : null}
        {alert.count > 1 ? (
          <Tooltip content={`出现了 ${alert.count} 次，已合并为一条`}>
            <Tag size="small" color="white">
              ×{alert.count}
            </Tag>
          </Tooltip>
        ) : null}
      </div>
      <div className={styles.message}>{alert.message}</div>
      <div className={styles.foot}>
        <Text type="tertiary" size="small" className={styles.time}>
          <span title={humDate(Math.floor(alert.last_at / 1000))}>{ago(alert.last_at, now)}</span>
          {alert.count > 1 || alert.first_at !== alert.last_at ? (
            <span> · 首次 {humDate(Math.floor(alert.first_at / 1000))}</span>
          ) : null}
          {alert.room && alert.url ? <span className={styles.url}> · {alert.url}</span> : null}
        </Text>
        {canManage ? (
          <Button size="small" theme="borderless" onClick={onAcknowledge} loading={clearing} className={styles.ack}>
            知道了
          </Button>
        ) : null}
      </div>
    </li>
  )
}
