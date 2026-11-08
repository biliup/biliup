import {currentEditorStore} from '@/app/lib/editor/state/store/core'
import {getPlayback} from './playback'
import {panels} from '../workspace/workspaceLayout'

/** Delayed imports may reveal their result only while the user's editor focus is unchanged. */
export function captureEditorFocus(){
  const owner=currentEditorStore(),before=getPlayback(),workspace=panels.get()
  return ()=>{
    const now=getPlayback()
    return !!owner&&currentEditorStore()===owner&&!before.playing&&!now.playing
      &&before.draft?.kind!=='history'&&now.draft?.kind!=='history'
      &&now.frame===before.frame&&now.rate===before.rate&&now.selected===before.selected&&now.cut===before.cut
      &&now.gainKey===before.gainKey&&now.markIn===before.markIn&&now.markOut===before.markOut&&now.draft===before.draft
      &&now.bottom===before.bottom&&now.zoom===before.zoom&&panels.get()===workspace
      &&now.also.length===before.also.length&&now.also.every((id,index)=>id===before.also[index])
  }
}
