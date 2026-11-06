'use client'
import {useRef,useState} from 'react'
import {Flag,LoaderCircle,LocateFixed,Pencil,Plus,RefreshCw,Trash2} from 'lucide-react'
import {clipsUrl,deleteClip,updateClip,usePolled,MAX_TITLE_CHARS,type Clip} from '@/app/lib/clips'
import {markersUrl,deleteMarker,renameMarker,formatSessionTime,type Marker} from '@/app/lib/markers'
import {sessionUrl,type SessionDetail} from '@/app/lib/sessions'
import {markerRange,type SavedRecordingSelection} from '@/app/lib/editor/recording-selection'
import {recordingRanges,recordingPoints} from '@/app/lib/editor/recording-ranges'
import type {EditorSession} from '@/app/lib/editor/session'
import {editorText as t} from '@/app/lib/editor/i18n'
import {useApp,type Project} from '@/app/lib/editor/state/store'
import {GlobalDialog} from '../../controls/GlobalDialog'
import {recordingImportFocus,focusRecordingRange,focusRecordingPoint} from './recording-focus'
import {useEditorAction} from '../../controls/useEditorAction'

type Dialog={action:'rename'|'delete';selection:SavedRecordingSelection}
const titleOf=(selection:SavedRecordingSelection)=>selection.kind==='clip'
  ? selection.value.title||t('Saved selection {0}',{0:selection.value.id})
  : selection.value.label||t('Recording marker {0}',{0:selection.value.id})

export function SavedRecordingSelections({session,source,kind}:{session:EditorSession;source:number;kind:'clips'|'markers'}){
  const app=useApp(),project=app.snapshot?.project as Project|undefined
  const detail=usePolled<SessionDetail>(sessionUrl(source),value=>value?.recording?2000:0)
  const clips=usePolled<{clips:Clip[]}>(kind==='clips'?clipsUrl(source):null,value=>detail.data?.recording||value?.clips.some(clip=>clip.state==='exporting')?2000:0)
  const markers=usePolled<{markers:Marker[]}>(kind==='markers'?markersUrl(source):null,()=>detail.data?.recording?2000:0)
  const [query,setQuery]=useState(''),[limit,setLimit]=useState(30)
  const [dialog,setDialog]=useState<Dialog>(),[name,setName]=useState('')
  const composing=useRef(false)
  const {busy,error,setError,editable,isCurrent,run}=useEditorAction()
  const dirty=!!app.biliupDirty||!!app.biliupSaving
  const list=kind==='clips'?clips:markers
  const rows:SavedRecordingSelection[]=kind==='clips'
    ? (clips.data?.clips??[]).filter(value=>value.state!=='discarded').map(value=>({kind:'clip',value}))
    : (markers.data?.markers??[]).map(value=>({kind:'marker',value}))
  const filtered=rows.filter(row=>titleOf(row).toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()))
    .sort((a,b)=>(a.kind==='clip'?a.value.in_ms:a.value.at_ms)-(b.kind==='clip'?b.value.in_ms:b.value.at_ms)||a.value.id-b.value.id)
  const refresh=async()=>{await Promise.all([detail.mutate(),list.mutate()])}
  const locate=(selection:SavedRecordingSelection)=>{
    setError('')
    try{if(selection.kind==='clip')focusRecordingRange(session,{sessionId:source,inMs:selection.value.in_ms,outMs:selection.value.out_ms},detail.data)
      else focusRecordingPoint(session,source,selection.value.at_ms,detail.data)}
    catch(cause){setError(cause instanceof Error?cause.message:String(cause))}
  }
  const openDialog=(action:Dialog['action'],selection:SavedRecordingSelection)=>{
    composing.current=false;setError('');setName(selection.kind==='clip'?selection.value.title:selection.value.label);setDialog({action,selection})
  }
  const saveDialog=()=>run(async()=>{
    if(!dialog||!editable)return
    const {action,selection}=dialog
    if(selection.kind==='clip'){
      if(action==='delete')await deleteClip(selection.value)
      else await updateClip(selection.value,{title:name.trim()})
    }else{
      const marker=selection.value
      if(action==='delete')await deleteMarker(source,marker.id,marker.session_id)
      else await renameMarker(source,marker.id,name.trim(),marker.session_id)
    }
    await refresh()
  },{onSuccess:()=>setDialog(undefined)})
  const problem=error||(list.error||detail.error?String(list.error??detail.error):'')
  return <>
    <div className="lib-section recording-selection-tools">
      <div className="recording-selection-search"><input className="field" aria-label={t('Search recording selections')} placeholder={t('Search')} value={query} onChange={event=>{setQuery(event.target.value);setLimit(30)}}/><button className="icon-btn" disabled={busy} title={t('Refresh')} aria-label={t('Refresh')} onClick={()=>void run(refresh,{readOnly:true})}><RefreshCw/></button></div>
      {problem&&!dialog&&<p role="alert">{problem}</p>}
    </div>
    <div className="suggestion-list">
      {!list.data&&!list.error?<p>{t('Loading…')}</p>:!filtered.length?<p>{t(query?'No matching selections':kind==='clips'?'No saved selections':'No recording markers')}</p>:filtered.slice(0,limit).map(selection=>{
        const range=selection.kind==='clip'?{sessionId:source,inMs:selection.value.in_ms,outMs:selection.value.out_ms}:detail.data?markerRange(selection.value,detail.data):undefined
        const present=project&&(selection.kind==='clip'
          ? recordingRanges(project,app.biliupSources??{},{sessionId:source,inMs:selection.value.in_ms,outMs:selection.value.out_ms},detail.data).length>0
          : recordingPoints(project,app.biliupSources??{},source,selection.value.at_ms,detail.data).length>0)
        return <article className="suggestion-card" key={selection.value.id}>
          <strong>{selection.kind==='marker'&&<Flag size={14}/>} {titleOf(selection)}</strong>
          {selection.kind==='marker'&&<span className="suggestion-time">{formatSessionTime(selection.value.at_ms)}</span>}
          {range&&<span className="suggestion-time">{formatSessionTime(range.inMs)} — {formatSessionTime(range.outMs)} · {t('{0} seconds',{0:Number(((range.outMs-range.inMs)/1000).toFixed(1))})}</span>}
          <div className="suggestion-actions">
            <button className="icon-btn" disabled={!present} title={t(present?'Locate on timeline':'Add this range to locate it on the timeline')} aria-label={t('Locate on timeline')} onClick={()=>locate(selection)}><LocateFixed/></button>
            <button className="btn primary" disabled={!editable||busy||dirty||!range||range.outMs<=range.inMs} title={dirty?t('Save your changes before adding a recording.'):undefined} onClick={()=>void run(async()=>{const focus=recordingImportFocus(session),added=await session.addRecordingSelection(selection);if(isCurrent())focus(added.items,added)})}>{busy?<LoaderCircle className="spin"/>:<Plus/>}{t('Add to timeline')}</button>
            <button className="icon-btn" disabled={!editable||busy} title={t('Rename')} aria-label={t('Rename')} onClick={()=>openDialog('rename',selection)}><Pencil/></button>
            <button className="icon-btn" disabled={!editable||busy} title={t('Delete')} aria-label={t('Delete')} onClick={()=>openDialog('delete',selection)}><Trash2/></button>
          </div>
        </article>
      })}
      {filtered.length>limit&&<button className="btn" onClick={()=>setLimit(value=>value+30)}>{t('Show more')}</button>}
    </div>
    {dialog&&<GlobalDialog label={t(dialog.action==='rename'?'Rename':dialog.selection.kind==='clip'?'Delete saved selection?':'Delete recording marker?')} busy={busy} onClose={()=>setDialog(undefined)}>
      <form className="modal suggestion-confirm" onSubmit={event=>{event.preventDefault();if(!composing.current)void saveDialog()}} onKeyDown={event=>{if(event.key==='Enter'&&(event.nativeEvent.isComposing||event.nativeEvent.keyCode===229))event.preventDefault()}}>
        <h2>{t(dialog.action==='rename'?'Rename':dialog.selection.kind==='clip'?'Delete saved selection?':'Delete recording marker?')}</h2>
        {dialog.action==='rename'?<input className="field" autoFocus disabled={busy||!editable} aria-label={t('Name')} value={name} maxLength={dialog.selection.kind==='clip'?MAX_TITLE_CHARS:100} onChange={event=>setName(event.target.value)} onCompositionStart={()=>{composing.current=true}} onCompositionEnd={()=>{composing.current=false}}/>:<><strong>{titleOf(dialog.selection)}</strong><p>{t(dialog.selection.kind==='clip'?'Its exported clip files will also be deleted. Timeline edits already added to projects are kept.':'The source marker will be deleted. Markers already added to project timelines are kept.')}</p></>}
        {problem&&<p role="alert">{problem}</p>}
        <footer><button type="button" className="btn" disabled={busy} onClick={()=>setDialog(undefined)}>{t('Cancel')}</button><button className={`btn ${dialog.action==='delete'?'danger':'primary'}`} disabled={busy||!editable}>{t(dialog.action==='delete'?'Delete':'Save')}</button></footer>
      </form>
    </GlobalDialog>}
  </>
}
