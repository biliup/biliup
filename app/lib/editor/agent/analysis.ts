// Adapted from Uncut agent/tools/{media,library}.ts, MIT; licenses/uncut/LICENSE.
import type {Project} from '../core/timeline/types'
import {decodeEnvelope} from '../core/text/transcript'
import {framesFor,rankEntry,videoOut,VIDEO_NOTE} from './video'
import {rankValue,type RankBy} from '../analysis/videoScores'
import type {VideoResult} from '../analysis/videoResult'
import {beatsOut,BEATS_NOTE} from './beats'
import {loudnessOut} from './loudness'
import {transcriptSummary} from './transcript'
import type {BeatsResult,LoudnessResult,TranscriptResult} from './analysisTypes'

type Outcome={state:string;cached?:boolean;result?:unknown;jobId?:string;error?:string;progress?:number}
type Range={start:number;end:number}
export function agentAnalysis(project:Project,mediaId:string,range:Range|undefined,results:Record<string,Outcome>,sound?:LoudnessResult){
  const media=project.media.find(m=>m.id===mediaId)
  if(!media)throw new Error('素材不在当前工程中')
  const fps=project.canvas.fps,toFrames=framesFor(fps,media.probe?.frames)
  const out:Record<string,unknown>={media:mediaId,fps,note:'Analysis times are project frames of the source media, usable as in.'}
  for(const [name,outcome] of Object.entries(results)){
    if(outcome.state!=='done'){out[name]=outcome;continue}
    const cached=outcome.cached===true
    if(name==='video')out[name]=videoOut(outcome.result as VideoResult,cached,toFrames,range,fps)
    if(name==='beats')out[name]={...beatsOut(outcome.result as BeatsResult,cached,toFrames,range,fps),note:BEATS_NOTE}
    if(name==='loudness')out[name]=loudnessOut(outcome.result as LoudnessResult,cached,toFrames,range)
    if(name==='transcript')out[name]=transcriptSummary(outcome.result as TranscriptResult,cached,{silences:sound?.silence?.ranges,envelope:sound?.envelope?decodeEnvelope(sound.envelope):undefined})
  }
  return out
}
export function agentRank(project:Project,by:RankBy,window:number,results:Record<string,Outcome>,skipped:Array<{media:string;reason:string}>){
  const ranked:Array<{value:number;out:unknown}>=[],pending:unknown[]=[],failed:unknown[]=[]
  for(const [id,outcome] of Object.entries(results)){
    const media=project.media.find(m=>m.id===id)
    if(!media)throw new Error('素材不在当前工程中')
    if(outcome.state==='failed'||outcome.state==='cancelled'){failed.push({media:id,...outcome});continue}
    if(outcome.state!=='done'){pending.push({media:id,...outcome});continue}
    const video=outcome.result as VideoResult
    if(!video.hasVideo){skipped.push({media:id,reason:'No video stream found'});continue}
    const entry=rankEntry(media,video,outcome.cached===true,framesFor(project.canvas.fps,media.probe?.frames),window)
    ranked.push({value:rankValue(entry.scores,entry.best,by),out:entry.out})
  }
  ranked.sort((a,b)=>b.value-a.value)
  return {by,window,ranked:ranked.map((r,i)=>({rank:i+1,...r.out as object})),pending,failed,skipped,note:`Best stretches stay inside a shot and do not cross dropped frames. Pending analysis continues in the shared queue; repeat this call to use its cached results. ${VIDEO_NOTE}`}
}
