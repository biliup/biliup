'use client'
import React, { useState } from 'react'
import Image from 'next/image'
import { LiveStreamerEntity, StreamerInfo } from '@/app/lib/api-streamer'
import { streamerStatusMeta, uploadStatusTag, platformName } from '@/app/lib/status'
import { timeAgo, formatDuration, formatRate, liveImageUrl } from '@/app/lib/use-dashboard'
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
 * 录制中的直播间封面缩略图(16:9,懒加载)。
 * 加载失败就整块消失,卡片回到没有封面时的样式;加载中显示占位底色。
 * 调用方用 key={src} 挂载,封面地址变化时状态自然重置。
 */
export function LiveCover({ src, alt }: { src: string; alt: string }) {
  const [state, setState] = useState<'loading' | 'ok' | 'error'>('loading')
  if (state === 'error') return null
  return (
    <figure className={styles.cover} data-state={state}>
      <Image
        src={src}
        alt={alt}
        fill
        sizes="(max-width: 600px) 100vw, 400px"
        loading="lazy"
        onLoad={() => setState('ok')}
        onError={() => setState('error')}
      />
    </figure>
  )
}

/** 录制中的主播头像小圆图;加载失败则不占位。 */
export function LiveAvatar({ src }: { src: string }) {
  const [failed, setFailed] = useState(false)
  if (failed) return null
  return (
    <Image
      className={styles.avatar}
      src={src}
      alt=""
      aria-hidden="true"
      width={22}
      height={22}
      loading="lazy"
      onError={() => setFailed(true)}
    />
  )
}

/**
 * 全局统一的直播间卡片:
 * 状态徽章 + 平台色 chip + [录制中:封面 / 头像 / 写盘速率] + 名称 + 实时时长(直播中)/ 最近录制(未开播)
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

  // 录制中才有封面 / 头像 / 速率;图片走同源代理,避开图片 CDN 的 Referer 校验
  const coverSrc = live ? liveImageUrl(streamer.id, 'cover', streamer.live_cover_url) : null
  const avatarSrc = live ? liveImageUrl(streamer.id, 'avatar', streamer.live_avatar_url) : null
  const rate = live ? formatRate(streamer.live_bytes_per_sec) : null

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

      {/* 录制中:直播间封面 */}
      {coverSrc ? <LiveCover key={coverSrc} src={coverSrc} alt={`${name} 的直播间封面`} /> : null}

      {/* 名称(录制中带头像)+ 实时时长与写盘速率 / 最近录制 */}
      <div className={styles.nameRow}>
        <div className={styles.nameWrap}>
          {avatarSrc ? <LiveAvatar key={avatarSrc} src={avatarSrc} /> : null}
          <div className={styles.name} title={name}>
            {name}
          </div>
        </div>
        {live ? (
          <span className={styles.liveStats}>
            {duration ? <span className={styles.liveInfo}>{duration}</span> : null}
            <span
              className={styles.rate}
              title={rate ? '写盘速率(最近 10 秒平均)' : '写盘速率:尚无采样'}
              aria-label={rate ? `写盘速率 ${rate}` : '写盘速率尚无采样'}
            >
              {rate ?? '—'}
            </span>
          </span>
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
