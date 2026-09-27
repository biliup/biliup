#!/usr/bin/env node
// 把某个 tag 的 Release 二进制装进 npm 包，产出可直接 `npm publish` 的目录：
//   node npm/scripts/prepare.mjs --tag v1.2.9 [--wait 300] [--out npm/dist] [--repo biliup/biliup]
// 版本号取自该 tag 的 Cargo.toml（[workspace.package] version），必须与 tag 一致。
// 资产按 GitHub 给出的 SHA-256 校验，并检查可执行文件头的系统 / 架构，任何一项不符都失败。
// 依赖系统的 tar（xz）与 unzip。GITHUB_TOKEN / GH_TOKEN 可选，用于放宽 API 限流。

import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { parseArgs } from 'node:util'
import { fileURLToPath } from 'node:url'

const NPM_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const REPO_ROOT = path.dirname(NPM_DIR)

// release.yml 的 matrix.build → npm/platforms/<目录>，以及二进制应有的格式与架构
const TARGETS = [
  { dir: 'linux-x64-gnu', build: 'x86_64-linux', format: 'elf', arch: 'x86_64' },
  { dir: 'linux-x64-musl', build: 'x86_64-linux-musl', format: 'elf', arch: 'x86_64' },
  { dir: 'linux-arm64-gnu', build: 'aarch64-linux', format: 'elf', arch: 'aarch64' },
  { dir: 'linux-arm-gnueabi', build: 'arm-linux', format: 'elf', arch: 'arm' },
  { dir: 'darwin-x64', build: 'x86_64-macos', format: 'macho', arch: 'x86_64' },
  { dir: 'darwin-arm64', build: 'aarch64-macos', format: 'macho', arch: 'arm64' },
  { dir: 'win32-x64', build: 'x86_64-windows', format: 'pe', arch: 'x86_64' },
]

const { values: args } = parseArgs({
  options: {
    tag: { type: 'string' },
    repo: { type: 'string', default: process.env.GITHUB_REPOSITORY || 'biliup/biliup' },
    wait: { type: 'string', default: '0' },
    out: { type: 'string', default: path.join(NPM_DIR, 'dist') },
  },
})

function die(msg) {
  console.error(`::error::${msg}`)
  process.exit(1)
}

const tag = args.tag
if (!tag || !/^v\d+\.\d+\.\d+$/.test(tag)) die(`--tag 必须是 vX.Y.Z 形式，收到 ${tag}`)
const waitMs = Number(args.wait) * 60_000
const outDir = path.resolve(args.out)
const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN
const apiHeaders = {
  accept: 'application/vnd.github+json',
  'x-github-api-version': '2022-11-28',
  ...(token ? { authorization: `Bearer ${token}` } : {}),
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

async function cargoVersion() {
  let toml
  try {
    toml = execFileSync('git', ['show', `${tag}:Cargo.toml`], { cwd: REPO_ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] })
  } catch {
    const res = await fetch(`https://raw.githubusercontent.com/${args.repo}/${tag}/Cargo.toml`)
    if (!res.ok) die(`读取 ${tag} 的 Cargo.toml 失败：本地没有该 tag，远端 HTTP ${res.status}`)
    toml = await res.text()
  }
  const section = toml.split(/^\[workspace\.package\]\s*$/m)[1]
  const version = section && section.split(/^\[/m)[0].match(/^version\s*=\s*"([^"]+)"/m)?.[1]
  if (!version) die(`${tag} 的 Cargo.toml 里没有 [workspace.package] version`)
  return version
}

async function waitForAssets(names) {
  const deadline = Date.now() + waitMs
  for (;;) {
    const res = await fetch(`https://api.github.com/repos/${args.repo}/releases/tags/${tag}`, { headers: apiHeaders })
    let missing = names
    let assets = []
    if (res.ok) {
      assets = (await res.json()).assets
      const ready = new Set(assets.filter((a) => a.state === 'uploaded').map((a) => a.name))
      missing = names.filter((n) => !ready.has(n))
      if (missing.length === 0) return new Map(assets.map((a) => [a.name, a]))
    } else if (res.status !== 404) {
      die(`查询 Release ${tag} 失败：HTTP ${res.status} ${await res.text()}`)
    }
    const why = res.ok ? `还缺 ${missing.length} 个资产：${missing.join(', ')}` : 'Release 还不存在'
    if (Date.now() >= deadline) die(`${tag}：${why}（已等待 ${args.wait} 分钟）`)
    console.log(`${new Date().toISOString()} ${why}，60 秒后重试`)
    await sleep(60_000)
  }
}

async function download(asset, dest) {
  const res = await fetch(asset.browser_download_url)
  if (!res.ok) die(`下载 ${asset.name} 失败：HTTP ${res.status}`)
  const buf = Buffer.from(await res.arrayBuffer())
  if (buf.length !== asset.size) die(`${asset.name} 大小 ${buf.length} 与 Release 记录的 ${asset.size} 不符`)
  const sha = createHash('sha256').update(buf).digest('hex')
  if (asset.digest) {
    if (asset.digest !== `sha256:${sha}`) die(`${asset.name} SHA-256 不符：下载得到 ${sha}，Release 记录 ${asset.digest}`)
  } else {
    console.warn(`::warning::${asset.name} 在 Release 上没有 digest，只校验了大小；sha256=${sha}`)
  }
  fs.writeFileSync(dest, buf)
  return sha
}

// 读可执行文件头，返回 { format, arch }
function inspectBinary(file) {
  const fd = fs.openSync(file, 'r')
  const h = Buffer.alloc(4096)
  fs.readSync(fd, h, 0, h.length, 0)
  fs.closeSync(fd)
  if (h.readUInt32BE(0) === 0x7f454c46) {
    const machine = { 0x3e: 'x86_64', 0xb7: 'aarch64', 0x28: 'arm' }[h.readUInt16LE(18)]
    return { format: 'elf', arch: machine }
  }
  if (h.readUInt32LE(0) === 0xfeedfacf) {
    const cpu = { 0x01000007: 'x86_64', 0x0100000c: 'arm64' }[h.readUInt32LE(4)]
    return { format: 'macho', arch: cpu }
  }
  if (h.toString('latin1', 0, 2) === 'MZ') {
    const pe = h.readUInt32LE(0x3c)
    if (pe + 6 <= h.length && h.toString('latin1', pe, pe + 4) === 'PE\0\0') {
      return { format: 'pe', arch: { 0x8664: 'x86_64', 0xaa64: 'arm64' }[h.readUInt16LE(pe + 4)] }
    }
  }
  return { format: 'unknown' }
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'))
}

function writeJson(file, data) {
  fs.writeFileSync(file, `${JSON.stringify(data, null, 2)}\n`)
}

function copyPackage(src, dest) {
  fs.rmSync(dest, { recursive: true, force: true })
  fs.cpSync(src, dest, { recursive: true, filter: (p) => !p.endsWith('.tgz') })
  fs.copyFileSync(path.join(REPO_ROOT, 'LICENSE'), path.join(dest, 'LICENSE'))
}

// 源码里的包清单必须与 TARGETS、根包 optionalDependencies 三方一致
const rootSrc = path.join(NPM_DIR, 'biliup')
const rootPkg = readJson(path.join(rootSrc, 'package.json'))
const platformDirs = fs.readdirSync(path.join(NPM_DIR, 'platforms')).sort()
const targetDirs = TARGETS.map((t) => t.dir).sort()
if (platformDirs.join() !== targetDirs.join()) {
  die(`npm/platforms/ 下的目录 [${platformDirs}] 与脚本 TARGETS [${targetDirs}] 不一致`)
}
const platformNames = platformDirs.map((d) => readJson(path.join(NPM_DIR, 'platforms', d, 'package.json')).name).sort()
const optional = Object.keys(rootPkg.optionalDependencies).sort()
if (optional.join() !== platformNames.join()) {
  die(`根包 optionalDependencies [${optional}] 与平台子包 [${platformNames}] 不一致`)
}

const version = await cargoVersion()
if (`v${version}` !== tag) die(`tag ${tag} 与 Cargo.toml 版本 ${version} 不一致`)
console.log(`${tag}：Cargo.toml 版本 ${version}`)

const assetName = (t) => `biliupR-${tag}-${t.build}.${t.format === 'pe' ? 'zip' : 'tar.xz'}`
const assets = await waitForAssets(TARGETS.map(assetName))

fs.mkdirSync(outDir, { recursive: true })
const work = fs.mkdtempSync(path.join(os.tmpdir(), 'biliup-npm-'))
const summary = []
try {
  for (const t of TARGETS) {
    const name = assetName(t)
    const archive = path.join(work, name)
    const sha = await download(assets.get(name), archive)
    const unpack = path.join(work, t.build)
    fs.mkdirSync(unpack)
    if (name.endsWith('.zip')) execFileSync('unzip', ['-q', archive, '-d', unpack])
    else execFileSync('tar', ['-xJf', archive, '-C', unpack])

    const exe = t.format === 'pe' ? 'biliup.exe' : 'biliup'
    const bin = path.join(unpack, `biliupR-${tag}-${t.build}`, exe)
    if (!fs.existsSync(bin)) die(`${name} 里没有 biliupR-${tag}-${t.build}/${exe}`)
    const got = inspectBinary(bin)
    if (got.format !== t.format || got.arch !== t.arch) {
      die(`${name} 的二进制是 ${got.format}/${got.arch}，应为 ${t.format}/${t.arch}`)
    }

    const pkgDir = path.join(outDir, t.dir)
    copyPackage(path.join(NPM_DIR, 'platforms', t.dir), pkgDir)
    fs.mkdirSync(path.join(pkgDir, 'bin'))
    fs.copyFileSync(bin, path.join(pkgDir, 'bin', exe))
    fs.chmodSync(path.join(pkgDir, 'bin', exe), 0o755)
    const pj = readJson(path.join(pkgDir, 'package.json'))
    pj.version = version
    writeJson(path.join(pkgDir, 'package.json'), pj)
    summary.push({ package: pj.name, asset: name, sha256: sha, bytes: fs.statSync(bin).size })
  }

  const rootOut = path.join(outDir, 'biliup')
  copyPackage(rootSrc, rootOut)
  rootPkg.version = version
  for (const dep of Object.keys(rootPkg.optionalDependencies)) rootPkg.optionalDependencies[dep] = version
  writeJson(path.join(rootOut, 'package.json'), rootPkg)
} finally {
  fs.rmSync(work, { recursive: true, force: true })
}

// 发布顺序：先平台子包，最后根包
const order = [...TARGETS.map((t) => t.dir), 'biliup']
fs.writeFileSync(path.join(outDir, 'publish-order.txt'), `${order.join('\n')}\n`)
console.table(summary)
console.log(`已生成 ${order.length} 个包：${outDir}`)
