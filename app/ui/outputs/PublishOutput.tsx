'use client'
import {useEffect,useRef,useState} from 'react'
import Link from 'next/link'
import useSWR from 'swr'
import {X} from 'lucide-react'
import {fetcher,type StudioEntity,type BiliType} from '@/app/lib/api-streamer'
import {useTypeTree} from '@/app/lib/use-streamers'
import {fetchUploadQueueTask,uploadTaskProgress,uploadStateLabel} from '@/app/lib/upload-queue'
import {previewOutput,submitOutput,captureOutputCover,uploadOutputCover,publicationLabels,type Publication,type PublishFields,type PublishPreview,type CoverSelection,type OutputCover as Cover} from '@/app/lib/output-publish'
import {type OutputEntry} from '@/app/lib/outputs'
import {GlobalDialog} from '@/app/ui/editor/controls/GlobalDialog'
import {editorText as t,useEditorLanguage} from '@/app/lib/editor/i18n'
import styles from './outputs.module.css'
import {OutputCover} from './OutputCover'

const archiveManager='https://member.bilibili.com/platform/upload-manager/article'
const localDate=(seconds:number|null)=>{if(!seconds)return '';const date=new Date(seconds*1000);return new Date(date.getTime()-date.getTimezoneOffset()*60000).toISOString().slice(0,16)}
const leafTypes=(types:BiliType[]):{id:number;name:string}[]=>types.flatMap(type=>type.children?.length?leafTypes(type.children):[{id:type.id,name:type.name}])

export function PublishOutput({entry,onClose,onSubmitted}:{entry:OutputEntry;onClose:()=>void;onSubmitted:()=>void}){
 useEditorLanguage()
 const {data:templates,error:templateError}=useSWR<StudioEntity[]>('/v1/upload/streamers',fetcher)
 const {typeTree,isError:categoryError}=useTypeTree()
 const [template,setTemplate]=useState<number>(),[fields,setFields]=useState<Partial<PublishFields>>({}),[preview,setPreview]=useState<PublishPreview>(),[reviewed,setReviewed]=useState(false)
 const [confirmed,setConfirmed]=useState(false),[busy,setBusy]=useState(false),[error,setError]=useState(''),[submitted,setSubmitted]=useState<Publication>()
 const [coverChoice,setCoverChoice]=useState<CoverSelection>({source:'template'}),[cover,setCover]=useState<Cover|null>(null)
 const [loadedCover,setLoadedCover]=useState<string>()
 const working=useRef(false),mounted=useRef(true)
 useEffect(()=>{mounted.current=true;return()=>{mounted.current=false}},[])
 const run=async(work:()=>Promise<void>)=>{if(working.current)return false;working.current=true;setBusy(true);setError('');try{await work();return mounted.current}catch(error){if(mounted.current)setError(error instanceof Error?error.message:String(error));return false}finally{working.current=false;if(mounted.current)setBusy(false)}}
 const choose=(id:number)=>void run(async()=>{
  setTemplate(id);setFields({});setPreview(undefined);setReviewed(false);setConfirmed(false);setCoverChoice({source:'template'});setCover(null)
  const next=await previewOutput(entry.kind,entry.id,id,{},{source:'template'})
  if(mounted.current){setPreview(next);setCover(next.cover);setReviewed(false);setError(next.cover_error??'')}
 })
 const refresh=()=>void run(async()=>{
  if(!template)return
  const next=await previewOutput(entry.kind,entry.id,template,fields,coverChoice)
  if(mounted.current){setPreview(next);setCover(next.cover);setReviewed(true);setConfirmed(false)}
 })
 const change=(patch:Partial<PublishFields>)=>{setFields(current=>({...current,...patch}));setReviewed(false);setConfirmed(false)}
 const chooseCover=(choice:CoverSelection)=>void run(async()=>{
  if(!template)return
  setCoverChoice(choice);setCover(null);setReviewed(false);setConfirmed(false)
  const next=await previewOutput(entry.kind,entry.id,template,fields,choice)
  if(mounted.current){setPreview(next);setCover(next.cover);setError(next.cover_error??'')}
 })
 const prepareCover=(work:()=>Promise<Cover>)=>run(async()=>{
  setReviewed(false);setConfirmed(false)
  const next=await work()
  if(mounted.current){setCover(next);setCoverChoice({source:'custom',id:next.id})}
 })
 const send=()=>void run(async()=>{
  if(!preview?.draft||!reviewed)return
  const receipt=await submitOutput(entry.kind,entry.id,preview.draft,confirmed)
  if(mounted.current){setSubmitted(receipt);onSubmitted()}
 })
 const needsConfirmation=preview?.previous&&['succeeded','unknown'].includes(preview.previous.state)
 const active=preview?.previous&&['uploading','submitting'].includes(preview.previous.state)
 const studio=preview?(reviewed?preview.studio:{...preview.studio,...fields}):undefined
 const title=t('Publish')
 return <GlobalDialog label={title} onClose={onClose} busy={busy}><section className={'modal '+styles.publishModal}>
  <header className={styles.dialogHeader}><h2>{title}</h2><button className="icon-btn" aria-label={t('Close')} disabled={busy} onClick={onClose}><X size={18}/></button></header>
  {submitted?<PublicationResult receipt={submitted}/>:<>
   <label className={styles.publishField}>{t('Upload template')}<select className="input" value={template??''} disabled={busy||!templates} onChange={event=>choose(Number(event.target.value))}><option value="" disabled>{t('Choose a template')}</option>{templates?.filter(template=>template.uploader?.toLowerCase()!=='noop').map(template=><option key={template.id} value={template.id}>{template.template_name}</option>)}</select></label>
   {templateError&&<p role="alert">{String(templateError)}</p>}
   {templates&&!templates.length&&<Link href="/upload-manager">{t('Manage upload templates')}</Link>}
   {studio&&<><fieldset className={styles.publishFields} disabled={busy||reviewed}>
    <label className={styles.publishField}>{t('Title')}<input className="input" value={studio.title} maxLength={80} onChange={event=>change({title:event.target.value})}/></label>
    <label className={styles.publishField}>{t('Description')}<textarea className="input" rows={4} value={studio.desc} maxLength={2000} onChange={event=>change({desc:event.target.value})}/></label>
    <label className={styles.publishField}>{t('Tags')}<input className="input" value={studio.tags.join(',')} onChange={event=>change({tags:event.target.value.split(/[,，]/)})}/></label>
    <div className={styles.publishColumns}>
     <label className={styles.publishField}>{t('Category')}<select className="input" value={studio.tid} onChange={event=>change({tid:Number(event.target.value),tid_v2:null})}><option value={preview?.studio.tid}>{t('Template category')} · {preview?.studio.tid}</option>{typeTree?.map((group:{value:number;label:string;children?:BiliType[]})=><optgroup label={group.label} key={group.value}>{(group.children?.length?leafTypes(group.children):[{id:group.value,name:group.label}]).map(child=><option value={child.id} key={child.id}>{child.name}</option>)}</optgroup>)}</select></label>
     <label className={styles.publishField}>{t('Copyright')}<select className="input" value={studio.copyright} onChange={event=>change({copyright:Number(event.target.value)})}><option value={1}>{t('Original')}</option><option value={2}>{t('Reprint')}</option></select></label>
    </div>
    {categoryError&&<p role="alert">{t('Categories could not be loaded. The template category is retained.')}</p>}
    {studio.copyright===2&&<label className={styles.publishField}>{t('Source')}<input className="input" value={studio.source} onChange={event=>change({source:event.target.value})}/></label>}
    <label className={styles.publishField}>{t('Scheduled publication')}<input className="input" type="datetime-local" value={localDate(studio.dtime)} onChange={event=>change({dtime:event.target.value?Math.floor(new Date(event.target.value).getTime()/1000):0})}/></label>
   </fieldset>
    <OutputCover entry={entry} cover={cover} choice={coverChoice} durationMs={preview?.duration_ms??0} disabled={busy||reviewed} onChoice={chooseCover} onUpload={file=>prepareCover(()=>uploadOutputCover(entry.kind,entry.id,file))} onCapture={at=>prepareCover(()=>captureOutputCover(entry.kind,entry.id,at))} onPreviewReady={(id,ready)=>setLoadedCover(current=>ready?id:current===id?undefined:current)}/>
    <dl className={styles.publishSummary}><dt>{t('File')}</dt><dd>{preview?.file_name}</dd>{!cover&&<><dt>{t('Cover')}</dt><dd>{t('Bilibili default cover')}</dd></>}{studio.dynamic&&<><dt>{t('Dynamic text')}</dt><dd>{studio.dynamic}</dd></>}{studio.only_self&&<><dt>{t('Visibility')}</dt><dd>{t('Only me')}</dd></>}</dl>
   </>}
   {preview?.problem&&reviewed&&<p role="alert">{preview.problem}</p>}
   {active&&<p role="status">{t('This output is already being published.')} <Link href={'/upload-queue?task='+encodeURIComponent(preview!.previous!.task_id??'')}>{t('Upload queue')}</Link></p>}
   {needsConfirmation&&<label className={styles.confirmPublication}><input type="checkbox" checked={confirmed} disabled={busy||!reviewed} onChange={event=>setConfirmed(event.target.checked)}/><span>{t(preview!.previous!.state==='unknown'?'I checked Bilibili and this output was not published.':'Publish this output again as a new submission.')} <a href={archiveManager} target="_blank" rel="noreferrer">{t('View submissions')}</a></span></label>}
   <footer className={styles.dialogFooter}><button className="btn" onClick={onClose} disabled={busy}>{t('Cancel')}</button><button className="btn" disabled={busy||!template} onClick={reviewed?()=>{setReviewed(false);setConfirmed(false)}:refresh}>{t(reviewed?'Edit':'Preview publication')}</button><button className="btn primary" disabled={busy||!reviewed||!preview?.draft||!!preview.problem||!!active||!!needsConfirmation&&!confirmed||!!preview?.cover&&loadedCover!==preview.cover.id} onClick={send}>{busy?t('Working…'):t('Submit publication')}</button></footer>
  </>}
  {error&&<p role="alert">{error}</p>}
 </section></GlobalDialog>
}
function PublicationResult({receipt}:{receipt:Publication}){
 const {data,error}=useSWR(receipt.task_id?['output-upload',receipt.task_id]:null,()=>fetchUploadQueueTask(receipt.task_id!),{refreshInterval:1500})
 return <div className={styles.publishResult}>
  <strong>{data?uploadStateLabel(data.state):t(publicationLabels[receipt.state])}</strong>
  {data&&<><p>{uploadTaskProgress(data)}</p>{data.error&&<p role="alert">{data.error}</p>}{data.bvid&&<a className="btn" href={'https://www.bilibili.com/video/'+encodeURIComponent(data.bvid)} target="_blank" rel="noreferrer">{t('View submission')}</a>}</>}
  {error&&<p role="alert">{String(error)}</p>}
  <Link className="btn" href={'/upload-queue?task='+encodeURIComponent(receipt.task_id??'')}>{t('Upload queue')}</Link>
 </div>
}
