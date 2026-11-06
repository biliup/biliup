'use client'
import {useEffect,useState} from 'react'
import Link from 'next/link'
import useSWR from 'swr'
import {fetcher} from '@/app/lib/api-streamer'
import type {AutomaticProcessingPage} from '@/app/lib/outputs'
import {formatSpan} from '@/app/lib/sessions'
import {recordingEditorHref} from '@/app/lib/workbench'
import {useMe} from '@/app/lib/use-me'
import {editorText as t,useEditorLanguage} from '@/app/lib/editor/i18n'
import styles from './outputs.module.css'

const labels={running:'Processing…',ready:'Completed',failed:'Failed',interrupted:'Interrupted'} as const
const operations:Record<string,string>={remux:'Remux',rendering:'Automatic rendering'}
const pageSize=10

export function AutomaticProcessing({state,taskId}:{state:string;taskId?:string|null}){
 useEditorLanguage()
 const {can}=useMe(),[page,setPage]=useState(0)
 useEffect(()=>setPage(0),[state,taskId])
 const query=new URLSearchParams({limit:String(pageSize),offset:String(page*pageSize)})
 if(state)query.set('state',state)
 if(taskId)query.set('task',taskId)
 const {data,error,mutate,isValidating}=useSWR<AutomaticProcessingPage>(can('file.view')&&!['queued','cancelled'].includes(state)?`/v1/outputs/processing?${query}`:null,fetcher,{refreshInterval:1500})
 useEffect(()=>{if(data)setPage(value=>Math.min(value,Math.max(0,Math.ceil(data.total/pageSize)-1)))},[data?.total])
 if(['queued','cancelled'].includes(state))return null
 return <section className={styles.automatic} aria-label={t('Automatic post-processing')}>
  <h2>{t('Automatic post-processing')}</h2>
  {error&&<div className={styles.loadError}><p role="alert">{String(error)}</p><button className="btn" disabled={isValidating} onClick={()=>void mutate().catch(()=>undefined)}>{t('Retry')}</button></div>}
  {!data?!error&&<p role="status">{t('Loading…')}</p>:!data.tasks.length?<p>{t('No post-processing records')}</p>:data.tasks.map(({task_id,upload_task,title,streamer_name,session_id,step})=>{
   const active=step.state==='running',ratio=step.total_ms&&step.total_ms>0?Math.max(0,Math.min(1,step.processed_ms/step.total_ms)):undefined
   return <article className={styles.row} key={`${task_id}-${step.id}`}>
    <div className={styles.name}>
     <strong title={step.output_file??step.file}>{step.output_file??step.file}</strong>
     <span>{streamer_name??title} · {t(operations[step.operation]??step.operation)} · {new Date(step.started_at).toLocaleString()}</span>
    </div>
    <div className={styles.status}>
     <span>{t(labels[step.state])}{active&&ratio!==undefined?` ${Math.round(ratio*100)}%`:''}</span>
     {active&&<><progress aria-label={t('Progress')} max={1} value={ratio}/><span>{formatSpan(step.processed_ms)} / {step.total_ms?formatSpan(step.total_ms):'—'}</span></>}
    </div>
    <div className={styles.actions}>
     {session_id!=null&&can('clip.edit')&&<Link className="btn" href={recordingEditorHref(session_id)}>{t('Edit recording')}</Link>}
     {upload_task&&<Link className="btn" href={`/upload-queue?task=${encodeURIComponent(task_id)}`}>{t('Task details')}</Link>}
    </div>
    {step.error&&<p className={styles.error} role="alert">{step.error}</p>}
   </article>
  })}
  {data&&data.total>pageSize&&<nav className={styles.pager} aria-label={t('Post-processing pages')}>
   <button className="btn" disabled={page===0} onClick={()=>setPage(value=>value-1)}>{t('Previous')}</button>
   <span>{page+1} / {Math.max(1,Math.ceil(data.total/pageSize))}</span>
   <button className="btn" disabled={(page+1)*pageSize>=data.total} onClick={()=>setPage(value=>value+1)}>{t('Next')}</button>
  </nav>}
 </section>
}
