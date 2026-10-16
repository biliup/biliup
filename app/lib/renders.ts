'use client'
import useSWR, { mutate } from 'swr'
import { API_BASE, fetcher } from './api-streamer'
import { send, sendJson } from './clips'

export interface DanmakuSettings {
  enabled: boolean; font: string; font_size: number; opacity: number; outline: number
  scroll_seconds: number; display_area: number; density: number; offset_ms: number
  segment_offsets: Record<string, number>
}
export type EffectType = 'mosaic' | 'blur' | 'smooth_blur' | 'solid' | 'image'
export interface MaskRegion {
  id: string; effect_type: EffectType; x: number; y: number; width: number; height: number
  strength: number; color: string; opacity: number; asset_id: number | null; lock_aspect: boolean
  intervals: { from_ms: number; to_ms: number }[]
}
export interface RenderRecipe { danmaku: DanmakuSettings; regions: MaskRegion[]; encoding?: EncoderConfig }
export interface RenderAsset { id: number; width: number; height: number; sha256: string; url: string }
export type VideoEncoder = 'auto' | 'libx264' | 'h264_nvenc' | 'h264_qsv' | 'h264_vaapi' | 'h264_amf'
export type RenderQuality = 'space_saver' | 'balanced' | 'high_quality'
export interface EncoderConfig { encoder: VideoEncoder; device: string | null; quality: RenderQuality }
export const defaultEncoderConfig = (): EncoderConfig => ({ encoder: 'auto', device: null, quality: 'balanced' })
export interface RenderEncoderCapability {
  name: Exclude<VideoEncoder, 'auto'>; label: string; available: boolean; device: string | null; error: string | null
}
export interface RenderJob {
  id: number; session_id: number; clip_id: number | null
  view_session_id?: number; source_session_id?: number
  file_name?: string | null
  state: 'queued' | 'running' | 'ready' | 'failed' | 'cancelled'; phase: string; ratio: number | null
  error: string | null; output_bytes: number | null; duration_ms: number | null; created_at: number
  download_url: string | null
  requested_encoder?: VideoEncoder; requested_device?: string | null
  encoder?: string | null; encoder_device?: string | null; fallback_reason?: string | null
  fps?: number | null; speed?: number | null; processed_ms?: number | null; total_ms?: number | null
  started_at?: number | null; finished_at?: number | null; elapsed_ms: number; eta_ms?: number | null
  updated_at?: number; session_name?: string; session_title?: string
}
export interface RenderCapabilities {
  available: boolean; ffmpeg: boolean; libass: boolean; libx264: boolean
  encoders?: RenderEncoderCapability[]
  /** 内置 Rust 弹幕转换器在工作进程中自检通过；不需要外部程序 */
  danmaku_factory: boolean; font: boolean; error: string | null
}
export const defaultRecipe = (): RenderRecipe => ({
  danmaku: { enabled: false, font: 'Noto Sans CJK SC', font_size: 38, opacity: 0.8, outline: 2,
    scroll_seconds: 12, display_area: 0.4, density: -1, offset_ms: 0, segment_offsets: {} }, regions: [],
  encoding: defaultEncoderConfig(),
})
export const recipeUrl = (session: number) => `/v1/sessions/${session}/render-recipe`
export const assetsUrl = (session: number) => `/v1/sessions/${session}/render-assets`
export const jobsUrl = (session: number) => `/v1/sessions/${session}/renders`
export const allJobsUrl = (filters: { state?: string; session_id?: string; limit?: number; offset?: number } = {}) => {
  const params = new URLSearchParams()
  if (filters.state) params.set('state', filters.state)
  if (filters.session_id) params.set('session_id', filters.session_id)
  params.set('limit', String(filters.limit ?? 25))
  params.set('offset', String(filters.offset ?? 0))
  return `/v1/renders?${params.toString()}`
}
export interface RenderGpuDevice {
  id: string; name: string; vendor: 'nvidia' | 'intel' | 'amd' | 'unknown'; render_node: string | null
  utilization_percent: number | null; encoder_utilization_percent: number | null
  memory_used_bytes: number | null; memory_total_bytes: number | null; error: string | null
  engine_utilization: { name: string; utilization_percent: number | null }[]
  memory_kind: 'dedicated' | 'shared' | 'unknown'
  memory_source: string | null
}
export interface RenderGpuSnapshot { devices: RenderGpuDevice[]; sampled_at: number }
export const assetUrl = (id: number) => `${API_BASE}/v1/render-assets/${id}`
export const renderDownloadUrl = (id: number) => `${API_BASE}/v1/renders/${id}/download`
export const apiResourceUrl = (url: string) => url.startsWith('/v1/') ? API_BASE + url : url
export function useRenderSettings(session: number | null) {
  return useSWR<RenderRecipe>(session === null ? null : recipeUrl(session), fetcher, { revalidateOnFocus: false })
}
export function useRenderAssets(session: number | null) {
  return useSWR<{ assets: RenderAsset[] }>(session === null ? null : assetsUrl(session), fetcher, { revalidateOnFocus: false })
}
export function useRenderJobs(session: number | null) {
  return useSWR<{ jobs: RenderJob[] }>(session === null ? null : jobsUrl(session), fetcher, { refreshInterval: 2000 })
}
export function useAllRenderJobs(filters: { state?: string; session_id?: string; limit?: number; offset?: number }, allowed = true) {
  return useSWR<{ jobs: RenderJob[]; total: number; limit: number; offset: number }>(allowed ? allJobsUrl(filters) : null, fetcher,
    { refreshInterval: 2000, revalidateOnFocus: false })
}
export function useRenderGpus(allowed: boolean) {
  return useSWR<RenderGpuSnapshot>(allowed ? '/v1/render-gpus' : null, fetcher,
    { refreshInterval: 2000, revalidateOnFocus: false, shouldRetryOnError: false })
}
export function useRenderCapabilities(allowed: boolean) {
  return useSWR<{ tools: RenderCapabilities; font_urls: string[] }>(allowed ? '/v1/render-tools' : null, fetcher,
    { revalidateOnFocus: false, shouldRetryOnError: false })
}
export async function saveRecipe(session: number, recipe: RenderRecipe) {
  const saved = await sendJson<RenderRecipe>(recipeUrl(session), 'PUT', recipe)
  await mutate(recipeUrl(session), saved, false)
  return saved
}
export async function uploadAsset(session: number, file: File): Promise<RenderAsset> {
  const res = await send(assetsUrl(session), { method: 'POST', body: file, headers: { 'Content-Type': file.type } })
  const asset = await res.json() as RenderAsset
  void mutate(assetsUrl(session)); return asset
}
export async function deleteAsset(session: number, id: number) {
  await send(`/v1/render-assets/${id}`, { method: 'DELETE' }); void mutate(assetsUrl(session))
}
export async function createRender(session: number, clipId?: number, recipe?: RenderRecipe): Promise<RenderJob> {
  const job = await sendJson<RenderJob>(jobsUrl(session), 'POST', { clip_id: clipId ?? null, ...(recipe ? { recipe } : {}) })
  void mutate(jobsUrl(session)); return job
}
export async function renderJobAction(session: number, id: number, action: 'cancel' | 'retry') {
  await send(`/v1/renders/${id}/${action}`, { method: 'POST' })
  void mutate(jobsUrl(session)); void mutate(key => typeof key === 'string' && key.startsWith('/v1/renders?'))
}
export interface AssPreview { ass: string; font_urls: string[]; width: number; height: number; origin_ms: number; estimated_timing: boolean; comment_count: number }
export async function assPreview(session: number, recipe: RenderRecipe, from: number, to: number, signal: AbortSignal, canvasSegmentId?: number) {
  const res = await send(`/v1/sessions/${session}/render-preview`, { method: 'POST', signal,
    headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ recipe, from_ms: Math.round(from), to_ms: Math.round(to), canvas_segment_id: canvasSegmentId }) })
  return res.json() as Promise<AssPreview>
}
