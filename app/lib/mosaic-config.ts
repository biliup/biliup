import type { MosaicConfig, MosaicRegion } from './api-streamer'

export const EMPTY_MOSAIC_CONFIG: MosaicConfig = { enabled: false, regions: [] }
export const MAX_MOSAIC_REGIONS = 32

const isRecord = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === 'object' && !Array.isArray(value)

/** The advanced override field only accepts a JSON object; an empty field clears it. */
export function parseOverrideText(text: string): Record<string, unknown> {
  const value: unknown = text.trim() ? JSON.parse(text) : {}
  if (!isRecord(value)) throw new Error('配置须为 JSON 对象')
  return value
}

/** Preserve other overrides while keeping the visual field and the JSON editor in sync. */
export function updateMosaicOverrideText(text: string, config: MosaicConfig | null | undefined): string {
  const override = parseOverrideText(text)
  if (config === undefined) delete override.mosaic_config
  else override.mosaic_config = config
  return JSON.stringify(override, null, 2)
}

/** Check the wire format before passing persisted or JSON-edited values to the editor. */
export function isMosaicConfig(value: unknown): value is MosaicConfig {
  if (!isRecord(value) || typeof value.enabled !== 'boolean' || !Array.isArray(value.regions)) {
    return false
  }
  if (value.mode != null && typeof value.mode !== 'string') return false
  return value.regions.every(region =>
    isRecord(region) && typeof region.id === 'string' &&
    ['x', 'y', 'width', 'height', 'strength'].every(key =>
      typeof region[key] === 'number' && Number.isFinite(region[key])) &&
    typeof region.effectType === 'string' && ['mosaic', 'blur', 'solid'].includes(region.effectType) &&
    (region.color == null || typeof region.color === 'string'))
}

/** Semi Form validator: null/undefined means that this streamer has no mosaic override. */
export function validateMosaicConfig(value: unknown): string {
  if (value == null) return ''
  if (!isMosaicConfig(value)) return '画面遮挡配置格式不正确，请检查配置 JSON'
  if (value.enabled && value.regions.length === 0) return '启用画面遮挡后，请至少绘制一个区域'
  if (value.regions.length > MAX_MOSAIC_REGIONS) return `最多可配置 ${MAX_MOSAIC_REGIONS} 个遮挡区域`

  const ids = new Set<string>()
  for (const [index, region] of value.regions.entries()) {
    const prefix = `区域 ${index + 1}：`
    if (!region.id || ids.has(region.id)) return prefix + '区域 ID 不能为空或重复'
    ids.add(region.id)
    if (region.x < 0 || region.y < 0 || region.x > 1 || region.y > 1 ||
        region.width < 0.01 || region.height < 0.01 ||
        region.x + region.width > 1 || region.y + region.height > 1) {
      return prefix + '区域不能超出画面，宽高至少为画面的 1%'
    }
    if (!Number.isInteger(region.strength) || region.strength < 0 || region.strength > 0xffffffff) {
      return prefix + '强度必须为非负整数'
    }
    if (region.effectType === 'mosaic' && (region.strength < 4 || region.strength > 64)) {
      return prefix + '马赛克强度应为 4–64'
    }
    if (region.effectType === 'blur' && (region.strength < 1 || region.strength > 100)) {
      return prefix + '模糊强度应为 1–100'
    }
    if (region.color != null && !/^#[\da-f]{6}$/i.test(region.color)) {
      return prefix + '纯色颜色应为 #RRGGBB 格式'
    }
  }
  return ''
}

export type NormalizedPoint = { x: number; y: number }

/** Clamp pointer positions, retaining the original start point even when dragging backwards. */
export function mosaicRectangle(start: NormalizedPoint, end: NormalizedPoint):
  Pick<MosaicRegion, 'x' | 'y' | 'width' | 'height'> {
  const clamp = (number: number) => Math.max(0, Math.min(1, number))
  const startX = clamp(start.x)
  const startY = clamp(start.y)
  const endX = clamp(end.x)
  const endY = clamp(end.y)
  const x = Math.min(startX, endX)
  const y = Math.min(startY, endY)
  return {
    x,
    y,
    width: Math.min(Math.abs(endX - startX), 1 - x),
    height: Math.min(Math.abs(endY - startY), 1 - y),
  }
}
