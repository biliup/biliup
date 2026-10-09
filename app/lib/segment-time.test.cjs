const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const ts = require('typescript')

const filename = path.join(__dirname, 'segment-time.ts')
const compiled = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText
const exportsForTest = {}
vm.runInThisContext(`(function(exports) { ${compiled}\n})`, { filename })(exportsForTest)
const {
  parseSegmentTime, validateSegmentTime, segmentTimeMode, segmentTimeSummary, updateSegmentTimeOverride,
} = exportsForTest

test('arbitrary 7 minute 30 second durations retain all supported recorder formats through save and reload', () => {
  for (const duration of ['00:07:30', '7:30', '450', ' 00:07:30 ']) {
    assert.equal(validateSegmentTime(duration), '')
    assert.equal(parseSegmentTime(duration), 450)
    const global = JSON.parse(JSON.stringify({ segment_time: duration.trim() }))
    assert.equal(parseSegmentTime(global.segment_time), 450)
    const room = JSON.parse(JSON.stringify(updateSegmentTimeOverride({ downloader: 'mesio' }, duration)))
    assert.equal(parseSegmentTime(room.segment_time), 450)
    assert.equal(room.downloader, 'mesio')
    assert.equal(segmentTimeMode(room.segment_time, 'room'), 'custom')
    assert.equal(segmentTimeSummary(room.segment_time), '7 分钟 30 秒')
  }
})

test('presets load by their duration without converting the saved value until edited', () => {
  for (const value of ['00:30:00', '30:00', '1800', ' 00:30:00 ']) {
    assert.equal(segmentTimeMode(value, 'global'), '00:30:00')
    assert.equal(segmentTimeMode(value, 'room'), '00:30:00')
    assert.equal(updateSegmentTimeOverride({}, value).segment_time, value.trim())
  }
  assert.equal(segmentTimeMode('00:00:00', 'global'), 'off')
  assert.equal(segmentTimeMode('00:00:01', 'global'), 'custom')
})

test('room inheritance removes only its duration override, while disabling survives null round trips', () => {
  const previous = { segment_time: '00:30:00', mosaic_config: { enabled: false }, file_size: 42 }
  const inherited = JSON.parse(JSON.stringify(updateSegmentTimeOverride(previous, undefined)))
  assert.equal(Object.hasOwn(inherited, 'segment_time'), false)
  assert.equal(segmentTimeMode(inherited.segment_time, 'room'), 'inherit')
  assert.deepEqual(inherited, { mosaic_config: { enabled: false }, file_size: 42 })
  for (const cleared of [null, '', '   ']) {
    const disabled = JSON.parse(JSON.stringify(updateSegmentTimeOverride(previous, cleared)))
    assert.equal(Object.hasOwn(disabled, 'segment_time'), true)
    assert.equal(disabled.segment_time, null)
    assert.equal(segmentTimeMode(disabled.segment_time, 'room'), 'off')
  }
  assert.equal(previous.segment_time, '00:30:00')
  assert.equal(segmentTimeMode(undefined, 'global'), 'off')
})

test('validator rejects malformed, zero, overflow and non-finite custom input before submission', () => {
  for (const value of [undefined, null, '', '  ']) assert.equal(validateSegmentTime(value), '')
  for (const value of [
    '0', '00:00:00', '0.0', '0.0000000001', 'abc', '7：30', '7:60', '90:00', '1:60:00',
    '1:2:3:4', '-5', '+5', 'NaN', 'Infinity', '1e3', '.5', '1.2.3', '00:00:60',
    '999999999999999999999999999999:00:00', '9007199254740992', 450, [], {},
  ]) assert.notEqual(validateSegmentTime(value), '', String(value))
  for (const value of ['0.5', '00:07:30.5', '1:2:3', '100:00:00', '0.000000001']) {
    assert.equal(validateSegmentTime(value), '', value)
  }
  assert.equal(parseSegmentTime('00:07:30.5'), 450.5)
  assert.equal(segmentTimeSummary('01:07:30.5'), '1 小时 7 分钟 30.5 秒')
  for (const value of [450, {}, []]) {
    assert.equal(segmentTimeMode(value, 'room'), 'custom')
    assert.equal(segmentTimeSummary(value), '')
  }
})
