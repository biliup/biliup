import type {Project} from './core/timeline/types'
import {layout} from './core/timeline/timeline'
import type {MediaSource} from './project'
import type {SessionDetail} from '../sessions'

export interface RecordingRange {sessionId:number;inMs:number;outMs:number}
export interface TimelineRange {items:string[];from:number;to:number}

type SourceView=Pick<SessionDetail,'id'|'members'>
function parts(project:Project,sources:Record<string,MediaSource>,sessionId:number,view?:SourceView){
  const {starts}=layout(project),fps=project.canvas.fps
  return (project.tracks.find(track=>track.role==='main')?.items??[]).map(item=>{
    const source=item.media?sources[item.media]:undefined
    if(item.type!=='clip'||source?.kind!=='recording'||item.loop||item.hold||item.duration<=0)return undefined
    const member=view?.id===sessionId?view.members?.find(member=>member.source_session_id===source.sessionId):undefined
    const origin=source.sessionId===sessionId?source.sourceStartMs:member?member.offset_ms+source.sourceStartMs:source.viewSessionId===sessionId?source.viewStartMs:undefined
    if(origin===undefined)return undefined
    const speed=item.speed??1,from=origin+(item.in??0)*1000/fps
    return {id:item.id,from,to:from+item.duration*speed*1000/fps,start:starts.get(item.id)??0,duration:item.duration,speed}
  })
}

/** Complete occurrences only: a jump across removed source footage is not the requested range. */
export function recordingRanges(project:Project,sources:Record<string,MediaSource>,range:RecordingRange,view?:SourceView):TimelineRange[]{
  if(!Number.isFinite(range.inMs)||!Number.isFinite(range.outMs)||range.outMs<=range.inMs)return []
  const clips=parts(project,sources,range.sessionId,view),fps=project.canvas.fps,tolerance=1000/fps+0.01,result:TimelineRange[]=[]
  for(let i=0;i<clips.length;i++){
    const first=clips[i]
    if(!first||first.from>range.inMs+tolerance||first.to<=range.inMs)continue
    const from=first.start+Math.max(0,Math.floor((range.inMs-first.from)*fps/(1000*first.speed)))
    let sourceEnd=range.inMs,timelineEnd=from
    const items:string[]=[]
    for(let j=i;j<clips.length;j++){
      const part=clips[j]
      if(!part||j>i&&(Math.abs(part.from-sourceEnd)>tolerance||Math.abs(part.start-timelineEnd)>1))break
      sourceEnd=Math.min(range.outMs,part.to)
      timelineEnd=part.start+Math.min(part.duration,Math.max(0,Math.ceil((sourceEnd-part.from)*fps/(1000*part.speed))))
      items.push(part.id)
      if(sourceEnd>=range.outMs-tolerance){if(timelineEnd>from)result.push({items,from,to:timelineEnd});break}
    }
  }
  return result
}

export function recordingPoints(project:Project,sources:Record<string,MediaSource>,sessionId:number,atMs:number,view?:SourceView){
  if(!Number.isFinite(atMs))return []
  const clips=parts(project,sources,sessionId,view)
  const points=clips.flatMap(part=>part&&part.from<=atMs&&part.to>atMs
    ? [{item:part.id,frame:Math.min(part.start+part.duration-1,part.start+Math.round((atMs-part.from)*project.canvas.fps/(1000*part.speed)))}]:[])
  // A live marker can sit exactly at the recording's current end; display its last recorded frame.
  return points.length?points:clips.flatMap(part=>part&&Math.abs(part.to-atMs)<=1000/project.canvas.fps/2&&atMs>=part.from
    ? [{item:part.id,frame:part.start+part.duration-1}]:[])
}

export function nearestRange(ranges:TimelineRange[],frame:number){
  return [...ranges].sort((a,b)=>Math.abs(a.from-frame)-Math.abs(b.from-frame)||a.from-b.from)[0]
}
