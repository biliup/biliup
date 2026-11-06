import {useContext,useEffect,useRef,useState,type ChangeEvent,type CompositionEvent,type KeyboardEvent} from 'react'
import {actions} from '@/app/lib/editor/state/store'
import {currentEditorStore,EditorStoreContext,projectOf} from '@/app/lib/editor/state/store/core'
import type {ProjectModel} from '@/app/lib/editor/state/project'
import {isTextComposition,primaryModifier} from '@/app/lib/editor/keyboard'
import {editorText} from '@/app/lib/editor/i18n'
import {dropDraft,showDraft,TYPING_PAUSE_MS,type PlanResult} from '../fields'
import type {PreviewDraft} from '../../preview/playback'

type Input=HTMLInputElement|HTMLTextAreaElement
interface Options {
 readonly project:ProjectModel
 readonly value:string
 readonly disabled:boolean
 readonly multiline:boolean
 readonly saveOnPause?:boolean
 readonly normalize?:(value:string)=>string
 readonly read:(project:ProjectModel)=>string|undefined
 readonly plan:(project:ProjectModel,value:string)=>PlanResult
 /** Lists have positional rows: changing the list invalidates an in-progress row edit. */
 readonly scope?:(project:ProjectModel)=>string
}
interface Burst {
 readonly dir:string
 readonly read:Options['read']
 readonly plan:Options['plan']
 readonly scope:Options['scope']
 readonly normalize:Options['normalize']
 expected:string|undefined
 context:string|undefined
}

/** Text, component fields and list rows share one preview and queued-save lifecycle. */
export function useTextDraft(options:Options){
 const owner=useContext(EditorStoreContext)
 const latest=useRef(options);latest.current=options
 const [value,setValue]=useState(options.value)
 const text=useRef(options.value),confirmed=useRef(options.value)
 const burst=useRef<Burst|undefined>(undefined)
 const draft=useRef<PreviewDraft|undefined>(undefined)
 const timer=useRef<ReturnType<typeof setTimeout>|undefined>(undefined)
 const composing=useRef(false),blurred=useRef(false),mounted=useRef(true)
 const saving=useRef(false),flushAgain=useRef(false)
 const clearTimer=()=>{clearTimeout(timer.current);timer.current=undefined}
 const drop=()=>{dropDraft(draft.current);draft.current=undefined}
 const display=(next:string)=>{text.current=next;if(mounted.current)setValue(next)}
 const current=(from?:Burst)=>{
  const state=owner?.getSnapshot(),snapshot=state?.snapshot
  return owner===currentEditorStore()&&state?.biliupCanEdit&&state.connection==='connected'&&snapshot&&(!from||snapshot.dir===from.dir)?snapshot:undefined
 }
 const preview=()=>{
  const from=burst.current,snapshot=from&&current(from)
  if(!from||!snapshot||!mounted.current||latest.current.disabled)return drop()
  const project=projectOf(snapshot)
  // An unrelated edit can refresh the base; a changed field or list must not be painted over.
  if(from.read(project)!==from.expected||(from.scope&&from.scope(project)!==from.context))return drop()
  const next=showDraft(project,from.plan(project,text.current))
  if(next)draft.current=next;else drop()
 }
 const reset=()=>{
  clearTimer();burst.current=undefined;flushAgain.current=false;drop()
  const snapshot=current()
  const saved=snapshot?latest.current.read(projectOf(snapshot)):undefined
  confirmed.current=saved??latest.current.value
  display(confirmed.current)
 }
 const flush=()=>{
  clearTimer()
  if(composing.current)return
  const from=burst.current
  if(!from)return
  if(saving.current){flushAgain.current=true;return}
  if(!current(from)||latest.current.disabled){drop();return}
  const submitted=from.normalize?.(confirmed.current)??confirmed.current
  if(submitted===from.expected){burst.current=undefined;drop();confirmed.current=submitted;display(submitted);return}
  const pending=draft.current;draft.current=undefined
  saving.current=true
  const finish=(ok:boolean)=>{
   saving.current=false;dropDraft(pending)
   const waiting=flushAgain.current;flushAgain.current=false
   if(burst.current!==from){
    if(waiting&&burst.current)flush()
    else if(!burst.current&&current(from)){
     const snapshot=owner?.getSnapshot().snapshot
     const saved=snapshot?latest.current.read(projectOf(snapshot)):undefined
     if(saved!==undefined){confirmed.current=saved;display(saved)}
    }
    return
   }
   const snapshot=current(from)
   if(!snapshot){drop();return}
   if(!ok)return // Keep the editable draft; the existing notice explains why it was refused.
   const project=projectOf(snapshot),saved=from.read(project)
   if(saved!==submitted)return
   from.expected=submitted;from.context=from.scope?.(project)
   if(!composing.current&&(from.normalize?.(text.current)??text.current)===submitted){burst.current=undefined;drop();confirmed.current=saved;display(saved)}
   else if(!composing.current){preview();if(waiting||!mounted.current)flush()}
  }
  void actions.itemEdit(project=>{
   if(from.expected===undefined||from.read(project)!==from.expected||(from.scope&&from.scope(project)!==from.context))return {problem:editorText('This text changed while you were typing. Copy your draft or press Escape to reload the saved text.')}
   return from.plan(project,submitted)
  },{dir:from.dir}).then(finish,()=>finish(false))
 }
 const schedule=()=>{
  clearTimer()
  if(!composing.current&&latest.current.saveOnPause!==false)timer.current=setTimeout(flush,TYPING_PAUSE_MS)
 }
 const change=(next:string)=>{
  const snapshot=current()
  if(!snapshot||latest.current.disabled)return
  if(!burst.current){
   const {project,read,plan,scope,normalize}=latest.current
   burst.current={dir:snapshot.dir,read,plan,scope,normalize,expected:read(project),context:scope?.(project)}
  }
  display(next)
  if(!composing.current)confirmed.current=next
  preview();schedule()
 }
 useEffect(()=>{if(!burst.current&&!composing.current){confirmed.current=options.value;display(options.value)}},[options.value])
 useEffect(()=>{if(options.disabled)reset()},[options.disabled])
 useEffect(()=>{
  mounted.current=true
  return ()=>{
   mounted.current=false;clearTimer()
   // Candidate text belongs to the IME. Only the last confirmed text can be saved on exit.
   composing.current=false;text.current=confirmed.current
   flush();drop()
  }
 },[])
 const input={
  value,
  onChange:(event:ChangeEvent<Input>)=>change(event.currentTarget.value),
  onFocus:()=>{blurred.current=false},
  onBlur:()=>{blurred.current=true;flush()},
  onCompositionStart:()=>{composing.current=true;clearTimer()},
  onCompositionEnd:(event:CompositionEvent<Input>)=>{
   composing.current=false;change(event.currentTarget.value)
   if(blurred.current)flush()
  },
  onKeyDown:(event:KeyboardEvent<Input>)=>{
   if(composing.current||isTextComposition(event.nativeEvent))return
   if(event.key==='Escape'){
    event.preventDefault();event.stopPropagation();reset();event.currentTarget.blur()
   }else if(event.key==='Enter'&&(!latest.current.multiline||primaryModifier(event))){
    event.preventDefault();event.stopPropagation();event.currentTarget.blur()
   }
  },
 }
 return {input,change,commit:flush,isComposing:()=>composing.current}
}
