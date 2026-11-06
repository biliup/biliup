import {fetcher} from '../api-streamer'
import {clipUrl,type Clip} from '../clips'
import {sessionUrl,isReadable,segmentEnd,type SessionDetail} from '../sessions'
import {markersUrl,type Marker} from '../markers'
import {projectsUrl,saveProject,type ProjectSummary} from './api'
import {DEFAULT_CANVAS,projectFromSession} from './project'
import {canvasForClip} from './core/timeline/canvasMatch'
import {recordingProbes} from './recording-probe'
import type {EditorSession} from './session'
import type {RecordingRange} from './recording-ranges'
import {readRecordingSelection} from './recording-selection'
import {editorText as t} from './i18n'
import {mutate} from 'swr'

export interface RecordingOwner {get(id:number):EditorSession;canEdit():boolean;isDisposed():boolean}
export type RecordingIntent={sessionId:number;atMs?:number}|{selectionId:number}
type Opened={id:number;at:number;end?:number}
type Openings={queue:Promise<unknown>;requests:Map<string,Promise<Opened>>}
const owners=new WeakMap<RecordingOwner,Openings>()

/** Serialize incoming source actions and reuse the same project session as manual edits. */
export function openRecording(owner:RecordingOwner,intent:RecordingIntent):Promise<Opened>{
  const state:Openings=owners.get(owner)??{queue:Promise.resolve(),requests:new Map()}
  owners.set(owner,state)
  const key='selectionId' in intent?`selection:${intent.selectionId}`:`session:${intent.sessionId}:${intent.atMs}`
  const pending=state.requests.get(key);if(pending)return pending
  const opening=state.queue.then(async()=>{
    const available=()=>{if(owner.isDisposed()||!owner.canEdit())throw new Error(t('This project cannot be edited.'))}
    available()
    let detail:SessionDetail,markers:Marker[],range:RecordingRange|undefined,title:string|undefined,atMs:number
    if('selectionId' in intent){
      const clip=await fetcher(clipUrl(intent.selectionId)) as Clip
      const selected=await readRecordingSelection({kind:'clip',value:clip})
      detail=selected.detail;markers=selected.markers;range=selected.range;title=selected.title;atMs=range.inMs
    }else{
      [detail,{markers}]=await Promise.all([fetcher(sessionUrl(intent.sessionId)) as Promise<SessionDetail>,fetcher(markersUrl(intent.sessionId)) as Promise<{markers:Marker[]}>])
      atMs=intent.atMs??detail.segments.filter(segment=>isReadable(segment)&&segmentEnd(segment)>segment.start_ms).sort((a,b)=>a.start_ms-b.start_ms)[0]?.start_ms??0
    }
    available()
    const projects=await fetcher(`${projectsUrl}?session_id=${detail.id}&limit=1`) as ProjectSummary[]
    available()
    let id=projects[0]?.id
    if(!id){
      if(!range&&!detail.segments.some(segment=>isReadable(segment)&&segment.start_ms<=atMs&&segmentEnd(segment)>=atMs&&segmentEnd(segment)>segment.start_ms))throw new Error(t('No recorded footage is available at this position.'))
      const probes=await recordingProbes(detail,range)
      available()
      const first=Object.values(probes)[0],canvas=first?canvasForClip(DEFAULT_CANVAS,first)??DEFAULT_CANVAS:DEFAULT_CANVAS
      const saved=await saveProject(projectFromSession(detail,{markers,range,name:title,canvas,probes}))
      id=saved.id;void mutate(key=>typeof key==='string'&&key.startsWith(projectsUrl))
    }
    available()
    const position=await owner.get(id).openRecordingAt(detail,markers,atMs,range)
    return {id,...position}
  })
  state.requests.set(key,opening)
  state.queue=opening.catch(()=>{})
  void opening.finally(()=>{state.requests.delete(key)}).catch(()=>{})
  return opening
}
