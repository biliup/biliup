'use client'
import type React from 'react'
import type { Tag } from '@douyinfe/semi-ui'
import useSWR from 'swr'
import { API_BASE, fetcher, handleResponse } from './api-streamer'

/**
 * Fleet 控制面的告警与控制台汇总（只有控制面有这些接口，普通实例 404）。
 * 告警只放在控制面内存里：重启就清空，也不往外发通知。时间字段是 Unix 毫秒。
 */

export type AlertKind =
  | 'node_offline'
  | 'disk_low'
  | 'config_failed'
  | 'room_failed'
  | 'recording_error'
  | 'upload_failed'

/** 与后端 `Alert` 一致 */
export interface FleetAlert {
  id: number
  kind: AlertKind
  node_id: number
  node_name: string
  /** 控制面的房间 id；节点自己添加的直播间为 null */
  room_id: number | null
  room: string | null
  url: string | null
  message: string
  first_at: number
  last_at: number
  count: number
  /** 已恢复的时间；还在持续为 null。投稿失败不会自己恢复 */
  resolved_at: number | null
}

/** 与后端 `AlertList` 一致 */
export interface FleetAlerts {
  now: number
  alerts: FleetAlert[]
  /** 最多留多少条 */
  capacity: number
  /** 低于这个协议次版本的节点不上报录制出错、投稿失败 */
  events_since: number
}

/** 与后端 `FleetSummary` 一致：控制台的 Fleet 汇总 */
export interface FleetSummary {
  nodes_total: number
  nodes_online: number
  recording: number
  /** 还没「知道了」的告警 */
  alerts: number
  /** 其中还没恢复的 */
  alerts_open: number
}

export const FLEET_ALERTS_KEY = '/v1/fleet/alerts'
export const FLEET_SUMMARY_KEY = '/v1/fleet/summary'
/** 控制面每 5 秒核对一次状态类告警 */
export const FLEET_ALERTS_REFRESH_MS = 5000

type TagColor = React.ComponentProps<typeof Tag>['color']

export const ALERT_KINDS: Record<AlertKind, { label: string; color: TagColor; hint: string }> = {
  node_offline: { label: '节点离线', color: 'grey', hint: '超过 1 分钟没收到节点的任何消息' },
  disk_low: {
    label: '磁盘不足',
    color: 'orange',
    hint: '录制目录剩余空间低于节点配置的 min_free_space；没配置时按总容量的 5%',
  },
  config_failed: { label: '配置应用失败', color: 'red', hint: '节点没能应用控制面下发的配置' },
  room_failed: { label: '落地失败', color: 'red', hint: '节点没能按分派录这个房间' },
  recording_error: { label: '录制出错', color: 'red', hint: '节点上的录制异常结束；重新录上后算恢复' },
  upload_failed: { label: '投稿失败', color: 'red', hint: '节点上传或投稿失败（含投稿后处理出错）' },
}

/** 节点页的告警页签与节点卡片共用同一个 key，SWR 只发一次请求 */
export function useFleetAlerts(enabled: boolean) {
  return useSWR<FleetAlerts>(enabled ? FLEET_ALERTS_KEY : null, fetcher, {
    refreshInterval: FLEET_ALERTS_REFRESH_MS,
  })
}

export async function clearAlert(id: number): Promise<void> {
  await handleResponse(await fetch(`${API_BASE}${FLEET_ALERTS_KEY}/${id}`, { method: 'DELETE' }))
}

/** 返回清掉的条数 */
export async function clearAlerts(): Promise<number> {
  const res = await fetch(API_BASE + FLEET_ALERTS_KEY, { method: 'DELETE' })
  await handleResponse(res)
  const body = (await res.json()) as { cleared: number }
  return body.cleared
}
