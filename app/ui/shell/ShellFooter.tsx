'use client'
import { Button } from '@douyinfe/semi-ui'
import { useState, type ReactNode } from 'react'
import styles from './shell.module.scss'

export type ShellActions = {
  /** 主按钮文字，写动作本身（创建 / 保存 / 添加 / 启用…）；不传则只有「取消」 */
  okText?: ReactNode
  okIcon?: ReactNode
  /** light：列表型抽屉里「+ 添加」这类不提交表单的主操作；其余一律实心 */
  okTheme?: 'solid' | 'light'
  /** 返回 Promise 时按钮自动转圈；reject 时保持打开（调用方负责提示），不丢已填内容 */
  onOk?: () => unknown
  okType?: 'primary' | 'danger'
  okDisabled?: boolean
  confirmLoading?: boolean
  /** null：不要「取消」（结果页只留主按钮，关闭靠右上角 × 与 Esc） */
  cancelText?: ReactNode
  onCancel?: () => void
  /** 底栏左侧：次要操作或一句提示 */
  footerExtra?: ReactNode
}

export default function ShellFooter({
  okText,
  okIcon,
  okTheme = 'solid',
  onOk,
  okType = 'primary',
  okDisabled,
  confirmLoading,
  cancelText = '取消',
  onCancel,
  footerExtra,
}: ShellActions) {
  const [busy, setBusy] = useState(false)
  const run = async () => {
    if (!onOk || busy) return
    setBusy(true)
    try {
      await onOk()
    } catch {
      // 校验失败或接口报错：表单已标红 / 调用方已弹提示，这里只保证容器不关
    } finally {
      setBusy(false)
    }
  }
  const loading = busy || confirmLoading
  return (
    <div className={styles.footer}>
      {footerExtra ? <div className={styles.footerExtra}>{footerExtra}</div> : null}
      {onCancel && cancelText !== null ? (
        <Button onClick={onCancel} disabled={loading}>
          {cancelText}
        </Button>
      ) : null}
      {okText ? (
        <Button theme={okTheme} type={okType} icon={okIcon} onClick={run} loading={loading} disabled={okDisabled}>
          {okText}
        </Button>
      ) : null}
    </div>
  )
}
