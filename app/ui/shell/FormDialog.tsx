'use client'
import { Modal } from '@douyinfe/semi-ui'
import type { ReactNode } from 'react'
import { useIsMobile } from '@/app/lib/useIsMobile'
import ShellFooter, { type ShellActions } from './ShellFooter'
import { DIALOG_WIDTH, NARROW, type DialogSize } from './sizes'
import styles from './shell.module.scss'

export type FormDialogProps = ShellActions & {
  title: ReactNode
  visible?: boolean
  size?: DialogSize
  /** 省略时用统一底栏；null 为不要底栏（查看型，如播放器）；传节点则完全自定义 */
  footer?: ReactNode | null
  /** false：提交中不让关（Esc、右上角 ×、「取消」都不响应） */
  closable?: boolean
  afterClose?: () => void
  children?: ReactNode
}

/** 居中弹窗：一次性的短任务。窄屏下 lg 档全屏，其余两档左右各留 16px */
export default function FormDialog({
  title,
  visible = true,
  size = 'md',
  footer,
  closable = true,
  afterClose,
  children,
  ...actions
}: FormDialogProps) {
  const isMobile = useIsMobile(NARROW)
  const full = isMobile && size === 'lg'
  const onCancel = closable ? actions.onCancel : undefined
  return (
    <Modal
      title={title}
      visible={visible}
      centered={!full}
      fullScreen={full}
      width={full ? undefined : `min(${DIALOG_WIDTH[size]}px, calc(100vw - 32px))`}
      className={`${styles.dialog} ${full ? styles.dialogFull : ''}`}
      closable={closable}
      closeOnEsc={closable}
      onCancel={onCancel}
      afterClose={afterClose}
      footer={footer === undefined ? <ShellFooter {...actions} onCancel={onCancel} /> : footer}
    >
      {children}
    </Modal>
  )
}
