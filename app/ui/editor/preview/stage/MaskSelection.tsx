'use client'
import {useEffect,useRef,type CSSProperties,type PointerEvent as ReactPointerEvent} from 'react'
import type {Project} from '@/app/lib/editor/core/timeline/types'
import {recordingEffect,RECORDING_MASK,planMaskRect,planRecordingEffectUpdate,type EditorMaskRegion} from '@/app/lib/editor/recording-effects'
import {maskRegions} from '@/app/lib/editor/core/timeline/masks'
import {moveRegion,resizeRegion,regionActive,type RenderRect,type ResizeHandle} from '@/app/lib/render-geometry'
import {actions,isTextEntry,useApp} from '@/app/lib/editor/state/store'
import {currentEditorStore,projectOf} from '@/app/lib/editor/state/store/core'
import {editorDialogOpen,editorEvents,editorRoot} from '@/app/lib/editor/environment'
import {isTextComposition} from '@/app/lib/editor/keyboard'
import {editorText as t,useEditorLanguage} from '@/app/lib/editor/i18n'
import {findItem,lockOf} from '../../timeline/edit'
import {clipState} from '../../timeline/clip'
import {getPlayback,playback,usePlayback} from '../playback'
import {beginEditorGesture,editorGestureOrigin,type GestureOrigin} from '../../workspace/pointerGesture'
import {useProjectEdit} from '../../workspace/projectEdit'

const HANDLES:ReadonlyArray<readonly [ResizeHandle,number,number]>=[['nw',0,0],['n',.5,0],['ne',1,0],['e',1,.5],['se',1,1],['s',.5,1],['sw',0,1],['w',0,.5]]
type Gesture={
 source:GestureOrigin;base:Project;item:string;region:EditorMaskRegion;kind:'move'|'draw'|ResizeHandle
 pointer:number;element:HTMLElement;px:number;py:number;bounds:DOMRect;origin:{x:number;y:number}
 context:{itemId:string;regionId:string;drawing:boolean};next?:RenderRect;moved:boolean;pending?:boolean
}
const release=(gesture:Gesture)=>{if(gesture.element.hasPointerCapture(gesture.pointer))gesture.element.releasePointerCapture(gesture.pointer)}
const clamp=(n:number)=>Math.max(0,Math.min(1,n))

/** Uses the same fitted content box as the compositor; coordinates remain source-normalized. */
export function MaskSelection({project,dir,disabled=false}:{project:Project;dir:string;disabled?:boolean}){
 useEditorLanguage()
 const app=useApp(),selected=usePlayback(s=>s.selected),frame=usePlayback(s=>s.hover??s.frame),draft=usePlayback(s=>s.draft)
 const playing=usePlayback(s=>s.playing),skimmingMedia=usePlayback(s=>!!s.hoverMedia)
 const picture=useRef<HTMLDivElement>(null),gesture=useRef<Gesture|undefined>(undefined)
 const stop=useRef<(()=>void)|undefined>(undefined),editor=useProjectEdit()
 const shown=draft?.base===project?draft.project:project
 const placed=findItem(shown,selected),boxState=placed?clipState(shown,placed.item.id,frame):undefined
 const edit=app.maskEditing?.itemId===selected?app.maskEditing:undefined
 const active=placed?.item.effects?.some(effect=>effect.module===RECORDING_MASK&&!effect.bypass)
 const locked=disabled||!app.biliupCanEdit||app.connection!=='connected'||!placed||!!lockOf(placed)||!!placed.track.hidden
 const visible=!locked&&!skimmingMedia&&!!active&&!!boxState?.inside&&placed?.item.type==='clip'

 const cancel=()=>stop.current?.()
 const clear=(current:Gesture)=>{
  if(gesture.current!==current)return
  gesture.current=undefined;stop.current=undefined;release(current);editor.cancel()
  if(current.kind==='draw'&&!current.pending&&current.source.owner===currentEditorStore()&&current.source.owner.getSnapshot().maskEditing===current.context)
   current.source.owner.set({maskEditing:{itemId:current.item,drawing:true}})
 }
 useEffect(()=>{cancel();return cancel},[project,dir,selected,frame,locked,visible])
 useEffect(()=>{
  const owner=currentEditorStore(),root=editorRoot()
  const outside=(event:PointerEvent)=>{
   if(!(event.target instanceof Element)||event.target.closest('.pv-mask-overlay, .mask-editor'))return
   if(owner?.getSnapshot().maskEditing)owner.set({maskEditing:undefined})
  }
  const key=(event:KeyboardEvent)=>{
   if(event.defaultPrevented||isTextComposition(event)||isTextEntry(event.target)||editorDialogOpen()
     ||(event.target instanceof Element&&event.target.closest('[data-testid=media-tile], [data-testid=generated-item]'))
     ||(event.target instanceof Element&&event.target.closest('[role=menu], [role=listbox]')))return
   const editing=owner?.getSnapshot().maskEditing
   if(!editing||editing.itemId!==getPlayback().selected||owner!==currentEditorStore())return
   if(event.key==='Escape'){
    event.preventDefault();event.stopImmediatePropagation();cancel();owner.set({maskEditing:undefined});return
   }
   const arrow=['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(event.key)
   if((!['Delete','Backspace'].includes(event.key)&&!arrow)||!editing.regionId||event.metaKey||event.ctrlKey||event.altKey)return
   const state=owner.getSnapshot()
   if(state.connection!=='connected'||!state.biliupCanEdit)return
   event.preventDefault();event.stopImmediatePropagation();cancel()
   const region=editing.regionId
   if(arrow){
    playback.pause()
    const step=event.shiftKey?10:1,dx=event.key==='ArrowLeft'?-step:event.key==='ArrowRight'?step:0,dy=event.key==='ArrowUp'?-step:event.key==='ArrowDown'?step:0
    void actions.itemEdit(project=>{
     const clip=findItem(project,editing.itemId)?.item,mask=clip?maskRegions(clip).find(value=>value.id===region):undefined
     if(!clip||!mask)return {problem:'The mask was removed.'}
     const media=project.media.find(value=>value.id===clip.media)
     return planMaskRect(project,clip.id,region,moveRegion(mask,dx/(media?.probe?.width??project.canvas.width),dy/(media?.probe?.height??project.canvas.height)))
    },{dir})
    return
   }
   void actions.itemEdit(project=>planRecordingEffectUpdate(project,editing.itemId,RECORDING_MASK,effect=>({...effect,params:{...effect.params,regions:((effect.params?.regions??[]) as EditorMaskRegion[]).filter(value=>value.id!==region)}})),{dir})
   owner.set({maskEditing:undefined})
  }
  root.addEventListener('pointerdown',outside,true);editorEvents.addEventListener('keydown',key,true)
  window.addEventListener('blur',cancel)
  return()=>{root.removeEventListener('pointerdown',outside,true);editorEvents.removeEventListener('keydown',key,true);window.removeEventListener('blur',cancel);cancel()}
 },[dir])

 const begin=(event:ReactPointerEvent<HTMLElement>,region:EditorMaskRegion,kind:Gesture['kind'])=>{
  if(event.button!==0||!visible||!placed||!picture.current)return
  const source=editorGestureOrigin()
  if(!source||source.dir!==dir||projectOf(source.owner.getSnapshot().snapshot!)!==project)return
  const bounds=picture.current.getBoundingClientRect()
  if(bounds.width<=0||bounds.height<=0)return
  event.preventDefault();event.stopPropagation();cancel();event.currentTarget.focus({preventScroll:true})
  playback.pause();playback.seek(frame)
  const context={itemId:placed.item.id,regionId:region.id,drawing:kind==='draw'}
  source.owner.set({maskEditing:context})
  const current:Gesture={source,base:project,item:placed.item.id,region,kind,pointer:event.pointerId,element:event.currentTarget,px:event.clientX,py:event.clientY,bounds,origin:{x:clamp((event.clientX-bounds.left)/bounds.width),y:clamp((event.clientY-bounds.top)/bounds.height)},moved:false,context}
  gesture.current=current
  const pointer=beginEditorGesture(event,{move:next=>move(next),cancel:()=>clear(current),commit:(_from,next)=>finish(next)})
  if(!pointer){clear(current);return}
  stop.current=()=>{pointer();clear(current)}
  event.currentTarget.setPointerCapture(event.pointerId)
 }
 const move=(event:PointerEvent,finishing=false)=>{
  const current=gesture.current
  if(!current||current.pending||event.pointerId!==current.pointer)return
  event.preventDefault()
  if(!event.buttons&&!finishing){cancel();return}
  const dx=event.clientX-current.px,dy=event.clientY-current.py
  if(!current.moved&&Math.hypot(dx,dy)<3)return
  current.moved=true
  current.next=undefined
  let rect:RenderRect
  if(current.kind==='draw'){
   const x=clamp((event.clientX-current.bounds.left)/current.bounds.width),y=clamp((event.clientY-current.bounds.top)/current.bounds.height)
   rect={x:Math.min(current.origin.x,x),y:Math.min(current.origin.y,y),width:Math.abs(current.origin.x-x),height:Math.abs(current.origin.y-y)}
   if(rect.width<.001||rect.height<.001){editor.cancel();return}
  }else rect=current.kind==='move'?moveRegion(current.region,dx/current.bounds.width,dy/current.bounds.height):resizeRegion(current.region,current.kind,dx/current.bounds.width,dy/current.bounds.height,event.shiftKey?!current.region.lock_aspect:current.region.lock_aspect)
  const plan=planMaskRect(current.base,current.item,current.region.id,rect,current.kind==='draw'?current.region:undefined)
  if('problem' in plan){editor.cancel();return}
  current.next=rect;editor.preview(current.base,plan)
 }
 const finish=(event:PointerEvent)=>{
  const current=gesture.current
  if(!current||event.pointerId!==current.pointer)return
  event.preventDefault()
  // Pointerup may carry a newer location than the last coalesced move.
  move(event,true)
  if(gesture.current!==current)return
  if(!current.moved||!current.next){clear(current);return}
  current.pending=true;release(current)
  const next=current.next
  void editor.commit(project=>planMaskRect(project,current.item,current.region.id,next,current.kind==='draw'?current.region:undefined),current.source).then(saved=>{
   const owner=current.source.owner
   if(saved&&owner===currentEditorStore()&&owner.getSnapshot().maskEditing===current.context)owner.set({maskEditing:{itemId:current.item,regionId:current.region.id}})
  }).finally(()=>clear(current))
 }
 if(!visible||!placed||!boxState)return null
 const {box}=boxState,content=box.content,regions=maskRegions(placed.item)
 const localMs=(frame-placed.start)*1000/shown.canvas.fps
 const rectStyle=(r:RenderRect):CSSProperties=>({left:r.x*content.w,top:r.y*content.h,width:r.width*content.w,height:r.height*content.h})
 const fresh=()=>({...(recordingEffect().params!.regions as EditorMaskRegion[])[0]!})
 return <div className="pv-mask-overlay" style={{left:box.x,top:box.y,width:box.w,height:box.h,borderRadius:box.radius}} onContextMenu={e=>{e.preventDefault();e.stopPropagation()}} onDoubleClick={e=>e.stopPropagation()} onClick={e=>e.stopPropagation()}>
  <div ref={picture} className="pv-mask-picture" style={{left:content.x-box.x,top:content.y-box.y,width:content.w,height:content.h}}>
   {regions.map((r,index)=>{
    const active=regionActive(r.intervals,localMs),chosen=edit?.regionId===r.id
    if(!active&&!chosen)return null
    return <div key={r.id} className="pv-mask-region" data-selected={chosen} data-active={active} style={rectStyle(r)} tabIndex={0} role="button" aria-label={t('Mask {0}',{0:index+1})}
     onFocus={()=>currentEditorStore()?.set({maskEditing:{itemId:placed.item.id,regionId:r.id}})}
     onPointerDown={event=>begin(event,r,'move')} onLostPointerCapture={event=>{if(gesture.current?.pointer===event.pointerId&&!gesture.current.pending)cancel()}}>
     <span className="pv-mask-label">{index+1}</span>
     {chosen&&!playing&&HANDLES.map(([handle,x,y])=><span key={handle} className="pv-mask-handle" data-direction={handle} style={{left:`${x*100}%`,top:`${y*100}%`}} onPointerDown={event=>begin(event,r,handle)}/>)}
    </div>
   })}
   {edit?.drawing&&!playing&&(regions.length<32||gesture.current?.kind==='draw')&&<div className="pv-mask-draw" tabIndex={0} aria-label={t('Draw mask')} onPointerDown={event=>begin(event,fresh(),'draw')} onLostPointerCapture={event=>{if(gesture.current?.pointer===event.pointerId&&!gesture.current.pending)cancel()}}/>}
  </div>
 </div>
}
