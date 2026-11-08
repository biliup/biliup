import type {Item,Project,Op} from './core/timeline/types'
import {layout} from './core/timeline/timeline'
import {editorText} from './i18n'
import {defaultRecipe,type DanmakuSettings} from '../renders'

/** An independent text layer whose source clock follows a video until explicitly retimed. */
export function danmakuLayer(project:Project,clip:Item,settings:DanmakuSettings,id:string):Item {
 const probe=project.media.find(m=>m.id===clip.media)?.probe
 return {id,type:'danmaku',name:editorText('Danmaku'),media:clip.media,start:layout(project).starts.get(clip.id)??0,duration:clip.duration,in:clip.in??0,
  linkedTo:clip.id,speed:clip.speed,transform:clip.transform,loop:clip.loop,hold:clip.hold,
  danmaku:{settings,width:probe?.width??project.canvas.width,height:probe?.height??project.canvas.height}}
}
export function planDanmaku(project:Project,id:string):{ops:Op[];summary:string}|{problem:string} {
 const clip=project.tracks.flatMap(t=>t.items).find(i=>i.id===id)
 if(!clip||clip.type!=='clip'||project.media.find(m=>m.id===clip.media)?.kind!=='video')return {problem:'Select a video clip in the timeline.'}
 if(clip.locked||project.tracks.some(track=>track.locked&&track.items.some(item=>item.id===id)))return {problem:'That clip is locked: unlock it to change its effects.'}
 if(project.tracks.some(t=>t.items.some(i=>i.type==='danmaku'&&i.linkedTo===id)))return {ops:[],summary:''}
 const key=crypto.randomUUID()
 return {ops:[{op:'add_track',index:0,track:{id:`danmaku-${key}`,name:editorText('Danmaku'),kind:'text',items:[]}},{op:'insert_item',track:`danmaku-${key}`,item:danmakuLayer(project,clip,{...defaultRecipe().danmaku,enabled:true},`comments-${key}`)}],summary:'Add danmaku layer'}
}

/** Merge only the changed controls into the item read from the edit queue. */
export function planDanmakuSettings(project:Project,id:string,patch:Partial<DanmakuSettings>):{ops:Op[];summary:string}|{problem:string} {
 for(const track of project.tracks){
  const item=track.items.find(item=>item.id===id)
  if(!item)continue
  if(item.type!=='danmaku'||!item.danmaku)return {problem:'The danmaku layer was removed.'}
  if(track.locked||item.locked)return {problem:'That clip is locked: unlock it to change its effects.'}
  return {ops:[{op:'update_item',item:id,set:{danmaku:{...item.danmaku,settings:{...item.danmaku.settings,...patch}}}}],summary:'Edit danmaku'}
 }
 return {problem:'The danmaku layer was removed.'}
}
