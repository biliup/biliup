import {apiFetch,handleResponse} from '../api-streamer'
import {isReadable,segmentEnd,type SessionDetail} from '../sessions'
import type {MediaProbe} from './core/timeline/types'

/** Every imported segment needs metadata for native media preview, placement and audio controls. */
export async function recordingProbes(detail:SessionDetail,range?:{inMs:number;outMs:number},signal?:AbortSignal):Promise<Record<string,MediaProbe>>{
  const from=range?.inMs??0,to=range?.outMs??Infinity
  const segments=[...detail.segments].sort((a,b)=>a.start_ms-b.start_ms||a.id-b.id)
    .filter(segment=>isReadable(segment)&&Math.min(segmentEnd(segment),to)>Math.max(segment.start_ms,from))
  const probes:Record<string,MediaProbe>={}
  // Keep requests bounded; preserve source order so the first clip determines an automatic canvas.
  for(let offset=0;offset<segments.length;offset+=3){
    const batch=segments.slice(offset,offset+3)
    const results=await Promise.allSettled(batch.map(async segment=>{
      const id=`recording-${segment.source_session_id??detail.id}-${segment.id}`
      const response=await apiFetch(`/v1/editor/sessions/${detail.id}/media/${encodeURIComponent(id)}/probe`,{signal})
      await handleResponse(response)
      return {id,probe:await response.json() as MediaProbe}
    }))
    for(let index=0;index<results.length;index++){
      const result=results[index]!
      if(result.status==='rejected')throw new Error(`${batch[index]!.file_name}: ${result.reason instanceof Error?result.reason.message:String(result.reason)}`)
      probes[result.value.id]=result.value.probe
    }
  }
  return probes
}
