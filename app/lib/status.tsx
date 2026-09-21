import React from 'react'
import { Tag } from '@douyinfe/semi-ui'

/**
 * 状态视觉体系(全局唯一事实来源)。
 * 后端 status / upload_status 均为 WorkerStatus 枚举的 Debug 字符串,
 * 常见取值:Working / Pending / Idle / Pause / OutOfSchedule / TitleExcluded / ""(空)。
 */

export const LIVE_STATUS = 'Working'
export const PAUSE_STATUS = 'Pause'
export const PENDING_STATUS = 'Pending'
/** 录制策略排期外(recording_policy.rs::Rejection::OutOfTimeRange) */
export const OUT_OF_SCHEDULE = 'OutOfSchedule'
/** 标题命中排除关键词(recording_policy.rs::Rejection::ExcludedKeyword) */
export const TITLE_EXCLUDED = 'TitleExcluded'

/** 统一后端版本号展示，避免版本字段自带 v 时渲染成 vv1.2.2。 */
export function formatVersion(version?: string): string | undefined {
  return typeof version === 'string' ? version.replace(/^v/i, '') : undefined
}

export function isLiveStatus(status?: string): boolean {
  return status === LIVE_STATUS
}

export function isPaused(status?: string): boolean {
  return status === PAUSE_STATUS
}

/** 直播状态 → 徽章视觉配置 */
export interface StatusVisual {
  label: string
  /** 样式类:badgeLive(绿,录制中)/ badgePause(橙,人为或规则暂停)/ badgeIdle(灰,其余) */
  cls: 'badgeLive' | 'badgePause' | 'badgeIdle'
}

/**
 * 与 master 上原 streamers 页的 switch 保持同一组文案:
 * Working 直播中 / Pending 检测中 / Idle 空闲(未开播) / OutOfSchedule 非录播时间 /
 * TitleExcluded 标题已排除 / Pause 暂停中。绿色只给真正在录的 Working。
 */
export function streamerStatusMeta(status?: string): StatusVisual {
  if (isLiveStatus(status)) return { label: '直播中', cls: 'badgeLive' }
  if (status === PENDING_STATUS) return { label: '检测中', cls: 'badgeIdle' }
  if (status === OUT_OF_SCHEDULE) return { label: '非录播时间', cls: 'badgeIdle' }
  if (status === TITLE_EXCLUDED) return { label: '标题已排除', cls: 'badgePause' }
  if (isPaused(status)) return { label: '已暂停', cls: 'badgePause' }
  return { label: '未开播', cls: 'badgeIdle' }
}

/**
 * 上传状态 → 独立的 Tag(与直播状态是两个维度,保留)。
 * 重要:后端 upload_status 是 WorkerStatus 枚举的 Debug 字符串,
 * 真实取值只有 Working / Pending / Idle / Pause / ""(空),没有 Failed / Uploaded。
 * 上传失败不会体现在这个字段里。
 */
export function uploadStatusTag(uploadStatus?: string): React.ReactNode {
  // 后端 upload_status 是 WorkerStatus 枚举:Working / Pending / Idle / Pause / ""(空)。
  // Idle / 空 表示没有上传活动,不是"上传完成";不配置上传时就是这个状态。
  // 因此只有真正有上传任务时才显示 Tag,避免没配上传却显示"待上传"/"上传完成"。
  const map: Record<string, { text: string; bg: string; color: string }> = {
    Working: { text: '上传中', bg: 'rgba(var(--semi-blue-4), 1)', color: '#fff' },
    Pending: { text: '待上传', bg: 'rgba(var(--semi-grey-3), 1)', color: 'var(--semi-color-text-0)' },
    Pause: { text: '上传暂停', bg: 'rgba(var(--semi-yellow-4), 1)', color: 'var(--semi-color-text-0)' },
  }
  const cfg = map[uploadStatus ?? '']
  if (!cfg) return null
  return (
    <Tag
      size="small"
      style={{ backgroundColor: cfg.bg, color: cfg.color, border: 'none', fontWeight: 500 }}
    >
      {cfg.text}
    </Tag>
  )
}

/** 直播状态 → Tag(直播管理页列表视图用),颜色与卡片徽章同一套 */
export function streamerStatusTag(status?: string): React.ReactNode {
  const m = streamerStatusMeta(status)
  const palette: Record<StatusVisual['cls'], { bg: string; color: string }> = {
    badgeLive: { bg: 'rgba(var(--semi-green-4), 0.13)', color: 'rgb(var(--semi-green-5))' },
    badgePause: { bg: 'rgba(var(--semi-orange-4), 0.14)', color: 'rgb(var(--semi-orange-5))' },
    badgeIdle: { bg: 'var(--semi-color-fill-1)', color: 'var(--semi-color-text-1)' },
  }
  const c = palette[m.cls]
  return (
    <Tag
      size="small"
      style={{ backgroundColor: c.bg, color: c.color, border: 'none', fontWeight: 600 }}
    >
      {m.label}
    </Tag>
  )
}

/** 直播地址 → 平台中文名(统一识别逻辑,避免各页面各写一份) */
export function platformName(url?: string): string {
  if (!url) return '直播源'
  const lower = url.toLowerCase()
  if (lower.includes('bilibili.com') || lower.includes('b23.tv')) return '哔哩哔哩'
  if (lower.includes('huya.com')) return '虎牙直播'
  if (lower.includes('douyu.com')) return '斗鱼直播'
  if (lower.includes('youtube.com') || lower.includes('youtu.be')) return 'YouTube'
  if (lower.includes('twitch.tv')) return 'Twitch'
  if (lower.includes('cc.163.com')) return 'CC直播'
  if (lower.includes('kuaishou.com')) return '快手直播'
  if (lower.includes('douyin.com')) return '抖音直播'
  try {
    return new URL(url).hostname || '直播源'
  } catch {
    return '直播源'
  }
}
