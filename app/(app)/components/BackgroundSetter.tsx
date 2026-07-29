'use client'
import { useEffect, useRef, useState } from 'react'
import { Button, Toast, Tooltip } from '@douyinfe/semi-ui'
import { IconImage, IconUpload, IconDelete } from '@douyinfe/semi-icons'
import { getBg, setBg } from '@/app/lib/useGlobalBackground'
import styles from './BackgroundSetter.module.scss'

function compressImage(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onerror = reject
    reader.onload = () => {
      const img = new Image()
      img.onerror = reject
      img.onload = () => {
        const max = 1600
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
          resolve(reader.result as string)
          return
        }
        ctx.drawImage(img, 0, 0, width, height)
        resolve(canvas.toDataURL('image/jpeg', 0.85))
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

  useEffect(() => {
    setHasBg(!!getBg())
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
    } catch {
      Toast.error('图片处理失败')
    }
    e.target.value = ''
  }

  const clear = () => {
    setBg('')
    setHasBg(false)
    setOpen(false)
    Toast.success('已恢复默认背景')
  }

  return (
    <div className={styles.float}>
      <input ref={fileRef} type="file" accept="image/*" hidden onChange={onFile} />
      {open && (
        <>
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
        </>
      )}
      <Tooltip content="背景设置" position="top">
        <Button
          theme={open ? 'solid' : 'borderless'}
          size="small"
          icon={<IconImage />}
          onClick={() => setOpen(v => !v)}
        />
      </Tooltip>
    </div>
  )
}
