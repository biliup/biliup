// Adapted from Uncut's timeline drag handling; one drag belongs to one editor revision.
import {useSyncExternalStore} from 'react'
import {currentEditorStore,projectOf} from '@/app/lib/editor/state/store/core'
import type {EditorStore} from '@/app/lib/editor/project-store'
import {editorRoot} from '@/app/lib/editor/environment'

export type EditorDrag =
 | {kind:'media';id:string;path:string}
 | {kind:'component';id:string;props?:Record<string,unknown>}
 | {kind:'text';id:string}
 | {kind:'effect';id:string}
 | {kind:'transition';id:string}
const dragTypes={media:'application/x-biliup-editor-media',component:'application/x-biliup-editor-component',text:'application/x-biliup-editor-text',effect:'application/x-biliup-editor-effect'} as const
export const CUT_DROP_PX=40
export const hasFiles=(types:ReadonlyArray<string>)=>types.includes('Files')
export const hasEditorDrag=(types:ReadonlyArray<string>)=>Object.values(dragTypes).some(type=>types.includes(type))
type Drag={owner:EditorStore;root:HTMLElement;dir:string;revision:number;token:string;value:EditorDrag;release?:()=>void}
let drag:Drag|undefined
let tilePointer:{readonly x:number;readonly y:number}|undefined
const listeners=new Set<()=>void>()
export const subscribeEditorDrag=(listener:()=>void)=>{listeners.add(listener);return ()=>{listeners.delete(listener)}}
const emit=()=>{for(const listener of listeners)listener()}
function valid(active:Drag){
 const state=active.owner.getSnapshot(),snapshot=state.snapshot
 if(currentEditorStore()!==active.owner||!active.root.isConnected||!state.biliupCanEdit||state.connection!=='connected'||snapshot?.dir!==active.dir||snapshot.revision!==active.revision)return false
 const value=active.value
 return value.kind!=='media'||projectOf(snapshot).media.some(media=>media.id===value.id&&media.path===value.path)
}
export function endEditorDrag(){
 const previous=drag;drag=undefined;tilePointer=undefined;previous?.release?.();emit()
}
export function cancelEditorDragFor(owner:EditorStore){if(drag?.owner===owner)endEditorDrag()}
export function beginEditorDrag(value:EditorDrag,data?:DataTransfer):(()=>void)|undefined{
 const owner=currentEditorStore(),state=owner?.getSnapshot(),snapshot=state?.snapshot
 if(!owner||!snapshot||!state?.biliupCanEdit||state.connection!=='connected')return
 endEditorDrag()
 const active:Drag={owner,root:editorRoot(),dir:snapshot.dir,revision:snapshot.revision,token:crypto.randomUUID(),value:structuredClone(value)}
 drag=active
 const cancel=()=>{if(drag===active)endEditorDrag()}
 const unsubscribe=owner.subscribe(()=>{if(!valid(active))cancel()})
 const hidden=()=>{if(document.hidden)cancel()}
 window.addEventListener('blur',cancel)
 window.addEventListener('dragend',cancel)
 document.addEventListener('visibilitychange',hidden)
 active.release=()=>{
  unsubscribe()
  window.removeEventListener('blur',cancel)
  window.removeEventListener('dragend',cancel)
  document.removeEventListener('visibilitychange',hidden)
 }
 if(data&&value.kind!=='transition'){data.setData(dragTypes[value.kind],active.token);data.effectAllowed='copy'}
 emit()
 return cancel
}
export const activeEditorDrag=()=>drag&&valid(drag)?drag.value:undefined
/** During dragover browsers expose only types. The payload still comes from this editor. */
export function overEditorDrag(data:Pick<DataTransfer,'types'>){
 const value=activeEditorDrag()
 return value&&value.kind!=='transition'&&[...data.types].includes(dragTypes[value.kind])?value:undefined
}
export function readEditorDrag(data:DataTransfer){
 const value=overEditorDrag(data)
 return value&&drag&&data.getData(dragTypes[value.kind])===drag.token?value:undefined
}
export const useDraggedTransition=()=>useSyncExternalStore(subscribeEditorDrag,()=>{const value=activeEditorDrag();return value?.kind==='transition'?value.id:undefined},()=>undefined)
export function setTilePointer(at:typeof tilePointer){tilePointer=activeEditorDrag()?.kind==='transition'?at:undefined;emit()}
export const useTilePointer=()=>useSyncExternalStore(subscribeEditorDrag,()=>activeEditorDrag()?.kind==='transition'?tilePointer:undefined,()=>undefined)
let transitionDrop:{owner:EditorStore|undefined;handler:(module:string,x:number,y:number)=>boolean}|undefined
export function onTransitionDrop(handler:(module:string,x:number,y:number)=>boolean){
 const target={owner:currentEditorStore(),handler};transitionDrop=target
 return ()=>{if(transitionDrop===target)transitionDrop=undefined}
}
export function dropTransition(module:string,x:number,y:number){
 const active=activeEditorDrag()
 return active?.kind==='transition'&&active.id===module&&transitionDrop?.owner===currentEditorStore()?transitionDrop?.handler(module,x,y)??false:false
}
