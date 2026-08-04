import { useState, useEffect, useRef } from 'react'

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

export const useSystemTheme = () => {
  const [theme, setTheme] = useState<string>('light')
  useEffect(() => {
    const getSystemTheme = () =>
      window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    setTheme(getSystemTheme)
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    const handleChange = () => setTheme(getSystemTheme)
    mediaQuery.addEventListener('change', handleChange)
    return () => mediaQuery.removeEventListener('change', handleChange)
  }, [])
  return theme
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

export const useTheme = (mode: string, systemTheme: string) => {
  const firstRun = useRef(true)
  useEffect(() => {
    // 首屏主题已由根布局 <head> 内联脚本前置设置；这里跳过首次执行，
    // 避免水合后用默认值（auto→system）又把已保存的主题覆盖掉一次，造成闪烁。
    if (firstRun.current) {
      firstRun.current = false
      return
    }
    localStorage.setItem('mode', mode)
    const actualMode = (mode === 'auto' ? systemTheme : mode) as 'light' | 'dark'
    applyThemeMode(actualMode)
  }, [mode, systemTheme])
}
