'use client'
import { useEffect } from 'react'

export const BG_KEY = 'biliup_ui_bg'
export const BG_OPACITY_KEY = 'biliup_ui_bg_opacity'
/** 默认遮罩透明度(0~1),保证文字可读 */
export const DEFAULT_BG_OPACITY = 0.35

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

/** 读取遮罩透明度(0.05~0.9,默认 0.35) */
export function getBgOpacity(): number {
  if (typeof window === 'undefined') return DEFAULT_BG_OPACITY
  const raw = parseFloat(localStorage.getItem(BG_OPACITY_KEY) ?? '')
  if (Number.isNaN(raw)) return DEFAULT_BG_OPACITY
  return Math.min(0.9, Math.max(0.05, raw))
}

/** 设置遮罩透明度并立即生效 */
export function setBgOpacity(v: number) {
  if (typeof window === 'undefined') return
  const clamped = Math.min(0.9, Math.max(0.05, v))
  try {
    localStorage.setItem(BG_OPACITY_KEY, String(clamped))
  } catch {
    /* ignore */
  }
  document.body.style.setProperty('--app-bg-mask-opacity', String(clamped))
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
  // 同步遮罩透明度(默认 0.35,用户可调)
  body.style.setProperty('--app-bg-mask-opacity', String(getBgOpacity()))
}

export function setBg(url: string) {
  if (typeof window === 'undefined') return
  if (url && !isAllowedBg(url)) {
    // 拒绝远程 URL，避免外链背景带来的各类副作用
    console.warn('[biliup] 仅支持本地图片作为背景，远程 URL 已被忽略')
    return
  }
  // localStorage 写入失败(如配额满)不阻断:背景仍即时应用,仅刷新后丢失
  if (url) {
    try {
      localStorage.setItem(BG_KEY, url)
    } catch {
      console.warn('[biliup] 背景图超过 localStorage 配额,本次仅即时生效')
    }
  } else {
    try {
      localStorage.removeItem(BG_KEY)
    } catch {
      /* ignore */
    }
  }
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
    // 先同步透明度(即使无背景也无害,避免首帧闪烁)
    document.body.style.setProperty('--app-bg-mask-opacity', String(getBgOpacity()))
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
