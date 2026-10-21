const assert = require('node:assert/strict')
const { test } = require('node:test')
const { buildSync } = require('esbuild')
const path = require('node:path')
const vm = require('node:vm')
const compiled = { exports: {} }
const source = buildSync({ entryPoints: [path.join(__dirname, 'bounded-tasks.ts')], bundle: true, write: false, format: 'cjs', platform: 'node' }).outputFiles[0].text
vm.runInThisContext(`(function(module,exports){${source}\n})`)(compiled, compiled.exports)
test('batch operations limit concurrent requests, preserve order and retain individual failures', async () => {
  let running = 0, peak = 0
  const items = Array.from({ length: 300 }, (_, i) => i)
  const results = await compiled.exports.boundedTasks(items, async i => {
    peak = Math.max(peak, ++running)
    await new Promise(resolve => setImmediate(resolve))
    running--
    if (i % 11 === 0) throw new Error(`failure ${i}`)
    return i
  })
  assert.equal(peak, 4)
  assert.equal(results.length, 300)
  items.forEach(i => i % 11 === 0 ? assert.equal(results[i].status, 'rejected') : assert.equal(results[i].value, i))
})
