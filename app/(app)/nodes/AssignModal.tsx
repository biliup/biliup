'use client'
import { useMemo, useState } from 'react'
import { Banner, Checkbox, Modal, Select, Tag, Toast, Typography } from '@douyinfe/semi-ui'
import {
  assignRoom,
  errorMessage,
  placementIssue,
  type FleetNode,
  type FleetRoom,
  type FleetTemplate,
} from '@/app/lib/use-fleet'
import styles from './page.module.scss'

const { Text } = Typography

const UNASSIGN = -1

/**
 * 分派 / 迁移一个房间。正常迁移是「上一台先停录、确认释放后才交给新节点」；
 * 上一台离线时会一直等，这里提供「强制迁移」并写明会重复录制。
 */
export default function AssignModal({
  room,
  nodes,
  template,
  onClose,
  onDone,
}: {
  room: FleetRoom
  nodes: FleetNode[]
  template: FleetTemplate | undefined
  onClose: () => void
  onDone: () => void
}) {
  const [target, setTarget] = useState<number | undefined>(room.node_id ?? undefined)
  const [force, setForce] = useState(false)
  const [saving, setSaving] = useState(false)
  const byId = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes])

  const holderId = room.releasing_node_id ?? room.node_id
  const holder = holderId !== null ? byId.get(holderId) : undefined
  const targetId = target === UNASSIGN ? null : (target ?? null)
  const moving = target !== undefined && holderId !== null && holderId !== targetId
  const unchanged = target === undefined || targetId === room.node_id

  const options = [
    ...[...nodes]
      .sort((a, b) => Number(b.online) - Number(a.online) || a.id - b.id)
      .map((node) => {
        const issue = placementIssue(node, room, template)
        return {
          value: node.id,
          disabled: issue !== null,
          label: (
            <span className={styles.nodeOption}>
              <span className={`${styles.dot} ${node.online ? styles.dotOn : ''}`} aria-hidden="true" />
              <span className={styles.nodeOptionName}>{node.name}</span>
              <Text type={issue ? 'danger' : 'tertiary'} size="small">
                {issue ?? `${node.online ? '在线' : '离线'} · ${node.assigned_rooms} 个房间`}
              </Text>
            </span>
          ),
        }
      }),
    { value: UNASSIGN, label: <Text type="tertiary">不分派（让当前节点停录）</Text> },
  ]

  const submit = async () => {
    if (unchanged) {
      onClose()
      return
    }
    setSaving(true)
    try {
      const next = await assignRoom(room.id, targetId, moving && force)
      const to = targetId !== null ? byId.get(targetId)?.name : null
      if (next?.releasing_node_id) {
        Toast.info(`等 ${byId.get(next.releasing_node_id)?.name ?? '上一台'} 停录后交给 ${to ?? '—'}`)
      } else {
        Toast.success(to ? `已分派到 ${to}` : '已取消分派')
      }
      onDone()
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
    } finally {
      setSaving(false)
    }
  }

  return (
    <Modal
      title={`${room.node_id !== null ? '迁移' : '分派'}「${room.remark}」`}
      visible
      onCancel={onClose}
      onOk={submit}
      okText={moving && force ? '强制迁移' : '确定'}
      okButtonProps={{ type: moving && force ? 'danger' : 'primary', disabled: unchanged }}
      confirmLoading={saving}
      width="min(520px, 94vw)"
    >
      <div className={styles.dialogBody}>
        <div className={styles.assignNow}>
          <Text type="tertiary" size="small">
            现在
          </Text>
          {holder ? (
            <Tag size="small" color={holder.online ? 'green' : 'grey'}>
              {holder.name}
              {holder.online ? '' : '（离线）'}
            </Tag>
          ) : (
            <Tag size="small">未分派</Tag>
          )}
          {room.releasing_node_id !== null && room.node_id !== null ? (
            <Text type="tertiary" size="small">
              → {byId.get(room.node_id)?.name ?? `节点 ${room.node_id}`}（迁移中）
            </Text>
          ) : null}
        </div>
        <Select
          value={target}
          onChange={(v) => setTarget(v as number)}
          optionList={options}
          placeholder="选择节点"
          style={{ width: '100%' }}
          aria-label="目标节点"
        />
        {moving ? (
          <>
            <Text type="tertiary" size="small">
              {holder?.online
                ? `${holder.name} 会先停止录制这个房间，确认释放后才交给新节点，两边不会同时录。`
                : `${holder?.name ?? '当前节点'} 离线：迁移会一直等它上线并确认释放。`}
            </Text>
            <Checkbox checked={force} onChange={(e) => setForce(Boolean(e.target.checked))}>
              强制迁移，不等 {holder?.name ?? '当前节点'} 确认释放
            </Checkbox>
            {force ? (
              <Banner
                type="danger"
                fullMode={false}
                closeIcon={null}
                description={`${holder?.name ?? '当前节点'} 如果其实还在录（例如只是和控制面断开了），它会和新节点同时录这个房间，产生重复的录播与投稿，直到它重新连上控制面才停。`}
              />
            ) : null}
          </>
        ) : null}
      </div>
    </Modal>
  )
}
