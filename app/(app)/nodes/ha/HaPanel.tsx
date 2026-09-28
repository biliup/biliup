'use client'
import { useMemo, useRef, useState, type ReactNode } from 'react'
import { mutate as revalidate } from 'swr'
import { Button, Popconfirm, Tag, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { humDate } from '@/app/lib/utils'
import { streamerStatusTag, uploadStatusTag } from '@/app/lib/status'
import type { LiveStreamerEntity } from '@/app/lib/api-streamer'
import { errorMessage } from '@/app/lib/use-fleet'
import { FLEET_SUMMARY_KEY, clearAlert, type FleetAlert } from '@/app/lib/fleet-alerts'
import {
  HA_ALERT_KINDS,
  MODE_TEXT,
  manual,
  needsHuman,
  primaryStateText,
  standbyStateText,
  switchLeader,
  type HaAlertKind,
  type HaMode,
  type HaParams,
  type ManualAction,
  type PairSync,
  type Side,
} from '@/app/lib/fleet-ha'
import { AlertItem } from '../AlertsPanel'
import alertStyles from '../alerts.module.scss'
import pageStyles from '../page.module.scss'
import styles from './ha.module.scss'

const { Text } = Typography

/** 面板要的配对情况，两边各自从 `GET /v1/fleet/ha` / `GET /v1/node/ha` 整理出来 */
export interface PairModel {
  /** 面板开在配对节点上 */
  onNode: boolean
  mode: HaMode
  params: HaParams
  leader: Side
  names: Record<Side, string>
  /** 对端在线（本机总是在线） */
  peerOnline: boolean
  /** 两台都在线、连接接上、对端版本够新：换上传主机要这个；不行时是原因 */
  bothIssue: string | null
  /** 改模式与参数此刻为什么不行（两台都在线才改） */
  modeIssue: string | null
  /** 做到一半的场次（换上传主机会被拒） */
  busy: string | null
  sync: PairSync | undefined
  /** 本机是备机时主机是否判离线（只有备机知道） */
  primaryOffline?: boolean
}

export interface ManualItem {
  key: string
  title: string
  state: string
  reason: string | null
  started_at: number
  ended_at: number | null
}

export interface RoomLine {
  key: string
  name: string
  url: string
  local: LiveStreamerEntity | undefined
  /** 最近一场的情况 */
  latest: string | null
  /** 这一场（或以后）谁投 */
  uploader: Side
}

export function peerOf(side: Side): Side {
  return side === 'controller' ? 'node' : 'controller'
}

function self(model: PairModel): Side {
  return model.onNode ? 'node' : 'controller'
}

function ms(at: number | null | undefined) {
  return at ? humDate(Math.floor(at / 1000)) : '—'
}

/** 配对状态：两台的角色、在线、模式与操作按钮 */
export function PairCard({
  model,
  canManage,
  onChanged,
  onMode,
  onJoin,
  extra,
  note,
}: {
  model: PairModel
  canManage: boolean
  onChanged: () => void
  onMode: () => void
  onJoin: () => void
  /** 按钮行末尾的操作（控制面上的「解除配对」） */
  extra?: ReactNode
  /** 卡片底部的一句提示 */
  note?: string
}) {
  const [switching, setSwitching] = useState(false)
  const me = self(model)
  const target = peerOf(model.leader)
  const switchTo = async () => {
    if (switching) return
    setSwitching(true)
    try {
      await switchLeader(target, model.onNode)
      Toast.success({ id: 'fleet-ha-role', content: `已换成由${model.names[target]}上传` })
      onChanged()
    } catch (e) {
      Toast.error({ id: 'fleet-ha-role', content: errorMessage(e), duration: 6 })
    } finally {
      setSwitching(false)
    }
  }
  const machine = (side: Side) => {
    const online = side === me || model.peerOnline
    const leader = side === model.leader
    return (
      <div key={side} className={styles.machine}>
        <div className={styles.machineHead}>
          <span className={`${pageStyles.dot} ${online ? pageStyles.dotOn : ''}`} aria-hidden="true" />
          <span className={styles.machineName} title={model.names[side]}>
            {model.names[side]}
          </span>
          <Tag size="small" color={leader ? 'blue' : 'grey'}>
            {leader ? '上传主机' : '备机'}
          </Tag>
        </div>
        <Text type="tertiary" size="small">
          {online ? '在线' : '离线'}
          {side === me ? ' · 你正在看的这台' : ''}
          {side === 'controller' ? ' · 控制面' : ''}
        </Text>
      </div>
    )
  }
  const switchButton = (
    <Button
      size="small"
      onClick={switchTo}
      loading={switching}
      disabled={!canManage || model.bothIssue !== null}
    >
      换成{model.names[target]}上传
    </Button>
  )
  const modeButton = (
    <Button size="small" onClick={onMode} disabled={!canManage || model.modeIssue !== null}>
      改模式与参数
    </Button>
  )
  return (
    <section className={styles.card} aria-label="配对">
      <div className={styles.cardHead}>
        <span className={styles.cardTitle}>一主一备</span>
        <Tag size="small" color="light-blue">
          {MODE_TEXT[model.mode].title}
        </Tag>
        {canManage ? (
          <div className={styles.headActions}>
            {model.bothIssue ? <Tooltip content={model.bothIssue}>{switchButton}</Tooltip> : switchButton}
            {model.modeIssue ? <Tooltip content={model.modeIssue}>{modeButton}</Tooltip> : modeButton}
            <Button size="small" onClick={onJoin}>
              加入配对
            </Button>
            {extra}
          </div>
        ) : null}
      </div>
      <div className={styles.machines}>{(['controller', 'node'] as Side[]).map(machine)}</div>
      <p className={styles.explain}>
        {MODE_TEXT[model.mode].hint}。此刻由<b>{model.names[model.leader]}</b>投稿。分主备只为不出两份稿件：在哪台上改直播间、模板、配置、B
        站账号都行，两台会同步。
      </p>
      {model.bothIssue && canManage ? (
        <Text type="tertiary" size="small">
          {model.modeIssue ? '换上传主机、改模式' : '换上传主机'}暂时不能用：{model.bothIssue}
        </Text>
      ) : null}
      {model.busy && !model.bothIssue && canManage ? (
        <Text type="tertiary" size="small">
          {model.busy}：换上传主机要等它了结
        </Text>
      ) : null}
      {model.primaryOffline ? (
        <div className={pageStyles.notice} role="status">
          主机判为离线：{model.mode === 1 ? '这台照常录，主机一直不回来时由这台投' : '这台接手录，主机回来先投它那半，这台再追加'}
        </div>
      ) : null}
      {note ? (
        <Text type="tertiary" size="small">
          {note}
        </Text>
      ) : null}
    </section>
  )
}

/** 同步：连接、排着没送到的修改 */
export function SyncCard({ model }: { model: PairModel }) {
  const sync = model.sync
  if (!sync || sync.linked === undefined) {
    return (
      <section className={styles.card} aria-label="同步">
        <div className={styles.cardHead}>
          <span className={styles.cardTitle}>两台之间的同步</span>
          <Tag size="small" color="grey">
            不同步
          </Tag>
        </div>
        <p className={styles.explain}>
          对端的 Fleet 协议版本低于 {sync?.min_proto ?? 5}，不能双向同步：配对里的直播间与模板只能在控制面上改。两台都升级到同一版本后自动开始同步
        </p>
      </section>
    )
  }
  return (
    <section className={styles.card} aria-label="同步">
      <div className={styles.cardHead}>
        <span className={styles.cardTitle}>两台之间的同步</span>
        <Tag size="small" color={sync.linked ? 'green' : 'orange'}>
          {sync.linked ? '连着' : '断开'}
        </Tag>
      </div>
      <div className={styles.facts}>
        <span>
          还没送到对端的修改 <b>{sync.pending ?? 0}</b> 条
        </span>
      </div>
      <p className={styles.explain}>
        {sync.linked
          ? '直播间、投稿模板、空间配置与 B 站账号在两台之间同步。'
          : '两台断开了：各自照常录、照常改，改动先记下，连上后补发。'}
        断开期间两台改了同一项的，连上后以后改的为准。
      </p>
    </section>
  )
}

/** 待人工处理：备机那份等人决定投还是放弃；两台上都能点 */
export function ManualList({
  items,
  onNode,
  canSubmit,
  onChanged,
}: {
  items: ManualItem[]
  onNode: boolean
  canSubmit: boolean
  onChanged: () => void
}) {
  const [acting, setActing] = useState<string | null>(null)
  const act = async (item: ManualItem, action: ManualAction) => {
    if (acting) return
    setActing(`${item.key}:${action}`)
    try {
      await manual(item.key, action, onNode)
      Toast.success({
        id: 'fleet-ha-manual',
        content: action === 'drop' ? `已放弃「${item.title}」这一场` : `已让备机投「${item.title}」这一场`,
      })
      onChanged()
    } catch (e) {
      Toast.error({ id: 'fleet-ha-manual', content: errorMessage(e), duration: 6 })
    } finally {
      setActing(null)
    }
  }
  return (
    <section className={styles.card} aria-label="待人工处理">
      <div className={styles.cardHead}>
        <span className={styles.cardTitle}>待人工处理</span>
        {items.length ? (
          <Tag size="small" color="orange">
            {items.length}
          </Tag>
        ) : null}
        <span className={styles.cardHint}>主机那份投不成或等不到主机时，备机这份等你决定</span>
      </div>
      {items.length === 0 ? (
        <div className={styles.empty}>没有要处理的场次</div>
      ) : (
        <ul className={styles.list}>
          {items.map((item) => (
            <li key={item.key} className={styles.row}>
              <div className={styles.rowMain}>
                <span className={styles.rowName}>{item.title}</span>
                <span className={styles.rowSub}>
                  {standbyStateText(item.state)}
                  {item.reason ? `：${item.reason}` : ''} · {ms(item.started_at)} 开播
                  {item.ended_at ? `，${ms(item.ended_at)} 下播` : ''}
                </span>
              </div>
              {canSubmit ? (
                <div className={styles.rowActions}>
                  <Popconfirm
                    title="让备机投这一场？"
                    content="备机用它自己录的那份投稿"
                    onConfirm={() => act(item, 'standby-upload')}
                  >
                    <Button size="small" theme="solid" loading={acting === `${item.key}:standby-upload`}>
                      备机投
                    </Button>
                  </Popconfirm>
                  <Popconfirm
                    title="放弃这一场？"
                    content="两台都不再投这一场；录像文件按各自的清理规则处理"
                    onConfirm={() => act(item, 'drop')}
                  >
                    <Button size="small" type="danger" loading={acting === `${item.key}:drop`}>
                      放弃
                    </Button>
                  </Popconfirm>
                </div>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

/** 每个直播间：谁录、谁投、最近一场怎样 */
export function RoomsCard({
  model,
  lines,
  unlanded = 0,
}: {
  model: PairModel
  lines: RoomLine[]
  /** 配对里还没落到这台上的直播间个数（节点上用） */
  unlanded?: number
}) {
  const me = self(model)
  const recording =
    model.mode === 1
      ? '两台都录'
      : `${model.names[model.leader]}录，${model.names[peerOf(model.leader)]}在它离线时接手`
  return (
    <section className={styles.card} aria-label="配对里的直播间">
      <div className={styles.cardHead}>
        <span className={styles.cardTitle}>配对里的直播间</span>
        <span className={styles.cardHint}>
          {lines.length} 个 · {recording} · 平时由{model.names[model.leader]}投
        </span>
      </div>
      {unlanded > 0 ? (
        <Text type="tertiary" size="small">
          另有 {unlanded} 个配对里的直播间还没落到这台上（常见原因：这台有同一地址的本地主播，先删掉那一行）
        </Text>
      ) : null}
      {lines.length === 0 ? (
        unlanded > 0 ? null : (
          <div className={styles.empty}>还没有直播间在配对里：在任一台添加直播间，或用「加入配对」把已有的加进来</div>
        )
      ) : (
        <div className={pageStyles.tableWrap} style={{ boxShadow: 'none' }}>
          <table className={pageStyles.roomTable}>
            <thead>
              <tr>
                <th>直播间</th>
                <th>{model.names[me]}此刻</th>
                <th>最近一场</th>
                <th>谁投</th>
              </tr>
            </thead>
            <tbody>
              {lines.map((line) => (
                <tr key={line.key}>
                  <td className={pageStyles.roomMain}>
                    <div className={pageStyles.roomName} title={line.name}>
                      {line.name || line.url}
                    </div>
                    <Text type="tertiary" size="small" className={pageStyles.roomSub} ellipsis={{ showTooltip: true }}>
                      {line.url}
                    </Text>
                  </td>
                  <td style={{ whiteSpace: 'nowrap' }}>
                    {line.local ? (
                      <>
                        {streamerStatusTag(line.local.status)} {uploadStatusTag(line.local.upload_status)}
                      </>
                    ) : (
                      <Text type="tertiary" size="small">
                        还没落地
                      </Text>
                    )}
                  </td>
                  <td>
                    <div className={styles.latest}>
                      <Text size="small">{line.latest ?? '—'}</Text>
                    </div>
                  </td>
                  <td style={{ whiteSpace: 'nowrap' }}>{model.names[line.uploader]}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  )
}

/** 主机那一侧记的一场 → 一句话 */
export function recordText(primaryState: string, standbyState: string | null): string {
  return `${primaryStateText(primaryState)}；${standbyStateText(standbyState)}`
}

/** 一主一备的告警（沿用 F4 告警，只列按内容认出的四类） */
export function HaAlerts({
  alerts,
  kindOf,
  now,
  canManage,
  onChanged,
}: {
  alerts: FleetAlert[]
  kindOf: (alert: FleetAlert) => HaAlertKind | null
  now: number
  canManage: boolean
  onChanged: () => void
}) {
  const [clearing, setClearing] = useState<number | null>(null)
  const busy = useRef(false)
  const shown = useMemo(
    () => alerts.map((alert) => ({ alert, kind: kindOf(alert) })).filter((a) => a.kind !== null),
    [alerts, kindOf],
  )
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
      revalidate(FLEET_SUMMARY_KEY).catch(() => undefined)
      onChanged()
    }
  }
  return (
    <section className={styles.card} aria-label="一主一备的告警">
      <div className={styles.cardHead}>
        <span className={styles.cardTitle}>告警</span>
        {shown.length ? (
          <Tag size="small" color={shown.some((a) => a.alert.resolved_at === null) ? 'red' : 'grey'}>
            {shown.length}
          </Tag>
        ) : null}
        <span className={styles.cardHint}>待人工处理、备机投稿失败、备机离线、缺 B 站账号；全部告警在「告警」页签</span>
      </div>
      {shown.length === 0 ? (
        <div className={styles.empty}>没有一主一备的告警</div>
      ) : (
        <ul className={alertStyles.list} aria-label="一主一备的告警列表">
          {shown.map(({ alert, kind }) => (
            <AlertItem
              key={alert.id}
              alert={alert}
              now={now}
              canManage={canManage}
              clearing={clearing === alert.id}
              onAcknowledge={() => acknowledge(alert)}
              label={HA_ALERT_KINDS[kind as HaAlertKind]}
            />
          ))}
        </ul>
      )}
    </section>
  )
}

/** 场次里有没有要人工处理的（备机上报的状态） */
export function manualFromRecords<T extends { standby_state: string | null }>(records: T[]): T[] {
  return records.filter((r) => needsHuman(r.standby_state))
}
