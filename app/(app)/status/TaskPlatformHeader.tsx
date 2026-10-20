'use client'

import type { ReactNode } from 'react'
import Link from 'next/link'
import { IconSetting } from '@douyinfe/semi-icons'
import { useMe } from '@/app/lib/use-me'
import PageHeader from '../components/PageHeader'
import styles from './platform.module.scss'

export default function TaskPlatformHeader({ active, description, actions }: {
  active: 'status' | 'nodes'; description?: string; actions?: ReactNode
}) {
  const { me } = useMe()
  const nodes = me?.fleet_controller || me?.fleet_node?.pair
  return <>
    <PageHeader icon={<IconSetting size="large" />} title="任务平台" description={description} actions={actions} />
    <nav className={styles.tabs} aria-label="任务平台">
      <Link href="/status" prefetch={false} aria-current={active === 'status' ? 'page' : undefined}>运行状态</Link>
      {nodes && <Link href="/nodes" prefetch={false} aria-current={active === 'nodes' ? 'page' : undefined}>
        {me?.fleet_controller ? '节点管理' : '一主一备'}
      </Link>}
    </nav>
  </>
}
