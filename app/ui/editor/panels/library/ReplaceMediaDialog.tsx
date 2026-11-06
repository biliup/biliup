'use client'
import { useEffect, useMemo, useRef, useState } from 'react'
import { FileUp, RefreshCw, X } from 'lucide-react'
import type { EditorSession } from '@/app/lib/editor/session'
import type { Media } from '@/app/lib/editor/core/timeline/types'
import type { Overrun } from '@/app/lib/editor/core/timeline/reshoot'
import { replacementPlan, type PreparedReplacement } from '@/app/lib/editor/media-replacement'
import { chooseEditorFiles,EDITOR_MEDIA_ACCEPT } from '@/app/lib/editor/browser-imports'
import { editorText as t, useEditorLanguage } from '@/app/lib/editor/i18n'
import { projectOf, useApp } from '@/app/lib/editor/state/store'
import { GlobalDialog } from '../../controls/GlobalDialog'
import { Segmented } from '../../inspector/controls'
import {useEditorAction} from '../../controls/useEditorAction'

export function ReplaceMediaDialog({ session, target, onClose }: { session: EditorSession; target: Media; onClose: () => void }) {
  const language = useEditorLanguage()
  const app = useApp()
  const [prepared, setPrepared] = useState<PreparedReplacement>()
  const [overrun, setOverrun] = useState<Overrun>('hold')
  const {run,busy,error,setError,editable}=useEditorAction()
  const [limit, setLimit] = useState(50)
  const receipt = useRef<PreparedReplacement | undefined>(undefined)
  useEffect(() => () => { if (receipt.current) session.discardReplacement(receipt.current.token) }, [session])
  const unconfirmed=prepared?session.replacementPending(prepared.token):false
  const available=editable&&!app.biliupDirty
  const preview = useMemo(() => {
    if (!prepared || !app.snapshot) return undefined
    try { return replacementPlan(projectOf(app.snapshot), prepared, target.kind === 'video' ? overrun : 'trim') }
    catch (problem) { return { problem: problem instanceof Error ? problem.message : String(problem) } }
  }, [prepared, app.snapshot, overrun, target.kind, language])
  const choose = () => {
    if(!available||unconfirmed)return
    void run(async current=>{
      const picked = await chooseEditorFiles(false, EDITOR_MEDIA_ACCEPT[target.kind], false)
      if(!current())return
      if(picked.problems.length)throw new Error(picked.problems.join('\n'))
      if(!picked.files.length)return
      const next = await session.prepareReplacement(target, picked.files[0]!)
      if (!current()) { session.discardReplacement(next.token); return }
      if (receipt.current) session.discardReplacement(receipt.current.token)
      receipt.current = next; setPrepared(next); setLimit(50)
    })
  }
  const commit = () => {
    if(!available||!prepared||!app.snapshot||busy||(!unconfirmed&&(!preview||'problem' in preview)))return
    const revision = app.snapshot.revision
    void run(async()=>{
      await session.replaceFile(prepared.token, target.kind === 'video' ? overrun : 'trim', revision)
    },{onSuccess:onClose})
  }
  return <GlobalDialog label={t('Replace file')} busy={busy} onClose={onClose}>
    <section className="modal replace-media" aria-busy={busy}>
      <header className="replace-media-header"><h2>{t('Replace file')}</h2><button className="icon-btn" disabled={busy} aria-label={t('Close')} onClick={onClose}><X /></button></header>
      <strong className="ellipsis" title={target.name}>{target.name}</strong>
      <button className="btn replace-media-file" disabled={busy || !available || unconfirmed} onClick={choose}>
        {busy ? <RefreshCw className="spin" /> : <FileUp />}<span>{prepared?.media.name ?? t('Choose replacement file')}</span>
      </button>
      {prepared && target.kind === 'video' && <Segmented<Overrun> testId="replace-media-overrun" disabled={busy||!available||unconfirmed} value={overrun} onChange={value => { setOverrun(value); setError('') }} options={[
        ['hold', t('Keep clip lengths')], ['trim', t('Trim to the new file')],
      ]} />}
      {preview && ('problem' in preview ? <p className="inline-error" role="alert">{preview.problem}</p> : <>
        <p>{t('Updates {0} timeline references.', { 0: preview.uses })}</p>
        {preview.changes.length > 0 && <div className="replace-media-changes"><ul>{preview.changes.slice(0, limit).map((text, index) => <li key={index}>{text}</li>)}</ul>
          {preview.changes.length > limit && <button className="link-btn" onClick={() => setLimit(limit + 50)}>{t('Show more')}</button>}
        </div>}
      </>)}
      {error && <p className="inline-error" role="alert">{error}</p>}
      <footer className="confirm-actions"><button className="btn ghost" disabled={busy} onClick={onClose}>{t('Cancel')}</button>
        <button className="btn primary" disabled={busy || !available || !prepared || (!unconfirmed&&(!preview||'problem' in preview))} onClick={commit}>{t(unconfirmed?'Check save':'Replace file')}</button>
      </footer>
    </section>
  </GlobalDialog>
}
