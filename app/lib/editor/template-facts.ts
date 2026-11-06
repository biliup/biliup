// Adapted from Uncut apps/server/src/templates/templates.ts, MIT; licenses/uncut/LICENSE.
import type {BrandKit,BrandState} from './contracts'
import type {Project,Media} from './core/timeline/types'
import type {TemplateFacts} from './core/templates'
import type {BeatsResult} from './agent/analysisTypes'
import {projectDuration} from './core/timeline/rules'
import {durationSeconds} from './core/skills/skills'
import {downbeatsOf} from './analysis/beatGrid'

function kitFacts(kit:BrandKit,files:Readonly<Record<string,string>>):NonNullable<TemplateFacts['kit']>{
  const logos=[...new Set(Object.values(kit.logos).filter((name):name is string=>!!name))]
  const used=new Set([...logos,...kit.fonts.flatMap(font=>font.file?[font.file]:[])])
  const typefaces:Array<{family:string;roles:string[];file?:string}>=[]
  for(const font of kit.fonts){
    const face=typefaces.find(face=>face.family===font.family&&face.file===font.file)
    if(face){if(font.role&&!face.roles.includes(font.role))face.roles.push(font.role)}
    else typefaces.push({family:font.family,roles:font.role?[font.role]:[],...(font.file?{file:font.file}:{})})
  }
  return {
    name:kit.name,logos,logoRoles:kit.logos,typefaces,
    fonts:[...new Set(kit.fonts.map(font=>`${font.family} (${font.file??'not in the kit as a file'})`))],
    files:Object.keys(files).filter(name=>!used.has(name)).sort(),
    screens:kit.examples.filter(name=>Object.hasOwn(files,name)),
    colors:kit.colors,about:kit.about,tone:kit.tone,taglines:kit.taglines,urls:kit.urls,dos:kit.dos,donts:kit.donts,
  }
}

/** Read only already-cached beat facts; applying a template never starts analysis or generation. */
export async function templateProjectFacts(project:Project,brand:BrandState,briefDuration:string|undefined,readBeats:(media:Media)=>Promise<BeatsResult|undefined>):Promise<TemplateFacts>{
  const fps=project.canvas.fps
  const secondsOf=(media:Media)=>media.probe?.frames===undefined?undefined:media.probe.frames/fps
  const audio=project.media.filter(media=>media.kind==='audio').sort((a,b)=>(secondsOf(b)??0)-(secondsOf(a)??0)).slice(0,6)
  const songs=await Promise.all(audio.map(async media=>({media,beats:await readBeats(media)})))
  const song=songs.find(song=>song.beats?.hasAudio&&typeof song.beats.bpm==='number'&&Number.isFinite(song.beats.bpm)&&song.beats.bpm>0&&(song.beats.beats?.length??0)>0)??songs[0]
  const songSeconds=song&&secondsOf(song.media),briefSeconds=durationSeconds(briefDuration),timelineSeconds=projectDuration(project)/fps
  const length:TemplateFacts['length']=briefSeconds?{seconds:briefSeconds,from:'brief'}:timelineSeconds>0?{seconds:timelineSeconds,from:'timeline'}:songSeconds?{seconds:songSeconds,from:'song'}:undefined
  const kit=brand.kits.find(kit=>kit.id===brand.active.kit)
  let music:TemplateFacts['music']
  if(song){
    const beats=song.beats?.hasAudio?song.beats:undefined
    const beatTimes=beats?.beats??[]
    const steady=!!beats&&typeof beats.bpm==='number'&&Number.isFinite(beats.bpm)&&beats.bpm>0&&beatTimes.length>0
    const grid=steady?downbeatsOf(beats!):undefined,span=length?.seconds??songSeconds
    const firstDownbeat=grid?beatTimes[grid.phase]:undefined
    music={name:song.media.name,seconds:songSeconds,
      ...(beats?{bpm:steady?beats.bpm:null,confidence:beats.confidence}:{}),
      ...(steady?{beatCount:beatTimes.filter(time=>span===undefined||time<span).length,beatSpan:span}:{}),
      ...(grid?.drop?{drop:{seconds:grid.drop.seconds,beat:grid.drop.beat}}:{}),
      ...(firstDownbeat!==undefined?{downbeat:{seconds:firstDownbeat,beat:grid!.phase}}:{}),
    }
  }
  return {project:project.name,canvas:project.canvas,
    media:project.media.map(media=>({name:media.name,kind:media.kind,seconds:secondsOf(media),fps:media.probe?.fps,width:media.probe?.width,height:media.probe?.height,hasAudio:media.probe?.hasAudio})),
    ...(length?{length}:{}),...(music?{music}:{}),...(kit?{kit:kitFacts(kit,brand.files[kit.id]??{})}:{}),
  }
}
