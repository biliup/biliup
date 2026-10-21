const assert = require('node:assert/strict')
const { test } = require('node:test')
const path = require('node:path')
const { buildSync } = require('esbuild')
const React = require('react')
const { renderToStaticMarkup } = require('react-dom/server')
const vm = require('node:vm')
const moduleSource = buildSync({
  entryPoints: [path.join(__dirname, 'EventTimeline.tsx')], bundle: true, write: false,
  format: 'cjs', platform: 'node', external: ['react', 'react/jsx-runtime', 'next/link'],
  loader: { '.scss': 'empty' },
}).outputFiles[0].text
const compiled = { exports: {} }
const testRequire = name => name === 'next/link'
  ? ({ children, prefetch, ...props }) => React.createElement('a', props, children)
  : require(name)
vm.runInThisContext(`(function(module,exports,require){${moduleSource}\n})`)(compiled, compiled.exports, testRequire)
const Timeline = compiled.exports.default

test('events share one date heading per local day and keep timestamps and session links', () => {
  const events = [
    { id: '1', ts: new Date(2026, 9, 21, 23, 5, 58).getTime(), text: '录制结束', kind: 'file', session_id: 99 },
    { id: '2', ts: new Date(2026, 9, 21, 0, 0, 1).getTime(), text: '录制开始', kind: 'start', sub: 'a'.repeat(300) },
    { id: '3', ts: new Date(2026, 9, 20, 23, 59, 59).getTime(), text: '昨日录制', kind: 'file' },
  ]
  const html = renderToStaticMarkup(React.createElement(Timeline, { events }))
  assert.equal((html.match(/<h3/g) || []).length, 2)
  assert.equal((html.match(/<time /g) || []).length, 3)
  assert.match(html, />23:05:58<\/time>/)
  assert.match(html, />00:00:01<\/time>/)
  assert.match(html, /href="\/replay\?session=99"/)
  assert.ok(html.includes('a'.repeat(300)), 'long names remain readable rather than being dropped')
})

test('empty and failed event requests render a fallback independently of the overview', () => {
  assert.match(renderToStaticMarkup(React.createElement(Timeline, { events: [] })), /暂无事件记录/)
  assert.match(renderToStaticMarkup(React.createElement(Timeline, { events: [], error: true })), /暂时无法加载/)
})
