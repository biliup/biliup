'use client'
import { Button } from '@douyinfe/semi-ui'
import { IconArrowLeft } from '@douyinfe/semi-icons'
import Link from 'next/link'
import { useState, useSyncExternalStore, type ReactNode } from 'react'
import PageHeader from '@/app/(app)/components/PageHeader'
import styles from './shell.module.scss'

export type FormPageProps = {
  title: ReactNode
  description?: ReactNode
  /** 返回哪一页：页头左侧的返回箭头回到这里。页面没有「取消」 */
  back: { href: string; label: string }
  /** 页头右上角的主按钮，写动作本身（创建模板 / 保存模板）；不传则页头不放按钮 */
  okText?: ReactNode
  okIcon?: ReactNode
  /** 返回 Promise 时按钮自动转圈；reject 时停在页面上（调用方负责提示），不丢已填内容 */
  onOk?: () => unknown
  okDisabled?: boolean
  /** 保存由表单自己的 onSubmit 驱动时，由调用方给出转圈状态 */
  okLoading?: boolean
  /** 页头主按钮左边的次要操作（如「清空覆盖」） */
  headerExtra?: ReactNode
  /** 内容占满页头下方的剩余高度、页面本身不滚，由内容自己分区滚动（配置页的 Tab 面板） */
  fill?: boolean
  children?: ReactNode
}

/** 表单页：页头（返回箭头 + 标题 + 右上角主按钮）+ 内容列。不套卡片，分区靠表单自己的分节标题 */
export default function FormPage({
  title,
  description,
  back,
  okText,
  okIcon,
  onOk,
  okDisabled,
  okLoading,
  headerExtra,
  fill,
  children,
}: FormPageProps) {
  const [busy, setBusy] = useState(false)
  const run = async () => {
    if (!onOk || busy) return
    setBusy(true)
    try {
      await onOk()
    } catch {
      // 校验失败或接口报错：表单已标红 / 调用方已弹提示
    } finally {
      setBusy(false)
    }
  }
  return (
    <>
      <PageHeader
        icon={
          <Link
            href={back.href}
            prefetch={false}
            className={styles.pageBack}
            aria-label={`返回${back.label}`}
            title={`返回${back.label}`}
          >
            <IconArrowLeft size="large" />
          </Link>
        }
        title={title}
        description={description}
        actions={
          headerExtra || okText ? (
            <>
              {headerExtra}
              {okText ? (
                <Button
                  theme="solid"
                  icon={okIcon}
                  onClick={run}
                  loading={busy || okLoading}
                  disabled={okDisabled || !onOk}
                >
                  {okText}
                </Button>
              ) : null}
            </>
          ) : undefined
        }
      />
      <div className={fill ? styles.pageFill : styles.pageBody}>{children}</div>
    </>
  )
}

const WIDE_FORM = '(min-width: 1024px)'

function subscribe(onChange: () => void) {
  const mq = window.matchMedia(WIDE_FORM)
  mq.addEventListener('change', onChange)
  return () => mq.removeEventListener('change', onChange)
}

/** 表单页的标签位置：宽屏在左、窄屏在上。抽屉与弹窗一律在上 */
export function usePageLabelPosition(): 'left' | 'top' {
  const wide = useSyncExternalStore(
    subscribe,
    () => window.matchMedia(WIDE_FORM).matches,
    () => false,
  )
  return wide ? 'left' : 'top'
}
