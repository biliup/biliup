const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const ts = require('typescript')

// Reuse the project's TypeScript compiler so these pure tests also run on Node 20/22.
const filename = path.join(__dirname, 'mosaic-config.ts')
const compiled = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText
const exportsForTest = {}
vm.runInThisContext(`(function(exports) { ${compiled}\n})`, { filename })(exportsForTest)
const {
  isMosaicConfig, validateMosaicConfig, mosaicRectangle, parseOverrideText, updateMosaicOverrideText,
} = exportsForTest

const region = (updates = {}) => ({
  id: 'region-1', x: 0.1, y: 0.2, width: 0.3, height: 0.15,
  effectType: 'mosaic', strength: 16, ...updates,
})
const config = (updates = {}) => ({ enabled: true, regions: [region()], ...updates })

test('visual changes serialize as an object under override.mosaic_config without dropping other settings', () => {
  const previous = JSON.stringify({ file_size: 12345, mosaic_config: config({ enabled: false }) })
  const changed = config({ regions: [region({ strength: 64 })], mode: 'post_segment' })
  const text = updateMosaicOverrideText(previous, changed)
  const override = parseOverrideText(text)
  assert.equal(override.file_size, 12345)
  assert.deepEqual(override.mosaic_config, changed)
  assert.equal(typeof override.mosaic_config, 'object')
  assert.equal(validateMosaicConfig(JSON.parse(JSON.stringify({ override })).override.mosaic_config), '')
})

test('removing the JSON mosaic override retains other settings and follows global configuration', () => {
  const text = updateMosaicOverrideText(JSON.stringify({ file_size: 42, mosaic_config: config() }), undefined)
  assert.deepEqual(parseOverrideText(text), { file_size: 42 })
  assert.equal(validateMosaicConfig(undefined), '')
  assert.equal(validateMosaicConfig(null), '')
  assert.deepEqual(parseOverrideText('   '), {})
  for (const invalid of ['null', '[]', 'true', '"string"', '{']) {
    assert.throws(() => parseOverrideText(invalid))
    assert.throws(() => updateMosaicOverrideText(invalid, config()))
  }
})

test('validation prevents enabled empty regions, malformed shapes, and invalid strengths or colors', () => {
  assert.equal(validateMosaicConfig(config()), '')
  assert.equal(validateMosaicConfig(config({ enabled: false, regions: [] })), '')
  assert.notEqual(validateMosaicConfig(config({ regions: [] })), '')
  for (const invalid of [
    config({ regions: [region({ strength: NaN })] }),
    config({ regions: [region({ x: Infinity })] }),
    config({ regions: [region({ x: -0.1 })] }),
    config({ regions: [region({ x: 0.9, width: 0.3 })] }),
    config({ regions: [region({ width: 0.001 })] }),
    config({ regions: [region({ strength: 3 })] }),
    config({ regions: [region({ strength: 65 })] }),
    config({ regions: [region({ strength: 16.5 })] }),
    config({ regions: [region({ effectType: 'blur', strength: 101 })] }),
    config({ regions: [region({ effectType: 'solid', color: 'red:enable=1' })] }),
    config({ regions: [region(), region()] }),
    config({ regions: Array.from({ length: 33 }, (_, i) => region({ id: `region-${i}` })) }),
    { enabled: true, regions: [{}] },
    { enabled: 'true', regions: [] },
  ]) {
    assert.notEqual(validateMosaicConfig(invalid), '')
  }
  assert.equal(validateMosaicConfig(config({ regions: [region({ effectType: 'solid', color: '#A0B0c0' })] })), '')
  assert.equal(isMosaicConfig(config({ regions: [region({ effectType: { toString() { throw Error('bad') } } })] })), false)
})

test('backward drags retain the fixed start point across multiple pointer moves', () => {
  const start = { x: 0.8, y: 0.8 }
  const first = mosaicRectangle(start, { x: 0.6, y: 0.6 })
  const second = mosaicRectangle(start, { x: 0.2, y: 0.2 })
  assert.equal(first.x, 0.6)
  assert.equal(second.x, 0.2)
  assert.ok(Math.abs(second.width - 0.6) < 1e-9)
  assert.ok(Math.abs(second.height - 0.6) < 1e-9)
  assert.deepEqual(start, { x: 0.8, y: 0.8 })
})

test('pointer capture outside the canvas clamps every rectangle to the video bounds', () => {
  const full = mosaicRectangle({ x: -1, y: 2 }, { x: 2, y: -1 })
  assert.deepEqual(full, { x: 0, y: 0, width: 1, height: 1 })
  const nearEdge = mosaicRectangle({ x: 0.7, y: 0.9 }, { x: 3, y: 3 })
  assert.equal(nearEdge.x + nearEdge.width, 1)
  assert.equal(nearEdge.y + nearEdge.height, 1)
})
