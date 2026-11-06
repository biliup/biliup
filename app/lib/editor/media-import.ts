import type {Media,Project} from './core/timeline/types'
import {takenIds,uniqueIn} from './core/timeline/arrange/place'
import {mediaForCanvas} from './project'

/** File identity is its registered path, while a timeline media ID survives file replacement. */
export function prepareImportedMedia(project:Project,incoming:Media,durationMs:number|null):{media:Media;existing:boolean}{
  const existing=project.media.find(media=>media.path===incoming.path&&media.kind===incoming.kind)
  if(existing)return {media:existing,existing:true}
  const taken=takenIds(project)
  const id=taken.has(incoming.id)?uniqueIn(incoming.id,taken):incoming.id
  return {media:{...mediaForCanvas(incoming,durationMs,project.canvas),id},existing:false}
}
