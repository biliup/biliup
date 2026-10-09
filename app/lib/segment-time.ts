/** The wire format remains segment_time: string | null; an absent room key inherits global config. */
export type SegmentTimeValue = string | null | undefined
export type SegmentTimeScope = 'global' | 'room'

export const SEGMENT_TIME_PRESETS = [
  { value: '00:05:00', label: '5 分钟' },
  { value: '00:10:00', label: '10 分钟' },
  { value: '00:15:00', label: '15 分钟' },
  { value: '00:30:00', label: '30 分钟' },
  { value: '01:00:00', label: '1 小时' },
  { value: '02:00:00', label: '2 小时' },
] as const

/** Match the recorder's existing [HH:]MM:SS[.fraction] / seconds syntax. */
export const parseSegmentTime = (value: unknown): number | undefined => {
  if (typeof value !== 'string') return undefined
  const parts = value.trim().split(':')
  if (parts.length < 1 || parts.length > 3) return undefined
  const last = parts[parts.length - 1]
  if (!/^\d+(?:\.\d*)?$/.test(last)) return undefined
  if (parts.slice(0, -1).some(part => !/^\d+$/.test(part))) return undefined
  const seconds = Number(last)
  const minutes = parts.length > 1 ? Number(parts[parts.length - 2]) : 0
  const hours = parts.length === 3 ? Number(parts[0]) : 0
  if (parts.length > 1 && (minutes >= 60 || seconds >= 60)) return undefined
  if (!Number.isSafeInteger(hours) || !Number.isSafeInteger(minutes)) return undefined
  const total = hours * 3600 + minutes * 60 + seconds
  return Number.isFinite(total) && total <= Number.MAX_SAFE_INTEGER ? total : undefined
}

export const validateSegmentTime = (value: unknown): string => {
  if (value == null || (typeof value === 'string' && value.trim() === '')) return ''
  const seconds = parseSegmentTime(value)
  if (seconds === undefined) return '请输入时:分:秒、分:秒或秒数，例如 00:07:30、7:30、450；分和秒须小于 60'
  if (seconds < 0.000000001) return '自定义分段时长必须大于 0；关闭分段请选择“不按时长分段”'
  return ''
}

export const segmentTimeMode = (value: SegmentTimeValue, scope: SegmentTimeScope): string => {
  if (value === undefined) return scope === 'room' ? 'inherit' : 'off'
  if (value === null) return 'off'
  // Manual override JSON can contain any type. Keep the editor usable so validation can explain it.
  if (typeof value !== 'string') return 'custom'
  if (value.trim() === '' || parseSegmentTime(value) === 0) return 'off'
  const seconds = parseSegmentTime(value)
  return SEGMENT_TIME_PRESETS.find(preset => parseSegmentTime(preset.value) === seconds)?.value ?? 'custom'
}

export const segmentTimeSummary = (value: SegmentTimeValue): string => {
  const seconds = parseSegmentTime(value)
  if (seconds === undefined || seconds <= 0) return ''
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const rest = Number((seconds % 60).toFixed(9))
  return [hours ? `${hours} 小时` : '', minutes ? `${minutes} 分钟` : '', rest ? `${rest} 秒` : '']
    .filter(Boolean).join(' ')
}

/** Keep the distinction between missing/inherited, null/disabled and a custom duration. */
export const updateSegmentTimeOverride = <T extends Record<string, unknown>>(
  previous: T,
  value: SegmentTimeValue,
): T & { segment_time?: string | null } => {
  const next = { ...previous } as T & { segment_time?: string | null }
  if (value === undefined) delete next.segment_time
  else next.segment_time = value === null || value.trim() === '' ? null : value.trim()
  return next
}
