'use client'
import { Modal } from '@douyinfe/semi-ui'
import type { ReactNode } from 'react'
import { useIsMobile } from '@/app/lib/useIsMobile'
import ShellFooter, { type ShellActions } from './ShellFooter'
import { DIALOG_WIDTH, IMMERSIVE_MAX_WIDTH, IMMERSIVE_TITLE_HEIGHT, NARROW, type DialogSize } from './sizes'
import styles from './shell.module.scss'

export type FormDialogProps = ShellActions & {
  title: ReactNode
  visible?: boolean
  size?: DialogSize
  /** 省略时用统一底栏；null 为不要底栏（查看型，如播放器）；传节点则完全自定义 */
  footer?: ReactNode | null
  /** false：提交中不让关（Esc、右上角 ×、「取消」都不响应） */
  closable?: boolean
  /** 沉浸式（播放器）：深色底、画面贴边，标题与关闭按钮在极简顶栏里，没有底栏；宽度按 16:9 画面放得下为准，窄屏全屏 */
  immersive?: boolean
  afterClose?: () => void
  children?: ReactNode
}

/** 居中弹窗：一次性的短任务。窄屏下 lg 档与沉浸式全屏，其余两档左右各留 16px */
export default function FormDialog({
  title,
  visible = true,
  size = 'md',
  footer,
  closable = true,
  immersive = false,
  afterClose,
  children,
  ...actions
}: FormDialogProps) {
  const isMobile = useIsMobile(NARROW)
  const full = isMobile && (size === 'lg' || immersive)
  const onCancel = closable ? actions.onCancel : undefined
  const width = immersive
    ? `min(${IMMERSIVE_MAX_WIDTH}px, calc(100vw - 32px), calc((100dvh - ${32 + IMMERSIVE_TITLE_HEIGHT}px) * 16 / 9))`
    : `min(${DIALOG_WIDTH[size]}px, calc(100vw - 32px))`
  return (
    <Modal
      title={title}
      visible={visible}
      centered={!full}
      fullScreen={full}
      width={full ? undefined : width}
      className={[styles.dialog, full && styles.dialogFull, immersive && styles.immersive].filter(Boolean).join(' ')}
      closable={closable}
      closeOnEsc={closable}
      onCancel={onCancel}
      afterClose={afterClose}
      footer={immersive ? null : footer === undefined ? <ShellFooter {...actions} onCancel={onCancel} /> : footer}
    >
      {children}
    </Modal>
  )
}
