import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const manifest = fs.readFileSync(process.argv[2] || path.join(root, 'Cargo.toml'), 'utf8')
const section = manifest.split(/^\[workspace\.package\]\s*$/m)[1]?.split(/^\[/m)[0]
const version = section?.match(/^version\s*=\s*"([^"]+)"/m)?.[1]
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  throw new Error('Cargo.toml must have a release version in [workspace.package]')
}
const configFile = process.argv[3] || path.join(root, 'tauri-app/src-tauri/tauri.conf.json')
const config = JSON.parse(fs.readFileSync(configFile, 'utf8'))
config.version = version
fs.writeFileSync(configFile, `${JSON.stringify(config, null, 2)}\n`)
console.log(`Desktop installer version: ${version}`)
