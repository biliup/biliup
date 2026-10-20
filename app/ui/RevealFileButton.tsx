'use client'
import { useState } from 'react'
import { Button, Toast } from '@douyinfe/semi-ui'
import { getFileLocation, revealFile, type FileLocationRequest } from '@/app/lib/files'
import styles from './reveal-file-button.module.scss'

async function copyText(text: string) {
  if (navigator.clipboard?.writeText) {
    try { await navigator.clipboard.writeText(text); return } catch { /* use fallback below */ }
  }
  const input = document.createElement('textarea')
  input.value = text
  input.style.position = 'fixed'
  input.style.opacity = '0'
  document.body.appendChild(input)
  input.select()
  const copied = document.execCommand('copy')
  input.remove()
  if (!copied) throw new Error('浏览器无法访问剪贴板，请手动复制路径。')
}

export default function RevealFileButton({ request, allowed = true }: { request: FileLocationRequest; allowed?: boolean }) {
  const [busy, setBusy] = useState(false)
  const [path, setPath] = useState('')
  const [pathCanOpen, setPathCanOpen] = useState(false)

  const locate = async () => {
    if (!allowed || busy) return
    setBusy(true)
    try {
      const location = await getFileLocation(request)
      setPath(location.path)
      setPathCanOpen(location.can_open)
      if (location.can_open) {
        try {
          await revealFile(request)
          Toast.success({ content: '已在运行 biliup 的电脑上打开所在文件夹。' })
        } catch (error) {
          Toast.error({ content: `打开文件夹失败：${error instanceof Error ? error.message : String(error)}` })
        }
      }
    } catch (error) {
      Toast.error({ content: `读取文件位置失败：${error instanceof Error ? error.message : String(error)}` })
    } finally { setBusy(false) }
  }

  const copy = async () => {
    if (!path) return
    try {
      await copyText(path)
      Toast.success({ content: '服务器文件路径已复制。' })
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : String(error) })
    }
  }

  if (!allowed) return null
  return <span className={styles.root}>
    <Button size="small" type="tertiary" loading={busy} disabled={busy} onClick={() => void locate()}>
      打开所在文件夹
    </Button>
    {path && <>
      {!pathCanOpen && <span className={styles.remote}>远程服务器文件</span>}
      {!pathCanOpen && <Button size="small" type="tertiary" onClick={() => void copy()}>复制服务器路径</Button>}
      {pathCanOpen && <Button size="small" type="tertiary" onClick={() => void copy()}>复制路径</Button>}
      <span className={styles.path} title={path}>{path}</span>
    </>}
  </span>
}
