'use client'
import { useEffect, useState } from 'react'
import Link from 'next/link'
import { Banner, Spin, Tag, Toast } from '@douyinfe/semi-ui'
import { FormDialog } from '@/app/ui/shell'
import { fetcher } from '@/app/lib/api-streamer'
import { ReportedError } from '@/app/lib/markers'
import { recipeUrl } from '@/app/lib/renders'
import {
  confirmSessionMerge, formatPrecise, formatSpan, mergePreview, mergeProblem, mergeSessions,
  replayHref, sessionUrl, type SessionDetail, type SessionMergeCandidate,
  segmentEnd,
} from '@/app/lib/sessions'
import styles from './sessions.module.scss'

export default function SessionMergeDialog({ sourceIds, candidate, canReadRecipe, onClose, onMerged }: {
  sourceIds: number[]
  candidate?: SessionMergeCandidate
  canReadRecipe: boolean
  onClose: () => void
  onMerged: (id: number) => void
}) {
  const [details, setDetails] = useState<SessionDetail[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [recipeDiff, setRecipeDiff] = useState<boolean | null>(candidate?.recipe_diff ?? null)
  const [busy, setBusy] = useState(false)
  const ids = sourceIds.join(',')
  useEffect(() => {
    let cancelled = false
    const selectedIds = ids.split(',').map(Number)
    void Promise.all(selectedIds.map(id => fetcher(sessionUrl(id)) as Promise<SessionDetail>)).then(
      data => { if (!cancelled) setDetails(data) },
      e => { if (!cancelled) setError(e instanceof Error ? e.message : String(e)) },
    )
    if (canReadRecipe) {
      void Promise.allSettled(selectedIds.map(id => fetcher(recipeUrl(id)))).then(results => {
        if (cancelled) return
        if (results.some(result => result.status === 'rejected')) { setRecipeDiff(null); return }
        const recipes = results.map(result => result.status === 'fulfilled' ? JSON.stringify(result.value) : '')
        setRecipeDiff(recipes.some(recipe => recipe !== recipes[0]))
      })
    }
    return () => { cancelled = true }
  }, [ids, candidate, canReadRecipe])

  const ordered = details ? details.map(detail => ({ ...detail,
    duration_ms: detail.segments.reduce((end, segment) => Math.max(end, segmentEnd(segment)), detail.duration_ms),
  })).sort((a, b) => a.started_at - b.started_at || a.id - b.id) : []
  const preview = mergePreview(ordered).flatMap((member, index) => ordered[index].kind === 'group'
    ? (ordered[index].members ?? []).map(source => ({ ...source, offset_ms: source.offset_ms + member.offset_ms }))
    : [member])
  const problem = details && !candidate ? mergeProblem(details) : null
  const appending = details?.some(source => source.kind === 'group') ?? false
  const confirm = async () => {
    if (!details || problem || busy) return
    setBusy(true)
    try {
      const result = candidate ? await confirmSessionMerge(candidate.id) : await mergeSessions(ordered.map(source => source.id))
      onMerged(result.id)
    } catch (e) {
      if (!(e instanceof ReportedError)) Toast.error({ content: `合并失败：${e instanceof Error ? e.message : String(e)}`, duration: 5 })
    } finally { setBusy(false) }
  }
  return <FormDialog
    title={candidate ? '确认续接场次' : '合并直播场次'} size="md"
    onCancel={onClose} closable={!busy} okText="确认合并" onOk={confirm}
    okDisabled={!details || !!problem || !!error} confirmLoading={busy}
  >
    <p className={styles.mergeDescription}>按以下顺序组成一条完整场次。原始录像、标记、切片和已发布稿件继续保留；停录缺口会显示在时间线上。</p>
    {!details && !error ? <div className={styles.mergeLoading}><Spin tip="正在读取来源场次…" /></div> : null}
    {error ? <Banner type="danger" description={`无法读取来源场次：${error}`} /> : null}
    {problem ? <Banner type="warning" description={problem} /> : null}
    {preview.length > 0 ? <ol className={styles.mergeSources}>
      {preview.map((source, index) => <li key={source.source_session_id}>
        {source.gap_before_ms > 0 ? <div className={styles.mergeGap}>停录缺口 {formatSpan(source.gap_before_ms)}</div> : null}
        <div className={styles.mergeSourceHead}>
          <strong>{index + 1}. {source.title || '无标题'}</strong>
          {source.ended_at === null ? <Tag size="small" color="red">录制中</Tag> : null}
          <Link className={styles.replayLink} href={replayHref(source.source_session_id)} target="_blank">原场次 #{source.source_session_id}</Link>
        </div>
        <div className={styles.mergeMeta}>{new Date(source.started_at).toLocaleString()} · 已录 {formatSpan(source.duration_ms)} · 合并时间 {formatPrecise(source.offset_ms)} → {formatPrecise(source.offset_ms + source.duration_ms)}</div>
      </li>)}
    </ol> : null}
    {details ? <Banner type={recipeDiff ? 'warning' : 'info'} description={recipeDiff
      ? appending
        ? '新来源的压制配方与现有合并场次不同。现有合并场次的独立配方继续保留，可在工作台调整；原场次配方和旧任务快照保留。'
        : '来源场次的压制配方不同。合并场次默认继承最早来源的配方，随后可在合并工作台独立调整；原场次配方和旧任务快照保留。'
      : recipeDiff === null
        ? appending
          ? '现有合并场次的压制配方继续保留；未能核对新来源配方，可在工作台查看并调整。'
          : '合并场次默认继承最早来源的压制配方；未能核对其他来源配方，可在合并工作台查看并调整。'
        : appending
          ? '来源配方一致。现有合并场次的独立配方继续保留。'
          : '来源配方一致。合并场次继承最早来源的压制配方，随后可独立调整。'} /> : null}
  </FormDialog>
}
