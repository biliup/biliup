const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const ts = require('typescript')

function loadPureModule(name) {
  const filename = path.join(__dirname, name)
  const compiled = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText
  const result = {}
  vm.runInThisContext(`(function(exports) { ${compiled}\n})`, { filename })(result)
  return result
}
const { frameAspectRatio, normalizedFramePoint } = loadPureModule('mosaic-frame.ts')
const { mosaicRectangle } = loadPureModule('mosaic-config.ts')

test('live JPEG aspect is retained for 4:3, ultrawide and portrait frames', () => {
  for (const [width, height] of [[2048, 1536], [3440, 1440], [1080, 1920], [2560, 1440]]) {
    assert.equal(frameAspectRatio({ width, height }), `${width} / ${height}`)
    const displayedWidth = 480
    const bounds = { left: 24, top: 96, width: displayedWidth, height: displayedWidth * height / width }
    assert.deepEqual(normalizedFramePoint(bounds.left, bounds.top, bounds), { x: 0, y: 0 })
    assert.deepEqual(normalizedFramePoint(bounds.left + bounds.width, bounds.top + bounds.height, bounds), { x: 1, y: 1 })
    const center = normalizedFramePoint(bounds.left + bounds.width / 2, bounds.top + bounds.height / 2, bounds)
    assert.ok(Math.abs(center.x - 0.5) < 1e-12)
    assert.ok(Math.abs(center.y - 0.5) < 1e-12)
  }
})

test('a rectangle selected on a resized portrait frame retains identical video coordinates', () => {
  const select = width => {
    const bounds = { left: 15, top: 80, width, height: width * 1920 / 1080 }
    const start = normalizedFramePoint(bounds.left + width * 0.8, bounds.top + bounds.height * 0.75, bounds)
    const end = normalizedFramePoint(bounds.left + width * 0.2, bounds.top + bounds.height * 0.25, bounds)
    return mosaicRectangle(start, end)
  }
  assert.deepEqual(select(450), select(225))
  const selected = select(450)
  assert.equal(selected.x, 0.2)
  assert.equal(selected.y, 0.25)
  assert.ok(Math.abs(selected.width - 0.6) < 1e-9)
  assert.equal(selected.height, 0.5)
})

test('reverse drag and pointer capture outside the live image cannot select its margins', () => {
  const bounds = { left: 30, top: 200, width: 400, height: 300 }
  const start = normalizedFramePoint(390, 470, bounds)
  const end = normalizedFramePoint(-100, -100, bounds)
  assert.deepEqual(mosaicRectangle(start, end), { x: 0, y: 0, width: 0.9, height: 0.9 })
  assert.deepEqual(normalizedFramePoint(500, 600, bounds), { x: 1, y: 1 })
})

test('missing image dimensions and collapsed canvas bounds prevent coordinate creation', () => {
  for (const dimensions of [{ width: 0, height: 720 }, { width: 1280, height: 0 },
    { width: NaN, height: 720 }, { width: 1280, height: Infinity }, { width: 12.5, height: 720 }]) {
    assert.equal(frameAspectRatio(dimensions), null)
  }
  for (const bounds of [{ left: 0, top: 0, width: 0, height: 100 },
    { left: 0, top: 0, width: 100, height: 0 }, { left: NaN, top: 0, width: 100, height: 100 }]) {
    assert.equal(normalizedFramePoint(40, 40, bounds), null)
  }
})
