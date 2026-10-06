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

/**
 * 可手输（allowCreate）的数值下拉框用的 `convert`：手输创建的选项值是字符串，而后端这几项是整数
 * （bili_qn / huya_max_ratio / douyu_rate 都是 u32），原样提交会让整份配置反序列化失败（422）。
 * 纯数字转成 number，其它输入原样保留交给校验规则提示。
 */
export const digitsToNumber = (value: unknown) =>
  typeof value === 'string' && /^\d+$/.test(value.trim()) ? Number(value.trim()) : value

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
