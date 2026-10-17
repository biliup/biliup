const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const ts = require('typescript')

const filename = path.join(__dirname, 'fleet-ha.ts')
const compiled = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText
const exportsForTest = {}
vm.runInThisContext(`(function(exports, require) { ${compiled}\n})`, { filename })(exportsForTest, () => ({}))
const { candidatesReadable } = exportsForTest

const ready = {
  paired: false,
  primary: { node: null, streamers: [], templates: [] },
  standby: { node: 2, streamers: [], templates: [] },
}

test('pairing candidates are readable only when both sides are present and error-free', () => {
  assert.equal(candidatesReadable(ready, undefined), true)
  assert.equal(candidatesReadable(ready, undefined, 2, null), true)
  assert.equal(candidatesReadable(ready, undefined, 3, null), false)
  assert.equal(candidatesReadable(ready, undefined, 2, 1), false)
  assert.equal(candidatesReadable(undefined, new Error('request failed')), false)
  assert.equal(candidatesReadable({ ...ready, primary: null }, undefined), false)
  assert.equal(candidatesReadable({ ...ready, standby: null }, undefined), false)
  assert.equal(candidatesReadable({ ...ready, primary: { error: 'unreadable' } }, undefined), false)
  assert.equal(candidatesReadable({ ...ready, standby: { error: 'unreadable' } }, undefined), false)
})
