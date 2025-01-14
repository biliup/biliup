import { useState, useEffect } from 'react'

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

export const useTheme = (mode: string, systemTheme: string) => {
  useEffect(() => {
    localStorage.setItem('mode', mode)
    switch (mode) {
      case 'light':
        document.body.setAttribute('theme-mode', 'light')
        break
      case 'dark':
        document.body.setAttribute('theme-mode', 'dark')
        break
      default:
        document.body.setAttribute('theme-mode', systemTheme)
        break
    }
  }, [mode, systemTheme])
}
