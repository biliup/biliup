import { useCallback, useEffect, useState, useSyncExternalStore, type RefObject } from 'react'
import type { FilmstripLayout, Waveform } from '@/app/lib/editor/contracts'

export interface Peaks { readonly rate: number; readonly seconds: number; readonly data: Uint8Array }
interface Picture { readonly src: string }
interface Strip extends Picture { readonly layout: FilmstripLayout }
type Snapshot<T> = { readonly phase: 'idle' | 'loading' | 'ready' | 'empty' | 'failed'; readonly value?: T }
type Loaded<T> = { value: T; bytes: number }
interface Entry<T> {
  url: string
  snapshot: Snapshot<T>
  listeners: Set<() => void>
  attempts: number
  generation: number
  bytes: number
  used: number
  controller?: AbortController | undefined
  timer?: ReturnType<typeof setTimeout> | undefined
}
const IDLE = { phase: 'idle' } as const
const queue = new Map<object, () => Promise<void>>()
let running = 0
function pump() {
  // Posters, strips and sound share two requests, leaving connections for playback.
  while (running < 2 && queue.size) {
    const [entry, task] = queue.entries().next().value!
    queue.delete(entry)
    running++
    void task().finally(() => { running--; pump() })
  }
}
class ResourceError extends Error {
  constructor(readonly retryable: boolean) { super('Preview unavailable') }
}
async function response(url: string, signal: AbortSignal) {
  // These URLs already include API_BASE. Cross-origin development also needs the session cookie.
  const result = await fetch(url, { credentials: 'include', signal })
  if (!result.ok) throw new ResourceError(![400, 401, 403, 410].includes(result.status))
  return result
}

class PreviewCache<T> {
  private entries = new Map<string, Entry<T>>()
  constructor(private load: (url: string, signal: AbortSignal) => Promise<Loaded<T> | undefined>, private dispose: (value: T) => void = () => {}) {}
  read = (url: string): Snapshot<T> => this.entries.get(url)?.snapshot ?? IDLE
  private notify(entry: Entry<T>) { entry.listeners.forEach(listener => listener()) }
  private stop(entry: Entry<T>) {
    entry.generation++
    queue.delete(entry)
    clearTimeout(entry.timer)
    entry.timer = undefined
    entry.controller?.abort()
    entry.controller = undefined
    if (entry.snapshot.phase === 'loading') entry.snapshot = IDLE
  }
  private prune() {
    let bytes = [...this.entries.values()].reduce((sum, entry) => sum + entry.bytes, 0)
    const unused = [...this.entries.values()].filter(entry => !entry.listeners.size).sort((a, b) => a.used - b.used)
    for (const entry of unused) {
      if (this.entries.size <= 64 && bytes <= 16 * 1024 * 1024) break
      this.stop(entry)
      if (entry.snapshot.value) this.dispose(entry.snapshot.value)
      bytes -= entry.bytes
      this.entries.delete(entry.url)
    }
  }
  private request(entry: Entry<T>) {
    if (!entry.listeners.size || entry.snapshot.phase === 'loading' || entry.snapshot.phase === 'ready' || entry.snapshot.phase === 'empty' || entry.timer || entry.attempts >= 4) return
    entry.snapshot = { phase: 'loading' }
    const generation = entry.generation
    queue.set(entry, async () => {
      if (entry.generation !== generation || !entry.listeners.size) return
      const controller = new AbortController()
      entry.controller = controller
      entry.attempts++
      const timeout = setTimeout(() => controller.abort(), 90_000)
      try {
        const loaded = await this.load(entry.url, controller.signal)
        if (entry.generation !== generation || controller.signal.aborted) {
          if (loaded) this.dispose(loaded.value)
          if (entry.generation !== generation) return
          throw new ResourceError(true)
        }
        entry.bytes = loaded?.bytes ?? 0
        entry.snapshot = loaded ? { phase: 'ready', value: loaded.value } : { phase: 'empty' }
      } catch (error) {
        if (entry.generation !== generation) return
        entry.snapshot = { phase: 'failed' }
        if (error instanceof ResourceError && !error.retryable) entry.attempts = 4
        if (entry.listeners.size && entry.attempts < 4) {
          entry.timer = setTimeout(() => { entry.timer = undefined; this.request(entry) }, 1000 * 2 ** entry.attempts)
        }
      } finally {
        clearTimeout(timeout)
        if (entry.generation === generation) {
          entry.controller = undefined
          this.notify(entry)
          this.prune()
        }
      }
    })
    this.notify(entry)
    pump()
  }
  subscribe(url: string, listener: () => void) {
    let entry = this.entries.get(url)
    if (!entry) {
      entry = { url, snapshot: IDLE, listeners: new Set(), attempts: 0, generation: 0, bytes: 0, used: Date.now() }
      this.entries.set(url, entry)
    }
    entry.used = Date.now()
    entry.listeners.add(listener)
    this.request(entry)
    this.prune()
    const current = entry
    return () => {
      current.listeners.delete(listener)
      if (!current.listeners.size) {
        this.stop(current)
        current.used = Date.now()
        this.prune()
      }
    }
  }
  retry(url: string) {
    const entry = this.entries.get(url)
    if (!entry) return
    this.stop(entry)
    if (entry.snapshot.value) this.dispose(entry.snapshot.value)
    entry.snapshot = IDLE
    entry.bytes = 0
    entry.attempts = 0
    this.request(entry)
    this.notify(entry)
  }
  reconnect() {
    for (const entry of this.entries.values()) if (entry.listeners.size && entry.snapshot.phase === 'failed') this.retry(entry.url)
  }
}
const releasePicture = (picture: Picture) => URL.revokeObjectURL(picture.src)
async function picture(result: Response) {
  const blob = await result.blob()
  if (!blob.size || blob.size > 8 * 1024 * 1024 || !blob.type.startsWith('image/')) throw new ResourceError(false)
  return { value: { src: URL.createObjectURL(blob) }, bytes: blob.size }
}
const posters = new PreviewCache<Picture>(async (url, signal) => picture(await response(url, signal)), releasePicture)
const strips = new PreviewCache<Strip>(async (url, signal) => {
  const result = await response(url, signal)
  const layout = JSON.parse(result.headers.get('x-biliup-filmstrip-layout') ?? 'null') as FilmstripLayout | null
  if (!layout || layout.version !== 1 || !Number.isFinite(layout.interval) || layout.interval <= 0 || !Number.isFinite(layout.seconds) || layout.seconds <= 0 || ![layout.count, layout.cols, layout.rows, layout.tileWidth, layout.tileHeight].every(value => Number.isInteger(value) && value > 0 && value <= 8192) || layout.count > layout.cols * layout.rows) throw new ResourceError(false)
  const loaded = await picture(result)
  return { ...loaded, value: { ...loaded.value, layout } }
}, releasePicture)
const waves = new PreviewCache<Peaks>(async (url, signal) => {
  const result = await response(url, signal)
  if (result.status === 204) return undefined
  const data = await result.json() as Waveform
  if (!Number.isFinite(data.rate) || data.rate <= 0 || !Number.isFinite(data.seconds) || data.seconds < 0 || typeof data.peaks !== 'string' || data.peaks.length > 24 * 1024 * 1024) throw new ResourceError(false)
  const binary = atob(data.peaks)
  const peaks = Uint8Array.from(binary, char => char.charCodeAt(0))
  return { value: { rate: data.rate, seconds: data.seconds, data: peaks }, bytes: peaks.byteLength }
})
let subscriptions = 0
function reconnect() { posters.reconnect(); strips.reconnect(); waves.reconnect() }
function useResource<T>(cache: PreviewCache<T>, url: string | undefined, wanted = true) {
  const subscribe = useCallback((listener: () => void) => {
    if (!url || !wanted) return () => {}
    if (subscriptions++ === 0) window.addEventListener('online', reconnect)
    const release = cache.subscribe(url, listener)
    return () => { release(); if (--subscriptions === 0) window.removeEventListener('online', reconnect) }
  }, [cache, url, wanted])
  const read = useCallback((): Snapshot<T> => url && wanted ? cache.read(url) : IDLE, [cache, url, wanted])
  return useSyncExternalStore(subscribe, read, (): Snapshot<T> => IDLE)
}
export const usePoster = (url: string | undefined, wanted = true) => useResource(posters, url, wanted)
export const useFilmstrip = (url: string | undefined, wanted: boolean) => useResource(strips, url, wanted).value
export const useWaveform = (url: string | undefined) => useResource(waves, url).value
export function usePreviewVisible(ref: RefObject<Element | null>) {
  const [visible, setVisible] = useState(false)
  useEffect(() => {
    const element = ref.current
    if (!element) return
    if (typeof IntersectionObserver === 'undefined') { setVisible(true); return }
    const observer = new IntersectionObserver(entries => setVisible(entries.some(entry => entry.isIntersecting)))
    observer.observe(element)
    return () => observer.disconnect()
  }, [ref])
  return visible
}
export const retryActivePreviews = reconnect
export function retryMediaPreviews(urls: { poster?: string | undefined; filmstrip?: string | undefined; waveform?: string | undefined }) {
  if (urls.poster) posters.retry(urls.poster)
  if (urls.filmstrip) strips.retry(urls.filmstrip)
  if (urls.waveform) waves.retry(urls.waveform)
}
