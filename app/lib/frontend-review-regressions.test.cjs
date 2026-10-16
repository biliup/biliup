const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const { test } = require('node:test')
const ts = require('typescript')

function loadTypeScript(name, dependencies = {}, globals = {}) {
  const filename = path.join(__dirname, name)
  const source = fs.readFileSync(filename, 'utf8')
  const compiled = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText
  const exports = {}
  vm.runInNewContext(compiled, {
    exports,
    require: (id) => {
      assert.ok(Object.hasOwn(dependencies, id), `unexpected dependency ${id} in ${name}`)
      return dependencies[id]
    },
    URL,
    Map,
    Set,
    ...globals,
  }, { filename })
  return exports
}

test('login return target cannot turn a same-origin double-slash path into an external navigation', () => {
  const { resolveLoginDestination } = loadTypeScript('login-redirect.ts')
  const origin = 'https://biliup.example'
  const next = 'https://biliup.example//attacker.example/path?from=login#top'
  const destination = resolveLoginDestination(next, origin)
  assert.ok(destination)
  assert.equal(new URL(destination).origin, origin)
  assert.equal(new URL(destination).pathname, '//attacker.example/path')
  assert.equal(resolveLoginDestination('https://attacker.example/path', origin), null)
})

test('submission stays locked for the whole pending request so close/reopen cannot acquire a second submit', async () => {
  const { SubmissionLock } = loadTypeScript('submission-lock.ts')
  const lock = new SubmissionLock()
  let finishRequest
  const request = new Promise((resolve) => { finishRequest = resolve })
  let posts = 0
  const submit = async () => {
    if (!lock.acquire()) return false
    posts += 1
    try {
      await request
      return true
    } finally {
      lock.release()
    }
  }

  const firstPost = submit()
  assert.equal(lock.isLocked, true)
  // A cancel handler checks this parent-owned lock while the request is pending.
  assert.equal(lock.isLocked, true)
  // Closing and reopening replaces the footer, but the second submit sees the same lock.
  assert.equal(await submit(), false)
  assert.equal(posts, 1)
  finishRequest()
  assert.equal(await firstPost, true)
  assert.equal(lock.isLocked, false)
  assert.equal(await submit(), true)
  assert.equal(posts, 2)
})

test('idle or terminal session rows keep a discovery poll while active work stays fast', () => {
  const { sessionJobRefreshInterval, ACTIVE_AUTO_CLIP_POLL_MS, DISCOVER_AUTO_CLIP_POLL_MS } =
    loadTypeScript('auto-clip-poll.ts')
  assert.equal(sessionJobRefreshInterval(null), DISCOVER_AUTO_CLIP_POLL_MS)
  assert.equal(sessionJobRefreshInterval(undefined), DISCOVER_AUTO_CLIP_POLL_MS)
  assert.equal(sessionJobRefreshInterval({ state: 'done' }), DISCOVER_AUTO_CLIP_POLL_MS)
  assert.equal(sessionJobRefreshInterval({ state: 'queued' }), ACTIVE_AUTO_CLIP_POLL_MS)
  assert.equal(sessionJobRefreshInterval({ state: 'running' }), ACTIVE_AUTO_CLIP_POLL_MS)
})

test('log stream discards prior tail on truncation and never retains more than 2000 rows', () => {
  const { appendLogMessage, LOG_TRUNCATED_MESSAGE, MAX_LOG_LINES } = loadTypeScript('log-buffer.ts')
  let rows = Array.from({ length: MAX_LOG_LINES }, (_, i) => `old-${i}`)
  rows = appendLogMessage(rows, 'new-line')
  assert.equal(rows.length, MAX_LOG_LINES)
  assert.equal(rows[0], 'old-1')
  assert.equal(rows.at(-1), 'new-line')
  rows = appendLogMessage(rows, LOG_TRUNCATED_MESSAGE)
  assert.deepEqual(Array.from(rows), [LOG_TRUNCATED_MESSAGE])
  rows = appendLogMessage(rows, 'reloaded-tail')
  assert.deepEqual(Array.from(rows), [LOG_TRUNCATED_MESSAGE, 'reloaded-tail'])
})

test('postprocessor conversion rejects form actions with no server HookStep contract', () => {
  const { formToHookStep, formListToHookStep } = loadTypeScript('postprocessor.ts')
  const templateModal = fs.readFileSync(path.join(__dirname, '..', 'ui', 'TemplateModal.tsx'), 'utf8')
  assert.doesNotMatch(templateModal, /Form\.Select\.Option value="webhook"/)
  assert.equal(formToHookStep({ cmd: 'rm' }), 'rm')
  assert.deepEqual(JSON.parse(JSON.stringify(formToHookStep({ cmd: 'mv', value: 'archive/' }))), { mv: 'archive/' })
  assert.throws(() => formToHookStep({ cmd: 'webhook', value: 'https://example.invalid/hook' }), /不支持的后处理操作/)
  assert.throws(() => formListToHookStep([{ cmd: 'run', value: 'echo ok' }, { cmd: 'webhook', value: 'x' }]), /不支持的后处理操作/)
})

test('danmaku EventSource opts into credentialed cross-origin requests', () => {
  const calls = []
  class FakeEventSource {
    static CONNECTING = 0
    static OPEN = 1
    static CLOSED = 2
    constructor(url, options) {
      calls.push({ url, options })
      this.readyState = FakeEventSource.CONNECTING
    }
    addEventListener() {}
    close() { this.readyState = FakeEventSource.CLOSED }
  }
  const { DanmakuFeed } = loadTypeScript('danmaku-feed.ts', {
    react: { useEffect() {}, useMemo: (fn) => fn() },
    './api-streamer': { API_BASE: 'http://localhost:19159', revalidateMe() {} },
  }, { window: {}, EventSource: FakeEventSource })
  const feed = new DanmakuFeed([10, 11])
  feed.subscribe(10, () => {})
  feed.open()
  assert.equal(calls.length, 1)
  assert.equal(calls[0].url, 'http://localhost:19159/v1/danmaku?ids=10,11')
  assert.deepEqual(JSON.parse(JSON.stringify(calls[0].options)), { withCredentials: true })
  feed.close()
})

test('login stylesheet compiles with a vertical overflow path for short viewports', () => {
  const sass = require('sass')
  const filename = path.join(__dirname, '..', '(auth)', 'login', 'index.module.scss')
  const css = sass.compile(filename).css
  const frame = css.match(/\.frame\s*\{([^}]*)\}/)?.[1] ?? ''
  assert.match(frame, /min-height:\s*100dvh/)
  assert.match(frame, /overflow-y:\s*auto/)
  assert.doesNotMatch(frame, /(?:^|\n)\s*height:\s*100vh\s*;/)
})
