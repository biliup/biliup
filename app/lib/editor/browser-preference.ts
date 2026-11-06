import {useSyncExternalStore} from 'react'

/** Browser preferences are shared by editor instances, never stored in a project document. */
export interface BrowserPreference<T> {
  get:()=>T
  initial:()=>T
  set:(value:T)=>void
  subscribe:(listener:()=>void)=>()=>void
}

const readers=new Map<string,()=>void>()
let watching=0

export function browserPreference<T>(name:string,initial:T,accepts:(value:unknown)=>value is T):BrowserPreference<T>{
  const key=`biliup.editor.${name}`
  let current=initial,memoryOnly=false
  const listeners=new Set<()=>void>()
  const assign=(next:T)=>{
    if(JSON.stringify(current)===JSON.stringify(next))return
    current=next
    for(const listener of listeners)listener()
  }
  const read=()=>{
    if(typeof window==='undefined'||memoryOnly)return
    let raw:string|null
    try{raw=window.localStorage.getItem(key)}catch{return}
    let next=initial
    if(raw!==null){try{const parsed:unknown=JSON.parse(raw);if(accepts(parsed))next=parsed}catch{}}
    assign(next)
  }
  readers.set(key,read)
  read()
  return {
    get:()=>current,
    initial:()=>initial,
    set:next=>{
      if(!accepts(next))return
      assign(next)
      try{
        if(next===undefined||JSON.stringify(next)===JSON.stringify(initial))window.localStorage.removeItem(key)
        else window.localStorage.setItem(key,JSON.stringify(next))
        memoryOnly=false
      }catch{memoryOnly=true}
    },
    subscribe:listener=>{listeners.add(listener);return ()=>{listeners.delete(listener)}},
  }
}

export const useBrowserPreference=<T,>(preference:BrowserPreference<T>)=>useSyncExternalStore(preference.subscribe,preference.get,preference.initial)

const storageChanged=(event:StorageEvent)=>{
  try{if(event.storageArea!==window.localStorage)return}catch{return}
  if(event.key===null)for(const read of readers.values())read()
  else readers.get(event.key)?.()
}

/** One storage listener for every mounted editor; released when the last editor closes. */
export function watchEditorPreferences(){
  if(watching++===0){
    for(const read of readers.values())read()
    window.addEventListener('storage',storageChanged)
  }
  let released=false
  return ()=>{
    if(released)return
    released=true
    if(--watching===0)window.removeEventListener('storage',storageChanged)
  }
}

export const booleanPreference=(name:string)=>browserPreference(name,true,(value):value is boolean=>typeof value==='boolean')
