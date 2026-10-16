import { apiFetch, handleResponse } from './api-streamer'

export type UploadSource = 'automatic' | 'manual' | 'clip' | 'sync' | 'ha' | 'ha_append' | 'ha_resume'
export type UploadState = 'queued' | 'running' | 'retrying' | 'submitting' | 'succeeded' | 'partial' | 'failed' | 'cancelled' | 'interrupted' | 'unknown' | 'paused' | 'skipped'

export interface UploadQueueFile {
  id: string
  order: number
  name: string
  state: UploadState | string
  bytes_confirmed: number
  total_bytes: number | null
  retries: number
  error: string | null
}

export interface UploadQueueTask {
  id: string
  source: UploadSource | string
  scheduler_group: string
  order: number
  queue_position: number | null
  title: string
  streamer_name: string | null
  session_id: number | null
  template_name: string | null
  state: UploadState | string
  phase: string | null
  current_file: string | null
  bytes_confirmed: number
  total_bytes: number | null
  speed_bytes_per_sec: number
  files_succeeded: number
  files_failed: number
  retries: number
  next_retry_at: number | null
  error: string | null
  bvid: string | null
  created_at: number
  updated_at: number
  files?: UploadQueueFile[]
}

export interface UploadQueueSummary {
  uploading: number
  queued: number
  attention: number
  speed_bytes_per_sec: number
}

export interface UploadQueuePage {
  tasks: UploadQueueTask[]
  total: number
  summary: UploadQueueSummary
  error?: string | null
}

export interface UploadQueueFilters {
  source?: string
  state?: string
  limit: number
  offset: number
}

export const uploadSourceLabels: Record<string, string> = {
  automatic: '自动投稿', manual: '手动投稿', clip: '切片发布', sync: '边录边传',
  ha: 'HA 投稿', ha_append: 'HA 追加投稿', ha_resume: 'HA 补投',
}

export const uploadStateMeta: Record<string, { label: string; tone: 'blue' | 'green' | 'orange' | 'red' | 'grey' }> = {
  queued: { label: '等待中', tone: 'grey' }, running: { label: '上传中', tone: 'blue' },
  retrying: { label: '重试中', tone: 'orange' }, submitting: { label: '提交稿件', tone: 'blue' },
  succeeded: { label: '成功', tone: 'green' }, partial: { label: '部分成功', tone: 'orange' },
  failed: { label: '失败', tone: 'red' }, cancelled: { label: '已取消', tone: 'grey' },
  interrupted: { label: '已中断', tone: 'orange' }, unknown: { label: '结果未知', tone: 'orange' },
  paused: { label: '已暂停', tone: 'grey' }, skipped: { label: '未投稿', tone: 'grey' },
}

export const uploadPhaseLabels: Record<string, string> = {
  waiting_slot: '等待上传位置', waiting_segment: '等待新分段', processing: '处理中',
  initializing: '初始化', uploading: '上传中', retrying: '重试中', submitting: '提交稿件', finished: '已结束',
}

export function uploadStateLabel(state: string | null | undefined): string {
  return state ? uploadStateMeta[state]?.label ?? '其他状态' : '未知状态'
}

export function uploadPhaseLabel(phase: string | null | undefined): string {
  if (!phase) return '处理中'
  return uploadPhaseLabels[phase] ?? '处理中'
}

const groupLabels: Record<string, string> = {
  'automatic-pool': '自动投稿队列', 'clip-publish': '切片发布队列',
  'manual': '手动投稿', 'sync': '边录边传', 'ha-pool': 'HA 投稿队列',
}

export function uploadGroupLabel(group: string) {
  return groupLabels[group] ?? '其他任务'
}

/** Tasks are ordered only within their real scheduler group; queue_position is shown only when supplied by the scheduler. */
export function groupUploadTasks(tasks: UploadQueueTask[]) {
  const groups = new Map<string, UploadQueueTask[]>()
  for (const task of tasks) groups.set(task.scheduler_group || 'other', [...(groups.get(task.scheduler_group || 'other') ?? []), task])
  return [...groups.entries()]
    .map(([key, items]) => ({
      key,
      label: uploadGroupLabel(key),
      tasks: items.sort((a, b) => a.order - b.order || a.created_at - b.created_at || a.id.localeCompare(b.id)),
    }))
    .sort((a, b) => a.label.localeCompare(b.label, 'zh-CN'))
}

export function formatUploadBytes(bytes: number | null | undefined): string {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) return '—'
  if (bytes < 1000) return `${Math.round(bytes)} B`
  const units = ['kB', 'MB', 'GB', 'TB']
  let value = bytes / 1000
  let index = 0
  while (value >= 1000 && index < units.length - 1) { value /= 1000; index++ }
  return `${value.toFixed(value >= 100 ? 0 : value >= 10 ? 1 : 2)} ${units[index]}`
}

export function formatUploadSpeed(bytesPerSecond: number | null | undefined): string {
  if (bytesPerSecond == null || !Number.isFinite(bytesPerSecond) || bytesPerSecond <= 0) return '0 B/s'
  return `${formatUploadBytes(bytesPerSecond)}/s`
}

export function uploadPercent(confirmed: number, total: number | null): number | null {
  if (total == null || total <= 0 || !Number.isFinite(total)) return null
  return Math.max(0, Math.min(100, (confirmed / total) * 100))
}

export function uploadTaskProgress(task: Pick<UploadQueueTask, 'bytes_confirmed' | 'total_bytes'>): string {
  const percent = uploadPercent(task.bytes_confirmed, task.total_bytes)
  return percent == null
    ? `${formatUploadBytes(task.bytes_confirmed)} / 总量未知`
    : `${formatUploadBytes(task.bytes_confirmed)} / ${formatUploadBytes(task.total_bytes)}（${Math.round(percent)}%）`
}

export async function fetchUploadQueue(filters: UploadQueueFilters): Promise<UploadQueuePage> {
  const query = new URLSearchParams()
  if (filters.source) query.set('source', filters.source)
  if (filters.state) query.set('state', filters.state)
  query.set('limit', String(filters.limit))
  query.set('offset', String(filters.offset))
  const response = await apiFetch(`/v1/upload-queue?${query}`)
  await handleResponse(response)
  return response.json()
}

export async function fetchUploadQueueTask(id: string): Promise<UploadQueueTask> {
  const response = await apiFetch(`/v1/upload-queue/${encodeURIComponent(id)}`)
  await handleResponse(response)
  return response.json()
}
