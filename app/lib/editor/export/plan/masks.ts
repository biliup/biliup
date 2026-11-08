import type {Item,Media} from '../../core/timeline/types'
import {maskRegions,type EditorMaskRegion} from '../../core/timeline/masks'
import {n} from './num'

export interface MaskLayer {
 readonly regions:ReadonlyArray<EditorMaskRegion>
 readonly width:number
 readonly height:number
 readonly start:number
 readonly images:Readonly<Record<string,string>>
}

export function planMasks(item:Item,media:Media,library:ReadonlyArray<Media>,fileOf:(media:Media)=>string,start:number):MaskLayer|undefined{
 const regions=maskRegions(item)
 if(!regions.length)return undefined
 const width=media.probe?.width,height=media.probe?.height
 if(!width||!height)throw new Error('Mask source dimensions are unavailable.')
 if(width*height>16_777_216)throw new Error('Mask source dimensions exceed the rendering limit.')
 const images:Record<string,string>={}
 for(const region of regions)if(region.effect_type==='image'){
  const image=library.find(media=>media.id===region.image?.media&&media.kind==='image')
  if(!image)throw new Error('The mask image is no longer in the project.')
  images[image.id]=fileOf(image)
 }
 return {regions,width,height,start,images}
}

/** Runs after source frame selection/speed/loop/hold and before shaders and picture placement. */
export function maskGraph(masks:MaskLayer,input:string,output:string,fps:number,frame:number,firstInput:number):{inputs:string[];graph:string[]}{
 const {width,height}=masks,inputs:string[]=[],graph:string[]=[]
 graph.push(`[${input}]scale=${width}:${height},format=rgba[${output}base]`)
 let previous=`${output}base`
 for(const [index,r] of masks.regions.entries()){
  if(r.opacity<=0)continue
  const key=`${output}m${index}`
  const x=Math.floor(r.x*width),y=Math.floor(r.y*height)
  const w=Math.max(1,Math.min(width,Math.ceil((r.x+r.width)*width))-x),h=Math.max(1,Math.min(height,Math.ceil((r.y+r.height)*height))-y)
  const at=`(t+${n((frame-masks.start)/fps)})`
  const enabled=r.intervals.length?r.intervals.map(span=>`gte(${at},${n(span.from_ms/1000)})*lt(${at},${n(span.to_ms/1000)})`).join('+'):'1'
  if(r.effect_type==='solid'){
   if(!/^#[0-9a-f]{6}$/i.test(r.color))throw new Error('Mask colour must be a six-digit hex colour.')
   graph.push(`[${previous}]drawbox=x=${x}:y=${y}:w=${w}:h=${h}:color=${r.color}@${n(r.opacity)}:t=fill:enable='${enabled}'[${key}]`)
  }else{
   if(r.effect_type==='image'){
    const file=r.image&&masks.images[r.image.media]
    if(!file)throw new Error('The mask image is no longer in the project.')
    const slot=firstInput+inputs.filter(value=>value==='-i').length
    inputs.push('-f','image2','-pattern_type','none','-i',file)
    graph.push(`[${slot}:v:0]scale=${w}:${h},format=rgba,colorchannelmixer=aa=${n(r.opacity)},setpts=0[${key}cover]`)
   }else{
    graph.push(`[${previous}]split=2[${key}keep][${key}crop]`)
    previous=`${key}keep`
    const block=Math.max(1,Math.round(r.strength)),flags=r.effect_type==='mosaic'?'neighbor':'bilinear'
    const transform=r.effect_type==='blur'?`gblur=sigma=${n(r.strength)}`:`scale=${Math.max(1,Math.floor(w/block))}:${Math.max(1,Math.floor(h/block))}:flags=${flags},scale=${w}:${h}:flags=${flags}`
    graph.push(`[${key}crop]crop=${w}:${h}:${x}:${y}:exact=1,${transform},format=rgba,colorchannelmixer=aa=${n(r.opacity)}[${key}cover]`)
   }
   graph.push(`[${previous}][${key}cover]overlay=x=${x}:y=${y}:format=auto:eof_action=repeat:enable='${enabled}'[${key}]`)
  }
  previous=key
 }
 graph.push(`[${previous}]null[${output}]`)
 return {inputs,graph}
}

export function applyLayerMasks(layer:{masks?:MaskLayer;f0:number},picked:ReadonlyArray<string>,inputs:string[],graph:string[],prefix:string,fps:number):string[]{
 if(!layer.masks)return [...picked]
 const pass=maskGraph(layer.masks,`${prefix}in`,prefix,fps,layer.f0,inputs.filter(value=>value==='-i').length)
 inputs.push(...pass.inputs)
 graph.push(`${picked.join(',')},settb=1/${fps},setpts=N[${prefix}in]`,...pass.graph)
 return [`[${prefix}]null`]
}
