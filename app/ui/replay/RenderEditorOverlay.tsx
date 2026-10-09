'use client'
import React, { useEffect, useRef, useState } from 'react'
import { apiResourceUrl, assPreview, type AssPreview, type MaskRegion } from '@/app/lib/renders'
import { moveRegion, pictureBounds, regionActive, resizeRegion, type PictureBounds, type ResizeHandle } from '@/app/lib/render-geometry'
import type { SegmentView } from '@/app/lib/sessions'
import type { RenderEditorState } from './RenderEditor'
import styles from './replay.module.scss'

const handles: ResizeHandle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w']
const handlePoint: Record<ResizeHandle, [number, number]> = {
  nw: [0, 0], n: [.5, 0], ne: [1, 0], e: [1, .5], se: [1, 1], s: [.5, 1], sw: [0, 1], w: [0, .5],
}
type AssRenderer = import('jassub').default
let assModule: Promise<{ default: typeof import('jassub').default }> | null = null
/** Load the bundled renderer as a script so Next doesn't recursively bundle Emscripten pthread workers. */
function loadAssRenderer() {
  if (!assModule) assModule = new Promise<{ default: typeof import('jassub').default }>((resolve, reject) => {
    const script = document.createElement('script')
    script.src = '/jassub/renderer.js'
    script.onload = () => {
      const rendererModule = (globalThis as typeof globalThis & { BiliupASS?: { default: typeof import('jassub').default } }).BiliupASS
      if (rendererModule?.default) resolve(rendererModule)
      else { assModule = null; script.remove(); reject(new Error('ASS 渲染器加载后不可用')) }
    }
    script.onerror = () => { assModule = null; script.remove(); reject(new Error('ASS 渲染器文件加载失败，请重新构建前端静态资源')) }
    document.head.appendChild(script)
  })
  return assModule
}

export default function RenderEditorOverlay({ state: s, video, current, segments, canEdit, onDimensions, canvasSegmentId }: {
  state: RenderEditorState; video: HTMLVideoElement | HTMLCanvasElement | null; current: () => number; segments: SegmentView[]
  canvasSegmentId?: number; canEdit: boolean; onDimensions: (dimensions: { width: number; height: number }) => void
}) {
  const holder = useRef<HTMLDivElement>(null)
  const frame = useRef<HTMLCanvasElement>(null)
  const assHost = useRef<HTMLDivElement>(null)
  const [bounds, setBounds] = useState<PictureBounds | null>(null)
  const [editBounds, setEditBounds] = useState<PictureBounds | null>(null)
  const [canvasSize, setCanvasSize] = useState<{ width: number; height: number } | null>(null)
  const [maskError, setMaskError] = useState<string | null>(null)
  const [assError, setAssError] = useState<string | null>(null)
  const [assLoading, setAssLoading] = useState(false)
  const [estimatedTiming, setEstimatedTiming] = useState(false)
  const [at, setAt] = useState(0)
  const latest = useRef({ recipe: s.recipe, current, preview: s.preview, canvasSize })
  useEffect(() => { latest.current = { recipe: s.recipe, current, preview: s.preview, canvasSize } })
  const images = useRef(new Map<number, HTMLImageElement>())
  const imageFailures = useRef(new Set<number>())
  const previews = useRef(new Map<string, AssPreview>())
  const fontCache = useRef(new Map<string, Uint8Array>())
  const regionDrag = useRef<{ id: string; region: MaskRegion; x: number; y: number; handle: ResizeHandle | null } | null>(null)
  const activeSegment = segments.find(segment => at >= segment.start_ms && at < (segment.end_ms ?? Infinity))
  const activeSegmentId = activeSegment?.id
  const activeSegmentStart = activeSegment?.start_ms
  const activeSegmentEnd = activeSegment?.end_ms
  const hasBounds = bounds !== null
  const danmakuEnabled = s.recipe.danmaku.enabled
  // Cache ASS around the playhead. Dragging a mask doesn't run the converter again.
  const windowIndex = Math.floor(at / 300000)
  const danmakuKey = JSON.stringify(s.recipe.danmaku)

  useEffect(() => {
    if (!video || !holder.current) return
    const root = holder.current
    let lastWidth = 0, lastHeight = 0
    const measure = () => {
      const rect = root.getBoundingClientRect()
      const videoWidth = 'videoWidth' in video ? video.videoWidth : video.width
      const videoHeight = 'videoHeight' in video ? video.videoHeight : video.height
      const target = s.preview && canvasSize ? canvasSize : { width: videoWidth, height: videoHeight }
      const picture = pictureBounds({ left: 0, top: 0, width: rect.width, height: rect.height }, target.width, target.height)
      setBounds(picture)
      setEditBounds(picture ? pictureBounds(picture, videoWidth, videoHeight) : null)
      if (videoWidth && videoHeight && (lastWidth !== videoWidth || lastHeight !== videoHeight)) {
        lastWidth = videoWidth; lastHeight = videoHeight
        onDimensions({ width: lastWidth, height: lastHeight })
      }
    }
    const observer = new ResizeObserver(measure); observer.observe(root)
    video.addEventListener('loadedmetadata', measure); video.addEventListener('resize', measure)
    measure()
    return () => { observer.disconnect(); video.removeEventListener('loadedmetadata', measure); video.removeEventListener('resize', measure) }
  }, [video, onDimensions, canvasSize, s.preview])

  useEffect(() => {
    images.current.clear(); imageFailures.current.clear()
    for (const asset of s.assets.data?.assets ?? []) {
      const image = new Image()
      image.crossOrigin = 'use-credentials'
      image.onload = () => { images.current.set(asset.id, image); imageFailures.current.delete(asset.id) }
      image.onerror = () => { imageFailures.current.add(asset.id) }
      image.src = apiResourceUrl(asset.url)
    }
  }, [s.assets.data])

  useEffect(() => {
    if (!video || !frame.current || !s.open) return
    const output = frame.current
    const ctx = output.getContext('2d')
    const composite = document.createElement('canvas'), sample = document.createElement('canvas')
    const drawing = composite.getContext('2d'), sampled = sample.getContext('2d')
    if (!ctx || !drawing || !sampled) { setMaskError('浏览器不支持 Canvas 2D，无法预览遮挡。请升级浏览器后重试。'); return }
    let raf = 0, previous = 0, errorShown = false
    const draw = (now: number) => {
      raf = requestAnimationFrame(draw)
      if (now - previous < 33) return
      previous = now
      const { recipe, preview, current, canvasSize } = latest.current
      const time = current(); setAt(time)
      const isVideo = 'videoWidth' in video
      const width = isVideo ? video.videoWidth : video.width, height = isVideo ? video.videoHeight : video.height
      if ((isVideo && video.readyState < 2) || !width || !height) { ctx.clearRect(0, 0, output.width, output.height); return }
      const target = preview && canvasSize ? canvasSize : { width, height }
      if (output.width !== target.width || output.height !== target.height) { output.width = target.width; output.height = target.height }
      if (composite.width !== width || composite.height !== height) { composite.width = width; composite.height = height }
      ctx.clearRect(0, 0, output.width, output.height)
      if (isVideo && !preview) return
      try {
        drawing.clearRect(0, 0, width, height); drawing.drawImage(video, 0, 0, width, height)
        for (const r of preview ? recipe.regions : []) {
          if (!regionActive(r.intervals, time)) continue
          const x = r.x * width, y = r.y * height, w = r.width * width, h = r.height * height
          drawing.save(); drawing.globalAlpha = r.opacity
          if (r.effect_type === 'solid') { drawing.fillStyle = r.color; drawing.fillRect(x, y, w, h) }
          else if (r.effect_type === 'image') {
            const image = r.asset_id ? images.current.get(r.asset_id) : null
            if (!image && r.asset_id && imageFailures.current.has(r.asset_id)) throw new Error(`遮挡图片 #${r.asset_id} 加载失败，请刷新或重新上传。`)
            if (image) drawing.drawImage(image, x, y, w, h)
          } else if (r.effect_type === 'mosaic') {
            const block = Math.max(1, Math.round(r.strength))
            sample.width = Math.max(1, Math.floor(w / block)); sample.height = Math.max(1, Math.floor(h / block))
            sampled.imageSmoothingEnabled = false; sampled.drawImage(composite, x, y, w, h, 0, 0, sample.width, sample.height)
            drawing.imageSmoothingEnabled = false; drawing.drawImage(sample, x, y, w, h)
          } else {
            sample.width = Math.max(1, Math.round(w)); sample.height = Math.max(1, Math.round(h))
            if (!('filter' in sampled)) throw new Error('浏览器不支持 Canvas 模糊滤镜，请使用 Chrome、Edge 或 Firefox 预览。')
            sampled.filter = `blur(${r.strength}px)`; sampled.drawImage(composite, x, y, w, h, 0, 0, sample.width, sample.height)
            drawing.drawImage(sample, x, y, w, h)
          }
          drawing.restore()
        }
        const destination = pictureBounds({ left: 0, top: 0, width: output.width, height: output.height }, width, height)
        ctx.fillStyle = 'black'; ctx.fillRect(0, 0, output.width, output.height)
        if (destination) ctx.drawImage(composite, destination.left, destination.top, destination.width, destination.height)
        if (errorShown) { setMaskError(null); errorShown = false }
      } catch (e) {
        if (!errorShown) { setMaskError(e instanceof Error ? e.message : String(e)); errorShown = true }
      }
    }
    raf = requestAnimationFrame(draw)
    return () => cancelAnimationFrame(raf)
  }, [video, s.open, hasBounds])

  /* eslint-disable react-hooks/set-state-in-effect -- The imperative ASS renderer reports loading and browser capability state. */
  useEffect(() => {
    if (!s.open || !s.preview || !video || !assHost.current || activeSegmentId === undefined || activeSegmentStart === undefined) {
      setAssError(null); setAssLoading(false); return
    }
    if (danmakuEnabled && (!globalThis.Worker || !globalThis.OffscreenCanvas || !HTMLCanvasElement.prototype.transferControlToOffscreen || !globalThis.WebAssembly)) {
      setAssError('当前浏览器缺少 ASS 预览所需的 WebAssembly / OffscreenCanvas。请更新 Chrome、Edge 或 Firefox；合成导出仍可使用。'); return
    }
    const abort = new AbortController(), host = assHost.current
    const canvas = document.createElement('canvas'); canvas.style.width = '100%'; canvas.style.height = '100%'
    host.replaceChildren(canvas)
    let renderer: AssRenderer | undefined, disposed = false, raf = 0
    setAssError(null); setAssLoading(danmakuEnabled)
    const timeout = setTimeout(() => {
      setAssError('ASS 预览初始化超时。请检查服务器字体、DanmakuFactory 以及浏览器是否允许 Web Worker，然后关闭并重新开启实时预览。')
      setAssLoading(false)
    }, 20000)
    const debounce = setTimeout(async () => {
      try {
        const origin = Math.max(activeSegmentStart, windowIndex * 300000 - 60000)
        const end = Math.min(activeSegmentEnd ?? origin + 660000, (windowIndex + 2) * 300000)
        const key = `${s.sessionId}:${activeSegmentId}:${canvasSegmentId}:${origin}:${end}:${danmakuKey}`
        const cached = previews.current.get(key)
        const [data, rendererModule] = await Promise.all([
          cached ? Promise.resolve(cached) : assPreview(s.sessionId, { ...latest.current.recipe, regions: [] }, origin, end, abort.signal, canvasSegmentId), danmakuEnabled ? loadAssRenderer() : Promise.resolve(null),
        ])
        if (disposed) return
        if (!cached) {
          if (previews.current.size >= 6) previews.current.delete(previews.current.keys().next().value!)
          previews.current.set(key, data)
        }
        setCanvasSize({ width: data.width, height: data.height })
        setEstimatedTiming(data.estimated_timing)
        if (!danmakuEnabled || !rendererModule) { clearTimeout(timeout); setAssLoading(false); return }
        const fontUrls = data.font_urls.map(apiResourceUrl)
        // Fetch with credentials first: worker fetches don't carry cross-origin server cookies.
        const fonts = await Promise.all(fontUrls.map(async url => {
          const cachedFont = fontCache.current.get(url)
          if (cachedFont) return cachedFont
          const res = await fetch(url, { credentials: 'include', signal: abort.signal })
          if (!res.ok) throw new Error(`字体加载失败（HTTP ${res.status}）`)
          const font = new Uint8Array(await res.arrayBuffer())
          fontCache.current.set(url, font)
          return font
        }))
        if (disposed) return
        renderer = new rendererModule.default({ canvas, subContent: data.ass, fonts, queryFonts: false,
          defaultFont: latest.current.recipe.danmaku.font,
          workerUrl: '/jassub/worker.js', wasmUrl: '/jassub/jassub-worker.wasm', modernWasmUrl: '/jassub/jassub-worker-modern.wasm' })
        await renderer.ready
        if (disposed) { void renderer.destroy(); return }
        clearTimeout(timeout); setAssLoading(false)
        let previousTime = NaN
        const draw = async () => {
          if (disposed || !renderer) return
          const time = (latest.current.current() - data.origin_ms) / 1000
          if (time !== previousTime) {
            previousTime = time
            await renderer.manualRender({ expectedDisplayTime: performance.now(), width: data.width, height: data.height, mediaTime: time }, true)
          }
          if (!disposed) raf = requestAnimationFrame(() => { void draw().catch(e => { setAssError(`ASS 预览失败：${e instanceof Error ? e.message : String(e)}。可关闭并重新开启预览。`) }) })
        }
        await draw()
      } catch (e) {
        if (!disposed) { clearTimeout(timeout); setAssLoading(false); setAssError(`ASS 预览失败：${e instanceof Error ? e.message : String(e)}。请检查弹幕 XML、转换器及字体配置。`) }
      }
    }, 450)
    return () => {
      disposed = true; clearTimeout(timeout); clearTimeout(debounce); cancelAnimationFrame(raf); abort.abort()
      if (renderer) void renderer.destroy().catch(() => undefined)
      canvas.remove()
    }
  // Recipe masks do not affect ASS and shouldn't reset the renderer during a drag.
  }, [s.sessionId, s.open, s.preview, danmakuKey, danmakuEnabled, video, activeSegmentId, activeSegmentStart, activeSegmentEnd, windowIndex, hasBounds, canvasSegmentId])
  /* eslint-enable react-hooks/set-state-in-effect */

  const pointerDown = (event: React.PointerEvent<HTMLDivElement>, region: MaskRegion, handle: ResizeHandle | null) => {
    if (!canEdit || !s.editing || !editBounds) return
    event.preventDefault(); event.stopPropagation(); event.currentTarget.setPointerCapture(event.pointerId)
    s.setSelected(region.id)
    regionDrag.current = { id: region.id, region, x: event.clientX, y: event.clientY, handle }
  }
  const pointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    const drag = regionDrag.current
    if (!drag || !editBounds || !canEdit || !s.editing) return
    const dx = (event.clientX - drag.x) / editBounds.width, dy = (event.clientY - drag.y) / editBounds.height
    const geometry = drag.handle ? resizeRegion(drag.region, drag.handle, dx, dy, drag.region.lock_aspect) : moveRegion(drag.region, dx, dy)
    s.updateRegion(drag.id, geometry)
  }
  return <div className={styles.renderOverlay} ref={holder} style={s.open && s.preview && video ? { background: 'black' } : undefined}>
    {s.open && bounds && <div className={styles.renderPicture} style={{ left: bounds.left, top: bounds.top, width: bounds.width, height: bounds.height }}>
      <canvas ref={frame} className={styles.renderCanvas} />
      <div ref={assHost} className={styles.renderAss} />
    </div>}
    {s.open && editBounds && <div className={styles.renderPicture} style={{ left: editBounds.left, top: editBounds.top, width: editBounds.width, height: editBounds.height }}>
      {s.editing && canEdit && s.recipe.regions.map(region => <div key={region.id} role="button" tabIndex={0}
        aria-label={`选择${region.effect_type}图层`} aria-pressed={region.id === s.selected}
        className={styles.renderRegion} data-selected={region.id === s.selected} data-inactive={!regionActive(region.intervals, at)}
        style={{ left: `${region.x * 100}%`, top: `${region.y * 100}%`, width: `${region.width * 100}%`, height: `${region.height * 100}%` }}
        onClick={() => s.setSelected(region.id)} onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); s.setSelected(region.id) } }}
        onPointerDown={e => pointerDown(e, region, null)} onPointerMove={pointerMove} onPointerUp={() => { regionDrag.current = null }} onPointerCancel={() => { regionDrag.current = null }}>
        <span>{region.effect_type === 'image' ? '图片' : region.effect_type === 'mosaic' ? '马赛克' : region.effect_type === 'blur' ? '模糊' : '纯色'}</span>
        {region.id === s.selected && handles.map(handle => <div key={handle} className={styles.renderResize} data-handle={handle}
          style={{ left: `${handlePoint[handle][0] * 100}%`, top: `${handlePoint[handle][1] * 100}%` }}
          aria-label={`缩放 ${handle}`} onPointerDown={e => pointerDown(e, region, handle)} />)}
      </div>)}
    </div>}
    {s.open && (maskError || assError || assLoading) && <div className={styles.renderPreviewNotice} role={maskError || assError ? 'alert' : 'status'}>
      {maskError || assError || '正在生成与导出一致的 ASS 预览…'}
    </div>}
    {s.open && !video && <div className={styles.renderPreviewNotice}>打开回看画面后可预览和拖拽遮挡。</div>}
    {s.open && video && !('videoWidth' in video) && !maskError && !assError && !assLoading && <div className={styles.renderPreviewNotice}>回看连接已暂停；可继续编辑当前帧，点击播放恢复。</div>}
    {s.open && s.preview && danmakuEnabled && !assError && !assLoading && estimatedTiming && <div className={styles.renderTimingNotice}>历史弹幕使用估算起点；可在面板中校准偏移。</div>}
  </div>
}
