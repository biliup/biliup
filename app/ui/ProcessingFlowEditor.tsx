'use client'

import React, { useState } from 'react'
import { withField } from '@douyinfe/semi-ui'
import type { CommonFieldProps } from '@douyinfe/semi-ui/lib/es/form/interface'
import { defaultRecipe, type RenderRecipe, type MaskRegion, type DanmakuSettings } from '../lib/renders'
import type { PostprocessorFormStep } from '../lib/postprocessor'
import {
  PROCESSING_STAGES, moveProcessingStep, stageSteps, validateProcessingFlow,
  type ProcessingFlow, type ProcessingStage,
} from '../lib/processing-flow'
import styles from './ProcessingFlowEditor.module.css'

const COMMANDS: Record<string, { title: string; icon: string; hint?: string }> = {
  run: { title: 'Shell 命令', icon: '>_', hint: '视频路径按行写入标准输入；没有视频的阶段为空。' },
  remux: { title: '转封装', icon: '⇄', hint: '仅 TS / M2TS 生效并重建时间戳，其他格式跳过。' },
  render: { title: '画面与弹幕', icon: '▧', hint: '遮挡与弹幕一次编码完成，需要该主播已开启录制。' },
  mv: { title: '移动文件', icon: '↗' },
  rm: { title: '删除文件', icon: '−', hint: '按素材保留规则清理，被引用的文件延后删除。' },
}

type Props = { value?: ProcessingFlow; onChange?: (value: ProcessingFlow) => void; disabled?: boolean; id?: string }
type Selection = { stage: ProcessingStage; index: number | null }

/** 新步骤的初始值，与后端 HookStep 的四种写法一致。 */
function newStep(cmd: string): PostprocessorFormStep {
  if (cmd === 'rm') return { cmd }
  if (cmd === 'remux') return { cmd, value: 'mp4' }
  if (cmd === 'render') {
    const recipe = defaultRecipe()
    recipe.danmaku.enabled = true
    return { cmd, value: JSON.stringify(recipe) }
  }
  return { cmd, value: '' }
}

function summary(step: PostprocessorFormStep): string {
  if (step.cmd !== 'render') return step.value ?? ''
  try {
    const recipe = JSON.parse(step.value || '{}')
    const parts = [
      recipe.danmaku?.enabled ? '压制弹幕' : '',
      recipe.regions?.length ? `${recipe.regions.length} 个遮挡区域` : '',
    ].filter(Boolean)
    return parts.join(' · ') || '未启用画面效果'
  } catch {
    return '配置待修正'
  }
}

export function ProcessingFlowEditor({ value = {}, onChange, disabled, id }: Props) {
  const [selection, setSelection] = useState<Selection>({ stage: 'segment_processor', index: null })
  const [dragging, setDragging] = useState<{ stage: ProcessingStage; index: number } | null>(null)
  const stage = PROCESSING_STAGES.find(s => s.key === selection.stage)!
  const steps = stageSteps(value, stage.key)
  const index = selection.index
  const step = index === null ? undefined : steps[index]
  const replace = (next: PostprocessorFormStep[]) => onChange?.({ ...value, [stage.key]: next })
  const change = (next: PostprocessorFormStep) => {
    if (index !== null) replace(steps.map((s, i) => i === index ? next : s))
  }
  const add = (cmd: string) => {
    replace([...steps, newStep(cmd)])
    setSelection({ stage: stage.key, index: steps.length })
  }
  const move = (from: number, to: number) => {
    if (from === to) return
    onChange?.(moveProcessingStep(value, stage.key, from, to))
    setSelection({ stage: stage.key, index: to })
  }

  return <div id={id} className={styles.flow} aria-label="处理流程">
    <div className={styles.stages}>
      {PROCESSING_STAGES.map(s => {
        const list = stageSteps(value, s.key)
        return <section key={s.key} className={`${styles.stage} ${selection.stage === s.key ? styles.active : ''}`}>
          <h4 className={styles.stageTitle}>{s.title}</h4>
          <ol className={styles.steps}>
            {list.map((item, i) => <li key={i}>
              <button type="button"
                className={`${styles.step} ${selection.stage === s.key && index === i ? styles.selected : ''}`}
                aria-pressed={selection.stage === s.key && index === i}
                aria-label={`${s.title} 步骤 ${i + 1}：${COMMANDS[item.cmd || '']?.title ?? item.cmd}`}
                draggable={!disabled}
                onDragStart={e => { e.dataTransfer.setData('text/plain', String(i)); e.dataTransfer.effectAllowed = 'move'; setDragging({ stage: s.key, index: i }) }}
                onDragEnd={() => setDragging(null)}
                onDragOver={e => { if (dragging?.stage === s.key) e.preventDefault() }}
                onDrop={e => {
                  e.preventDefault()
                  if (!disabled && dragging?.stage === s.key) move(dragging.index, i)
                  setDragging(null)
                }}
                onClick={() => setSelection({ stage: s.key, index: i })}>
                <span className={`${styles.icon} ${item.cmd === 'rm' ? styles.danger : ''}`} aria-hidden>{COMMANDS[item.cmd || '']?.icon ?? '?'}</span>
                <strong>{COMMANDS[item.cmd || '']?.title ?? item.cmd}</strong>
                <span className={styles.summary} title={summary(item)}>{summary(item)}</span>
              </button>
            </li>)}
            {s.key === 'postprocessor' && value[s.key] == null && <li>
              <span className={styles.defaultStep} title="未配置上传后步骤时，服务按保留规则删除文件">
                <span className={`${styles.icon} ${styles.danger}`} aria-hidden>−</span><strong>默认清理</strong>
              </span>
            </li>}
          </ol>
          <button type="button" className={styles.add} disabled={disabled}
            aria-label={`在${s.title}添加步骤`} onClick={() => setSelection({ stage: s.key, index: null })}>＋</button>
        </section>
      })}
    </div>
    <div className={styles.details}>
      {step ? <>
        <header className={styles.detailHead}>
          <strong>{stage.title}</strong>
          <div className={styles.actions}>
            <button type="button" aria-label="步骤上移" disabled={disabled || index === 0} onClick={() => move(index!, index! - 1)}>↑</button>
            <button type="button" aria-label="步骤下移" disabled={disabled || index === steps.length - 1} onClick={() => move(index!, index! + 1)}>↓</button>
            <button type="button" className={styles.delete} disabled={disabled}
              onClick={() => { replace(steps.filter((_, i) => i !== index)); setSelection({ stage: stage.key, index: null }) }}>删除</button>
          </div>
        </header>
        <label>操作类型<select value={step.cmd} onChange={e => change(newStep(e.target.value))}>
          {!(stage.commands as readonly string[]).includes(step.cmd || '') && <option value={step.cmd}>{step.cmd}（此阶段不支持）</option>}
          {stage.commands.map(cmd => <option key={cmd} value={cmd}>{COMMANDS[cmd].title}</option>)}
        </select></label>
        {COMMANDS[step.cmd || '']?.hint && <p className={styles.hint}>{COMMANDS[step.cmd || '']?.hint}</p>}
        {step.cmd === 'run' && <label>Shell 命令
          <textarea rows={3} value={step.value ?? ''} placeholder="输入要执行的命令" onChange={e => change({ ...step, value: e.target.value })} />
        </label>}
        {step.cmd === 'mv' && <label>目标目录
          <input value={step.value ?? ''} placeholder="backup/" onChange={e => change({ ...step, value: e.target.value })} />
        </label>}
        {step.cmd === 'remux' && <label>目标容器
          <select value={step.value ?? 'mp4'} onChange={e => change({ ...step, value: e.target.value })}>
            <option value="mp4">MP4</option>
          </select>
        </label>}
        {step.cmd === 'render' && <RecipeFields step={step} onChange={change} />}
      </> : <>
        <header className={styles.detailHead}><strong>{stage.title}</strong><span className={styles.count}>{steps.length} 步</span></header>
        <div className={styles.catalog}>
          {stage.commands.map(cmd => <button type="button" disabled={disabled} key={cmd} onClick={() => add(cmd)}>
            <span className={styles.icon} aria-hidden>{COMMANDS[cmd].icon}</span><strong>{COMMANDS[cmd].title}</strong>
          </button>)}
        </div>
      </>}
    </div>
  </div>
}

function RecipeFields({ step, onChange }: { step: PostprocessorFormStep; onChange: (step: PostprocessorFormStep) => void }) {
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
    <div className={styles.groupTitle}><strong>弹幕</strong>
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
    <div className={styles.groupTitle}><strong>画面遮挡</strong>
      <button type="button" disabled={recipe.regions.length >= 32} onClick={addRegion}>＋ 添加区域</button>
    </div>
    {recipe.regions.length > 0 && <div className={styles.maskPreview} aria-label="遮挡位置示意图">
      {recipe.regions.map((r, i) => <div key={r.id ?? i}
        style={{ left: `${r.x * 100}%`, top: `${r.y * 100}%`, width: `${r.width * 100}%`, height: `${r.height * 100}%` }}>{i + 1}</div>)}
    </div>}
    {recipe.regions.map((r, i) => <div className={styles.region} key={r.id ?? i}>
      <div className={styles.groupTitle}><strong>区域 {i + 1}</strong>
        <button type="button" className={styles.delete} onClick={() => update({ ...recipe, regions: recipe.regions.filter((_, n) => n !== i) })}>删除区域 {i + 1}</button>
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
      {!!r.intervals?.length && <p className={styles.hint}>含 {r.intervals.length} 个限定时段，在回看编辑器里调整。</p>}
    </div>)}
    <div className={styles.groupTitle}><strong>视频编码</strong></div>
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

const Wrapped = withField(ProcessingFlowEditor)
export default function ProcessingFlowField(props: Omit<Props, 'value'> & CommonFieldProps) {
  return <Wrapped {...props} validator={validateProcessingFlow} />
}
