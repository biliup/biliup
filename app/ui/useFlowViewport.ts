'use client'

import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import type { KeyboardEvent, PointerEvent } from 'react'
import { fitFlow, focusFlow, pinchCamera, zoomAt, type Camera, type Point, type Viewport } from '../lib/flow-viewport'

export function useFlowViewport() {
  const viewportRef = useRef<HTMLDivElement>(null)
  const cameraRef = useRef<Camera>({ x: 0, y: 0, zoom: 1 })
  const sizeRef = useRef<Viewport>({ width: 0, height: 0 })
  const frame = useRef<number | undefined>(undefined)
  const pointers = useRef(new Map<number, Point>())
  const tap = useRef<{ id: number; origin: Point; button: HTMLButtonElement } | null>(null)
  const space = useRef(false)
  const [camera, setCamera] = useState<Camera>({ x: 0, y: 0, zoom: 1 })
  const [dragging, setDragging] = useState(false)
  const [ready, setReady] = useState(false)

  const stop = useCallback(() => { cancelAnimationFrame(frame.current ?? 0); frame.current = undefined }, [])
  const set = useCallback((next: Camera) => { cameraRef.current = next; setCamera(next) }, [])
  const animate = useCallback((target: Camera) => {
    stop()
    const from = cameraRef.current
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) { set(target); return }
    const start = performance.now()
    const tick = (time: number) => {
      const progress = Math.min(1, (time - start) / 800)
      const t = 1 - Math.pow(1 - progress, 3)
      set({ x: from.x + (target.x - from.x) * t, y: from.y + (target.y - from.y) * t, zoom: from.zoom + (target.zoom - from.zoom) * t })
      frame.current = progress < 1 ? requestAnimationFrame(tick) : undefined
    }
    frame.current = requestAnimationFrame(tick)
  }, [set, stop])

  useLayoutEffect(() => {
    const element = viewportRef.current!
    const observer = new ResizeObserver(([entry]) => {
      const next = { width: entry.contentRect.width, height: entry.contentRect.height }
      if (!next.width || !next.height) return
      stop()
      const previous = sizeRef.current
      set(previous.width ? {
        ...cameraRef.current,
        x: cameraRef.current.x + (next.width - previous.width) / 2,
        y: cameraRef.current.y + (next.height - previous.height) / 2,
      } : fitFlow(next))
      sizeRef.current = next
      setReady(true)
    })
    observer.observe(element)
    return () => { observer.disconnect(); stop() }
  }, [set, stop])

  useEffect(() => {
    const element = viewportRef.current!
    const wheel = (event: WheelEvent) => {
      event.preventDefault()
      stop()
      const rect = element.getBoundingClientRect()
      const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? rect.height : 1
      // Browser trackpad pinch events arrive as ctrl+wheel; both paths keep the pointer anchored.
      const delta = Math.max(-400, Math.min(400, event.deltaY * unit))
      set(zoomAt(cameraRef.current, { x: event.clientX - rect.left, y: event.clientY - rect.top }, cameraRef.current.zoom * Math.exp(-delta * (event.ctrlKey ? .008 : .002))))
    }
    element.addEventListener('wheel', wheel, { passive: false })
    return () => element.removeEventListener('wheel', wheel)
  }, [set, stop])

  const fit = useCallback(() => animate(fitFlow(sizeRef.current)), [animate])
  const focus = useCallback(() => animate(focusFlow(sizeRef.current)), [animate])
  const zoom = useCallback((ratio: number) => {
    const size = sizeRef.current
    animate(zoomAt(cameraRef.current, { x: size.width / 2, y: size.height / 2 }, cameraRef.current.zoom * ratio))
  }, [animate])
  const point = (event: PointerEvent) => {
    const rect = viewportRef.current!.getBoundingClientRect()
    return { x: event.clientX - rect.left, y: event.clientY - rect.top }
  }
  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 && event.button !== 1) return
    if (event.button === 0 && event.pointerType !== 'touch' && !space.current && (event.target as Element).closest('button')) return
    event.preventDefault()
    stop()
    const origin = point(event)
    pointers.current.set(event.pointerId, origin)
    const button = (event.target as Element).closest('button')
    tap.current = event.pointerType === 'touch' && pointers.current.size === 1 && button
      ? { id: event.pointerId, origin, button } : null
    event.currentTarget.setPointerCapture(event.pointerId)
    event.currentTarget.focus({ preventScroll: true })
    setDragging(true)
  }
  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const old = pointers.current.get(event.pointerId)
    if (!old) return
    const before = Array.from(pointers.current.values())
    const next = point(event)
    if (tap.current && Math.hypot(next.x - tap.current.origin.x, next.y - tap.current.origin.y) > 6) tap.current = null
    pointers.current.set(event.pointerId, next)
    const after = Array.from(pointers.current.values())
    if (before.length >= 2) set(pinchCamera(cameraRef.current, before.slice(0, 2) as [Point, Point], after.slice(0, 2) as [Point, Point]))
    else set({ ...cameraRef.current, x: cameraRef.current.x + next.x - old.x, y: cameraRef.current.y + next.y - old.y })
  }
  const onPointerEnd = (event: PointerEvent<HTMLDivElement>) => {
    const button = event.type === 'pointerup' && tap.current?.id === event.pointerId ? tap.current.button : null
    tap.current = null
    pointers.current.delete(event.pointerId)
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId)
    if (!pointers.current.size) setDragging(false)
    button?.click()
  }
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target !== event.currentTarget) return
    if (event.key === ' ') { event.preventDefault(); space.current = true; return }
    if (event.key === '+' || event.key === '=') { event.preventDefault(); zoom(1.25); return }
    if (event.key === '-') { event.preventDefault(); zoom(.8); return }
    if (event.key === '0' || event.key === 'Home') { event.preventDefault(); fit(); return }
    const movement: Record<string, Point> = { ArrowLeft: { x: 64, y: 0 }, ArrowRight: { x: -64, y: 0 }, ArrowUp: { x: 0, y: 64 }, ArrowDown: { x: 0, y: -64 } }
    const delta = movement[event.key]
    if (delta) { event.preventDefault(); stop(); set({ ...cameraRef.current, x: cameraRef.current.x + delta.x, y: cameraRef.current.y + delta.y }) }
  }
  return {
    viewportRef, camera, ready, dragging, fit, focus, zoom,
    handlers: { onPointerDown, onPointerMove, onPointerUp: onPointerEnd, onPointerCancel: onPointerEnd, onLostPointerCapture: onPointerEnd,
      onKeyDown, onKeyUp: () => { space.current = false }, onBlur: () => { space.current = false } },
  }
}
