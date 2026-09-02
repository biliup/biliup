'use client'
import React from 'react'
import { DashboardEvent } from '@/app/lib/use-dashboard'
import styles from './event-timeline.module.scss'

function fmtTime(ts: number): string {
  const d = new Date(ts * 1000)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`
}

/**
 * 最近事件时间线:前端由 /v1/streamer-info(开始录制)与 /v1/videos(文件生成)
 * 合成,零后端改动。
 */
export default function EventTimeline({ events }: { events: DashboardEvent[] }) {
  if (events.length === 0) {
    return (
      <div className={styles.empty}>
        最近 24h 没有录制活动
      </div>
    )
  }
  return (
    <div className={styles.timeline}>
      {events.map((e, idx) => (
        <div key={`${e.kind}-${e.ts}-${idx}`} className={`${styles.item} ${styles[e.kind]}`}>
          <span className={styles.time}>{fmtTime(e.ts)}</span>
          <span className={styles.text}>{e.text}</span>
          {e.sub ? <span className={styles.tag}>{e.sub}</span> : null}
        </div>
      ))}
    </div>
  )
}
