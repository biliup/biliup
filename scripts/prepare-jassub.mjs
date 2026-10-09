import { build } from 'esbuild'
import { mkdir, copyFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const dest = resolve(root, 'public/jassub')
await mkdir(dest, { recursive: true })
await build({ entryPoints: [resolve(root, 'node_modules/jassub/dist/worker/worker.js')],
  outfile: resolve(dest, 'worker.js'), bundle: true, format: 'esm', platform: 'browser', target: 'es2022', minify: true })
await build({ entryPoints: [resolve(root, 'node_modules/jassub/dist/jassub.js')],
  outfile: resolve(dest, 'renderer.js'), bundle: true, format: 'iife', globalName: 'BiliupASS',
  platform: 'browser', target: 'es2022', minify: true, logOverride: { 'empty-import-meta': 'silent' } })
for (const name of ['jassub-worker.wasm', 'jassub-worker-modern.wasm']) {
  await copyFile(resolve(root, 'node_modules/jassub/dist/wasm', name), resolve(dest, name))
}
await copyFile(resolve(root, 'node_modules/jassub/dist/wasm/jassub-worker.js'), resolve(dest, 'jassub-worker.js'))
await copyFile(resolve(root, 'node_modules/jassub/LICENSE'), resolve(dest, 'LICENSE'))
