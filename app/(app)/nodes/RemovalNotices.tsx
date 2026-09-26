'use client'
import { useState } from 'react'
import { Banner } from '@douyinfe/semi-ui'
import { humDate } from '@/app/lib/utils'
import type { FleetNode, FleetRoom, Removal } from '@/app/lib/use-fleet'
import styles from './page.module.scss'

const key = (removal: Removal) => `${removal.node_id}:${removal.started_at}`

function list(names: string[]): string {
  return names.length > 3 ? `${names.slice(0, 3).join('、')} 等 ${names.length} 个` : names.join('、')
}

/** 「移除并自动改派」的进度与结果：进行中的一直显示，结束的可以关掉（后端留 10 分钟） */
export default function RemovalNotices({
  removals,
  rooms,
  nodes,
}: {
  removals: Removal[]
  rooms: FleetRoom[]
  nodes: FleetNode[]
}) {
  const [dismissed, setDismissed] = useState<string[]>([])
  const visible = removals.filter((removal) => !dismissed.includes(key(removal)))
  if (visible.length === 0) return null
  const nodeName = (id: number) => nodes.find((node) => node.id === id)?.name ?? `节点 ${id}`

  return (
    <div className={styles.removals}>
      {visible.map((removal) => {
        if (removal.state === 'removing') {
          const released = removal.rooms.filter(
            (room) =>
              room.release === 'released' ||
              rooms.find((current) => current.id === room.room_id)?.releasing_node_id !== removal.node_id,
          ).length
          return (
            <Banner
              key={key(removal)}
              type="info"
              closeIcon={null}
              data-removal={removal.node_id}
              title={`正在移除 ${removal.node_name}`}
              description={`${released} / ${removal.rooms.length} 个房间已确认释放并交给新节点；全部释放后移除它，最晚 ${humDate(Math.floor(removal.deadline / 1000))}`}
            />
          )
        }
        const handed = removal.rooms.filter((room) => room.release === 'released' && room.node_id !== null)
        const unconfirmed = (release: string) => release === 'offline' || release === 'timeout'
        const forced = removal.rooms.filter((room) => room.node_id !== null && unconfirmed(room.release))
        const unplaced = removal.rooms.filter((room) => room.node_id === null)
        const targets = [...new Set(handed.map((room) => nodeName(room.node_id as number)))]
        return (
          <Banner
            key={key(removal)}
            type={forced.length || unplaced.length ? 'warning' : 'success'}
            data-removal={removal.node_id}
            title={`已移除 ${removal.node_name}`}
            onClose={() => setDismissed((current) => [...current, key(removal)])}
            description={
              <ul className={styles.removalList}>
                {removal.rooms.length === 0 ? <li>它没有分派中的房间</li> : null}
                {handed.length ? (
                  <li>
                    {handed.length} 个房间已确认释放，之后才交给 {list(targets)}，没有重复录制
                  </li>
                ) : null}
                {forced.length ? (
                  <li>
                    {list(forced.map((room) => room.remark))}：
                    {forced.some((room) => room.release === 'timeout') ? '节点超时未释放' : '节点离线'}
                    ，没等到确认就改派了；原节点将在发现被移除后暂停它们
                  </li>
                ) : null}
                {unplaced.length ? (
                  <li>
                    {list(unplaced.map((room) => room.remark))} 没找到合适的节点，留在未分派：{unplaced[0].unplaced}
                    {unplaced.some((room) => unconfirmed(room.release)) ? '；原节点将在发现被移除后暂停它们' : ''}
                  </li>
                ) : null}
              </ul>
            }
          />
        )
      })}
    </div>
  )
}
