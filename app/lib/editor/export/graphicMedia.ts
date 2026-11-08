// The sandbox only names registered project media. Rust answers via the renderer's private binding.
import {graphicMediaAsk} from '@/app/ui/editor/preview/graphicMessage'
import {parseGraphicMediaRequest} from '../core/visual/graphicMedia'
let frame:HTMLIFrameElement|undefined,sequence=0
const waiting=new Map<number,{source:Window;seq:unknown;timer:ReturnType<typeof setTimeout>}>()
export function graphicMediaFrame(current:HTMLIFrameElement){
  for(const value of waiting.values())clearTimeout(value.timer)
  waiting.clear();frame=current
}
export function graphicMediaReply(seq:number,result:{bytes?:string;type?:string;error?:string}){
  const pending=waiting.get(seq);if(!pending)return;waiting.delete(seq);clearTimeout(pending.timer)
  if(pending.source!==frame?.contentWindow)return
  try{
    if(result.error)throw new Error(result.error)
    if(!result.bytes||!['image/png','image/bmp','image/svg+xml'].includes(result.type??''))throw new Error('素材帧格式无效')
    const binary=atob(result.bytes),bytes=new Uint8Array(binary.length);for(let i=0;i<binary.length;i++)bytes[i]=binary.charCodeAt(i)
    pending.source.postMessage({biliup:'media-reply',seq:pending.seq,blob:new Blob([bytes],{type:result.type})},'*')
  }catch(error){pending.source.postMessage({biliup:'media-reply',seq:pending.seq,error:error instanceof Error?error.message:String(error)},'*')}
}
addEventListener('message',(event:MessageEvent)=>{
  const ask=graphicMediaAsk(event,[frame?.contentWindow]);if(!ask)return
  const source=event.source as Window
  const fail=(error:string)=>source.postMessage({biliup:'media-reply',seq:ask.seq,error},'*')
  const query=new URLSearchParams();for(const [key,value]of Object.entries({t:ask.t,w:ask.w,h:ask.h}))if(value!==undefined)query.set(key,String(value))
  const request=parseGraphicMediaRequest(ask.id,query);if('problem'in request)return fail(request.problem)
  const binding=(globalThis as typeof globalThis&{__biliup_graphic_media?:(payload:string)=>void}).__biliup_graphic_media
  if(!binding)return fail('素材帧服务未连接')
  if(waiting.size>=64)return fail('单帧请求的素材数量超过限制')
  const seq=++sequence,timer=setTimeout(()=>graphicMediaReply(seq,{error:'读取素材帧超时'}),30000)
  waiting.set(seq,{source,seq:ask.seq,timer});binding(JSON.stringify({seq,request}))
})
