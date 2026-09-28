'use client'
import { useState } from 'react'
import useSWR from 'swr'
import { Button, Modal, Spin, Toast, Typography } from '@douyinfe/semi-ui'
import { fetcher } from '@/app/lib/api-streamer'
import { errorMessage } from '@/app/lib/use-fleet'
import {
  NODE_HA_CANDIDATES_KEY,
  candidatesKey,
  joinOnNode,
  joinPair,
  type Candidates,
  type RowsView,
  type Side,
} from '@/app/lib/fleet-ha'
import { AdoptionResult, RowPicker, defaultPicked, pickedCount, type Picked } from './RowPicker'
import pageStyles from '../page.module.scss'
import styles from './ha.module.scss'

const { Text } = Typography

type Result = Partial<Record<Side, RowsView>>

/**
 * 配对之后把还没纳入的本地行加进配对。控制面上两台的都能加（配对节点那台经控制面请它加入），
 * 配对节点上只加本机的
 */
export default function HaJoinDialog({
  onNode,
  peerName,
  onClose,
  onJoined,
}: {
  onNode: boolean
  /** 配对节点的名字（控制面上用） */
  peerName: string
  onClose: () => void
  onJoined: () => void
}) {
  const key = onNode ? NODE_HA_CANDIDATES_KEY : candidatesKey(null)
  const { data, error, isLoading, mutate } = useSWR<Candidates | RowsView>(key, fetcher, { revalidateOnFocus: false })
  const views: Partial<Record<Side, RowsView | null>> = !data
    ? {}
    : onNode
      ? { node: data as RowsView }
      : { node: (data as Candidates).standby, controller: (data as Candidates).primary }
  const [draft, setDraft] = useState<Partial<Record<Side, Picked>> | null>(null)
  const picks: Record<Side, Picked> = {
    node: draft?.node ?? defaultPicked(views.node),
    controller: draft?.controller ?? defaultPicked(views.controller),
  }
  const [busy, setBusy] = useState(false)
  const [result, setResult] = useState<Result | null>(null)
  const total = (views.node ? pickedCount(picks.node) : 0) + (views.controller ? pickedCount(picks.controller) : 0)

  const submit = async () => {
    if (busy || total === 0) return
    setBusy(true)
    const done: Result = {}
    const failures: string[] = []
    for (const side of ['node', 'controller'] as Side[]) {
      if (!views[side] || pickedCount(picks[side]) === 0) continue
      try {
        const res = onNode ? await joinOnNode(picks[side]) : await joinPair(side, picks[side])
        if (res) done[side] = res
      } catch (e) {
        failures.push(errorMessage(e))
        done[side] = { node: null, error: errorMessage(e) }
      }
    }
    setBusy(false)
    if (failures.length) Toast.error({ id: 'fleet-ha-join', content: failures.join('；'), duration: 6 })
    else Toast.success({ id: 'fleet-ha-join', content: '已请它们加入配对' })
    setResult(done)
    mutate().catch(() => undefined)
    onJoined()
  }

  const names: Record<Side, string> = onNode
    ? { node: '本机', controller: '控制面' }
    : { node: peerName, controller: '「本机」' }

  return (
    <Modal
      title={result ? '加入的结果' : '加入配对'}
      visible
      onCancel={busy ? undefined : onClose}
      closeOnEsc={!busy}
      style={{ width: 'min(680px, 94vw)' }}
      footer={
        <div className={pageStyles.dialogFoot}>
          {result ? (
            <Button theme="solid" onClick={onClose}>
              完成
            </Button>
          ) : (
            <>
              <Button onClick={onClose} disabled={busy}>
                取消
              </Button>
              <Button theme="solid" onClick={submit} loading={busy} disabled={total === 0}>
                加入（{total} 项）
              </Button>
            </>
          )}
        </div>
      }
    >
      <div className={`${pageStyles.dialogBody} ${styles.dialogScroll}`}>
        {result ? (
          <>
            <Text type="tertiary" size="small">
              在录或上一场还没投完的，等这一场录完投完再加入；两台都落地后才算加入
            </Text>
            {(['node', 'controller'] as Side[]).map((side) =>
              result[side] ? <AdoptionResult key={side} title={names[side]} view={result[side]} /> : null,
            )}
          </>
        ) : !data && isLoading ? (
          <div className={pageStyles.dialogCenter}>
            <Spin />
          </div>
        ) : error ? (
          <div className={styles.pickError}>读不到清单：{errorMessage(error)}</div>
        ) : (
          <>
            <Text type="tertiary" size="small">
              下面是{onNode ? '本机' : '两台'}上还没纳入配对的本地直播间与投稿模板。加入之后两台都录、只由一台投稿，在哪台改都会同步
            </Text>
            {(['node', 'controller'] as Side[]).map((side) =>
              views[side] === undefined ? null : (
                <RowPicker
                  key={side}
                  title={names[side]}
                  view={views[side]}
                  picked={picks[side]}
                  disabled={busy}
                  onChange={(p) => setDraft({ ...picks, [side]: p })}
                />
              ),
            )}
          </>
        )}
      </div>
    </Modal>
  )
}
