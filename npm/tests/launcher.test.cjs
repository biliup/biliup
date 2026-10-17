'use strict'

const assert = require('node:assert/strict')
const { spawn } = require('node:child_process')
const { once } = require('node:events')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { test } = require('node:test')

const launcher = path.resolve(__dirname, '../biliup/bin/biliup.js')

function run(...args) {
  return spawn(process.execPath, [launcher, ...args], {
    env: { ...process.env, BILIUP_BINARY_PATH: process.execPath },
    stdio: ['ignore', 'pipe', 'pipe'],
  })
}

test('preserves arguments, output and the binary exit status', async () => {
  const child = run('-e', 'console.log(process.argv[1]);process.exit(37)', 'argument with spaces')
  const output = []
  child.stdout.on('data', (data) => output.push(data))
  const [code, signal] = await once(child, 'exit')
  assert.equal(code, 37)
  assert.equal(signal, null)
  assert.equal(Buffer.concat(output).toString(), 'argument with spaces\n')
})

test('reports a missing binary as a normal failure', async () => {
  const child = spawn(process.execPath, [launcher], {
    env: { ...process.env, BILIUP_BINARY_PATH: path.join(os.tmpdir(), 'biliup-missing-binary') },
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  const errors = []
  child.stderr.on('data', (data) => errors.push(data))
  const [code] = await once(child, 'exit')
  assert.equal(code, 1)
  assert.match(Buffer.concat(errors).toString(), /ENOENT/)
})

test('forwards termination to the binary and waits for graceful shutdown', {
  skip: process.platform === 'win32',
  timeout: 10000,
}, async () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'biliup-launcher-'))
  const marker = path.join(tmp, 'stopped')
  const child = run('-e', `
    process.on('SIGTERM', () => {
      setTimeout(() => { require('fs').writeFileSync(process.argv[1], 'flushed');process.exit(0) }, 100)
    })
    console.log('ready')
    setInterval(() => {}, 1000)
  `, marker)
  try {
    await once(child.stdout, 'data')
    const exited = once(child, 'exit')
    child.kill('SIGTERM')
    const [code, signal] = await exited
    assert.equal(code, 0)
    assert.equal(signal, null)
    assert.equal(fs.readFileSync(marker, 'utf8'), 'flushed')
  } finally {
    child.kill('SIGKILL')
    fs.rmSync(tmp, { recursive: true, force: true })
  }
})

test('preserves a binary termination signal', { skip: process.platform === 'win32' }, async () => {
  const child = run('-e', "process.kill(process.pid, 'SIGTERM')")
  const [code, signal] = await once(child, 'exit')
  assert.equal(code, null)
  assert.equal(signal, 'SIGTERM')
})
