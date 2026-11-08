// Adapted from Uncut agent/tools/transcript.ts and turns/notes.ts, MIT; licenses/uncut/LICENSE.
import type {Project} from '../core/timeline/types'
import {planTextCut,decodeEnvelope,DEFAULT_PAUSE_MIN_S,type TextCutRequest} from '../core/text/transcript'
import {planMakeCaptions,captionCardList,type CaptionLines,type CaptionMode} from '../core/text/captions'
import {subtitleCues,subtitleTime,writeSubtitles,type SubtitleFormat} from '../core/text/subtitles'
import {agentEdit} from './runtime'
import {agentSpeechTarget,mediaHeardOnMain,type SpeechArgs} from './speech'
import type {TranscriptResult,LoudnessResult} from './analysisTypes'

type EditArgs=SpeechArgs&{remove?:{from:string|number;to?:string|number;clip?:string;cycle?:number}[];pauses?:{min?:number};fillers?:boolean;dryRun?:boolean;mode?:CaptionMode;look?:string;captionStyle?:string;lines?:CaptionLines;hideFillers?:boolean}
function wordIndex(value:string|number){
  const index=typeof value==='number'?value:/^w?\d+$/.test(value)?Number(value.replace(/^w/,'')):NaN
  if(!Number.isSafeInteger(index)||index<0)throw new Error('请使用转写结果中的词编号，例如 w12')
  return index
}
export function agentSpeechPlan(project:Project,tool:string,args:EditArgs,transcript:TranscriptResult,sound:LoudnessResult,turnId:string){
  const target=agentSpeechTarget(project,tool,args),media=project.media.find(m=>m.id===target.media)!,fps=project.canvas.fps
  if(!transcript.hasAudio||!transcript.words?.length)throw new Error('此素材没有可编辑的语音')
  const speech={words:transcript.words,language:transcript.language,seconds:transcript.seconds,segments:transcript.segments}
  const measured={silences:sound.silence?.ranges,envelope:sound.envelope?decodeEnvelope(sound.envelope):undefined}
  if(tool==='transcript_edit'){
    if(!args.remove?.length&&!args.pauses&&!args.fillers)throw new Error('请选择要删除的词、停顿或语气词')
    const targets=target.request?.clips??target.request?.items
    if(targets&&args.remove?.some(range=>range.clip!==undefined&&!targets.includes(range.clip)))throw new Error('选词范围不属于指定片段')
    const request:TextCutRequest={...target.request,...measured,
      ...(args.remove?{remove:args.remove.map(range=>({from:wordIndex(range.from),to:wordIndex(range.to??range.from),...(range.clip!==undefined?{clip:range.clip}:{}),...(range.cycle!==undefined?{cycle:range.cycle}:{})}))}:{}),
      ...(args.pauses?{pauses:{min:args.pauses.min??DEFAULT_PAUSE_MIN_S}}:{}),...(args.fillers?{fillers:true}:{})}
    const items=!target.item&&!mediaHeardOnMain(project).includes(media.id)?project.tracks.filter(t=>t.role!=='main').flatMap(t=>t.items.filter(i=>(i.type==='clip'||i.type==='audio')&&i.media===media.id).map(i=>i.id)):undefined
    const plan=planTextCut(project,media.id,speech,{...request,...(items?.length?{items}:{})})
    if('problem'in plan)throw new Error(plan.problem)
    const secs=(frames:number)=>Number((frames/fps).toFixed(2))
    return {ops:plan.ops,summary:plan.summary,result:{media:media.id,...(target.item?{item:target.item}:{}),
      removed:plan.cuts.slice(0,200).map(c=>({cycle:c.cycle,...(c.words?{words:c.words[0]===c.words[1]?`w${c.words[0]}`:`w${c.words[0]}–w${c.words[1]}`} : {}),text:c.text||'(pause)',...(c.pauses?{pauses:c.pauses}:{}),...(c.fillers?{fillers:c.fillers}:{}),mediaFrames:{start:c.from,end:c.to},frames:c.frames,seconds:secs(c.frames),clips:c.clips})),
      removedCount:plan.cuts.length,...(plan.cuts.length>200?{truncated:true}:{}),framesRemoved:plan.framesRemoved,secondsSaved:secs(plan.framesRemoved),duration:plan.after,seconds:secs(plan.after),durationBefore:plan.before,
      ...(plan.notes.length?{notes:plan.notes}:{}),note:args.dryRun?'Dry run; nothing changed.':!plan.ops.length?'Nothing to cut; nothing changed.':'Word ids belong to the original transcript and do not change after cuts. project_read shows the remaining items; other tracks do not ripple with Main. One turn undo restores the edit.'}}
  }
  if(tool!=='captions_make')throw new Error('未知语音编辑工具')
  const plan=planMakeCaptions(project,{media:media.id,transcript:speech,...measured,...(args.mode?{mode:args.mode}:{}),...(args.look?{look:args.look}:{}),...(args.captionStyle!==undefined?{captionStyle:args.captionStyle}:{}),...(args.lines?{lines:args.lines}:{}),...(args.hideFillers!==undefined?{hideFillers:args.hideFillers}:{})})
  if('problem'in plan)throw new Error(plan.problem)
  const after=agentEdit(project,{ops:plan.ops,summary:plan.summary,turnId}).project
  const item=after.tracks.flatMap(t=>t.items).find(i=>i.id===plan.itemId)!,cards=captionCardList(after,item)
  return {ops:plan.ops,summary:plan.summary,result:{media:media.id,item:plan.itemId,remade:plan.remade,mode:item.mode??'line',words:plan.words,cards:cards.slice(0,40),cardCount:cards.length,fixesDropped:plan.fixesDropped,...(plan.fixesDropped?{notes:[`${plan.fixesDropped} corrections or manual card breaks could not be matched unambiguously to the new transcript and were reset. Other corrections were preserved.`]}:{})}}
}
export function agentSubtitles(project:Project,format:SubtitleFormat){
  if(!['srt','vtt'].includes(format))throw new Error('请选择 SRT 或 WebVTT 格式')
  const cues=subtitleCues(project),fps=project.canvas.fps
  if(!cues.length)throw new Error('工程中没有可导出的字幕')
  return {text:writeSubtitles(cues,fps,format),format,cues:cues.length,first:cues.slice(0,5).map(c=>({from:c.from,to:c.to,time:`${subtitleTime(c.from,fps,format)} --> ${subtitleTime(c.to,fps,format)}`,text:c.text}))}
}
