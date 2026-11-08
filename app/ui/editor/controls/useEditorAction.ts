'use client'
import {useContext,useEffect,useMemo,useState} from 'react'
import {autoClipError} from '@/app/lib/auto-clip'
import {currentEditorStore,EditorStoreContext,useApp} from '@/app/lib/editor/state/store/core'

/** One pending action per mounted editor control, bound to its editor instance. */
export function useEditorAction(){
  const owner=useContext(EditorStoreContext),app=useApp(),work=useMemo(()=>({mounted:false,pending:false}),[owner])
  const [busy,setBusy]=useState(false),[error,setError]=useState('')
  useEffect(()=>{work.mounted=true;setBusy(false);setError('');return()=>{work.mounted=false}},[work])
  const isCurrent=()=>work.mounted&&!!owner&&currentEditorStore()===owner
  const editable=app.connection==='connected'&&!!app.biliupCanEdit
  const run=async<T,>(action:(current:()=>boolean)=>Promise<T>,options:{readOnly?:boolean;onSuccess?:(value:T)=>void}={})=>{
    if(work.pending||!owner||!isCurrent())return
    const state=owner.getSnapshot()
    if(!options.readOnly&&(state.connection!=='connected'||!state.biliupCanEdit))return
    work.pending=true;setBusy(true);setError('')
    try{const value=await action(isCurrent);if(isCurrent()){options.onSuccess?.(value);return value}}
    catch(cause){if(isCurrent())setError(autoClipError(cause))}
    finally{work.pending=false;if(work.mounted)setBusy(false)}
  }
  return {busy,error,setError,editable,isCurrent,run}
}
