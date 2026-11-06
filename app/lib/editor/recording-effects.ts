import type {ModuleUse,Project} from './core/timeline/types'
import {RECORDING_MASK,type EditorMaskRegion} from './core/timeline/masks'
export {RECORDING_MASK,type EditorMaskRegion} from './core/timeline/masks'
import {planDanmaku} from './danmaku'
export const DANMAKU_TOOL='recorded-danmaku'

/** Read the current item at commit time so queued controls cannot overwrite each other. */
export function planRecordingEffectUpdate(project:Project,id:string,module:string,update:(effect:ModuleUse)=>ModuleUse|null){
 for(const track of project.tracks)for(const item of track.items){
  if(item.id!==id)continue
  if(track.locked||item.locked)return {problem:'That clip is locked: unlock it to change its effects.'}
  if(!item.effects?.some(effect=>effect.module===module))return {problem:'The effect was removed.'}
  const effects=item.effects.flatMap(effect=>effect.module===module?(update(effect)??[]):effect)
  if(module===RECORDING_MASK){
   const regions=(effects.find(effect=>effect.module===module)?.params?.regions??[]) as EditorMaskRegion[]
   if(regions.some(region=>region.intervals.length>1000))return {problem:'A mask can have up to 1000 time ranges.'}
   const end=Math.max(1,Math.round(item.duration*1000/project.canvas.fps))
   if(regions.some(region=>region.intervals.some(span=>span.from_ms<0||span.to_ms<=span.from_ms||span.to_ms>end)))return {problem:'Mask ranges must stay inside the clip.'}
  }
  return {ops:[{op:'update_item' as const,item:id,set:{effects}}],summary:'Effects'}
 }
 return {problem:'Select a video clip in the timeline.'}
}
export function recordingEffect():ModuleUse {
 return {module:RECORDING_MASK,params:{regions:[{id:crypto.randomUUID(),effect_type:'mosaic',x:.1,y:.1,width:.2,height:.2,strength:12,color:'#000000',opacity:1,lock_aspect:true,intervals:[]}]}}
}

/** Address a mask by identity, including during queued numeric/colour gestures. */
export function planMaskRegionUpdate(project:Project,itemId:string,regionId:string,update:(region:EditorMaskRegion)=>EditorMaskRegion){
 const item=project.tracks.flatMap(track=>track.items).find(item=>item.id===itemId)
 const regions=(item?.effects?.find(effect=>effect.module===RECORDING_MASK)?.params?.regions??[]) as EditorMaskRegion[]
 if(!regions.some(region=>region.id===regionId))return {problem:'The mask was removed.'}
 return planRecordingEffectUpdate(project,itemId,RECORDING_MASK,effect=>({...effect,params:{...effect.params,
  regions:((effect.params?.regions??[]) as EditorMaskRegion[]).map(region=>region.id===regionId?update(region):region),
 }}))
}

export function planMaskRect(project:Project,itemId:string,regionId:string,rect:Pick<EditorMaskRegion,'x'|'y'|'width'|'height'>,create?:EditorMaskRegion){
 const geometry={x:rect.x,y:rect.y,width:rect.width,height:rect.height}
 if(!Object.values(geometry).every(Number.isFinite)||rect.x<0||rect.y<0||rect.width<.001||rect.height<.001||rect.x+rect.width>1.000001||rect.y+rect.height>1.000001)return {problem:'The mask must stay inside the source picture.'}
 const item=project.tracks.flatMap(track=>track.items).find(item=>item.id===itemId)
 const existing=(item?.effects?.find(effect=>effect.module===RECORDING_MASK)?.params?.regions??[]) as EditorMaskRegion[]
 if(existing.some(region=>region.id===regionId))return planMaskRegionUpdate(project,itemId,regionId,region=>({...region,...geometry}))
 if(!create)return {problem:'The mask was removed.'}
 if(existing.length>=32)return {problem:'A clip can have up to 32 masks.'}
 return planRecordingEffectUpdate(project,itemId,RECORDING_MASK,effect=>{
  const regions=(effect.params?.regions??[]) as EditorMaskRegion[]
  return {...effect,params:{...effect.params,regions:[...regions,{...create,...geometry}]}}
 })
}

export function planRecordingEffect(project:Project,id:string,module:string):{ops:import('./core/timeline/types').Op[];summary:string}|{problem:string} {
 if(module===DANMAKU_TOOL)return planDanmaku(project,id)
 if(module!==RECORDING_MASK)return {problem:'This effect is unavailable.'}
 for(const track of project.tracks)for(const item of track.items){
  if(item.id!==id)continue
  const media=project.media.find(media=>media.id===item.media)
  if(item.type!=='clip'||!media||media.kind==='audio')return {problem:'Select a video or image clip in the timeline.'}
  if(track.locked||item.locked)return {problem:'That clip is locked: unlock it to change its effects.'}
  const existing=item.effects?.find(effect=>effect.module===module)
  if(existing&&!existing.bypass)return {ops:[],summary:''}
  const effects=existing?(item.effects??[]).map(effect=>effect.module===module?{...effect,bypass:false}:effect):[...(item.effects??[]),recordingEffect()]
  return {ops:[{op:'update_item',item:id,set:{effects}}],summary:'Add mask'}
 }
 return {problem:'Select a video clip in the timeline.'}
}
