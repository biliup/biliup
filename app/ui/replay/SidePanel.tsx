'use client'
import React, { useState } from 'react'
import { Button, Empty, Input, Popconfirm, Progress, TabPane, Tabs, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconDelete, IconEdit, IconFlag, IconScissors } from '@douyinfe/semi-icons'
import { deleteMarker, formatSessionTime, type Marker, renameMarker, ReportedError } from '@/app/lib/markers'
import { type Clip, clipModeText, deleteClip, extensionOf, MAX_TITLE_CHARS, updateClip } from '@/app/lib/clips'
import { formatSize } from '@/app/lib/use-dashboard'
import { formatPrecise, formatSpan } from '@/app/lib/sessions'
import styles from './replay.module.scss'

const MAX_LABEL_CHARS = 100

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

function InlineRename({
  initial,
  placeholder,
  maxLength = MAX_LABEL_CHARS,
  onSave,
  onCancel,
}: {
  initial: string
  placeholder: string
  maxLength?: number
  onSave: (label: string) => Promise<void> | void
  onCancel: () => void
}) {
  const [value, setValue] = useState(initial)
  const [saving, setSaving] = useState(false)
  const save = async () => {
    setSaving(true)
    try {
      await onSave(value.trim())
    } finally {
      setSaving(false)
    }
  }
  return (
    <div className={styles.renameRow}>
      <Input
        size="small"
        autoFocus
        value={value}
        maxLength={maxLength}
        placeholder={placeholder}
        aria-label="名字"
        disabled={saving}
        onChange={setValue}
        onEnterPress={save}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.stopPropagation()
            onCancel()
          }
        }}
      />
      <Button size="small" theme="solid" loading={saving} onClick={save}>
        保存
      </Button>
      <Button size="small" theme="borderless" type="tertiary" disabled={saving} onClick={onCancel}>
        取消
      </Button>
    </div>
  )
}

function rangeText(m: Marker): string {
  const parts: string[] = []
  if (m.lookback_ms) parts.push(`前 ${formatSpan(m.lookback_ms)}`)
  if (m.lookahead_ms) parts.push(`后 ${formatSpan(m.lookahead_ms)}`)
  return parts.length ? `默认范围：${parts.join('、')}` : '没有默认范围'
}

function MarkerRow({
  marker,
  current,
  canEdit,
  editReason,
  onSeek,
  onSelect,
}: {
  marker: Marker
  current: boolean
  canEdit: boolean
  editReason: string
  onSeek: (m: Marker) => void
  /** 窄屏没有选段，不给这个动作 */
  onSelect?: (m: Marker) => void
}) {
  const [renaming, setRenaming] = useState(false)
  const time = formatSessionTime(marker.at_ms)
  return (
    <li id={`marker-row-${marker.id}`} className={styles.row} data-current={current || undefined}>
      <button
        type="button"
        className={styles.rowTime}
        onClick={() => onSeek(marker)}
        title={`跳到 ${time}`}
        style={marker.color ? { color: marker.color } : undefined}
      >
        <IconFlag size="small" aria-hidden="true" />
        {time}
      </button>
      <div className={styles.rowBody}>
        {renaming ? (
          <InlineRename
            initial={marker.label}
            placeholder={`给 ${time} 的标记起个名字`}
            onCancel={() => setRenaming(false)}
            onSave={async (label) => {
              try {
                await renameMarker(marker.session_id, marker.id, label)
                setRenaming(false)
              } catch (e) {
                if (!(e instanceof ReportedError)) Toast.error({ content: `改名失败：${errorText(e)}`, duration: 4 })
              }
            }}
          />
        ) : (
          <>
            <span className={styles.rowLabel} data-empty={!marker.label || undefined}>
              {marker.label || '未命名标记'}
            </span>
            <span className={styles.rowMeta}>{rangeText(marker)}</span>
          </>
        )}
      </div>
      {renaming ? null : (
        <div className={styles.rowActions}>
          {onSelect ? (
            <Tooltip content="按这个标记的默认范围建一个选段（入点、出点吸附到关键帧）">
              <Button
                size="small"
                theme="borderless"
                icon={<IconScissors />}
                aria-label={`按 ${time} 的标记选段`}
                onClick={() => onSelect(marker)}
              />
            </Tooltip>
          ) : null}
          <Tooltip content={canEdit ? '改名' : editReason}>
            <Button
              size="small"
              theme="borderless"
              icon={<IconEdit />}
              aria-label={`给 ${time} 的标记改名`}
              disabled={!canEdit}
              onClick={() => setRenaming(true)}
            />
          </Tooltip>
          {canEdit ? (
            <Popconfirm
              title="删除这个标记？"
              content={`${time} ${marker.label || '未命名标记'}。删掉后，它占着的录像按保留设置正常清理`}
              okText="删除"
              okType="danger"
              onConfirm={() =>
                deleteMarker(marker.session_id, marker.id).then(
                  () => Toast.success({ content: `已删除 ${time} 的标记`, duration: 2 }),
                  (e: unknown) => {
                    if (!(e instanceof ReportedError)) Toast.error({ content: `删除失败：${errorText(e)}`, duration: 4 })
                  }
                )
              }
            >
              <Button size="small" theme="borderless" type="danger" icon={<IconDelete />} aria-label={`删除 ${time} 的标记`} />
            </Popconfirm>
          ) : (
            <Tooltip content={editReason}>
              <Button size="small" theme="borderless" type="danger" icon={<IconDelete />} aria-label="删除" disabled />
            </Tooltip>
          )}
        </div>
      )}
    </li>
  )
}

function ClipStatus({ clip }: { clip: Clip }) {
  if (clip.state === 'exporting') {
    const ratio = clip.progress?.ratio ?? null
    return (
      <div className={styles.clipStatus} data-state="exporting" role="status">
        <span>
          正在剪 · {clip.progress?.phase ?? '准备中'}
          {ratio !== null ? ` ${Math.round(ratio * 100)}%` : ''}
        </span>
        {ratio !== null ? (
          <Progress percent={Math.round(ratio * 100)} size="small" aria-label="导出进度" className={styles.clipProgress} />
        ) : null}
      </div>
    )
  }
  if (clip.state === 'ready' || clip.state === 'published') {
    const widened =
      clip.cut_in_ms !== null &&
      clip.cut_out_ms !== null &&
      (clip.cut_in_ms !== clip.in_ms || clip.cut_out_ms !== clip.out_ms)
    const facts = [
      clipModeText(clip.mode),
      extensionOf(clip).slice(1).toUpperCase(),
      clip.output_bytes !== null ? formatSize(clip.output_bytes) : null,
      clip.duration_ms !== null ? formatSpan(clip.duration_ms) : null,
    ].filter(Boolean)
    return (
      <div className={styles.clipStatus} data-state="ready">
        <span>
          {clip.state === 'published' ? `已发布${clip.archive_bvid ? ` ${clip.archive_bvid}` : ''}` : '已剪好'} ·{' '}
          {facts.join(' · ')}
        </span>
        {widened ? (
          <span className={styles.rowMeta}>
            实际切在 {formatPrecise(clip.cut_in_ms!)} → {formatPrecise(clip.cut_out_ms!)}（对齐关键帧）
          </span>
        ) : null}
      </div>
    )
  }
  if (clip.state === 'failed') {
    return (
      <div className={styles.clipStatus} data-state="failed" role="alert">
        失败{clip.mode ? `（${clipModeText(clip.mode)}）` : ''}：{clip.error || '原因未知'}
      </div>
    )
  }
  return (
    <div className={styles.clipStatus} data-state="draft">
      草稿（还没导出）
    </div>
  )
}

function ClipRow({
  clip,
  active,
  warning,
  canEdit,
  editReason,
  onLoad,
  compact,
}: {
  clip: Clip
  active: boolean
  compact: boolean
  warning: string | null
  canEdit: boolean
  editReason: string
  onLoad: (c: Clip) => void
}) {
  const [renaming, setRenaming] = useState(false)
  return (
    <li className={styles.row} data-current={active || undefined} data-clip-state={clip.state}>
      <button type="button" className={styles.rowTime} onClick={() => onLoad(clip)} title={compact ? '跳到入点' : '载入到细节条，跳到入点'}>
        <IconScissors size="small" aria-hidden="true" />
        {formatSessionTime(clip.in_ms)}
      </button>
      <div className={styles.rowBody}>
        {renaming ? (
          <InlineRename
            initial={clip.title}
            placeholder="给这个切片起个名字"
            maxLength={MAX_TITLE_CHARS}
            onCancel={() => setRenaming(false)}
            onSave={async (title) => {
              try {
                await updateClip(clip, { title })
                setRenaming(false)
              } catch (e) {
                if (!(e instanceof ReportedError)) Toast.error({ content: `改名失败：${errorText(e)}`, duration: 4 })
              }
            }}
          />
        ) : (
          <>
            <span className={styles.rowLabel} data-empty={!clip.title || undefined}>
              {clip.title || `未命名切片 #${clip.id}`}
            </span>
            <span className={styles.rowMeta}>
              {formatPrecise(clip.in_ms)} → {formatPrecise(clip.out_ms)} · {formatSpan(clip.out_ms - clip.in_ms)}
            </span>
            <ClipStatus clip={clip} />
            {warning ? <span className={styles.rowWarn}>{warning}</span> : null}
          </>
        )}
      </div>
      {renaming ? null : (
        <div className={styles.rowActions}>
          <Tooltip content={canEdit ? '改名' : editReason}>
            <Button
              size="small"
              theme="borderless"
              icon={<IconEdit />}
              aria-label="给切片改名"
              disabled={!canEdit}
              onClick={() => setRenaming(true)}
            />
          </Tooltip>
          {canEdit ? (
            <Popconfirm
              title="删除这个切片？"
              content={clip.state === 'exporting' ? '正在进行的导出会停下，' : clip.file_name ? '导出的文件会一起删掉' : undefined}
              okText="删除"
              okType="danger"
              onConfirm={() =>
                deleteClip(clip).then(
                  () => Toast.success({ content: '已删除切片', duration: 2 }),
                  (e: unknown) => {
                    if (!(e instanceof ReportedError)) Toast.error({ content: `删除失败：${errorText(e)}`, duration: 4 })
                  }
                )
              }
            >
              <Button size="small" theme="borderless" type="danger" icon={<IconDelete />} aria-label="删除切片" />
            </Popconfirm>
          ) : (
            <Tooltip content={editReason}>
              <Button size="small" theme="borderless" type="danger" icon={<IconDelete />} aria-label="删除" disabled />
            </Tooltip>
          )}
        </div>
      )}
    </li>
  )
}

export type PanelTab = 'markers' | 'clips'

export function SidePanel({
  tab,
  onTab,
  markers,
  markersLoading,
  markersError,
  clips,
  clipsLoading,
  clipsError,
  activeClip,
  clipWarning,
  currentMarker,
  canEdit,
  editReason,
  onSeekMarker,
  onSelectMarker,
  onLoadClip,
  compact,
}: {
  tab: PanelTab
  onTab: (tab: PanelTab) => void
  markers: Marker[]
  markersLoading: boolean
  markersError: boolean
  clips: Clip[]
  clipsLoading: boolean
  clipsError: boolean
  activeClip: number | null
  clipWarning: (c: Clip) => string | null
  currentMarker: number | null
  canEdit: boolean
  editReason: string
  onSeekMarker: (m: Marker) => void
  onSelectMarker: (m: Marker) => void
  onLoadClip: (c: Clip) => void
  /** 手机宽度：没有细节条，标记不能选段，切片只能跳到入点 */
  compact: boolean
}) {
  const { Text } = Typography
  const markerList = (
    <>
      {!canEdit ? (
        <Text type="tertiary" size="small" className={styles.panelNote}>
          {editReason}
        </Text>
      ) : null}
      {markersError ? (
        <Empty description="标记列表加载失败，稍后会自动重试" className={styles.empty} />
      ) : markers.length === 0 ? (
        <Empty
          description={markersLoading ? '正在加载标记…' : '这一场还没有标记。按「标记」或 M 在当前画面打一个'}
          className={styles.empty}
        />
      ) : (
        <ul className={styles.list} aria-label="标记列表">
          {markers.map((m) => (
            <MarkerRow
              key={m.id}
              marker={m}
              current={currentMarker === m.id}
              canEdit={canEdit}
              editReason={editReason}
              onSeek={onSeekMarker}
              onSelect={compact ? undefined : onSelectMarker}
            />
          ))}
        </ul>
      )}
    </>
  )
  return (
    <section className={styles.panel} aria-label="标记与切片">
      <Tabs type="line" size="small" activeKey={tab} onChange={(k) => onTab(k as PanelTab)} keepDOM={false}>
        <TabPane tab={`标记 ${markers.length}`} itemKey="markers">
          {markerList}
        </TabPane>
        <TabPane tab={`切片 ${clips.length}`} itemKey="clips">
          <Text type="tertiary" size="small" className={styles.panelNote}>
            切片按场次时间记下入点、出点，存在服务器上；删掉切片会一起删掉它导出的文件。
          </Text>
          {clipsError && clips.length === 0 ? (
            <Empty description="切片列表加载失败，稍后会自动重试" className={styles.empty} />
          ) : clips.length === 0 ? (
            <Empty
              description={
                clipsLoading
                  ? '正在加载切片…'
                  : compact
                    ? '还没有切片。选段需要至少 760 px 宽的窗口'
                    : '还没有切片。在细节条上用 I / O（或「入点」「出点」按钮）选一段，再点「存为切片」'
              }
              className={styles.empty}
            />
          ) : (
            <ul className={styles.list} aria-label="切片列表">
              {clips.map((c) => (
                <ClipRow
                  key={c.id}
                  clip={c}
                  active={activeClip === c.id}
                  warning={clipWarning(c)}
                  canEdit={canEdit}
                  editReason={editReason}
                  onLoad={onLoadClip}
                  compact={compact}
                />
              ))}
            </ul>
          )}
        </TabPane>
      </Tabs>
    </section>
  )
}
