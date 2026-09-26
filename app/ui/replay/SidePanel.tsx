'use client'
import React, { useState } from 'react'
import {
  Button,
  Checkbox,
  Empty,
  Input,
  Popconfirm,
  Progress,
  TabPane,
  Tabs,
  Toast,
  Tooltip,
  Typography,
} from '@douyinfe/semi-ui'
import { IconDelete, IconDownload, IconEdit, IconFlag, IconScissors, IconSend } from '@douyinfe/semi-icons'
import { deleteMarker, formatSessionTime, type Marker, renameMarker, ReportedError } from '@/app/lib/markers'
import {
  type Clip,
  type ClipMode,
  clipModeText,
  deleteClip,
  downloadClip,
  exportClip,
  extensionOf,
  type FfmpegState,
  isMp4,
  MAX_TITLE_CHARS,
  updateClip,
} from '@/app/lib/clips'
import { formatSize } from '@/app/lib/use-dashboard'
import { formatPrecise, formatSpan } from '@/app/lib/sessions'
import { canPublish, MAX_BATCH, needsConfirm, type PublishJob } from '@/app/lib/publish'
import { BvLink, JobStatus } from '@/app/ui/publish/JobStatus'
import type { PublishTarget } from '@/app/ui/publish/PublishDrawer'
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
          {clip.state === 'published' ? '已发布' : '已剪好'} · {facts.join(' · ')}
        </span>
        {clip.state === 'published' && clip.archive_bvid ? (
          <span className={styles.publishedLine}>
            <BvLink bvid={clip.archive_bvid} />
            {clip.published_at ? (
              <span className={styles.rowMeta}>
                {new Date(clip.published_at).toLocaleString('zh-CN', {
                  month: 'numeric',
                  day: 'numeric',
                  hour: '2-digit',
                  minute: '2-digit',
                  hour12: false,
                })}{' '}
                投稿
              </span>
            ) : null}
          </span>
        ) : null}
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

export const QUICK_TIP = '快速剪：按关键帧切，不转码，几秒就好；入点、出点放宽到最近的关键帧，保持录像原格式'
export const PRECISE_TIP = '精确剪：用服务器上的 ffmpeg 转码成 H.264 / AAC 的 MP4，首尾帧对准选段；较慢，占 CPU'

function ExportButton({
  mode,
  clip,
  ffmpeg,
  canEdit,
  editReason,
  label,
  primary,
}: {
  mode: ClipMode
  clip: Clip
  ffmpeg: FfmpegState
  canEdit: boolean
  editReason: string
  label: string
  primary?: boolean
}) {
  const [busy, setBusy] = useState(false)
  const reason = !canEdit ? editReason : mode === 'precise' ? ffmpeg.reason : null
  const run = async () => {
    setBusy(true)
    try {
      await exportClip(clip, mode)
    } catch (e) {
      if (!(e instanceof ReportedError)) Toast.error({ content: `没能开始导出：${errorText(e)}`, duration: 4 })
    } finally {
      setBusy(false)
    }
  }
  const tip = `${mode === 'quick' ? QUICK_TIP : PRECISE_TIP}${clip.file_name ? '。之前导出的文件会被新文件替换' : ''}`
  return (
    <Tooltip content={reason ?? tip} className={styles.passTip}>
      <span className={styles.inlineWrap}>
        <Button size="small" theme={primary ? 'solid' : 'light'} loading={busy} disabled={reason !== null} onClick={run}>
          {label}
        </Button>
      </span>
    </Tooltip>
  )
}

function DownloadButton({
  clip,
  format,
  label,
  tip,
  disabledReason,
  canDownload,
}: {
  clip: Clip
  format: 'source' | 'mp4'
  label: string
  tip?: string
  disabledReason: string | null
  canDownload: boolean
}) {
  const [busy, setBusy] = useState(false)
  const reason = !canDownload ? '没有下载文件的权限：需要 file.view' : disabledReason
  const run = async () => {
    setBusy(true)
    try {
      await downloadClip(clip, format)
    } catch (e) {
      if (!(e instanceof ReportedError)) Toast.error({ content: `下载失败：${errorText(e)}`, duration: 5 })
    } finally {
      setBusy(false)
    }
  }
  const button = (
    <Button size="small" theme="light" icon={<IconDownload />} loading={busy} disabled={reason !== null} onClick={run}>
      {busy && format === 'mp4' && !isMp4(clip) ? '正在转 MP4…' : label}
    </Button>
  )
  if (reason === null && !tip) return button
  return (
    <Tooltip content={reason ?? tip} className={styles.passTip}>
      <span className={styles.inlineWrap}>{button}</span>
    </Tooltip>
  )
}

/** 切片卡片上的操作：还没导出 → 快速剪 / 精确剪；失败 → 重试；剪好 → 下载源格式 / MP4、改用另一种方式重剪 */
function ClipTools({
  clip,
  ffmpeg,
  canEdit,
  editReason,
  canDownload,
  publish,
}: {
  clip: Clip
  ffmpeg: FfmpegState
  canEdit: boolean
  editReason: string
  canDownload: boolean
  /** 「发布」按钮；不能发布（没权限、已发布、在发布队列里）时为 null */
  publish: React.ReactNode
}) {
  if (clip.state === 'discarded') return null
  if (clip.state === 'exporting') return publish ? <div className={styles.clipTools}>{publish}</div> : null
  const ext = extensionOf(clip).slice(1).toUpperCase()
  if (clip.state === 'ready' || clip.state === 'published') {
    const other: ClipMode = clip.mode === 'precise' ? 'quick' : 'precise'
    return (
      <div className={styles.clipTools}>
        {publish}
        <DownloadButton
          clip={clip}
          format="source"
          label={`下载 ${ext || '文件'}`}
          disabledReason={null}
          canDownload={canDownload}
        />
        {isMp4(clip) ? null : (
          <DownloadButton
            clip={clip}
            format="mp4"
            label="MP4"
            tip="用 ffmpeg 转封装成 MP4（不转码），第一次要等几秒"
            disabledReason={
              ffmpeg.reason ? `${ffmpeg.reason}。可以先下载源格式（${ext}），多数播放器和剪辑软件能直接打开` : null
            }
            canDownload={canDownload}
          />
        )}
        {clip.state === 'ready' ? (
          <ExportButton
            mode={other}
            clip={clip}
            ffmpeg={ffmpeg}
            canEdit={canEdit}
            editReason={editReason}
            label={other === 'precise' ? '改用精确剪' : '改用快速剪'}
          />
        ) : null}
      </div>
    )
  }
  const retry = clip.state === 'failed' ? (clip.mode ?? 'quick') : null
  return (
    <div className={styles.clipTools}>
      {publish}
      {retry ? (
        <ExportButton
          mode={retry}
          clip={clip}
          ffmpeg={ffmpeg}
          canEdit={canEdit}
          editReason={editReason}
          label={`重试${clipModeText(retry)}`}
          primary
        />
      ) : null}
      {retry !== 'quick' ? (
        <ExportButton
          mode="quick"
          clip={clip}
          ffmpeg={ffmpeg}
          canEdit={canEdit}
          editReason={editReason}
          label={retry ? '改用快速剪' : '快速剪'}
          primary={!retry}
        />
      ) : null}
      {retry !== 'precise' ? (
        <ExportButton
          mode="precise"
          clip={clip}
          ffmpeg={ffmpeg}
          canEdit={canEdit}
          editReason={editReason}
          label={retry ? '改用精确剪' : '精确剪'}
        />
      ) : null}
    </div>
  )
}

function ClipRow({
  clip,
  active,
  warning,
  ffmpeg,
  canEdit,
  editReason,
  canDownload,
  onLoad,
  compact,
  job,
  canSubmit,
  onPublish,
  selecting,
  selected,
  onToggle,
}: {
  clip: Clip
  active: boolean
  compact: boolean
  warning: string | null
  ffmpeg: FfmpegState
  canEdit: boolean
  editReason: string
  canDownload: boolean
  onLoad: (c: Clip) => void
  /** 这个切片最近的发布任务 */
  job: PublishJob | undefined
  canSubmit: boolean
  onPublish: (c: Clip) => void
  selecting: boolean
  selected: boolean
  onToggle: (c: Clip, on: boolean) => void
}) {
  const [renaming, setRenaming] = useState(false)
  const queued = !!job && job.state !== 'done'
  const unknown = needsConfirm(clip) && !queued
  const publishable = canPublish(clip, job)
  const publish =
    canSubmit && publishable ? (
      <Tooltip
        content={
          unknown
            ? '上次投稿的结果未知：先到 B 站稿件管理核对，确认没有这个稿件再发'
            : '选模板、改标题和封面，然后导出（需要时）→ 上传 → 投稿'
        }
        className={styles.passTip}
      >
        <span className={styles.inlineWrap}>
          <Button
            size="small"
            theme={clip.state === 'ready' ? 'solid' : 'light'}
            type={unknown ? 'warning' : 'primary'}
            icon={<IconSend />}
            onClick={() => onPublish(clip)}
          >
            {unknown ? '核对后再发布' : '发布'}
          </Button>
        </span>
      </Tooltip>
    ) : null
  return (
    <li
      id={`clip-row-${clip.id}`}
      className={styles.row}
      data-current={active || undefined}
      data-clip-state={clip.state}
      data-clip-id={clip.id}
    >
      {selecting ? (
        <Checkbox
          className={styles.rowCheck}
          checked={selected}
          disabled={!publishable}
          onChange={(e) => onToggle(clip, !!e.target.checked)}
          aria-label={`选中切片 ${clip.title || `#${clip.id}`}`}
        />
      ) : null}
      <button type="button" className={styles.rowTime} onClick={() => onLoad(clip)} title={compact ? '跳到入点' : '载入到细节条，跳到入点'}>
        <IconScissors size="small" aria-hidden="true" />
        {formatSessionTime(clip.in_ms)}
      </button>
      <div className={styles.rowBody}>
        {renaming ? (
          <InlineRename
            initial={clip.title}
            placeholder="给这个切片起个名字（下载时用作文件名）"
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
            {job && (queued || clip.state !== 'published') ? <JobStatus job={job} canSubmit={canSubmit} /> : null}
            {unknown ? (
              <span className={styles.rowWarn} data-submit-unknown="">
                上次投稿的结果未知（投稿请求发出后服务退出了），B 站那边可能已经有这个稿件
              </span>
            ) : null}
            {warning ? <span className={styles.rowWarn}>{warning}</span> : null}
            <ClipTools
              clip={clip}
              ffmpeg={ffmpeg}
              canEdit={canEdit}
              editReason={editReason}
              canDownload={canDownload}
              publish={publish}
            />
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
          {canEdit && queued ? (
            <Tooltip content="在发布队列里，先移出队列再删">
              <Button size="small" theme="borderless" type="danger" icon={<IconDelete />} aria-label="删除" disabled />
            </Tooltip>
          ) : canEdit ? (
            <Popconfirm
              title="删除这个切片？"
              content={
                clip.state === 'exporting'
                  ? '正在进行的导出会停下，写了一半的文件一起删掉'
                  : clip.file_name
                    ? `导出的文件（${clip.file_name}${clip.output_bytes !== null ? `，${formatSize(clip.output_bytes)}` : ''}）会一起删掉`
                    : undefined
              }
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
  canDownload,
  ffmpeg,
  onSeekMarker,
  onSelectMarker,
  onLoadClip,
  compact,
  canSubmit,
  jobs,
  onPublish,
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
  canDownload: boolean
  ffmpeg: FfmpegState
  onSeekMarker: (m: Marker) => void
  onSelectMarker: (m: Marker) => void
  onLoadClip: (c: Clip) => void
  /** 手机宽度：没有细节条，标记不能选段，切片只能跳到入点 */
  compact: boolean
  canSubmit: boolean
  /** 各切片最近的发布任务 */
  jobs: Map<number, PublishJob>
  onPublish: (target: PublishTarget) => void
}) {
  const { Text } = Typography
  const [selecting, setSelecting] = useState(false)
  const [selected, setSelected] = useState<Set<number>>(() => new Set())
  const picked = clips.filter((c) => selected.has(c.id) && canPublish(c, jobs.get(c.id)))
  const pickableCount = clips.filter((c) => canPublish(c, jobs.get(c.id))).length
  const toggle = (c: Clip, on: boolean) =>
    setSelected((prev) => {
      const next = new Set(prev)
      if (on) next.add(c.id)
      else next.delete(c.id)
      return next
    })
  const stopSelecting = () => {
    setSelecting(false)
    setSelected(new Set())
  }
  const openBatch = () => {
    const list = [...picked].sort((a, b) => a.in_ms - b.in_ms || a.id - b.id)
    onPublish({ clips: list, batch: true })
    stopSelecting()
  }
  const running = [...new Set(jobs.values())].filter((j) => j.state === 'queued' || j.state === 'running').length
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
            快速剪按关键帧切、不转码；精确剪用 ffmpeg 转码成 MP4，首尾对准选段。导出的文件存在服务器的 clips
            目录下，删掉切片时一起删掉。
            {canSubmit
              ? `发布固定按转载投稿，同一时间只传一个稿件${running ? `（发布队列里还有 ${running} 个）` : ''}。`
              : '发布需要 upload.submit 权限，所以这里不显示「发布」。'}
          </Text>
          {canSubmit && (selecting || pickableCount > 1) ? (
            <div className={styles.batchBar} role="toolbar" aria-label="集中发布">
              {selecting ? (
                <>
                  <span className={styles.rowMeta}>
                    已选 {picked.length} 个{picked.length > MAX_BATCH ? `（一次最多 ${MAX_BATCH} 个）` : ''}
                  </span>
                  <Button
                    size="small"
                    theme="solid"
                    icon={<IconSend />}
                    disabled={picked.length === 0 || picked.length > MAX_BATCH}
                    onClick={openBatch}
                  >
                    集中发布
                  </Button>
                  <Button
                    size="small"
                    theme="borderless"
                    onClick={() => setSelected(new Set(clips.filter((c) => canPublish(c, jobs.get(c.id))).map((c) => c.id)))}
                  >
                    全选
                  </Button>
                  <Button size="small" theme="borderless" type="tertiary" onClick={stopSelecting}>
                    取消
                  </Button>
                </>
              ) : (
                <Button size="small" theme="light" icon={<IconSend />} onClick={() => setSelecting(true)}>
                  选择多个切片集中发布
                </Button>
              )}
            </div>
          ) : null}
          {clipsError && clips.length === 0 ? (
            <Empty description="切片列表加载失败，稍后会自动重试" className={styles.empty} />
          ) : clips.length === 0 ? (
            <Empty
              description={
                clipsLoading
                  ? '正在加载切片…'
                  : compact
                    ? '还没有切片。选段需要至少 760 px 宽的窗口；看直播时也可以在直播预览里「剪下刚才」'
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
                  ffmpeg={ffmpeg}
                  canEdit={canEdit}
                  editReason={editReason}
                  canDownload={canDownload}
                  onLoad={onLoadClip}
                  compact={compact}
                  job={jobs.get(c.id)}
                  canSubmit={canSubmit}
                  onPublish={(clip) => onPublish({ clips: [clip], batch: false })}
                  selecting={selecting}
                  selected={selected.has(c.id)}
                  onToggle={toggle}
                />
              ))}
            </ul>
          )}
        </TabPane>
      </Tabs>
    </section>
  )
}
