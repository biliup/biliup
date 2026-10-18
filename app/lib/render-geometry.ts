/** Coordinates are normalized to the actual picture, never the player's letterbox. */
export interface RenderRect { x: number; y: number; width: number; height: number }
export type ResizeHandle = 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w' | 'nw'
export interface PictureBounds { left: number; top: number; width: number; height: number }
const clamp = (n: number, min: number, max: number) => Math.min(max, Math.max(min, n))

export function pictureBounds(box: PictureBounds, videoWidth: number, videoHeight: number): PictureBounds | null {
  if (![box.left, box.top, box.width, box.height, videoWidth, videoHeight].every(Number.isFinite)
    || Math.min(box.width, box.height, videoWidth, videoHeight) <= 0) return null
  const scale = Math.min(box.width / videoWidth, box.height / videoHeight)
  const width = videoWidth * scale, height = videoHeight * scale
  return { left: box.left + (box.width - width) / 2, top: box.top + (box.height - height) / 2, width, height }
}

export function moveRegion(rect: RenderRect, dx: number, dy: number): RenderRect {
  return { ...rect, x: clamp(rect.x + dx, 0, 1 - rect.width), y: clamp(rect.y + dy, 0, 1 - rect.height) }
}

/** Keep the opposite edge/corner fixed. Edge handles with aspect lock grow around the other axis' centre. */
export function resizeRegion(rect: RenderRect, handle: ResizeHandle, dx: number, dy: number, lock: boolean): RenderRect {
  const west = handle.includes('w'), east = handle.includes('e'), north = handle.includes('n'), south = handle.includes('s')
  let width = clamp(rect.width + (west ? -dx : east ? dx : 0), 0.001, 1)
  let height = clamp(rect.height + (north ? -dy : south ? dy : 0), 0.001, 1)
  if (lock) {
    const ratio = rect.width / rect.height
    if ((west || east) && !(north || south)) height = width / ratio
    else if ((north || south) && !(west || east)) width = height * ratio
    else if (Math.abs(width / rect.width - 1) >= Math.abs(height / rect.height - 1)) height = width / ratio
    else width = height * ratio
    width = Math.max(width, 0.001, 0.001 * ratio)
    height = width / ratio
    const anchorX = west ? rect.x + rect.width : east ? rect.x : rect.x + rect.width / 2
    const anchorY = north ? rect.y + rect.height : south ? rect.y : rect.y + rect.height / 2
    const maxWidth = west ? anchorX : east ? 1 - anchorX : Math.min(anchorX, 1 - anchorX) * 2
    const maxHeight = north ? anchorY : south ? 1 - anchorY : Math.min(anchorY, 1 - anchorY) * 2
    const scale = Math.min(1, maxWidth / width, maxHeight / height)
    width *= scale; height *= scale
    return { x: west ? anchorX - width : east ? anchorX : anchorX - width / 2,
      y: north ? anchorY - height : south ? anchorY : anchorY - height / 2, width, height }
  }
  const x = west ? clamp(rect.x + rect.width - width, 0, rect.x + rect.width - 0.001) : rect.x
  const y = north ? clamp(rect.y + rect.height - height, 0, rect.y + rect.height - 0.001) : rect.y
  width = west ? rect.x + rect.width - x : Math.min(width, 1 - x)
  height = north ? rect.y + rect.height - y : Math.min(height, 1 - y)
  return { x, y, width, height }
}

export function regionActive(intervals: { from_ms: number; to_ms: number }[], ms: number): boolean {
  return intervals.length === 0 || intervals.some(t => ms >= t.from_ms && ms < t.to_ms)
}

/** Convert a normalized image size while retaining its pixel aspect on the reference video. */
export function imageRegionSize(imageWidth: number, imageHeight: number, videoWidth: number, videoHeight: number) {
  const width = 0.25
  const height = width * videoWidth * imageHeight / (videoHeight * imageWidth)
  const scale = Math.min(1, 0.8 / height)
  return { width: width * scale, height: height * scale }
}
