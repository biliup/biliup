import type { NormalizedPoint } from './mosaic-config'

export type FrameDimensions = { width: number; height: number }
export type FrameBounds = FrameDimensions & { left: number; top: number }

/** The displayed canvas has the JPEG's actual aspect, with no fitted-image margins. */
export function frameAspectRatio({ width, height }: FrameDimensions): string | null {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0) return null
  return `${width} / ${height}`
}

/** Map the image's displayed bounds directly to the persisted video percentages. */
export function normalizedFramePoint(clientX: number, clientY: number, bounds: FrameBounds): NormalizedPoint | null {
  if (![clientX, clientY, bounds.left, bounds.top, bounds.width, bounds.height].every(Number.isFinite) ||
      bounds.width <= 0 || bounds.height <= 0) return null
  const clamp = (value: number) => Math.max(0, Math.min(1, value))
  return {
    x: clamp((clientX - bounds.left) / bounds.width),
    y: clamp((clientY - bounds.top) / bounds.height),
  }
}

/** Do not expose arbitrary response bodies, stream URLs or platform authentication errors. */
export function mosaicFrameFailure(status: number): string {
  if (status === 401) return '登录会话已失效，请重新登录后抓取直播画面。'
  if (status === 403) return '没有抓取直播画面的权限，请联系管理员。'
  if (status === 404) return '找不到这个直播间，请刷新主播列表后重试。'
  if (status === 409) return '当前直播间没有可用的直播画面，请确认已开播后重试。'
  if (status === 415) return '当前直播流暂不支持抓帧，请检查下载器和预览支持。'
  if (status === 429) return '抓帧请求过于频繁，请稍后重试。'
  if (status === 504) return '抓取直播画面超时，请重试。'
  return '暂时无法抓取直播画面，请检查直播状态和后端日志后重试。'
}
