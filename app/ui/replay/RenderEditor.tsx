'use client'
import React, { useEffect, useRef, useState } from 'react'
import { Button, Progress, Toast } from '@douyinfe/semi-ui'
import {
  apiResourceUrl, createRender, defaultRecipe, deleteAsset, renderJobAction, saveRecipe, uploadAsset,
  useRenderAssets, useRenderCapabilities, useRenderJobs, useRenderSettings, type DanmakuSettings, type EffectType,
  type MaskRegion, type RenderAsset, type RenderRecipe,
} from '@/app/lib/renders'
import { imageRegionSize } from '@/app/lib/render-geometry'
import type { SegmentView } from '@/app/lib/sessions'
import { formatSessionTime } from '@/app/lib/markers'
import styles from './replay.module.scss'

export function useRenderEditor(sessionId: number, allowed: boolean) {
  const settings = useRenderSettings(allowed ? sessionId : null)
  const assets = useRenderAssets(allowed ? sessionId : null)
  const jobs = useRenderJobs(allowed ? sessionId : null)
  const capabilities = useRenderCapabilities(allowed)
  const [recipe, setRecipe] = useState<RenderRecipe>(defaultRecipe)
  const currentRecipe = useRef(recipe)
  useEffect(() => { currentRecipe.current = recipe }, [recipe])
  const [selected, setSelected] = useState<string | null>(null)
  const [open, setOpen] = useState(false)
  const [editing, setEditing] = useState(false)
  const [preview, setPreview] = useState(true)
  const [dirty, setDirty] = useState(false)
  const loaded = useRef<number | null>(null)
  useEffect(() => {
    if (settings.data && loaded.current !== sessionId) {
      loaded.current = sessionId; setRecipe(settings.data); setDirty(false)
    }
  }, [settings.data, sessionId])
  const change = (next: RenderRecipe | ((current: RenderRecipe) => RenderRecipe)) => {
    setRecipe(next); setDirty(true)
  }
  const updateRegion = (id: string, patch: Partial<MaskRegion>) => {
    change(current => ({ ...current, regions: current.regions.map(r => r.id === id ? { ...r, ...patch } : r) }))
  }
  const save = async () => {
    const snapshot = recipe
    await saveRecipe(sessionId, snapshot)
    // A drag during the request is still unsaved.
    if (currentRecipe.current === snapshot) setDirty(false)
  }
  return { sessionId, settings, assets, jobs, capabilities, recipe, change, updateRegion, selected, setSelected,
    open, setOpen, editing, setEditing, preview, setPreview, dirty, save }
}
export type RenderEditorState = ReturnType<typeof useRenderEditor>
const errorText = (e: unknown) => e instanceof Error ? e.message : String(e)

function NumberField({ label, value, min, max, step = 1, disabled, onChange }: {
  label: string; value: number; min?: number; max?: number; step?: number; disabled: boolean; onChange: (n: number) => void
}) {
  return <label className={styles.renderField}><span>{label}</span><input type="number" value={Number(value.toFixed(6))}
    min={min} max={max} step={step} disabled={disabled} onChange={event => {
      if (event.target.value === '') return
      const n = event.target.valueAsNumber
      if (Number.isFinite(n)) onChange(Math.min(max ?? Infinity, Math.max(min ?? -Infinity, n)))
    }} /></label>
}

export default function RenderEditor({ state: s, canEdit, canDownload, recording, segments, duration, current,
  dimensions, onExportClip, exportReason, exporting }: {
  state: RenderEditorState; canEdit: boolean; canDownload: boolean; recording: boolean
  segments: SegmentView[]; duration: number; current: () => number; dimensions: { width: number; height: number } | null
  onExportClip: () => Promise<void>; exportReason: string | null; exporting: boolean
}) {
  const [busy, setBusy] = useState(false)
  const [view, setView] = useState<string | null>(null)
  const file = useRef<HTMLInputElement>(null)
  const region = s.recipe.regions.find(r => r.id === s.selected)
  const run = async (action: () => Promise<unknown>, message?: string) => {
    setBusy(true)
    try { await action(); if (message) Toast.success(message) }
    catch (e) { Toast.error({ content: errorText(e), duration: 6 }) }
    finally { setBusy(false) }
  }
  const add = (effect: EffectType, asset?: RenderAsset) => {
    const size = asset ? imageRegionSize(asset.width, asset.height, dimensions?.width ?? 1920, dimensions?.height ?? 1080)
      : { width: 0.2, height: 0.2 }
    const newRegion: MaskRegion = { id: crypto.randomUUID(), effect_type: effect, x: 0.1, y: 0.1, ...size,
      strength: 12, color: '#000000', opacity: 1, asset_id: asset?.id ?? null, lock_aspect: true, intervals: [] }
    s.change(current => ({ ...current, regions: [...current.regions, newRegion] }))
    s.setSelected(newRegion.id); s.setEditing(true)
  }
  const setDanmaku = (patch: Partial<DanmakuSettings>) => s.change(r => ({ ...r, danmaku: { ...r.danmaku, ...patch } }))
  const reorder = (id: string, dir: number) => s.change(current => {
    const regions = [...current.regions], index = regions.findIndex(r => r.id === id), target = index + dir
    if (target < 0 || target >= regions.length) return current
    ;[regions[index], regions[target]] = [regions[target], regions[index]]
    return { ...current, regions }
  })
  const disabled = !canEdit || !s.settings.data || !!s.settings.error
  const d = s.recipe.danmaku
  const cap = s.capabilities.data?.tools
  const capabilityReason = s.capabilities.error ? `无法检查合成工具：${errorText(s.capabilities.error)}`
    : !cap ? '正在检查服务器的合成工具…'
    : !cap.ffmpeg || !cap.libx264 ? (cap.error ?? '服务器需要安装支持 libx264 的 FFmpeg')
    : d.enabled && d.font !== 'Noto Sans CJK SC' ? '请将弹幕字体改为共享的随包中文字体，以保证预览和导出一致'
    : d.enabled && (!cap.libass || !cap.danmaku_factory || !cap.font) ? (cap.error ?? '弹幕合成需要 libass、DanmakuFactory 和随包中文字体')
    : null
  return <section className={styles.renderPanel} aria-label="画面与弹幕">
    <div className={styles.renderToolbar}>
      <strong>画面与弹幕</strong>
      <Button size="small" theme={s.open ? 'solid' : 'light'} onClick={() => s.setOpen(!s.open)}>{s.open ? '收起' : '编辑与合成'}</Button>
      {s.open && <><label><input type="checkbox" checked={s.preview} onChange={e => s.setPreview(e.target.checked)} />实时预览</label>
        <label><input type="checkbox" checked={s.editing} disabled={disabled} onChange={e => s.setEditing(e.target.checked)} />画面拖拽编辑</label>
        <Button size="small" disabled={disabled || !s.dirty || busy} loading={busy} onClick={() => run(s.save, '画面与弹幕配置已保存')}>保存{s.dirty ? ' *' : ''}</Button></>}
    </div>
    {s.open && <>
      {s.settings.error ? <p className={styles.renderError}>无法读取合成配置：{errorText(s.settings.error)}</p>
        : !s.settings.data ? <p>正在读取合成配置…</p> : null}
      {!canEdit && <p className={styles.renderHint}>需要 clip.edit 权限才能修改配置和创建合成任务。</p>}
      <p className={styles.renderHint}>在纯净画面上编辑，导出依次应用遮挡和弹幕。配置中的位置以视频画面百分比计；正偏移表示弹幕延后。原素材会保留。</p>
      {capabilityReason && <p className={styles.renderError}>{capabilityReason}。安装随包合成工具或配置 ffmpeg_path、danmaku_factory_path / BILIUP_RENDER_TOOLS_DIR 后，<button onClick={() => s.capabilities.mutate()}>重新检查</button>。</p>}
      <div className={styles.renderColumns}>
        <div>
          <div className={styles.renderToolbar}>
            <strong>遮挡图层 · 从下至上</strong>
            {(['mosaic', 'blur', 'solid'] as const).map((type, i) => <Button key={type} size="small" disabled={disabled || s.recipe.regions.length >= 32}
              onClick={() => add(type)}>{['马赛克', '模糊', '纯色'][i]}</Button>)}
            <Button size="small" disabled={disabled || busy || s.recipe.regions.length >= 32} onClick={() => file.current?.click()}>上传图片</Button>
            <input ref={file} type="file" accept="image/png,image/jpeg" hidden onChange={e => {
              const uploaded = e.target.files?.[0]; e.target.value = ''
              if (uploaded) void run(async () => add('image', await uploadAsset(s.sessionId, uploaded)))
            }} />
          </div>
          <div className={styles.renderAssets}>
            {s.assets.data?.assets.map(asset => <div key={asset.id}>
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <button disabled={disabled || s.recipe.regions.length >= 32} onClick={() => add('image', asset)} title={`添加图片 #${asset.id}`}><img src={apiResourceUrl(asset.url)} alt={`遮挡图片 #${asset.id}`} /></button>
              <Button size="small" disabled={disabled || busy || s.recipe.regions.some(r => r.asset_id === asset.id)} onClick={() => run(() => deleteAsset(s.sessionId, asset.id))}>删除</Button>
            </div>)}
          </div>
          {s.assets.error && <p className={styles.renderError}>图片列表加载失败：{errorText(s.assets.error)}</p>}
          <ol className={styles.renderLayers}>
            {s.recipe.regions.map((r, index) => <li key={r.id} data-selected={r.id === s.selected}>
              <button onClick={() => s.setSelected(r.id)}>{index + 1}. {({ mosaic: '马赛克', blur: '模糊', solid: '纯色', image: '图片' })[r.effect_type]} {r.asset_id ? `#${r.asset_id}` : ''}</button>
              <Button size="small" disabled={disabled || index === 0} onClick={() => reorder(r.id, -1)}>下移</Button>
              <Button size="small" disabled={disabled || index === s.recipe.regions.length - 1} onClick={() => reorder(r.id, 1)}>上移</Button>
              <Button size="small" type="danger" disabled={disabled} onClick={() => s.change(current => ({ ...current, regions: current.regions.filter(x => x.id !== r.id) }))}>移除</Button>
            </li>)}
          </ol>
          {region && <fieldset disabled={disabled} className={styles.renderFields}>
            <legend>选中图层 · 拖拽移动，八个控制柄缩放</legend>
            <NumberField label="左侧 %" value={region.x * 100} min={0} max={(1 - region.width) * 100} step={0.1} disabled={disabled} onChange={x => s.updateRegion(region.id, { x: x / 100 })} />
            <NumberField label="顶部 %" value={region.y * 100} min={0} max={(1 - region.height) * 100} step={0.1} disabled={disabled} onChange={y => s.updateRegion(region.id, { y: y / 100 })} />
            <NumberField label="宽度 %" value={region.width * 100} min={0.1} max={(1 - region.x) * 100} step={0.1} disabled={disabled} onChange={n => {
              let width = n / 100, height = region.height
              if (region.lock_aspect) { height = width * region.height / region.width; if (height > 1 - region.y) { height = 1 - region.y; width = height * region.width / region.height } }
              s.updateRegion(region.id, { width, height })
            }} />
            <NumberField label="高度 %" value={region.height * 100} min={0.1} max={(1 - region.y) * 100} step={0.1} disabled={disabled} onChange={n => {
              let height = n / 100, width = region.width
              if (region.lock_aspect) { width = height * region.width / region.height; if (width > 1 - region.x) { width = 1 - region.x; height = width * region.height / region.width } }
              s.updateRegion(region.id, { width, height })
            }} />
            <label className={styles.renderField}><span>锁定比例</span><input type="checkbox" checked={region.lock_aspect} onChange={e => s.updateRegion(region.id, { lock_aspect: e.target.checked })} /></label>
            <NumberField label="不透明度 %" value={region.opacity * 100} min={0} max={100} disabled={disabled} onChange={n => s.updateRegion(region.id, { opacity: n / 100 })} />
            {['blur', 'mosaic'].includes(region.effect_type) && <NumberField label="强度" value={region.strength} min={1} max={100} disabled={disabled} onChange={strength => s.updateRegion(region.id, { strength })} />}
            {region.effect_type === 'solid' && <label className={styles.renderField}><span>颜色</span><input type="color" value={region.color} onChange={e => s.updateRegion(region.id, { color: e.target.value })} /></label>}
            <div className={styles.renderIntervals}>
              <strong>生效时段（场次毫秒，终点不包含）</strong>
              {region.intervals.length === 0 && <span>全程生效</span>}
              {region.intervals.map((interval, index) => <div key={index}>
                <NumberField label="起点" value={interval.from_ms} min={0} max={interval.to_ms - 1} disabled={disabled} onChange={from_ms => s.updateRegion(region.id, { intervals: region.intervals.map((x, i) => i === index ? { ...x, from_ms: Math.round(from_ms) } : x) })} />
                <Button size="small" disabled={disabled} onClick={() => s.updateRegion(region.id, { intervals: region.intervals.map((x, i) => i === index ? { from_ms: Math.round(current()), to_ms: Math.max(x.to_ms, Math.round(current()) + 1) } : x) })}>当前作起点</Button>
                <NumberField label="终点" value={interval.to_ms} min={interval.from_ms + 1} disabled={disabled} onChange={to_ms => s.updateRegion(region.id, { intervals: region.intervals.map((x, i) => i === index ? { ...x, to_ms: Math.round(to_ms) } : x) })} />
                <Button size="small" disabled={disabled || current() <= interval.from_ms} onClick={() => s.updateRegion(region.id, { intervals: region.intervals.map((x, i) => i === index ? { ...x, to_ms: Math.round(current()) } : x) })}>当前作终点</Button>
                <Button size="small" type="danger" disabled={disabled} onClick={() => s.updateRegion(region.id, { intervals: region.intervals.filter((_, i) => i !== index) })}>删除时段</Button>
              </div>)}
              <Button size="small" disabled={disabled} onClick={() => { const from = Math.round(current()); s.updateRegion(region.id, { intervals: [...region.intervals, { from_ms: from, to_ms: Math.max(duration, from + 1) }] }) }}>添加时段</Button>
              <Button size="small" disabled={disabled || region.intervals.length === 0} onClick={() => s.updateRegion(region.id, { intervals: [] })}>改为全程</Button>
            </div>
          </fieldset>}
        </div>
        <div>
          <strong>弹幕</strong>
          <fieldset className={styles.renderFields} disabled={disabled}>
            <label className={styles.renderField}><span>压制弹幕</span><input type="checkbox" checked={d.enabled} onChange={e => setDanmaku({ enabled: e.target.checked })} /></label>
            <label className={styles.renderField}><span>共享字体</span><select value={d.font} onChange={e => setDanmaku({ font: e.target.value })}>
              {d.font !== 'Noto Sans CJK SC' && <option value={d.font} disabled>{d.font}（旧配置，需改为随包字体）</option>}
              <option value="Noto Sans CJK SC">Noto Sans CJK SC（随包中文字体）</option>
            </select></label>
            <NumberField label="字号" value={d.font_size} min={8} max={200} disabled={disabled} onChange={font_size => setDanmaku({ font_size })} />
            <NumberField label="不透明度 %" value={d.opacity * 100} min={0} max={100} disabled={disabled} onChange={n => setDanmaku({ opacity: n / 100 })} />
            <NumberField label="描边" value={d.outline} min={0} max={4} step={0.1} disabled={disabled} onChange={outline => setDanmaku({ outline })} />
            <NumberField label="滚动秒数" value={d.scroll_seconds} min={2} max={60} step={0.1} disabled={disabled} onChange={scroll_seconds => setDanmaku({ scroll_seconds })} />
            <NumberField label="显示高度 %" value={d.display_area * 100} min={5} max={100} disabled={disabled} onChange={n => setDanmaku({ display_area: n / 100 })} />
            <NumberField label="密度（-1 自动）" value={d.density} min={-1} max={1000} disabled={disabled} onChange={density => setDanmaku({ density: Math.round(density) })} />
            <NumberField label="场次偏移 ms" value={d.offset_ms} min={-3600000} max={3600000} disabled={disabled} onChange={offset_ms => setDanmaku({ offset_ms: Math.round(offset_ms) })} />
            <details className={styles.renderSegmentOffsets}><summary>分段偏移补偿</summary>
              {segments.map(segment => <NumberField key={segment.id} label={`#${segment.id} · ${formatSessionTime(segment.start_ms)}`} value={d.segment_offsets[String(segment.id)] ?? 0}
                min={-3600000} max={3600000} disabled={disabled} onChange={n => setDanmaku({ segment_offsets: { ...d.segment_offsets, [segment.id]: Math.round(n) } })} />)}
            </details>
          </fieldset>
          <div className={styles.renderToolbar}>
            <Button theme="solid" disabled={disabled || !!exportReason || !!capabilityReason || exporting || busy} loading={exporting} title={exportReason ?? capabilityReason ?? undefined} onClick={() => run(onExportClip)}>合成导出选段</Button>
            <Button disabled={disabled || recording || busy || duration <= 0 || !!capabilityReason} title={recording ? '场次结束后可以导出整场' : capabilityReason ?? undefined} onClick={() => run(async () => { await s.save(); await createRender(s.sessionId, undefined, s.recipe) }, '整场合成任务已加入队列')}>导出整场 MP4</Button>
          </div>
          {exportReason && <p className={styles.renderHint}>{exportReason}</p>}
          <strong>合成任务</strong>
          {s.jobs.error && <p className={styles.renderError}>任务列表加载失败：{errorText(s.jobs.error)}</p>}
          <ul className={styles.renderJobs}>{s.jobs.data?.jobs.map(job => <li key={job.id}>
            <div>#{job.id} {job.clip_id ? `切片 #${job.clip_id}` : '整场'} · {({ queued: '排队中', running: '合成中', ready: '完成', failed: '失败', cancelled: '已取消' })[job.state]} · {job.phase}</div>
            {job.ratio !== null && <Progress percent={Math.round(job.ratio * 100)} showInfo />}
            {job.error && <p className={styles.renderError}>{job.error}</p>}
            <div className={styles.renderToolbar}>
              {['queued', 'running'].includes(job.state) && <Button size="small" disabled={!canEdit || busy} onClick={() => run(() => renderJobAction(s.sessionId, job.id, 'cancel'))}>取消</Button>}
              {['failed', 'cancelled'].includes(job.state) && <Button size="small" disabled={!canEdit || busy} onClick={() => run(() => renderJobAction(s.sessionId, job.id, 'retry'))}>重试原配置</Button>}
              {job.state === 'ready' && job.download_url && canDownload && <>
                <Button size="small" onClick={() => setView(apiResourceUrl(job.download_url!))}>播放</Button>
                <a href={`${apiResourceUrl(job.download_url)}?attachment=true`} download>下载 MP4</a>
              </>}
            </div>
          </li>)}</ul>
          {s.jobs.data?.jobs.length === 0 && <p className={styles.renderHint}>还没有合成任务。</p>}
          {view && <div><Button size="small" onClick={() => setView(null)}>关闭成品预览</Button><video src={view} controls className={styles.renderResult} /></div>}
        </div>
      </div>
    </>}
  </section>
}
