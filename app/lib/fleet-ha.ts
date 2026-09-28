'use client'
import type React from 'react'
import type { Tag } from '@douyinfe/semi-ui'
import { send } from './use-fleet'
import type { FleetAlert } from './fleet-alerts'

/**
 * 一主一备（HA Pair）的接口。控制面上是 `/v1/fleet/ha*`，配对里的节点上是 `/v1/node/ha*`
 * （节点不在配对里时这组地址落回页面）。时间字段是 Unix 毫秒，参数里的时长是秒。
 */

export type Side = 'controller' | 'node'
export type HaMode = 1 | 2

/** 与后端 `HaParams` 一致 */
export interface HaParams {
  offline_grace: number
  standby_upload_delay: number
  upload_start_timeout: number
  upload_stall_timeout: number
  progress_interval: number
  manual_timeout: number
  delete_standby_copy: boolean
}

export const DEFAULT_PARAMS: HaParams = {
  offline_grace: 60,
  standby_upload_delay: 600,
  upload_start_timeout: 1800,
  upload_stall_timeout: 900,
  progress_interval: 60,
  manual_timeout: 86400,
  delete_standby_copy: false,
}

/** 与后端 `MAX_SECONDS` 一致：每个时长 1 秒到一周 */
export const MAX_SECONDS = 7 * 24 * 60 * 60

export type DurationKey = Exclude<keyof HaParams, 'delete_standby_copy'>

/** 各参数的一句话说明；`modes` 是它起作用的模式 */
export const PARAM_FIELDS: { key: DurationKey; label: string; hint: string; modes: HaMode[] }[] = [
  { key: 'offline_grace', label: '判主机离线', hint: '备机连不上主机超过这么久，算主机离线', modes: [1, 2] },
  {
    key: 'standby_upload_delay',
    label: '主机离线后备机等多久再投',
    hint: '算主机离线之后再等这么久，主机还没回来备机才投',
    modes: [1],
  },
  {
    key: 'upload_start_timeout',
    label: '主机迟迟不投',
    hint: '主机在线，但下播这么久还没开始投，备机投',
    modes: [1],
  },
  { key: 'upload_stall_timeout', label: '主机投到一半停住', hint: '主机的上传进度停住这么久，备机投', modes: [1] },
  {
    key: 'progress_interval',
    label: '主机报进度的间隔',
    hint: '主机投稿时隔这么久告诉备机一次进度，要比上一项短',
    modes: [1, 2],
  },
  {
    key: 'manual_timeout',
    label: '接手的一场等主机多久',
    hint: '备机接手录的那一场等主机回来先投它那半，等满这么久转为待人工处理',
    modes: [2],
  },
]

export const MODE_TEXT: Record<HaMode, { title: string; hint: string }> = {
  1: {
    title: '模式 1：两台都录',
    hint: '两台同时录；上传主机投稿，它投不成或停住时备机用自己那份补投。多占一份带宽与磁盘，最稳',
  },
  2: {
    title: '模式 2：主机录，备机接手',
    hint: '只有上传主机录，备机盯着开播；主机离线时备机接手录，主机回来先投它那半、备机再追加为分 P',
  },
}

/** 与后端 `Pair` 一致 */
export interface Pair {
  primary_node_id: number
  standby_node_id: number
  mode: HaMode
  params: HaParams
  /** 此刻上传的一台；为空是「本机」节点 */
  leader_node_id: number | null
  created_at: number
  updated_at: number
}

export interface PairSync {
  min_proto?: number
  linked?: boolean
  /** 排着还没送到对端的修改条数 */
  pending?: number
}

/** 与后端 `SessionRecord` 一致（主机那一侧记的场次） */
export interface SessionRecord {
  session_key: string
  room_id: number
  started_at: number
  ended_at: number | null
  primary_state: string
  /** 备机上报的状态（`ReportedState`） */
  standby_state: string | null
  uploader: 'primary' | 'standby' | null
  bvid: string | null
  upload_bytes: number
  progress_at: number | null
  reason: string | null
  updated_at: number
}

/** 备机手里的一场（`GET /v1/node/ha` 与控制面当备机时的 `local_standby`） */
export interface StandbySession {
  id: string
  key: string
  room: number
  url: string
  remark: string
  kind: 'backup' | 'takeover' | 'normal'
  state: string
  started_at: number
  ended_at: number | null
  takeover_of: string | null
  bvid: string | null
  reason: string | null
  since: number | null
  collected: boolean
  segments: number
}

export interface StandbyView {
  role: 'standby'
  leader: Side
  mode: HaMode
  params: HaParams
  /** 此刻上传主机的节点 id */
  primary: number
  rooms: number[]
  linked: boolean
  primary_offline: boolean
  sessions: StandbySession[]
}

export interface PrimaryView {
  role: 'primary'
  leader: 'node'
  mode: HaMode
  params: HaParams
  linked: boolean
  reported: boolean
  sessions: SessionRecord[]
}

/** `GET /v1/node/ha` */
export type NodeHa = (StandbyView | PrimaryView) & { sync?: PairSync }

export type HandbackStage = 'hold' | 'held' | 'released'
export type LocalActivity = 'recording' | 'uploading' | 'idle' | 'gone'

export interface HandbackEntry {
  url?: string
  template?: number
  stage: HandbackStage
  since?: number
  /** 点过「立即交还」 */
  force?: boolean
  /** 房间在「本机」上此刻在做什么（模板没有） */
  local?: LocalActivity
}

export interface Returning {
  primary: number
  rooms?: Record<string, HandbackEntry>
  templates?: Record<string, HandbackEntry>
  /** 那台节点此刻在不在线 */
  online?: boolean
}

export interface ExcludedRoom {
  id: number
  url: string
  remark: string
  reason: string
}

/** `GET /v1/fleet/ha` */
export interface FleetHa {
  pair: Pair | null
  active: boolean
  local_node: number | null
  leader: Side | null
  local_role: 'primary' | 'standby' | null
  busy: string | null
  standby: { id: number; online: boolean; proto: number | null; linked: boolean; reported: boolean } | null
  min_proto: number
  sync: PairSync
  excluded: ExcludedRoom[]
  sessions: SessionRecord[]
  local_standby: StandbyView | null
  /** 解除配对后还没交还完的：节点 id → 那些行 */
  handback?: Record<string, Returning>
  /** 配对生效时：解除后会交还给备机的 Fleet 房间与模板 */
  returns?: { rooms: number[]; templates: number[] }
}

/** 一台上还没纳入配对的本地主播或模板（与后端 `LocalRow` 一致） */
export interface LocalRow {
  id: number
  name: string
  url?: string
  template?: number
  hooks: boolean
  busy: boolean
  state: 'local' | 'waiting' | 'joining'
  reason?: string
  refused?: string
  included?: boolean
}

export interface RowsView {
  node: number | null
  streamers?: LocalRow[]
  templates?: LocalRow[]
  error?: string
}

export interface Candidates {
  paired: boolean
  standby: RowsView | null
  primary: RowsView | null
}

export const FLEET_HA_KEY = '/v1/fleet/ha'
export const NODE_HA_KEY = '/v1/node/ha'
export const NODE_HA_CANDIDATES_KEY = '/v1/node/ha/candidates'
/** 与后端 `HA_SINCE` / `PAIR_SINCE` 一致：当备机、双向同步各要的最低协议次版本 */
export const HA_SINCE = 4
export const PAIR_SINCE = 5

export function candidatesKey(standby: number | null) {
  return standby === null ? '/v1/fleet/ha/candidates' : `/v1/fleet/ha/candidates?standby=${standby}`
}

export interface Selection {
  streamers?: number[]
  templates?: number[]
}

export function designate(body: { standby: number; mode: HaMode; params?: HaParams; adopt?: Selection }) {
  return send<{ pair: Pair; adoption: RowsView | null }>('PUT', FLEET_HA_KEY, body)
}

export function dissolve() {
  return send<{ dissolved: boolean }>('DELETE', FLEET_HA_KEY)
}

/** 控制面上把一台的本地行加进配对；节点上用 `joinOnNode` */
export function joinPair(side: Side, selection: Required<Selection>) {
  return send<RowsView>('POST', `${FLEET_HA_KEY}/join`, { side, ...selection })
}

export function joinOnNode(selection: Required<Selection>) {
  return send<RowsView>('POST', `${NODE_HA_KEY}/join`, selection)
}

/** `onNode`：在配对节点上操作（经控制面提交） */
export function switchLeader(primary: Side, onNode: boolean) {
  return send<unknown>('POST', `${onNode ? NODE_HA_KEY : FLEET_HA_KEY}/role`, { primary })
}

export function configureOnNode(mode: HaMode, params: HaParams) {
  return send<unknown>('PUT', NODE_HA_KEY, { mode, params })
}

export type ManualAction = 'standby-upload' | 'drop'

export function manual(key: string, action: ManualAction, onNode: boolean) {
  return send<unknown>(
    'POST',
    `${onNode ? NODE_HA_KEY : FLEET_HA_KEY}/sessions/${encodeURIComponent(key)}/${action}`,
  )
}

/** 秒 → 「10 分钟」「1 天」这样的说法 */
export function humanSeconds(seconds: number): string {
  if (seconds % 86400 === 0) return `${seconds / 86400} 天`
  if (seconds % 3600 === 0) return `${seconds / 3600} 小时`
  if (seconds % 60 === 0) return `${seconds / 60} 分钟`
  return `${seconds} 秒`
}

/** 与后端 `HaParams::validate` 一致；合法时返回 null */
export function paramsIssue(params: HaParams): string | null {
  for (const field of PARAM_FIELDS) {
    const value = params[field.key]
    if (!Number.isInteger(value) || value < 1 || value > MAX_SECONDS) {
      return `「${field.label}」要在 1 秒到 ${humanSeconds(MAX_SECONDS)}之间`
    }
  }
  if (params.progress_interval >= params.upload_stall_timeout) {
    return '「主机报进度的间隔」要比「主机投到一半停住」短'
  }
  return null
}

const STANDBY_STATES: Record<string, string> = {
  recording: '备机在录',
  holding: '备机录完，等主机的结果',
  awaiting_primary: '备机录完，等主机回来先投它那半',
  uploading: '备机在投',
  appending: '备机在追加分 P',
  uploaded: '备机投成',
  appended: '备机追加成',
  done: '主机投成，备机这份不用投',
  manual: '待人工处理',
  dropped: '已放弃',
  failed: '备机投失败',
  empty: '备机没有可投的文件',
  standby: '备机只盯着开播',
}

const PRIMARY_STATES: Record<string, string> = {
  none: '主机没有这一场',
  recording: '主机在录',
  recorded: '主机录完，还没开始投',
  uploading: '主机在投',
  uploaded: '主机投成',
  failed: '主机投稿失败',
  skipped: '主机没投（被过滤或没有文件）',
  interrupted: '主机在录或投的途中退出了',
  handed_over: '交给备机负责',
}

export function standbyStateText(state: string | null): string {
  return state ? (STANDBY_STATES[state] ?? state) : '备机没有这一场'
}

export function primaryStateText(state: string): string {
  return PRIMARY_STATES[state] ?? state
}

/** 备机这一场要人工决定（与后端 `manual` 能处理的状态一致，但只列真正卡住的两种） */
export function needsHuman(state: string | null): boolean {
  return state === 'manual' || state === 'failed'
}

/** 交还中一行此刻卡在哪 */
export function handbackBlocker(entry: HandbackEntry, online: boolean | undefined): string {
  if (entry.stage === 'released') return '已交给备机，等它确认'
  if (online === false) return '备机离线：等它回来再交还'
  if (entry.stage === 'hold') return '等备机确认挡住开录'
  switch (entry.local) {
    case 'recording':
      return entry.force ? '本机在录：录完就交还' : '本机在录：录完、投完再交还'
    case 'uploading':
      return entry.force ? '本机还在投，这时已可交接' : '本机在投：投完再交还'
    case 'gone':
      return '本机上已经没有这一行'
    case 'idle':
      return '本机空闲，马上交还'
    default:
      return '等本机这个直播间空闲（没在录、没在投）再交还'
  }
}

type TagColor = React.ComponentProps<typeof Tag>['color']

export type HaAlertKind = 'manual' | 'standby_failed' | 'standby_offline' | 'missing_account'

/** 一主一备的四类告警（沿用 F4 告警，按内容认出来换成这里的说法） */
export const HA_ALERT_KINDS: Record<HaAlertKind, { label: string; color: TagColor; hint: string }> = {
  manual: {
    label: '待人工处理',
    color: 'orange',
    hint: '主机那份投不成，或等不到主机回来：备机这份等你决定投还是放弃',
  },
  standby_failed: {
    label: '备机投稿失败',
    color: 'red',
    hint: '备机自己投这一场失败了；可以在「待人工处理」里让它再投，或者放弃',
  },
  standby_offline: {
    label: '备机离线',
    color: 'grey',
    hint: '主机照常录、照常投；模式 1 少一份备录，模式 2 这时没人接手',
  },
  missing_account: {
    label: '缺 B 站账号',
    color: 'red',
    hint: '这台没有投稿模板要用的账号，轮到它投时会失败；在任一台登录这个账号，会同步到另一台',
  },
}

/** 这条 F4 告警是不是一主一备的，是哪一类 */
export function haAlertKind(alert: FleetAlert, pair: Pair): HaAlertKind | null {
  const inPair = alert.node_id === pair.primary_node_id || alert.node_id === pair.standby_node_id
  if (!inPair) return null
  if (alert.kind === 'upload_failed' && alert.message.startsWith('一主一备：这一场待人工处理')) return 'manual'
  if (alert.kind === 'upload_failed' && alert.message.startsWith('一主一备：备机投这一场失败')) return 'standby_failed'
  if (alert.kind === 'node_offline' && alert.node_id === pair.standby_node_id) return 'standby_offline'
  if (alert.kind === 'room_failed' && alert.message.includes('没有登记 B 站账号')) return 'missing_account'
  return null
}
