'use client'

import React, { useId, useLayoutEffect, useRef, useState } from 'react'
import { Button, Input, Select, withField } from '@douyinfe/semi-ui'
import { IconDownloadStroked, IconCloudUploadStroked, IconCodeStroked, IconFlowChartStroked, IconCheckCircleStroked, IconPlayCircle, IconLayers, IconExpand, IconShrink } from '@douyinfe/semi-icons'
import type { CommonFieldProps } from '@douyinfe/semi-ui/lib/es/form/interface'
import { defaultRecipe } from '../lib/renders'
import type { PostprocessorFormStep } from '../lib/postprocessor'
import { PROCESSING_STAGES, moveProcessingStep, stageSteps, type ProcessingFlow, type ProcessingStage } from '../lib/processing-flow'
import RecipeFields from './ProcessingRecipeFields'
import { useFlowViewport } from './useFlowViewport'
import styles from './ProcessingFlowEditor.module.css'

export type FlowNode = 'start' | 'download' | 'upload' | ProcessingStage
export const FLOW_NODES: Record<FlowNode, { title: string; description: string; icon: React.ReactNode }> = {
  start: { title: '直播开始', description: '设置录播备注、直播链接、文件名模板和视频格式。', icon: <IconPlayCircle /> },
  download: { title: '下载', description: '设置录制时间范围、不录制关键词与 ffmpeg 参数。', icon: <IconDownloadStroked /> },
  upload: { title: '上传', description: '为这个直播间选择投稿模板，分段上传后统一投稿。', icon: <IconCloudUploadStroked /> },
  preprocessor: { title: '下载前处理', description: '下载直播前触发，按顺序执行 run 命令。', icon: <IconCodeStroked /> },
  segment_processor: { title: '分段时后处理', description: '每个分段上传前触发，按顺序执行操作。弹幕与遮挡可在同一个 render 步骤中完成。', icon: <IconFlowChartStroked /> },
  downloaded_processor: { title: '下载后处理', description: '整场下载和分段处理完成后执行一次，接收实际文件列表；配置后普通录制将等待此步骤完成再上传。', icon: <IconCodeStroked /> },
  postprocessor: { title: '后处理', description: '上传、投稿成功后处理视频文件；未配置上传时，在分段处理成功后执行。默认 rm 删除文件，mv 将文件移动到指定目录。', icon: <IconCheckCircleStroked /> },
}
export const isInnerNode = (node: FlowNode) => node === 'download' || node === 'segment_processor' || node === 'upload'
const COMMANDS: Record<string, { label: string; placeholder?: string; description: string }> = {
  run: { label: 'run（运行）', placeholder: 'echo hello!', description: '使用当前运行用户在 shell 执行命令，例如 echo hello!。' },
  mv: { label: 'mv（移动到）', placeholder: 'backup/', description: '移动文件到指定目录，例如 backup/。' },
  rm: { label: 'rm（删除文件）', description: '删除本段视频、弹幕，以及压制或转封装留下的原片和中间文件；有保留期或剪辑引用时延后清理。' },
  remux: { label: 'remux（转封装）', placeholder: 'mp4', description: '将视频转封装为 MP4，保留原文件。' },
  render: { label: 'render（画面与弹幕）', description: '将弹幕烧录、遮挡等操作应用到视频。' },
  webhook: { label: 'webhook', placeholder: 'https://example.com/webhook', description: '上传结束后 POST 直播与投稿信息（stream_info）及异常（caught_error）。返回 success 后才执行文件后处理；回调失败则保留文件。' },
}
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

/** One persistent world: opening the group moves the camera, never swaps scenes. */
export function ProcessingFlowGraph({ value, selected, onSelect, uploadLabel, expanded, onExpandedChange }: {
  value: ProcessingFlow; selected: FlowNode; onSelect: (node: FlowNode) => void; uploadLabel?: string
  expanded: boolean; onExpandedChange: (expanded: boolean) => void
}) {
  const marker = useId().replace(/:/g, '')
  const { viewportRef, camera, ready, dragging, fit, focus, zoom, handlers } = useFlowViewport()
  const detailed = camera.zoom > 2
  const previousExpanded = useRef(expanded)
  useLayoutEffect(() => {
    if (!ready || previousExpanded.current === expanded) return
    previousExpanded.current = expanded
    if (expanded) focus()
    else fit()
    viewportRef.current?.focus({ preventScroll: true })
  }, [expanded, ready, focus, fit, viewportRef])
  const toggle = (open: boolean) => {
    onExpandedChange(open)
    if (open === expanded) { if (open) focus(); else fit() }
    viewportRef.current?.focus({ preventScroll: true })
    if (open && !isInnerNode(selected)) onSelect('download')
  }
  const select = (key: FlowNode) => {
    onSelect(key)
    if (!isInnerNode(key)) { onExpandedChange(false); fit() }
  }
  // SVG strokes keep fractional widths; CSS borders round to device pixels
  // before the camera transform, making thin borders grow at high zoom.
  const outline = <svg className={styles.nodeOutline} aria-hidden="true"><rect width="100%" height="100%" /></svg>
  const processor = (key: ProcessingStage, className?: string) => <button type="button"
    className={`${styles.edgeProcessor} ${className || ''} ${selected === key ? styles.selected : ''}`}
    aria-label={FLOW_NODES[key].title} title={FLOW_NODES[key].description}
    aria-pressed={selected === key} aria-controls="flow-node-settings" onClick={() => select(key)}>
    <span aria-hidden="true">{FLOW_NODES[key].icon}</span>
    <span>{FLOW_NODES[key].title}</span>
    <span className={styles.count}>{stageSteps(value, key).length}</span>
    {outline}
  </button>
  const node = (key: 'start' | 'download' | 'upload', detail?: string) => <button type="button"
    className={`${styles.node} ${styles[key]} ${selected === key ? styles.selected : ''}`}
    aria-label={FLOW_NODES[key].title} aria-pressed={selected === key} aria-controls="flow-node-settings" onClick={() => select(key)}>
    <span className={styles.nodeIcon} aria-hidden="true">{FLOW_NODES[key].icon}</span>
    <span className={styles.nodeText}><strong>{FLOW_NODES[key].title}</strong>{detail && <small title={detail}>{detail}</small>}</span>
    {selected === key && <span className={styles.selectionMark} aria-hidden="true"><IconCheckCircleStroked /></span>}
    {outline}
  </button>
  const arrow = (id: string, color?: string, inner = false) => <marker id={id} style={{ fill: color }} viewBox="0 0 10 10" refX="9" refY="5"
    markerUnits="userSpaceOnUse" markerWidth={8 / (camera.zoom * (inner ? .1875 : 1))}
    markerHeight={8 / (camera.zoom * (inner ? .1875 : 1))} orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" /></marker>
  const shortcut = (key: 'download' | 'segment_processor' | 'upload', className: string) => <div className={styles.previewItem}>
    <button type="button" id={`${marker}-${key}-shortcut`} className={`${styles.previewButton} ${className} ${selected === key ? styles.selected : ''}`}
      aria-label={FLOW_NODES[key].title} title={FLOW_NODES[key].title}
      aria-pressed={selected === key} aria-controls="flow-node-settings" onClick={() => select(key)}>
      <span aria-hidden="true">{FLOW_NODES[key].icon}</span>
      {outline}
    </button>
    <label htmlFor={`${marker}-${key}-shortcut`} className={styles.previewLabel} aria-hidden="true">{FLOW_NODES[key].title}</label>
  </div>
  return <nav className={styles.canvas} aria-label="直播处理流程" data-detailed={detailed} style={{ '--camera-zoom': camera.zoom } as React.CSSProperties}>
    <div className={styles.canvasHeader}><strong>处理流程</strong></div>
    <div className={styles.graphArea}>
      <div ref={viewportRef} className={styles.graph} role="region" aria-label="流程画布" tabIndex={0}
        aria-describedby={`${marker}-gestures`} data-dragging={dragging} {...handlers}>
        <span id={`${marker}-gestures`} className={styles.srOnly}>滚轮或双指缩放，拖动空白处平移。方向键平移，加减键缩放，Home 键查看全部流程。</span>
        <div className={styles.world} style={{ visibility: ready ? 'visible' : 'hidden', transform: `translate(${camera.x}px, ${camera.y}px) scale(${camera.zoom})` }}>
          <div className={styles.externalFlow}>
            <svg className={styles.connections} aria-hidden="true">
              <defs>{arrow(marker)}{arrow(`${marker}-upload`, 'var(--upload-color)')}</defs>
              <g>
                <path d="M 340 72 V 167" markerEnd={`url(#${marker})`} />
                <path className={styles.downloadRoute} d="M 174 235 H 110 Q 90 235 90 255 V 455" />
                <circle className={styles.downloadRoute} cx="90" cy="455" r={3 / camera.zoom} />
                <path className={styles.uploadRoute} markerEnd={`url(#${marker}-upload)`} d="M 340 303 V 544" />
              </g>
            </svg>
            {node('start')}
            {processor('preprocessor', styles.beforeDownload)}
            {processor('downloaded_processor', styles.afterDownload)}
            {processor('postprocessor', styles.afterUpload)}
            <div className={styles.completion}>
              <span className={styles.nodeIcon} aria-hidden="true"><IconCheckCircleStroked /></span>
              <strong>流程完成</strong>
            </div>
          </div>
          <div className={styles.subgraph}>
            <div className={`${styles.subgraphNode} ${isInnerNode(selected) ? styles.selected : ''}`}
              aria-hidden={detailed} inert={detailed}>
              <button type="button" className={styles.groupExpand} onClick={() => toggle(true)}
                aria-label="展开下载 & 上传" aria-expanded={detailed} aria-controls={`${marker}-inner`}>
                <span className={styles.groupLabel}>
                  <span className={styles.groupIcon} aria-hidden="true"><IconLayers /></span>
                  <span className={styles.expandIcon} aria-hidden="true"><IconExpand /></span>
                </span>
              </button>
              <div className={styles.groupPreview}>
                {shortcut('download', styles.previewDownload)}
                <span className={styles.previewEdge} aria-hidden="true" />
                {shortcut('segment_processor', styles.previewProcessor)}
                <span className={styles.previewEdge} aria-hidden="true" />
                {shortcut('upload', styles.previewUpload)}
              </div>
              {outline}
            </div>
            <div id={`${marker}-inner`} className={styles.innerFlow} aria-hidden={!detailed} inert={!detailed}>
              <svg className={styles.connections} aria-hidden="true">
                <defs>{arrow(`${marker}-inner-arrow`, undefined, true)}</defs>
                <g markerEnd={`url(#${marker}-inner-arrow)`}>
                  <path d="M 166 150 V 232" />
                  <path d="M 288 272 H 302 Q 316 272 316 258 V 124 Q 316 110 302 110 H 288" className={styles.repeatLine} />
                </g>
              </svg>
              {node('download')}
              {processor('segment_processor', styles.segmentProcessor)}
              {node('upload', uploadLabel || '选择投稿模板')}
            </div>
          </div>
        </div>
      </div>
      <div className={styles.cameraTools} role="group" aria-label="画布视图">
        <button type="button" onClick={() => zoom(.8)} aria-label="缩小画布" title="缩小（−）">−</button>
        <span className={styles.zoomValue}>{Math.round(camera.zoom * 100)}%</span>
        <button type="button" onClick={() => zoom(1.25)} aria-label="放大画布" title="放大（+）">+</button>
        <button type="button" onClick={() => toggle(false)} aria-label="查看全部流程" title="查看全部流程（Home）"><IconExpand aria-hidden="true" /></button>
        {detailed && <button type="button" onClick={() => toggle(false)} aria-label="收起下载 & 上传" title="收起下载 & 上传"><IconShrink aria-hidden="true" /></button>}
      </div>
    </div>
  </nav>
}

type Props = { value?: ProcessingFlow; onChange?: (value: ProcessingFlow) => void; disabled?: boolean; stage: ProcessingStage; id?: string }
export function ProcessingFlowEditor({ value = {}, onChange, disabled, stage: key, id }: Props) {
  const labelId = useId()
  const [dragging, setDragging] = useState<number | null>(null)
  const stage = PROCESSING_STAGES.find(s => s.key === key)!
  const steps = stageSteps(value, key)
  const replace = (next: PostprocessorFormStep[]) => onChange?.({ ...value, [key]: next })
  const change = (index: number, next: PostprocessorFormStep) => replace(steps.map((s, i) => i === index ? next : s))
  const move = (from: number, to: number) => onChange?.(moveProcessingStep(value, key, from, to))
  return <section id={id} className={styles.steps} aria-label={`${stage.title}配置`}>
    <div className={styles.stepsToolbar}>
      <Button disabled={disabled} onClick={() => replace([...steps, newStep('run')])}>＋ 添加行</Button></div>
    {!steps.length && <div className={styles.empty}>{key === 'postprocessor' ? '保留文件' : '暂无操作'}</div>}
    <ol className={styles.stepList}>{steps.map((step, index) => <li key={`${key}:${index}`} className={styles.step}
      onDragOver={e => { if (!disabled && dragging !== null) e.preventDefault() }}
      onDrop={e => { e.preventDefault(); if (!disabled && dragging !== null) move(dragging, index); setDragging(null) }}>
      <div className={styles.commandRow}>
        <span className={styles.stepNumber} draggable={!disabled} title="拖动调整顺序"
          onDragStart={e => { e.dataTransfer.setData('text/plain', String(index)); setDragging(index) }} onDragEnd={() => setDragging(null)}>{String(index + 1).padStart(2, '0')}</span>
        <span id={`${labelId}-${index}`} className={styles.srOnly}>{stage.title}操作 {index + 1}</span>
        <Select aria-labelledby={`${labelId}-${index}`} aria-describedby={`${labelId}-${index}-help`} value={step.cmd} disabled={disabled} className={styles.commandSelect}
          onChange={cmd => change(index, newStep(String(cmd)))} optionList={[
            ...(!(stage.commands as readonly string[]).includes(step.cmd || '') ? [{ value: step.cmd, label: `${step.cmd}（不支持）` }] : []),
            ...stage.commands.map(cmd => ({ value: cmd, label: COMMANDS[cmd].label })),
          ]} />
        {step.cmd === 'rm' ? null
          : step.cmd === 'render' ? <span className={styles.parameterHint}>在下方配置画面与弹幕</span>
          : <Input aria-label={`${stage.title}参数 ${index + 1}`} aria-describedby={`${labelId}-${index}-help`} prefix="=" className={styles.parameter} disabled={disabled}
            value={step.value ?? ''} placeholder={COMMANDS[step.cmd || '']?.placeholder} onChange={text => change(index, { ...step, value: text })} />}
        <div className={styles.actions}>
          <Button theme="borderless" aria-label={`操作 ${index + 1} 上移`} disabled={disabled || index === 0} onClick={() => move(index, index - 1)}>↑</Button>
          <Button theme="borderless" aria-label={`操作 ${index + 1} 下移`} disabled={disabled || index === steps.length - 1} onClick={() => move(index, index + 1)}>↓</Button>
          <Button theme="borderless" type="danger" aria-label={`删除操作 ${index + 1}`} disabled={disabled} onClick={() => replace(steps.filter((_, i) => i !== index))}>×</Button>
        </div>
      </div>
      <p id={`${labelId}-${index}-help`} className={styles.commandHelp}>
        {COMMANDS[step.cmd || '']?.description}
        {step.cmd === 'run' && (key === 'segment_processor' || key === 'postprocessor') && ' 视频文件路径作为标准输入传入。'}
      </p>
      {step.cmd === 'run' && key === 'downloaded_processor' && <label>
        <input type="checkbox" checked={step.json === true} disabled={disabled}
          onChange={e => change(index, { ...step, json: e.target.checked })} />
        使用 JSON 标准输入（主播、地址、标题、起止时间和视频列表）
      </label>}
      {step.cmd === 'render' && <fieldset disabled={disabled} className={styles.fields}><RecipeFields step={step} onChange={next => change(index, next)} /></fieldset>}
    </li>)}</ol>
  </section>
}
const Wrapped = withField(ProcessingFlowEditor)
export default function ProcessingFlowField(props: Omit<Props, 'value'> & CommonFieldProps) {
  // StreamerEditor validates all stages on save; an empty newly added row is still being edited.
  return <Wrapped {...props} />
}
