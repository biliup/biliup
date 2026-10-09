const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const ts = require('typescript')
const filename = path.join(__dirname, 'render-geometry.ts')
const compiled = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText
const geometry = {}
vm.runInThisContext(`(function(exports) { ${compiled}\n})`, { filename })(geometry)
const { pictureBounds, moveRegion, resizeRegion, regionActive, imageRegionSize } = geometry
const close = (a, b) => assert.ok(Math.abs(a - b) < 1e-8, `${a} != ${b}`)

test('picture bounds exclude letterboxes for portrait and ultrawide video', () => {
  assert.deepEqual(pictureBounds({ left: 10, top: 20, width: 800, height: 450 }, 1920, 1080), { left: 10, top: 20, width: 800, height: 450 })
  const portrait = pictureBounds({ left: 0, top: 0, width: 800, height: 450 }, 1080, 1920)
  close(portrait.width, 253.125); close(portrait.left, 273.4375)
  const wide = pictureBounds({ left: 0, top: 0, width: 800, height: 450 }, 3440, 1440)
  close(wide.width, 800); close(wide.height, 800 * 1440 / 3440)
  assert.equal(pictureBounds({ left: 0, top: 0, width: 0, height: 100 }, 1920, 1080), null)
  assert.equal(pictureBounds({ left: 0, top: 0, width: 800, height: 450 }, 0, 0), null)
})
test('drag movement clamps the whole image into the picture', () => {
  const rect = { x: .3, y: .2, width: .2, height: .3 }
  assert.deepEqual(moveRegion(rect, -100, 100), { x: 0, y: .7, width: .2, height: .3 })
  assert.deepEqual(moveRegion(rect, .1, .1), { x: .4, y: .30000000000000004, width: .2, height: .3 })
})
test('all eight locked resize handles retain aspect and stay bounded', () => {
  const rect = { x: .3, y: .3, width: .2, height: .1 }
  for (const handle of ['n', 'ne', 'e', 'se', 's', 'sw', 'w', 'nw']) {
    for (const [dx, dy] of [[.1, .1], [-.05, -.02], [100, 100], [-100, -100]]) {
      const result = resizeRegion(rect, handle, dx, dy, true)
      close(result.width / result.height, 2)
      assert.ok(result.x >= -1e-10 && result.y >= -1e-10 && result.width > 0 && result.height > 0)
      assert.ok(result.x + result.width <= 1 + 1e-10 && result.y + result.height <= 1 + 1e-10)
      assert.ok(result.width >= .001 - 1e-10 && result.height >= .001 - 1e-10)
      if (handle.includes('w')) close(result.x + result.width, rect.x + rect.width)
      if (handle.includes('n')) close(result.y + result.height, rect.y + rect.height)
      if (handle === 'e' || handle === 'w') close(result.y + result.height / 2, rect.y + rect.height / 2)
      if (handle === 'n' || handle === 's') close(result.x + result.width / 2, rect.x + rect.width / 2)
    }
  }
})
test('unlocked corner resize fixes the opposite corner and cannot flip', () => {
  const rect = { x: .2, y: .2, width: .3, height: .4 }
  const result = resizeRegion(rect, 'nw', -.1, -.1, false)
  close(result.x, .1); close(result.y, .1); close(result.width, .4); close(result.height, .5)
  const crossed = resizeRegion(rect, 'nw', 1, 1, false)
  close(crossed.x + crossed.width, .5); close(crossed.y + crossed.height, .6)
  close(crossed.width, .001); close(crossed.height, .001)
})
test('intervals include starts, exclude ends and leave gaps inactive', () => {
  const intervals = [{ from_ms: 1000, to_ms: 2000 }, { from_ms: 4000, to_ms: 5000 }]
  for (const ms of [1000, 1999, 4000, 4999]) assert.equal(regionActive(intervals, ms), true)
  for (const ms of [0, 999, 2000, 3000, 5000]) assert.equal(regionActive(intervals, ms), false)
  assert.equal(regionActive([], 99999), true)
})
test('image insertion retains pixel aspect across reference video shapes', () => {
  for (const [iw, ih, vw, vh] of [[300, 200, 1920, 1080], [100, 900, 1920, 1080], [300, 200, 1080, 1920]]) {
    const size = imageRegionSize(iw, ih, vw, vh)
    close(size.width * vw / (size.height * vh), iw / ih)
    assert.ok(size.width <= .25 && size.height <= .8)
  }
})
