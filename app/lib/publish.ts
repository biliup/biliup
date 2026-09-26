'use client'
import { useEffect, useMemo, useRef } from 'react'
import { mutate } from 'swr'
import { API_BASE } from './api-streamer'
import { type Clip, type ClipMode, clipsUrl, clipUrl, refresh, send, sendJson, usePolled } from './clips'

/** 切片封面从哪来（文件存在服务器上，GET /v1/clips/{cid}/cover） */
export type CoverSource = { source: 'frame'; t_ms: number } | { source: 'upload' } | { source: 'live' }

/** 发布设置里覆盖上传模板的部分；没给的字段沿用模板。与后端 `StudioOverride` 一致 */
export interface StudioOverride {
  title?: string
  desc?: string
  tags?: string[]
  tid?: number
  tid_v2?: number
  /** 定时发布的 Unix 秒；0 = 立即发布（不用模板里的定时） */
  dtime?: number
  cover?: CoverSource
}

/** 与后端一致 */
export const MAX_ARCHIVE_TITLE = 80
export const MAX_DESC = 2000
export const MAX_TAGS = 12
export const MAX_TAG_CHARS = 20
export const MAX_BATCH = 50
export const MAX_PARTS = 100
/** 定时发布：至少几小时之后、最多几天之内（与上传模板的定时选项、后端检查相同） */
export const DTIME_MIN_HOURS = 4
export const DTIME_MAX_DAYS = 15
export const COVER_TYPES = ['image/jpeg', 'image/png', 'image/webp']
export const MAX_COVER_BYTES = 5 * 1024 * 1024
/** B 站的稿件管理页：投稿结果未知时让用户去这里核对 */
export const ARCHIVE_MANAGER_URL = 'https://member.bilibili.com/platform/upload-manager/article'

export type JobState = 'queued' | 'running' | 'paused' | 'failed' | 'done'
export type JobStep = 'export' | 'upload' | 'submit'
export const JOB_STEPS: JobStep[] = ['export', 'upload', 'submit']

/** GET /v1/publish-jobs 里的一个任务（后端 `JobView`）；任务只在服务端内存里，服务重启后清空 */
export interface PublishJob {
  id: number
  session_id: number
  /** 各 P 的切片（合成多 P 时有多个） */
  clip_ids: number[]
  combine: boolean
  state: JobState
  step: JobStep | null
  /** 现在在做什么 */
  detail: string
  ratio: number | null
  /** 已经传好的 P 数 */
  uploaded: number
  error: string | null
  bvid: string | null
  title: string | null
  created_by: number | null
  created_at: number
  updated_at: number
}

export interface PublishQueue {
  /** 暂停的原因（B 站 601）；null = 没暂停。整个队列共用，不分场次 */
  paused: string | null
  jobs: PublishJob[]
}

/** 集中发布时没排进队列的一组切片 */
export interface Skipped {
  clip_ids: number[]
  reason: string
}

export interface Enqueued {
  jobs: PublishJob[]
  skipped: Skipped[]
}

/** 预览：按发布设置算出的最终稿件字段 */
export interface RenderedArchive {
  title: string
  desc: string
  tags: string[]
  tid: number | null
  tid_v2: number | null
  /** 一律是 2（转载） */
  copyright: number
  source: string
  dtime: number | null
  part_titles: string[]
  /** 模板本来选的是自制 */
  template_self_made: boolean
  template_name: string
}

export interface PreviewArchive {
  clip_ids: number[]
  template_id: number | null
  rendered: RenderedArchive | null
  /** 发不了的原因 */
  problem: string | null
  cover: CoverSource | null
}

export interface PublishBody {
  clip_ids: number[]
  combine?: boolean
  /** 不给 = 用切片存着的；null = 主播绑定的模板 */
  template_id?: number | null
  studio_override?: StudioOverride
  mode?: ClipMode
  /** 用户已到 B 站稿件管理核对过：上次投稿结果未知的切片也照发 */
  confirm_unknown?: boolean
}

export const queueUrl = (sessionId: number | null) =>
  sessionId === null ? '/v1/publish-jobs' : `/v1/publish-jobs?session=${sessionId}`
export const coverUrl = (clipId: number, nonce: number) => `${API_BASE}/v1/clips/${clipId}/cover?v=${nonce}`
export const thumbUrl = (sessionId: number, t: number, w = 480) =>
  `${API_BASE}/v1/sessions/${sessionId}/thumb?t=${Math.max(0, Math.round(t))}&w=${w}`
export const archiveUrl = (bvid: string) => `https://www.bilibili.com/video/${bvid}`

const active = (job: PublishJob) => job.state === 'queued' || job.state === 'running'
const queueRefresh = (data?: PublishQueue) => (data?.jobs.some(active) ? 1000 : 0)

/**
 * 发布队列（`sessionId` 为 null 时看全部场次）；有任务在排队或进行中时每秒刷新。任务结束时刷新涉及的切片，
 * 让「已发布」、稿件号及时出现。`enabled` 为 false 时不请求（没有权限时）。
 */
export function usePublishQueue(sessionId: number | null, enabled = true) {
  const swr = usePolled<PublishQueue>(enabled ? queueUrl(sessionId) : null, queueRefresh)
  const seen = useRef<Map<number, JobState>>(new Map())
  const jobs = swr.data?.jobs
  useEffect(() => {
    if (!jobs) return
    const sessions = new Set<number>()
    for (const job of jobs) {
      const before = seen.current.get(job.id)
      if (before !== undefined && before !== job.state && (job.state === 'done' || job.state === 'failed')) {
        sessions.add(job.session_id)
        for (const id of job.clip_ids) void mutate(clipUrl(id))
      }
      seen.current.set(job.id, job.state)
    }
    for (const id of sessions) void mutate(clipsUrl(id))
  }, [jobs])
  const byClip = useMemo(() => {
    const map = new Map<number, PublishJob>()
    for (const job of jobs ?? []) {
      for (const id of job.clip_ids) {
        const seen = map.get(id)
        // 同一个切片以未完成的任务为准，其次是最新的
        if (!seen || (seen.state === 'done' && job.state !== 'done') || (seen.state === job.state && job.id > seen.id)) {
          map.set(id, job)
        }
      }
    }
    return map
  }, [jobs])
  return { ...swr, byClip }
}

/** 队列按场次和全部两种 key 缓存，改动后两种都刷新 */
function refreshQueue(sessionId: number) {
  void mutate((key) => typeof key === 'string' && key.startsWith('/v1/publish-jobs'))
  void mutate(clipsUrl(sessionId))
}

export async function previewPublish(body: PublishBody): Promise<PreviewArchive[]> {
  const res = await sendJson<{ archives: PreviewArchive[] }>('/v1/publish-jobs/preview', 'POST', body)
  return res.archives
}

/** 发布一个切片：给了设置就先存到切片上；需要时先导出 */
export async function publishClip(
  clip: Pick<Clip, 'id' | 'session_id'>,
  body: Omit<PublishBody, 'clip_ids' | 'combine'>
): Promise<Enqueued> {
  const res = await sendJson<Enqueued>(`${clipUrl(clip.id)}/publish`, 'POST', body)
  refreshQueue(clip.session_id)
  return res
}

/** 集中发布：默认每个切片一个稿件，`combine` 时合成一个多 P 稿件。没排上的列在 `skipped` 里 */
export async function publishBatch(sessionId: number, body: PublishBody): Promise<Enqueued> {
  const res = await sendJson<Enqueued>('/v1/publish-jobs', 'POST', body)
  refreshQueue(sessionId)
  return res
}

/** 601 暂停后继续（整个队列） */
export async function resumeQueue(): Promise<void> {
  await sendJson('/v1/publish-jobs/resume', 'POST')
  void mutate((key) => typeof key === 'string' && key.startsWith('/v1/publish-jobs'))
}

export async function retryJob(job: PublishJob): Promise<void> {
  await sendJson(`/v1/publish-jobs/${job.id}/retry`, 'POST')
  refreshQueue(job.session_id)
}

export async function removeJob(job: PublishJob): Promise<void> {
  await send(`/v1/publish-jobs/${job.id}`, { method: 'DELETE' })
  refreshQueue(job.session_id)
}

/** 取 `tMs`（场次时间）那一帧做封面 */
export async function coverFromFrame(clip: Pick<Clip, 'id' | 'session_id'>, tMs: number): Promise<StudioOverride> {
  const over = await sendJson<StudioOverride>(`${clipUrl(clip.id)}/cover`, 'PUT', { t: Math.max(0, Math.round(tMs)) })
  refresh(clip)
  return over
}

/** 用开播时记下的直播间封面 */
export async function coverFromLive(clip: Pick<Clip, 'id' | 'session_id'>): Promise<StudioOverride> {
  const over = await sendJson<StudioOverride>(`${clipUrl(clip.id)}/cover`, 'PUT', { live: true })
  refresh(clip)
  return over
}

/** 上传本机图片做封面（JPEG / PNG / WebP，最大 5 MiB） */
export async function coverFromFile(clip: Pick<Clip, 'id' | 'session_id'>, file: File): Promise<StudioOverride> {
  const res = await send(`${clipUrl(clip.id)}/cover`, {
    method: 'PUT',
    headers: { 'Content-Type': file.type },
    body: file,
  })
  refresh(clip)
  return res.json()
}

/** 删掉切片封面，改回模板的封面 */
export async function removeCover(clip: Pick<Clip, 'id' | 'session_id'>): Promise<void> {
  await send(`${clipUrl(clip.id)}/cover`, { method: 'DELETE' })
  refresh(clip)
}

export function stepText(step: JobStep | null): string {
  return step === 'export' ? '导出' : step === 'upload' ? '上传' : step === 'submit' ? '投稿' : ''
}

/** 上次投稿结果未知（或投稿中没记上结果）：B 站那边可能已经有稿件，再发布前要用户核对 */
export function needsConfirm(clip: Pick<Clip, 'submit_state'>): boolean {
  return clip.submit_state != null
}

/** 能发布：没发布过、不在发布队列里未完成的任务中 */
export function canPublish(clip: Clip, job: PublishJob | undefined): boolean {
  return clip.state !== 'published' && clip.state !== 'discarded' && !(job && job.state !== 'done')
}
