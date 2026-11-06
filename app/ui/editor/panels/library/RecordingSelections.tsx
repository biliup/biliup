'use client'
import {useId,useState} from 'react'
import useSWR from 'swr'
import {ArrowLeft} from 'lucide-react'
import {fetcher} from '@/app/lib/api-streamer'
import {sessionUrl,type SessionDetail} from '@/app/lib/sessions'
import type {EditorSession} from '@/app/lib/editor/session'
import {useApp,type Project} from '@/app/lib/editor/state/store'
import {editorText as t,useEditorLanguage} from '@/app/lib/editor/i18n'
import {RecordingSuggestions} from './RecordingSuggestions'
import {SavedRecordingSelections} from './SavedRecordingSelections'

export function RecordingSelections({session,onClose}:{session:EditorSession;onClose:()=>void}){
  useEditorLanguage()
  const tabId=useId(),tabs=['clips','markers','suggestions'] as const
  const app=useApp(),project=app.snapshot?.project as Project|undefined
  const sources=(project?.media??[]).flatMap(media=>{const source=app.biliupSources?.[media.id];return source?.kind==='recording'?[source]:[]})
  const [chosen,setChosen]=useState<number|undefined>(sources[0]?.viewSessionId),[tab,setTab]=useState<'clips'|'markers'|'suggestions'>('clips')
  const ids=[...new Set(sources.flatMap(source=>tab==='suggestions'?[source.sessionId]:[source.viewSessionId,source.sessionId]))]
  const source=chosen!==undefined&&ids.includes(chosen)?chosen:ids[0]
  return <aside className="library recording-selections">
    <div className="lib-head"><button className="icon-btn" aria-label={t('Back to media')} onClick={onClose}><ArrowLeft/></button><span>{t('Recording selections')}</span></div>
    {source&&<div className="lib-section recording-selection-source"><select className="field" aria-label={t('Recording session')} value={source} onChange={event=>setChosen(Number(event.target.value))}>{ids.map(id=><SessionOption key={id} id={id}/>)}</select></div>}
    <div className="recording-selection-tabs" role="tablist" aria-label={t('Recording selections')}>{tabs.map((value,index)=><button key={value} id={`${tabId}-${value}`} aria-controls={`${tabId}-panel`} className="btn" role="tab" tabIndex={tab===value?0:-1} aria-selected={tab===value} onClick={()=>setTab(value)} onKeyDown={event=>{
      const next=event.key==='ArrowRight'?(index+1)%tabs.length:event.key==='ArrowLeft'?(index+tabs.length-1)%tabs.length:event.key==='Home'?0:event.key==='End'?tabs.length-1:undefined
      if(next===undefined)return;const value=tabs[next];if(!value)return;event.preventDefault();setTab(value);document.getElementById(`${tabId}-${value}`)?.focus()
    }}>{t({clips:'Saved selections',markers:'Recording markers',suggestions:'Suggestions'}[value])}</button>)}</div>
    {source?<div id={`${tabId}-panel`} role="tabpanel" aria-labelledby={`${tabId}-${tab}`}>{tab==='suggestions'?<RecordingSuggestions key={`${session.id}:${source}`} session={session} source={source}/>:<SavedRecordingSelections key={`${session.id}:${source}:${tab}`} session={session} source={source} kind={tab}/>}</div>:<p className="lib-section">{t('No recordings added')}</p>}
  </aside>
}

function SessionOption({id}:{id:number}){
  const {data}=useSWR<SessionDetail>(sessionUrl(id),fetcher)
  return <option value={id}>{data?`${data.title||data.streamer_name} · ${new Date(data.started_at).toLocaleDateString()}${data.kind==='group'?` · ${t('Merged session')}`:''}`:t('Session {0}',{0:id})}</option>
}
