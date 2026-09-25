'use client'
import { API_BASE, handleResponse } from './api-streamer'
import type { SystemCpu, SystemDisk, SystemMemory, SystemSample } from './use-system-stats'

/**
 * Fleet 控制面的接口（只有以 `--controller` 启动的实例才有，普通实例全部 404）。
 * 时间字段一律是 Unix 毫秒；节点的采样时刻用的是节点自己的时钟。
 */

export interface PoolUsage {
  capacity: number
  occupied: number
}

/** 与后端 `Summary` 一致：在线时是最新心跳，离线时是最后落盘的那一份 */
export interface NodeSummary {
  pools: { download: PoolUsage; upload: PoolUsage }
  rooms: number
  recording: number
  cpu: SystemCpu | null
  memory: SystemMemory | null
  disk: SystemDisk | null
  interfaces: string[]
}

/** 与后端 `NodeView` 一致 */
export interface FleetNode {
  id: number
  name: string
  endpoint_id: string
  labels: string[]
  allow_hooks: boolean
  created_at: number
  last_seen_at: number | null
  version: string | null
  online: boolean
  connected_at: number | null
  path: 'relay' | 'direct' | null
  summary: NodeSummary | null
  interval_ms: number | null
  samples: SystemSample[]
}

export interface FleetNodes {
  now: number
  controller: string
  nodes: FleetNode[]
}

/** `POST /v1/fleet/join-tokens` 的响应；票据只在这一次返回 */
export interface IssuedTicket {
  id: string
  created_at: number
  expires_at: number
  ticket: string
  command: string
  docker_command: string
  docker_env: string
  relays: string[]
  private_only: boolean
}

export interface JoinToken {
  id: string
  created_by: number | null
  created_at: number
  expires_at: number
  used_at: number | null
  used_by_node: number | null
  state: 'active' | 'used' | 'expired'
}

export interface JoinTokens {
  now: number
  tokens: JoinToken[]
}

export const FLEET_NODES_KEY = '/v1/fleet/nodes'
export const FLEET_TOKENS_KEY = '/v1/fleet/join-tokens'
/** 节点每 10 秒一次心跳，列表 5 秒拉一次足够 */
export const FLEET_REFRESH_MS = 5000

export async function issueTicket(): Promise<IssuedTicket> {
  const res = await fetch(API_BASE + FLEET_TOKENS_KEY, { method: 'POST' })
  await handleResponse(res)
  return res.json()
}

export async function revokeNode(id: number): Promise<void> {
  await handleResponse(await fetch(`${API_BASE}${FLEET_NODES_KEY}/${id}`, { method: 'DELETE' }))
}

export async function voidToken(id: string): Promise<void> {
  await handleResponse(await fetch(`${API_BASE}${FLEET_TOKENS_KEY}/${encodeURIComponent(id)}`, { method: 'DELETE' }))
}

/**
 * 复制到剪贴板。通过局域网 IP 用 http 打开时不是安全上下文，`navigator.clipboard` 不存在，
 * 退回到选中隐藏文本框再 execCommand。
 */
export async function copyText(text: string): Promise<boolean> {
  if (typeof window !== 'undefined' && window.isSecureContext && navigator.clipboard) {
    try {
      await navigator.clipboard.writeText(text)
      return true
    } catch {
      // 权限被拒时走下面的退路
    }
  }
  const area = document.createElement('textarea')
  area.value = text
  area.setAttribute('readonly', '')
  area.style.position = 'fixed'
  area.style.top = '-1000px'
  document.body.appendChild(area)
  area.select()
  let ok = false
  try {
    ok = document.execCommand('copy')
  } catch {
    ok = false
  }
  document.body.removeChild(area)
  return ok
}
