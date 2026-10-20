'use client'

import { Suspense } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import useSWR, { mutate } from 'swr'
import { Button, Empty, Notification, Spin } from '@douyinfe/semi-ui'
import { fetcher, put, sendRequest, type LiveStreamerEntity, type StudioEntity } from '@/app/lib/api-streamer'
import { formListToHookStep } from '@/app/lib/postprocessor'
import { useMe } from '@/app/lib/use-me'
import StreamerEditor from '@/app/ui/StreamerEditor'
import { FormPage } from '@/app/ui/shell'

const BACK = { href: '/streamers', label: '直播管理' }

function EditorPage() {
  const router = useRouter()
  const params = useSearchParams()
  const rawId = params.get('id')
  const id = Number(rawId)
  const { me, can, error: meError } = useMe()
  const allowed = can('streamer.edit')
  const rooms = useSWR<LiveStreamerEntity[]>(allowed && rawId ? '/v1/streamers' : null, fetcher)
  const templates = useSWR<StudioEntity[]>(allowed ? '/v1/upload/streamers' : null, fetcher)
  const entity = rooms.data?.find(room => room.id === id)
  const title = rawId ? `编辑直播间${entity ? `「${entity.remark}」` : ''}` : '新建直播间'
  const error = meError ?? rooms.error ?? templates.error
  let state
  if (error) state = <Empty title="加载失败" description={error.message}><Button onClick={() => { mutate('/v1/me'); rooms.mutate(); templates.mutate() }}>重试</Button></Empty>
  else if (!me) state = <Spin />
  else if (!allowed) state = <Empty title="没有编辑直播间的权限" />
  else if (rawId !== null && (!Number.isInteger(id) || id <= 0)) state = <Empty title="无效的直播间编号" />
  else if (!templates.data || (rawId && !rooms.data)) state = <Spin />
  else if (rawId && !entity) state = <Empty title="直播间不存在或已被删除" />
  else if (entity && me.fleet_node?.streamers.includes(entity.id)) state = <Empty title="这个直播间由控制面托管" description="请前往控制面的任务平台 › 节点管理 › 房间编辑。" />
  if (state) return <FormPage title={title} back={BACK}>{state}</FormPage>

  const save = async (values: Record<string, any>) => {
    const payload: Record<string, any> = { ...entity, ...values }
    delete payload.status
    delete payload.statusTag
    delete payload.upload_status
    if (can('streamer.hooks') && payload.postprocessor) payload.postprocessor = formListToHookStep(payload.postprocessor)
    try {
      if (entity) await put('/v1/streamers', { arg: payload })
      else await sendRequest('/v1/streamers', { arg: payload })
      await mutate('/v1/streamers')
      router.push(BACK.href)
    } catch (error) {
      Notification.error({ title: entity ? '更新失败' : '创建失败', content: error instanceof Error ? error.message : String(error) })
      throw error
    }
  }
  return <StreamerEditor key={entity?.id ?? 'new'} entity={entity} title={title} back={BACK} onOk={save}
    templateOptions={templates.data!.map(t => ({ value: t.id, label: t.template_name }))} />
}

export default function Page() {
  return <Suspense><EditorPage /></Suspense>
}
