const assert = require('node:assert/strict')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const { buildSync } = require('esbuild')
const result = buildSync({ entryPoints: [path.join(__dirname, 'download-format.ts')], bundle: true, write: false, format: 'cjs', platform: 'node' })
const loaded = { exports: {} }
vm.runInThisContext(`(function(module, exports) { ${result.outputFiles[0].text}\n})`)(loaded, loaded.exports)
const { downloadFormatPolicy: policy, savedDownloadFormat: save } = loaded.exports

test('native and automatic downloaders cannot set a misleading container suffix', () => {
  for (const downloader of [null, 'mesio', 'stream-gears', 'streamlink', 'yt-dlp', 'sync-downloader']) {
    assert.deepEqual(policy(downloader).formats, [])
    assert.equal(save('mp4', downloader, ''), null)
  }
})

test('FFmpeg and ytarchive offer only their supported remux containers', () => {
  for (const downloader of ['ffmpeg', 'ffmpeg-external', 'ffmpeg-internal']) {
    assert.equal(save(' .MP4 ', downloader, ''), 'mp4')
    assert.equal(save('ts', downloader, ''), 'ts')
  }
  assert.deepEqual(policy('ytarchive').formats, ['mp4', 'mkv'])
  assert.equal(save('flv', 'ytarchive', ''), null)
})

test('Twitch ffmpeg is routed through streamlink and cannot remux', () => {
  assert.deepEqual(policy('ffmpeg', 'https://www.twitch.tv/example').formats, [])
  assert.equal(save('mp4', 'ffmpeg', 'https://twitch.tv/example'), null)
  assert.equal(save('mp4', 'ffmpeg', 'https://twitch.tv.example.org/example'), 'mp4')
})

test('unavailable inherited configuration preserves existing values until resolved', () => {
  assert.equal(save('mp4', undefined, '', false), 'mp4')
  assert.equal(save('mp4', 'ffmpeg', '', true), 'mp4')
  assert.equal(save('mp4', 'mesio', '', true), null)
})
