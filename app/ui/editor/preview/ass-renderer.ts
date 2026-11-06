type AssModule = {default: typeof import('jassub').default}
let pending: Promise<AssModule> | undefined

/** Keep the worker bundle separate from Next's module graph. */
export function loadAssRenderer(): Promise<AssModule> {
  if (!pending) pending = new Promise((resolve, reject) => {
    const script = document.createElement('script')
    script.src = '/jassub/renderer.js'
    const fail = (message: string) => {
      pending = undefined
      script.remove()
      reject(new Error(message))
    }
    script.onload = () => {
      const module = (globalThis as typeof globalThis & {BiliupASS?: AssModule}).BiliupASS
      if (module?.default) resolve(module)
      else fail('ASS 渲染器加载后不可用')
    }
    script.onerror = () => fail('ASS 渲染器文件加载失败，请重新构建前端静态资源')
    document.head.append(script)
  })
  return pending
}
