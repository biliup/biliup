// Adapted from Uncut agent/tools/{transcript,brand}.ts, MIT; licenses/uncut/LICENSE.
import type {Project,Item} from '../core/timeline/types'
import {layout} from '../core/timeline/timeline'
import {speechItem,transcriptScope,decodeEnvelope} from '../core/text/transcript'
import type {BeatsResult,LoudnessResult,TranscriptResult} from './analysisTypes'
import {transcriptOut,TRANSCRIPT_NOTE} from './transcript'
import {beatGrid,type GridItem} from '../analysis/beatGrid'

export type SpeechArgs={media?:string;item?:string;range?:{start:number;end:number};format?:'text'|'words';from?:number;language?:string;snap?:number[]}
type Outcome={state:string;cached?:boolean;result?:unknown;jobId?:string;error?:string}
export function mediaHeardOnMain(project:Project){
  const ids=new Set<string>()
  for(const track of project.tracks.filter(t=>t.role==='main'))for(const item of track.items){
    if(item.type!=='clip')continue
    const id=speechItem(project,item)?.media??(project.media.find(m=>m.id===item.media&&m.kind!=='image'&&m.probe?.hasAudio!==false)?item.media:undefined)
    if(id)ids.add(id)
  }
  return [...ids]
}
export function agentSpeechTarget(project:Project,tool:string,args:SpeechArgs){
  if(args.range&&(!Number.isSafeInteger(args.range.start)||!Number.isSafeInteger(args.range.end)||args.range.start<0||args.range.end<=args.range.start))throw new Error('范围应为起点小于终点的整数帧区间')
  let id=args.media,item:Item|undefined,request:{items?:readonly string[];clips?:readonly string[]}|undefined
  if(tool!=='beat_grid'){
    if(args.item!==undefined){
      item=project.tracks.flatMap(t=>t.items).find(i=>i.id===args.item)
      if(!item)throw new Error('时间轴中没有这个片段')
      const spoken=speechItem(project,item)
      if(!spoken?.media)throw new Error('此片段及其关联音轨没有可转写的声音')
      if(id!==undefined&&id!==spoken.media)throw new Error('指定素材与片段的语音来源不一致')
      id=spoken.media
      const onMain=(id:string|undefined)=>!!id&&project.tracks.some(t=>t.role==='main'&&t.items.some(i=>i.id===id))
      const clip=[item.id,item.linkedTo,spoken.linkedTo].find(onMain)
      const scope=clip?undefined:transcriptScope(project,item.id)
      request=clip?{clips:[clip]}:{items:scope?.items??[spoken.id]}
    }
    if(!id&&tool!=='transcript_read'){
      const heard=mediaHeardOnMain(project)
      if(heard.length!==1)throw new Error(heard.length?'时间轴上有多份语音，请指定素材或片段':'请指定素材或片段，或先将语音加入时间轴')
      id=heard[0]
    }
    if(!id)throw new Error('请指定素材或时间轴片段')
  }else{
    const audio=project.tracks.filter(t=>t.kind==='audio').flatMap(t=>t.items.filter(i=>i.type==='audio'&&i.media))
    item=args.item?audio.find(i=>i.id===args.item):audio.sort((a,b)=>b.duration-a.duration)[0]
    if(!item)throw new Error(args.item?'没有找到指定的音频片段':'请先将音乐加入时间轴')
    id=item.media
    if((item.speed??1)<=0||!Number.isFinite(item.speed??1))throw new Error('音频片段的播放速度无效')
  }
  const media=project.media.find(m=>m.id===id)
  if(!media)throw new Error('素材不在当前工程中')
  if(media.kind==='image')throw new Error('图片没有可分析的声音')
  return {media:media.id,...(item?{item:item.id}:{}),...(request?{request}:{}),...(tool==='beat_grid'?{grid:{id:item!.id,media:media.id,start:layout(project).starts.get(item!.id)??item!.start??0,in:item!.in??0,duration:item!.duration,speed:item!.speed??1,loop:item!.loop===true,sourceFrames:media.probe?.frames} satisfies GridItem}:{})}
}
export function agentSpeechResult(project:Project,tool:string,args:SpeechArgs,outcome:Outcome,sound?:LoudnessResult){
  const target=agentSpeechTarget(project,tool,args),media=project.media.find(m=>m.id===target.media)!,fps=project.canvas.fps
  if(outcome.state!=='done')return {media:target.media,item:target.item,[tool==='transcript_read'?'transcript':'beats']:outcome,...(!['failed','cancelled'].includes(outcome.state)?{note:'Analysis continues in the shared queue. Repeat this call with the same arguments to reuse its result.'}:{})}
  if(tool==='transcript_read'){
    const result=outcome.result as TranscriptResult
    const out=transcriptOut(result,{cached:outcome.cached===true,fps,length:media.probe?.frames,range:args.range,from:args.from,format:args.format??'text',silences:sound?.silence?.ranges,envelope:sound?.envelope?decodeEnvelope(sound.envelope):undefined})
    return {media:media.id,...(args.item?{item:args.item}:{}),...out,note:[typeof out.note==='string'?out.note:'',result.hasAudio?TRANSCRIPT_NOTE:''].filter(Boolean).join(' ')}
  }
  const beats=outcome.result as BeatsResult
  if(!beats.hasAudio||!beats.bpm)return {item:target.item,media:target.media,bpm:null,note:'No steady beat found in this sound; use the picture and words to choose cuts.'}
  return {cached:outcome.cached===true,...beatGrid(beats,target.grid!,fps,args.range,args.snap)}
}
