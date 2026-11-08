import {denoiseMediaIds, duckMediaIds, soundMedia, trackAudible, spanOf, layout, type Project} from '../core'
import {propMediaIds} from '../core/timeline/mediaPlan'
import {renderWindow,type FrameRange} from './range'

/** Muted/soloed-out buses do not require a noise profile or trigger ducking work. */
export function envelopeMediaIds(project:Project){
  const heard={...project,tracks:project.tracks.map(track=>trackAudible(project,track)?track:{...track,mixer:{...track.mixer,effects:[]}})}
  return [...new Set([...duckMediaIds(heard),...denoiseMediaIds(heard)])]
}

/** Select before resolving files: the library is not the set of render dependencies. */
export function selectRenderInputs(project:Project,range?:FrameRange|null,purpose:'export'|'audio'|'picture'='export'){
  renderWindow(project,range)
  const picture=new Set(purpose==='audio'?[]:pictureItemIds(project,range))
  // Ducking and measured noise floors depend on the original edited track, including
  // speech just outside the selected range. Do not recalculate them on trimmed clips.
  const media=new Set(purpose==='picture'?[]:envelopeMediaIds(project))
  let graphics=false
  for(const track of project.tracks){
    const audible=purpose!=='picture'&&trackAudible(project,track)
    for(const item of track.items){
      if(audible){
        // A bus's state can carry into the range, and lookahead can read after it.
        // Keep the original audio inputs; the final mix is trimmed after processing.
        const sound=soundMedia(project,item)
        if(sound)media.add(sound.id)
      }
      if(!picture.has(item.id))continue
      if((item.type==='clip'||item.type==='danmaku')&&item.media)media.add(item.media)
      if(item.type==='graphic')graphics=true
      if(item.props)propMediaIds(item.props,media)
      for(const effect of item.effects??[])if(!effect.bypass)propMediaIds(effect.params,media)
    }
  }
  // Keep track lengths and mixer semantics, but do not plan pictures for an audio measurement.
  const output=purpose==='audio'?{...project,tracks:project.tracks.map(track=>({...track,hidden:true}))}:project
  return {project:output,media:[...media],graphics}
}

/** Original item identities used by picture preparation, including transition handles. */
export function pictureItemIds(project:Project,range?:FrameRange|null):string[]{
  const window=renderWindow(project,range),{starts}=layout(project)
  return project.tracks.flatMap(track=>track.hidden||track.kind==='audio'?[]:track.items.flatMap((item,index)=>{
    if(item.type==='danmaku'&&!item.danmaku?.settings.enabled)return []
    if(item.type==='captions')return [item.id]
    const span=spanOf(track,index,starts.get(item.id)??0)
    return Math.min(window.to,Math.ceil(span.to))>Math.max(window.from,Math.ceil(span.from))?[item.id]:[]
  }))
}
