'use client'
import Link from 'next/link'
import React, { useRef, useState } from 'react'
import { Banner, Button, Popconfirm, Space, Toast } from '@douyinfe/semi-ui'
import { mutate } from 'swr'
import { ME_KEY, proxy } from '@/app/lib/api-streamer'
import { useMe } from '@/app/lib/use-me'

export const REVOKED_TITLE = '该节点已被移出机群，原受管主播已暂停，确认后手动恢复'
export const REVOKED_HINT = '被移出机群时暂停，确认不会与接手的节点重复录制后再恢复'

/**
 * 被控制面移除过的机器：原受管主播转为本机自管并暂停（`/v1/me` 的 `fleet_revoked`），
 * 这里提示并给「全部恢复」「知道了」两个入口。直播管理页与控制台共用。
 * 请求只在点击时发出，不放在 effect 里，StrictMode 下也只发一次。
 */
export default function FleetRevokedBanner({
  className,
  linkToStreamers = false,
}: {
  className?: string
  /** 控制台上多给一个去「直播管理」逐个处理的入口 */
  linkToStreamers?: boolean
}) {
  const { me, can } = useMe()
  const busy = useRef(false)
  const [pending, setPending] = useState<'resume' | 'dismiss' | null>(null)
  const revoked = me?.fleet_revoked
  if (!revoked || revoked.streamers.length === 0) return null
  const count = revoked.streamers.length
  const canControl = can('recording.control')

  const act = async (kind: 'resume' | 'dismiss') => {
    if (busy.current) return
    busy.current = true
    setPending(kind)
    try {
      if (kind === 'resume') {
        const res = await proxy('/v1/node/revoked/resume', { method: 'POST' })
        const { resumed } = (await res.json()) as { resumed: number[] }
        Toast.success(`已恢复 ${resumed.length} 个直播间`)
      } else {
        await proxy('/v1/node/revoked', { method: 'DELETE' })
        Toast.info('已关闭提示，这些直播间保持暂停')
      }
      await Promise.all([mutate(ME_KEY), mutate('/v1/streamers')])
    } catch (e: any) {
      Toast.error(`${kind === 'resume' ? '恢复' : '操作'}失败：${e?.message ?? String(e)}`)
    } finally {
      busy.current = false
      setPending(null)
    }
  }

  return (
    <Banner
      type="warning"
      fullMode={false}
      closeIcon={null}
      className={className}
      title={REVOKED_TITLE}
      description={
        <>
          <div>
            {`${count} 个直播间原由控制面 ${revoked.controller} 管理，${new Date(revoked.revoked_at).toLocaleString()} 本机被移出机群时已暂停，避免和接手它们的节点重复录制、重复投稿。`}
            {canControl
              ? '确认控制面不会再让别的节点录它们后，点「全部恢复」，或在「直播管理」里标着「待恢复」的直播间上逐个恢复。'
              : '需要有「启停录制」权限的用户确认后恢复。'}
          </div>
          {canControl || linkToStreamers ? (
            <Space wrap style={{ marginTop: 10 }}>
              {canControl ? (
                <>
                  <Popconfirm
                    title={`恢复录制这 ${count} 个直播间？`}
                    content="如果控制面已把它们分派给别的节点，恢复后两边会重复录制、重复投稿。"
                    onConfirm={() => act('resume')}
                  >
                    <Button theme="solid" type="warning" size="small" loading={pending === 'resume'} disabled={pending !== null}>
                      全部恢复
                    </Button>
                  </Popconfirm>
                  <Popconfirm
                    title="只关闭这条提示？"
                    content="直播间保持暂停，之后在「直播管理」里逐个恢复。暂停不再跨重启保留：本机重启后它们会照常录制。"
                    onConfirm={() => act('dismiss')}
                  >
                    <Button size="small" loading={pending === 'dismiss'} disabled={pending !== null}>
                      知道了
                    </Button>
                  </Popconfirm>
                </>
              ) : null}
              {linkToStreamers ? (
                <Link href="/streamers" prefetch={false}>
                  去直播管理 →
                </Link>
              ) : null}
            </Space>
          ) : null}
        </>
      }
    />
  )
}
