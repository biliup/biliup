'use client'
import { useEffect, useState } from 'react'

/**
 * 水合安全的移动端判定。
 * 关键：首屏（服务端 + 客户端首次渲染）统一返回 false，
 * 等挂载完成后再通过 effect 读 window.innerWidth 更新，
 * 避免和 react-use 的 useWindowSize 一样在渲染期读 window 导致 hydration mismatch。
 */
export function useIsMobile(breakpoint = 640): boolean {
  const [isMobile, setIsMobile] = useState(false)
  useEffect(() => {
    const check = () => setIsMobile(window.innerWidth <= breakpoint)
    check()
    window.addEventListener('resize', check)
    return () => window.removeEventListener('resize', check)
  }, [breakpoint])
  return isMobile
}

/**
 * 水合安全的视口宽度。
 * 首屏统一返回 Infinity（与 SSR 一致），挂载后由 effect 填真实值。
 */
export function useWindowWidth(): number {
  const [width, setWidth] = useState(Infinity)
  useEffect(() => {
    const update = () => setWidth(window.innerWidth)
    update()
    window.addEventListener('resize', update)
    return () => window.removeEventListener('resize', update)
  }, [])
  return width
}
