'use client'
import React from 'react'
import { Collapse } from '@douyinfe/semi-ui'

type Props = {
  header: string
  itemKey: string
  /**
   * 只输出字段，不套 Collapse.Panel。
   * 空间配置页由左侧平台列表负责切换，右侧再出现一排带平台名的折叠栏就是重复导航（#1705）；
   * 直播管理页的「配置覆写」弹窗仍以 Collapse.Panel 形式嵌在 Collapse 里。
   */
  bare?: boolean
  children: React.ReactNode
}

const PlatformPanel: React.FC<Props> = ({ header, itemKey, bare, children }) => {
  if (bare) {
    return <>{children}</>
  }
  return (
    <Collapse.Panel header={header} itemKey={itemKey}>
      {children}
    </Collapse.Panel>
  )
}

export default PlatformPanel
