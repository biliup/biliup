import {useSyncExternalStore} from 'react'
import type {MultiLayout,TakeKind} from '@/app/lib/editor/contracts'
import type {CaptureProgress} from '@/app/lib/editor/capture-upload'
import {actions,getState,projectOf} from '@/app/lib/editor/state/store'
import {API_BASE} from '@/app/lib/api-streamer'
import {editorText} from '@/app/lib/editor/i18n'
import {captureEditorFocus} from '../preview/focus'
import {playback} from '../preview/playback'
import {layoutOf} from '@/app/lib/editor/state/project'
import {layout} from '@/app/lib/editor/core/timeline/timeline'
import {revealItem} from '@/app/lib/editor/state/store/edits'

export type RecordingDraft={id:string;dir:string;server:string;createdAt:number;name?:string;placeAt?:number;fps:number;layout?:MultiLayout;multi:boolean;parts:readonly {blob:Blob;kind:TakeKind;device?:string;offsetMs:number;recordedMs:number}[]}
type Recovery={draft:RecordingDraft;busy:boolean;durable:boolean;problem?:string;progress?:CaptureProgress}
let entries:readonly Recovery[]=[],loaded:Promise<void>|undefined
const listeners=new Set<()=>void>()
const notify=()=>{for(const listener of listeners)listener()}
const replace=(id:string,patch:Partial<Recovery>)=>{entries=entries.map(entry=>entry.draft.id===id?{...entry,...patch}:entry);notify()}
const subscribe=(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener)}}
export const useCaptureRecovery=()=>useSyncExternalStore(subscribe,()=>entries,()=>entries)
const server=()=>new URL(API_BASE||'/',window.location.href).href.replace(/\/$/,'')

async function database(){
  return new Promise<IDBDatabase>((resolve,reject)=>{
    const request=indexedDB.open('biliup-editor-captures',1)
    request.onupgradeneeded=()=>request.result.createObjectStore('drafts',{keyPath:'id'})
    request.onsuccess=()=>resolve(request.result)
    request.onerror=()=>reject(request.error)
    request.onblocked=()=>reject(new Error('录制草稿数据库被其他页面占用'))
  })
}
async function write(id:string,draft?:RecordingDraft){
  const db=await database()
  try{await new Promise<void>((resolve,reject)=>{
    const tx=db.transaction('drafts','readwrite'),store=tx.objectStore('drafts')
    if(draft)store.put(draft);else store.delete(id)
    tx.oncomplete=()=>resolve();tx.onerror=()=>reject(tx.error);tx.onabort=()=>reject(tx.error)
  })}finally{db.close()}
}
export function loadCaptureRecovery(){
  return loaded??= (async()=>{
    try{
      const db=await database()
      const drafts=await new Promise<RecordingDraft[]>((resolve,reject)=>{
        const tx=db.transaction('drafts'),request=tx.objectStore('drafts').getAll()
        request.onsuccess=()=>resolve(request.result);request.onerror=()=>reject(request.error)
      }).finally(()=>db.close())
      const known=new Set(entries.map(entry=>entry.draft.id))
      entries=[...entries,...drafts.filter(draft=>!known.has(draft.id)&&draft.server===server()&&draft.parts.every(part=>part.blob instanceof Blob)).map(draft=>({draft,busy:false,durable:true}))].sort((a,b)=>a.draft.createdAt-b.draft.createdAt);notify()
    }catch{/* A denied/quota-limited database still allows recovery in this page. */}
  })()
}
export async function discardCapture(id:string){
  const entry=entries.find(entry=>entry.draft.id===id)
  if(!entry||entry.busy)return
  replace(id,{busy:true,problem:undefined,progress:undefined})
  try{if(entry.durable)await write(id);entries=entries.filter(entry=>entry.draft.id!==id);notify()}
  catch(error){replace(id,{busy:false,problem:error instanceof Error?error.message:String(error)})}
}
export function downloadCapture(id:string,index:number){
  const part=entries.find(entry=>entry.draft.id===id)?.draft.parts[index]
  if(!part)return
  const url=URL.createObjectURL(part.blob),anchor=document.createElement('a')
  anchor.href=url;anchor.download=`recording-${id}-${part.kind}.${part.blob.type.includes('mp4')?'mp4':'webm'}`;anchor.click();setTimeout(()=>URL.revokeObjectURL(url),60000)
}
export async function retryCapture(id:string,unchanged=captureEditorFocus()){
  const entry=entries.find(entry=>entry.draft.id===id)
  if(!entry||entry.busy)return {problem:editorText('Recording is already being saved.')}
  const {draft}=entry
  if(getState().snapshot?.dir!==draft.dir)return {problem:editorText('Open the original project to save this recording.')}
  replace(id,{busy:true,problem:undefined,progress:{phase:'uploading',loaded:0,total:draft.parts.reduce((total,part)=>total+part.blob.size,0)}})
  try{
    // A crash/reload before the success reply may leave a draft for an already committed take.
    const project=projectOf(getState().snapshot!)
    const existing=project.media.filter(media=>media.source?.takeId===id)
    if(existing.length&&draft.parts.every(part=>existing.some(media=>media.source?.kind===part.kind))){await discardSaved(id);return {saved:{name:draft.name??existing[0]!.name,seconds:Math.min(...existing.map(media=>(media.probe?.frames??0)/project.canvas.fps))}}}
    const common={dir:draft.dir,name:draft.name,recoveryId:id,placeAt:draft.placeAt,placeFps:draft.fps,onProgress:(progress:CaptureProgress)=>replace(id,{progress})}
    const first=draft.parts[0]!
    const result=draft.multi?await actions.saveMultiTake({...common,layout:draft.layout,parts:draft.parts}):await actions.saveTake(first.blob,{...common,kind:first.kind,device:first.device})
    if('problem'in result){replace(id,{busy:false,problem:result.problem,progress:undefined});return result}
    await discardSaved(id)
    if(unchanged()){
      const snapshot=getState().snapshot
      const ids='items'in result.saved?result.saved.items.map(item=>item.itemId):result.saved.itemId?[result.saved.itemId]:[]
      if(snapshot&&ids.length){
        const current=projectOf(snapshot),positions=layout(current).starts
        if(ids.every(id=>positions.has(id))){
          const timing=layoutOf(current)
          playback.setProject(timing.frames,current.canvas.fps,timing.end)
          playback.hoverMedia(undefined);playback.selectMany(ids)
          playback.seek(Math.min(...ids.map(id=>positions.get(id)!)))
          revealItem(ids[0]!)
        }
      }
    }
    return result
  }catch(error){const problem=error instanceof Error?error.message:String(error);replace(id,{busy:false,problem,progress:undefined});return {problem}}
}
async function discardSaved(id:string){
  try{await write(id)}catch{/* Stable take IDs make a leftover draft safe to retry after reload. */}
  entries=entries.filter(entry=>entry.draft.id!==id);notify()
}
export async function retainCaptureDraft(input:Omit<RecordingDraft,'id'|'server'|'createdAt'>,id=crypto.randomUUID(),busy=false){
  await loadCaptureRecovery()
  const draft:RecordingDraft={...input,id,server:server(),createdAt:Date.now()}
  let durable=true
  try{await write(draft.id,draft)}catch{durable=false}
  entries=[...entries.filter(entry=>entry.draft.id!==id),{draft,busy,durable}];notify()
  return draft.id
}
export async function releaseCaptureDraft(id:string){await discardSaved(id)}
export function failCaptureDraft(id:string,problem:string){replace(id,{busy:false,problem})}
export async function saveCaptureDraft(input:Omit<RecordingDraft,'id'|'server'|'createdAt'>){
  const unchanged=captureEditorFocus()
  const id=await retainCaptureDraft(input)
  return retryCapture(id,unchanged)
}
