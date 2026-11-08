import {useContext,useEffect,useRef,useState} from 'react'
import {EditorStoreContext} from '@/app/lib/editor/state/store/core'
import {beginPointerGesture,editorGestureOrigin,ownsEditorGesture,type GestureOrigin} from '../workspace/pointerGesture'

interface Options<T> {
 value:T
 disabled?:boolean
 onPreview?:(value:T)=>void
 onCommit:(value:T,origin?:GestureOrigin)=>void
 onCancel?:()=>void
}
interface Active<T> {initial:T;value:T;origin:GestureOrigin|undefined;callbacks:Options<T>;release:()=>void}

/** One control gesture, including keyboard input and unsaved new-project fields. */
export function useControlGesture<T>(options:Options<T>){
 const owner=useContext(EditorStoreContext)
 const latest=useRef(options);latest.current=options
 const active=useRef<Active<T>|undefined>(undefined)
 const stop=useRef<(()=>void)|undefined>(undefined)
 const mounted=useRef(true)
 const [local,setLocal]=useState<{value:T}|undefined>(undefined)
 const available=()=>!latest.current.disabled&&(!active.current?.origin||ownsEditorGesture(active.current.origin))
 const finish=(save:boolean)=>{
  const gesture=active.current
  if(!gesture)return
  const accept=save&&available()
  active.current=undefined
  const end=stop.current;stop.current=undefined
  end?.();gesture.release()
  if(mounted.current)setLocal(undefined)
  if(accept&&!Object.is(gesture.value,gesture.initial))gesture.callbacks.onCommit(gesture.value,gesture.origin)
  else gesture.callbacks.onCancel?.()
 }
 const cancel=()=>finish(false)
 const begin=()=>{
  if(active.current)return available()
  if(latest.current.disabled)return false
  const origin=owner?editorGestureOrigin():undefined
  if(owner&&origin?.owner!==owner)return false
  const visibility=()=>{if(document.hidden)cancel()}
  const release=origin?.owner.subscribe(()=>{if(!available())cancel()})
  window.addEventListener('blur',cancel)
  document.addEventListener('visibilitychange',visibility)
  active.current={initial:latest.current.value,value:latest.current.value,origin,callbacks:latest.current,release:()=>{
   release?.();window.removeEventListener('blur',cancel);document.removeEventListener('visibilitychange',visibility)
  }}
  return true
 }
 const change=(value:T,preview=true)=>{
  if(!begin()){cancel();return}
  const gesture=active.current!
  gesture.value=value
  setLocal({value})
  if(preview)gesture.callbacks.onPreview?.(value)
 }
 const pointer=(event:{pointerId:number;currentTarget:Element},move:(event:PointerEvent)=>void=()=>{})=>{
  cancel()
  if(!begin())return false
  stop.current=beginPointerGesture(event,{move,commit:()=>finish(true),cancel},available)
  return true
 }
 useEffect(()=>{if(options.disabled)cancel()},[options.disabled])
 useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;cancel()}},[])
 return {local,begin,change,cancel,commit:()=>finish(true),pointer,isPointer:()=>!!stop.current,isActive:()=>!!active.current}
}
