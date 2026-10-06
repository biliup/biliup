'use client'
import { useState } from 'react'
import { Button, Popconfirm, Toast, Tooltip } from '@douyinfe/semi-ui'
import { errorMessage } from '@/app/lib/use-fleet'
import { forceIssue, handbackAction, type HandbackAction } from '@/app/lib/fleet-ha'
import type { HandbackRow } from './HandbackList'

/** 待归还一行上的两个手动动作：放弃交还（留在主机）、立即交还（只有房间，本机只在投时） */
export default function HandbackActions({
  row,
  label,
  nodeName,
  onChanged,
}: {
  row: HandbackRow
  label: string
  nodeName: string
  onChanged: () => void
}) {
  const [acting, setActing] = useState<HandbackAction | null>(null)
  if (row.entry.stage === 'released') return null
  const act = async (action: HandbackAction) => {
    if (acting) return
    setActing(action)
    try {
      await handbackAction(row.kind, row.id, action)
      Toast.success({
        id: 'fleet-ha-handback',
        content: action === 'abandon' ? `已放弃交还：${label}留在「本机」` : `已立即交还：${label}交给 ${nodeName}`,
      })
      onChanged()
    } catch (e) {
      Toast.error({ id: 'fleet-ha-handback', content: errorMessage(e), duration: 6 })
    } finally {
      setActing(null)
    }
  }
  const issue = row.kind === 'rooms' ? forceIssue(row.entry, row.online !== false) : null
  const force = (
    <Button size="small" disabled={issue !== null} loading={acting === 'force'}>
      立即交还
    </Button>
  )
  return (
    <>
      <Popconfirm
        title="放弃交还，留在主机？"
        content={
          row.kind === 'rooms'
            ? `以后由「本机」录；${nodeName} 上原来那一行在它连上后撤掉，不会两台都录`
            : `这个模板留在「本机」；${nodeName} 上原来那一份在它连上后撤掉`
        }
        onConfirm={() => act('abandon')}
      >
        <Button size="small" loading={acting === 'abandon'}>
          放弃交还，留在主机
        </Button>
      </Popconfirm>
      {row.kind === 'rooms' ? (
        issue ? (
          <Tooltip content={issue}>{force}</Tooltip>
        ) : (
          <Popconfirm
            title="立即交还？"
            content={`「本机」正在投的这一场接着投完；之后开播由 ${nodeName} 录`}
            onConfirm={() => act('force')}
          >
            {force}
          </Popconfirm>
        )
      ) : null}
    </>
  )
}
