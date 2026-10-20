'use client'
import { Typography } from '@douyinfe/semi-ui'
import { ReactNode } from 'react'
import Link from 'next/link'
import styles from './PageHeader.module.scss'

const { Title, Text } = Typography

/**
 * 统一的页面头部范式：标题 + 描述 + 右侧操作区。
 * 所有业务页面都应以 <PageHeader /> 开头，保证全局一致。
 */
export default function PageHeader({
  title,
  description,
  icon,
  actions,
  breadcrumbs,
}: {
  title: ReactNode
  description?: ReactNode
  icon?: ReactNode
  actions?: ReactNode
  breadcrumbs?: { href: string; label: string }[]
}) {
  return (
    <header className={styles.header}>
      {icon && !breadcrumbs ? <div className={styles.icon}>{icon}</div> : null}
      <div className={styles.titles}>
        {breadcrumbs ? <nav aria-label="面包屑"><ol className={styles.breadcrumbs}>
          {breadcrumbs.map(crumb => <li key={crumb.href}><Link href={crumb.href} prefetch={false}>{crumb.label}</Link><span aria-hidden="true">/</span></li>)}
          <li aria-current="page"><Title heading={4} style={{ margin: 0 }}>{title}</Title></li>
        </ol></nav> :
        <Title heading={4} style={{ margin: 0 }}>
          {title}
        </Title>}
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
