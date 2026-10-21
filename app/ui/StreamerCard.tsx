'use client'
import React, { useEffect, useState } from 'react'
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
import { useOpenLivePage } from './LivePreview'
import { MarkerCount } from './MarkerControls'
import { LiveRateChart, SPARK_RATE_WINDOW_MS, useCardRateChart } from './LiveRateChart'
import styles from './streamer-card.module.scss'
import { useVisibleElement } from '@/app/lib/use-visible-element'

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

/** 平台品牌色:用于封面右下角的平台标识,打破卡片单调并提供平台识别 */
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
 * 直播间封面缩略图(16:9,懒加载)，离线时读取最近有效封面。
 * 更新失败时保留已经显示的图片；没有历史图片时保留占位，每分钟重试。
 * 传入 onPreview 时封面可点击,中央显示播放标记,点击进入直播预览页。
 */
export function LiveCover({
  src,
  alt,
  onPreview,
  platform,
  title,
  overlay,
  onStateChange,
}: {
  src: string
  alt: string
  onPreview?: (e: React.MouseEvent | React.KeyboardEvent) => void
  platform?: React.ReactNode
  title?: string
  /** 叠在封面底部的一层（码率 sparkline）；点击会冒泡到封面本身的 onClick */
  overlay?: React.ReactNode
  /** 加载结果回调：父组件据此决定是否叠加码率曲线 */
  onStateChange?: (state: 'ok' | 'error') => void
}) {
  const { ref, active } = useVisibleElement<HTMLElement>()
  const [retryAt, setRetryAt] = useState(0)
  const desiredSrc = retryAt ? `${src}${src.includes('?') ? '&' : '?'}retry=${retryAt}` : src
  const [requestSrc, setRequestSrc] = useState('')
  // Keep the last request while offscreen, including when the source changes during a poll.
  if (active && requestSrc !== desiredSrc) setRequestSrc(desiredSrc)
  const [result, setResult] = useState<{ src: string; state: 'ok' | 'error' } | null>(null)
  const [lastGood, setLastGood] = useState<string | null>(null)
  const state = result?.src === requestSrc ? result.state : 'loading'
  useEffect(() => {
    if (!active || state === 'loading') return
    const timer = setTimeout(() => setRetryAt(Date.now()), state === 'error' ? 60_000 : 300_000)
    return () => clearTimeout(timer)
  }, [active, state, requestSrc])
  const clickable = !!onPreview
  return (
    <figure
      ref={ref}
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
                onPreview?.(e)
              }
            }
          : undefined
      }
    >
      {lastGood && lastGood !== requestSrc ? (
        <Image src={lastGood} alt={alt} fill sizes="(max-width: 600px) 100vw, 400px" style={{ opacity: 1 }} />
      ) : null}
      {requestSrc ? <Image
        key={requestSrc}
        src={requestSrc}
        alt={alt}
        fill
        sizes="(max-width: 600px) 100vw, 400px"
        loading="lazy"
        style={{ opacity: state === 'ok' ? 1 : 0 }}
        onLoad={() => {
          setResult({ src: requestSrc, state: 'ok' })
          setLastGood(requestSrc)
          onStateChange?.('ok')
        }}
        onError={() => {
          setResult({ src: requestSrc, state: 'error' })
          onStateChange?.(lastGood ? 'ok' : 'error')
        }}
      /> : null}
      {!lastGood && state === 'error' ? <span className={styles.coverPlaceholder}>暂无可用封面</span> : null}
      {clickable ? (
        <span className={styles.coverPlay} aria-hidden="true">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor">
            <path d="M8 5.5v13l11-6.5z" />
          </svg>
        </span>
      ) : null}
      {overlay ? <div className={styles.coverOverlay}>{overlay}</div> : null}
      {title || platform ? (
        <div className={styles.coverFooter} data-chart={!!overlay}>
          {title ? <div className={styles.coverTitle} title={title}>{title}</div> : null}
          {platform ? <div className={styles.coverPlatform}>{platform}</div> : null}
        </div>
      ) : null}
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
 * 封面（平台主播名 / 头像、直播标题、状态、平台）+ 录播备注 / 选择框 + URL + 上传状态。
 * 时长、写盘速率 / 最近录制放在底部左侧，操作按钮放右侧。主页与直播管理页共用。
 */
export default function StreamerCard({
  streamer,
  info,
  selectable,
  selected,
  onToggleSelect,
  actions,
}: StreamerCardProps) {
  const { ref, active } = useVisibleElement<HTMLElement>()
  const meta = streamerStatusMeta(streamer.status)
  const live = streamer.status === 'Working'
  const paused = streamer.status === 'Pause'
  const uploadTag = uploadStatusTag(streamer.upload_status)
  const remark = streamer.remark || streamer.url
  // 历史场次 info.name 会被录播备注覆盖，不能用它或 remark 充当平台主播名。
  const broadcasterName = streamer.live_streamer_name?.trim()
  // 封面和「预览」按钮都进直播预览页
  const openLivePage = useOpenLivePage()
  const previewable = canPreview(streamer)

  const lastRec = info?.date && !live ? timeAgo(info.date) : null

  // 封面使用最近一次有效的本地缓存，主播离线时仍可展示；头像与实时速率只在录制时显示
  const coverSrc = liveImageUrl(streamer.id, 'cover', streamer.live_cover_url)
  const avatarSrc = live ? liveImageUrl(streamer.id, 'avatar', streamer.live_avatar_url) : null
  const rate = live ? formatRate(streamer.live_bytes_per_sec) : null

  // 码率 sparkline(页面工具栏「码率图」开关,默认关):有封面就叠在封面底部,封面没有 / 加载失败就放名称行下
  const [rateChartOn] = useCardRateChart()
  const [coverFailed, setCoverFailed] = useState(false)
  const coverShown = !!coverSrc && !coverFailed
  const sparkline =
    active && live && rateChartOn ? (
      <LiveRateChart
        id={streamer.id}
        windowMs={SPARK_RATE_WINDOW_MS}
        variant="sparkline"
        height={coverShown ? 34 : 28}
        label={`${remark} 写盘速率`}
        className={coverShown ? undefined : styles.spark}
        overlay={coverShown}
      />
    ) : null

  const plat = platformName(streamer.url)
  const platColor = PLAT_COLORS[plat] ?? DEFAULT_PLAT_COLOR
  const streamerName = (
    <div className={styles.nameWrap}>
      {avatarSrc ? <LiveAvatar key={avatarSrc} src={avatarSrc} /> : null}
      {broadcasterName ? <div className={styles.name} title={broadcasterName}>{broadcasterName}</div> : null}
    </div>
  )

  const statusTag = (
    <span className={`${styles.badge} ${styles[meta.cls]}`}>
      <span className={styles.bdot} />
      {meta.label}
    </span>
  )
  const platformTag = (
    <span className={styles.platChip} style={{ backgroundColor: platTint(platColor), color: platColor }} title={streamer.url}>
      <span className={styles.platName}>{plat}</span>
    </span>
  )

  const cardCls = [
    styles.card,
    live ? styles.rec : '',
    paused ? styles.paused : '',
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <article ref={ref} className={cardCls} data-id={streamer.id}>
      <div className={styles.coverFrame}>
        {coverSrc ? (
          <LiveCover
            src={coverSrc}
            alt={`${broadcasterName || remark} 的直播间封面`}
            platform={platformTag}
            title={info?.title}
            onPreview={previewable ? (e) => openLivePage(streamer.id, e) : undefined}
            overlay={coverShown ? sparkline : null}
            onStateChange={(s) => setCoverFailed(s === 'error')}
          />
        ) : null}
        <div className={coverSrc ? styles.coverHeader : styles.nameRow}>
          {streamerName}
          <span className={styles.coverStatus}>{statusTag}</span>
          {!coverSrc ? platformTag : null}
        </div>
      </div>

      {/* 录播备注与平台主播名独立；选择框放在备注后，点击不会触发直播预览。 */}
      <div className={styles.remarkRow}>
        <div className={styles.remark} title={remark}>{remark}</div>
        {selectable ? (
          <label className={styles.selection} title={`选择 ${remark}`}>
            <input
              type="checkbox"
              className={styles.chk}
              checked={!!selected}
              onChange={() => onToggleSelect?.(streamer.id)}
              aria-label={`选择 ${remark}`}
            />
          </label>
        ) : null}
      </div>

      {/* 开了码率图但没有封面可叠:sparkline 放这一行 */}
      {sparkline && !coverShown ? sparkline : null}

      <a
        className={styles.url}
        href={streamer.url}
        target="_blank"
        rel="noreferrer"
        title={streamer.url}
      >
        {streamer.url}
      </a>
      {uploadTag ? <div className={styles.meta}>{uploadTag}</div> : null}
      {live || lastRec || actions ? (
        <div className={styles.footer}>
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
              <MarkerCount count={streamer.marker_count} />
            </span>
          ) : lastRec ? (
            <span className={styles.recInfo}>{lastRec}</span>
          ) : null}
          {actions ? <div className={styles.actions}>{actions}</div> : null}
        </div>
      ) : null}
    </article>
  )
}
