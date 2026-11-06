import type {Marker} from '../markers'
import type {SessionDetail} from '../sessions'
import {mediaForCanvas,projectFromSession,type EditorDocument} from './project'
import {editHistory,type EditorHistory} from './history'
import {applyEdit,createSession} from './core/timeline/apply-ops'
import {mainEnd} from './core/timeline/rules'
import {takenIds,uniqueIn} from './core/timeline/arrange/place'
import type {MediaProbe,Op} from './core/timeline/types'
import {planImportCanvas} from './core/timeline/canvasMatch'
import {editorText as t} from './i18n'

export interface RecordingImport {
  readonly detail:SessionDetail
  readonly markers?:Marker[]
  readonly range?:{inMs:number;outMs:number}
  readonly title?:string
  readonly probes?:Readonly<Record<string,MediaProbe>>
}
export interface RecordingImportSelection {
  readonly sessionId:number
  readonly segmentIds:ReadonlyArray<number>
}

/** Recordings, saved selections and suggestions share native media and timeline operations. */
function recordingPlan(document:EditorDocument,{detail,markers=[],range,title,probes={}}:RecordingImport){
  const before=document.project,seed=projectFromSession(detail,{markers,range,canvas:before.canvas,probes})
  const matching=planImportCanvas(before,seed.project.media,Object.fromEntries(Object.entries(document.sources).map(([id,source])=>[id,source.durationMs])))
  const current=matching.project,imported=matching.ops.length?projectFromSession(detail,{markers,range,canvas:current.canvas,probes}):seed
  const taken=takenIds(current),sources={...document.sources},ids=new Map<string,string>(),ops:Op[]=[...matching.ops]
  for(const media of imported.project.media){
    const existing=current.media.find(item=>item.path===media.path&&sources[item.id]?.kind==='recording')
    const id=existing?.id??(taken.has(media.id)?uniqueIn(media.id,taken):media.id)
    taken.add(id);ids.set(media.id,id)
    const source=imported.sources[media.id],previous=sources[id]
    sources[id]=previous?.kind==='recording'&&source.kind==='recording'
      ? {...previous,durationMs:Math.max(previous.durationMs,source.durationMs),state:source.state,hasDanmaku:source.hasDanmaku} : source
    if(!existing)ops.push({op:'add_media',media:{...media,id}})
    else {
      const probe=mediaForCanvas({...existing,probe:{...existing.probe,...media.probe}},sources[id]!.durationMs,current.canvas).probe!
      if(Object.entries(probe).some(([key,value])=>existing.probe?.[key as keyof MediaProbe]!==value))ops.push({op:'update_media',media:id,set:{probe}})
    }
  }
  const start=mainEnd(current),items:string[]=[]
  if(range){
    let main=current.tracks.find(track=>track.role==='main')
    if(main?.locked)throw new Error(t('Unlock the main track to add a recording range.'))
    if(!main){main={id:uniqueIn('main',taken),name:t('Main'),kind:'video',role:'main',items:[]};ops.push({op:'add_track',track:{...main}})}
    for(const source of imported.project.tracks.find(track=>track.role==='main')!.items){
      const id=crypto.randomUUID();taken.add(id);items.push(id)
      ops.push({op:'insert_item',track:main.id,item:{...source,id,...(source.media?{media:ids.get(source.media)!}:{}),...(title?{name:title}:{})}})
    }
    if(imported.project.markers?.length)ops.push({op:'update_project',set:{markers:[...(current.markers??[]),...imported.project.markers.map(marker=>({...marker,frame:marker.frame+start}))]}})
  }
  const project=applyEdit(createSession(before),{actor:'user',ops}).project
  const prepared:EditorDocument={...document,project,sources,
    sourceMarkers:[...document.sourceMarkers,...(range?imported.sourceMarkers.map(marker=>({...marker,projectFrame:marker.projectFrame+start})):[])],
    warnings:[...new Set([...document.warnings,...imported.warnings])],origin:document.origin.sessionId?document.origin:imported.origin}
  return {ops,document:prepared,items,start}
}

/** One user selection creates one saved revision and one undo step, across any number of sessions. */
export function planRecordingImport(history:EditorHistory,document:EditorDocument,inputs:ReadonlyArray<RecordingImport>,summary:string){
  const ops:Op[]=[],items:string[]=[]
  let prepared={...document,project:history.current.project}
  const start=mainEnd(prepared.project)
  for(const input of inputs){
    const plan=recordingPlan(prepared,input)
    ops.push(...plan.ops);items.push(...plan.items);prepared=plan.document
  }
  const next=ops.length?editHistory(history,{actor:'user',baseRevision:history.current.revision,ops,summary}):history
  return {history:next,document:{...prepared,project:next.current.project},items,start,end:mainEnd(next.current.project)}
}
