'use client'
import { useSyncExternalStore } from 'react'

/**
 * 记在 localStorage 里的界面偏好（同屏路数、弹幕开关等）。
 * 用 useSyncExternalStore 而不是「useState + effect 里 setState」：服务端与首屏统一给默认值，
 * 挂载后同步读到真实值，没有水合不一致，也不触发 react-hooks/set-state-in-effect。
 */
const listeners = new Set<() => void>()

function notify() {
  listeners.forEach((l) => l())
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  window.addEventListener('storage', listener)
  return () => {
    listeners.delete(listener)
    window.removeEventListener('storage', listener)
  }
}

export function readPref(key: string): string | null {
  if (typeof window === 'undefined') return null
  try {
    return window.localStorage.getItem(key)
  } catch {
    return null
  }
}

export function writePref(key: string, value: string) {
  try {
    window.localStorage.setItem(key, value)
  } catch {
    /* 隐私模式等写不进去时就只在本次会话里生效 */
  }
  notify()
}

/** 布尔偏好。 */
export function useBoolPref(key: string, fallback: boolean): [boolean, (v: boolean) => void] {
  const value = useSyncExternalStore(
    subscribe,
    () => {
      const raw = readPref(key)
      return raw === null ? fallback : raw === '1'
    },
    () => fallback
  )
  return [value, (v: boolean) => writePref(key, v ? '1' : '0')]
}

/** 只接受给定候选值的字符串偏好（档位一类的枚举）。 */
export function useEnumPref<T extends string>(key: string, options: readonly T[], fallback: T): [T, (v: T) => void] {
  const value = useSyncExternalStore(
    subscribe,
    () => {
      const raw = readPref(key)
      return options.includes(raw as T) ? (raw as T) : fallback
    },
    () => fallback
  )
  return [value, (v: T) => writePref(key, v)]
}

/** 只接受给定候选值的数字偏好。 */
export function useChoicePref(key: string, options: number[], fallback: number): [number, (v: number) => void] {
  const value = useSyncExternalStore(
    subscribe,
    () => {
      const n = Number(readPref(key))
      return options.includes(n) ? n : fallback
    },
    () => fallback
  )
  return [value, (v: number) => writePref(key, String(v))]
}
