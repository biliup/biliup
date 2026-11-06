import {layout,sourceAt,spanOf,type Project} from '../core'
import {pictureItemIds} from './inputs'
import type {FrameRange} from './range'

/** Job-local source windows. Keep complete cycles only when a visible span crosses a loop boundary. */
export function danmakuRenderWindows(project:Project,range?:FrameRange|null){
 const {starts,duration}=layout(project),fps=project.canvas.fps
 const visible=new Set(pictureItemIds(project,range))
 const windows:Array<{item:string;from:number;to:number;in:number;loop:boolean}>=[]
 for(const track of project.tracks){
  if(track.hidden||track.kind==='audio')continue
  track.items.forEach((item,index)=>{
   if(item.type!=='danmaku'||!visible.has(item.id))return
   const length=project.media.find(media=>media.id===item.media)?.probe?.frames
   if(!length||!Number.isSafeInteger(length))throw new Error('Danmaku source duration is unavailable.')
   const start=starts.get(item.id)??0,span=spanOf(track,index,start)
   const first=Math.max(0,Math.ceil(span.from))-start,last=Math.min(duration,Math.ceil(span.to))-1-start
   if(last<first)return
   const raw=(local:number)=>(item.in??0)+local*(item.speed??1)
   const loops=item.loop===true&&Math.floor(raw(first)/length)!==(Math.ceil(raw(last+1)/length)-1)
   const firstSource=sourceAt(project,item,first)*fps,lastSource=sourceAt(project,item,last)*fps
   // Round only tiny floating-point errors at exact frame boundaries.
   const from=loops?0:Math.max(0,Math.min(length-1,Math.floor(firstSource+1e-7)))
   const to=loops?length:Math.max(from+1,Math.min(length,Math.ceil(lastSource+(item.speed??1)-1e-7)))
   const at=loops?(item.in??0):Math.max(0,Math.round(sourceAt(project,item,0)*fps)-from)
   windows.push({item:item.id,from,to,in:at,loop:loops})
  })
 }
 return windows
}
