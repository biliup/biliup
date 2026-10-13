const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const ts = require('typescript')

function load(name, dependencies, globals = {}) {
  const filename = path.join(__dirname, name)
  const compiled = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.ReactJSX },
  }).outputText
  const exports = {}
  vm.runInNewContext(compiled, { exports, ...globals, require: id => {
    assert.ok(Object.hasOwn(dependencies, id), `unexpected dependency ${id}`)
    return dependencies[id]
  } }, { filename })
  return exports
}

class ReportedError extends Error {}
const sessions = load('sessions.ts', { './api-streamer': {}, swr: {}, './markers': { ReportedError } })
const markerHelpers = load('markers.ts', { './api-streamer': {}, swr: {} })
const source = (id, patch = {}) => ({ id, kind: 'source', streamer_id: 4, streamer_name: '主播', title: '',
  started_at: 1_700_000_000_000, ended_at: 1_700_000_010_000, duration_ms: 9000, recording: false, ...patch })
const marker = (id, at, patch = {}) => ({ id, session_id: 7, at_ms: at, label: '高光', color: '#ff6600',
  lookback_ms: 60000, lookahead_ms: 30000, ...patch })

test('merge preview keeps the 16 minute 35 second gap after the media end despite clock drift', () => {
  const first = source(7)
  const second = source(8, { started_at: first.ended_at + 995000, ended_at: first.ended_at + 1005000 })
  const input = [second, first]
  const preview = sessions.mergePreview(input)
  assert.deepEqual(Array.from(preview, member => member.source_session_id), [7, 8])
  assert.equal(preview[1].gap_before_ms, 995000)
  assert.equal(preview[1].offset_ms, first.duration_ms + 995000)
  assert.notEqual(preview[1].offset_ms, second.started_at - first.started_at)
  assert.deepEqual(input, [second, first])
})

test('merge validation accepts renamed streamers but rejects overlapping, unrelated and unfinished earlier sources', () => {
  const first = source(7)
  const second = source(8, { started_at: first.ended_at + 1000, ended_at: null, recording: true, streamer_name: '改过名字' })
  assert.equal(sessions.mergeProblem([second, first]), null)
  assert.match(sessions.mergeProblem([first]), /至少两场/)
  assert.match(sessions.mergeProblem([first, { ...second, streamer_id: 5 }]), /同一主播/)
  assert.match(sessions.mergeProblem([first, { ...second, streamer_id: null }]), /关联主播/)
  assert.match(sessions.mergeProblem([first, { ...second, started_at: first.ended_at - 1 }]), /重叠/)
  assert.match(sessions.mergeProblem([{ ...first, recording: true }, second]), /最后一个/)
  assert.match(sessions.mergeProblem([{ ...first, group_session_id: 11 }, second]), /原始场次/)
})

test('cover time conversion preserves owner frames and rejects other sources and the recording gap', () => {
  const detail = { id: 12, members: [
    { source_session_id: 7, offset_ms: 0, duration_ms: 9000 },
    { source_session_id: 8, offset_ms: 1004000, duration_ms: 10000 },
  ], segments: [{ source_session_id: 8, start_ms: 1004000, end_ms: 1019000 }] }
  assert.equal(sessions.timeForOwner(detail, 8, 1006000), 2000)
  assert.equal(sessions.timeForOwner(detail, 8, 1018000), 14000, 'live detail extends beyond the persisted member duration')
  assert.equal(sessions.timeForOwner(detail, 8, 5000), null)
  assert.equal(sessions.timeForOwner(detail, 8, 1000000), null)
  assert.equal(sessions.timeForOwner(detail, 7, 1006000), null)
  assert.equal(sessions.timeForOwner(detail, 12, 1006000), 1006000)
})

function clipModule(replies = {}) {
  const requests = [], refreshes = []
  const exports = load('clips.ts', {
    react: {}, swr: { mutate: key => { refreshes.push(key); return Promise.resolve() } },
    './markers': { ReportedError },
    './api-streamer': {
      fetcher: async url => { requests.push({ url }); if (!(url in replies)) throw new Error(`missing ${url}`); return replies[url] },
      apiFetch: async (url, init) => { requests.push({ url, init }); return { json: async () => replies[url] ?? {} } },
      handleResponse: async response => response,
    },
  })
  return { ...exports, requests, refreshes }
}

test('inherited clip updates use the group route while keeping canonical ownership and refreshing both views', async () => {
  const clip = Object.freeze({ id: 20, session_id: 8, view_session_id: 12, in_ms: 1014000, out_ms: 1024000 })
  const response = { ...clip, in_ms: 1015000 }
  const api = clipModule({ '/v1/sessions/12/clips/20': response })
  assert.equal(await api.updateClip(clip, { in_ms: 1015000 }), response)
  assert.equal(api.requests[0].url, '/v1/sessions/12/clips/20')
  assert.equal(JSON.parse(api.requests[0].init.body).in_ms, 1015000)
  assert.equal(clip.session_id, 8)
  assert.ok(api.refreshes.includes('/v1/sessions/8/clips'))
  assert.ok(api.refreshes.includes('/v1/sessions/12/clips'))
})

test('nullable view metadata never refreshes or mutates a null session', async () => {
  const clip = { id: 20, session_id: 8, view_session_id: null }
  const api = clipModule()
  api.refresh(clip)
  await api.deleteClip(clip)
  assert.equal(api.requests[0].url, '/v1/sessions/8/clips/20')
  assert.equal(api.refreshes.some(key => key.includes('/null/')), false)
})

test('publishing inherited clips obtains owner coordinates and preserves the group selection order', async () => {
  const inherited = { id: 20, session_id: 8, view_session_id: 12, in_ms: 1014000, out_ms: 1024000 }
  const canonical = { ...inherited, view_session_id: null, in_ms: 10000, out_ms: 20000 }
  const own = { id: 21, session_id: 12, view_session_id: null, in_ms: 1025000, out_ms: 1029000 }
  const api = clipModule({ '/v1/clips/20': canonical })
  const target = await api.canonicalPublishClips([inherited, own])
  assert.equal(target[0], canonical)
  assert.equal(target[1], own)
  assert.equal(inherited.in_ms, 1014000)
  assert.deepEqual(Array.from(target, clip => clip.id), [20, 21])
  assert.equal(api.requests.length, 1)
})

test('all coincident markers survive clustering and out-of-window markers are excluded', () => {
  const input = [marker(3, 100000), marker(1, 100000), marker(2, 100001), marker(4, 300000), marker(5, 700000)]
  const clusters = markerHelpers.markerClusters(input, 0, 600000)
  assert.deepEqual(Array.from(clusters, cluster => Array.from(cluster, marker => marker.id)), [[1, 3, 2], [4]])
  assert.deepEqual(input.map(marker => marker.id), [3, 1, 2, 4, 5])
})

function timeline(globals = {}) {
  const jsx = (type, props, key) => ({ type, props, key })
  const exports = load('../ui/replay/Timeline.tsx', {
    react: { useEffect() {}, useRef: () => ({ current: null }), useState: initial => [initial, () => {}] },
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@douyinfe/semi-ui': { Tooltip: props => props.children, Popover: props => [props.children, props.content] },
    '@douyinfe/semi-icons': { IconFlag: () => jsx('svg', {}) },
    '@/app/lib/markers': markerHelpers,
    '@/app/lib/sessions': sessions,
    './replay.module.scss': new Proxy({}, { get: (_, key) => key }),
  }, globals)
  const flatten = node => {
    if (!node || typeof node !== 'object') return []
    if (Array.isArray(node)) return node.flatMap(flatten)
    if (typeof node.type === 'function') return flatten(node.type(node.props))
    return [node, ...flatten(node.props?.children)]
  }
  return { ...exports, flatten }
}

test('both timeline layers expose flag buttons whose activation picks once without seeking the background', () => {
  const { OverviewBar, DetailBar, flatten } = timeline()
  for (const Component of [OverviewBar, DetailBar]) {
    const picked = [], seeks = []
    const chosen = marker(1, 100000)
    const tree = Component({ duration: 600000, from: 0, to: 600000, segments: [], gaps: [], points: [],
      markers: [chosen], selection: { in: null, out: null }, playhead: null, detailWindow: null,
      onPickMarker: marker => picked.push(marker), onSeek: time => seeks.push(time) })
    const flag = flatten(tree).find(node => node.props?.['data-marker-id'] === chosen.id)
    assert.equal(flag.type, 'button')
    assert.match(flag.props['aria-label'], /1:40\.0.*高光.*0:40\.0.*2:10\.0/)
    assert.equal(flag.props.style.color, chosen.color)
    let stopped = 0
    flag.props.onPointerDown({ stopPropagation: () => stopped++ })
    flag.props.onKeyDown({ key: ' ', stopPropagation: () => stopped++ })
    flag.props.onClick({ stopPropagation: () => stopped++ })
    assert.equal(stopped, 3)
    assert.equal(picked[0], chosen)
    assert.equal(picked.length, 1)
    assert.equal(seeks.length, 0)
  }
})

test('dense marker menus retain a separate actionable button for every marker', () => {
  const { OverviewBar, flatten } = timeline()
  const picked = []
  const markers = [marker(1, 100000), marker(2, 100000), marker(3, 100000)]
  const tree = OverviewBar({ duration: 600000, segments: [], gaps: [], markers, selection: { in: null, out: null },
    playhead: null, detailWindow: null, onPickMarker: marker => picked.push(marker) })
  const flags = flatten(tree).filter(node => node.props?.['data-marker-id'])
  assert.equal(flags.length, 3)
  for (const flag of flags) flag.props.onClick({ stopPropagation() {} })
  assert.deepEqual(picked.map(marker => marker.id), [1, 2, 3])
})

test('keyboard activation of a dense flag opens the explicit menu and moves focus to a marker choice', () => {
  const callbacks = []
  let focused = 0
  const { OverviewBar, flatten } = timeline({ setTimeout: callback => callbacks.push(callback),
    document: { querySelector: selector => { assert.match(selector, /data-marker-cluster-menu/); return { focus: () => focused++ } } },
  })
  const tree = OverviewBar({ duration: 600000, segments: [], gaps: [], markers: [marker(1, 100000), marker(2, 100000)],
    selection: { in: null, out: null }, playhead: null, detailWindow: null, onPickMarker() {} })
  const trigger = flatten(tree).find(node => node.props?.['data-marker-cluster'])
  let stopped = 0, prevented = 0
  trigger.props.onKeyDown({ key: 'Enter', stopPropagation: () => stopped++, preventDefault: () => prevented++ })
  callbacks.forEach(callback => callback())
  assert.equal(stopped, 1)
  assert.equal(prevented, 1)
  assert.equal(focused, 1)
})
