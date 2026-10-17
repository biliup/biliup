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

test('room cookie choices keep inheritance and explicit anonymous requests distinct', () => {
  const auth = loadAuth()
  for (const value of [undefined, null]) assert.equal(auth.douyuCookieOverrideMode(value), 'inherit')
  assert.equal(auth.douyuCookieOverrideMode(''), 'anonymous')
  assert.equal(auth.douyuCookieOverrideMode('acf_auth=fixture%2F=='), 'custom')
  assert.equal(auth.douyuRoomCookieValue('', 'custom'), null)
  assert.equal(auth.douyuRoomCookieValue('  ', 'custom'), null)
  assert.equal(auth.douyuRoomCookieValue('acf_auth=fixture', 'inherit'), null)
  assert.equal(auth.douyuRoomCookieValue(null, 'anonymous'), '')
  assert.equal(auth.douyuRoomCookieValue('acf_auth=fixture%2F==', 'custom'), 'acf_auth=fixture%2F==')
  assert.equal(auth.douyuAuthFieldsEqual({}, { douyu_cookie: null }, true), true)
  assert.equal(auth.douyuAuthFieldsEqual({}, { douyu_cookie: '' }, true), false)
  assert.equal(auth.douyuAuthFieldsEqual({ douyu_cookie: '' }, { douyu_cookie: null }, true), false)
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
