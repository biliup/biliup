'use client'
import { useEffect, useRef, useState } from 'react'
import { Button, Slider, Toast, Tooltip } from '@douyinfe/semi-ui'
import { IconImage, IconUpload, IconDelete } from '@douyinfe/semi-icons'
import {
  getBg,
  setBg,
  getBgOpacity,
  setBgOpacity,
} from '@/app/lib/useGlobalBackground'
import styles from './BackgroundSetter.module.scss'

/**
 * 背景图压缩:统一压成 JPEG 并限制体积,避免超大的 data URL 塞进
 * localStorage(配额约 5MB)导致刷新后背景丢失。
 */
const MAX_BG_BYTES = 2 * 1024 * 1024 // 压缩后上限 2MB(data URL 字符数约等于字节数×1.33)
const MAX_BG_DIMENSION = 1600

function compressImage(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    // 仅接受可转 JPEG 的位图格式;动图(GIF/AVIF)会被取首帧、特殊格式会失真或过大,不在此列
    if (!/^image\/(jpeg|png|webp|bmp)$/i.test(file.type)) {
      reject(new Error('不支持的图片格式'))
      return
    }
    const reader = new FileReader()
    reader.onerror = reject
    reader.onload = () => {
      const img = new Image()
      img.onerror = reject
      img.onload = () => {
        const max = MAX_BG_DIMENSION
        let { width, height } = img
        if (width > max || height > max) {
          const ratio = Math.min(max / width, max / height)
          width = Math.round(width * ratio)
          height = Math.round(height * ratio)
        }
        const canvas = document.createElement('canvas')
        canvas.width = width
        canvas.height = height
        const ctx = canvas.getContext('2d')
        if (!ctx) {
          reject(new Error('图片处理失败'))
          return
        }
        ctx.drawImage(img, 0, 0, width, height)
        // 统一压成 JPEG;1600px 上限下体积通常在几百 KB,这里再做一层硬校验,
        // 极端高分辨率图若仍超限则拒绝,避免超大 data URL 撑爆 localStorage。
        const dataUrl = canvas.toDataURL('image/jpeg', 0.7)
        if (dataUrl.length > MAX_BG_BYTES) {
          reject(new Error('图片过大,请换一张更小的图片'))
          return
        }
        resolve(dataUrl)
      }
      img.src = reader.result as string
    }
    reader.readAsDataURL(file)
  })
}

export default function BackgroundSetter() {
  const fileRef = useRef<HTMLInputElement>(null)
  const [open, setOpen] = useState(false)
  const [hasBg, setHasBg] = useState(false)
  const [opacity, setOpacity] = useState(0.35)

  useEffect(() => {
    setHasBg(!!getBg())
    setOpacity(getBgOpacity())
  }, [])

  const onFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return
    if (!file.type.startsWith('image/')) {
      Toast.error('请选择图片文件')
      return
    }
    try {
      const data = await compressImage(file)
      setBg(data)
      setHasBg(true)
      setOpen(false)
      Toast.success('背景已应用')
    } catch (err: any) {
      // 区分具体失败原因:格式不支持 / 图片过大 / 处理失败
      Toast.error(err?.message || '图片处理失败')
    }
    e.target.value = ''
  }

  const clear = () => {
    setBg('')
    setHasBg(false)
    setOpen(false)
    Toast.success('已恢复默认背景')
  }

  const onOpacityChange = (v: number | number[] | undefined) => {
    const raw = Array.isArray(v) ? (v[0] ?? 0) : (v ?? 0)
    const val = Math.min(0.8, Math.max(0.05, Number(raw) / 100))
    setOpacity(val)
    setBgOpacity(val)
  }

  return (
    <div className={`${styles.float} ${open ? styles.open : ''}`}>
      <input ref={fileRef} type="file" accept="image/*" hidden onChange={onFile} />

      {open && (
        <>
          <div className={styles.btnRow}>
            <Tooltip content="上传背景" position="top">
              <Button
                theme="borderless"
                size="small"
                icon={<IconUpload />}
                onClick={() => fileRef.current?.click()}
              />
            </Tooltip>
            <Tooltip content="清除背景" position="top">
              <Button
                theme="borderless"
                size="small"
                icon={<IconDelete />}
                disabled={!hasBg}
                onClick={clear}
              />
            </Tooltip>
          </div>

          <div className={styles.opacityRow}>
            <div className={styles.opacityLabel}>
              <span>遮罩透明度</span>
              <span>{Math.round(opacity * 100)}%</span>
            </div>
            <Slider
              min={5}
              max={80}
              step={5}
              value={Math.round(opacity * 100)}
              onChange={onOpacityChange}
              tipFormatter={(v) => `${v}%`}
            />
          </div>
        </>
      )}

      <Tooltip content="背景设置" position="top">
        <Button
          theme={open ? 'solid' : 'borderless'}
          size="small"
          icon={<IconImage />}
          onClick={() => setOpen((v) => !v)}
        />
      </Tooltip>
    </div>
  )
}
