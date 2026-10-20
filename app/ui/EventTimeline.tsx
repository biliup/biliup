'use client'
import React from 'react'
import { DashboardEvent } from '@/app/lib/use-dashboard'
import Link from 'next/link'
import styles from './event-timeline.module.scss'

function fmtTime(ts: number): string {
  if (!Number.isFinite(ts)) return '时间未知'
  return new Date(ts).toLocaleString('zh-CN')
}

export default function EventTimeline({ events, error = false }: { events: DashboardEvent[]; error?: boolean }) {
  if (error) return <div className={styles.empty}>事件记录暂时无法加载，控制台其它数据仍可正常使用。</div>
  if (events.length === 0) {
    return (
      <div className={styles.empty}>
        暂无事件记录
      </div>
    )
  }
  return (
    <div className={styles.timeline}>
      {events.map((e) => (
        <div key={e.id} className={`${styles.item} ${e.kind.includes('start') ? styles.start : e.kind.includes('file') ? styles.file : ''}`}>
          <span className={styles.time}>{fmtTime(e.ts)}</span>
          <span className={styles.text}>{e.text}</span>
          {e.sub ? <span className={styles.tag}>{e.sub}</span> : null}
          {e.session_id != null ? <Link className={styles.link} href={`/replay?session=${e.session_id}`}>打开场次</Link> : null}
        </div>
      ))}
    </div>
  )
}
