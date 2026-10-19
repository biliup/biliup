'use client'
import React, { useState } from 'react'
import { defaultRecipe, type RenderRecipe, type MaskRegion, type DanmakuSettings } from '../lib/renders'
import type { PostprocessorFormStep } from '../lib/postprocessor'
import styles from './ProcessingFlowEditor.module.css'

export default function RecipeFields({ step, onChange }: { step: PostprocessorFormStep; onChange: (step: PostprocessorFormStep) => void }) {
  let recipe: RenderRecipe
  try {
    const parsed = JSON.parse(step.value || '{}')
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error()
    const defaults = defaultRecipe()
    recipe = { ...defaults, ...parsed, danmaku: { ...defaults.danmaku, ...parsed.danmaku }, encoding: { ...defaults.encoding, ...parsed.encoding }, regions: parsed.regions ?? [] }
    if (!Array.isArray(recipe.regions) || recipe.regions.some(r => !r || typeof r !== 'object')) throw new Error()
  } catch {
    return <label>修正画面与弹幕设置
      <textarea rows={6} value={step.value ?? ''} onChange={e => onChange({ ...step, value: e.target.value })} />
    </label>
  }
  const update = (next: RenderRecipe) => onChange({ ...step, value: JSON.stringify(next) })
  const danmaku = (patch: Partial<DanmakuSettings>) => update({ ...recipe, danmaku: { ...recipe.danmaku, ...patch } })
  const region = (index: number, patch: Partial<MaskRegion>) => update({ ...recipe, regions: recipe.regions.map((r, i) => i === index ? { ...r, ...patch } : r) })
  const addRegion = () => update({ ...recipe, regions: [...recipe.regions, {
    id: crypto.randomUUID(), effect_type: 'mosaic', x: .1, y: .1, width: .2, height: .2,
    strength: 12, color: '#000000', opacity: 1, asset_id: null, lock_aspect: true, intervals: [],
  }] })
  return <>
    <div className={styles.section}><strong>弹幕</strong>
      <label className={styles.check}><input type="checkbox" checked={recipe.danmaku.enabled} onChange={e => danmaku({ enabled: e.target.checked })} />压制录制弹幕</label>
    </div>
    {recipe.danmaku.enabled && <div className={styles.grid}>
      <NumberField title="字号" value={recipe.danmaku.font_size} min={8} max={200} change={font_size => danmaku({ font_size })} />
      <NumberField title="不透明度 %" value={recipe.danmaku.opacity * 100} min={0} max={100} change={v => danmaku({ opacity: v / 100 })} />
      <NumberField title="描边" value={recipe.danmaku.outline} min={0} max={4} step={.1} change={outline => danmaku({ outline })} />
      <NumberField title="滚动时长（秒）" value={recipe.danmaku.scroll_seconds} min={2} max={60} change={scroll_seconds => danmaku({ scroll_seconds })} />
      <NumberField title="显示区域 %" value={recipe.danmaku.display_area * 100} min={5} max={100} change={v => danmaku({ display_area: v / 100 })} />
      <NumberField title="时间偏移（毫秒）" value={recipe.danmaku.offset_ms} min={-3600000} max={3600000} change={offset_ms => danmaku({ offset_ms })} />
    </div>}
    <div className={styles.section}><strong>画面遮挡</strong>
      <button type="button" className={styles.iconButton} disabled={recipe.regions.length >= 32} onClick={addRegion}>＋ 添加区域</button>
    </div>
    {recipe.regions.length > 0 && <div className={styles.maskPreview} aria-label="遮挡位置示意图">
      {recipe.regions.map((r, i) => <div key={r.id ?? i}
        style={{ left: `${r.x * 100}%`, top: `${r.y * 100}%`, width: `${r.width * 100}%`, height: `${r.height * 100}%` }}>{i + 1}</div>)}
    </div>}
    {recipe.regions.map((r, i) => <div className={styles.region} key={r.id ?? i}>
      <div className={styles.section}><strong>区域 {i + 1}</strong>
        <button type="button" className={`${styles.iconButton} ${styles.delete}`} onClick={() => update({ ...recipe, regions: recipe.regions.filter((_, n) => n !== i) })}>删除区域 {i + 1}</button>
      </div>
      <div className={styles.grid}>
        <label>遮挡效果<select value={r.effect_type ?? 'mosaic'} onChange={e => region(i, { effect_type: e.target.value as MaskRegion['effect_type'] })}>
          <option value="mosaic">马赛克</option><option value="blur">模糊</option><option value="smooth_blur">平滑模糊</option><option value="solid">纯色</option>
          {r.effect_type === 'image' && <option value="image">图片（自动处理不支持）</option>}
        </select></label>
        <NumberField title={`区域 ${i + 1} 强度`} value={r.strength ?? 12} min={1} max={100} change={strength => region(i, { strength })} />
        {(['x', 'y', 'width', 'height'] as const).map((key, n) => <NumberField key={key} title={`区域 ${i + 1} ${['横坐标', '纵坐标', '宽度', '高度'][n]} %`}
          value={(r[key] ?? (n < 2 ? 0 : .2)) * 100} min={n < 2 ? 0 : .1} max={100} step={.1} change={v => region(i, { [key]: v / 100 })} />)}
        <NumberField title={`区域 ${i + 1} 不透明度 %`} value={(r.opacity ?? 1) * 100} min={0} max={100} change={v => region(i, { opacity: v / 100 })} />
        {r.effect_type === 'solid' && <label>遮挡颜色<input type="color" value={r.color ?? '#000000'} onChange={e => region(i, { color: e.target.value })} /></label>}
      </div>
      {!!r.intervals?.length && <p className={styles.help}>含 {r.intervals.length} 个限定时段，在回看编辑器里调整。</p>}
    </div>)}
    <div className={styles.section}><strong>视频编码</strong></div>
    <div className={styles.grid}>
      <label>编码器<select value={recipe.encoding?.encoder ?? 'auto'} onChange={e => update({ ...recipe, encoding: { ...recipe.encoding!, encoder: e.target.value as NonNullable<RenderRecipe['encoding']>['encoder'], device: null } })}>
        <option value="auto">自动选择可用编码器</option><option value="libx264">CPU · libx264</option><option value="h264_nvenc">NVIDIA · NVENC</option><option value="h264_qsv">Intel · QSV</option><option value="h264_vaapi">VAAPI</option><option value="h264_amf">AMD · AMF</option>
      </select></label>
      <label>质量<select value={recipe.encoding?.quality ?? 'balanced'} onChange={e => update({ ...recipe, encoding: { ...recipe.encoding!, quality: e.target.value as NonNullable<RenderRecipe['encoding']>['quality'] } })}>
        <option value="space_saver">体积优先</option><option value="balanced">均衡</option><option value="high_quality">高画质</option>
      </select></label>
    </div>
  </>
}

function NumberField({ title, value, min, max, step = 1, change }: { title: string; value: number; min: number; max: number; step?: number; change: (value: number) => void }) {
  const [draft, setDraft] = useState<{ value: number; text: string } | null>(null)
  return <label>{title}<input type="number" value={draft?.value === value ? draft.text : Math.round(value * 1000) / 1000} min={min} max={max} step={step}
    onChange={e => {
      const n = e.target.valueAsNumber
      setDraft({ value: Number.isFinite(n) ? n : value, text: e.target.value })
      if (Number.isFinite(n)) change(n)
    }} onBlur={e => {
      const n = e.target.valueAsNumber
      change(Math.max(min, Math.min(max, Number.isFinite(n) ? n : value)))
      setDraft(null)
    }} /></label>
}

