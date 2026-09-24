'use client'
import React, { useCallback, useEffect, useState } from 'react'
import { Button, Input, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconFlag } from '@douyinfe/semi-icons'
import type { LiveStreamerEntity } from '@/app/lib/api-streamer'
import { useMe } from '@/app/lib/use-me'
import {
  createLiveMarker,
  deleteMarker,
  formatSessionTime,
  type Marker,
  playerLatencyMs,
  renameMarker,
  ReportedError,
} from '@/app/lib/markers'
import styles from './markers.module.scss'

/** 标记后的 Toast 停留多久；鼠标悬停、正在改名时不计时 */
const TOAST_MS = 6000
const TOAST_DONE_MS = 1800
const MAX_LABEL_CHARS = 100

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

/** 本场标记数（主播卡片、监视器小窗）；0 个不显示 */
export function MarkerCount({
  count,
  compact = false,
}: {
  count?: number | null
  compact?: boolean
}) {
  if (!count) return null
  return (
    <span
      className={styles.count}
      data-compact={compact || undefined}
      title={`本场已打 ${count} 个标记`}
      aria-label={`本场 ${count} 个标记`}
    >
      <IconFlag size="small" aria-hidden="true" />
      {count}
    </span>
  )
}

/** 焦点在输入框里（改名、搜索）时，M 是在打字，不是标记 */
function isTyping(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null
  return !!el && (el.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName))
}

function markDisabledReason(
  streamer: LiveStreamerEntity,
  canEdit: boolean,
  meLoading: boolean
): string | null {
  if (meLoading) return '正在读取权限…'
  if (!canEdit) return '只读观察者不能打标记：需要 clip.edit 权限，请让管理员把你的角色改成操作员'
  if (streamer.session_id == null) return '这一场还没有写出第一个分段，画面开始写盘后才能标记'
  return null
}

type ToastMode = 'idle' | 'renaming' | 'saving' | 'done'

/** 标记后的 Toast：显示标在场次的哪一刻，可以撤销或改名，不抢焦点、不挡画面 */
function MarkerToast({ marker, onClose }: { marker: Marker; onClose: () => void }) {
  const [mode, setMode] = useState<ToastMode>('idle')
  const [label, setLabel] = useState(marker.label)
  const [saved, setSaved] = useState(marker.label)
  const [note, setNote] = useState<string | null>(null)
  const [hover, setHover] = useState(false)

  useEffect(() => {
    if (hover || mode === 'renaming' || mode === 'saving') return
    const t = setTimeout(onClose, mode === 'done' ? TOAST_DONE_MS : TOAST_MS)
    return () => clearTimeout(t)
  }, [hover, mode, onClose])

  const undo = () => {
    setMode('saving')
    deleteMarker(marker.session_id, marker.id).then(
      () => {
        setNote('已撤销这个标记')
        setMode('done')
      },
      (e: unknown) => {
        setNote(`撤销失败：${errorText(e)}`)
        setMode('idle')
      }
    )
  }
  const save = () => {
    setMode('saving')
    renameMarker(marker.session_id, marker.id, label).then(
      m => {
        setSaved(m.label)
        setNote(m.label ? `已改名为「${m.label}」` : '已清空名字')
        setMode('done')
      },
      (e: unknown) => {
        setNote(`改名失败：${errorText(e)}`)
        setMode('renaming')
      }
    )
  }

  const time = formatSessionTime(marker.at_ms)
  return (
    <div
      className={styles.toast}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      data-mode={mode}
    >
      {mode === 'renaming' || (mode === 'saving' && label !== saved) ? (
        <div className={styles.toastRow}>
          <Input
            size="small"
            autoFocus
            value={label}
            maxLength={MAX_LABEL_CHARS}
            placeholder={`给 ${time} 的标记起个名字`}
            aria-label="标记名"
            disabled={mode === 'saving'}
            onChange={v => setLabel(v)}
            onEnterPress={save}
            onKeyDown={e => {
              if (e.key === 'Escape') {
                e.stopPropagation()
                setLabel(saved)
                setMode('idle')
              }
            }}
            className={styles.toastInput}
          />
          <Button size="small" theme="solid" loading={mode === 'saving'} onClick={save}>
            保存
          </Button>
          <Button
            size="small"
            theme="borderless"
            type="tertiary"
            disabled={mode === 'saving'}
            onClick={() => {
              setLabel(saved)
              setMode('idle')
            }}
          >
            取消
          </Button>
        </div>
      ) : (
        <div className={styles.toastRow}>
          <span className={styles.toastText}>
            {mode === 'done' && note ? (
              note
            ) : (
              <>
                已标记 <b>{time}</b>
                {saved ? <span className={styles.toastLabel}>「{saved}」</span> : null}
              </>
            )}
          </span>
          {mode === 'done' ? null : (
            <span className={styles.toastActions}>
              <Button
                size="small"
                theme="borderless"
                disabled={mode === 'saving'}
                onClick={() => setMode('renaming')}
              >
                改名
              </Button>
              <Button
                size="small"
                theme="borderless"
                type="danger"
                loading={mode === 'saving'}
                onClick={undo}
              >
                撤销
              </Button>
            </span>
          )}
        </div>
      )}
      {note && mode !== 'done' ? <div className={styles.toastNote}>{note}</div> : null}
    </div>
  )
}

function showMarkerToast(marker: Marker) {
  const id = `marker-${marker.id}`
  const close = () => Toast.close(id)
  Toast.success({ id, duration: 0, content: <MarkerToast marker={marker} onClose={close} /> })
}

/**
 * 预览弹层里的「标记」：按钮 + 快捷键 M（弹层打开期间）。
 * 标记的是屏幕上正在放的这一帧：按下时刻减去播放器延迟，由服务端换算成场次时间。
 */
export function LiveMarkBar({
  streamer,
  playerRoot,
  active,
}: {
  streamer: LiveStreamerEntity
  /** 播放区容器，从里面找 `<video>` 读缓冲深度 */
  playerRoot: React.RefObject<HTMLDivElement | null>
  /** 弹层打开时才响应快捷键 */
  active: boolean
}) {
  const { Text } = Typography
  const { can, isLoading } = useMe()
  const reason = markDisabledReason(streamer, can('clip.edit'), isLoading)
  const sessionId = streamer.session_id ?? null
  const count = streamer.marker_count ?? 0

  const mark = useCallback(() => {
    if (reason !== null || sessionId === null) {
      Toast.warning({ id: 'marker-disabled', content: reason ?? '现在不能标记', duration: 3 })
      return
    }
    const pressedAt = Date.now()
    const latencyMs = playerLatencyMs(playerRoot.current)
    createLiveMarker(sessionId, { pressedAt, latencyMs }).then(showMarkerToast, (e: unknown) => {
      if (!(e instanceof ReportedError)) {
        Toast.error({ content: `标记失败：${errorText(e)}`, duration: 4 })
      }
    })
  }, [reason, sessionId, playerRoot])

  useEffect(() => {
    if (!active) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'm' && e.key !== 'M') return
      if (e.repeat || e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target)) return
      e.preventDefault()
      mark()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [active, mark])

  const button = (
    <Button
      className={styles.markBtn}
      theme="solid"
      icon={<IconFlag />}
      disabled={reason !== null}
      onClick={mark}
      aria-keyshortcuts="M"
    >
      标记
      <kbd className={styles.kbd} aria-hidden="true">
        M
      </kbd>
    </Button>
  )
  return (
    <div className={styles.bar} data-disabled={reason !== null || undefined}>
      {reason !== null ? (
        <Tooltip content={reason}>
          {/* disabled 按钮不触发鼠标事件，包一层让 tooltip 仍能弹出 */}
          <span className={styles.disabledWrap}>{button}</span>
        </Tooltip>
      ) : (
        button
      )}
      <Text type="tertiary" size="small" className={styles.hint}>
        {reason ?? '标记正在放的画面（默认带上之前 60 秒），之后在剪辑台里从这里剪'}
      </Text>
      {count > 0 ? (
        <span className={styles.barCount} aria-label={`本场 ${count} 个标记`}>
          <IconFlag size="small" aria-hidden="true" />
          本场 {count} 个
        </span>
      ) : null}
    </div>
  )
}
