'use client'
import { Suspense } from 'react'
import useSWR from 'swr'
import { useRouter, useSearchParams } from 'next/navigation'
import { Button, Empty, Spin } from '@douyinfe/semi-ui'
import { fetcher } from '@/app/lib/api-streamer'
import { useMe } from '@/app/lib/use-me'
import {
  errorMessage,
  FLEET_NODES_KEY,
  FLEET_TEMPLATES_KEY,
  type FleetNodes,
  type FleetTemplate,
} from '@/app/lib/use-fleet'
import { FormPage } from '@/app/ui/shell'
import FleetTemplateEditor, { FLEET_TEMPLATES_BACK } from '../FleetTemplateEditor'
import FleetPageGate from '../FleetPageGate'

/** `/nodes/template` 新建、`/nodes/template?id=N` 编辑 Fleet 投稿模板 */
function FleetTemplatePage() {
  const router = useRouter()
  const searchParams = useSearchParams()
  const id = Number(searchParams.get('id'))
  const editing = Number.isInteger(id) && id > 0
  const { me } = useMe()
  const controller = me?.fleet_controller === true
  const { data: nodes, error: nodesError } = useSWR<FleetNodes>(controller ? FLEET_NODES_KEY : null, fetcher)
  const { data: templates, error, mutate } = useSWR<FleetTemplate[]>(controller ? FLEET_TEMPLATES_KEY : null, fetcher)
  const back = () => router.push(FLEET_TEMPLATES_BACK.href)
  const title = editing ? '编辑投稿模板' : '新建投稿模板'

  const loadError = error ?? nodesError
  if (loadError) {
    return (
      <FormPage title={title} back={FLEET_TEMPLATES_BACK}>
        <Empty title="加载失败" description={errorMessage(loadError)} style={{ padding: '48px 0' }}>
          <Button onClick={() => mutate()}>重试</Button>
        </Empty>
      </FormPage>
    )
  }
  if (!nodes || !templates) {
    return (
      <FormPage title={title} back={FLEET_TEMPLATES_BACK}>
        <div style={{ padding: '64px 0', textAlign: 'center' }}>
          <Spin size="large" />
        </div>
      </FormPage>
    )
  }
  const template = editing ? templates.find((t) => t.id === id) : null
  if (template === undefined) {
    return (
      <FormPage title={title} back={FLEET_TEMPLATES_BACK}>
        <Empty title="没有这个模板" description={`投稿模板 #${id} 不存在或已被删除`} style={{ padding: '48px 0' }} />
      </FormPage>
    )
  }
  return (
    <FleetTemplateEditor
      key={template?.id ?? 'new'}
      template={template}
      nodes={nodes.nodes}
      onSaved={() => {
        mutate().catch(() => undefined)
        back()
      }}
    />
  )
}

export default function Page() {
  return (
    <Suspense>
      <FleetPageGate
        title="Fleet 投稿模板"
        back={FLEET_TEMPLATES_BACK}
        perm="node.manage"
        denied="新建和修改 Fleet 投稿模板需要「管理节点」权限"
      >
        <FleetTemplatePage />
      </FleetPageGate>
    </Suspense>
  )
}
