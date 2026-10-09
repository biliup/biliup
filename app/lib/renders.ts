'use client'
import useSWR, { mutate } from 'swr'
import { API_BASE, fetcher } from './api-streamer'
import { send, sendJson } from './clips'

export interface DanmakuSettings {
  enabled: boolean; font: string; font_size: number; opacity: number; outline: number
  scroll_seconds: number; display_area: number; density: number; offset_ms: number
  segment_offsets: Record<string, number>
}
export type EffectType = 'mosaic' | 'blur' | 'solid' | 'image'
export interface MaskRegion {
  id: string; effect_type: EffectType; x: number; y: number; width: number; height: number
  strength: number; color: string; opacity: number; asset_id: number | null; lock_aspect: boolean
  intervals: { from_ms: number; to_ms: number }[]
}
export interface RenderRecipe { danmaku: DanmakuSettings; regions: MaskRegion[] }
export interface RenderAsset { id: number; width: number; height: number; sha256: string; url: string }
export interface RenderJob {
  id: number; session_id: number; clip_id: number | null
  state: 'queued' | 'running' | 'ready' | 'failed' | 'cancelled'; phase: string; ratio: number | null
  error: string | null; output_bytes: number | null; duration_ms: number | null; created_at: number
  download_url: string | null
}
export interface RenderCapabilities {
  available: boolean; ffmpeg: boolean; libass: boolean; libx264: boolean
  danmaku_factory: boolean; font: boolean; error: string | null
}
export const defaultRecipe = (): RenderRecipe => ({
  danmaku: { enabled: false, font: 'Noto Sans CJK SC', font_size: 38, opacity: 0.8, outline: 2,
    scroll_seconds: 12, display_area: 0.4, density: -1, offset_ms: 0, segment_offsets: {} }, regions: [],
})
export const recipeUrl = (session: number) => `/v1/sessions/${session}/render-recipe`
export const assetsUrl = (session: number) => `/v1/sessions/${session}/render-assets`
export const jobsUrl = (session: number) => `/v1/sessions/${session}/renders`
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
  await send(`/v1/renders/${id}/${action}`, { method: 'POST' }); void mutate(jobsUrl(session))
}
export interface AssPreview { ass: string; font_urls: string[]; width: number; height: number; origin_ms: number; estimated_timing: boolean; comment_count: number }
export async function assPreview(session: number, recipe: RenderRecipe, from: number, to: number, signal: AbortSignal, canvasSegmentId?: number) {
  const res = await send(`/v1/sessions/${session}/render-preview`, { method: 'POST', signal,
    headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ recipe, from_ms: Math.round(from), to_ms: Math.round(to), canvas_segment_id: canvasSegmentId }) })
  return res.json() as Promise<AssPreview>
}
