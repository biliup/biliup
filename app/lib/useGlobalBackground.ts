'use client'
import { useEffect } from 'react'

export const BG_KEY = 'biliup_ui_bg'

export function getBg(): string {
  if (typeof window === 'undefined') return ''
  return localStorage.getItem(BG_KEY) || ''
}

export function applyBg(url: string) {
  if (typeof document === 'undefined') return
  const body = document.body
  if (url) {
    body.style.setProperty('--app-bg-image', `url("${url}")`)
    body.classList.add('app-has-bg')
  } else {
    body.style.removeProperty('--app-bg-image')
    body.classList.remove('app-has-bg')
  }
}

export function setBg(url: string) {
  if (typeof window === 'undefined') return
  if (url) localStorage.setItem(BG_KEY, url)
  else localStorage.removeItem(BG_KEY)
  applyBg(url)
}

export function validateBg(url: string): Promise<boolean> {
  return new Promise((resolve) => {
    if (!url) return resolve(false)
    if (url.startsWith('data:')) return resolve(true)
    const img = new Image()
    img.onload = () => resolve(true)
    img.onerror = () => resolve(false)
    img.src = url
  })
}

export function useGlobalBackgroundInit() {
  useEffect(() => {
    const url = getBg()
    if (!url) return
    validateBg(url).then((ok) => {
      if (ok) applyBg(url)
      else setBg('')
    })
  }, [])
}
