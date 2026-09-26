'use client'
import { API_BASE, handleResponse } from './api-streamer'
import type { SystemCpu, SystemDisk, SystemMemory, SystemSample } from './use-system-stats'
import type { NodeConfigState } from './fleet-config'

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

export interface ToolStatus {
  available: boolean
  version: string | null
}

/** 节点上报的 B 站账号：只有 mid 与昵称，凭据文件不出节点 */
export interface FleetAccount {
  mid: number
  uname: string
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
  /** 在线时的协议次版本号；低于 1 的节点收不了房间 */
  proto: number | null
  tools: { ffmpeg: ToolStatus } | null
  /** 分派给它的房间数（含迁移中、还没交给它的） */
  assigned_rooms: number
  accounts: FleetAccount[]
  /** 最近一次下发的房间与模板它是否已经确认；离线为 null */
  synced: boolean | null
  config: NodeConfigState
  /** 正在「移除并自动改派」，等它确认释放房间 */
  removing: boolean
}

/**
 * 一个房间在被移除节点上的释放情况（与后端 `Release` 一致）：`released` 为节点确认释放后新节点才接手；
 * `offline` / `timeout` 为没等到确认就移除了，节点发现被移除后会暂停这个房间
 */
export type RoomRelease = 'waiting' | 'released' | 'offline' | 'timeout'

export interface RemovedRoom {
  room_id: number
  remark: string
  /** 改派到的节点；null 为没找到合适的节点、留在未分派，原因在 `unplaced` */
  node_id: number | null
  unplaced: string | null
  release: RoomRelease
}

/** 与后端 `Removal` 一致：一次「移除并自动改派」的进度与结果，结束后留 10 分钟 */
export interface Removal {
  node_id: number
  node_name: string
  state: 'removing' | 'done'
  started_at: number
  /** 最晚这时移除 */
  deadline: number
  finished_at: number | null
  rooms: RemovedRoom[]
}

export interface FleetNodes {
  now: number
  controller: string
  controller_version: string
  controller_proto: number
  nodes: FleetNode[]
  removals: Removal[]
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
  /** 内嵌 relay 的端口；用外部 relay（--relay-listen off）时为 null */
  relay_port: number | null
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

/** 与后端 `RoomStatus` 一致 */
export type RoomStatus =
  | 'unassigned'
  | 'releasing'
  | 'deleting'
  | 'offline'
  | 'outdated'
  | 'syncing'
  | 'failed'
  | 'monitoring'
  | 'recording'
  | 'paused'

/** 房间的录制设置，与本地直播间同形（`RoomSpec`） */
export interface RoomSpec {
  url: string
  remark: string
  filename_prefix?: string | null
  time_range?: string | null
  format?: string | null
  override?: Record<string, unknown> | null
  preprocessor?: unknown[] | null
  segment_processor?: unknown[] | null
  downloaded_processor?: unknown[] | null
  postprocessor?: unknown[] | null
  opt_args?: string[] | null
  excluded_keywords?: string[] | null
}

/** 与后端 `RoomView` 一致 */
export interface FleetRoom extends RoomSpec {
  id: number
  template_id: number | null
  node_id: number | null
  epoch: number
  paused: boolean
  /** 还没确认释放的上一台；非空时迁移（或删除）还在进行 */
  releasing_node_id: number | null
  deleted_at: number | null
  created_at: number
  updated_at: number
  status: RoomStatus
  error: string | null
  releasing_online: boolean | null
}

export interface FleetRooms {
  now: number
  rooms: FleetRoom[]
}

/** 与后端 `Template` 一致：本地投稿模板的形状，`user_cookie` 换成了 `account_mid` */
export interface FleetTemplate {
  id: number
  template_name: string
  title: string | null
  tid: number | null
  tid_v2: number | null
  copyright: number | null
  copyright_source: string | null
  cover_path: string | null
  description: string | null
  dynamic: string | null
  dtime: number | null
  dolby: number | null
  hires: number | null
  charging_pay: number | null
  no_reprint: number | null
  is_only_self: number | null
  uploader: string | null
  account_mid: number | null
  tags: string[]
  credits: { username: string; uid: number }[] | null
  up_selection_reply: number | null
  up_close_reply: number | null
  up_close_danmu: number | null
  extra_fields: string | null
  created_at: number
  updated_at: number
}

export type TemplateInput = Omit<FleetTemplate, 'id' | 'created_at' | 'updated_at'>

/** `GET /v1/fleet/accounts`：各节点登记的 B 站账号 */
export interface NodeAccount {
  node_id: number
  mid: number
  uname: string
  reported_at: number
}

export const FLEET_NODES_KEY = '/v1/fleet/nodes'
export const FLEET_TOKENS_KEY = '/v1/fleet/join-tokens'
export const FLEET_ROOMS_KEY = '/v1/fleet/rooms'
export const FLEET_TEMPLATES_KEY = '/v1/fleet/templates'
export const FLEET_ACCOUNTS_KEY = '/v1/fleet/accounts'
/** 节点每 10 秒一次心跳，列表 5 秒拉一次足够 */
export const FLEET_REFRESH_MS = 5000

/** `extraRelays`：额外写进票据的 relay 地址（候选地址）；`--relay-url` 仍排在它们前面 */
export async function issueTicket(extraRelays: string[] = []): Promise<IssuedTicket> {
  const res = await fetch(API_BASE + FLEET_TOKENS_KEY, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(extraRelays.length ? { extra_relays: extraRelays } : {}),
  })
  await handleResponse(res)
  return res.json()
}

export async function revokeNode(id: number): Promise<void> {
  await handleResponse(await fetch(`${API_BASE}${FLEET_NODES_KEY}/${id}`, { method: 'DELETE' }))
}

/** 与后端 `REMOVAL_WAIT` 一致：在线节点确认释放房间最多等这么久 */
export const REMOVAL_WAIT_SECONDS = 60

/**
 * 移除节点，并把它的房间按负载改派到其他节点。在线节点先迁移、等它确认释放再移除，
 * 返回 `state: 'removing'`，结果随后出现在节点列表的 `removals` 里；离线节点当场移除，返回 `done`
 */
export async function revokeAndReassign(id: number): Promise<Removal> {
  const res = await fetch(`${API_BASE}${FLEET_NODES_KEY}/${id}?reassign=auto`, { method: 'DELETE' })
  await handleResponse(res)
  return res.json()
}

export async function voidToken(id: string): Promise<void> {
  await handleResponse(await fetch(`${API_BASE}${FLEET_TOKENS_KEY}/${encodeURIComponent(id)}`, { method: 'DELETE' }))
}

async function send<T>(method: string, path: string, body?: unknown): Promise<T | null> {
  const res = await fetch(API_BASE + path, {
    method,
    headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  await handleResponse(res)
  return res.status === 204 ? null : res.json()
}

export type RoomInput = RoomSpec & { template_id: number | null }

/** `auto_node`：由控制面按负载选节点，此时 `node_id` 必须为 null */
export function createRoom(room: RoomInput & { node_id: number | null; paused?: boolean; auto_node?: boolean }) {
  return send<FleetRoom>('POST', FLEET_ROOMS_KEY, room)
}

export function updateRoom(id: number, room: RoomInput) {
  return send<FleetRoom>('PUT', `${FLEET_ROOMS_KEY}/${id}`, room)
}

/** `nodeId` 为 null 是取消分派。`force` 时不等上一台确认释放 */
export function assignRoom(id: number, nodeId: number | null, force = false) {
  return send<FleetRoom>('POST', `${FLEET_ROOMS_KEY}/${id}/assign`, { node_id: nodeId, force })
}

/** 不再等上一台确认释放；返回 null 表示房间已删除（它本来在等删除） */
export function forceRelease(id: number) {
  return send<FleetRoom>('POST', `${FLEET_ROOMS_KEY}/${id}/force`)
}

export function pauseRoom(id: number, paused: boolean) {
  return send<FleetRoom>('POST', `${FLEET_ROOMS_KEY}/${id}/pause`, { paused })
}

/** 返回 null 表示已删除；返回房间表示还在等节点释放 */
export function deleteRoom(id: number, force = false) {
  return send<FleetRoom>('DELETE', `${FLEET_ROOMS_KEY}/${id}${force ? '?force=true' : ''}`)
}

export function createTemplate(template: TemplateInput) {
  return send<FleetTemplate>('POST', FLEET_TEMPLATES_KEY, template)
}

export function updateTemplate(id: number, template: TemplateInput) {
  return send<FleetTemplate>('PUT', `${FLEET_TEMPLATES_KEY}/${id}`, template)
}

export function deleteTemplate(id: number) {
  return send<null>('DELETE', `${FLEET_TEMPLATES_KEY}/${id}`)
}

/** 与后端 `RoomSpec::has_hooks` 一致：处理器里有 `run` 步骤（rm、mv、remux 等文件操作和 override 都不算） */
export function roomHasHooks(room: RoomSpec): boolean {
  const steps = [room.preprocessor, room.segment_processor, room.downloaded_processor, room.postprocessor]
  return steps.some((list) => (list ?? []).some((step) => typeof step === 'object' && step !== null && 'run' in step))
}

/** 节点是否版本太旧、收不了房间（离线时不知道版本，不算） */
export function nodeOutdated(node: FleetNode): boolean {
  return node.online && (node.proto ?? 0) < 1
}

/**
 * 房间放到这台节点上违反哪条硬约束（与后端 `check_target` 一致）；能放返回 null。
 * 只是提前提示，最终以后端为准。
 */
export function placementIssue(node: FleetNode, room: RoomSpec, template: FleetTemplate | undefined): string | null {
  if (nodeOutdated(node)) return '版本太旧，收不了房间'
  if (roomHasHooks(room) && !node.allow_hooks) return '没带 --allow-hooks，不能放带 run 命令的房间'
  if (template?.account_mid && !node.accounts.some((a) => a.mid === template.account_mid)) {
    return `没有登记模板要用的 B 站账号 ${template.account_mid}`
  }
  return null
}

/** 服务端错误是 `{"message": ...}` 的 JSON 文本，取出其中的 message */
export function errorMessage(e: unknown): string {
  const raw = (e as Error | undefined)?.message ?? String(e)
  try {
    const parsed = JSON.parse(raw)
    return typeof parsed?.message === 'string' ? parsed.message : raw
  } catch {
    return raw
  }
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
