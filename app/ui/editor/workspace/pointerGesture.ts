import {currentEditorStore} from '@/app/lib/editor/state/store/core'
import type {EditorStore} from '@/app/lib/editor/project-store'
import {isTextComposition} from '@/app/lib/editor/keyboard'

export interface GestureOrigin {
 readonly owner:EditorStore
 readonly dir:string
 readonly baseRevision:number
}

export function editorGestureOrigin():GestureOrigin|undefined {
 const owner=currentEditorStore(),state=owner?.getSnapshot(),snapshot=state?.snapshot
 return owner&&snapshot&&state?.biliupCanEdit&&state.connection==='connected'?{owner,dir:snapshot.dir,baseRevision:snapshot.revision}:undefined
}
export function ownsEditorGesture(origin:GestureOrigin){
 const now=origin.owner.getSnapshot()
 return currentEditorStore()===origin.owner&&now.connection==='connected'&&!!now.biliupCanEdit&&now.snapshot?.dir===origin.dir&&now.snapshot.revision===origin.baseRevision
}

/** A pointer edit belongs to the project and revision where it began. */
export function beginEditorGesture(
 event:{readonly pointerId:number;readonly currentTarget:Element},
 handlers:{move:(event:PointerEvent)=>void;commit:(origin:GestureOrigin,event:PointerEvent)=>void;cancel:()=>void},
):(()=>void)|undefined {
 const origin=editorGestureOrigin()
 if(!origin)return
 return beginPointerGesture(event,{...handlers,commit:next=>handlers.commit(origin,next)},()=>ownsEditorGesture(origin),origin.owner.subscribe)
}

/** Viewing and selecting keep project ownership without requiring edit permission. */
export function beginEditorViewGesture(
 event:{readonly pointerId:number;readonly currentTarget:Element},
 handlers:{move:(event:PointerEvent)=>void;commit:(event:PointerEvent)=>void;cancel:()=>void},
):(()=>void)|undefined {
 const owner=currentEditorStore(),snapshot=owner?.getSnapshot().snapshot
 if(!owner||!snapshot)return
 const valid=()=>{
  const now=owner.getSnapshot().snapshot
  return currentEditorStore()===owner&&now?.dir===snapshot.dir&&now.revision===snapshot.revision
 }
 return beginPointerGesture(event,handlers,valid,owner.subscribe)
}

/** Pointer lifetime shared by saved project controls and unsaved form controls. */
export function beginPointerGesture(
 event:{readonly pointerId:number;readonly currentTarget:Element},
 handlers:{move:(event:PointerEvent)=>void;commit:(event:PointerEvent)=>void;cancel:()=>void},
 available:()=>boolean,
 subscribe?:(listener:()=>void)=>()=>void,
):()=>void {
 const element=event.currentTarget,pointer=event.pointerId
 let finished=false
 let unsubscribe=()=>{}
 const valid=()=>element.isConnected&&available()
 const finish=(event?:PointerEvent)=>{
  if(finished)return
  const accepted=!!event&&valid();finished=true
  unsubscribe()
  window.removeEventListener('pointermove',move)
  window.removeEventListener('pointerup',up)
  window.removeEventListener('pointercancel',pointerCancel)
  window.removeEventListener('blur',cancel)
  window.removeEventListener('keydown',key,true)
  document.removeEventListener('visibilitychange',visibility)
  if(accepted&&event)handlers.commit(event);else handlers.cancel()
 }
 const cancel=()=>finish()
 const move=(next:PointerEvent)=>{
  if(next.pointerId!==pointer)return
  if(!valid())return cancel()
  handlers.move(next)
 }
 const up=(next:PointerEvent)=>{if(next.pointerId===pointer)finish(next)}
 const pointerCancel=(next:PointerEvent)=>{if(next.pointerId===pointer)cancel()}
 const visibility=()=>{if(document.hidden)cancel()}
 const key=(next:KeyboardEvent)=>{
  if(isTextComposition(next)||next.key!=='Escape')return
  next.preventDefault();next.stopImmediatePropagation();cancel()
 }
 unsubscribe=subscribe?.(()=>{if(!valid())cancel()})??(()=>{})
 window.addEventListener('pointermove',move)
 window.addEventListener('pointerup',up)
 window.addEventListener('pointercancel',pointerCancel)
 window.addEventListener('blur',cancel)
 window.addEventListener('keydown',key,true)
 document.addEventListener('visibilitychange',visibility)
 return cancel
}
