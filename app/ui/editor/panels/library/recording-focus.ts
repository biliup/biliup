import type {EditorSession} from '@/app/lib/editor/session'
import type {SessionDetail} from '@/app/lib/sessions'
import type {Project} from '@/app/lib/editor/state/store'
import {currentEditorStore} from '@/app/lib/editor/state/store/core'
import {revealItem} from '@/app/lib/editor/state/store/edits'
import {layoutOf} from '@/app/lib/editor/state/project'
import {layout} from '@/app/lib/editor/core/timeline/timeline'
import {nearestRange,recordingRanges,recordingPoints,type RecordingRange,type TimelineRange} from '@/app/lib/editor/recording-ranges'
import {editorText as t} from '@/app/lib/editor/i18n'
import {getPlayback,playback} from '../../preview/playback'
import {captureEditorFocus} from '../../preview/focus'
import {panels} from '../../workspace/workspaceLayout'

function current(session:EditorSession){
  if(currentEditorStore()!==session.store)return
  const state=session.store.getSnapshot(),project=state.snapshot?.project as Project|undefined
  return project?{project,sources:state.biliupSources??{}}:undefined
}

/** A slow import must not replace a selection or playhead the user has since moved. */
export function recordingImportFocus(session:EditorSession){
  const unchanged=captureEditorFocus()
  return (ids:string[],range:Pick<TimelineRange,'from'|'to'>)=>{
    if(!unchanged())return
    const state=current(session),present=new Set(state?.project.tracks.flatMap(track=>track.items.map(item=>item.id)))
    if(!state||!ids.length||ids.some(id=>!present.has(id)))return
    focusRecordingItems(session,ids,range)
  }
}

export function focusRecordingItems(session:EditorSession,ids:string[],range?:Pick<TimelineRange,'from'|'to'>){
  const state=current(session);if(!state)return
  const positions=layout(state.project).starts,items=state.project.tracks.flatMap(track=>track.items).filter(item=>ids.includes(item.id))
  const first=items[0]
  if(!first||items.length!==new Set(ids).size)throw new Error(t('This range is no longer on the timeline.'))
  const from=range?.from??Math.min(...items.map(item=>positions.get(item.id)??0)),to=range?.to??Math.max(...items.map(item=>(positions.get(item.id)??0)+item.duration))
  const timing=layoutOf(state.project)
  playback.pause();playback.setProject(timing.frames,state.project.canvas.fps,timing.end);playback.hoverMedia(undefined)
  playback.selectMany(items.map(item=>item.id));playback.seek(from);playback.setMark('in',from);playback.setMark('out',to)
  panels.show('bottom');playback.setBottom('timeline');revealItem(first.id)
}

export function focusRecordingRange(session:EditorSession,range:RecordingRange,view?:SessionDetail){
  const state=current(session);if(!state)return
  const match=nearestRange(recordingRanges(state.project,state.sources,range,view),getPlayback().frame)
  if(!match)throw new Error(t('This range is not on the timeline. Add it to edit it.'))
  focusRecordingItems(session,match.items,match)
}

export function focusRecordingPoint(session:EditorSession,sessionId:number,atMs:number,view?:SessionDetail){
  const state=current(session);if(!state)return
  const at=getPlayback().frame,points=recordingPoints(state.project,state.sources,sessionId,atMs,view).sort((a,b)=>Math.abs(a.frame-at)-Math.abs(b.frame-at))
  const point=points[0];if(!point)throw new Error(t('This marker is outside the current edit. Add its recording range to edit it.'))
  const timing=layoutOf(state.project)
  playback.pause();playback.setProject(timing.frames,state.project.canvas.fps,timing.end);playback.hoverMedia(undefined)
  playback.select(point.item);playback.seek(point.frame);playback.setMark('in',undefined);playback.setMark('out',undefined)
  panels.show('bottom');playback.setBottom('timeline');revealItem(point.item)
}
