'use client'
import React from 'react'
import { LiveStreamerEntity, StreamerInfo } from '@/app/lib/api-streamer'
import { streamerStatusMeta, uploadStatusTag, platformName } from '@/app/lib/status'
import { timeAgo, formatDuration } from '@/app/lib/use-dashboard'
import styles from './streamer-card.module.scss'

export interface StreamerCardProps {
  streamer: LiveStreamerEntity
  /** 匹配 url 的最新 streamer-info(标题 / 最近开始时间) */
  info?: StreamerInfo
  /** 直播管理页的批量选择 */
  selectable?: boolean
  selected?: boolean
  onToggleSelect?: (id: number) => void
  /** 卡片底部操作区(编辑 / 暂停 / 删除等) */
  actions?: React.ReactNode
}

/** 平台品牌色:用于右上角平台 chip,打破卡片单调并提供平台识别 */
const PLAT_COLORS: Record<string, string> = {
  哔哩哔哩: '#fb7299',
  虎牙直播: '#ff6a00',
  斗鱼直播: '#ff6b35',
  抖音直播: '#22c1c3',
  YouTube: '#ff0000',
  Twitch: '#9146ff',
  CC直播: '#3aac5f',
  快手直播: '#ff4906',
}
const DEFAULT_PLAT_COLOR = '#6b6c75'

/** 平台色 → 10% 淡底(hex + alpha 后缀) */
function platTint(color: string): string {
  return color + '1a'
}

/**
 * 全局统一的直播间卡片:
 * 状态徽章 + 平台色 chip + 名称 + 实时时长(直播中)/ 最近录制(未开播)
 * + 标题 + URL + 上传状态。主页与直播管理页共用。
 */
export default function StreamerCard({
  streamer,
  info,
  selectable,
  selected,
  onToggleSelect,
  actions,
}: StreamerCardProps) {
  const meta = streamerStatusMeta(streamer.status)
  const live = streamer.status === 'Working'
  const paused = streamer.status === 'Pause'
  const name = streamer.remark || streamer.url

  // 实时时长:直播中时由"直播开始时间"计算
  let duration: string | null = null
  if (live && info?.date) {
    duration = formatDuration(Date.now() / 1000 - info.date)
  }
  const lastRec = info?.date && !live ? timeAgo(info.date) : null

  const plat = platformName(streamer.url)
  const platColor = PLAT_COLORS[plat] ?? DEFAULT_PLAT_COLOR

  const cardCls = [
    styles.card,
    live ? styles.rec : '',
    paused ? styles.paused : '',
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <article className={cardCls} data-id={streamer.id}>
      {/* 头部:状态徽章(左)+ 平台 chip(右) */}
      <div className={styles.topRow}>
        <span className={styles.left}>
          {selectable ? (
            <input
              type="checkbox"
              className={styles.chk}
              checked={!!selected}
              onChange={() => onToggleSelect?.(streamer.id)}
              aria-label={`选择 ${name}`}
            />
          ) : null}
          <span className={`${styles.badge} ${styles[meta.cls]}`}>
            <span className={styles.bdot} />
            {meta.label}
          </span>
        </span>
        <span
          className={styles.platChip}
          style={{ backgroundColor: platTint(platColor), color: platColor }}
          title={streamer.url}
        >
          <span className={styles.platDot} style={{ backgroundColor: platColor }} />
          {plat}
        </span>
      </div>

      {/* 名称 + 实时时长 / 最近录制 */}
      <div className={styles.nameRow}>
        <div className={styles.name} title={name}>
          {name}
        </div>
        {duration ? (
          <span className={styles.liveInfo}>{duration}</span>
        ) : lastRec ? (
          <span className={styles.recInfo}>{lastRec}</span>
        ) : null}
      </div>

      {/* 标题行:始终渲染(空字符串占位),保证所有卡片等高、标题基线对齐 */}
      <div className={styles.title} title={info?.title || ''}>
        {info?.title || ''}
      </div>
      <a
        className={styles.url}
        href={streamer.url}
        target="_blank"
        rel="noreferrer"
        title={streamer.url}
      >
        {streamer.url}
      </a>
      <div className={styles.meta}>{uploadStatusTag(streamer.upload_status)}</div>
      {actions ? <div className={styles.actions}>{actions}</div> : null}
    </article>
  )
}
