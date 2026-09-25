'use client'
import React, { useState } from 'react'
import Image from 'next/image'
import { LiveStreamerEntity, StreamerInfo } from '@/app/lib/api-streamer'
import { streamerStatusMeta, uploadStatusTag, platformName } from '@/app/lib/status'
import {
  timeAgo,
  formatDuration,
  formatRate,
  liveImageUrl,
  useNowSec,
  canPreview,
} from '@/app/lib/use-dashboard'
import { LivePreviewButton, LivePreviewModal } from './LivePreview'
import { MarkerCount } from './MarkerControls'
import { LiveRateChart, SPARK_RATE_WINDOW_MS, useCardRateChart } from './LiveRateChart'
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
 * 传入 onPreview 时封面可点击,中央显示播放标记,点击打开直播预览。
 */
export function LiveCover({
  src,
  alt,
  onPreview,
  overlay,
  onStateChange,
}: {
  src: string
  alt: string
  onPreview?: () => void
  /** 叠在封面底部的一层（码率 sparkline）；点击会冒泡到封面本身的 onClick */
  overlay?: React.ReactNode
  /** 加载结果回调：父组件据此决定 overlay 放不放得下（加载失败整块消失） */
  onStateChange?: (state: 'ok' | 'error') => void
}) {
  const [state, setState] = useState<'loading' | 'ok' | 'error'>('loading')
  if (state === 'error') return null
  const clickable = !!onPreview && state === 'ok'
  return (
    <figure
      className={`${styles.cover} ${clickable ? styles.coverClickable : ''}`}
      data-state={state}
      onClick={clickable ? onPreview : undefined}
      role={clickable ? 'button' : undefined}
      tabIndex={clickable ? 0 : undefined}
      aria-label={clickable ? '预览直播' : undefined}
      onKeyDown={
        clickable
          ? (e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault()
                onPreview?.()
              }
            }
          : undefined
      }
    >
      <Image
        src={src}
        alt={alt}
        fill
        sizes="(max-width: 600px) 100vw, 400px"
        loading="lazy"
        onLoad={() => {
          setState('ok')
          onStateChange?.('ok')
        }}
        onError={() => {
          setState('error')
          onStateChange?.('error')
        }}
      />
      {clickable ? (
        <span className={styles.coverPlay} aria-hidden="true">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor">
            <path d="M8 5.5v13l11-6.5z" />
          </svg>
        </span>
      ) : null}
      {overlay ? <div className={styles.coverOverlay}>{overlay}</div> : null}
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
 * 直播中的实时时长:由"直播开始时间"和共享时钟算出。
 * 只有录制中的卡片挂载它,所以只有这些卡片会随时钟每秒重渲染这一小块,其余卡片不受影响。
 */
function LiveDuration({ since }: { since: number }) {
  const now = useNowSec()
  return <span className={styles.liveInfo}>{formatDuration(now - since)}</span>
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
  // 封面点击打开的预览弹层;按钮自带一套,两处入口共用同一个弹层状态
  const [previewOpen, setPreviewOpen] = useState(false)
  const previewable = canPreview(streamer)

  const lastRec = info?.date && !live ? timeAgo(info.date) : null

  // 录制中才有封面 / 头像 / 速率;图片走同源代理,避开图片 CDN 的 Referer 校验
  const coverSrc = live ? liveImageUrl(streamer.id, 'cover', streamer.live_cover_url) : null
  const avatarSrc = live ? liveImageUrl(streamer.id, 'avatar', streamer.live_avatar_url) : null
  const rate = live ? formatRate(streamer.live_bytes_per_sec) : null

  // 码率 sparkline(页面工具栏「码率图」开关,默认关):有封面就叠在封面底部,封面没有 / 加载失败就放名称行下
  const [rateChartOn] = useCardRateChart()
  const [coverFailed, setCoverFailed] = useState(false)
  const coverShown = !!coverSrc && !coverFailed
  const sparkline =
    live && rateChartOn ? (
      <LiveRateChart
        id={streamer.id}
        windowMs={SPARK_RATE_WINDOW_MS}
        variant="sparkline"
        height={coverShown ? 34 : 28}
        label={`${name} 写盘速率`}
        className={coverShown ? undefined : styles.spark}
        overlay={coverShown}
      />
    ) : null

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
          {live ? <MarkerCount count={streamer.marker_count} /> : null}
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

      {/* 录制中:直播间封面(可预览时点击即打开播放器) */}
      {coverSrc ? (
        <LiveCover
          key={coverSrc}
          src={coverSrc}
          alt={`${name} 的直播间封面`}
          onPreview={previewable ? () => setPreviewOpen(true) : undefined}
          overlay={coverShown ? sparkline : null}
          onStateChange={(s) => setCoverFailed(s === 'error')}
        />
      ) : null}

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
            {info?.date ? <LiveDuration since={info.date} /> : null}
            <span
              className={styles.rate}
              title={rate ? '写盘速率(最近 10 秒平均)' : '写盘速率:尚无采样'}
              aria-label={rate ? `写盘速率 ${rate}` : '写盘速率尚无采样'}
            >
              {rate ?? '—'}
            </span>
            <LivePreviewButton streamer={streamer} size="small" iconOnly className={styles.previewBtn} />
          </span>
        ) : lastRec ? (
          <span className={styles.recInfo}>{lastRec}</span>
        ) : null}
      </div>

      {/* 开了码率图但没有封面可叠:sparkline 放这一行 */}
      {sparkline && !coverShown ? sparkline : null}

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
      {previewOpen ? (
        <LivePreviewModal
          streamer={streamer}
          visible={previewOpen}
          onClose={() => setPreviewOpen(false)}
        />
      ) : null}
    </article>
  )
}
