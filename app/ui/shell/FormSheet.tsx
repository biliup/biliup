'use client'
import { SideSheet } from '@douyinfe/semi-ui'
import type { ReactNode } from 'react'
import { useIsMobile } from '@/app/lib/useIsMobile'
import ShellFooter, { type ShellActions } from './ShellFooter'
import { NARROW, SHEET_WIDTH, type SheetSize } from './sizes'
import styles from './shell.module.scss'

export type FormSheetProps = ShellActions & {
  title: ReactNode
  visible?: boolean
  size?: SheetSize
  /** 省略时用统一底栏；null 为不要底栏 */
  footer?: ReactNode | null
  /** 正文不滚、占满标题与底栏之间，由内容自己分区滚动（带 Tab 的配置） */
  fill?: boolean
  closable?: boolean
  children?: ReactNode
}

/** 右侧抽屉：列表里某一项的查看与编辑。窄屏下占满宽度；标题栏右侧只有关闭按钮，操作都在底栏 */
export default function FormSheet({
  title,
  visible = true,
  size = 'md',
  footer,
  fill,
  closable = true,
  children,
  ...actions
}: FormSheetProps) {
  const isMobile = useIsMobile(NARROW)
  const onCancel = closable ? actions.onCancel : undefined
  return (
    <SideSheet
      visible={visible}
      width={isMobile ? '100%' : SHEET_WIDTH[size]}
      className={`${styles.sheet} ${fill ? styles.sheetFill : ''}`}
      closable={closable}
      closeOnEsc={closable}
      onCancel={onCancel}
      title={
        <div className={styles.sheetTitle}>
          <span className={styles.sheetTitleText}>{title}</span>
        </div>
      }
      footer={footer === undefined ? <ShellFooter {...actions} onCancel={onCancel} /> : footer}
    >
      {children}
    </SideSheet>
  )
}
