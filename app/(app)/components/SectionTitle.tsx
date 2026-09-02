'use client'
import { ReactNode } from 'react'
import styles from './SectionTitle.module.scss'

/**
 * 统一的 section 标题范式：主色图标 + 标题。
 * 替代 dashboard 里各 frame 的「硬编码 RGB 色块 + 白色图标」野路子结构，
 * 让所有 section 头共享同一套设计语言（主色图标容器）。
 */
export default function SectionTitle({
  icon,
  title,
}: {
  icon: ReactNode
  title: ReactNode
}) {
  return (
    <div className={styles.wrap}>
      <span className={styles.icon}>{icon}</span>
      <span className={styles.title}>{title}</span>
    </div>
  )
}
