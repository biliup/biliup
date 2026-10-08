const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const test = require('node:test')
const vm = require('node:vm')
const ts = require('typescript')

// Use the existing TypeScript dependency; no frontend test runner or browser is required.
function loadAuth(api = {}) {
  const source = fs.readFileSync(path.join(__dirname, 'douyu-auth.ts'), 'utf8')
  const compiled = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText
  const exports = {}
  vm.runInNewContext(compiled, {
    exports,
    require: (id) => {
      assert.equal(id, './api-streamer')
      return api
    },
  })
  return exports
}

const status = (overrides = {}) => ({
  streamer_id: null, enabled: true, has_cookie: true, has_ltp0: true, has_device_id: true,
  login_state: 'valid', refresh_state: 'scheduled', account_id: '42',
  last_success_at: 1700000000, last_checked_at: 1700000001, next_refresh_at: 1700259200,
  failure_count: 0, needs_login: false, ...overrides,
})

test('status parser returns only safe known fields and rejects unknown states', () => {
  const auth = loadAuth()
  const parsed = auth.parseDouyuAuthStatus(status({ cookie: 'secret-cookie', ltp0: 'secret-ticket', last_error: 'secret-response' }))
  assert.equal(parsed.account_id, '42')
  assert.equal(parsed.last_success_at, 1700000000)
  assert.equal(Object.hasOwn(parsed, 'cookie'), false)
  assert.equal(Object.hasOwn(parsed, 'ltp0'), false)
  assert.equal(Object.hasOwn(parsed, 'last_error'), false)
  assert.throws(() => auth.parseDouyuAuthStatus(status({ login_state: 'secret-response' })), /状态响应无效/)
  assert.equal(auth.parseDouyuAuthStatus(status({ account_id: 'credential-content', next_refresh_at: Infinity })).account_id, null)
  assert.equal(auth.parseDouyuAuthStatus(status({ next_refresh_at: Infinity })).next_refresh_at, null)
})

test('unsaved auth change detection respects empty values and default enabled switch', () => {
  const auth = loadAuth()
  assert.equal(auth.douyuAuthFieldsEqual({}, { douyu_cookie: '', douyu_ltp0: null, douyu_auto_refresh: true }), true)
  assert.equal(auth.douyuAuthFieldsEqual({ douyu_ltp0: 'saved' }, { douyu_ltp0: 'edited' }), false)
  assert.equal(auth.douyuAuthFieldsEqual({}, { douyu_auto_refresh: false }), false)
  assert.equal(auth.douyuAuthFieldsEqual({ douyu_cookie: 'acf_auth=a%2F==' }, { douyu_cookie: 'acf_auth=a/==' }), false)
})

test('manual refresh reports success only with a new successful exchange', () => {
  const auth = loadAuth()
  const previous = status()
  assert.match(auth.douyuManualRefreshMessage(previous, status({ last_success_at: previous.last_success_at + 60 })), /已续期/)
  assert.match(auth.douyuManualRefreshMessage(status({ last_success_at: null }), status()), /已续期/)
  // Manual renewal is also supported when the automatic switch is disabled.
  assert.match(auth.douyuManualRefreshMessage(previous, status({
    enabled: false, refresh_state: 'disabled', last_success_at: previous.last_success_at + 60,
  })), /已续期/)
})

test('a valid retained Cookie after failure or cooldown is not reported as renewal success', () => {
  const auth = loadAuth()
  const previous = status()
  assert.match(auth.douyuManualRefreshMessage(previous, status({ refresh_state: 'retry', failure_count: 1 })), /未完成.*退避重试/)
  assert.match(auth.douyuManualRefreshMessage(previous, status()), /没有新的续期成功记录.*冷却期/)
  assert.match(auth.douyuManualRefreshMessage(previous, status({ refresh_state: 'refreshing' })), /正在进行/)
  assert.match(auth.douyuManualRefreshMessage(previous, status({
    refresh_state: 'credentials_invalid', needs_login: true, last_success_at: previous.last_success_at + 60,
  })), /未完成.*重新登录/)
  assert.match(auth.douyuManualRefreshMessage(previous, status({
    refresh_state: 'retry', last_success_at: previous.last_success_at + 60,
  })), /未完成.*退避重试/)
})

test('manual refresh sends only the saved credential target, never source credentials', async () => {
  const calls = []
  const auth = loadAuth({ apiFetch: async (url, init) => {
    calls.push({ url, body: init.body, method: init.method })
    return { status: 200, ok: true, json: async () => status() }
  } })
  await auth.refreshDouyuAuth()
  await auth.refreshDouyuAuth(12)
  assert.deepEqual(calls, [
    { url: '/v1/douyu/auth/refresh', body: '{}', method: 'POST' },
    { url: '/v1/douyu/auth/refresh', body: '{"streamer_id":12}', method: 'POST' },
  ])
})

test('credential request failures do not read or expose upstream response bodies', async () => {
  const auth = loadAuth({ apiFetch: async () => ({
    status: 502, ok: false,
    text: async () => { throw new Error('must not read credential response body') },
    json: async () => { throw new Error('must not read credential response body') },
  }) })
  await assert.rejects(auth.refreshDouyuAuth(), /斗鱼登录请求失败，请稍后重试/)
})

test('forbidden credential requests refresh permissions without exposing the response body', async () => {
  let revalidated = 0
  const auth = loadAuth({
    apiFetch: async () => ({ status: 403, ok: false, text: async () => { throw new Error('must not read body') } }),
    revalidateMe: () => { revalidated += 1 },
  })
  await assert.rejects(auth.fetchDouyuAuthStatus('/v1/douyu/auth/status'), /没有权限管理斗鱼登录凭据/)
  assert.equal(revalidated, 1)
})

test('unsaved cookie validation uses only the provided cookie and requires a boolean valid field', async () => {
  const input = 'acf_uid=42; acf_auth=fixture%2F=='
  let payload
  let result = { valid: true }
  const auth = loadAuth({ apiFetch: async (_url, init) => {
    payload = JSON.parse(init.body)
    return { status: 200, ok: true, json: async () => result }
  } })
  assert.equal(await auth.testDouyuCookie(input), true)
  assert.deepEqual(payload, { cookie: input })
  result = { valid: 'secret-error' }
  await assert.rejects(auth.testDouyuCookie(input), /验证响应无效/)
})
