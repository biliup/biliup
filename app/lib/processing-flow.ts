import { formListToHookStep, hookStepListToForm, type PostprocessorFormStep } from './postprocessor'

export const PROCESSING_STAGES = [
  { key: 'preprocessor', title: '下载前处理', commands: ['run'] },
  { key: 'segment_processor', title: '分段时后处理', commands: ['run', 'remux', 'render'] },
  { key: 'downloaded_processor', title: '下载后处理', commands: ['run'] },
  { key: 'postprocessor', title: '后处理', commands: ['run', 'mv', 'rm', 'webhook'] },
] as const
export type ProcessingStage = typeof PROCESSING_STAGES[number]['key']
export type ProcessingFlow = Partial<Record<ProcessingStage, PostprocessorFormStep[] | null>>

/** UI-only grouping. Persist the existing four hook arrays, never a second workflow. */
export function processingFlowFromEntity(entity?: Partial<Record<ProcessingStage, unknown[] | null>>): ProcessingFlow {
  return Object.fromEntries(PROCESSING_STAGES.map(({ key }) => [key,
    entity?.[key] == null ? entity?.[key] : hookStepListToForm(entity[key]),
  ]))
}

export function stageSteps(flow: ProcessingFlow, stage: ProcessingStage): PostprocessorFormStep[] {
  // None means the server's default cleanup. An explicit [] keeps the files.
  return flow[stage] ?? (stage === 'postprocessor' ? [{ cmd: 'rm' }] : [])
}

/** postprocessor is converted by the existing local/Fleet submit handlers. */
export function processingFlowToForm(flow: ProcessingFlow) {
  return Object.fromEntries(PROCESSING_STAGES.map(({ key }) => [key,
    flow[key] == null || key === 'postprocessor' ? flow[key] : formListToHookStep(flow[key]),
  ]))
}

export function moveProcessingStep(flow: ProcessingFlow, stage: ProcessingStage, from: number, to: number): ProcessingFlow {
  const steps = [...stageSteps(flow, stage)]
  if (from < 0 || to < 0 || from >= steps.length || to >= steps.length || from === to) return flow
  steps.splice(to, 0, steps.splice(from, 1)[0])
  return { ...flow, [stage]: steps }
}

export function validateProcessingFlow(flow?: ProcessingFlow): string {
  if (!flow) return ''
  for (const stage of PROCESSING_STAGES) {
    for (const [index, step] of stageSteps(flow, stage.key).entries()) {
      const prefix = `${stage.title} · 步骤 ${index + 1}：`
      if (!(stage.commands as readonly string[]).includes(step.cmd ?? '')) return `${prefix}此阶段不支持该操作`
      if (step.cmd === 'rm') continue
      if (!step.value?.trim()) return `${prefix}${step.cmd === 'run' ? '请填写命令' : step.cmd === 'mv' ? '请填写目标目录' : '请填写参数'}`
      if (step.cmd === 'webhook') {
        try {
          if (!['http:', 'https:'].includes(new URL(step.value).protocol)) throw new Error()
        } catch { return `${prefix}请填写有效的 HTTP 或 HTTPS 回调地址` }
      }
      if (step.cmd === 'remux' && step.value !== 'mp4') return `${prefix}转封装仅支持 MP4`
      if (step.cmd === 'render') {
        try {
          const recipe = JSON.parse(step.value)
          if (!recipe || typeof recipe !== 'object' || Array.isArray(recipe)) throw new Error()
          const regions = recipe.regions ?? []
          if (!Array.isArray(regions) || regions.length > 32) return `${prefix}最多添加 32 个遮挡区域`
          for (const r of regions) {
            if (!r || typeof r !== 'object') return `${prefix}遮挡区域格式无效`
            if (r.effect_type === 'image') return `${prefix}自动处理不支持场次图片素材，请使用马赛克、模糊或纯色`
            const x = r.x ?? 0, y = r.y ?? 0, w = r.width ?? .2, h = r.height ?? .2
            if (![x, y, w, h].every(Number.isFinite) || x < 0 || y < 0 || w < .001 || h < .001 || x + w > 1.000001 || y + h > 1.000001) {
              return `${prefix}遮挡区域必须完整位于画面内，宽高至少为 0.1%`
            }
          }
        } catch { return `${prefix}画面与弹幕设置必须是 JSON 对象` }
      }
    }
  }
  return ''
}
