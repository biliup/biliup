'use client'
import { useEffect } from 'react'

export const BG_KEY = 'biliup_ui_bg'

// 仅允许本地 data URL（base64/blob）作为背景，禁止一切远程 http/https 外链背景，
// 避免外链失效 / CDN 慢 / 图片追踪 / 混合内容等问题。
function isRemoteUrl(url: string): boolean {
  // 匹配 http://、https://、以及协议相对地址 //xxx
  return /^(https?:)?\/\//i.test(url)
}

function isAllowedBg(url: string): boolean {
  // 空串表示清除；非空必须是以 data: 开头的本地资源
  if (!url) return true
  return url.startsWith('data:') && !isRemoteUrl(url)
}

export function getBg(): string {
  if (typeof window === 'undefined') return ''
  return localStorage.getItem(BG_KEY) || ''
}

export function applyBg(url: string) {
  if (typeof document === 'undefined') return
  const body = document.body
  // 去除可能引发 CSS 注入的引号/反斜杠，data URL 本身不含这些字符
  const safe = url.replace(/["\\]/g, '')
  if (url) {
    body.style.setProperty('--app-bg-image', `url("${safe}")`)
    body.classList.add('app-has-bg')
  } else {
    body.style.removeProperty('--app-bg-image')
    body.classList.remove('app-has-bg')
  }
}

export function setBg(url: string) {
  if (typeof window === 'undefined') return
  if (url && !isAllowedBg(url)) {
    // 拒绝远程 URL，避免外链背景带来的各类副作用
    console.warn('[biliup] 仅支持本地图片作为背景，远程 URL 已被忽略')
    return
  }
  if (url) localStorage.setItem(BG_KEY, url)
  else localStorage.removeItem(BG_KEY)
  applyBg(url)
}

export function validateBg(url: string): Promise<boolean> {
  return new Promise((resolve) => {
    if (!url) return resolve(false)
    // 仅接受本地 data URL，不加载任何远程资源（不再发起网络请求）
    if (url.startsWith('data:')) return resolve(true)
    return resolve(false)
  })
}

export function useGlobalBackgroundInit() {
  useEffect(() => {
    const url = getBg()
    if (!url) return
    // 清理历史遗留的远程背景配置，避免旧数据继续生效
    if (!isAllowedBg(url)) {
      setBg('')
      return
    }
    validateBg(url).then((ok) => {
      if (ok) applyBg(url)
      else setBg('')
    })
  }, [])
}
