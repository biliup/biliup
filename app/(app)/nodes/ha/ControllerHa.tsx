'use client'
import { useCallback, useState } from 'react'
import useSWR from 'swr'
import { Button, Typography } from '@douyinfe/semi-ui'
import { fetcher, type LiveStreamerEntity } from '@/app/lib/api-streamer'
import { FLEET_REFRESH_MS, type FleetNode, type FleetRoom, type FleetTemplate } from '@/app/lib/use-fleet'
import type { FleetAlert } from '@/app/lib/fleet-alerts'
import { PAIR_SINCE, haAlertKind, needsHuman, standbyStateText, type FleetHa, type Side } from '@/app/lib/fleet-ha'
import {
  HaAlerts,
  ManualList,
  PairCard,
  RoomsCard,
  SyncCard,
  peerOf,
  recordText,
  type ManualItem,
  type PairModel,
  type RoomLine,
} from './HaPanel'
import HandbackList from './HandbackList'
import HandbackActions from './HandbackActions'
import ModeDialog from './ModeDialog'
import HaJoinDialog from './HaJoinDialog'
import DissolveDialog from './DissolveDialog'
import styles from './ha.module.scss'

const { Text } = Typography

/** 控制面节点页的「一主一备」页签 */
export default function ControllerHa({
  ha,
  nodes,
  rooms,
  templates,
  alerts,
  alertsNow,
  canManage,
  canSubmit,
  onChanged,
}: {
  ha: FleetHa
  nodes: FleetNode[]
  rooms: FleetRoom[]
  templates: FleetTemplate[]
  alerts: FleetAlert[]
  alertsNow: number
  canManage: boolean
  canSubmit: boolean
  onChanged: () => void
}) {
  const pair = ha.pair
  const { data: streamers } = useSWR<LiveStreamerEntity[]>(pair ? '/v1/streamers' : null, fetcher, {
    refreshInterval: FLEET_REFRESH_MS,
  })
  const [dialog, setDialog] = useState<'mode' | 'join' | 'dissolve' | null>(null)
  const kindOf = useCallback((alert: FleetAlert) => (pair ? haAlertKind(alert, pair) : null), [pair])

  const handback = ha.handback && Object.keys(ha.handback).length ? ha.handback : null
  const handbackList = handback ? (
    <HandbackList
      handback={handback}
      nodes={nodes}
      rooms={rooms}
      templates={templates}
      actions={
        canManage
          ? (row, label, nodeName) => (
              <HandbackActions row={row} label={label} nodeName={nodeName} onChanged={onChanged} />
            )
          : undefined
      }
    />
  ) : null

  if (!pair) {
    return (
      <div className={styles.panel}>
        {handbackList}
        {!handback ? <div className={styles.empty}>没有配对</div> : null}
      </div>
    )
  }

  const standbyName = nodes.find((n) => n.id === pair.standby_node_id)?.name ?? `节点 #${pair.standby_node_id}`
  const names: Record<Side, string> = { controller: '「本机」', node: standbyName }
  const leader: Side = ha.leader ?? 'controller'
  const online = ha.standby?.online === true
  const proto = ha.standby?.proto ?? 0
  const offline = `${standbyName} 不在线：只剩一台在线时不能换，等两台都在线`
  const model: PairModel = {
    onNode: false,
    mode: pair.mode,
    params: pair.params,
    leader,
    names,
    peerOnline: online,
    bothIssue: !online
      ? offline
      : proto < PAIR_SINCE
        ? `${standbyName} 的 Fleet 协议版本是 ${proto}，低于 ${PAIR_SINCE}，不认识换上传主机：先升级它`
        : ha.sync.linked === false
          ? '两台之间的连接还没接上（节点刚连上时要等一会儿），稍后再试'
          : null,
    modeIssue: online ? null : offline,
    busy: ha.busy,
    sync: ha.sync,
  }

  const roomName = (id: number) => {
    const room = rooms.find((r) => r.id === id)
    return room ? room.remark || room.url : `直播间 #${id}`
  }
  const manualItems: ManualItem[] =
    ha.local_role === 'standby'
      ? (ha.local_standby?.sessions ?? [])
          .filter((s) => needsHuman(s.state))
          .map((s) => ({
            key: s.key || s.id,
            title: s.remark || s.url,
            state: s.state,
            reason: s.reason,
            started_at: s.started_at,
            ended_at: s.ended_at,
          }))
      : ha.sessions
          .filter((s) => needsHuman(s.standby_state))
          .map((s) => ({
            key: s.session_key,
            title: roomName(s.room_id),
            state: s.standby_state ?? '',
            reason: s.reason,
            started_at: s.started_at,
            ended_at: s.ended_at,
          }))

  const excluded = new Set(ha.excluded.map((e) => e.id))
  const byUrl = new Map((streamers ?? []).map((s) => [s.url, s]))
  const lines: RoomLine[] = rooms
    .filter((r) => r.node_id === pair.primary_node_id && r.deleted_at === null && !excluded.has(r.id))
    .map((room) => {
      let latest: string | null = null
      let uploader = leader
      if (ha.local_role === 'standby') {
        const session = (ha.local_standby?.sessions ?? [])
          .filter((s) => s.room === room.id)
          .sort((a, b) => b.started_at - a.started_at)[0]
        if (session) latest = standbyStateText(session.state)
      } else {
        const session = ha.sessions.filter((s) => s.room_id === room.id).sort((a, b) => b.started_at - a.started_at)[0]
        if (session) {
          latest = recordText(session.primary_state, session.standby_state)
          if (session.uploader === 'standby') uploader = peerOf(leader)
        }
      }
      return { key: String(room.id), name: room.remark, url: room.url, local: byUrl.get(room.url), latest, uploader }
    })

  return (
    <div className={styles.panel}>
      <PairCard
        model={model}
        canManage={canManage}
        onChanged={onChanged}
        onMode={() => setDialog('mode')}
        onJoin={() => setDialog('join')}
        extra={
          <Button size="small" type="danger" onClick={() => setDialog('dissolve')}>
            解除配对
          </Button>
        }
        note={canManage ? undefined : '换上传主机、改模式、加入配对、解除配对要「管理节点」权限'}
      />
      <ManualList items={manualItems} onNode={false} canSubmit={canSubmit} onChanged={onChanged} />
      {handbackList}
      <RoomsCard model={model} lines={lines} />
      {ha.excluded.length ? (
        <Text type="tertiary" size="small">
          不纳入配对、只由「本机」录的直播间：{ha.excluded.map((e) => e.remark || e.url).join('、')}（
          {ha.excluded[0].reason}）
        </Text>
      ) : null}
      <SyncCard model={model} />
      <HaAlerts alerts={alerts} kindOf={kindOf} now={alertsNow} canManage={canManage} onChanged={onChanged} />
      {dialog === 'mode' ? (
        <ModeDialog
          mode={pair.mode}
          params={pair.params}
          standby={pair.standby_node_id}
          onNode={false}
          onClose={() => setDialog(null)}
          onSaved={() => {
            setDialog(null)
            onChanged()
          }}
        />
      ) : null}
      {dialog === 'join' ? (
        <HaJoinDialog onNode={false} peerName={standbyName} onClose={() => setDialog(null)} onJoined={onChanged} />
      ) : null}
      {dialog === 'dissolve' ? (
        <DissolveDialog
          ha={ha}
          standbyName={standbyName}
          rooms={rooms}
          templates={templates}
          onClose={() => setDialog(null)}
          onDone={() => {
            setDialog(null)
            onChanged()
          }}
        />
      ) : null}
    </div>
  )
}
