'use client'
import { useEffect, useRef, useState } from 'react'
import type { ProjectSnapshot } from '@/app/lib/editor/contracts'
import { mediaUses, itemNeedsMedia, planRemoveMedia } from '@/app/lib/editor/core/timeline/mediaPlan'
import { editorText as t, useEditorLanguage } from '@/app/lib/editor/i18n'
import { actions, projectOf, useApp } from '@/app/lib/editor/state/store'
import { currentEditorStore } from '@/app/lib/editor/state/store/core'
import { GlobalDialog } from '../../controls/GlobalDialog'

function reviewRemoval(snapshot: ProjectSnapshot, mediaId: string, path: string) {
  const project = projectOf(snapshot), media = project.media.find(item => item.id === mediaId && item.path === path)
  if (!media) return undefined
  const uses = mediaUses(project, mediaId), count = uses.filter(use => itemNeedsMedia(use.item, mediaId)).length
  return { dir: snapshot.dir, revision: snapshot.revision, media, count, effects: uses.length - count, plan: planRemoveMedia(project, mediaId) }
}

/** Confirmation and the submitted edit refer to the same visible revision. */
export function RemoveMediaDialog({ mediaId, path, dir, onClose }: { mediaId: string; path: string; dir: string; onClose: () => void }) {
  useEditorLanguage()
  const app = useApp()
  const [owner] = useState(currentEditorStore)
  const [review, setReview] = useState(() => app.snapshot?.dir === dir ? reviewRemoval(app.snapshot, mediaId, path) : undefined)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  const active = useRef(true)
  useEffect(() => { active.current = true; return () => { active.current = false } }, [])
  const stale = review?.revision !== app.snapshot?.revision
  const available = app.snapshot?.dir === dir && owner === currentEditorStore()
  useEffect(() => { if (!available) onClose() }, [available, onClose])
  const editable = available && app.connection === 'connected' && app.biliupCanEdit
  const refresh = () => {
    if (app.snapshot?.dir !== dir) return
    setReview(reviewRemoval(app.snapshot, mediaId, path)); setError(undefined)
  }
  const remove = async () => {
    if (!editable || busy || stale || !review || 'problem' in review.plan) return
    setBusy(true); setError(undefined)
    try {
      const removed = await actions.removeMedia(mediaId, { dir, baseRevision: review.revision })
      if (!active.current || owner !== currentEditorStore()) return
      if (removed) onClose()
      else setError(owner?.getSnapshot().notice?.text ?? t('The project changed. Review the removal again.'))
    } catch (problem) {
      if (active.current && owner === currentEditorStore()) setError(problem instanceof Error ? problem.message : String(problem))
    } finally {
      if (active.current) setBusy(false)
    }
  }
  if (!available) return null
  return <GlobalDialog label={t('Remove from project')} busy={busy} onClose={onClose}>
    <section className="modal confirm-modal" data-testid="remove-media-confirm" aria-busy={busy}>
      <div className="confirm-title">{review ? t('Remove {0} from the project?', { 0: review.media.name }) : t('This media changed or was removed.')}</div>
      {review && <div className="confirm-text" data-testid="remove-media-count">
        {review.count > 0 && (review.count === 1 ? t('1 item on the timeline uses it and goes too.') : t('{0} items on the timeline use it and go too.', { 0: review.count }))}
        {review.effects > 0 && <div>{t('Image effects on {0} clips will be removed; those clips will stay.', { 0: review.effects })}</div>}
      </div>}
      {review && 'problem' in review.plan && <p className="inline-error" role="alert">{t(review.plan.problem)}</p>}
      {stale && review && <p className="inline-error" role="alert">{t('The project changed. Review the removal again.')}</p>}
      {error && <p className="inline-error" role="alert">{t(error)}</p>}
      <div className="confirm-actions">
        <button className="btn" data-testid="remove-media-cancel" autoFocus disabled={busy} onClick={onClose}>{t('Cancel')}</button>
        {review && (stale ? <button className="btn primary" disabled={!editable || busy} onClick={refresh}>{t('Refresh')}</button>
          : <button className="btn danger" data-testid="remove-media-ok" disabled={!editable || busy || 'problem' in review.plan} onClick={() => void remove()}>{t('Remove')}</button>)}
      </div>
    </section>
  </GlobalDialog>
}
