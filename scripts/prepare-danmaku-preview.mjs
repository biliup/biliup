import { createHash, randomUUID } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs'
import { gzipSync } from 'node:zlib'
import { transformSync, version } from 'esbuild'

const into = file => new URL(file, import.meta.url)
const read = file => readFileSync(into(file), 'utf8').replace(/\r\n/g, '\n')
const js = read('../resources/danmaku-preview/viewer.js')
const css = read('../resources/danmaku-preview/viewer.css')
const standaloneJs = js.replace(/^[ \t]*\/\* In-app only \*\/\n[\s\S]*?^[ \t]*\/\* End in-app only \*\/\n/gm, '')
const standaloneCss = css.split('\n/* In-app preview */\n', 1)[0]
const fallback = 'i{display:block;color-scheme:light dark;font:14px system-ui,sans-serif}i>d{display:block}i>d::before{content:attr(p) "  "}'

async function loadPreview(encoded) {
  const bytes = Uint8Array.from(atob(encoded), char => char.charCodeAt(0))
  const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream('gzip'))
  const [css, code] = await new Response(stream).json()
  for (const [tag, text] of [['style', css], ['script', code]]) {
    const node = document.createElementNS('http://www.w3.org/1999/xhtml', tag)
    node.textContent = text
    document.documentElement.append(node)
  }
}

// Strip app-only code before compressing the fragment embedded in recordings.
for (const inApp of [false, true]) {
  const code = (inApp ? js : standaloneJs).replace('__DANMAKU_DATA__', inApp ? 'data' : 'root')
  const styles = inApp ? css : standaloneCss
  const file = into(inApp ? '../build/danmaku-preview-app.xml' : '../build/danmaku-preview.xml')
  const revision = createHash('sha256')
    .update(JSON.stringify([version, read(import.meta.url), inApp, code, styles]))
    .digest('hex').slice(0, 16)
  const header = '\n\t<!-- biliup-danmaku-preview:' + revision + ' -->\n'
  if (existsSync(file) && readFileSync(file, 'utf8').startsWith(header)) continue

  const minify = (source, loader) => transformSync(source, { loader, minify: true }).code.trimEnd()
  const bundle = JSON.stringify([minify(styles, 'css'), minify(code, 'js')])
  const payload = gzipSync(bundle, { level: 9 }).toString('base64')
  const loader = minify('(' + loadPreview + ')(' + JSON.stringify(payload) + ')', 'js')
  const tag = (name, text) =>
    '\t<h:' + name + ' xmlns:h="http://www.w3.org/1999/xhtml"><![CDATA[' + text + ']]></h:' + name + '>'
  const xml = header + tag('style', fallback) + '\n' + tag('script', loader)
  mkdirSync(into('../build/'), { recursive: true })
  const temp = new URL('danmaku-preview-' + randomUUID() + '.tmp', file)
  try {
    writeFileSync(temp, xml, { flag: 'wx' })
    renameSync(temp, file)
  } finally {
    rmSync(temp, { force: true })
  }
}
