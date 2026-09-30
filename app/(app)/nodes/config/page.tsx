'use client'
import { Suspense } from 'react'
import useSWR from 'swr'
import { useSearchParams } from 'next/navigation'
import { Button, Empty, Spin } from '@douyinfe/semi-ui'
import { fetcher } from '@/app/lib/api-streamer'
import { useMe } from '@/app/lib/use-me'
import { errorMessage, FLEET_NODES_KEY, FLEET_REFRESH_MS, type FleetNodes } from '@/app/lib/use-fleet'
import { FLEET_CONFIG_BACK, FleetConfigStatePage, FleetGlobalConfigPage, FleetNodeConfigPage } from '../FleetConfigEditor'
import FleetPageGate from '../FleetPageGate'
import styles from '../fleet-config.module.scss'

/** `/nodes/config` 为全局 Fleet 配置，`/nodes/config?node=N` 为这台节点的覆盖 */
function FleetConfig() {
  const searchParams = useSearchParams()
  const nodeId = Number(searchParams.get('node'))
  const override = Number.isInteger(nodeId) && nodeId > 0
  const { can } = useMe()
  const canManage = can('node.manage')
  const { data, error, mutate } = useSWR<FleetNodes>(
    override ? FLEET_NODES_KEY : null,
    fetcher,
    { refreshInterval: FLEET_REFRESH_MS },
  )

  if (!override) return <FleetGlobalConfigPage canManage={canManage} />

  const title = '节点覆盖'
  if (!data) {
    return (
      <FleetConfigStatePage title={title}>
        <div className={styles.center}>
          {error ? (
            <>
              <Empty title="加载失败" description={errorMessage(error)} />
              <Button onClick={() => mutate()}>重试</Button>
            </>
          ) : (
            <Spin size="large" />
          )}
        </div>
      </FleetConfigStatePage>
    )
  }
  const node = data.nodes.find((n) => n.id === nodeId)
  if (!node) {
    return (
      <FleetConfigStatePage title={title}>
        <div className={styles.center}>
          <Empty title="节点不存在或已被移除" />
        </div>
      </FleetConfigStatePage>
    )
  }
  return <FleetNodeConfigPage key={node.id} node={node} canManage={canManage} />
}

export default function Page() {
  return (
    <Suspense>
      <FleetPageGate
        title="Fleet 配置"
        back={FLEET_CONFIG_BACK}
        perm="config.view"
        denied="查看 Fleet 配置需要「查看配置」权限"
        fill
      >
        <FleetConfig />
      </FleetPageGate>
    </Suspense>
  )
}
