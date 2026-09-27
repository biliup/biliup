'use client'
import React, { useState } from 'react'
import { Button, Checkbox, Empty, Popconfirm, Radio, RadioGroup, Tag, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconBulb, IconClose, IconScissors, IconTick } from '@douyinfe/semi-icons'
import {
  type AutoClipAvailability,
  type AutoClipJob,
  autoClipError,
  confidenceLevel,
  dismissSuggestion,
  effectiveState,
  isActive,
  jobHints,
  type Suggestion,
  type SuggestionState,
} from '@/app/lib/auto-clip'
import { formatSessionTime, ReportedError } from '@/app/lib/markers'
import { formatPrecise, formatSpan } from '@/app/lib/sessions'
import { useNowSec } from '@/app/lib/use-dashboard'
import { useEnumPref } from '@/app/lib/use-local-pref'
import { AutoClipTrigger, EDIT_REASON, JobBadge, JobHints, JobProgress, SetupNote } from '@/app/ui/auto-clip/AutoClipJob'
import type { Selection } from './Timeline'
import styles from './replay.module.scss'

const SORTS = ['time', 'confidence'] as const
type Sort = (typeof SORTS)[number]

/** 离过期不到这么久时在卡片上提醒 */
const EXPIRY_WARN_MS = 12 * 3_600_000

function Evidence({ s }: { s: Suggestion }) {
  const level = confidenceLevel(s.confidence)
  const chips: string[] = []
  if (s.evidence.danmaku > 0) chips.push(`弹幕高峰 ${s.evidence.danmaku} 条`)
  if (s.evidence.asr_lines > 0) chips.push(`语音 ${s.evidence.asr_lines} 句`)
  if (s.evidence.images.length > 0) chips.push(`画面 ${s.evidence.images.length} 张`)
  if (!level && chips.length === 0 && s.tags.length === 0) return null
  return (
    <span className={styles.sgSignals}>
      {level ? (
        <Tooltip content="模型对这一段的自评，不是播放数据；只作排序参考">
          <Tag size="small" color={level.tone} data-testid="suggestion-confidence">
            模型自评 {level.label} {s.confidence!.toFixed(2)}
          </Tag>
        </Tooltip>
      ) : null}
      {chips.map((c) => (
        <Tag key={c} size="small" color="violet" type="light">
          {c}
        </Tag>
      ))}
      {s.tags.map((t) => (
        <Tag key={`tag-${t}`} size="small" color="grey" type="ghost">
          {t}
        </Tag>
      ))}
    </span>
  )
}

function StateLine({
  state,
  s,
  nowMs,
  onOpenClip,
}: {
  state: SuggestionState
  s: Suggestion
  nowMs: number
  onOpenClip: (clipId: number) => void
}) {
  if (state === 'expired') {
    return (
      <span className={styles.sgState} data-state="expired">
        已过期（72 小时没处理），不能再接受；要用这一段请重新生成候选，或载入选段后手动存为切片
      </span>
    )
  }
  if (state === 'accepted') {
    return (
      <span className={styles.sgState} data-state="accepted">
        已接受
        {s.clip_id !== null ? (
          <>
            {' → '}
            <button type="button" className={styles.sgLink} onClick={() => onOpenClip(s.clip_id!)}>
              切片 #{s.clip_id}
            </button>
          </>
        ) : (
          '（切片已删除）'
        )}
      </span>
    )
  }
  if (state === 'dismissed') {
    return (
      <span className={styles.sgState} data-state="dismissed">
        已丢弃
      </span>
    )
  }
  if (s.expires_at !== null && s.expires_at - nowMs < EXPIRY_WARN_MS) {
    const hours = Math.max(1, Math.round((s.expires_at - nowMs) / 3_600_000))
    return (
      <span className={styles.sgState} data-state="expiring">
        约 {hours} 小时后过期
      </span>
    )
  }
  return null
}

function SuggestionRow({
  s,
  state,
  nowMs,
  active,
  selection,
  canEdit,
  problemOf,
  onSeek,
  onLoad,
  onAccept,
  onOpenClip,
}: {
  s: Suggestion
  state: SuggestionState
  nowMs: number
  active: boolean
  selection: Selection
  canEdit: boolean
  problemOf: (from: number, to: number) => string | null
  onSeek: (s: Suggestion) => void
  onLoad?: (s: Suggestion) => void
  onAccept: (s: Suggestion) => Promise<void>
  onOpenClip: (clipId: number) => void
}) {
  const [busy, setBusy] = useState<'accept' | 'dismiss' | null>(null)
  const time = formatSessionTime(s.in_ms)
  const edited =
    active &&
    selection.in !== null &&
    selection.out !== null &&
    (selection.in !== s.in_ms || selection.out !== s.out_ms)
  const range = edited ? { in: selection.in!, out: selection.out! } : { in: s.in_ms, out: s.out_ms }
  const problem = problemOf(range.in, range.out)
  const acceptReason = !canEdit
    ? EDIT_REASON
    : state === 'expired'
      ? '已过期（72 小时没处理），重新生成候选后再选'
      : problem
  const accept = async () => {
    setBusy('accept')
    try {
      await onAccept(s)
    } finally {
      setBusy(null)
    }
  }
  const dismiss = async () => {
    setBusy('dismiss')
    try {
      await dismissSuggestion(s)
      Toast.info({ content: `已丢弃 ${time} 的候选`, duration: 2 })
    } catch (e) {
      if (!(e instanceof ReportedError)) Toast.error({ content: `丢弃失败：${autoClipError(e)}`, duration: 4 })
    } finally {
      setBusy(null)
    }
  }
  const pending = state === 'pending'
  return (
    <li
      id={`suggestion-row-${s.id}`}
      className={styles.row}
      data-current={active || undefined}
      data-suggestion-state={state}
      data-suggestion-id={s.id}
    >
      <button type="button" className={styles.rowTime} onClick={() => onSeek(s)} title={`跳到 ${time}`}>
        <IconBulb size="small" aria-hidden="true" />
        {time}
      </button>
      <div className={styles.rowBody}>
        <span className={styles.rowLabel} data-empty={!s.title || undefined}>
          {s.title || '未命名候选'}
        </span>
        <span className={styles.rowMeta}>
          {formatPrecise(range.in)} → {formatPrecise(range.out)} · {formatSpan(range.out - range.in)}
          {edited ? '（按当前选段）' : ''}
        </span>
        {s.reason ? <span className={styles.sgReason}>{s.reason}</span> : null}
        <Evidence s={s} />
        <StateLine state={state} s={s} nowMs={nowMs} onOpenClip={onOpenClip} />
        {pending && problem ? <span className={styles.rowWarn}>{problem}</span> : null}
        {pending || state === 'expired' ? (
          <div className={styles.clipTools}>
            <Tooltip
              content={
                acceptReason ??
                (edited ? '按当前选段建一个切片草稿（不导出、不投稿）' : '按这一段建一个切片草稿（不导出、不投稿）')
              }
              className={styles.passTip}
            >
              <span className={styles.inlineWrap}>
                <Button
                  size="small"
                  theme="solid"
                  icon={<IconTick />}
                  loading={busy === 'accept'}
                  disabled={acceptReason !== null || busy !== null}
                  onClick={accept}
                >
                  接受
                </Button>
              </span>
            </Tooltip>
            {onLoad ? (
              <Tooltip content="载入到选段和细节条，可以先微调入点、出点再接受" className={styles.passTip}>
                <span className={styles.inlineWrap}>
                  <Button size="small" theme="light" icon={<IconScissors />} onClick={() => onLoad(s)}>
                    载入选段
                  </Button>
                </span>
              </Tooltip>
            ) : null}
            {pending ? (
              canEdit ? (
                <Popconfirm
                  title="丢弃这个候选？"
                  content="丢弃后不能恢复；以后重新生成时，和它大体重合的片段也不会再出现"
                  okText="丢弃"
                  okType="danger"
                  onConfirm={dismiss}
                >
                  <Button size="small" theme="borderless" type="tertiary" icon={<IconClose />} loading={busy === 'dismiss'}>
                    丢弃
                  </Button>
                </Popconfirm>
              ) : (
                <Tooltip content={EDIT_REASON}>
                  <span className={styles.inlineWrap}>
                    <Button size="small" theme="borderless" type="tertiary" icon={<IconClose />} disabled>
                      丢弃
                    </Button>
                  </span>
                </Tooltip>
              )
            ) : null}
          </div>
        ) : null}
      </div>
    </li>
  )
}

function JobArea({
  sessionId,
  job,
  availability,
  recording,
  canEdit,
}: {
  sessionId: number
  job: AutoClipJob | null
  availability: AutoClipAvailability
  recording: boolean
  canEdit: boolean
}) {
  const hints = job ? jobHints(job) : []
  return (
    <div className={styles.sgJob} data-testid="auto-clip-job">
      <div className={styles.sgJobHead}>
        {job ? (
          <JobBadge job={job} detail />
        ) : (
          <span className={styles.rowMeta}>还没有为这一场生成过候选</span>
        )}
        <span className={styles.sgJobActions}>
          <AutoClipTrigger sessionId={sessionId} job={job} availability={availability} recording={recording} />
        </span>
      </div>
      {job ? <JobProgress job={job} /> : null}
      <JobHints hints={hints} />
      {availability.setupReason && !isActive(job) ? <SetupNote reason={availability.setupReason} /> : null}
      {!canEdit ? (
        <Typography.Text type="tertiary" size="small">
          {EDIT_REASON}
        </Typography.Text>
      ) : null}
    </div>
  )
}

export function SuggestionsPanel({
  sessionId,
  suggestions,
  loading,
  error,
  job,
  availability,
  recording,
  activeSuggestion,
  selection,
  canEdit,
  compact,
  problemOf,
  onSeek,
  onLoad,
  onAccept,
  onOpenClip,
}: {
  sessionId: number
  suggestions: Suggestion[]
  loading: boolean
  error: boolean
  job: AutoClipJob | null
  availability: AutoClipAvailability
  recording: boolean
  activeSuggestion: number | null
  selection: Selection
  canEdit: boolean
  compact: boolean
  problemOf: (from: number, to: number) => string | null
  onSeek: (s: Suggestion) => void
  onLoad: (s: Suggestion) => void
  onAccept: (s: Suggestion) => Promise<void>
  onOpenClip: (clipId: number) => void
}) {
  const now = useNowSec() * 1000
  const [sort, setSort] = useEnumPref<Sort>('biliup.replay.suggestionSort', SORTS, 'time')
  const [showHandled, setShowHandled] = useState(false)
  const withState = suggestions.map((s) => ({ s, state: effectiveState(s, now) }))
  const handled = withState.filter((x) => x.state === 'accepted' || x.state === 'dismissed').length
  const pendingCount = withState.filter((x) => x.state === 'pending').length
  const expiredCount = withState.filter((x) => x.state === 'expired').length
  const shown = withState
    .filter((x) => showHandled || (x.state !== 'accepted' && x.state !== 'dismissed'))
    .sort((a, b) =>
      sort === 'confidence'
        ? (b.s.confidence ?? -1) - (a.s.confidence ?? -1) || a.s.in_ms - b.s.in_ms
        : a.s.in_ms - b.s.in_ms || a.s.id - b.s.id
    )
  const running = isActive(job)
  const emptyText = error
    ? '候选列表加载失败，稍后会自动重试'
    : loading
      ? '正在加载候选…'
      : running
        ? '正在生成，做完后候选会列在这里'
        : suggestions.length > 0
          ? `待处理的候选都处理完了（已处理 ${handled} 个）`
          : job?.state === 'done'
            ? '这次没有挑出候选。可以换个模型、补上热词后重新生成'
            : '还没有候选。「生成候选」会让模型从这一场的语音、弹幕和截图里挑出值得剪的片段'
  return (
    <>
      <JobArea sessionId={sessionId} job={job} availability={availability} recording={recording} canEdit={canEdit} />
      {suggestions.length > 0 ? (
        <div className={styles.sgBar} role="toolbar" aria-label="候选筛选">
          <span className={styles.rowMeta}>
            待处理 {pendingCount}
            {expiredCount ? ` · 已过期 ${expiredCount}` : ''}
          </span>
          <RadioGroup
            type="button"
            buttonSize="small"
            value={sort}
            onChange={(e) => setSort(e.target.value as Sort)}
            aria-label="排序"
          >
            <Radio value="time">按时间</Radio>
            <Radio value="confidence">按自评</Radio>
          </RadioGroup>
          {handled > 0 ? (
            <Checkbox checked={showHandled} onChange={(e) => setShowHandled(!!e.target.checked)}>
              显示已处理 {handled}
            </Checkbox>
          ) : null}
        </div>
      ) : null}
      {shown.length === 0 ? (
        <Empty description={emptyText} className={styles.empty} />
      ) : (
        <ul className={styles.list} aria-label="候选列表">
          {shown.map(({ s, state }) => (
            <SuggestionRow
              key={s.id}
              s={s}
              state={state}
              nowMs={now}
              active={activeSuggestion === s.id}
              selection={selection}
              canEdit={canEdit}
              problemOf={problemOf}
              onSeek={onSeek}
              onLoad={compact ? undefined : onLoad}
              onAccept={onAccept}
              onOpenClip={onOpenClip}
            />
          ))}
        </ul>
      )}
    </>
  )
}
