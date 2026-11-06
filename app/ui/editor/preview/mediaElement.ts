import type mpegts from 'mpegts.js'
import type {Media} from '@/app/lib/editor/core/timeline/types'
import {getState} from '@/app/lib/editor/state/store'
import {editorText} from '@/app/lib/editor/i18n'

type Source={readonly owner:object;readonly url:string;readonly filename:string;readonly position:()=>number}
type Binding={readonly source:Source;readonly streamed:boolean;readonly indexed:()=>boolean;readonly canSeek:(seconds:number)=>boolean;readonly dispose:()=>void}
const bindings=new WeakMap<HTMLMediaElement,Binding>()
const container=(filename:string)=>/\.flv$/i.test(filename)?'flv':/\.(?:ts|mts|m2ts)$/i.test(filename)?'mpegts':undefined
export const mediaPosition=new WeakMap<HTMLMediaElement,number>()

/** An unindexed MediaSource duration may describe only the data received so far. */
export function reachableMediaTime(element:HTMLMediaElement,seconds:number){
  const binding=bindings.get(element)
  let target=Math.max(0,seconds)
  if((!binding?.streamed||binding.indexed())&&Number.isFinite(element.duration)&&element.duration>0)target=Math.min(target,Math.max(0,element.duration-0.001))
  if(binding?.streamed&&element.buffered.length&&target<1){
    const start=element.buffered.start(0)
    if(start<1&&target<start)target=start
  }
  return target
}

export function canSeekEditorMedia(element:HTMLMediaElement,seconds:number){
  return bindings.get(element)?.canSeek(reachableMediaTime(element,seconds))??true
}

function applyPosition(element:HTMLMediaElement){
  const desired=mediaPosition.get(element)
  if(desired===undefined||element.readyState<1)return
  const target=reachableMediaTime(element,desired)
  if(!canSeekEditorMedia(element,target)||element.seeking)return
  if(Math.abs(element.currentTime-target)>0.004)element.currentTime=target
  else mediaPosition.delete(element)
}

/** Keep the latest requested position; buffered data and seek completion resume it. */
export function seekEditorMedia(element:HTMLMediaElement,seconds:number){
  if(!Number.isFinite(seconds))return
  mediaPosition.set(element,seconds)
  applyPosition(element)
}

export function releaseEditorMedia(element:HTMLMediaElement){bindings.get(element)?.dispose()}

function bindSource(element:HTMLMediaElement,source:Source){
  const initial=source.position()
  releaseEditorMedia(element)
  const type=source.url.includes('/proxy?')?undefined:container(source.filename)
  let disposed=false,failed=false,indexed=false,player:mpegts.Player|undefined,destroy:((player:mpegts.Player)=>void)|undefined
  const active=()=>!disposed&&bindings.get(element)===binding
  const stopPlayer=()=>{const previous=player;player=undefined;if(previous)destroy?.(previous)}
  const restore=()=>{
    if(!active())return
    seekEditorMedia(element,source.position())
  }
  const resume=()=>{if(active()&&!failed)applyPosition(element)}
  const nativeError=()=>{if(active()){failed=true;queueMicrotask(()=>{if(active())stopPlayer()})}}
  const binding:Binding={source,streamed:!!type,indexed:()=>indexed,canSeek:seconds=>{
    if(failed)return false
    if(!type||indexed)return true
    for(let i=0;i<element.buffered.length;i++)if(seconds>=element.buffered.start(i)&&seconds<element.buffered.end(i))return true
    return false
  },dispose:()=>{
    if(disposed)return
    disposed=true
    element.removeEventListener('loadedmetadata',restore)
    element.removeEventListener('error',nativeError)
    for(const event of ['seeked','progress','canplay','durationchange'])element.removeEventListener(event,resume)
    stopPlayer()
    if(bindings.get(element)===binding){bindings.delete(element);mediaPosition.delete(element);element.pause();element.removeAttribute('src');element.load()}
  }}
  bindings.set(element,binding)
  if(Number.isFinite(initial))mediaPosition.set(element,initial)
  element.dataset.src=source.url
  delete element.dataset.failedProxy
  delete element.dataset.sourceError
  element.addEventListener('loadedmetadata',restore)
  element.addEventListener('error',nativeError)
  for(const event of ['seeked','progress','canplay','durationchange'])element.addEventListener(event,resume)
  if(!type){element.src=source.url;return binding.dispose}
  const fail=(message:string)=>{
    if(!active()||failed)return
    failed=true;element.dataset.sourceError=message
    queueMicrotask(()=>{if(active())stopPlayer()})
    element.dispatchEvent(new Event('error'))
  }
  void import('@/app/lib/mpegts').then(({mpegts,AbortableFetchLoader,destroyMpegts})=>{
    if(!active())return
    if(!mpegts.isSupported()){fail(editorText('This browser cannot play this recording format.'));return}
    destroy=destroyMpegts
    player=mpegts.createPlayer({type,url:source.url,isLive:false,cors:true,withCredentials:true,...(element instanceof HTMLAudioElement?{hasVideo:false}:{})},{
      customLoader:AbortableFetchLoader,enableStashBuffer:false,accurateSeek:true,seekType:'range',
      lazyLoad:true,lazyLoadMaxDuration:60,lazyLoadRecoverDuration:15,
      autoCleanupSourceBuffer:true,autoCleanupMaxBackwardDuration:30,autoCleanupMinBackwardDuration:10,
    })
    player.on(mpegts.Events.ERROR,(_type:unknown,detail:unknown,info?:{msg?:string})=>fail(info?.msg??String(detail)))
    player.on(mpegts.Events.MEDIA_INFO,(info:{hasKeyframesIndex?:boolean})=>{if(active()){indexed=info.hasKeyframesIndex===true;resume()}})
    player.on(mpegts.Events.STATISTICS_INFO,resume)
    player.on(mpegts.Events.LOADING_COMPLETE,resume)
    player.attachMediaElement(element)
    player.load()
  }).catch(error=>fail(error instanceof Error?error.message:String(error)))
  return binding.dispose
}

/** Use the original native element: FLV/TS are transmuxed into its MediaSource, without another player UI. */
export function bindEditorMedia(element:HTMLMediaElement,media:Pick<Media,'id'|'path'>,url:string,position:()=>number){
  const filename=getState().biliupSources?.[media.id]?.fileName??media.path
  const owner={}
  bindSource(element,{owner,url,filename,position})
  return ()=>{if(bindings.get(element)?.source.owner===owner)releaseEditorMedia(element)}
}

/** A disposable proxy may fail. Retry its original once, using the same demuxer and time target. */
export function fallbackEditorProxy(element:HTMLMediaElement,url:string|undefined,original:string|undefined){
  const binding=bindings.get(element)
  if(!url||!original||!url.includes('/proxy?')||element.dataset.failedProxy===url||!binding)return false
  bindSource(element,{...binding.source,url:original})
  element.dataset.failedProxy=url
  return true
}
