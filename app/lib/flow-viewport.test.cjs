const assert = require('node:assert/strict')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const { buildSync } = require('esbuild')
const result = buildSync({ entryPoints: [path.join(__dirname, 'flow-viewport.ts')], bundle: true, write: false, format: 'cjs', platform: 'node' })
const loaded = { exports: {} }
vm.runInThisContext(`(function(module, exports) { ${result.outputFiles[0].text}\n})`)(loaded, loaded.exports)
const { zoomAt, pinchCamera, fitFlow, focusFlow, FLOW_WORLD, FLOW_FOCUS, MIN_ZOOM, MAX_ZOOM } = loaded.exports
const close = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1e-8, `${actual} != ${expected}`)
const worldAt = (camera, point) => ({ x: (point.x - camera.x) / camera.zoom, y: (point.y - camera.y) / camera.zoom })

test('wheel zoom keeps the world point under the pointer, including zoom limits', () => {
  const initial = { x: -405, y: 23, zoom: 2.5 }
  const pointer = { x: 323, y: 140 }
  const anchor = worldAt(initial, pointer)
  for (const target of [.001, .8, 4, 200]) {
    const next = zoomAt(initial, pointer, target)
    assert.ok(next.zoom >= MIN_ZOOM && next.zoom <= MAX_ZOOM)
    close(worldAt(next, pointer).x, anchor.x)
    close(worldAt(next, pointer).y, anchor.y)
    const restored = zoomAt(next, pointer, initial.zoom)
    close(restored.x, initial.x)
    close(restored.y, initial.y)
  }
})

test('overview fits the complete main path and focus centers the same persistent group', () => {
  for (const viewport of [{ width: 530, height: 700 }, { width: 350, height: 290 }, { width: 800, height: 960 }]) {
    const fitted = fitFlow(viewport)
    assert.ok(fitted.x >= 0 && fitted.y >= 0)
    assert.ok(fitted.x + FLOW_WORLD.width * fitted.zoom <= viewport.width)
    assert.ok(fitted.y + FLOW_WORLD.height * fitted.zoom <= viewport.height)
    const focused = focusFlow(viewport)
    assert.ok(focused.zoom > fitted.zoom)
    close(focused.x + FLOW_FOCUS.x * focused.zoom, viewport.width / 2)
    close(focused.y + FLOW_FOCUS.y * focused.zoom, viewport.height / 2)
  }
})

test('two finger pinch combines zoom and midpoint translation without losing the anchor', () => {
  const initial = { x: 10, y: -20, zoom: .75 }
  const before = [{ x: 100, y: 200 }, { x: 200, y: 200 }]
  const after = [{ x: 80, y: 260 }, { x: 280, y: 260 }]
  const next = pinchCamera(initial, before, after)
  close(next.zoom, 1.5)
  const oldAnchor = worldAt(initial, { x: 150, y: 200 })
  const newAnchor = worldAt(next, { x: 180, y: 260 })
  close(newAnchor.x, oldAnchor.x)
  close(newAnchor.y, oldAnchor.y)
  const restored = pinchCamera(next, after, before)
  close(restored.x, initial.x)
  close(restored.y, initial.y)
  close(restored.zoom, initial.zoom)
  assert.ok(Object.values(pinchCamera(initial, [before[0], before[0]], after)).every(Number.isFinite))
})

// Exercise the production pointer handlers with touch events, independently of browser mouse emulation.
const hookCode = buildSync({ entryPoints: [path.join(__dirname, '../ui/useFlowViewport.ts')], bundle: true, external: ['react'], write: false, format: 'cjs', platform: 'node' }).outputFiles[0].text
function touchHarness() {
  const effects = [], states = [], captures = new Set(), listeners = {}
  const react = {
    useRef: current => ({ current }), useCallback: fn => fn,
    useEffect: fn => effects.push(fn), useLayoutEffect: fn => effects.push(fn),
    useState: initial => { const index = states.push(initial) - 1; return [initial, next => { states[index] = next }] },
  }
  const module = { exports: {} }
  vm.runInNewContext(hookCode, {
    module, exports: module.exports, require: name => { assert.equal(name, 'react'); return react },
    cancelAnimationFrame() {}, window: { matchMedia: () => ({ matches: true }) },
    ResizeObserver: class { constructor(callback) { this.callback = callback } observe() { this.callback([{ contentRect: { width: 366, height: 340 } }]) } disconnect() {} },
  })
  const hook = module.exports.useFlowViewport()
  const element = {
    getBoundingClientRect: () => ({ left: 0, top: 0, width: 366, height: 340 }),
    setPointerCapture: id => captures.add(id), hasPointerCapture: id => captures.has(id), releasePointerCapture: id => captures.delete(id),
    focus() {}, addEventListener: (name, fn) => { listeners[name] = fn }, removeEventListener() {},
  }
  hook.viewportRef.current = element
  effects.forEach(fn => fn())
  let clicks = 0
  const button = { tagName: 'BUTTON', click: () => { clicks++ } }
  const pointer = (handler, id, x, y, onButton = false) => hook.handlers[handler]({
    type: handler === 'onPointerUp' ? 'pointerup' : handler === 'onPointerCancel' ? 'pointercancel' : handler,
    button: 0, pointerType: 'touch', pointerId: id, clientX: x, clientY: y,
    target: { closest: selector => onButton === 'label' ? (selector === 'label' ? { control: button } : null) : (onButton && selector === 'button' ? button : null) }, currentTarget: element, preventDefault() {},
  })
  return { pointer, camera: () => states[0], dragging: () => states[1], clicks: () => clicks, captures, listeners }
}

test('a touch tap selects once; dragging from the same node pans without selecting', () => {
  const h = touchHarness()
  h.pointer('onPointerDown', 1, 140, 160, true)
  h.pointer('onPointerUp', 1, 140, 160, true)
  assert.equal(h.clicks(), 1)
  assert.equal(h.dragging(), false)
  const before = h.camera()
  h.pointer('onPointerDown', 2, 140, 160, true)
  h.pointer('onPointerMove', 2, 170, 200, true)
  h.pointer('onPointerUp', 2, 170, 200, true)
  close(h.camera().x, before.x + 30)
  close(h.camera().y, before.y + 40)
  assert.equal(h.clicks(), 1)
  assert.equal(h.captures.size, 0)
  h.pointer('onPointerDown', 3, 140, 200, 'label')
  h.pointer('onPointerUp', 3, 140, 200, 'label')
  assert.equal(h.clicks(), 2)
  h.pointer('onPointerDown', 4, 140, 200, 'label')
  h.pointer('onPointerMove', 4, 170, 240, 'label')
  h.pointer('onPointerUp', 4, 170, 240, 'label')
  assert.equal(h.clicks(), 2)
})

test('two touches zoom without accidental selection and continue panning after one lifts', () => {
  const h = touchHarness()
  const before = h.camera()
  h.pointer('onPointerDown', 1, 100, 140, true)
  h.pointer('onPointerDown', 2, 200, 140, true)
  h.pointer('onPointerMove', 2, 300, 140, true)
  close(h.camera().zoom, before.zoom * 2)
  h.pointer('onPointerUp', 2, 300, 140, true)
  const pinched = h.camera()
  h.pointer('onPointerMove', 1, 110, 155, true)
  h.pointer('onPointerUp', 1, 110, 155, true)
  close(h.camera().x, pinched.x + 10)
  close(h.camera().y, pinched.y + 15)
  assert.equal(h.clicks(), 0)
  assert.equal(h.dragging(), false)
})

test('a cancelled touch never selects a node; trackpad pinch keeps its pointer anchored', () => {
  const h = touchHarness()
  h.pointer('onPointerDown', 1, 100, 140, true)
  h.pointer('onPointerCancel', 1, 100, 140, true)
  assert.equal(h.clicks(), 0)
  assert.equal(h.captures.size, 0)
  const before = h.camera()
  const pointer = { x: 180, y: 160 }
  h.listeners.wheel({ preventDefault() {}, ctrlKey: true, deltaMode: 0, deltaY: -40, clientX: pointer.x, clientY: pointer.y })
  assert.ok(h.camera().zoom > before.zoom)
  close(worldAt(h.camera(), pointer).x, worldAt(before, pointer).x)
  close(worldAt(h.camera(), pointer).y, worldAt(before, pointer).y)
})
