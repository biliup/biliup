import { useCallback, useEffect, useRef, useSyncExternalStore } from 'react'

export const responsiveMap = {
  xs: '(max-width: 575px)',
  sm: '(min-width: 576px)',
  md: '(min-width: 768px)',
  lg: '(min-width: 992px)',
  xl: '(min-width: 1200px)',
  xxl: '(min-width: 1600px)',
}

export interface RegisterMediaQueryOption {
  match?: (e: MediaQueryList | MediaQueryListEvent) => void
  unmatch?: (e: MediaQueryList | MediaQueryListEvent) => void
  callInInit?: boolean
}

/**
 * register matchFn and unMatchFn callback while media query
 * @param {string} media media string
 * @param {object} param param object
 * @returns function
 */
export const registerMediaQuery = (
  media: string,
  { match, unmatch, callInInit = true }: RegisterMediaQueryOption
): (() => void) => {
  if (typeof window !== 'undefined') {
    const mediaQueryList = window.matchMedia(media)
    const handlerMediaChange = function (e: MediaQueryList | MediaQueryListEvent): void {
      if (e.matches) {
        match && match(e)
      } else {
        unmatch && unmatch(e)
      }
    }
    callInInit && handlerMediaChange(mediaQueryList)
    if (Object.prototype.hasOwnProperty.call(mediaQueryList, 'addEventListener')) {
      mediaQueryList.addEventListener('change', handlerMediaChange)
      return (): void => mediaQueryList.removeEventListener('change', handlerMediaChange)
    }
    mediaQueryList.addListener(handlerMediaChange)
    return (): void => mediaQueryList.removeListener(handlerMediaChange)
  }
  return () => undefined
}

export const humDate = (time: number): string =>
  new Date(time * 1000)
    .toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    })
    .replaceAll('/', '-')

const DARK_SCHEME_QUERY = '(prefers-color-scheme: dark)'
const subscribeSystemTheme = (onChange: () => void) => {
  const mediaQuery = window.matchMedia(DARK_SCHEME_QUERY)
  mediaQuery.addEventListener('change', onChange)
  return () => mediaQuery.removeEventListener('change', onChange)
}
const getSystemTheme = () => (window.matchMedia(DARK_SCHEME_QUERY).matches ? 'dark' : 'light')
const getServerSystemTheme = () => 'light'

/**
 * 系统配色(light / dark)。matchMedia 是浏览器侧的外部状态,用 useSyncExternalStore 订阅:
 * 服务端与水合期固定为 light(与 SSR 输出一致),水合后 React 自行切到真实值,不需要在 effect 里 setState。
 */
export const useSystemTheme = () =>
  useSyncExternalStore(subscribeSystemTheme, getSystemTheme, getServerSystemTheme)

/* ---------- localStorage 偏好(侧栏折叠、主题模式)的订阅式读写 ---------- */

const storageListeners = new Map<string, Set<() => void>>()
/** localStorage 写入失败(配额满、被禁用)时的内存兜底,保证本页内仍能切换,只是刷新后不保留 */
const storageFallback = new Map<string, string | null>()

const readStorage = (key: string): string | null => {
  if (storageFallback.has(key)) return storageFallback.get(key) ?? null
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}
const writeStorage = (key: string, value: string | null) => {
  try {
    if (value === null) localStorage.removeItem(key)
    else localStorage.setItem(key, value)
    storageFallback.delete(key)
  } catch {
    storageFallback.set(key, value)
  }
  storageListeners.get(key)?.forEach(listener => listener())
}
const subscribeStorage = (key: string, onChange: () => void) => {
  let listeners = storageListeners.get(key)
  if (!listeners) {
    listeners = new Set()
    storageListeners.set(key, listeners)
  }
  listeners.add(onChange)
  return () => {
    listeners.delete(onChange)
  }
}
const getServerStorage = () => null

/**
 * 读写 localStorage 里的一个键,并订阅它在本页内的变化。
 * 替代「useState + 挂载 effect 里 setState」的写法:服务端与水合期快照固定为 null(与 SSR 输出一致),
 * 水合完成后 React 自己用真实值重渲染一次,既没有水合不一致也没有级联渲染。
 * 同一个键的写入都要走返回的 setter,订阅者才会收到通知;不要在别处直接改 localStorage 里的这个键。
 */
export const useLocalStorageValue = (key: string) => {
  const subscribe = useCallback((onChange: () => void) => subscribeStorage(key, onChange), [key])
  const getSnapshot = useCallback(() => readStorage(key), [key])
  const value = useSyncExternalStore(subscribe, getSnapshot, getServerStorage)
  const setValue = useCallback((next: string | null) => writeStorage(key, next), [key])
  return [value, setValue] as const
}

/**
 * 同步主题属性到 <html> 与 <body>。
 * Semi Design 的 CSS 变量绑定在 body[theme-mode] 上；
 * 我们自己的 CSS（如 .shadow）与 no-flash 脚本使用 <html> 上的 theme-mode。
 * 两者都写才能同时兼容 Semi 与自定义选择器。
 */
export const applyThemeMode = (mode: 'light' | 'dark') => {
  if (typeof document === 'undefined') return
  document.documentElement.setAttribute('theme-mode', mode)
  document.body.setAttribute('theme-mode', mode)
}

/**
 * mode / systemTheme 变化时把实际主题写到 DOM 上。
 * mode 的持久化由 useLocalStorageValue('mode') 的 setter 负责，这里不再回写 localStorage：
 * 两处都写会在 StrictMode 的双次 effect 下用水合期的默认值 auto 把刚读到的已保存主题覆盖掉。
 */
export const useTheme = (mode: string, systemTheme: string) => {
  const firstRun = useRef(true)
  useEffect(() => {
    // 首屏主题已由根布局 <head> 内联脚本前置设置；这里跳过首次执行，
    // 避免水合后用默认值（auto→system）又把已保存的主题覆盖掉一次，造成闪烁。
    if (firstRun.current) {
      firstRun.current = false
      return
    }
    const actualMode = (mode === 'auto' ? systemTheme : mode) as 'light' | 'dark'
    applyThemeMode(actualMode)
  }, [mode, systemTheme])
}
