const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const ts = require('typescript')

const filename = path.join(__dirname, 'local-material-preset.ts')
const compiled = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText
const exportsForTest = {}
vm.runInThisContext(`(function(exports) { ${compiled}\n})`, { filename })(exportsForTest)
const { applyLocalMaterialPreset } = exportsForTest

test('local material preset keeps unrelated overrides and splits root postprocessor', () => {
  const result = applyLocalMaterialPreset({ file_size: 123, segment_time: '01:00:00' })
  assert.deepEqual(result.override, {
    file_size: 123,
    segment_time: '01:00:00',
    downloader: 'mesio',
    uploader: 'Noop',
    filtering_threshold: 0,
    mosaic_config: { enabled: false, regions: [] },
  })
  assert.deepEqual(result.postprocessor, [])
})

test('local material preset does not share mosaic state between calls', () => {
  const first = applyLocalMaterialPreset()
  first.override.mosaic_config.regions.push({ id: 'temporary' })
  const second = applyLocalMaterialPreset()
  assert.deepEqual(second.override.mosaic_config, { enabled: false, regions: [] })
})
