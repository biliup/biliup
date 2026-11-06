// Adapted from Uncut exporter/rehearse.ts (MIT): the export's frame order, held frames
// and periodically refined blur samples, on one persistent graphic page.
import type {Project} from '../core/timeline/types'
import {BLUR_REFINE,blurLevels,blurRefinable,motionBlurDefault} from '../core/visual/motionBlur'
import {graphicWithAssets,type RenderAssets} from '../core/visual/renderAssets'
import {planExport,type VisualLayer} from './exportPlan'
import {drawGraphic} from './generation'
export function rehearsalPlan(project:Project,paths:Record<string,string>,graphics:Record<string,string>,assets:RenderAssets){
  const plan=planExport(project,{width:project.canvas.width,height:project.canvas.height,mediaPath:media=>{if(!paths[media.id])throw new Error(`素材不可读取：${media.id}`);return paths[media.id]!},graphic:src=>graphics[src]===undefined?undefined:graphicWithAssets(graphics[src]!,assets),motionBlur:motionBlurDefault(project,'high')})
  if('problem' in plan)throw new Error(plan.problem)
  const layers=plan.visuals.filter((l):l is Extract<VisualLayer,{kind:'graphic'}>=>l.kind==='graphic').map(layer=>{
    const calls:{frame:number;local:number;ts:readonly number[]}[]=[]
    let previous:readonly number[]|undefined
    for(const [i,local] of layer.frames.entries()){
      const ts=layer.motion?.ts[i]??[local/plan.fps]
      if(previous&&previous.length===ts.length&&previous.every((v,j)=>v===ts[j]))continue
      previous=ts
      const refined=layer.motion&&calls.length%30===0&&blurRefinable(ts)?[ts,...blurLevels(ts,Math.max(BLUR_REFINE.max,ts.length))].flat():ts
      calls.push({frame:layer.f0+i,local,ts:refined})
    }
    return {doc:layer.doc,name:layer.item.name??layer.item.src,info:{fps:plan.fps,duration:(layer.item.in??0)+layer.item.duration,...plan.canvas,props:layer.item.props??{}},calls}
  })
  return {layers,total:layers.reduce((n,l)=>n+l.calls.length,0),width:plan.canvas.width,height:plan.canvas.height,fps:plan.fps}
}
export async function rehearsalChunk(calls:{frame:number;local:number;ts:readonly number[]}[],info:Record<string,unknown>,name:string,budgetMs:number){
  const until=Date.now()+Math.max(0,budgetMs),fps=Number(info.fps)
  let covered=0,reached=0
  for(const call of calls){
    if(Date.now()>=until)return {covered,reached,partial:true}
    try{for(const t of call.ts)await drawGraphic(t,{...info,frame:call.local})}
    catch(error){return {covered,reached:call.frame/fps,frame:call.frame,failure:`The graphic "${name}" would stop the export at ${(call.frame/fps).toFixed(2)} s (its frame ${call.local}): ${String(error)}`}}
    covered++;reached=(call.frame+1)/fps
  }
  return {covered,reached}
}
