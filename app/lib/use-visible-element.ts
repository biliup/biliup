'use client'
import { useEffect, useRef, useState } from 'react'

/** Network refreshes and charts only run near the viewport in a visible tab. */
export function useVisibleElement<T extends HTMLElement>() {
  const ref = useRef<T>(null)
  const [active, setActive] = useState(false)
  useEffect(() => {
    const element = ref.current
    if (!element) return
    let near = false
    const update = () => setActive(near && document.visibilityState === 'visible')
    const observer = new IntersectionObserver(([entry]) => {
      near = entry.isIntersecting
      update()
    }, { rootMargin: '150px' })
    observer.observe(element)
    document.addEventListener('visibilitychange', update)
    return () => { observer.disconnect(); document.removeEventListener('visibilitychange', update) }
  }, [])
  return { ref, active }
}
