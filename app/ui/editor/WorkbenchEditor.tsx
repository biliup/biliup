'use client'
import { useEffect, useLayoutEffect, useState } from 'react'
import { useMe } from '@/app/lib/use-me'
import { EditorSession } from '@/app/lib/editor/session'
import { projectEditorHref } from '@/app/lib/workbench'
import EditorWorkspace from './EditorWorkspace'
import {WorkbenchShell} from './workspace/WorkbenchShell'
import {openRecording} from '@/app/lib/editor/open-recording'
import {editorText as t,useEditorLanguage} from '@/app/lib/editor/i18n'

class SessionOwner {
  constructor(public canEdit:()=>boolean, public close:()=>void) {}
  readonly sessions = new Map<number,EditorSession>()
  private disposed=false
  private disposeTimer?: ReturnType<typeof setTimeout>
  update(canEdit:()=>boolean,close:()=>void) {this.canEdit=canEdit;this.close=close}
  retain() {clearTimeout(this.disposeTimer);this.disposed=false}
  release() {this.disposeTimer=setTimeout(()=>this.dispose(),0)}
  isDisposed() {return this.disposed}
  get(id:number) {
    if(this.disposed)throw new Error(t('This project cannot be edited.'))
    const existing=this.sessions.get(id);if(existing)return existing
    const session = new EditorSession(id,()=>this.canEdit(),()=>this.close())
    this.sessions.set(id,session);return session
  }
  dispose() {this.disposed=true;for(const session of this.sessions.values())session.dispose();this.sessions.clear()}
}

/** Live review links and projects share the same persisted editing surface. */
export default function WorkbenchEditor({sourceId, selectionId, projectId, active, initialT,initialEnd}: {
  sourceId: number|null; selectionId:number|null; projectId: number|null; active: boolean; initialT: number|null;initialEnd:number|null
}) {
  const {can} = useMe()
  const close = () => window.history.pushState(null,'','/workbench')
  const [owner] = useState(()=>new SessionOwner(()=>can('clip.edit'),close))
  useLayoutEffect(()=>{owner.update(()=>can('clip.edit'),close)})
  const key = projectId ? `project:${projectId}` : null
  const current = projectId ? owner.get(projectId) : undefined
  useEffect(()=>{
    owner.retain()
    const unload=(event:BeforeUnloadEvent)=>{if([...owner.sessions.values()].some(session=>session.hasPendingWork())){event.preventDefault();event.returnValue=''}}
    window.addEventListener('beforeunload',unload)
    return()=>{window.removeEventListener('beforeunload',unload);owner.release()}
  },[owner])
  if(!projectId&&(selectionId||sourceId))return <WorkbenchShell hasEditor><RecordingEntry key={selectionId?`selection:${selectionId}`:`session:${sourceId}`} owner={owner} sourceId={sourceId} selectionId={selectionId} at={initialT} allowed={can('clip.edit')}/></WorkbenchShell>
  return <WorkbenchShell project={current} hasEditor={!!current}>{active&&current&&<EditorWorkspace key={key} projectId={current.id} ownedSession={current} active onClose={close} initialT={initialT} initialEnd={initialEnd}/>}</WorkbenchShell>
}

function RecordingEntry({owner,sourceId,selectionId,at,allowed}:{owner:SessionOwner;sourceId:number|null;selectionId:number|null;at:number|null;allowed:boolean}){
 useEditorLanguage()
 const [error,setError]=useState(''),[retry,setRetry]=useState(0)
 useEffect(()=>{if(!allowed||!selectionId&&!sourceId)return;let active=true;setError('');void openRecording(owner,selectionId?{selectionId}:{sessionId:sourceId!,atMs:at??undefined}).then(result=>{
  if(active)window.history.replaceState(null,'',projectEditorHref(result.id,result.at,result.end))
 }).catch(error=>{if(active)setError(error instanceof Error?error.message:String(error))});return()=>{active=false}},[owner,sourceId,selectionId,at,allowed,retry])
 return <div className="splash" role="status">{!allowed?t('This project cannot be edited.'):error?<>{error}<button className="btn" onClick={()=>{setError('');setRetry(n=>n+1)}}>{t('Retry')}</button></>:t('Opening recording…')}</div>
}
