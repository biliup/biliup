'use client'
import {focusRecordingItems,focusRecordingRange,recordingImportFocus} from './recording-focus'
import {useState} from 'react'
import useSWR from 'swr'
import {Check,LoaderCircle,LocateFixed,Plus,Sparkles,Trash2,X} from 'lucide-react'
import {fetcher} from '@/app/lib/api-streamer'
import {useMe} from '@/app/lib/use-me'
import {useNowSec} from '@/app/lib/use-dashboard'
import {formatSessionTime} from '@/app/lib/markers'
import {sessionUrl,type SessionDetail} from '@/app/lib/sessions'
import {AUTO_CLIP_SETTINGS_HREF,autoClipError,cancelAutoClip,dismissSuggestion,effectiveState,isActive,useAutoClip,useSessionAutoClip,useSuggestions,type Suggestion,type EditorSuggestionUse} from '@/app/lib/auto-clip'
import type {EditorSession} from '@/app/lib/editor/session'
import {editorText as t} from '@/app/lib/editor/i18n'
import {actions,useApp} from '@/app/lib/editor/state/store'
import {GlobalDialog} from '../../controls/GlobalDialog'
import {SuggestionEstimate} from './SuggestionEstimate'
import {useEditorAction} from '../../controls/useEditorAction'

export function RecordingSuggestions({session,source}:{session:EditorSession;source:number}){
  const app=useApp(),{can}=useMe(),now=useNowSec()*1000
  const {data:detail,error:detailError}=useSWR<SessionDetail>(sessionUrl(source),fetcher)
  const availability=useAutoClip(),jobState=useSessionAutoClip(source),list=useSuggestions(source),job=jobState.data?.job
  const [filter,setFilter]=useState<'pending'|'all'>('pending'),[sort,setSort]=useState<'time'|'confidence'>('time')
  const [estimate,setEstimate]=useState(false)
  const [confirm,setConfirm]=useState<Suggestion|'cancel'>()
  const {busy,error,setError,editable,isCurrent,run}=useEditorAction()
  const dirty=!!app.biliupDirty||!!app.biliupSaving
  const openConfirm=(value:Suggestion|'cancel')=>{setError('');setConfirm(value)}
  const locate=(suggestion:Suggestion,used?:EditorSuggestionUse)=>{
    setError('')
    try{
      if(used?.projectId===session.id)focusRecordingItems(session,used.items)
      else focusRecordingRange(session,{sessionId:source,inMs:suggestion.in_ms,outMs:suggestion.out_ms})
    }catch(cause){setError(autoClipError(cause))}
  }
  const rows=(list.data?.suggestions??[]).filter(item=>filter==='all'||effectiveState(item,now)==='pending').sort((a,b)=>sort==='confidence'?(b.confidence??-1)-(a.confidence??-1)||a.in_ms-b.in_ms:a.in_ms-b.in_ms)
  const uses=new Map((list.data?.editor_uses??[]).map(use=>[use.suggestionId,use]))
  const active=isActive(job),status=availability.status
  const setupReason=!status?t('Loading…'):!status.enabled?t('Enable automatic clip suggestions in settings.'):!status.configured||!status.asr_model?t('Configure the analysis and transcription models in settings.'):null
  const blocked=setupReason??(detail?.recording?t('Wait until the recording ends.'):!detail?t('Loading…'):null)
  const jobLabel=job?t(job.state==='running'?({audio:'Extracting audio',asr:'Transcribing',signals:'Reading recording signals',analyze:'Analyzing'}[job.stage??'audio']):({queued:'Queued',done:'Done',failed:'Failed',canceled:'Cancelled'}[job.state])):''
  return <>
    <div className="lib-section suggestion-tools">
      {active?<button className="btn" disabled={!editable||busy} onClick={()=>openConfirm('cancel')}><X/>{t('Cancel generation')}</button>:<button className="btn" disabled={!editable||busy||!!blocked} title={blocked??undefined} onClick={()=>{setError('');setEstimate(true)}}><Sparkles/>{t(job?'Generate again':'Generate suggestions')}</button>}
      {!!setupReason&&<div className="suggestion-problem">{setupReason}{can('config.view')&&<a className="link-btn" href={AUTO_CLIP_SETTINGS_HREF}>{t('Open settings')}</a>}</div>}
      {job&&<div className="suggestion-job" aria-live="polite"><span>{jobLabel}{job.state==='running'&&job.progress_total>0?` · ${Math.min(job.progress_done,job.progress_total)}/${job.progress_total}`:''}{job.state==='queued'&&job.not_before>now?` · ${new Date(job.not_before).toLocaleTimeString()}`:''}</span>{job.state==='running'&&job.progress_total>0&&<progress aria-label={t('Generation progress')} value={Math.min(job.progress_done,job.progress_total)} max={job.progress_total}/>}</div>}
      {job?.error&&<p role="alert">{job.error}</p>}
      {!!job?.warnings.length&&<details><summary>{t('Warnings')} ({job.warnings.length})</summary><ul>{job.warnings.map((warning,index)=><li key={index}>{warning}</li>)}</ul></details>}
      {!confirm&&(error||detailError||jobState.error||list.error)&&<p role="alert">{error||autoClipError(detailError??jobState.error??list.error)}</p>}
      <div className="suggestion-filters"><select className="field" aria-label={t('Suggestion status')} value={filter} onChange={e=>setFilter(e.target.value as typeof filter)}><option value="pending">{t('Pending')}</option><option value="all">{t('All')}</option></select><select className="field" aria-label={t('Sort by')} value={sort} onChange={e=>setSort(e.target.value as typeof sort)}><option value="time">{t('Time')}</option><option value="confidence">{t('Model confidence')}</option></select></div>
    </div>
    <div className="suggestion-list">
      {!list.data?<p>{t('Loading…')}</p>:!rows.length?<p>{t('No clip suggestions')}</p>:rows.map(suggestion=>{
        const state=effectiveState(suggestion,now),used=uses.get(suggestion.id)
        return <article className="suggestion-card" key={suggestion.id}>
          <strong>{suggestion.title||t('Untitled suggestion')}</strong>
          <span className="suggestion-time">{formatSessionTime(suggestion.in_ms)} — {formatSessionTime(suggestion.out_ms)} · {t('{0} seconds',{0:Number(((suggestion.out_ms-suggestion.in_ms)/1000).toFixed(1))})}</span>
          {suggestion.reason&&<p>{suggestion.reason}</p>}
          <div className="suggestion-evidence">{suggestion.confidence!==null&&<span title={t('Model estimate, not audience performance')}>{t('Confidence {0}%',{0:Math.round(suggestion.confidence*100)})}</span>}{suggestion.evidence.danmaku>0&&<span>{t('{0} danmaku messages',{0:suggestion.evidence.danmaku})}</span>}{suggestion.tags.map(tag=><span key={tag}>{tag}</span>)}</div>
          <div className="suggestion-actions">
            <button className="icon-btn" title={t('Locate on timeline')} aria-label={t('Locate on timeline')} onClick={()=>locate(suggestion,used)}><LocateFixed/></button>
            {state==='pending'?<><button className="btn primary" disabled={!editable||busy||dirty} onClick={()=>void run(async()=>{const focus=recordingImportFocus(session),result=await session.acceptSuggestion(suggestion);if(isCurrent())focus(result.items,result)})}>{busy?<LoaderCircle className="spin"/>:<Plus/>}{t('Add to timeline')}</button><button className="icon-btn" disabled={!editable||busy} title={t('Dismiss suggestion')} aria-label={t('Dismiss suggestion')} onClick={()=>openConfirm(suggestion)}><Trash2/></button></>:<span className="suggestion-state">{state==='accepted'&&<Check size={14}/>}{t(state==='accepted'?'Accepted':state==='dismissed'?'Dismissed':'Expired')}</span>}
            {used?.projectId&&used.projectId!==session.id&&<button className="link-btn" title={used.projectName??undefined} onClick={()=>void actions.openProject(`biliup:${used.projectId}`)}>{t('Open project')}</button>}
          </div>
        </article>
      })}
    </div>
    {estimate&&<SuggestionEstimate key={source} sessionId={source} status={availability.status} onClose={()=>setEstimate(false)}/>}
    {confirm&&<GlobalDialog label={t(confirm==='cancel'?'Cancel generation?':'Dismiss suggestion?')} busy={busy} onClose={()=>setConfirm(undefined)}><section className="modal suggestion-confirm"><h2>{t(confirm==='cancel'?'Cancel generation?':'Dismiss suggestion?')}</h2><p>{t(confirm==='cancel'?'Completed transcripts will be kept.':'Similar ranges will not be suggested again.')}</p>{error&&<p role="alert">{error}</p>}<footer><button className="btn" disabled={busy} onClick={()=>setConfirm(undefined)}>{t('Back')}</button><button className="btn danger" disabled={busy||!editable} onClick={()=>void run(async()=>{if(confirm==='cancel')await cancelAutoClip(source);else await dismissSuggestion(confirm)},{onSuccess:()=>setConfirm(undefined)})}>{t(confirm==='cancel'?'Cancel generation':'Dismiss suggestion')}</button></footer></section></GlobalDialog>}
  </>
}
