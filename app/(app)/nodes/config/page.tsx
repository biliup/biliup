'use client'
import { Suspense, useEffect } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import { Spin } from '@douyinfe/semi-ui'
import { useMe } from '@/app/lib/use-me'
import { FLEET_CONFIG_BACK, FleetConfigStatePage, FleetGlobalConfigPage } from '../FleetConfigEditor'
import FleetPageGate from '../FleetPageGate'
import styles from '../fleet-config.module.scss'

/** `/nodes/config`：全局 Fleet 配置。节点覆盖是节点的附属内容，在节点页上以抽屉打开（`/nodes?override=N`） */
function FleetConfig() {
  const router = useRouter()
  const searchParams = useSearchParams()
  const nodeId = Number(searchParams.get('node'))
  const legacyOverride = Number.isInteger(nodeId) && nodeId > 0
  const { can } = useMe()

  // #1797 时节点覆盖是 /nodes/config?node=N，旧链接转到节点页并打开那台节点的抽屉
  useEffect(() => {
    if (legacyOverride) router.replace(`/nodes?override=${nodeId}`)
  }, [legacyOverride, nodeId, router])

  if (legacyOverride) {
    return (
      <FleetConfigStatePage title="节点覆盖">
        <div className={styles.center}>
          <Spin size="large" />
        </div>
      </FleetConfigStatePage>
    )
  }
  return <FleetGlobalConfigPage canManage={can('node.manage')} />
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
