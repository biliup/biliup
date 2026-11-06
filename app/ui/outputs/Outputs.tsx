'use client'
import {useEffect,useRef,useState} from 'react'
import {useSearchParams} from 'next/navigation'
import Link from 'next/link'
import useSWR from 'swr'
import {X} from 'lucide-react'
import {apiFetch,fetcher,handleResponse} from '@/app/lib/api-streamer'
import {apiResourceUrl} from '@/app/lib/renders'
import {clipUrl,downloadClip,type Clip} from '@/app/lib/clips'
import {displayOutput as display,outputLabels as labels,outputClip,outputHref,outputEditorHref,type OutputEntry as Entry} from '@/app/lib/outputs'
import {PublishDrawer,type PublishTarget} from '@/app/ui/publish/PublishDrawer'
import {PublishOutput} from './PublishOutput'
import {publishable,publicationLabels} from '@/app/lib/output-publish'
import {formatSpan} from '@/app/lib/sessions'
import {AutomaticProcessing} from './AutomaticProcessing'
import {useMe} from '@/app/lib/use-me'
import {editorText as t,useEditorLanguage} from '@/app/lib/editor/i18n'
import {GlobalDialog} from '@/app/ui/editor/controls/GlobalDialog'
import workspace from '@/app/ui/editor/workspace.module.css'
import styles from './outputs.module.css'

export default function Outputs(){
 useEditorLanguage();const {can,isLoading:permissionLoading,error:permissionError}=useMe(),[state,setState]=useState(''),[page,setPage]=useState(0),[playing,setPlaying]=useState<Entry>(),[error,setError]=useState(''),[busy,setBusy]=useState<string>()
 const [publishing,setPublishing]=useState<PublishTarget|null>(null),working=useRef(false),mounted=useRef(true)
 const [publishingOutput,setPublishingOutput]=useState<Entry>()
 useEffect(()=>{mounted.current=true;return()=>{mounted.current=false}},[])
 const params=useSearchParams(),focus=/^(editor|render|clip)-([1-9]\d*)$/.exec(params.get('output')??'')
 const view=JSON.stringify([params.toString(),state,page]),currentView=useRef(view)
 currentView.current=view
 const processingTask=focus?null:params.get('task')
 const showExports=!processingTask&&(!!focus||state!=='interrupted')
 const filter=state,base='/outputs'
 const {data,error:loadError,mutate,isValidating}=useSWR<{jobs:Entry[];total:number}>(can('file.view')&&showExports?`/v1/outputs?limit=25&offset=${focus?0:page*25}${!focus&&filter?`&state=${filter}`:''}${focus?`&kind=${focus[1]}&id=${focus[2]}`:''}`:null,fetcher,{refreshInterval:1500})
 useEffect(()=>{if(data&&!focus)setPage(value=>Math.min(value,Math.max(0,Math.ceil(data.total/25)-1)))},[data?.total,focus?.[0]])
 const run=async(key:string,work:(current:()=>boolean)=>Promise<void>)=>{if(working.current)return;const current=()=>mounted.current&&currentView.current===view;working.current=true;setBusy(key);setError('');try{await work(current)}catch(error){if(current())setError(error instanceof Error?error.message:String(error))}finally{working.current=false;if(mounted.current)setBusy(undefined)}}
 const action=(entry:Entry,url:string)=>run(url,async(current)=>{
  const response=await apiFetch(url,{method:'POST',...(entry.kind==='clip'?{headers:{'Content-Type':'application/json'},body:JSON.stringify({mode:entry.job.mode??'quick'})}:{})})
  await handleResponse(response)
  const body=await response.text(),result=body?JSON.parse(body):null
  void mutate().catch(()=>undefined)
  if(!current())return
  const retry=/^editor-export-([1-9]\d*)$/.exec(result?.jobId??'')
  if(retry){setPage(0);setState('');window.history.pushState(null,'',outputHref('editor',Number(retry[1])))}
  else if(entry.kind==='render'&&url.endsWith('/retry')&&Number.isSafeInteger(result?.id)){setPage(0);setState('');window.history.pushState(null,'',outputHref('render',result.id))}
 })
 const publish=(expected:Clip)=>run(`publish:${expected.id}`,async(current)=>{
  const clip=await fetcher(clipUrl(expected.id)) as Clip
  if(clip.updated_at!==expected.updated_at||clip.file_name!==expected.file_name){await mutate();throw new Error(t('This output changed. Review it before publishing.'))}
  if(current())setPublishing({clips:[clip],batch:false})
 })
 if(permissionLoading||permissionError||!can('file.view'))return <div className={`${workspace.workspace} ${styles.root}`}><main className={styles.content}>
  <header className={styles.header}><h1>{t('Processing tasks')}</h1></header>
  <p role={permissionError?'alert':'status'}>{permissionLoading?t('Loading…'):permissionError?String(permissionError):t('You do not have permission to view outputs.')}</p>
 </main></div>
 return <div className={`${workspace.workspace} ${styles.root}`}><main className={styles.content}>
  <header className={styles.header}><h1>{t('Processing tasks')}</h1>{focus||processingTask?<Link className="btn" href={base} onClick={()=>{setState('');setPage(0)}}>{t('Show all')}</Link>:<select className="input" aria-label={t('Status')} value={state} onChange={e=>{setState(e.target.value);setPage(0)}}><option value="">{t('All')}</option>{Object.entries({...labels,running:'Processing…',interrupted:'Interrupted'}).map(([value,label])=><option key={value} value={value}>{t(label)}</option>)}</select>}</header>
  {!focus&&<AutomaticProcessing state={processingTask?'':state} taskId={processingTask}/>}
  {error&&<p className={styles.error} role="alert">{error}</p>}
  {loadError&&<div className={styles.loadError}><p role="alert">{String(loadError)}</p><button className="btn" disabled={isValidating} onClick={()=>void mutate().catch(()=>undefined)}>{t('Retry')}</button></div>}
  {showExports&&!focus&&<h2 className={styles.sectionTitle}>{t('Video exports')}</h2>}
  {showExports&&!data&&!loadError&&<p role="status">{t('Loading…')}</p>}
  {showExports&&data&&!data.jobs.length&&<p className={styles.empty}>{t(focus?'This output is no longer available.':state?'No matching video exports':'No video exports yet')}</p>}
  {showExports&&data?.jobs.map(entry=>{const item=display(entry),active=item.state==='queued'||item.state==='running',clip=outputClip(entry),source=can('clip.edit')?outputEditorHref(entry):undefined;return <article className={styles.row} key={`${entry.kind}-${entry.id}`}>
   <div className={styles.name}><strong title={item.title}>{item.title}</strong><span>{source?<Link className={styles.source} href={source} title={t('Continue editing')}>{item.source||t('Recording')}</Link>:item.source||t('Recording')} · {new Date(entry.created_at).toLocaleString()}</span></div>
   <div className={styles.status}><span>{active&&item.stage&&!['queued','running','preparing'].includes(item.stage)?t(item.stage):t(labels[item.state])}{active&&item.progress!=null?` ${Math.round(item.progress*100)}%`:''}</span>{active&&<><progress aria-label={t('Progress')} max={1} value={item.progress??undefined}/>{item.processed!=null&&!!item.total&&<span>{formatSpan(item.processed)} / {formatSpan(item.total)}</span>}{item.eta!=null&&<span>{t('Remaining')} {formatSpan(item.eta)}</span>}</>}</div>
   <div className={styles.actions}>{item.url&&<><button className="btn" onClick={()=>setPlaying(entry)}>{t('Play')}</button>{entry.kind==='clip'?<button className="btn" disabled={!!busy} onClick={()=>void run(`download:${entry.id}`,()=>downloadClip(entry.job,'source'))}>{t('Download')}</button>:<a className="btn" download href={apiResourceUrl(item.url)+'?attachment=true'}>{t('Download')}</a>}{entry.kind==='editor'&&entry.job.subtitlesFile&&<a className="btn" download href={apiResourceUrl(item.url)+'?attachment=true&subtitles=true'}>{t('Download subtitles')}</a>}</>}
    {clip&&item.state==='ready'&&can('upload.submit')&&<button className="btn" disabled={!!busy} onClick={()=>void publish(clip)}>{t(clip.archive_bvid?'Publish again':'Publish')}</button>}
    {publishable(entry)&&item.state==='ready'&&can('upload.submit')&&<button className="btn" disabled={!!busy||!!entry.publication&&['uploading','submitting'].includes(entry.publication.state)} onClick={()=>setPublishingOutput(entry)}>{t(entry.publication?.state==='succeeded'?'Publish again':'Publish')}</button>}
    {entry.publication?.task_id&&<Link className="btn" href={'/upload-queue?task='+encodeURIComponent(entry.publication.task_id)}>{t(publicationLabels[entry.publication.state])}</Link>}
    {entry.publication?.bvid&&<a className="btn" target="_blank" rel="noreferrer" href={'https://www.bilibili.com/video/'+encodeURIComponent(entry.publication.bvid)}>{t('View submission')}</a>}
    {clip?.archive_bvid&&<a className="btn" target="_blank" rel="noreferrer" href={`https://www.bilibili.com/video/${encodeURIComponent(clip.archive_bvid)}`}>{t('View submission')}</a>}
    {active&&item.cancel&&can('clip.edit')&&<button className="btn" disabled={!!busy} onClick={()=>void action(entry,item.cancel!)}>{t('Cancel')}</button>}
    {entry.retry_id?<Link className="btn" href={outputHref(entry.kind,entry.retry_id)}>{t('View retry')}</Link>:item.retry&&['failed','cancelled'].includes(item.state)&&can('clip.edit')&&<button className="btn" disabled={!!busy} title={t('Retry the saved export version and settings')} onClick={()=>void action(entry,item.retry!)}>{t('Retry')}</button>}
   </div>
   {item.error&&<p className={styles.error} role="alert">{item.error}</p>}
   {!!item.warnings?.length&&item.state!=='failed'&&<details className={styles.warnings}><summary>{t('Warnings')} ({item.warnings.length})</summary><ul>{item.warnings.map((warning,index)=><li key={index}>{warning}</li>)}</ul></details>}
   {entry.publication?.error&&<p className={styles.error} role="alert">{entry.publication.error}</p>}
  </article>})}
  {showExports&&!focus&&!!data?.total&&<footer className={styles.pager}><span>{page+1} / {Math.max(1,Math.ceil(data.total/25))}</span><button className="btn" disabled={!page} onClick={()=>setPage(page-1)}>{t('Previous')}</button><button className="btn" disabled={(page+1)*25>=data.total} onClick={()=>setPage(page+1)}>{t('Next')}</button></footer>}
  {playing&&<OutputPreview key={`${playing.kind}-${playing.id}`} entry={playing} onClose={()=>setPlaying(undefined)}/>}
  <PublishDrawer target={publishing} onClose={()=>{setPublishing(null);void mutate()}}/>
  {publishingOutput&&<PublishOutput entry={publishingOutput} onClose={()=>{setPublishingOutput(undefined);void mutate()}} onSubmitted={()=>void mutate()}/>}
 </main></div>
}

function OutputPreview({entry,onClose}:{entry:Entry;onClose:()=>void}){
 const item=display(entry),[loading,setLoading]=useState(true),[failed,setFailed]=useState(false)
 return <GlobalDialog label={t('Export preview')} onClose={onClose}><section className="modal workspace-output" aria-label={t('Export preview')}>
  <header><strong>{item.title}</strong><button className="icon-btn" aria-label={t('Close')} onClick={onClose}><X/></button></header>
  {failed?<p role="alert">{t('The output could not be played.')}</p>:loading&&<p role="status">{t('Preparing playback…')}</p>}
  {item.url&&<video controls autoPlay src={apiResourceUrl(item.url)} onCanPlay={()=>setLoading(false)} onError={()=>{setLoading(false);setFailed(true)}}/>}
 </section></GlobalDialog>
}

