/**
 * 后处理步骤的双向转换:表单形态 {cmd, value} ↔ 后端 HookStep 形态。
 *
 * 后端 HookStep 是 #[serde(untagged)] 枚举(声明顺序即匹配顺序):
 *   Run { run: string } | Move { mv: string } | Remux { remux: string } | Remove("rm")
 * 表单(ArrayField)中每个步骤的字段是 cmd(Select) + value(Input)。
 *
 * 严禁把 {mv: target} 改写成 {run: "mv target"} —— untagged 枚举先匹配 Run,
 * 会经 sh -c / cmd /C 走 Shell 执行,路径里的特殊字符会被解释,且失去
 * Rust fs::rename 的安全移动语义。
 */

export interface PostprocessorFormStep {
  cmd?: string
  value?: string
}

/** 后端 HookStep → 表单 {cmd, value}(回显 / 编辑用) */
export function hookStepToForm(step: unknown): PostprocessorFormStep {
  if (typeof step === 'string') {
    // Remove("rm")
    return { cmd: 'rm' }
  }
  if (step && typeof step === 'object') {
    const s = step as Record<string, unknown>
    // 已是表单形态
    if (typeof s.cmd === 'string') {
      return { cmd: s.cmd, value: typeof s.value === 'string' ? s.value : undefined }
    }
    // {run}/{mv}/{remux} 等单键对象 → cmd/key, value/value
    const keys = Object.keys(s).filter(k => s[k] !== undefined && s[k] !== null && s[k] !== '')
    if (keys.length > 0) {
      return { cmd: keys[0], value: String(s[keys[0]]) }
    }
  }
  return { cmd: 'run', value: '' }
}

/** 表单 {cmd, value} → 后端 HookStep(提交前用) */
export function formToHookStep(step: PostprocessorFormStep): unknown {
  const cmd = step && step.cmd
  const value = step && step.value !== undefined ? step.value : ''
  switch (cmd) {
    case 'rm':
      // Remove("rm")
      return 'rm'
    case 'run':
      return { run: value }
    case 'mv':
      return { mv: value }
    case 'remux':
      return { remux: value }
    default:
      // webhook 等后端暂不支持的形态:原样返回,不静默丢弃
      return step
  }
}

/** 整段转换:后端 HookStep[] → 表单步骤 */
export function hookStepListToForm(list: unknown[] | undefined | null): PostprocessorFormStep[] {
  return (list || []).map(hookStepToForm)
}

/** 整段转换:表单步骤 → 后端 HookStep[] */
export function formListToHookStep(list: PostprocessorFormStep[] | undefined | null): unknown[] {
  return (list || []).map(formToHookStep)
}