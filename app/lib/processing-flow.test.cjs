const assert = require('node:assert/strict')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const { buildSync } = require('esbuild')
function load(file) {
  const result = buildSync({ entryPoints: [path.join(__dirname, file)], bundle: true, write: false, format: 'cjs', platform: 'node' })
  const module = { exports: {} }
  vm.runInThisContext(`(function(module, exports) { ${result.outputFiles[0].text}\n})`)(module, module.exports)
  return module.exports
}
const { processingFlowFromEntity: from, processingFlowToForm: to, stageSteps, moveProcessingStep: move, validateProcessingFlow: validate } = load('processing-flow.ts')
const { formListToHookStep } = load('postprocessor.ts')
function submit(flow) {
  const result = to(flow)
  if (result.postprocessor) result.postprocessor = formListToHookStep(result.postprocessor)
  return result
}

test('local and Fleet form round trip preserves existing hook arrays and native recipes', () => {
  const recipe = { danmaku: { enabled: true, segment_offsets: { '12': 500 } }, regions: [{ id: 'mask', x: 0, y: 0, width: .2, height: .3, intervals: [{ from_ms: 0, to_ms: 1000 }] }] }
  const entity = { preprocessor: [{ run: 'echo ready' }], segment_processor: [{ remux: 'mp4' }, { render: recipe }], downloaded_processor: [{ run: 'echo ended' }], postprocessor: [{ mv: 'archive with spaces/&literal/' }] }
  assert.deepEqual(submit(from(entity)), entity)
  const oldForm = { ...entity, postprocessor: [{ cmd: 'mv', value: 'archive with spaces/&literal/' }] }
  assert.deepEqual(submit(from(oldForm)), entity)
  assert.equal(validate(from(entity)), '')
})

test('implicit default cleanup and explicit keep-files are different after saving and reopening', () => {
  for (const postprocessor of [undefined, null, [], ['rm']]) {
    const flow = from({ postprocessor })
    assert.deepEqual(submit(flow).postprocessor, postprocessor)
    assert.equal(stageSteps(flow, 'postprocessor').length, postprocessor?.length ?? 1)
  }
  const keep = { ...from({ postprocessor: null }), postprocessor: [] }
  assert.deepEqual(submit(keep).postprocessor, [])
  assert.deepEqual(stageSteps(from(submit(keep)), 'postprocessor'), [])
})

test('reordering changes only the selected phase and preserves the step payload', () => {
  const flow = from({ segment_processor: [{ run: 'echo first' }, { remux: 'mp4' }], preprocessor: [{ run: 'echo before' }] })
  const reordered = move(flow, 'segment_processor', 1, 0)
  assert.deepEqual(submit(reordered).segment_processor, [{ remux: 'mp4' }, { run: 'echo first' }])
  assert.deepEqual(reordered.preprocessor, flow.preprocessor)
  assert.deepEqual(submit(flow).segment_processor, [{ run: 'echo first' }, { remux: 'mp4' }])
  assert.equal(move(flow, 'segment_processor', 0, -1), flow)
})

test('the editor rejects operations unsupported by the existing stage and malformed render settings', () => {
  assert.match(validate({ preprocessor: [{ cmd: 'render', value: '{}' }] }), /不支持/)
  assert.match(validate({ postprocessor: [{ cmd: 'remux', value: 'mp4' }] }), /不支持/)
  assert.match(validate({ segment_processor: [{ cmd: 'render', value: '[]' }] }), /JSON 对象/)
  assert.match(validate({ segment_processor: [{ cmd: 'render', value: '{' }] }), /JSON 对象/)
  assert.match(validate({ segment_processor: [{ cmd: 'render', value: JSON.stringify({ regions: [{ x: .9, width: .2 }] }) }] }), /画面内/)
  assert.match(validate({ segment_processor: [{ cmd: 'render', value: JSON.stringify({ regions: [{ effect_type: 'image' }] }) }] }), /场次图片/)
  assert.match(validate({ downloaded_processor: [{ cmd: 'run', value: ' ' }] }), /填写/)
})
