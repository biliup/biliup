import {fetcher} from '../api-streamer'
import {clipsUrl,type Clip} from '../clips'
import {markersUrl,type Marker} from '../markers'
import {sessionUrl,segmentEnd,unavailableIn,gapsIn,isReadable,type SessionDetail} from '../sessions'
import {suggestionsUrl,effectiveState,type Suggestion,type SuggestionList} from '../auto-clip'
import type {RecordingRange} from './recording-ranges'
import {editorText as t} from './i18n'

export type SavedRecordingSelection={kind:'clip';value:Clip}|{kind:'marker';value:Marker}

export function markerRange(marker:Marker,detail:SessionDetail){
  const end=Math.max(detail.duration_ms,...detail.segments.map(segmentEnd))
  const member=detail.members?.find(member=>member.source_session_id===(marker.source_session_id??marker.session_id))
  return {sessionId:marker.view_session_id??marker.session_id,inMs:Math.max(member?.offset_ms??0,marker.at_ms-marker.lookback_ms),outMs:Math.min(end,member?member.offset_ms+member.duration_ms:end,marker.at_ms+marker.lookahead_ms)}
}

export function requireRecordedRange(detail:SessionDetail,range:RecordingRange){
  const end=Math.max(detail.duration_ms,...detail.segments.map(segmentEnd))
  if(range.sessionId!==detail.id||!Number.isFinite(range.inMs)||!Number.isFinite(range.outMs)||range.inMs<0||range.outMs<=range.inMs||range.outMs>end
    ||unavailableIn(detail.segments,range.inMs,range.outMs).length||gapsIn(detail.gaps,range.inMs,range.outMs)>0){
    throw new Error(t('Some footage in this selection is unavailable.'))
  }
  let covered=range.inMs
  for(const segment of detail.segments.filter(isReadable).sort((a,b)=>a.start_ms-b.start_ms)){
    if(segmentEnd(segment)<=covered)continue
    if(segment.start_ms>covered)break
    covered=Math.max(covered,segmentEnd(segment))
    if(covered>=range.outMs)return
  }
  throw new Error(t('Some footage in this selection is unavailable.'))
}

/** Read the current source object before adding it; an edited/deleted selection must not be guessed. */
export async function readRecordingSelection(selection:SavedRecordingSelection,signal?:AbortSignal){
  const expected=selection.value,sessionId=expected.view_session_id??expected.session_id
  const detail=await fetcher(sessionUrl(sessionId),{signal}) as SessionDetail
  if(selection.kind==='clip'){
    const {clips}=await fetcher(clipsUrl(sessionId),{signal}) as {clips:Clip[]}
    const clip=clips.find(clip=>clip.id===expected.id)
    if(!clip||clip.state==='discarded')throw new Error(t('This saved selection is no longer available.'))
    if(clip.in_ms!==selection.value.in_ms||clip.out_ms!==selection.value.out_ms)throw new Error(t('This selection changed. Refresh the list before adding it.'))
    const range={sessionId,inMs:clip.in_ms,outMs:clip.out_ms}
    requireRecordedRange(detail,range)
    const {markers}=clip.marker_id===null?{markers:[] as Marker[]}:await fetcher(markersUrl(sessionId),{signal}) as {markers:Marker[]}
    return {detail,range,markers:markers.filter(marker=>marker.id===clip.marker_id),title:clip.title||t('Saved selection {0}',{0:clip.id})}
  }
  const {markers}=await fetcher(markersUrl(sessionId),{signal}) as {markers:Marker[]}
  const marker=markers.find(marker=>marker.id===expected.id)
  if(!marker)throw new Error(t('This recording marker was deleted.'))
  if(marker.at_ms!==selection.value.at_ms||marker.lookback_ms!==selection.value.lookback_ms||marker.lookahead_ms!==selection.value.lookahead_ms)throw new Error(t('This selection changed. Refresh the list before adding it.'))
  const range=markerRange(marker,detail)
  if(range.outMs<=range.inMs)throw new Error(t('This marker has no recorded range yet.'))
  requireRecordedRange(detail,range)
  return {detail,range,markers:[marker],title:marker.label||t('Recording marker {0}',{0:marker.id})}
}

/** Acceptance still happens in the save transaction; avoid probing an already stale suggestion. */
export async function readRecordingSuggestion(expected:Suggestion,signal?:AbortSignal){
  const sessionId=expected.view_session_id??expected.session_id
  const [detail,list]=await Promise.all([
    fetcher(sessionUrl(sessionId),{signal}) as Promise<SessionDetail>,
    fetcher(suggestionsUrl(sessionId),{signal}) as Promise<SuggestionList>,
  ])
  const suggestion=list.suggestions.find(item=>item.id===expected.id)
  if(!suggestion||effectiveState(suggestion,Date.now())!=='pending')throw new Error(t('This suggestion is no longer pending. Refresh the list.'))
  if(suggestion.in_ms!==expected.in_ms||suggestion.out_ms!==expected.out_ms)throw new Error(t('This selection changed. Refresh the list before adding it.'))
  const range={sessionId,inMs:suggestion.in_ms,outMs:suggestion.out_ms}
  requireRecordedRange(detail,range)
  return {detail,range,suggestion}
}
