const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const ts = require('typescript')

const filename = path.join(__dirname, 'fleet-config.ts')
const compiled = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText
const exportsForTest = {}
vm.runInThisContext(`(function(exports, require) { ${compiled}\n})`, { filename })(exportsForTest, () => ({}))
const { globalPayload, overridePayload } = exportsForTest

test('custom duration is preserved by both global and node config payloads', () => {
  const initial = { segment_time: '00:30:00', file_size: 1024 }
  const values = { ...initial, segment_time: '00:07:30' }
  assert.deepEqual(globalPayload(initial, initial, values), values)
  assert.deepEqual(overridePayload({}, initial, values, initial), { segment_time: '00:07:30' })
})

test('node duration and size explicitly disabled with null stay disabled after save and reload', () => {
  const global = { segment_time: '00:30:00', file_size: 1024, filename_prefix: 'global' }
  const values = { segment_time: null, file_size: null, filename_prefix: null }
  const body = overridePayload({}, global, values, global)
  assert.deepEqual(body, { segment_time: null, file_size: null })
  const loaded = JSON.parse(JSON.stringify(body))
  assert.deepEqual(overridePayload(loaded, values, values, global), loaded)
})

test('removing a custom duration or matching global settings returns to inheritance', () => {
  const global = { segment_time: '00:30:00', file_size: 1024 }
  const current = { segment_time: '00:07:30', file_size: 512 }
  for (const value of [undefined, '']) {
    assert.deepEqual(overridePayload(current, current, { segment_time: value, file_size: value }, global), {})
    const disabled = { segment_time: null, file_size: null }
    assert.deepEqual(overridePayload(disabled, disabled, { segment_time: value, file_size: value }, global), {})
  }
  assert.deepEqual(overridePayload(current, current, global, global), {})
  assert.deepEqual(overridePayload({}, { segment_time: null }, { segment_time: null }, { segment_time: null }), {})
})
