#!/usr/bin/env node
'use strict'

// 按平台找到 optionalDependencies 里装上的那个子包，把参数、标准输入输出和退出码原样交给它的二进制。
const { spawnSync } = require('child_process')
const fs = require('fs')
const path = require('path')

const pkg = require('../package.json')

// 键为 `${process.platform}-${process.arch}[-libc]`，值按优先级排列。
// x64 的 musl 构建是静态链接的，glibc 系统上缺 gnu 子包时也能跑。
const PACKAGES = {
  'linux-x64-glibc': ['@biliup/linux-x64-gnu', '@biliup/linux-x64-musl'],
  'linux-x64-musl': ['@biliup/linux-x64-musl'],
  'linux-arm64-glibc': ['@biliup/linux-arm64-gnu'],
  'linux-arm-glibc': ['@biliup/linux-arm-gnueabi'],
  'darwin-x64': ['@biliup/darwin-x64'],
  'darwin-arm64': ['@biliup/darwin-arm64'],
  'win32-x64': ['@biliup/win32-x64'],
}

function detectLibc() {
  try {
    const ldd = fs.readFileSync('/usr/bin/ldd', 'latin1')
    if (ldd.includes('musl')) return 'musl'
    if (ldd.includes('GNU C Library') || ldd.includes('glibc')) return 'glibc'
  } catch {
    // 没有 ldd（如精简容器）时退回 Node 自己的判断
  }
  try {
    if (process.report) process.report.excludeNetwork = true
    const header = process.report && process.report.getReport().header
    return header && header.glibcVersionRuntime ? 'glibc' : 'musl'
  } catch {
    return 'glibc'
  }
}

function platformKey() {
  const base = `${process.platform}-${process.arch}`
  return process.platform === 'linux' ? `${base}-${detectLibc()}` : base
}

function findBinary(candidates) {
  const exe = process.platform === 'win32' ? 'biliup.exe' : 'biliup'
  for (const name of candidates) {
    let dir
    try {
      dir = path.dirname(require.resolve(`${name}/package.json`))
    } catch {
      continue
    }
    const bin = path.join(dir, 'bin', exe)
    if (fs.existsSync(bin)) return bin
  }
  return null
}

function fail(lines) {
  process.stderr.write(`${lines.join('\n')}\n`)
  process.exit(1)
}

function resolveBinary() {
  if (process.env.BILIUP_BINARY_PATH) return process.env.BILIUP_BINARY_PATH

  const key = platformKey()
  const candidates = PACKAGES[key]
  if (!candidates) {
    fail([
      `biliup: npm 包不支持当前平台 ${key}。`,
      `支持的平台：${Object.keys(PACKAGES).join(', ')}`,
      '其它平台可以用 `pip install biliup`，或从 https://github.com/biliup/biliup/releases 下载二进制，',
      '再用环境变量 BILIUP_BINARY_PATH 指向它。',
    ])
  }

  const bin = findBinary(candidates)
  if (!bin) {
    fail([
      `biliup: 没有找到当前平台（${key}）的二进制子包 ${candidates[0]}@${pkg.version}。`,
      `它作为 optionalDependencies 随 ${pkg.name} 一起安装，常见原因：`,
      '  - 安装时用了 --omit=optional / --no-optional / --ignore-optional（yarn 同名参数）',
      '    或 npm config 里设了 omit=optional；',
      '  - package-lock.json / node_modules 是在别的平台上生成后拷过来的；',
      '  - 安装时访问不到该子包（镜像源尚未同步等）。',
      `重新安装：删除 node_modules 与 lock 文件后执行 \`npm install ${pkg.name} --include=optional\`，`,
      `或直接安装子包：\`npm install ${candidates[0]}@${pkg.version}\`。`,
    ])
  }
  return bin
}

const result = spawnSync(resolveBinary(), process.argv.slice(2), { stdio: 'inherit' })

if (result.error) {
  fail([`biliup: 启动二进制失败：${result.error.message}`])
}
if (result.signal) {
  process.kill(process.pid, result.signal)
} else {
  process.exit(result.status ?? 1)
}
