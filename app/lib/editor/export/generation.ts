// Adapted from Uncut's generate/svg.ts (MIT). The same graphic bootstrap drives preview and rendering.
import {graphicDocument} from '../core/visual/graphics'
import {graphicMediaFrame} from './graphicMedia'
const SEEK=`<script>(()=>{if(typeof window.render==='function')return;window.render=t=>{for(const a of document.getAnimations()){a.pause();a.currentTime=t*1000}for(const svg of document.querySelectorAll('svg')){try{svg.pauseAnimations();svg.setCurrentTime(t)}catch{}}}})();</script>`
let frame:HTMLIFrameElement|undefined
let sequence=0
let dimensions={width:1920,height:1080}
export async function loadGenerated(markup:string,width:number,height:number) {
  if(new TextEncoder().encode(markup).length>512*1024)throw new Error('图形内容超过 512 KiB')
  frame?.remove();dimensions={width,height}
  const svg=/^\s*(?:<\?xml[^>]*>\s*)?(?:<!--[\s\S]*?-->\s*)*<svg\b/i.test(markup)
  const body=svg?`<style>body>svg{display:block;width:${width}px;height:${height}px}</style>${markup.replace(/^\s*<\?xml[^>]*>/i,'')}`:markup
  await loadGraphicDocument(graphicDocument(`${body}\n${SEEK}`,{width,height}),width,height)
}
export async function loadGraphicDocument(doc:string,width:number,height:number){
  frame?.remove();dimensions={width,height}
  document.head.innerHTML=''
  document.body.replaceChildren()
  const style=document.createElement('style');style.textContent='html,body{margin:0;padding:0;background:transparent;overflow:hidden}iframe{display:block;border:0;position:absolute;inset:0}';document.head.append(style)
  frame=document.createElement('iframe');frame.sandbox.add('allow-scripts');frame.width=String(width);frame.height=String(height)
  frame.referrerPolicy='no-referrer'
  const current=frame
  graphicMediaFrame(current)
  await new Promise<void>((resolve,reject)=>{
    const timer=setTimeout(()=>reject(new Error('图形加载超时')),10000)
    current.onload=()=>{clearTimeout(timer);resolve()}
    current.onerror=()=>{clearTimeout(timer);reject(new Error('图形加载失败'))}
    current.srcdoc=doc
    document.body.append(current)
  })
}
export async function drawGenerated(index:number,total:number,fps:number) {
  return drawGraphic(index/fps,{frame:index,fps,duration:total,...dimensions,props:{}})
}
export async function drawGraphic(time:number,info:Record<string,unknown>,census?:number){
  const source=frame?.contentWindow;if(!source)throw new Error('图形尚未加载')
  const seq=++sequence
  return new Promise<unknown>((resolve,reject)=>{
    const cleanup=()=>{clearTimeout(timer);window.removeEventListener('message',receive)}
    const receive=(event:MessageEvent)=>{
      const result=event.data
      if(event.source!==source||result?.biliup!=='rendered'||result.seq!==seq)return
      cleanup();if(result.error||result.skipped)reject(new Error(String(result.error??'图形跳过了输出帧').slice(0,500)));else resolve(result)
    }
    const timer=setTimeout(()=>{cleanup();reject(new Error('图形逐帧渲染超时'))},35000)
    window.addEventListener('message',receive)
    source.postMessage({biliup:'render',seq,t:time,info,...(census!==undefined?{census}:{})},'*')
  })
}
