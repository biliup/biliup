'use client'

import { Suspense } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import useSWR, { mutate } from 'swr'
import { Button, Empty, Spin, Toast } from '@douyinfe/semi-ui'
import { fetcher } from '@/app/lib/api-streamer'
import { roomFromForm, roomToForm } from '@/app/lib/streamer-form'
import { FLEET_NODES_KEY, FLEET_ROOMS_KEY, FLEET_TEMPLATES_KEY, createRoom, updateRoom, errorMessage, type FleetNodes, type FleetRooms, type FleetTemplate } from '@/app/lib/use-fleet'
import StreamerEditor from '@/app/ui/StreamerEditor'
import { FormPage } from '@/app/ui/shell'
import FleetPageGate from '../FleetPageGate'
import RequiredLabelsField from '../RequiredLabelsField'
import TargetNodeField from '../TargetNodeField'

const BACK = { href: '/nodes?tab=rooms', label: '房间' }

function RoomPage() {
  const router = useRouter()
  const params = useSearchParams()
  const rawId = params.get('id')
  const id = Number(rawId)
  const rooms = useSWR<FleetRooms>(rawId ? FLEET_ROOMS_KEY : null, fetcher)
  const nodes = useSWR<FleetNodes>(FLEET_NODES_KEY, fetcher)
  const templates = useSWR<FleetTemplate[]>(FLEET_TEMPLATES_KEY, fetcher)
  const room = rooms.data?.rooms.find(r => r.id === id && !r.deleted_at)
  const title = rawId ? `编辑房间${room ? `「${room.remark}」` : ''}` : '新建房间'
  const error = rooms.error ?? nodes.error ?? templates.error
  let state
  if (error) state = <Empty title="加载失败" description={errorMessage(error)}><Button onClick={() => { rooms.mutate(); nodes.mutate(); templates.mutate() }}>重试</Button></Empty>
  else if (rawId !== null && (!Number.isInteger(id) || id <= 0)) state = <Empty title="无效的房间编号" />
  else if (!nodes.data || !templates.data || (rawId && !rooms.data)) state = <Spin />
  else if (rawId && !room) state = <Empty title="房间不存在或已被删除" />
  if (state) return <FormPage title={title} back={BACK}>{state}</FormPage>

  const save = async (values: Record<string, any>) => {
    try {
      if (room) await updateRoom(room.id, roomFromForm(values))
      else await createRoom({ ...roomFromForm(values), node_id: values.node_id === -1 ? null : values.node_id ?? null, auto_node: values.node_id === -1 })
      Toast.success(room ? '已保存，已下发到节点' : '已添加房间')
      await Promise.all([mutate(FLEET_ROOMS_KEY), mutate(FLEET_NODES_KEY)])
      router.push(BACK.href)
    } catch (error) {
      Toast.error({ content: errorMessage(error), duration: 6 })
      throw error
    }
  }
  return <StreamerEditor key={room?.id ?? 'new'} fleet entity={room ? roomToForm(room) : undefined} title={title} back={BACK} onOk={save}
    templateOptions={templates.data!.map(t => ({ value: t.id, label: t.template_name }))}
    extraFields={<><RequiredLabelsField nodes={nodes.data!.nodes} />{!room && <TargetNodeField nodes={nodes.data!.nodes} />}</>} />
}

export default function Page() {
  return <Suspense><FleetPageGate title="房间配置" back={BACK} perm="node.manage" denied="编辑房间需要管理节点权限" fill><RoomPage /></FleetPageGate></Suspense>
}
