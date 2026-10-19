export type Point = { x: number; y: number }
export type Viewport = { width: number; height: number }
export type Camera = Point & { zoom: number }
export const FLOW_WORLD = { width: 600, height: 660 }
export const FLOW_FOCUS = { x: 340, y: 235 }
export const MIN_ZOOM = .25
export const MAX_ZOOM = 10

export function zoomAt(camera: Camera, anchor: Point, zoom: number): Camera {
  const next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom))
  const ratio = next / camera.zoom
  return { x: anchor.x - (anchor.x - camera.x) * ratio, y: anchor.y - (anchor.y - camera.y) * ratio, zoom: next }
}

export function centerOn(point: Point, viewport: Viewport, zoom: number): Camera {
  return { x: viewport.width / 2 - point.x * zoom, y: viewport.height / 2 - point.y * zoom, zoom }
}

export function fitFlow(viewport: Viewport): Camera {
  const zoom = Math.max(MIN_ZOOM, Math.min(1, (viewport.width - 32) / FLOW_WORLD.width, (viewport.height - 32) / FLOW_WORLD.height))
  return centerOn({ x: FLOW_WORLD.width / 2, y: FLOW_WORLD.height / 2 }, viewport, zoom)
}

export function focusFlow(viewport: Viewport): Camera {
  return centerOn(FLOW_FOCUS, viewport, Math.max(2.1, Math.min(6, (viewport.width - 80) / 62.25, (viewport.height - 100) / 61.875)))
}

export function pinchCamera(camera: Camera, before: [Point, Point], after: [Point, Point]): Camera {
  const center = (points: [Point, Point]) => ({ x: (points[0].x + points[1].x) / 2, y: (points[0].y + points[1].y) / 2 })
  const distance = (points: [Point, Point]) => Math.hypot(points[0].x - points[1].x, points[0].y - points[1].y)
  const oldCenter = center(before)
  const newCenter = center(after)
  const zoomed = zoomAt(camera, oldCenter, camera.zoom * distance(after) / Math.max(1, distance(before)))
  return { ...zoomed, x: zoomed.x + newCenter.x - oldCenter.x, y: zoomed.y + newCenter.y - oldCenter.y }
}
