// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// At-target preview shares export measurement; results belong to the open editor instance.
import {useContext,useEffect} from 'react'
import {projectTargetLufs} from '@/app/lib/editor/core/audio/effects'
import {audioMixKey} from '@/app/lib/editor/core/audio/mixKey'
import type {SpeechEnvelope} from '@/app/lib/editor/core/text/transcript'
import type {ProjectModel} from '@/app/lib/editor/state/project'
import {currentCommandService} from '@/app/lib/editor/state/store/server'
import {currentEditorStore,EditorStoreContext,useApp} from '@/app/lib/editor/state/store/core'
import {booleanPreference,useBrowserPreference} from '@/app/lib/editor/browser-preference'

const choice=booleanPreference('preview.atTarget')
export const setPreviewAtTarget=(atTarget:boolean)=>choice.set(atTarget)

export function useLoudnessPreview(){
  const editor=useContext(EditorStoreContext)
  const {loudnessPreview:state,biliupDirty,biliupSaveError,connection}=useApp()
  const atTarget=useBrowserPreference(choice)
  return {
    atTarget,measuring:!!state?.request,step:state?.request?.step,
    fresh:!!state?.result&&state.result.key===state.desiredKey,
    gainDb:state?.result?.gainDb,lufs:state?.result?.lufs,targetLufs:state?.result?.targetLufs,
    problem:state?.problem,waitingForSave:!!biliupDirty||!!biliupSaveError,online:connection==='connected',
    retry:()=>{if(editor){const now=editor.getSnapshot().loudnessPreview;editor.set({loudnessPreview:{...now,result:undefined,problem:undefined,retry:(now?.retry??0)+1}})}},
  }
}

/** Keep the last measured gain during edits, but never borrow another editor's or history's mix. */
export function previewLoudnessGain(project:ProjectModel):number {
  const state=currentEditorStore()?.getSnapshot()
  const current=state?.snapshot?.project as ProjectModel|undefined
  const gain=state?.loudnessPreview?.result?.gainDb
  return choice.get()&&current&&gain!==undefined&&audioMixKey(current)===audioMixKey(project)?10**(gain/20):1
}

const envelopes=new WeakMap<ReadonlyMap<string,SpeechEnvelope|null>,number>()
let nextEnvelope=0,nextRequest=0
function envelopeKey(value:ReadonlyMap<string,SpeechEnvelope|null>){
  let key=envelopes.get(value)
  if(key===undefined){key=++nextEnvelope;envelopes.set(value,key)}
  return key
}

/** Delay until editing settles; cancel the actual server work on every invalidation. */
export function useLoudnessMeasuring(project:ProjectModel,analysis:ReadonlyMap<string,SpeechEnvelope|null>){
  const editor=useContext(EditorStoreContext),service=currentCommandService()
  const {connection,snapshot,biliupDirty,biliupSaveError,loudnessPreview}=useApp()
  const atTarget=useBrowserPreference(choice),target=projectTargetLufs(project)
  const key=`${target}|${envelopeKey(analysis)}|${audioMixKey(project)}`
  useEffect(()=>{
    if(!editor||!service||currentEditorStore()!==editor)return
    const state=editor.getSnapshot().loudnessPreview
    editor.set({loudnessPreview:{...state,desiredKey:key}})
    if(connection!=='connected'||!atTarget||biliupDirty||biliupSaveError||state?.result?.key===key)return
    const requestId=`loudness-${++nextRequest}`
    const timer=setTimeout(()=>{
      if(currentEditorStore()===editor)void service('export.loudness',{requestId,key,targetLufs:target})
    },1500)
    return ()=>{clearTimeout(timer);void service('export.loudnessCancel',{requestId})}
  },[editor,service,connection,atTarget,key,biliupDirty,biliupSaveError,snapshot?.revision,loudnessPreview?.retry,target])
}
