'use client'
import { useState } from 'react'
import useSWR from 'swr'
import { Button, Empty, Spin } from '@douyinfe/semi-ui'
import { IconServer } from '@douyinfe/semi-icons'
import PageHeader from '../../components/PageHeader'
import dc from '@/app/ui/data-card.module.scss'
import { fetcher, type LiveStreamerEntity } from '@/app/lib/api-streamer'
import { useMe } from '@/app/lib/use-me'
import { FLEET_REFRESH_MS, errorMessage } from '@/app/lib/use-fleet'
import { NODE_HA_KEY, needsHuman, standbyStateText, type NodeHa as NodeHaView, type Side } from '@/app/lib/fleet-ha'
import { ManualList, PairCard, RoomsCard, SyncCard, peerOf, type ManualItem, type PairModel, type RoomLine } from './HaPanel'
import ModeDialog from './ModeDialog'
import HaJoinDialog from './HaJoinDialog'
import pageStyles from '../page.module.scss'
import styles from './ha.module.scss'

/** 配对里的节点上的「一主一备」页：与控制面上的面板同样的内容，解除配对只在控制面上做 */
export default function NodeHa() {
  const { me, can } = useMe()
  const { data, error, isLoading, mutate } = useSWR<NodeHaView>(NODE_HA_KEY, fetcher, {
    refreshInterval: FLEET_REFRESH_MS,
  })
  const pairIds = me?.fleet_node?.pair?.streamers
  const { data: streamers } = useSWR<LiveStreamerEntity[]>(data ? '/v1/streamers' : null, fetcher, {
    refreshInterval: FLEET_REFRESH_MS,
  })
  const [dialog, setDialog] = useState<'mode' | 'join' | null>(null)
  const canManage = can('node.manage')
  const refresh = () => {
    mutate().catch(() => undefined)
  }

  let body
  if (!data && isLoading) {
    body = (
      <div className={pageStyles.center}>
        <Spin size="large" />
      </div>
    )
  } else if (!data) {
    body = (
      <div className={pageStyles.center}>
        <Empty title="加载失败" description={errorMessage(error) || '读不到配对情况'} />
        <Button onClick={refresh} style={{ marginTop: 12 }}>
          重试
        </Button>
      </div>
    )
  } else {
    const standby = data.role === 'standby' ? data : null
    const leader: Side = standby ? standby.leader : 'node'
    const controllerName = me?.fleet_node?.controller
    const names: Record<Side, string> = {
      node: '本机',
      controller: controllerName ? `控制面（${controllerName}）` : '控制面',
    }
    const sync = data.sync
    const issue = !sync
      ? '与控制面的双向同步没有接上（控制面的版本较旧）：只能在控制面上改'
      : !sync.linked
        ? '控制面那台不在线：只剩一台在线时不能换，等两台都在线'
        : null
    const model: PairModel = {
      onNode: true,
      mode: data.mode,
      params: data.params,
      leader,
      names,
      peerOnline: sync?.linked ?? data.linked,
      bothIssue: issue,
      modeIssue: issue,
      busy: null,
      sync,
      primaryOffline: standby?.primary_offline,
    }
    const manualItems: ManualItem[] = standby
      ? standby.sessions
          .filter((s) => needsHuman(s.state))
          .map((s) => ({
            key: s.key || s.id,
            title: s.remark || s.url,
            state: s.state,
            reason: s.reason,
            started_at: s.started_at,
            ended_at: s.ended_at,
          }))
      : data.role === 'primary'
        ? data.sessions
            .filter((s) => needsHuman(s.standby_state))
            .map((s) => ({
              key: s.session_key,
              title: `直播间 #${s.room_id}`,
              state: s.standby_state ?? '',
              reason: s.reason,
              started_at: s.started_at,
              ended_at: s.ended_at,
            }))
        : []
    const lines: RoomLine[] = (streamers ?? [])
      .filter((s) => pairIds?.includes(s.id))
      .map((s) => {
        const session = standby?.sessions
          .filter((x) => x.url === s.url)
          .sort((a, b) => b.started_at - a.started_at)[0]
        return {
          key: String(s.id),
          name: s.remark,
          url: s.url,
          local: s,
          latest: session ? standbyStateText(session.state) : null,
          uploader: session && ['uploading', 'appending', 'uploaded', 'appended'].includes(session.state) ? peerOf(leader) : leader,
        }
      })
    body = (
      <div className={styles.panel}>
        <PairCard
          model={model}
          canManage={canManage}
          onChanged={refresh}
          onMode={() => setDialog('mode')}
          onJoin={() => setDialog('join')}
          note={
            canManage
              ? '解除配对在控制面的「节点 › 一主一备」里操作'
              : '换上传主机、改模式、加入配对要「管理节点」权限；解除配对在控制面上操作'
          }
        />
        <ManualList items={manualItems} onNode canSubmit={can('upload.submit')} onChanged={refresh} />
        <RoomsCard model={model} lines={lines} unlanded={standby ? Math.max(0, standby.rooms.length - lines.length) : 0} />
        <SyncCard model={model} />
        {dialog === 'mode' ? (
          <ModeDialog
            mode={data.mode}
            params={data.params}
            standby={0}
            onNode
            onClose={() => setDialog(null)}
            onSaved={() => {
              setDialog(null)
              refresh()
            }}
          />
        ) : null}
        {dialog === 'join' ? (
          <HaJoinDialog onNode peerName={names.controller} onClose={() => setDialog(null)} onJoined={refresh} />
        ) : null}
      </div>
    )
  }

  return (
    <>
      <PageHeader
        icon={<IconServer size="large" />}
        title="一主一备"
        description="这台与控制面那台配成一主一备：两台录同样的直播间，只由一台投稿；在哪台上改都会同步"
      />
      <div className={dc.content}>
        {data && error ? (
          <div className={pageStyles.notice} role="status">
            连接中断，下面是最后一次拿到的数据（{errorMessage(error)}）
          </div>
        ) : null}
        {body}
      </div>
    </>
  )
}
