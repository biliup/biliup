import type {RenderJob} from './renders'
import type {Clip} from './clips'
import type {ExportJob} from './editor/contracts'
import type {Publication} from './output-publish'
import type {ProcessingProgress} from './upload-queue'
import {preparingStatus} from './editor/export/jobs'
import {projectEditorHref,recordingEditorHref,savedSelectionEditorHref} from './workbench'

export interface AutomaticProcessingEntry {
  task_id:string
  upload_task:boolean
  title:string
  streamer_name:string|null
  session_id:number|null
  step:ProcessingProgress
}
export interface AutomaticProcessingPage {tasks:AutomaticProcessingEntry[];total:number}

type OutputBase={id:number;created_at:number;project_id:number|null;project_name:string|null;project_exists:boolean;retry_id:number|null;publication:Publication|null}
export type OutputEntry=OutputBase&({kind:'editor';job:ExportJob}|{kind:'render';job:RenderJob;clip:Clip|null}|{kind:'clip';job:Clip})
export const outputLabels={queued:'Queued',running:'Exporting…',ready:'Completed',failed:'Failed',cancelled:'Cancelled'} as const
type State=keyof typeof outputLabels
export interface OutputDisplay {
  title:string;source?:string|null;state:State;progress?:number|null;error?:string|null;url?:string|null
  processed?:number|null;total?:number|null;eta?:number|null;stage?:string|null;cancel?:string;retry?:string
  warnings?:ReadonlyArray<string>
}
export const outputHref=(kind:OutputEntry['kind'],id:number)=>`/outputs?output=${kind}-${id}`
export const outputClip=(entry:OutputEntry)=>entry.kind==='clip'?entry.job:entry.kind==='render'?entry.clip:null
/** Continue from the original registered source, without importing an already encoded output. */
export function outputEditorHref(entry:OutputEntry):string|undefined{
  if(entry.kind==='editor')return entry.project_exists&&entry.project_id!=null?projectEditorHref(entry.project_id):undefined
  if(entry.kind==='clip')return savedSelectionEditorHref(entry.id)
  return entry.job.clip_id!=null?savedSelectionEditorHref(entry.job.clip_id):recordingEditorHref(entry.job.view_session_id??entry.job.session_id)
}
export function displayOutput(entry:OutputEntry):OutputDisplay{
  if(entry.kind==='render')return {title:entry.job.file_name??entry.job.session_title??`#${entry.id}`,source:entry.job.session_title||entry.job.session_name,state:entry.job.state,progress:entry.job.ratio,error:entry.job.error,url:entry.job.download_url,processed:entry.job.processed_ms,total:entry.job.total_ms,eta:entry.job.eta_ms,stage:entry.job.phase,cancel:`/v1/renders/${entry.id}/cancel`,retry:`/v1/renders/${entry.id}/retry`,warnings:entry.job.fallback_reason?[entry.job.fallback_reason]:undefined}
  if(entry.kind==='clip'){
    const job=entry.job,ready=job.state==='ready'||job.state==='published'
    return {title:job.title||job.file_name||`#${entry.id}`,source:entry.project_name,state:ready?'ready':job.state==='exporting'?'running':'failed',progress:job.progress?.ratio,error:job.error,
      url:ready?`/v1/clips/${entry.id}/download?format=mp4&inline=true`:null,total:job.duration_ms,stage:job.progress?.phase,
      retry:job.state==='failed'?`/v1/clips/${entry.id}/export`:undefined}
  }
  const job=entry.job,state=job.phase==='done'?'ready':['preparing','rendering'].includes(job.phase)?'running':job.phase as State
  const preparing=preparingStatus(job)
  return {title:job.output,source:entry.project_name,state,progress:job.phase==='preparing'?preparing.fraction:job.progress,error:job.error,warnings:job.warnings,
    url:job.phase==='done'?`/v1/editor/projects/${entry.project_id}/exports/${entry.id}/download`:null,processed:job.phase==='rendering'?job.progress*job.seconds*1000:null,total:job.seconds*1000,
    eta:job.phase==='rendering'&&job.etaSeconds!=null?job.etaSeconds*1000:null,stage:job.phase==='preparing'?preparing.text:null,cancel:`/v1/editor/projects/${entry.project_id}/exports/${entry.id}/cancel`,
    retry:entry.project_exists?`/v1/editor/projects/${entry.project_id}/exports/${entry.id}/retry`:undefined}
}
