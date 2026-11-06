import type React from 'react'
import {actions} from '@/app/lib/editor/state/store'
import {currentEditorStore,projectOf} from '@/app/lib/editor/state/store/core'
import type {ProjectModel} from '@/app/lib/editor/state/project'
import {getPlayback,playback,type PreviewDraft} from '../../preview/playback'
import {beginEditorGesture,beginEditorViewGesture,editorGestureOrigin,ownsEditorGesture} from '../../workspace/pointerGesture'
import type {Plan} from '../edit'
import type {DragView} from './dragView'

export interface TimelineGestureContext {
 readonly project:ProjectModel
 readonly stopDrag:React.RefObject<(()=>void)|undefined>
}

/** One timeline drag owns its lane feedback, playback preview and eventual save. */
export function beginTimelineEdit(
 event:React.PointerEvent,
 ctx:TimelineGestureContext&{readonly setDrag:React.Dispatch<React.SetStateAction<DragView|undefined>>},
 handlers:{move:(event:PointerEvent)=>void;release:(event:PointerEvent)=>Plan|undefined;stop?:()=>void},
){
 ctx.stopDrag.current?.()
 const origin=editorGestureOrigin()
 if(!origin||projectOf(origin.owner.getSnapshot().snapshot!)!==ctx.project)return
 let shown:DragView|undefined,preview:PreviewDraft|undefined
 let releasePending=()=>{}
 const clear=()=>{
  releasePending();releasePending=()=>{}
  const previous=shown;shown=undefined
  if(previous)ctx.setDrag(current=>current===previous?undefined:current)
  if(preview&&getPlayback().draft===preview)playback.setDraft(undefined)
  preview=undefined
  if(ctx.stopDrag.current===cancel)ctx.stopDrag.current=undefined
 }
 const pointer=beginEditorGesture(event,{
  move:handlers.move,
  cancel:()=>{handlers.stop?.();clear()},
  commit:(from,next)=>{
   handlers.stop?.()
   const plan=handlers.release(next)
   if(!plan?.ops.length){clear();return}
   releasePending=from.owner.subscribe(()=>{if(!ownsEditorGesture(from))clear()})
   // The queued edit keeps the revision the user manipulated; a later edit must not retarget it.
   void actions.itemEdit(()=>plan,{dir:from.dir,baseRevision:from.baseRevision}).finally(clear)
  },
 })
 if(!pointer)return
 const cancel=()=>{pointer();clear()}
 ctx.stopDrag.current=cancel
 return {
  show(view:DragView,rendered:ProjectModel=view.view){
   if(!ownsEditorGesture(origin))return cancel()
   shown=view;ctx.setDrag(view)
   if(view.refused){
    if(preview&&getPlayback().draft===preview)playback.setDraft(undefined)
    preview=undefined
   }else{
    preview={base:ctx.project,project:rendered}
    playback.setDraft(preview)
   }
  },
 }
}

/** Scrubbing, selecting and panning remain available without edit permission. */
export function beginTimelineView(
 event:React.PointerEvent,
 ctx:TimelineGestureContext,
 handlers:{move:(event:PointerEvent)=>void;finish:()=>void},
){
 ctx.stopDrag.current?.()
 const owner=currentEditorStore(),snapshot=owner?.getSnapshot().snapshot
 if(!owner||!snapshot||projectOf(snapshot)!==ctx.project)return false
 const finish=()=>{
  if(ctx.stopDrag.current===cancel)ctx.stopDrag.current=undefined
  handlers.finish()
 }
 const cancel=beginEditorViewGesture(event,{move:handlers.move,commit:finish,cancel:finish})
 if(!cancel)return false
 ctx.stopDrag.current=cancel
 return true
}
