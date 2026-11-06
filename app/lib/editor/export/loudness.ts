import {apiFetch,handleResponse} from '../../api-streamer'
import type {ExportJob} from '../contracts'

export interface LoudnessResult {readonly gainDb?:number;readonly lufs?:number;readonly cancelled?:boolean}
export interface LoudnessPreviewState {
  readonly desiredKey?:string
  readonly result?:LoudnessResult&{readonly key:string;readonly targetLufs:number}
  readonly request?:{readonly id:string;readonly key:string;readonly step?:ExportJob['step']}
  readonly problem?:string
  readonly retry?:number
}

function progressStep(value:unknown):ExportJob['step']{
  if(value===undefined)return undefined
  if(!value||typeof value!=='object'||Array.isArray(value))throw new Error('响度测量进度无效')
  const step=value as NonNullable<ExportJob['step']>
  if(!['sound','loudness'].includes(step.kind)
    ||[step.done,step.total,step.pass].some(number=>number!==undefined&&(!Number.isSafeInteger(number)||number<0)))throw new Error('响度测量进度无效')
  return step
}

/** The response owns the server work; aborting this reader stops the measurement. */
export async function measureLoudness(project:number,revision:number,targetLufs:number,signal:AbortSignal,onProgress:(step:ExportJob['step'])=>void):Promise<LoudnessResult>{
  const response=await apiFetch(`/v1/editor/projects/${project}/exports/loudness`,{
    method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({revision,targetLufs}),signal,
  })
  await handleResponse(response)
  if(!response.body)throw new Error('响度测量连接已断开')
  const reader=response.body.getReader(),decoder=new TextDecoder()
  let pending=''
  try{
    while(true){
      let timer:ReturnType<typeof setTimeout>|undefined
      const {done,value}=await Promise.race([
        reader.read(),
        new Promise<never>((_,reject)=>{timer=setTimeout(()=>reject(new Error('响度测量连接超时，请重试')),45000)}),
      ]).finally(()=>clearTimeout(timer))
      if(done)throw new Error('响度测量连接已断开，请重试')
      pending+=decoder.decode(value,{stream:true})
      let newline:number
      while((newline=pending.indexOf('\n'))>=0){
        const line=pending.slice(0,newline).trim();pending=pending.slice(newline+1)
        if(!line)continue
        if(line.length>65536)throw new Error('响度测量响应过大')
        const event:unknown=JSON.parse(line)
        if(!event||typeof event!=='object'||Array.isArray(event))throw new Error('响度测量响应无效')
        const data=event as {type?:unknown;step?:ExportJob['step'];result?:LoudnessResult;message?:unknown}
        if(data.type==='progress'){onProgress(progressStep(data.step));continue}
        if(data.type==='keepalive')continue
        if(data.type==='error')throw new Error(typeof data.message==='string'?data.message:'响度测量失败')
        if(data.type==='result'&&data.result&&typeof data.result==='object'&&!Array.isArray(data.result)){
          const {gainDb,lufs,cancelled}=data.result
          if((gainDb!==undefined&&!Number.isFinite(gainDb))||(lufs!==undefined&&!Number.isFinite(lufs))
            ||(cancelled!==undefined&&typeof cancelled!=='boolean'))throw new Error('响度测量结果无效')
          return data.result
        }
        throw new Error('响度测量响应无效')
      }
      if(pending.length>65536)throw new Error('响度测量响应过大')
    }
  }finally{await reader.cancel().catch(()=>{});reader.releaseLock()}
}
