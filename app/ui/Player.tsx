import React, { useEffect, useRef } from 'react'
import Artplayer from 'artplayer'
import mpegts from 'mpegts.js'

type VideoPlayer = Artplayer | null

interface PlayerConfig {
  url: string
  height?: string
  width?: string
}

function playFlv(video: HTMLVideoElement, url: string, art: Artplayer) {
  if (mpegts.isSupported()) {
    if (art.flv) {
      art.flv.destroy()
      art.flv = null
    }

    const flv = mpegts.createPlayer({
      type: 'flv',
      url: url,
    })

    art.flv = flv
    art.on('destroy', () => {
      if (art.flv) {
        art.flv.destroy()
        art.flv = null
      }
    })

    flv.attachMediaElement(video)
    flv.load()
  } else {
    art.notice.show = 'Unsupported playback format: flv'
  }
}

const Players: React.FC<PlayerConfig> = ({ url, height = '100%', width = '100%' }) => {
  const containerRef = useRef<HTMLDivElement>(null)
  const playerRef = useRef<VideoPlayer>(null)

  useEffect(() => {
    if (!containerRef.current) return

    try {
      if (playerRef.current) {
        playerRef.current.destroy()
        playerRef.current = null
      }

      if (url.endsWith('.flv')) {
        playerRef.current = new Artplayer({
          container: containerRef.current,
          url,
          type: 'flv',
          customType: {
            flv: playFlv,
          },
          autoSize: true,
          fullscreen: true,
          fullscreenWeb: true,
          autoOrientation: true,
          plugins: [],
        })
      } else {
        playerRef.current = new Artplayer({
          container: containerRef.current,
          url,
          autoSize: true,
          fullscreen: true,
          fullscreenWeb: true,
          autoOrientation: true,
          plugins: [],
        })
      }
    } catch (error) {
      console.error('播放器初始化失败:', error)
    }

    return () => {
      if (playerRef.current) {
        playerRef.current.destroy()
        playerRef.current = null
      }
    }
  }, [url, height, width])

  return <div ref={containerRef} style={{ width, height }} />
}

export default Players
