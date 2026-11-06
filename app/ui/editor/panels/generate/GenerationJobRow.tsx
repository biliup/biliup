// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { LoaderCircle, X } from 'lucide-react'
import { useEffect, useState } from 'react'
import type { GenerateJob } from '@/app/lib/editor/contracts'
import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n'
import { actions, useApp } from '@/app/lib/editor/state/store'
import {useEditorAction} from '../../controls/useEditorAction'

/** The same running job and import controls in Generate and the sound library. */
export function GenerationJobRow({ job, online }: { job: GenerateJob; online: boolean }) {
  useEditorLanguage()
  const app=useApp()
  const agentWorking=!!job.agentTurnId&&app.agent?.running===job.agentTurnId
  const {busy,error,editable,run}=useEditorAction()
  const [now,setNow]=useState(Date.now())
  useEffect(()=>{
    const timer=setInterval(()=>setNow(Date.now()),500)
    return ()=>clearInterval(timer)
  },[])
  const elapsed=(now-Date.parse(job.startedAt))/1000
  const secs=Number.isFinite(elapsed)?Math.max(0,Math.floor(elapsed)):0
  const disabled=!online||!editable||busy
  return <div className="import-progress gen-progress" data-testid="generate-progress">
    <LoaderCircle className="spin"/>
    <span className="gen-status">{editorText(job.status??'Working…')} <span className="muted">{secs} {editorText(' s')}</span></span>
    {job.phase==='importing'&&!agentWorking&&<button className="link-btn" disabled={disabled} onClick={()=>run(()=>actions.importGenerated(job.id))}>{editorText('Retry import')}</button>}
    <button className="link-btn" data-testid="generate-cancel" disabled={disabled} title={editorText('Cancel locally; the provider may still charge for submitted work')} onClick={()=>run(()=>actions.cancelGenerate(job.id))}>
      <X/>{editorText('Cancel')}
    </button>
    <div className="bar indeterminate"><span/></div>
    {error&&<span className="inline-error" role="alert">{error}</span>}
  </div>
}
