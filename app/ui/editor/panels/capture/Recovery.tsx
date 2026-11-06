import {useEffect} from 'react'
import {Download,RefreshCw,Trash2} from 'lucide-react'
import {discardCapture,downloadCapture,loadCaptureRecovery,retryCapture,useCaptureRecovery} from '../../capture/recovery'
import {useApp} from '@/app/lib/editor/state/store'
import {editorText,useEditorLanguage} from '@/app/lib/editor/i18n'

export function CaptureRecovery(){
  useEditorLanguage()
  const app=useApp(),entries=useCaptureRecovery().filter(entry=>entry.draft.dir===app.snapshot?.dir)
  useEffect(()=>{void loadCaptureRecovery()},[])
  if(!entries.length)return null
  return <div className="capture-recovery" aria-live="polite">{entries.map(entry=>{
    const {draft,progress}=entry
    const name=draft.name||editorText(draft.multi?'Multi-source recording':{screen:'Screen recording',camera:'Camera recording',voiceover:'Voiceover',system:'System audio'}[draft.parts[0]!.kind])
    const status=progress?.phase==='uploading'?`${editorText('Uploading…')} ${Math.round(100*progress.loaded/Math.max(1,progress.total))}%`:progress?.phase==='processing'?editorText('Processing recording…'):progress?.phase==='saving'?editorText('Saving to project…'):undefined
    return <div className="capture-recovery-item" key={draft.id}>
      <div className="capture-recovery-title"><strong className="ellipsis" title={name}>{name}</strong><span>{new Date(draft.createdAt).toLocaleTimeString([],{hour:'2-digit',minute:'2-digit'})}</span></div>
      {status&&<div className="capture-recovery-status"><RefreshCw className="spin" size={14}/>{status}</div>}
      {entry.problem&&<div className="capture-message error" role="alert">{editorText(entry.problem)}</div>}
      {!entry.durable&&<div className="capture-message error">{editorText('Browser storage is unavailable. Keep this page open or download the recording.')}</div>}
      {!entry.busy&&<div className="capture-recovery-actions">
        <button className="btn" disabled={!app.biliupCanEdit||app.connection!=='connected'} onClick={()=>void retryCapture(draft.id)}><RefreshCw size={14}/>{editorText('Retry save')}</button>
        {draft.parts.map((part,index)=><button key={part.kind} className="icon-btn" aria-label={editorText('Download recording: {0}',{0:editorText({screen:'Screen',camera:'Camera',voiceover:'Microphone',system:'System audio'}[part.kind])})} title={editorText(part.kind==='screen'?'Screen':part.kind==='camera'?'Camera':part.kind==='system'?'System audio':'Microphone')} onClick={()=>downloadCapture(draft.id,index)}><Download size={14}/></button>)}
        <button className="icon-btn danger" aria-label={editorText('Discard recording')} title={editorText('Discard recording')} onClick={()=>void discardCapture(draft.id)}><Trash2 size={14}/></button>
      </div>}
    </div>
  })}</div>
}
