'use client'
import {ArrowDown,ArrowUp,Plus,Trash2,X,ImagePlus,Pencil} from 'lucide-react'
import {useEffect,useRef} from 'react'
import {Section,Prop,NumInput,Toggle,SettledColorField} from './controls'
import {actions,useApp,type Project} from '@/app/lib/editor/state/store'
import {currentEditorStore} from '@/app/lib/editor/state/store/core'
import type {Item,ModuleUse} from '@/app/lib/editor/core/timeline/types'
import {layout} from '@/app/lib/editor/core/timeline/timeline'
import {RECORDING_MASK,recordingEffect,planRecordingEffectUpdate,planMaskRegionUpdate,type EditorMaskRegion} from '@/app/lib/editor/recording-effects'
import {maskSize,maskRange,mergeMaskRanges} from '@/app/lib/editor/mask-regions'
import {imageRegionSize} from '@/app/lib/render-geometry'
import {editorText as t,useEditorLanguage} from '@/app/lib/editor/i18n'
import {getPlayback,playback,usePlayback} from '../preview/playback'
import {chooseEditorFiles,EDITOR_MEDIA_ACCEPT} from '@/app/lib/editor/browser-imports'
import {importFileSelection} from '@/app/lib/editor/state/store/server'
import {useProjectEdit} from '../workspace/projectEdit'
import {editorGestureOrigin,type GestureOrigin} from '../workspace/pointerGesture'

export function RecordingEffects({project,item:savedItem,locked}:{project:Project;item:Item;locked:boolean}){
 useEditorLanguage()
 const edit=useProjectEdit()
 const preview=usePlayback(state=>state.draft)
 const item=(preview?.base===project?preview.project.tracks.flatMap(track=>track.items).find(item=>item.id===savedItem.id):undefined)??savedItem
 const app=useApp(),dir=app.snapshot?.dir
 const element=useRef<HTMLDivElement>(null)
 const selectedRegion=app.maskEditing?.itemId===item.id?app.maskEditing.regionId:undefined
 useEffect(()=>{if(selectedRegion)element.current?.querySelector(`[data-mask-region="${CSS.escape(selectedRegion)}"]`)?.scrollIntoView({block:'nearest'})},[selectedRegion])
 const disabled=locked||app.connection!=='connected'||!app.biliupCanEdit
 const media=project.media.find(media=>media.id===item.media)
 const duration=Math.max(1,Math.round(item.duration*1000/project.canvas.fps))
 const images=project.media.filter(media=>media.kind==='image')
 const update=(module:string,change:(effect:ModuleUse)=>ModuleUse|null)=>{
  if(disabled||!dir)return
  void edit.commit(current=>planRecordingEffectUpdate(current,item.id,module,change),editorGestureOrigin())
 }
 const regions=(change:(regions:EditorMaskRegion[])=>EditorMaskRegion[])=>update(RECORDING_MASK,effect=>({...effect,params:{...effect.params,regions:change((effect.params?.regions??[]) as EditorMaskRegion[])}}))
 const region=(id:string,change:(region:EditorMaskRegion)=>EditorMaskRegion,origin:GestureOrigin|undefined=editorGestureOrigin())=>{
  if(!disabled)void edit.commit(current=>planMaskRegionUpdate(current,item.id,id,change),origin)
 }
 const number=(id:string,label:string,value:number,min:number,max:number,change:(region:EditorMaskRegion,n:number)=>EditorMaskRegion,step=.1,unit?:string)=><Prop label={t(label)}><NumInput testId={`recording-${label}`} ariaLabel={t(label)} value={value} min={min} max={max} step={step} unit={unit} disabled={disabled}
  onPreview={n=>edit.preview(project,planMaskRegionUpdate(project,item.id,id,current=>change(current,n)))} onCancel={edit.cancel}
  onCommit={(n,origin)=>{if(n!==undefined)region(id,current=>change(current,n),origin)}}/></Prop>
 const selectImage=(r:EditorMaskRegion,id:string)=>{
  const image=images.find(image=>image.id===id)
  if(!image)return r
  const size=imageRegionSize(image.probe?.width??1,image.probe?.height??1,media?.probe?.width??project.canvas.width,media?.probe?.height??project.canvas.height)
  return {...r,effect_type:'image' as const,image:{media:id},...size,x:Math.min(r.x,1-size.width),y:Math.min(r.y,1-size.height)}
 }
 const reorder=(id:string,direction:-1|1)=>regions(current=>{
  const index=current.findIndex(region=>region.id===id),target=index+direction
  if(index<0||target<0||target>=current.length)return current
  const next=[...current];[next[index],next[target]]=[next[target]!,next[index]!];return next
 })
 const markedRange=(id:string)=>{
  const playback=getPlayback()
  if(!dir||disabled)return
  void actions.itemEdit(current=>{
   if(playback.markIn===undefined||playback.markOut===undefined)return {problem:'Mark an in and out point on the timeline first.'}
   const clip=current.tracks.flatMap(track=>track.items).find(clip=>clip.id===item.id)
   if(!clip)return {problem:'Select a video clip in the timeline.'}
   const intervals=maskRange(current,clip,playback.markIn,playback.markOut)
   if(!intervals.length)return {problem:'The marked range does not overlap this clip.'}
   return planRecordingEffectUpdate(current,item.id,RECORDING_MASK,effect=>({...effect,params:{...effect.params,regions:((effect.params?.regions??[]) as EditorMaskRegion[]).map(r=>r.id===id?{...r,intervals:mergeMaskRanges([...r.intervals,...intervals])}:r)}}))
  },{dir})
 }
 const addRange=(id:string)=>{
  if(!dir||disabled)return
  const frame=getPlayback().frame
  void actions.itemEdit(current=>{
   const clip=current.tracks.flatMap(track=>track.items).find(clip=>clip.id===item.id)
   if(!clip)return {problem:'Select a video clip in the timeline.'}
   const start=layout(current).starts.get(clip.id)??0
   const from=frame>=start&&frame<start+clip.duration?frame:start
   const intervals=maskRange(current,clip,from,start+clip.duration)
   return planRecordingEffectUpdate(current,item.id,RECORDING_MASK,effect=>({...effect,params:{...effect.params,regions:((effect.params?.regions??[]) as EditorMaskRegion[]).map(r=>r.id===id?{...r,intervals:mergeMaskRanges([...r.intervals,...intervals])}:r)}}))
  },{dir})
 }
 const selectRegion=(regionId:string)=>{if(!disabled)currentEditorStore()?.set({maskEditing:{itemId:item.id,regionId}})}
 const addMask=()=>{
  if(disabled||!dir)return
  const owner=currentEditorStore(),added=(recordingEffect().params!.regions as EditorMaskRegion[])[0]!
  void actions.itemEdit(current=>{
   const clip=current.tracks.flatMap(track=>track.items).find(clip=>clip.id===item.id)
   const regions=(clip?.effects?.find(effect=>effect.module===RECORDING_MASK)?.params?.regions??[]) as EditorMaskRegion[]
   if(regions.length>=32)return {problem:'A clip can have up to 32 masks.'}
   return planRecordingEffectUpdate(current,item.id,RECORDING_MASK,effect=>({...effect,params:{...effect.params,regions:[...regions,added]}}))
  },{dir}).then(saved=>{if(saved&&owner===currentEditorStore())owner?.set({maskEditing:{itemId:item.id,regionId:added.id}})})
 }
 return <div className="mask-editor" ref={element}>{(item.effects??[]).filter(e=>e.module===RECORDING_MASK).map(effect=>{
  const maskRegions=(effect.params?.regions??[]) as EditorMaskRegion[]
  return <Section key={effect.module} title={t('Mask')} reveal={selectedRegion} action={<>
   <button className="icon-btn" title={t('Draw mask')} aria-label={t('Draw mask')} disabled={disabled||!!effect.bypass||maskRegions.length>=32} aria-pressed={app.maskEditing?.itemId===item.id&&!!app.maskEditing.drawing} onClick={()=>{
    playback.pause();playback.select(item.id)
    const start=layout(project).starts.get(item.id)??0,frame=getPlayback().frame
    if(frame<start||frame>=start+item.duration)playback.seek(start)
    currentEditorStore()?.set({maskEditing:{itemId:item.id,drawing:true}})
   }}><Pencil/></button>
   <Toggle testId={`power-${effect.module}`} label={t('Enabled')} value={!effect.bypass} disabled={disabled} onChange={enabled=>update(effect.module,current=>({...current,bypass:!enabled}))}/>
   <button className="icon-btn" aria-label={t('Remove effect')} disabled={disabled} onClick={()=>update(effect.module,()=>null)}><X/></button>
  </>}>
  <>
   {maskRegions.map((r,index)=><div key={r.id} data-mask-region={r.id} data-selected={selectedRegion===r.id} onPointerDownCapture={()=>selectRegion(r.id)} onFocusCapture={()=>selectRegion(r.id)}><Section title={`${t('Mask')} ${index+1}`} reveal={selectedRegion===r.id?r.id:undefined} action={<>
    <button className="icon-btn" title={t('Bring forward')} aria-label={t('Bring forward')} disabled={disabled||index===maskRegions.length-1} onClick={()=>reorder(r.id,1)}><ArrowUp/></button>
    <button className="icon-btn" title={t('Send backward')} aria-label={t('Send backward')} disabled={disabled||index===0} onClick={()=>reorder(r.id,-1)}><ArrowDown/></button>
    <button className="icon-btn" aria-label={t('Delete')} disabled={disabled} onClick={()=>regions(current=>current.filter(other=>other.id!==r.id))}><Trash2/></button>
   </>}>
    <Prop label={t('Effect')}><select className="input" disabled={disabled} value={r.effect_type} onChange={e=>{const type=e.target.value as EditorMaskRegion['effect_type'];region(r.id,current=>type==='image'&&images[0]?selectImage(current,images[0].id):{...current,effect_type:type,image:undefined})}}>
     {[['mosaic','Mosaic'],['blur','Blur'],['smooth_blur','Smooth blur'],['solid','Solid colour'],['image','Image']].map(([id,label])=><option key={id} value={id} disabled={id==='image'&&!images.length&&r.effect_type!=='image'}>{t(label!)}</option>)}
    </select></Prop>
    {r.effect_type==='image'&&<Prop label={t('Image')}><select className="input" disabled={disabled} value={r.image?.media??''} onChange={e=>region(r.id,current=>selectImage(current,e.target.value))}>
     {images.map(image=><option key={image.id} value={image.id}>{image.name}</option>)}
    </select></Prop>}
    {number(r.id,'X',r.x*100,0,(1-r.width)*100,(current,n)=>({...current,x:Math.min(n/100,1-current.width)}),.1,'%')}
    {number(r.id,'Y',r.y*100,0,(1-r.height)*100,(current,n)=>({...current,y:Math.min(n/100,1-current.height)}),.1,'%')}
    {number(r.id,'Width',r.width*100,.1,(1-r.x)*100,(current,n)=>({...current,...maskSize(current,'width',n/100)}),.1,'%')}
    {number(r.id,'Height',r.height*100,.1,(1-r.y)*100,(current,n)=>({...current,...maskSize(current,'height',n/100)}),.1,'%')}
    <Prop label={t('Lock aspect ratio')}><Toggle testId={`mask-aspect-${r.id}`} label={t('Lock aspect ratio')} value={r.lock_aspect} disabled={disabled} onChange={lock_aspect=>region(r.id,current=>({...current,lock_aspect}))}/></Prop>
    {number(r.id,'Opacity',r.opacity*100,0,100,(current,n)=>({...current,opacity:n/100}),1,'%')}
    {r.effect_type==='solid'?<Prop label={t('Colour')}><SettledColorField project={project} testId={`mask-colour-${r.id}`} value={r.color} disabled={disabled} noAlpha hexOnly
     onPreview={color=>edit.preview(project,planMaskRegionUpdate(project,item.id,r.id,current=>({...current,color})))} onCancel={edit.cancel}
     onCommit={(color,origin)=>region(r.id,current=>({...current,color}),origin)}/></Prop>:r.effect_type!=='image'&&number(r.id,'Strength',r.strength,1,100,(current,n)=>({...current,strength:n}),1)}
    <Section title={t('Clip time ranges')} open={r.intervals.length>0}>
     <Prop label={t('Entire clip')}><Toggle testId={`mask-entire-${r.id}`} label={t('Entire clip')} value={!r.intervals.length} disabled={disabled} onChange={enabled=>enabled?region(r.id,current=>({...current,intervals:[]})):addRange(r.id)}/></Prop>
     {r.intervals.map((span,i)=><div className="mask-range" key={i}>
      {number(r.id,'Start',span.from_ms/1000,0,Math.min(duration-1,span.to_ms-1)/1000,(current,n)=>({...current,intervals:current.intervals.map((v,j)=>i===j?{...v,from_ms:Math.min(Math.round(n*1000),v.to_ms-1)}:v)}),.001,'s')}
      {number(r.id,'End',span.to_ms/1000,(span.from_ms+1)/1000,duration/1000,(current,n)=>({...current,intervals:current.intervals.map((v,j)=>i===j?{...v,to_ms:Math.max(Math.round(n*1000),v.from_ms+1)}:v)}),.001,'s')}
      <button className="btn ghost" disabled={disabled} onClick={()=>regions(current=>current.flatMap(region=>region.id!==r.id?[region]:region.intervals.length===1?[]:[{...region,intervals:region.intervals.filter((_,j)=>j!==i)}]))}>{t('Remove range')}</button>
     </div>)}
     <div className="field-row">
      <button className="btn" disabled={disabled||r.intervals.length>=1000} onClick={()=>addRange(r.id)}><Plus size={14}/>{t('Add range')}</button>
      <button className="btn" disabled={disabled||r.intervals.length>=1000} onClick={()=>markedRange(r.id)}>{t('Use marked range')}</button>
     </div>
    </Section>
   </Section></div>)}
   <div className="field-row">
    <button className="btn" disabled={disabled||maskRegions.length>=32} onClick={addMask}><Plus size={14}/>{t('Mask')}</button>
    <button className="btn" disabled={disabled||!!app.uploading} onClick={()=>importFileSelection(chooseEditorFiles(false,EDITOR_MEDIA_ACCEPT.image))}><ImagePlus size={14}/>{t('Import image…')}</button>
   </div>
  </>
  </Section>
 })}</div>
}
