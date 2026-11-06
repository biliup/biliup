'use client'
import {useEffect,useRef,useState,useSyncExternalStore} from 'react'
import useSWR from 'swr'
import dynamic from 'next/dynamic'
import {ChevronDown,Film,Play,Search,X} from 'lucide-react'
import {API_BASE,fetcher} from '@/app/lib/api-streamer'
import {formatSpan,isReadable,segmentEnd,sessionUrl,type SegmentView,type SessionDetail,type SessionPage,type SessionSummary} from '@/app/lib/sessions'
import type {EditorSession} from '@/app/lib/editor/session'
import type {Project} from '@/app/lib/editor/core/timeline/types'
import {currentEditorStore} from '@/app/lib/editor/state/store/core'
import {editorText as t,useEditorLanguage} from '@/app/lib/editor/i18n'
import {GlobalDialog} from '../controls/GlobalDialog'

const Player=dynamic(()=>import('@/app/ui/Player'),{ssr:false})
const mediaId=(sessionId:number,segment:SegmentView)=>`recording-${segment.source_session_id??sessionId}-${segment.id}`
type Selection=Record<string,{sessionId:number;segmentId:number}>

export function SourcePicker({project,onClose}:{project:EditorSession;onClose:()=>void}){
 useEditorLanguage()
 const [page,setPage]=useState(1),[search,setSearch]=useState(''),[query,setQuery]=useState('')
 const [selected,setSelected]=useState<Selection>({}),[busy,setBusy]=useState(false),[error,setError]=useState('')
 const [playing,setPlaying]=useState<{sessionId:number;segment:SegmentView}>()
 const state=useSyncExternalStore(project.store.subscribe,project.store.getSnapshot,project.store.getSnapshot)
 const mounted=useRef(true),submitting=useRef(false)
 useEffect(()=>{mounted.current=true;return()=>{mounted.current=false}},[project])
 const active=()=>mounted.current&&currentEditorStore()===project.store
 const editable=state.connection==='connected'&&!!state.biliupCanEdit
 const present=new Set((state.snapshot?.project as Project|undefined)?.media.flatMap(media=>{
  const source=state.biliupSources?.[media.id]
  return source?.kind==='recording'?[`recording-${source.sessionId}-${source.segmentId}`]:[]
 }))
 useEffect(()=>{const timer=setTimeout(()=>{setQuery(search.trim());setPage(1)},250);return()=>clearTimeout(timer)},[search])
 const {data,error:loadError}=useSWR<SessionPage>(`/v1/sessions?page=${page}&page_size=20&q=${encodeURIComponent(query)}`,fetcher,{refreshInterval:10000})
 const count=Object.keys(selected).filter(id=>!present.has(id)).length
 const toggle=(sessionId:number,segments:SegmentView[],checked:boolean)=>setSelected(previous=>{
  const next={...previous}
  for(const segment of segments){const id=mediaId(sessionId,segment);if(checked)next[id]={sessionId,segmentId:segment.id};else delete next[id]}
  return next
 })
 const add=async()=>{
  if(submitting.current||!editable||!active()||!count)return
  submitting.current=true
  setBusy(true);setError('')
  try{
   const groups=new Map<number,Set<number>>()
   for(const [id,item] of Object.entries(selected)){if(present.has(id))continue;const group=groups.get(item.sessionId)??new Set<number>();group.add(item.segmentId);groups.set(item.sessionId,group)}
   await project.importRecordings([...groups].map(([sessionId,ids])=>({sessionId,segmentIds:[...ids]})))
   if(active())onClose()
  }catch(cause){if(active())setError(cause instanceof Error?cause.message:String(cause))}finally{submitting.current=false;if(active())setBusy(false)}
 }
 return <GlobalDialog label={t('Add recordings')} onClose={onClose} busy={busy}><section className="modal workbench-picker">
  <header className="source-section"><h2>{t('Add recordings')}</h2><button className="icon-btn" aria-label={t('Close')} disabled={busy} onClick={onClose}><X/></button></header>
  <label className="recording-picker-search"><Search size={16}/><input className="field" aria-label={t('Search recordings')} placeholder={t('Search streamer, session or filename')} maxLength={256} value={search} disabled={busy} onChange={event=>setSearch(event.target.value)}/></label>
  {(error||loadError)&&<p role="alert">{error||String(loadError)}</p>}
  <div className="workbench-picker-scroll">
   {!data?<p>{t('Loading…')}</p>:!data.items.length?<p className="recording-picker-empty">{t('No recordings found')}</p>:data.items.map(session=><RecordingGroup key={session.id} session={session} query={query} selected={selected} present={present} busy={busy||!editable} toggle={toggle} preview={segment=>setPlaying({sessionId:session.id,segment})}/>)}
  </div>
  <footer className="recording-picker-footer"><div className="workspace-bin-pager"><button className="btn" disabled={busy||page<=1} onClick={()=>setPage(page-1)}>{t('Previous')}</button><span>{page} / {Math.max(1,Math.ceil((data?.total??0)/20))}</span><button className="btn" disabled={busy||!data||page*20>=data.total} onClick={()=>setPage(page+1)}>{t('Next')}</button></div><button className="btn primary" disabled={busy||!editable||!count} onClick={()=>void add()}>{busy?t('Adding…'):t('Add {0} recordings',{0:count})}</button></footer>
  {playing&&<GlobalDialog label={playing.segment.file_name} onClose={()=>setPlaying(undefined)}><section className="modal workspace-output"><header><strong>{playing.segment.file_name}</strong><button className="icon-btn" aria-label={t('Close preview')} onClick={()=>setPlaying(undefined)}><X/></button></header><Player url={`${API_BASE}/v1/editor/sessions/${playing.sessionId}/media/${mediaId(playing.sessionId,playing.segment)}/file`} type={playing.segment.container==='flv'?'flv':playing.segment.container==='ts'?'mpegts':undefined} height="50vh"/></section></GlobalDialog>}
 </section></GlobalDialog>
}

function RecordingGroup({session,query,selected,present,busy,toggle,preview}:{session:SessionSummary;query:string;selected:Selection;present:Set<string>;busy:boolean;toggle:(id:number,segments:SegmentView[],checked:boolean)=>void;preview:(segment:SegmentView)=>void}){
 const [open,setOpen]=useState(true)
 const {data,error}=useSWR<SessionDetail>(sessionUrl(session.id),fetcher,{refreshInterval:session.recording?10000:0})
 const term=query.toLocaleLowerCase(),sessionMatches=`${session.streamer_name} ${session.title}`.toLocaleLowerCase().includes(term)
 const segments=(data?.segments??[]).filter(segment=>sessionMatches||segment.file_name.toLocaleLowerCase().includes(term)||data?.members?.some(member=>member.source_session_id===segment.source_session_id&&member.title.toLocaleLowerCase().includes(term)))
 const available=segments.filter(segment=>isReadable(segment)&&segmentEnd(segment)>segment.start_ms&&!present.has(mediaId(session.id,segment)))
 const checked=available.length>0&&available.every(segment=>selected[mediaId(session.id,segment)])
 const some=available.some(segment=>selected[mediaId(session.id,segment)])
 return <article className="recording-picker-group">
  <header><input type="checkbox" aria-label={t('Select recordings from {0}',{0:session.title||session.streamer_name})} disabled={busy||!available.length} checked={checked} ref={input=>{if(input)input.indeterminate=some&&!checked}} onChange={event=>toggle(session.id,available,event.target.checked)}/><button className="recording-picker-heading" aria-expanded={open} onClick={()=>setOpen(!open)}><div><strong>{session.title||session.streamer_name}</strong><span>{session.streamer_name} · {new Date(session.started_at).toLocaleString()}{session.recording?` · ${t('Recording now')}`:''}</span></div><ChevronDown size={16} style={{transform:open?undefined:'rotate(-90deg)'}}/></button></header>
  {open&&<div className="recording-picker-files">{error?<p role="alert">{String(error)}</p>:!data?<p>{t('Loading…')}</p>:!segments.length?<p>{t('No recordings found')}</p>:segments.map(segment=>{
   const id=mediaId(session.id,segment),added=present.has(id),readable=isReadable(segment)&&segmentEnd(segment)>segment.start_ms
   return <div className="recording-picker-file" key={segment.id}><label><input type="checkbox" disabled={busy||added||!readable} checked={added||!!selected[id]} onChange={event=>toggle(session.id,[segment],event.target.checked)}/><Film size={18}/><span className="recording-picker-name">{segment.file_name}</span></label><span className="recording-picker-meta">{added?t('Already added'):!readable?t('Unavailable'):formatSpan(segmentEnd(segment)-segment.start_ms)}</span><button className="icon-btn" aria-label={t('Play {0}',{0:segment.file_name})} disabled={!readable} onClick={()=>preview(segment)}><Play size={16}/></button></div>
  })}</div>}
 </article>
}
