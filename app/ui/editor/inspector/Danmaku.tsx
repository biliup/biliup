'use client'
import {Section,Prop,NumInput,Toggle} from './controls'
import {useApp} from '@/app/lib/editor/state/store'
import {planDanmakuSettings} from '@/app/lib/editor/danmaku'
import type {Item,Project} from '@/app/lib/editor/core/timeline/types'
import type {DanmakuSettings} from '@/app/lib/renders'
import {editorText as t,useEditorLanguage} from '@/app/lib/editor/i18n'
import {useProjectEdit} from '../workspace/projectEdit'
import {editorGestureOrigin,type GestureOrigin} from '../workspace/pointerGesture'
import {usePlayback} from '../preview/playback'

type NumericSetting=Exclude<keyof DanmakuSettings,'enabled'|'font'|'segment_offsets'>
const numericPatch=(key:NumericSetting,value:number):Partial<DanmakuSettings>=>({[key]:key==='density'||key==='offset_ms'?Math.round(value):value})

export function DanmakuEditor({project,item:savedItem,locked}:{project:Project;item:Item;locked:boolean}){
 useEditorLanguage()
 const edit=useProjectEdit()
 const preview=usePlayback(state=>state.draft)
 const item=(preview?.base===project?preview.project.tracks.flatMap(track=>track.items).find(item=>item.id===savedItem.id):undefined)??savedItem
 const {snapshot,biliupCanEdit,connection}=useApp()
 const dir=snapshot?.dir,disabled=locked||!biliupCanEdit||connection!=='connected'
 const data=item.danmaku;if(!data)return null
 const save=(patch:Partial<DanmakuSettings>,origin:GestureOrigin|undefined=editorGestureOrigin())=>{
  if(disabled||!dir)return
  void edit.commit(current=>planDanmakuSettings(current,item.id,patch),origin)
 }
 const number=(label:string,key:NumericSetting,min:number,max:number,step=1)=><Prop key={key} label={t(label)}><NumInput testId={`danmaku-${key}`} ariaLabel={t(label)} value={data.settings[key]} min={min} max={max} step={step} disabled={disabled}
  onPreview={n=>edit.preview(project,planDanmakuSettings(project,item.id,numericPatch(key,n)))} onCancel={edit.cancel}
  onCommit={(n,origin)=>{if(n!==undefined)save(numericPatch(key,n),origin)}}/></Prop>
 return <Section title={t('Danmaku')} action={<Toggle testId="danmaku-enabled" label={t('Enabled')} value={data.settings.enabled} disabled={disabled} onChange={enabled=>save({enabled})}/>}>
 {number('Font size','font_size',8,200)}
 {number('Opacity','opacity',0,1,.05)}
 {number('Outline','outline',0,4,.1)}
 {number('Scroll duration','scroll_seconds',2,60,.1)}
 {number('Display area','display_area',.05,1,.05)}
 {number('Density','density',-1,1000)}
 {number('Time offset','offset_ms',-3600000,3600000)}
 </Section>
}
