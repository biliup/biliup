'use client'
import { Typography } from '@douyinfe/semi-ui'
import { ReactNode, useEffect, useRef } from 'react'
import styles from './PageHeader.module.scss'

const { Title, Text } = Typography

/**
 * 统一的页面头部范式：标题 + 描述 + 右侧操作区。
 * 所有业务页面都应以 <PageHeader /> 开头，保证全局一致。
 *
 * 页头吸顶，实际高度随描述折行变化；这里把它写到 :root 的
 * `--page-header-height`，页面内需要贴在页头下方吸顶的元素直接引用即可。
 */
export default function PageHeader({
  title,
  description,
  icon,
  actions,
}: {
  title: ReactNode
  description?: ReactNode
  icon?: ReactNode
  actions?: ReactNode
}) {
  const ref = useRef<HTMLElement>(null)

  useEffect(() => {
    const el = ref.current
    if (!el || typeof ResizeObserver === 'undefined') return
    const root = document.documentElement
    const update = () => root.style.setProperty('--page-header-height', `${el.offsetHeight}px`)
    update()
    const ro = new ResizeObserver(update)
    ro.observe(el)
    return () => {
      ro.disconnect()
      root.style.removeProperty('--page-header-height')
    }
  }, [])

  return (
    <header className={styles.header} ref={ref}>
      {icon ? <div className={styles.icon}>{icon}</div> : null}
      <div className={styles.titles}>
        <Title heading={4} style={{ margin: 0 }}>
          {title}
        </Title>
        {description ? (
          <Text type="tertiary" size="small">
            {description}
          </Text>
        ) : null}
      </div>
      {actions ? <div className={styles.actions}>{actions}</div> : null}
    </header>
  )
}
