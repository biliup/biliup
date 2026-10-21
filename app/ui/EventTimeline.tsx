'use client'
import React from 'react'
import { DashboardEvent } from '@/app/lib/use-dashboard'
import Link from 'next/link'
import styles from './event-timeline.module.scss'

const dateFormat = new Intl.DateTimeFormat('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' })
const timeFormat = new Intl.DateTimeFormat('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false })

export default function EventTimeline({ events, error = false }: { events: DashboardEvent[]; error?: boolean }) {
  if (error) return <div className={styles.empty}>事件记录暂时无法加载，概览其它数据仍可正常使用。</div>
  if (events.length === 0) {
    return (
      <div className={styles.empty}>
        暂无事件记录
      </div>
    )
  }
  const groups = new Map<string, DashboardEvent[]>()
  for (const event of events) {
    const date = new Date(event.ts)
    const day = Number.isFinite(date.getTime()) ? dateFormat.format(date) : '日期未知'
    const group = groups.get(day) ?? []
    group.push(event)
    groups.set(day, group)
  }
  return (
    <div className={styles.groups}>
      {[...groups].map(([day, items]) => (
        <section key={day} aria-label={day}>
          <h3 className={styles.date}>{day}</h3>
          <ol className={styles.timeline}>
            {items.map(e => (
              <li key={e.id} className={`${styles.item} ${e.kind.includes('start') ? styles.start : e.kind.includes('file') ? styles.file : ''}`}>
                <div className={styles.meta}>
                  <time className={styles.time} dateTime={Number.isFinite(new Date(e.ts).getTime()) ? new Date(e.ts).toISOString() : undefined}>
                    {Number.isFinite(new Date(e.ts).getTime()) ? timeFormat.format(new Date(e.ts)) : '时间未知'}
                  </time>
                  {e.session_id != null ? <Link className={styles.link} href={`/replay?session=${e.session_id}`} prefetch={false}>打开场次</Link> : null}
                </div>
                <div className={styles.text}>{e.text}</div>
                {e.sub ? <div className={styles.tag}>{e.sub}</div> : null}
              </li>
            ))}
          </ol>
        </section>
      ))}
    </div>
  )
}
