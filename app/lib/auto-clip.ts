'use client'
import { useEffect, useRef } from 'react'
import useSWR, { mutate } from 'swr'
import { fetcher } from './api-streamer'
import { type Clip, refresh as refreshClip, send, usePolled } from './clips'
import { formatSpan } from './sessions'

/** GET /v1/auto-clip/status（后端 `api/auto_clip.rs::StatusView`） */
export interface AutoClipStatus {
  /** 全局 `auto_clip.enabled` */
  enabled: boolean
  /** 填了接口地址和 chat 模型 */
  configured: boolean
  api_host: string | null
  chat_model: string | null
  asr_host: string | null
  asr_model: string | null
  key_source: 'env' | 'config' | null
  thumbnails: 'auto' | 'on' | 'off'
  thumbnails_active: boolean
  vision: boolean | null
  asr_segments: boolean | null
  max_asr_minutes: number
  max_chat_tokens: number
}

export type JobState = 'queued' | 'running' | 'done' | 'failed' | 'canceled'
export type JobStage = 'audio' | 'asr' | 'signals' | 'analyze'

/** 自动切片任务（后端 `auto_clip/jobs.rs::Job`） */
export interface AutoClipJob {
  id: number
  session_id: number
  trigger: 'auto' | 'manual'
  state: JobState
  stage: JobStage | null
  /** 进度的单位随阶段变：抽音频按分段、转写按音频块、分析按窗 */
  progress_done: number
  progress_total: number
  /** 下播后自动入队的任务要等到这个时刻（毫秒）才开始 */
  not_before: number
  error: string | null
  asr_planned_seconds: number | null
  asr_seconds: number
  tokens_in: number
  tokens_out: number
  images: number
  warnings: string[]
  reuse_transcript: boolean
  created_at: number
  started_at: number | null
  finished_at: number | null
}

/** 生成前的预估（后端 `auto_clip/runner.rs::Estimate`） */
export interface AutoClipEstimate {
  recorded_seconds: number
  /** 这次要送转写的秒数（已跳过静音、扣掉已转写的） */
  asr_seconds: number
  transcribed_seconds: number
  /** `silence`：按抽过音频的静音表算；`duration`：还没抽过音频，按录像时长算（实际会更少） */
  basis: 'silence' | 'duration'
  max_asr_minutes: number
  over_limit: boolean
  message: string | null
  chat: {
    tokens: number
    images: number
    windows: number
    /** `transcript`：转写齐了按实际提示词算；`duration`：还有没转写的语音，按时长估 */
    basis: 'transcript' | 'duration'
  }
  max_chat_tokens: number
  chat_over_limit: boolean
  chat_message: string | null
}

/** GET / POST /v1/sessions/{id}/auto-clip */
export interface SessionAutoClip {
  enabled: boolean
  job: AutoClipJob | null
  estimate: AutoClipEstimate | null
}

export type SuggestionState = 'pending' | 'accepted' | 'dismissed' | 'expired'

/** 候选片段（后端 `auto_clip/suggestions.rs::Suggestion`） */
export interface Suggestion {
  id: number
  session_id: number
  job_id: number | null
  in_ms: number
  out_ms: number
  title: string
  reason: string
  /** 模型自评的把握，0～1；没给时为 null */
  confidence: number | null
  tags: string[]
  evidence: {
    /** 区间里的转写句子数 */
    asr_lines: number
    /** 与区间重叠的弹幕高峰里的弹幕条数 */
    danmaku: number
    /** 引用到的缩图（场次时间） */
    images: number[]
  }
  state: SuggestionState
  clip_id: number | null
  created_at: number
  updated_at: number
  /** 只有待处理的有：过了这个时刻（建出来 72 小时）就过期 */
  expires_at: number | null
}

export const AUTO_CLIP_STATUS_KEY = '/v1/auto-clip/status'
export const sessionAutoClipUrl = (sessionId: number) => `/v1/sessions/${sessionId}/auto-clip`
export const suggestionsUrl = (sessionId: number) => `/v1/sessions/${sessionId}/suggestions`

/** 设置页里自动切片那一节 */
export const AUTO_CLIP_SETTINGS_HREF = '/dashboard'

/** 任务在排队或运行时多久刷新一次 */
const ACTIVE_POLL_MS = 3_000
/** 有待处理的候选时多久刷新一次列表（跟上后台的过期清理） */
const PENDING_POLL_MS = 60_000

export type AutoClipAvailability = {
  status: AutoClipStatus | undefined
  /**
   * 入口要不要出现：填过接口（`configured`）或开着（`enabled`）。都没有时说明没用过这个功能，
   * 各处不显示入口，也不再请求场次级的接口
   */
  visible: boolean
  enabled: boolean
  /** 不能生成候选的原因（设置层面的）；能生成时为 null */
  setupReason: string | null
}

/** 全局的自动切片状态：各页共用一个请求；`active` 为 false 时不请求 */
export function useAutoClip(active = true): AutoClipAvailability {
  const { data } = useSWR<AutoClipStatus>(active ? AUTO_CLIP_STATUS_KEY : null, fetcher, {
    revalidateOnFocus: false,
    shouldRetryOnError: false,
  })
  const visible = !!data && (data.configured || data.enabled)
  const setupReason = !data
    ? '正在读取自动切片的设置…'
    : !data.enabled
      ? '自动切片没有开启：到「空间配置」→「自动切片（实验）」里打开'
      : !data.configured
        ? '没有配置 chat 接口：到「空间配置」→「自动切片（实验）」填好接口地址和分析用的模型'
        : !data.asr_model
          ? '没有配置转写模型：到「空间配置」→「自动切片（实验）」填好转写模型（asr_model）'
          : null
  return { status: data, visible, enabled: !!data?.enabled, setupReason }
}

export const isActive = (job: AutoClipJob | null | undefined): boolean =>
  !!job && (job.state === 'queued' || job.state === 'running')

const sessionRefresh = (data?: SessionAutoClip) => (isActive(data?.job) ? ACTIVE_POLL_MS : 0)

/**
 * 场次的自动切片任务与预估；任务在排队或运行时每 3 秒刷新，任务结束时顺带刷新候选列表。
 * `sessionId` 为 null 时不请求。
 */
export function useSessionAutoClip(sessionId: number | null) {
  const swr = usePolled<SessionAutoClip>(
    sessionId === null ? null : sessionAutoClipUrl(sessionId),
    sessionRefresh,
    { revalidateOnFocus: false, shouldRetryOnError: false }
  )
  const job = swr.data?.job ?? null
  const last = useRef<{ id: number; active: boolean } | null>(null)
  useEffect(() => {
    if (sessionId === null || !job) return
    const before = last.current
    last.current = { id: job.id, active: isActive(job) }
    if (before && before.id === job.id && before.active && !isActive(job)) void mutate(suggestionsUrl(sessionId))
  }, [sessionId, job])
  return swr
}

const suggestionsRefresh = (data?: { suggestions: Suggestion[] }) =>
  data?.suggestions.some((s) => s.state === 'pending') ? PENDING_POLL_MS : 0

/** 场次的候选列表；`sessionId` 为 null 时不请求 */
export function useSuggestions(sessionId: number | null) {
  return usePolled<{ suggestions: Suggestion[] }>(
    sessionId === null ? null : suggestionsUrl(sessionId),
    suggestionsRefresh,
    { revalidateOnFocus: false }
  )
}

/** 过了 72 小时还没处理的候选后台会定时记为过期；在那之前界面先按过期显示 */
export function effectiveState(s: Suggestion, nowMs: number): SuggestionState {
  return s.state === 'pending' && s.expires_at !== null && s.expires_at <= nowMs ? 'expired' : s.state
}

/** 后端的错误有的是 `{"message": …}`（`ApiError`），有的是纯文本：取出给人看的那句 */
export function autoClipError(e: unknown): string {
  const raw = e instanceof Error ? e.message : String(e)
  try {
    const parsed = JSON.parse(raw)
    if (parsed && typeof parsed.message === 'string' && parsed.message) return parsed.message
  } catch {
    // 纯文本
  }
  return raw
}

async function postJson<T>(url: string, body: unknown, method = 'POST'): Promise<T> {
  const res = await send(url, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  return res.json()
}

/** 只要预估，不入队 */
export function fetchEstimate(sessionId: number, reuseTranscript: boolean): Promise<SessionAutoClip> {
  return postJson(sessionAutoClipUrl(sessionId), { reuse_transcript: reuseTranscript })
}

/** 确认后入队 */
export async function startAutoClip(sessionId: number, reuseTranscript: boolean): Promise<SessionAutoClip> {
  const result = await postJson<SessionAutoClip>(sessionAutoClipUrl(sessionId), {
    confirm: true,
    reuse_transcript: reuseTranscript,
  })
  await mutate(sessionAutoClipUrl(sessionId), result, { revalidate: false })
  return result
}

export async function cancelAutoClip(sessionId: number): Promise<void> {
  await send(sessionAutoClipUrl(sessionId), { method: 'DELETE' })
  await mutate(sessionAutoClipUrl(sessionId))
}

/** 接受候选：按给的入点、出点（不给就用候选自己的）建切片草稿 */
export async function acceptSuggestion(
  s: Pick<Suggestion, 'id' | 'session_id'>,
  body: { in_ms?: number; out_ms?: number; title?: string } = {}
): Promise<{ suggestion: Suggestion; clip: Clip }> {
  const result = await postJson<{ suggestion: Suggestion; clip: Clip }>(
    `${suggestionsUrl(s.session_id)}/${s.id}/accept`,
    body
  )
  refreshClip(result.clip)
  void mutate(suggestionsUrl(s.session_id))
  return result
}

export async function dismissSuggestion(s: Pick<Suggestion, 'id' | 'session_id'>): Promise<Suggestion> {
  const res = await send(`${suggestionsUrl(s.session_id)}/${s.id}/dismiss`, { method: 'POST' })
  const updated: Suggestion = await res.json()
  void mutate(suggestionsUrl(s.session_id))
  return updated
}

export function isOverLimit(job: AutoClipJob): boolean {
  return job.state === 'failed' && !!job.error && /超过每场.*上限/.test(job.error)
}

export type JobTone = 'grey' | 'blue' | 'green' | 'orange' | 'red'

/** 任务的状态标签：排队 / 转写中 / 生成中 / 完成 / 失败 / 超上限 / 已取消 */
export function jobBadge(job: AutoClipJob, nowMs: number): { label: string; tone: JobTone; detail: string } {
  const progress = (unit: string) =>
    job.progress_total > 0 ? ` ${Math.min(job.progress_done, job.progress_total)}/${job.progress_total} ${unit}` : ''
  switch (job.state) {
    case 'queued':
      return {
        label: '排队',
        tone: 'grey',
        detail:
          job.not_before > nowMs
            ? `下播后等录像落盘，${new Date(job.not_before).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', hour12: false })} 开始`
            : '等前面的任务做完',
      }
    case 'running':
      if (job.stage === 'audio' || job.stage === null) {
        return { label: '转写中', tone: 'blue', detail: `抽音频${progress('段')}` }
      }
      if (job.stage === 'asr') return { label: '转写中', tone: 'blue', detail: `语音转写${progress('块')}` }
      if (job.stage === 'signals') return { label: '生成中', tone: 'blue', detail: '整理弹幕高峰和截图' }
      return { label: '生成中', tone: 'blue', detail: `分析${progress('窗')}` }
    case 'done':
      return { label: '完成', tone: job.warnings.length ? 'orange' : 'green', detail: usageText(job) }
    case 'canceled':
      return { label: '已取消', tone: 'grey', detail: '' }
    default:
      return isOverLimit(job)
        ? { label: '超上限', tone: 'orange', detail: '没有调用模型' }
        : { label: '失败', tone: 'red', detail: usageText(job) }
  }
}

/** 这次实际的用量：转写分钟、chat token、截图 */
export function usageText(job: AutoClipJob): string {
  const parts: string[] = []
  if (job.asr_seconds > 0) parts.push(`转写 ${formatSpan(job.asr_seconds * 1000)}`)
  const tokens = job.tokens_in + job.tokens_out
  if (tokens > 0) parts.push(`chat ${tokens.toLocaleString('zh-CN')} token`)
  if (job.images > 0) parts.push(`截图 ${job.images} 张`)
  return parts.join(' · ')
}

export type JobHint = {
  /** 后端给的原文 */
  text: string
  /** 补充的下一步 */
  todo: string | null
  /** 下一步要去设置页 */
  settings: boolean
  level: 'error' | 'warning'
}

const WARNING_TODO: [RegExp, string, boolean][] = [
  [
    /不能看图|拒收图片/,
    '换一个能看图的 chat 模型，在「空间配置」里重新「测试连接」；不需要截图时可以把「随分析发送截图」改成关，就不会再提示',
    true,
  ],
  [/还没对当前 chat 模型做连通性测试/, '到「空间配置」→「自动切片（实验）」点「测试连接」，再重新生成', true],
  [
    /chat 已用约.*超过每场上限/,
    '只分析了前面的部分。要分析完整场：调高每场 chat 用量上限（max_chat_tokens）后重新生成，已转写的部分会沿用，不再计转写费',
    true,
  ],
  [/没有弹幕记录/, '在录制设置里开启弹幕录制，之后的场次会多一个信号', false],
  [/分句时间戳/, '候选边界会粗一些；可以载入选段后在细节条上微调入点、出点再接受', false],
]

const ERROR_TODO: [RegExp, string, boolean][] = [
  [/超过每场上限 \d+ 分钟/, '调高每场转写上限（max_asr_minutes）后重新生成；这次没有调用转写，不产生费用', true],
  [/超过每场上限 \d+ token/, '调高每场 chat 用量上限（max_chat_tokens）或关掉截图后重新生成；已转写的部分会沿用', true],
  [/API key|没有权限使用模型|地址或模型不存在|接口地址不对|连不上|不是预期的格式/, '到「空间配置」→「自动切片（实验）」改好后点「测试连接」确认，再重新生成', true],
  [/限流|额度/, '稍后重新生成；已转写的部分会沿用', false],
  [/超时/, '检查网络，或在「空间配置」里调大超时后重新生成', true],
  [/服务端出错/, '稍后重新生成；一直这样就换一个接口地址', false],
  [/没有可以转写的音频|没有音频/, '这一场的录像里没有音轨，不能生成候选', false],
]

/** 任务的失败原因和警告，每条附上能照着做的下一步 */
export function jobHints(job: AutoClipJob): JobHint[] {
  const hints: JobHint[] = []
  if (job.state === 'failed' && job.error) {
    const hit = ERROR_TODO.find(([re]) => re.test(job.error!))
    hints.push({
      text: job.error,
      todo: hit ? hit[1] : '按上面的原因处理后重新生成',
      settings: hit ? hit[2] : false,
      level: isOverLimit(job) ? 'warning' : 'error',
    })
  }
  for (const w of job.warnings) {
    const hit = WARNING_TODO.find(([re]) => re.test(w))
    hints.push({ text: w, todo: hit ? hit[1] : null, settings: hit ? hit[2] : false, level: 'warning' })
  }
  return hints
}

export function confidenceLevel(c: number | null): { label: string; tone: 'green' | 'blue' | 'grey' } | null {
  if (c === null) return null
  if (c >= 0.75) return { label: '高', tone: 'green' }
  if (c >= 0.5) return { label: '中', tone: 'blue' }
  return { label: '低', tone: 'grey' }
}
