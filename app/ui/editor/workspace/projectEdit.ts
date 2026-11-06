import {useContext,useEffect,useRef} from 'react'
import {actions} from '@/app/lib/editor/state/store'
import {currentEditorStore,EditorStoreContext,projectOf} from '@/app/lib/editor/state/store/core'
import type {ProjectModel} from '@/app/lib/editor/state/project'
import {draftOf,type Plan} from '../timeline/edit'
import {getPlayback,playback,type PreviewDraft} from '../preview/playback'
import {ownsEditorGesture,type GestureOrigin} from './pointerGesture'

type Result=Plan|{readonly problem:string}
type Draft={preview:PreviewDraft;origin:GestureOrigin}
const drop=(draft:Draft|undefined)=>{if(draft&&getPlayback().draft===draft.preview)playback.setDraft(undefined)}

/** Inspector, timeline and mixer controls share the owning project's preview and save queue. */
export function useProjectEdit(){
 const owner=useContext(EditorStoreContext)
 const draft=useRef<Draft|undefined>(undefined)
 const cancel=()=>{drop(draft.current);draft.current=undefined}
 useEffect(()=>{
  const release=owner?.subscribe(()=>{if(draft.current&&!ownsEditorGesture(draft.current.origin))cancel()})
  return()=>{release?.();cancel()}
 },[owner])
 const state=()=>{
  const current=owner?.getSnapshot()
  return owner===currentEditorStore()&&current?.biliupCanEdit&&current.connection==='connected'?current:undefined
 }
 const commit=(make:(project:ProjectModel)=>Result,origin?:GestureOrigin):Promise<boolean>=>{
  const snapshot=state()?.snapshot
  const previous=draft.current;draft.current=undefined
  const from=origin??previous?.origin
  if(!snapshot||(from&&(from.owner!==owner||from.dir!==snapshot.dir))){drop(previous);return Promise.resolve(false)}
  return actions.itemEdit(make,{dir:snapshot.dir,...(from?{baseRevision:from.baseRevision}:{})}).finally(()=>drop(previous))
 }
 const preview=(project:ProjectModel,plan:Result)=>{
  const snapshot=state()?.snapshot
  if(!owner||!snapshot||projectOf(snapshot)!==project||'problem' in plan||!plan.ops.length)return cancel()
  const result=draftOf(project,plan.ops)
  if(!('project' in result))return cancel()
  const origin={owner,dir:snapshot.dir,baseRevision:snapshot.revision}
  draft.current={origin,preview:{base:project,project:result.project}}
  playback.setDraft(draft.current.preview)
 }
 return {commit,preview,cancel}
}
