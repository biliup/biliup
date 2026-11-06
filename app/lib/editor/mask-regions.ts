import type {Item,Project} from './core/timeline/types'
import {layout} from './core/timeline/timeline'
import type {EditorMaskRegion} from './recording-effects'
export {mergeMaskRanges} from './core/timeline/masks'

export type MaskInterval=EditorMaskRegion['intervals'][number]
const clamp=(value:number,min:number,max:number)=>Math.max(min,Math.min(max,value))

export function maskSize(region:EditorMaskRegion,axis:'width'|'height',value:number){
 const ratio=region.width/region.height
 let width=axis==='width'?value:region.lock_aspect?value*ratio:region.width
 let height=axis==='height'?value:region.lock_aspect?value/ratio:region.height
 if(region.lock_aspect){const scale=Math.min(1,(1-region.x)/width,(1-region.y)/height);width*=scale;height*=scale}
 return {width:clamp(width,.001,1-region.x),height:clamp(height,.001,1-region.y)}
}

/** A marked timeline selection in clip-local milliseconds, independent of source playback. */
export function maskRange(project:Project,item:Item,from:number,to:number):MaskInterval[]{
 const placed=layout(project).starts.get(item.id)
 if(placed===undefined)return []
 const start=Math.max(0,from-placed),end=Math.min(item.duration,to-placed)
 if(end<=start)return []
 const from_ms=Math.round(start*1000/project.canvas.fps)
 return [{from_ms,to_ms:Math.max(from_ms+1,Math.round(end*1000/project.canvas.fps))}]
}
