import React from 'react'
import { Tag } from '@douyinfe/semi-ui'

/**
 * 状态视觉体系(全局唯一事实来源)。
 * 后端 status / upload_status 均为 WorkerStatus 枚举的 Debug 字符串,
 * 已验证取值只有 Working / Pending / Idle / Pause / ""(空)。
 */

export const LIVE_STATUS = 'Working'
export const PAUSE_STATUS = 'Pause'

export function isLiveStatus(status?: string): boolean {
  return status === LIVE_STATUS
}

export function isPaused(status?: string): boolean {
  return status === PAUSE_STATUS
}

/** 直播状态 → 徽章视觉配置(2 色 + 暂停态) */
export interface StatusVisual {
  label: string
  /** 挂到样式类:badge-live / badge-idle / badge-pause */
  cls: string
}

export function streamerStatusMeta(status?: string): StatusVisual {
  if (isLiveStatus(status)) return { label: '直播中', cls: 'badgeLive' }
  if (isPaused(status)) return { label: '已暂停', cls: 'badgePause' }
  return { label: '未开播', cls: 'badgeIdle' }
}

/** 兼容旧调用方:返回纯色元数据(主页任务行 / 卡片共用) */
export function statusMeta(status?: string): { text: string; color: string; cls: string } {
  const m = streamerStatusMeta(status)
  return {
    text: m.label,
    color: m.cls === 'badgeLive' ? 'rgb(var(--semi-green-5))' : m.cls === 'badgePause' ? 'rgb(var(--semi-orange-5))' : 'rgb(var(--semi-red-5))',
    cls: m.cls === 'badgeLive' ? 'dotLive' : m.cls === 'badgePause' ? 'dotPause' : 'dotOffline',
  }
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

/**
 * 平台标签:低饱和中性色,不要抢状态标签的戏。
 */
export function platformTag(url?: string): React.ReactNode {
  return (
    <Tag
      size="small"
      style={{
        backgroundColor: 'var(--semi-color-fill-1)',
        color: 'var(--semi-color-text-2)',
        border: '1px solid var(--semi-color-border)',
      }}
    >
      {platformName(url)}
    </Tag>
  )
}

/**
 * 直播状态 → Tag(2 色版,备用)。
 */
export function streamerStatusTag(status?: string): React.ReactNode {
  const m = streamerStatusMeta(status)
  const live = m.cls === 'badgeLive'
  const pause = m.cls === 'badgePause'
  return (
    <Tag
      size="small"
      style={{
        backgroundColor: live
          ? 'rgba(var(--semi-green-4), 1)'
          : pause
            ? 'rgba(var(--semi-orange-4), 1)'
            : 'rgba(var(--semi-red-4), 1)',
        color: '#fff',
        border: 'none',
        fontWeight: 500,
      }}
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
