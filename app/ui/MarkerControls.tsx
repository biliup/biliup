'use client'
import React, { useCallback, useEffect, useState } from 'react'
import { Button, Input, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconDownload, IconFlag, IconHistory, IconScissors } from '@douyinfe/semi-icons'
import type { LiveStreamerEntity } from '@/app/lib/api-streamer'
import { useMe } from '@/app/lib/use-me'
import { formatSize } from '@/app/lib/use-dashboard'
import { replayHref } from '@/app/lib/sessions'
import { type Clip, createLiveClip, downloadClip, exportClip, LAST_CLIP_OPTIONS, useClip } from '@/app/lib/clips'
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
export function isTyping(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null
  return !!el && (el.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName))
}

function markDisabledReason(
  streamer: LiveStreamerEntity,
  canEdit: boolean,
  meLoading: boolean
): string | null {
  if (meLoading) return '正在读取权限…'
  if (!canEdit) return '只读观察者不能打标记和剪切片：需要 clip.edit 权限，请让管理员把你的角色改成操作员'
  if (streamer.session_id == null) return '这一场还没有写出第一个分段，画面开始写盘后才能标记和剪'
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

export function showMarkerToast(marker: Marker) {
  const id = `marker-${marker.id}`
  const close = () => Toast.close(id)
  Toast.success({ id, duration: 0, content: <MarkerToast marker={marker} onClose={close} /> })
}

/**
 * 「剪下刚才 N 秒」之后的 Toast：跟着导出进度走；剪好了给下载和「在回看里打开」，失败了给原因和重试。
 * 导出中、失败时一直留着（右上角可关），剪好后停留一会儿自动关。
 */
function ClipToast({
  initial,
  span,
  canDownload,
  onClose,
}: {
  initial: Clip
  span: string
  canDownload: boolean
  onClose: () => void
}) {
  const { data, error } = useClip(initial.id)
  const clip = data ?? initial
  const gone = !data && !!error && /不存在|删掉|404/.test(errorText(error))
  const [hover, setHover] = useState(false)
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<string | null>(null)

  const ready = clip.state === 'ready'
  useEffect(() => {
    if (!ready || hover) return
    const t = setTimeout(onClose, TOAST_MS * 2)
    return () => clearTimeout(t)
  }, [ready, hover, onClose])

  const act = (run: () => Promise<unknown>, failure: string) => {
    setBusy(true)
    setNote(null)
    run().then(
      () => setBusy(false),
      (e: unknown) => {
        setBusy(false)
        if (!(e instanceof ReportedError)) setNote(`${failure}：${errorText(e)}`)
      }
    )
  }
  const ratio = clip.progress?.ratio ?? null
  return (
    <div
      className={styles.toast}
      data-clip-toast={clip.id}
      data-clip-state={gone ? 'gone' : clip.state}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
    >
      <div className={styles.toastRow}>
        <span className={styles.toastText} role="status">
          {gone ? (
            <>这个切片已经被删掉了</>
          ) : clip.state === 'exporting' ? (
            <>
              正在快速剪刚才 <b>{span}</b> · {clip.progress?.phase ?? '准备中'}
              {ratio !== null ? ` ${Math.round(ratio * 100)}%` : ''}
            </>
          ) : ready ? (
            <>
              已剪好刚才 <b>{span}</b>
              {clip.output_bytes !== null ? `（${formatSize(clip.output_bytes)}）` : ''}
            </>
          ) : clip.state === 'failed' ? (
            <>没剪成：{clip.error || '原因未知'}</>
          ) : (
            <>切片已存下，还没导出</>
          )}
        </span>
        {gone ? null : (
          <span className={styles.toastActions}>
            {ready && canDownload ? (
              <Button
                size="small"
                theme="borderless"
                icon={<IconDownload />}
                loading={busy}
                onClick={() => act(() => downloadClip(clip, 'source'), '下载失败')}
              >
                下载
              </Button>
            ) : null}
            {clip.state === 'failed' || clip.state === 'draft' ? (
              <Button
                size="small"
                theme="borderless"
                loading={busy}
                onClick={() => act(() => exportClip(clip, 'quick'), '没能重试')}
              >
                {clip.state === 'failed' ? '重试' : '快速剪'}
              </Button>
            ) : null}
            {/* 新标签页打开，不打断正在看的直播 */}
            <Button
              size="small"
              theme="borderless"
              icon={<IconHistory />}
              onClick={() => window.open(replayHref(clip.session_id, clip.in_ms, clip.id), '_blank', 'noopener')}
            >
              在回看里打开
            </Button>
          </span>
        )}
      </div>
      {note ? <div className={styles.toastNote}>{note}</div> : null}
    </div>
  )
}

function showClipToast(clip: Clip, span: string, canDownload: boolean) {
  const id = `clip-${clip.id}`
  const close = () => Toast.close(id)
  Toast.info({
    id,
    duration: 0,
    icon: <IconScissors />,
    content: <ClipToast initial={clip} span={span} canDownload={canDownload} onClose={close} />,
  })
}

/** 「剪下刚才 N 秒」的一组按钮：按屏幕上正在放的这一帧往前剪，建好立即快速剪 */
function LastClipButtons({
  sessionId,
  reason,
  canDownload,
  playerRoot,
}: {
  sessionId: number | null
  reason: string | null
  canDownload: boolean
  playerRoot: React.RefObject<HTMLDivElement | null>
}) {
  const [pending, setPending] = useState<number | null>(null)
  const cut = useCallback(
    (lastMs: number, span: string) => {
      if (reason !== null || sessionId === null) {
        Toast.warning({ id: 'clip-disabled', content: reason ?? '现在不能剪', duration: 3 })
        return
      }
      const pressedAt = Date.now()
      const latencyMs = playerLatencyMs(playerRoot.current)
      setPending(lastMs)
      createLiveClip(sessionId, { lastMs, pressedAt, latencyMs }).then(
        (clip) => {
          setPending(null)
          showClipToast(clip, span, canDownload)
        },
        (e: unknown) => {
          setPending(null)
          if (!(e instanceof ReportedError)) Toast.error({ content: `没剪成：${errorText(e)}`, duration: 5 })
        }
      )
    },
    [reason, sessionId, canDownload, playerRoot]
  )
  return (
    <span className={styles.clipGroup} role="group" aria-label="剪下刚才">
      <span className={styles.clipLabel}>
        <IconScissors size="small" aria-hidden="true" />
        剪下刚才
      </span>
      {LAST_CLIP_OPTIONS.map((o) => {
        const button = (
          <Button
            className={styles.clipBtn}
            theme="light"
            disabled={reason !== null}
            loading={pending === o.ms}
            onClick={() => cut(o.ms, o.label)}
            aria-label={`剪下刚才 ${o.label}`}
          >
            {o.label}
          </Button>
        )
        return (
          <Tooltip
            key={o.ms}
            content={reason ?? `把正在放的画面之前 ${o.label} 快速剪成一个文件（按关键帧切，不转码，保持录像原格式）`}
          >
            {reason !== null ? <span className={styles.disabledWrap}>{button}</span> : button}
          </Tooltip>
        )
      })}
    </span>
  )
}

/**
 * 直播预览页里的「标记」：按钮 + 快捷键 M（在预览页期间），以及「剪下刚才 30 秒 / 1 分钟 / 2 分钟」。
 * 标记和剪的都是屏幕上正在放的这一帧：按下时刻减去播放器延迟，由服务端换算成场次时间。
 */
export function LiveMarkBar({
  streamer,
  playerRoot,
  active,
}: {
  streamer: LiveStreamerEntity
  /** 播放区容器，从里面找 `<video>` 读缓冲深度 */
  playerRoot: React.RefObject<HTMLDivElement | null>
  /** 为 true 时才响应快捷键 */
  active: boolean
}) {
  const { Text } = Typography
  const { can, isLoading } = useMe()
  const reason = markDisabledReason(streamer, can('clip.edit'), isLoading)
  const canDownload = can('file.view')
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
      <LastClipButtons sessionId={sessionId} reason={reason} canDownload={canDownload} playerRoot={playerRoot} />
      <Text type="tertiary" size="small" className={styles.hint}>
        {reason ?? '标记正在放的画面（默认带上之前 60 秒），之后在录像回看里剪；或者直接剪下刚才的一段'}
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
