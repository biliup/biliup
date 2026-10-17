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

test('postprocessor conversion rejects form actions with no server HookStep contract', () => {
  const { formToHookStep, formListToHookStep } = loadTypeScript('postprocessor.ts')
  assert.equal(formToHookStep({ cmd: 'rm' }), 'rm')
  assert.deepEqual(JSON.parse(JSON.stringify(formToHookStep({ cmd: 'mv', value: 'archive/' }))), { mv: 'archive/' })
  assert.throws(() => formToHookStep({ cmd: 'webhook', value: 'https://example.invalid/hook' }), /不支持的后处理操作/)
  assert.throws(() => formListToHookStep([{ cmd: 'run', value: 'echo ok' }, { cmd: 'webhook', value: 'x' }]), /不支持的后处理操作/)
})
