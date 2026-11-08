import type {Item,Project} from '@/app/lib/editor/core/timeline/types'
import {maskRegions} from '@/app/lib/editor/core/timeline/masks'
import type {RecordingSource,EditorDocument} from '@/app/lib/editor/project'
import {apiResourceUrl,assPreview,defaultRecipe,type DanmakuSettings} from '@/app/lib/renders'
import {currentEditorStore} from '@/app/lib/editor/state/store/core'
import {mediaUrl} from '@/app/lib/editor/state/store/urls'
import {loadAssRenderer} from './ass-renderer'
import {editorText} from '@/app/lib/editor/i18n'

type AssRenderer = import('jassub').default
const destroyRenderer=(renderer:AssRenderer|undefined)=>{if(renderer)void renderer.destroy().catch(()=>{})}

/** Native picture masks and source-timed comments feed the existing compositor. */
export class RecordingEffectsRenderer {
 readonly canvas=document.createElement('canvas')
 private readonly owner=currentEditorStore()
 private readonly dir=this.owner?.getSnapshot().snapshot?.dir
 private readonly sample=document.createElement('canvas')
 private subtitles?:HTMLCanvasElement
 private renderer?:AssRenderer
 private preparing?:AssRenderer
 private abort?:AbortController
 private key=''
 private pendingKey=''
 private commentTimer?:ReturnType<typeof setTimeout>
 private origin=0
 private time=NaN
 private drawing=false
 private renderFailed=false
 private disposed=false
 private epoch=0
 private images=new Map<string,HTMLImageElement>()
 constructor(private repaint:()=>void){this.canvas.className='pv-media';this.canvas.style.pointerEvents='none'}
 private active(){return !this.disposed&&!!this.owner&&this.owner===currentEditorStore()&&this.owner.getSnapshot().snapshot?.dir===this.dir}
 private refresh(){if(this.active())this.repaint()}
 private report(message:string){if(this.active())this.owner!.set({notice:{text:message}})}
 private resetComments(){
  this.epoch++
  clearTimeout(this.commentTimer);this.commentTimer=undefined;this.pendingKey=''
  this.abort?.abort();this.abort=undefined
  destroyRenderer(this.preparing);this.preparing=undefined
  destroyRenderer(this.renderer);this.renderer=undefined
  this.subtitles?.remove();this.subtitles=undefined
  this.key='';this.time=NaN;this.drawing=false;this.renderFailed=false
 }
 dispose(){
  this.disposed=true;this.resetComments()
  for(const image of this.images.values()){image.onload=null;image.onerror=null;image.src=''}
  this.images.clear();this.canvas.remove()
 }
 private image(url:string){
  let image=this.images.get(url)
  if(!image){
   image=new Image();image.crossOrigin='use-credentials'
   const loaded=image
   image.onload=()=>{if(this.images.get(url)===loaded)this.refresh()}
   image.onerror=()=>{if(this.images.get(url)===loaded)this.report(editorText("Couldn't load the mask image."))}
   this.images.set(url,image);image.src=url
  }
  return image
 }
 draw(project:Project,sources:Readonly<EditorDocument['sources']>,item:Item,picture:HTMLVideoElement|HTMLImageElement|HTMLCanvasElement,seconds:number,localSeconds:number){
  if(!this.active())return picture
  const media=project.media.find(media=>media.id===item.media)
  const width=picture instanceof HTMLVideoElement?picture.videoWidth:picture instanceof HTMLImageElement?picture.naturalWidth:picture.width
  const height=picture instanceof HTMLVideoElement?picture.videoHeight:picture instanceof HTMLImageElement?picture.naturalHeight:picture.height
  if(!width||!height||(picture instanceof HTMLVideoElement&&picture.readyState<2))return picture
  if(this.canvas.width!==width)this.canvas.width=width
  if(this.canvas.height!==height)this.canvas.height=height
  const ctx=this.canvas.getContext('2d')!,sample=this.sample.getContext('2d')!
  const scaleX=width/(media?.probe?.width||width),scaleY=height/(media?.probe?.height||height)
  ctx.clearRect(0,0,width,height);ctx.drawImage(picture,0,0,width,height)
  const usedImages=new Set<string>()
  for(const r of maskRegions(item)){
   // Retain and preload timed image covers even while their interval is inactive.
   const cover=r.effect_type==='image'?project.media.find(media=>media.id===r.image?.media):undefined
   const url=cover?mediaUrl(cover.id,cover.path):undefined
   const image=url?this.image(url):undefined
   if(url)usedImages.add(url)
   if(r.intervals.length&&!r.intervals.some(span=>localSeconds*1000>=span.from_ms&&localSeconds*1000<span.to_ms))continue
   const x=Math.floor(r.x*width),y=Math.floor(r.y*height),w=Math.max(1,Math.min(width,Math.ceil((r.x+r.width)*width))-x),h=Math.max(1,Math.min(height,Math.ceil((r.y+r.height)*height))-y)
   ctx.save();ctx.globalAlpha=r.opacity
   if(r.effect_type==='solid'){ctx.fillStyle=r.color;ctx.fillRect(x,y,w,h)}
   else if(r.effect_type==='image'){
    if(image?.complete&&image.naturalWidth)ctx.drawImage(image,x,y,w,h)
   }else if(r.effect_type==='mosaic'||r.effect_type==='smooth_blur'){
    const strength=Math.max(1,Math.round(r.strength))
    this.sample.width=Math.max(1,Math.min(w,Math.floor(w/(strength*scaleX))));this.sample.height=Math.max(1,Math.min(h,Math.floor(h/(strength*scaleY))))
    sample.imageSmoothingEnabled=r.effect_type==='smooth_blur';sample.drawImage(this.canvas,x,y,w,h,0,0,this.sample.width,this.sample.height)
    ctx.imageSmoothingEnabled=sample.imageSmoothingEnabled;ctx.drawImage(this.sample,x,y,w,h)
   }else{this.sample.width=w;this.sample.height=h;sample.filter=`blur(${r.strength*Math.sqrt(scaleX*scaleY)}px)`;sample.drawImage(this.canvas,x,y,w,h,0,0,w,h);ctx.drawImage(this.sample,x,y,w,h)}
   ctx.restore()
  }
  for(const [url,image] of this.images){if(!usedImages.has(url)){image.onload=null;image.onerror=null;image.src='';this.images.delete(url)}}
  const source=item.media?sources[item.media]:undefined
  const settings=item.type==='danmaku'?item.danmaku?.settings:undefined
  if(settings?.enabled&&source?.kind==='recording'&&media){
   // The service returns the complete source ASS, so crossing five-minute windows,
   // looping and seeking must not launch the same XML conversion again.
   const key=JSON.stringify([media.id,media.path,source,settings,width,height])
   this.requestComments(source,settings,key)
   const time=(source.sourceStartMs+seconds*1000-this.origin)/1000
   if(this.key===key&&this.renderer&&!this.drawing&&!this.renderFailed&&this.time!==time){
    this.time=time;this.drawing=true
    const renderer=this.renderer,epoch=this.epoch
    void renderer.manualRender({expectedDisplayTime:performance.now(),width,height,mediaTime:time},true)
     .then(()=>{if(this.active()&&epoch===this.epoch){this.drawing=false;this.refresh()}})
     .catch(error=>{if(this.active()&&epoch===this.epoch){this.drawing=false;this.renderFailed=true;this.report(editorText('Danmaku preview failed: {0}',{0:String(error)}))}})
   }
   if(this.key===key&&this.renderer&&this.subtitles?.width&&this.subtitles.height)ctx.drawImage(this.subtitles,0,0,width,height)
  }else if(this.key||this.pendingKey)this.resetComments()
  return this.canvas
 }
 private requestComments(source:RecordingSource,settings:DanmakuSettings,key:string){
  if(key===this.key){clearTimeout(this.commentTimer);this.commentTimer=undefined;this.pendingKey='';return}
  if(key===this.pendingKey)return
  clearTimeout(this.commentTimer);this.pendingKey=key
  const requested=structuredClone(settings)
  // Coalesce numeric preview changes before regenerating ASS.
  this.commentTimer=setTimeout(()=>{
   this.commentTimer=undefined
   if(!this.active()||this.pendingKey!==key)return
   this.pendingKey='';this.load(source,requested,key)
  },180)
 }
 private load(source:RecordingSource,settings:DanmakuSettings,key:string){
  this.resetComments();this.key=key
  const abort=new AbortController();this.abort=abort
  const signal=abort.signal,epoch=this.epoch
  const active=()=>!signal.aborted&&this.active()&&epoch===this.epoch
  const recipe={...defaultRecipe(),danmaku:structuredClone(settings)}
  const canvas=document.createElement('canvas');this.subtitles=canvas
  void (async()=>{
   const [data,lib]=await Promise.all([
    assPreview(source.sessionId,recipe,source.sourceStartMs,source.sourceStartMs+source.durationMs,signal,source.segmentId),loadAssRenderer(),
   ])
   if(!active())return
   const fonts=await Promise.all(data.font_urls.map(async url=>{
    const response=await fetch(apiResourceUrl(url),{credentials:'include',signal})
    if(!response.ok)throw new Error(editorText("Couldn't load the danmaku font."))
    return new Uint8Array(await response.arrayBuffer())
   }))
   if(!active())return
   // JASSUB sizes its worker from the layout box; a detached canvas has zero size.
   Object.assign(canvas.style,{position:'fixed',left:'-100000px',top:'0',width:`${data.width}px`,height:`${data.height}px`,pointerEvents:'none',visibility:'hidden'})
   canvas.setAttribute('aria-hidden','true');document.body.append(canvas)
   const renderer=new lib.default({canvas,subContent:data.ass,fonts,queryFonts:false,defaultFont:recipe.danmaku.font,workerUrl:'/jassub/worker.js',wasmUrl:'/jassub/jassub-worker.wasm',modernWasmUrl:'/jassub/jassub-worker-modern.wasm'})
   this.preparing=renderer
   await renderer.ready
   if(!active())return
   this.preparing=undefined;this.renderer=renderer;this.origin=data.origin_ms;this.refresh()
  })().catch(error=>{
   if(!active())return
   destroyRenderer(this.preparing);this.preparing=undefined;canvas.remove()
   this.report(editorText('Danmaku preview failed: {0}',{0:String(error)}))
  })
 }
}
