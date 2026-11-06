import type {Item,ModuleUse} from './types'
import type {MaskRegion} from '../../../renders'

export const RECORDING_MASK='effect.biliup-mask'
/** Coordinates use the source picture; intervals use milliseconds from the clip's start. */
export type EditorMaskRegion=Omit<MaskRegion,'asset_id'>&{image?:{media:string}}
export type MaskInterval=EditorMaskRegion['intervals'][number]

export function maskRegions(item:Item):EditorMaskRegion[]{
 return (item.effects?.find(effect=>effect.module===RECORDING_MASK&&!effect.bypass)?.params?.regions??[]) as EditorMaskRegion[]
}

export function mergeMaskRanges(intervals:ReadonlyArray<MaskInterval>):MaskInterval[]{
 const sorted=intervals.filter(span=>span.to_ms>span.from_ms).slice().sort((a,b)=>a.from_ms-b.from_ms)
 const merged:MaskInterval[]=[]
 for(const span of sorted){const previous=merged.at(-1);if(previous&&span.from_ms<=previous.to_ms)previous.to_ms=Math.max(previous.to_ms,span.to_ms);else merged.push({...span})}
 return merged
}

/** Crop and retime along with the picture, including a negative start when extending a clip. */
export function sliceMaskEffects(item:Item,from:number,to:number,duration:number,fps:number):Item['effects']{
 if(!item.effects)return undefined
 const first=from*1000/fps,last=to*1000/fps,ratio=duration/(to-from),end=Math.max(1,Math.round(duration*1000/fps))
 const effects:ModuleUse[]=item.effects.flatMap(effect=>{
  if(effect.module!==RECORDING_MASK)return [effect]
  const original=(effect.params?.regions??[]) as EditorMaskRegion[]
  const regions=original.flatMap(region=>{
   if(!region.intervals.length)return [region]
   const intervals=mergeMaskRanges(region.intervals.flatMap(span=>{
    const a=Math.max(first,span.from_ms),b=Math.min(last,span.to_ms)
    if(b<=a)return []
    const from_ms=Math.max(0,Math.min(end-1,Math.round((a-first)*ratio)))
    const to_ms=Math.max(from_ms+1,Math.min(end,Math.round((b-first)*ratio)))
    return [{from_ms,to_ms}]
   }))
   // An empty interval list means the whole clip. A discarded timed region must disappear.
   return intervals.length?[{...region,intervals}]:[]
  })
  return original.length&&!regions.length?[]:[{...effect,params:{...effect.params,regions}}]
 })
 return effects.length?effects:undefined
}
