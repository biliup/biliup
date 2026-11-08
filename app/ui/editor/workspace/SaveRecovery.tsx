'use client'
import { useEffect, useMemo, useRef, useState } from 'react'
import { Minus, Pencil, Plus, RefreshCw, X } from 'lucide-react'
import { editorText as t, useEditorLanguage } from '@/app/lib/editor/i18n'
import type { EditorSession } from '@/app/lib/editor/session'
import { saveDifferences, type SaveReview } from '@/app/lib/editor/save-review'
import { useApp } from '@/app/lib/editor/state/store'
import { GlobalDialog } from '../controls/GlobalDialog'

/** Shares the editor's current draft and server history; never opens another editor. */
export function SaveRecovery({ session }: { session: EditorSession }) {
  const language = useEditorLanguage()
  const app = useApp()
  const [open, setOpen] = useState(false)
  const [review, setReview] = useState<SaveReview>()
  const [choice, setChoice] = useState<'local' | 'server'>('local')
  const [busy, setBusy] = useState<'loading' | 'saving' | 'retry'>()
  const [error, setError] = useState<string>()
  const [limit, setLimit] = useState(50)
  const request = useRef(0)
  useEffect(() => () => { request.current++ }, [session])
  const differences = useMemo(() => review ? saveDifferences(
    choice === 'local' ? review.local : review.remote.document,
    choice === 'local' ? review.remote.document : review.local,
  ) : [], [review, choice, language])
  const stale = !!review && (app.snapshot?.dir !== 'biliup:' + review.projectId || app.snapshot?.revision !== review.localRevision)
  const close = () => {
    if (busy) return
    request.current++
    setOpen(false)
    setReview(undefined)
    setError(undefined)
  }
  const load = async () => {
    const token = ++request.current
    setOpen(true); setBusy('loading'); setError(undefined); setReview(undefined); setLimit(50)
    try {
      const next = await session.reviewSave()
      if (token === request.current) setReview(next)
    } catch (problem) {
      if (token === request.current) setError(problem instanceof Error ? problem.message : String(problem))
    } finally {
      if (token === request.current) setBusy(undefined)
    }
  }
  const retry = async () => {
    const token = ++request.current
    setBusy('retry')
    try { await session.retrySave() }
    finally { if (token === request.current) setBusy(undefined) }
  }
  const resolve = async () => {
    if (!review || stale || busy) return
    const token = ++request.current
    setBusy('saving'); setError(undefined)
    try {
      await session.resolveSave(review, choice)
      if (token === request.current) { setOpen(false); setReview(undefined) }
    } catch (problem) {
      if (token === request.current) setError(problem instanceof Error ? problem.message : String(problem))
    } finally {
      if (token === request.current) setBusy(undefined)
    }
  }
  return <>
    {app.biliupSaveError && <div className="biliup-save-error" role="alert">
      <span>{app.biliupSaveError}</span>
      {!app.biliupSaveConflict && <button className="btn" disabled={!!busy || app.biliupSaving || !app.biliupCanEdit} onClick={() => void retry()}>{t('重试保存')}</button>}
      <button className="btn" disabled={!!busy || app.biliupSaving} onClick={() => void load()}>{t(app.biliupSaveConflict ? 'Resolve conflict' : 'Compare with server')}</button>
    </div>}
    {open && <GlobalDialog label={t('Review unsaved changes')} busy={!!busy} onClose={close}>
      <section className="modal save-review" aria-busy={!!busy}>
        <header className="save-review-header">
          <h2>{t('Review unsaved changes')}</h2>
          <button className="icon-btn" disabled={!!busy} aria-label={t('Close')} onClick={close}><X /></button>
        </header>
        {busy === 'loading' && <div className="save-review-loading" role="status"><RefreshCw className="spin" />{t('Loading…')}</div>}
        {review && <>
          <div className="save-review-choices">
            <label className="save-review-choice" data-selected={choice === 'local'}>
              <input type="radio" name="save-version" value="local" checked={choice === 'local'} disabled={!!busy} onChange={() => { setChoice('local'); setLimit(50) }} />
              <strong>{t('Keep current edits')}</strong>
              <span className="ellipsis">{review.local.project.name}</span>
              <span>{review.local.project.canvas.width} × {review.local.project.canvas.height} · {review.local.project.canvas.fps} {t(' fps')}</span>
            </label>
            <label className="save-review-choice" data-selected={choice === 'server'}>
              <input type="radio" name="save-version" value="server" checked={choice === 'server'} disabled={!!busy} onChange={() => { setChoice('server'); setLimit(50) }} />
              <strong>{t('Use server version')}</strong>
              <span className="ellipsis">{review.remote.name}</span>
              <span>{t('Revision {0}', { 0: review.remote.revision })} · {new Date(review.remote.updated_at).toLocaleString(language)}</span>
            </label>
          </div>
          <div className="save-review-differences">
            {differences.length === 0 ? <p>{t('Both versions have the same content.')}</p> : <ul>
              {differences.slice(0, limit).map((row, index) => <li key={index} data-kind={row.kind}>
                {row.kind === 'added' ? <Plus size={14} /> : row.kind === 'removed' ? <Minus size={14} /> : <Pencil size={14} />}
                <span className="save-review-kind">{t(row.kind === 'added' ? 'Added' : row.kind === 'removed' ? 'Removed' : 'Changed')}</span>
                <span>{row.label}</span>
              </li>)}
            </ul>}
            {differences.length > limit && <button className="link-btn" onClick={() => setLimit(limit + 50)}>{t('Show more')}</button>}
          </div>
          {choice === 'server' && differences.length > 0 && <p className="save-review-discard">{t('This discards the unsaved edits in this page.')}</p>}
          {stale && <p className="inline-error" role="alert">{t('Local edits have changed. Refresh the comparison.')}</p>}
        </>}
        {error && <p className="inline-error" role="alert">{error}</p>}
        <footer className="save-review-actions">
          <button className="btn ghost" disabled={!!busy} onClick={() => void load()}><RefreshCw size={14} />{t('Refresh comparison')}</button>
          <span className="grow" />
          <button className="btn ghost" disabled={!!busy} onClick={close}>{t('Cancel')}</button>
          <button className={'btn ' + (choice === 'server' && differences.length ? 'danger' : 'primary')} disabled={!review || stale || !!busy || (choice === 'local' && !app.biliupCanEdit)} onClick={() => void resolve()}>
            {t(choice === 'local' ? 'Save current edits' : differences.length ? 'Discard edits and load' : 'Load saved version')}
          </button>
        </footer>
      </section>
    </GlobalDialog>}
  </>
}
