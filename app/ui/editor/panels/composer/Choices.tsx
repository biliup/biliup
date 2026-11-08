import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Composer controls adapted from Biliup, MIT. Models belong to configured APIs, never a desktop CLI.
import { useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from 'react'
import { Check, ChevronDown, Gauge, Search, Sparkles, Star, Eye, ShieldCheck } from 'lucide-react'
import { effortsFor, isAgentModelProvider, validAgentModel, useAgentPrefs, setAgentPrefs, toggleFavourite, type AgentPrefs } from '@/app/lib/editor/state/agentPrefs'
import type { AgentAccess } from '@/app/lib/editor/contracts'
import {actions,useApp} from '@/app/lib/editor/state/store'
import { editorEvents } from '@/app/lib/editor/environment'
import { isTextComposition, primaryModifier, usesCommandKey } from '@/app/lib/editor/keyboard'
import { displayKey } from '../../workspace/keymap'
export function ProviderMark({ size = 'sm' }: { provider: string; size?: 'sm' | 'md' }) {
  useEditorLanguage(); return <Sparkles className={`provider-mark ${size}`} aria-hidden /> }
export const ACCESS_INFO = {
 look: { name: '仅查看', short: '仅查看', get detail() { return editorText("读取工程，不修改时间轴"); }, icon: Eye },
 ask: { name: '先确认', short: '先确认', get detail() { return editorText("应用编辑前确认"); }, icon: ShieldCheck },
 auto: { name: '自动', short: '自动', get detail() { return editorText("允许编辑，可整轮撤销"); }, icon: Sparkles },
} satisfies Record<AgentAccess, { name: string; short: string; detail: string; icon: typeof Eye }>
export function ComposerChoices({ dir, provider }: { dir: string; provider: string }) {
  useEditorLanguage();
 const {agent}=useApp()
 const { prefs, favourites, models: remembered } = useAgentPrefs(dir,provider)
 const [open,setOpen] = useState<'model'|'effort'|'access'>()
 const [search,setSearch]=useState('')
 const root=useRef<HTMLDivElement>(null)
 const supported=isAgentModelProvider(provider)
 const configured=agent?.model
 const models=useMemo(()=>[...new Set([...favourites,...(validAgentModel(configured)?[configured]:[]),...remembered])],[favourites,configured,remembered])
 const query=search.trim()
 const visible=[{id:'',name:editorText('Default')},...models.map(id=>({id,name:id}))]
   .filter(model=>!query||model.name.toLowerCase().includes(query.toLowerCase()))
 if(validAgentModel(query)&&!models.includes(query))visible.push({id:query,name:editorText('Use {0}',{0:query})})
 const close=(restore=false)=>{
  if(restore&&open)root.current?.querySelector<HTMLButtonElement>(`button[data-choice="${open}"]`)?.focus({preventScroll:true})
  setOpen(undefined)
 }
 const pick=(patch:Partial<AgentPrefs>)=>{setAgentPrefs(dir,provider,patch);close(true)}
 const toggle=(menu:typeof open)=>{setSearch('');setOpen(open===menu?undefined:menu)}
 const moveFocus=(event:ReactKeyboardEvent)=>{
  if(isTextComposition(event.nativeEvent)||!['ArrowDown','ArrowUp'].includes(event.key))return
  const buttons=[...root.current!.querySelectorAll<HTMLButtonElement>('[data-choice-pick]')]
  if(!buttons.length)return
  event.preventDefault();event.stopPropagation()
  const at=buttons.findIndex(button=>button===document.activeElement)
  buttons[(at+(event.key==='ArrowDown'?1:at<0?0:-1)+buttons.length)%buttons.length]?.focus()
 }
 useEffect(()=>{
  const onKey=(event:KeyboardEvent)=>{
   if(!supported||isTextComposition(event)||event.repeat||!primaryModifier(event)||event.shiftKey||event.altKey||(usesCommandKey()?event.ctrlKey:event.metaKey))return
   if(!root.current?.closest('.composer')?.contains(document.activeElement))return
   const index=['1','2','3','4'].indexOf(event.key),model=index<0?undefined:models[index]
   if(!model)return
   event.preventDefault();event.stopImmediatePropagation();setAgentPrefs(dir,provider,{model});setOpen(undefined)
  }
  editorEvents.addEventListener('keydown',onKey,true)
  return()=>editorEvents.removeEventListener('keydown',onKey,true)
 },[dir,provider,models,supported])
 useEffect(()=>{
  if(!open)return
  const outside=(event:PointerEvent)=>{if(!root.current?.contains(event.target as Node))setOpen(undefined)}
  const escape=(event:KeyboardEvent)=>{if(event.key==='Escape'&&!isTextComposition(event)){event.preventDefault();event.stopImmediatePropagation();close(true)}}
  window.addEventListener('pointerdown',outside,true);editorEvents.addEventListener('keydown',escape,true)
  return()=>{window.removeEventListener('pointerdown',outside,true);editorEvents.removeEventListener('keydown',escape,true)}
 },[open])
 const AccessIcon = ACCESS_INFO[prefs.access].icon
 return <div className="composer-choices" ref={root} onKeyDown={event=>{if(event.key===' '||event.key==='Enter')event.stopPropagation()}} onKeyUp={event=>{if(event.key===' '||event.key==='Enter')event.stopPropagation()}}>
  <button className={`choice model${open==='model'?' open':''}`} data-choice="model" data-testid="agent-model" disabled={!supported} onClick={()=>toggle('model')} aria-expanded={open==='model'} aria-label={editorText('Model')} title={`${provider==='google'?'Google':provider==='openai'?'OpenAI':provider} · ${prefs.model||agent?.model||editorText('Default')}`}><Sparkles/><span className="choice-label">{prefs.model || agent?.model || editorText("配置模型")}</span><ChevronDown/></button>
  <button className={`choice${open==='effort'?' open':''}`} data-choice="effort" data-testid="agent-effort" disabled={!supported} aria-expanded={open==='effort'} aria-label={editorText('推理强度')} title={editorText('推理强度')} onClick={()=>toggle('effort')}><Gauge/><span className="choice-label">{editorText(prefs.effort??'Default')}</span></button>
  <button className={`choice${open==='access'?' open':''}`} data-choice="access" data-testid="agent-access" disabled={!supported} aria-expanded={open==='access'} aria-label={editorText(ACCESS_INFO[prefs.access].name)} title={editorText(ACCESS_INFO[prefs.access].detail)} onClick={()=>toggle('access')}><AccessIcon/></button>
  {open && <div className="choice-menu" role="dialog" aria-label={editorText(open==='model'?'Model':open==='effort'?'推理强度':'模型选项')} onKeyDown={event=>{moveFocus(event);event.stopPropagation()}} onKeyUp={event=>event.stopPropagation()}>
   {open==='model' && <>
    <label className="choice-search"><Search/><input autoFocus maxLength={100} value={search} aria-label={editorText('Search models or enter an ID')} placeholder={editorText('Search models or enter an ID')} onChange={event=>setSearch(event.target.value)} onKeyDown={event=>{
     if(isTextComposition(event.nativeEvent))return
     if(event.key==='Enter'&&visible[0]){event.preventDefault();event.stopPropagation();pick({model:visible[0].id||undefined})}
     else moveFocus(event)
    }}/></label>
    {visible.map(model=>{
     const selected=(prefs.model??'')===model.id,favourite=favourites.includes(model.id),quick=models.indexOf(model.id)
     const favouriteLabel=editorText(favourite?'Remove from favourites':'Add to favourites')
     return <div key={model.id} className={`choice-option model-option${selected?' active':''}`}>
      <button className="model-pick" data-choice-pick data-testid="agent-model-option" data-model={model.id} aria-pressed={selected} title={model.id||agent?.model||editorText('Default')} onClick={()=>pick({model:model.id||undefined})}>
       <span className="grow">{model.name}</span>{selected&&<Check/>}{quick>=0&&quick<4&&<kbd className="model-key">{displayKey(`⌘${quick+1}`)}</kbd>}
      </button>
      {model.id&&<button className={`icon-btn choice-star${favourite?' on':''}`} aria-pressed={favourite} title={favouriteLabel} aria-label={`${favouriteLabel} · ${model.id}`} onClick={()=>toggleFavourite(provider,model.id)}><Star/></button>}
     </div>
    })}
    {!visible.length&&<div className="choice-empty">{editorText('No model matches')}</div>}
    <button className="choice-option" data-choice-pick onClick={()=>{close();actions.openSettings('services')}}>{editorText('Configure model service')}</button>
   </>}
   {open==='effort' && [undefined,...effortsFor(provider)].map(effort=><button className={`choice-option${prefs.effort===effort?' active':''}`} data-choice-pick key={effort??'default'} aria-pressed={prefs.effort===effort} onClick={()=>pick({effort})}><span className="grow">{editorText(effort??'Default')}</span>{prefs.effort===effort&&<Check/>}</button>)}
   {open==='access' && (Object.keys(ACCESS_INFO) as AgentAccess[]).map(access => <button className={`choice-option${prefs.access===access?' active':''}`} data-choice-pick key={access} aria-pressed={prefs.access===access} title={editorText(ACCESS_INFO[access].detail)} onClick={()=>pick({access})}><span className="grow">{editorText(ACCESS_INFO[access].name)}</span>{prefs.access===access&&<Check/>}</button>)}
  </div>}
 </div>
}
