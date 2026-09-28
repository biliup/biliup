'use client'
import { useState } from 'react'
import { Button, Modal, Toast, Typography } from '@douyinfe/semi-ui'
import { errorMessage, type FleetRoom, type FleetTemplate } from '@/app/lib/use-fleet'
import { dissolve, type FleetHa } from '@/app/lib/fleet-ha'
import pageStyles from '../page.module.scss'
import styles from './ha.module.scss'

const { Text } = Typography

/** 解除配对前的确认：用大白话说清哪些行回备机、哪些留在主机、在录在投的那一场怎么办 */
export default function DissolveDialog({
  ha,
  standbyName,
  rooms,
  templates,
  onClose,
  onDone,
}: {
  ha: FleetHa
  standbyName: string
  rooms: FleetRoom[]
  templates: FleetTemplate[]
  onClose: () => void
  onDone: () => void
}) {
  const [busy, setBusy] = useState(false)
  const online = ha.standby?.online === true
  const back = ha.returns ?? { rooms: [], templates: [] }
  const backRooms = back.rooms.map((id) => rooms.find((r) => r.id === id) ?? { id, remark: `房间 #${id}`, url: '' })
  const backTemplates = back.templates.map(
    (id) => templates.find((t) => t.id === id)?.template_name ?? `模板 #${id}`,
  )
  const primary = ha.pair?.primary_node_id
  const excluded = new Set(ha.excluded.map((e) => e.id))
  const staying = rooms.filter(
    (r) => r.node_id === primary && r.deleted_at === null && !excluded.has(r.id) && !back.rooms.includes(r.id),
  )
  const total = back.rooms.length + back.templates.length

  const confirm = async () => {
    if (busy) return
    setBusy(true)
    try {
      await dissolve()
      Toast.success({
        id: 'fleet-ha-dissolve',
        content: total
          ? `已解除配对；${total} 行要交还给 ${standbyName}，进度在「待归还」里`
          : '已解除配对',
        duration: 5,
      })
      onDone()
    } catch (e) {
      Toast.error({ id: 'fleet-ha-dissolve', content: errorMessage(e), duration: 6 })
      setBusy(false)
    }
  }

  return (
    <Modal
      title="解除一主一备"
      visible
      onCancel={busy ? undefined : onClose}
      closeOnEsc={!busy}
      style={{ width: 'min(600px, 94vw)' }}
      footer={
        <div className={pageStyles.dialogFoot}>
          <Button onClick={onClose} disabled={busy}>
            取消
          </Button>
          <Button theme="solid" type="danger" onClick={confirm} loading={busy}>
            解除配对
          </Button>
        </div>
      }
    >
      <div className={`${pageStyles.dialogBody} ${styles.dialogScroll}`}>
        <ul className={styles.bullets}>
          <li>
            当初从 <b>{standbyName}</b> 纳入的直播间与模板<b>还给它</b>：它上面照旧是这些行，「本机」上撤掉。
          </li>
          <li>配对期间在任一台新建的、原本就在「本机」上的，<b>留在「本机」</b>。</li>
          <li>
            正在录或还没投完的那一场，「本机」<b>录完、投完再交还</b>
            ：不会两台各录一段，也不会投两次。
          </li>
          <li>
            {online ? `${standbyName} 此刻在线。` : <span className={styles.warn}>{standbyName} 此刻不在线。</span>}
            它不在线时，这些行先列在「待归还」里，等它回来再交；在那里也可以「放弃交还」留在主机。
          </li>
          <li>之后两台不再同步；各自已有的 B 站账号都留着。</li>
        </ul>
        {ha.returns ? (
          <section className={styles.pickGroup} aria-label="交还的行">
            <div className={styles.pickTitle}>
              还给 {standbyName}
              <Text type="tertiary" size="small">
                {total} 行
              </Text>
            </div>
            {total === 0 ? (
              <div className={styles.empty}>没有从它纳入的行</div>
            ) : (
              <ul className={styles.pickList}>
                {backRooms.map((room) => (
                  <li key={`r-${room.id}`} className={styles.pickRow}>
                    <span className={styles.pickText}>
                      <Text strong ellipsis={{ showTooltip: true }}>
                        直播间「{room.remark || room.url}」
                      </Text>
                      {room.url ? <span className={styles.pickReason}>{room.url}</span> : null}
                    </span>
                  </li>
                ))}
                {backTemplates.map((name, i) => (
                  <li key={`t-${back.templates[i]}`} className={styles.pickRow}>
                    <Text strong ellipsis={{ showTooltip: true }}>
                      投稿模板「{name}」
                    </Text>
                  </li>
                ))}
              </ul>
            )}
          </section>
        ) : null}
        <Text type="tertiary" size="small">
          留在「本机」的直播间 {staying.length} 个{staying.length ? `：${staying.slice(0, 5).map((r) => r.remark || r.url).join('、')}${staying.length > 5 ? ' 等' : ''}` : ''}
        </Text>
      </div>
    </Modal>
  )
}
