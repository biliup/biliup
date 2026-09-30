'use client'
import type { ReactNode } from 'react'
import { Button, Empty, Spin } from '@douyinfe/semi-ui'
import { IconServer } from '@douyinfe/semi-icons'
import { useMe, type Permission } from '@/app/lib/use-me'
import { errorMessage } from '@/app/lib/use-fleet'
import { FormPage } from '@/app/ui/shell'
import styles from './page.module.scss'

/**
 * 只在 Fleet 控制面上有意义的页面（投稿模板、Fleet 配置）先过这一道：
 * 本机不是控制面、或当前账号没有 `perm` 时直接说明原因，不去请求数据，也不给出可编辑的表单。
 * 侧栏的「节点」项按 `streamer.view` 放行、`controllerOnly` 只管显隐，手输地址进来的都靠这里挡。
 */
export default function FleetPageGate({
  title,
  back,
  perm,
  denied,
  fill,
  children,
}: {
  title: string
  back: { href: string; label: string }
  perm: Permission
  /** 没有 `perm` 时的说明 */
  denied: string
  fill?: boolean
  children: ReactNode
}) {
  const { me, error, can } = useMe()
  if (me && me.fleet_controller && can(perm)) return <>{children}</>
  let body: ReactNode
  if (!me) {
    body = error ? (
      <>
        <Empty title="加载失败" description={errorMessage(error)} />
        <Button onClick={() => window.location.reload()}>重试</Button>
      </>
    ) : (
      <Spin size="large" />
    )
  } else if (!me.fleet_controller) {
    body = (
      <Empty
        image={<IconServer size="extra-large" style={{ color: 'var(--semi-color-text-3)' }} />}
        title="本机不是控制面"
        description="用 biliup server --controller 启动后，才能在这里管理 Fleet 的投稿模板与配置"
      />
    )
  } else {
    body = <Empty title="没有权限" description={denied} />
  }
  return (
    <FormPage title={title} back={back} fill={fill}>
      <div className={styles.center}>{body}</div>
    </FormPage>
  )
}
