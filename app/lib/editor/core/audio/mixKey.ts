import {audioSpan, soundMedia} from './audio'
import {layout} from '../timeline/timeline'
import type {Project} from '../timeline/types'

const keys=new WeakMap<Project,string>()
/** The mix identity excludes picture styling, titles and display names. */
export function audioMixKey(project:Project):string {
  const cached=keys.get(project)
  if(cached!==undefined)return cached
  const {starts,duration}=layout(project)
  const key=JSON.stringify([project.canvas.fps,duration,project.master??null,project.tracks.map(track=>[
    track.id,track.kind,track.role,track.mixer??null,track.items.flatMap((item,index)=>{
      const media=soundMedia(project,item)
      if(!media)return []
      const start=starts.get(item.id)??0
      return [[item.id,item.type,start,audioSpan(project,track,index,start),item.duration,item.in??0,
        item.speed??1,item.keepPitch!==false,!!item.loop,!!item.hold,item.gainDb??0,item.gainKeys??[],
        item.fadeIn??0,item.fadeOut??0,media.id,media.path,media.probe,media.source]]
    }),
  ])])
  keys.set(project,key)
  return key
}
