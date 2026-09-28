'use client'
import type { ReactNode } from 'react'
import { Tag, Typography } from '@douyinfe/semi-ui'
import type { FleetNode, FleetRoom, FleetTemplate } from '@/app/lib/use-fleet'
import { handbackBlocker, type HandbackEntry, type Returning } from '@/app/lib/fleet-ha'
import styles from './ha.module.scss'

const { Text } = Typography

export type HandbackRow = {
  node: number
  kind: 'rooms' | 'templates'
  id: number
  entry: HandbackEntry
  online: boolean | undefined
}

function stageTag(entry: HandbackEntry, online: boolean | undefined) {
  if (entry.stage === 'released') return <Tag color="green">交接中</Tag>
  if (online === false) return <Tag color="grey">等备机回来</Tag>
  if (entry.local === 'recording' || entry.local === 'uploading') return <Tag color="orange">等本机空闲</Tag>
  return <Tag color="blue">交还中</Tag>
}

/** 解除配对之后还没交还完的行：每行写卡在哪（备机离线等它回来 / 本机在录或在投等空闲） */
export default function HandbackList({
  handback,
  nodes,
  rooms,
  templates,
  actions,
}: {
  handback: Record<string, Returning>
  nodes: FleetNode[]
  rooms: FleetRoom[]
  templates: FleetTemplate[]
  /** 每行右侧的操作；`label` 是这一行的称呼，`nodeName` 是交还给的节点 */
  actions?: (row: HandbackRow, label: string, nodeName: string) => ReactNode
}) {
  return (
    <>
      {Object.entries(handback).map(([nodeKey, returning]) => {
        const node = Number(nodeKey)
        const name = nodes.find((n) => n.id === node)?.name ?? `节点 #${node}`
        const rows: HandbackRow[] = [
          ...Object.entries(returning.rooms ?? {}).map(([id, entry]) => ({
            node,
            kind: 'rooms' as const,
            id: Number(id),
            entry,
            online: returning.online,
          })),
          ...Object.entries(returning.templates ?? {}).map(([id, entry]) => ({
            node,
            kind: 'templates' as const,
            id: Number(id),
            entry,
            online: returning.online,
          })),
        ]
        return (
          <section key={nodeKey} className={styles.card} aria-label={`待归还给 ${name}`}>
            <div className={styles.cardHead}>
              <span className={styles.cardTitle}>待归还给 {name}</span>
              {returning.online === undefined ? null : (
                <Tag size="small" color={returning.online ? 'green' : 'grey'}>
                  {returning.online ? '在线' : '离线'}
                </Tag>
              )}
              <span className={styles.cardHint}>{rows.length} 行</span>
            </div>
            <p className={styles.explain}>
              配对已经解除。这些是当初从 {name} 纳入的行：{returning.online === false ? `${name} 离线，等它回来再交；` : ''}
              「本机」在录或在投的，等录完投完再交，不会两台各录一段。
            </p>
            <ul className={styles.list}>
              {rows.map((row) => {
                const room = row.kind === 'rooms' ? rooms.find((r) => r.id === row.id) : undefined
                const template = row.kind === 'templates' ? templates.find((t) => t.id === row.id) : undefined
                const label =
                  row.kind === 'rooms'
                    ? `直播间「${room?.remark || row.entry.url || `#${row.id}`}」`
                    : `投稿模板「${template?.template_name ?? `#${row.id}`}」`
                return (
                  <li key={`${row.kind}-${row.id}`} className={styles.row}>
                    <div className={styles.rowMain}>
                      <span className={styles.rowName}>{label}</span>
                      <span className={styles.rowSub}>
                        {row.kind === 'rooms' ? handbackBlocker(row.entry, row.online) : templateBlocker(row.entry, row.online)}
                        {row.entry.url ? <Text type="tertiary" size="small"> · {row.entry.url}</Text> : null}
                      </span>
                    </div>
                    {stageTag(row.entry, row.online)}
                    {actions ? <div className={styles.rowActions}>{actions(row, label, name)}</div> : null}
                  </li>
                )
              })}
            </ul>
          </section>
        )
      })}
    </>
  )
}

function templateBlocker(entry: HandbackEntry, online: boolean | undefined): string {
  if (entry.stage === 'released') return '已交给备机，等它确认'
  if (online === false) return '备机离线：等它回来再交还'
  return '用它的直播间都交完之后自动交还'
}
