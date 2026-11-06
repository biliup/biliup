'use client'
import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { useMe } from '@/app/lib/use-me'
import { EditorSession } from '@/app/lib/editor/session'
import { bindEditorRoot, editorEvents } from '@/app/lib/editor/environment'
import { activateEditorStore, EditorStoreContext, useApp } from '@/app/lib/editor/state/store/core'
import { attachCommands } from '@/app/lib/editor/state/store/server'
import { onKeyDown } from '@/app/lib/editor/state/store/keys'
import { loadFonts,syncKitFonts } from './preview/fonts'
import {loadRenderAssets,renderAssetsKey} from '@/app/lib/editor/render-assets'
import { getPlayback, getPreviewPlayback, playback, previewPlayback } from './preview/playback'
import { layoutOf } from '@/app/lib/editor/state/project'
import { cancelEditorDragFor } from './timeline/dnd'
import { Editor } from './workspace/Editor'
import { restoreWorkspaceLayout } from './workspace/workspaceLayout'
import {watchEditorPreferences} from '@/app/lib/editor/browser-preference'
import { SaveRecovery } from './workspace/SaveRecovery'
import { installScrollbars } from './workspace/scrollbars'
import { Settings } from './settings/Settings'
import { ShortcutsSheet } from './settings/Shortcuts'
import { installLeave, LeaveDialog } from '@/app/lib/editor/state/leave'
import {readEditorView,saveEditorView,type EditorView} from '@/app/lib/editor/workspace-memory'
import type {Project} from '@/app/lib/editor/core/timeline/types'
import styles from './workspace.module.css'

type WorkspaceSession = EditorSession
const views = new WeakMap<WorkspaceSession, EditorView>()
function focusRequestedRange(project:Project,at:number|null|undefined,end:number|null|undefined){
  if(at==null)return
  const timing=layoutOf(project),from=Math.min(timing.end,Math.max(0,Math.round(at*project.canvas.fps/1000)))
  const to=end==null?undefined:Math.min(timing.end,Math.round(end*project.canvas.fps/1000))
  playback.pause();playback.setProject(timing.frames,project.canvas.fps,timing.end);playback.hoverMedia(undefined);playback.seek(from)
  if(to!==undefined&&to>from){
    const ids=(project.tracks.find(track=>track.role==='main')?.items??[]).filter(item=>{
      const start=timing.starts.get(item.id)??0
      return start<to&&start+item.duration>from
    }).map(item=>item.id)
    playback.selectMany(ids);playback.setMark('in',from);playback.setMark('out',to)
  }else{playback.selectMany([]);playback.setMark('in',undefined);playback.setMark('out',undefined)}
}
export default function EditorWorkspace({ projectId, active, onClose, ownedSession, initialT, initialEnd }: { projectId: number; active: boolean; onClose: () => void; ownedSession?: WorkspaceSession; initialT?: number | null;initialEnd?:number|null }) {
  const language = useEditorLanguage();
  const root = useRef<HTMLDivElement>(null)
  const { can } = useMe()
  const options = useRef({ can, onClose,initialT,initialEnd })
  useLayoutEffect(() => { options.current = { can, onClose,initialT,initialEnd } }, [can, onClose,initialT,initialEnd])
  const [session, setSession] = useState<WorkspaceSession>()
  useLayoutEffect(() => {
    const element = root.current!
    restoreWorkspaceLayout()
    const releasePreferences=watchEditorPreferences()
    const instance = ownedSession ?? new EditorSession(projectId, () => options.current.can('clip.edit'), () => options.current.onClose())
    const releaseRoot = bindEditorRoot(element)
    const releaseScrollbars = installScrollbars()
    const releaseStore = activateEditorStore(instance.store)
    const releaseCommands = attachCommands(instance.run, instance.importFiles, instance.uploadOne)
    const releaseLeave = installLeave()
    setSession(instance)
    let disposed = false
    let opened=false
    const viewKey=String(projectId)
    void Promise.all([instance.store.getSnapshot().snapshot ? Promise.resolve() : instance.open(), loadFonts()]).then(() => {
      if (!disposed) { const project = instance.store.getSnapshot().snapshot?.project as import('@/app/lib/editor/core/timeline/types').Project | undefined
        if (project) { const layout = layoutOf(project); playback.reset(); playback.setProject(layout.frames, project.canvas.fps, layout.end)
          const previous = views.get(instance)??readEditorView(viewKey)
          if(previous) {
            const frame=(value:number|undefined)=>value===undefined?undefined:value*project.canvas.fps/previous.fps
            playback.seek(frame(previous.frame)!);playback.setMark('in',frame(previous.markIn));playback.setMark('out',frame(previous.markOut))
          }
          focusRequestedRange(project,options.current.initialT,options.current.initialEnd)
          opened=true
        }
      }
    }).catch(error => { if (!disposed) instance.store.set({ connection: 'failed', connectionMessage: error instanceof Error ? error.message : String(error) }) })
    const remember=()=>{
      if(!opened)return
      const {frame,fps,markIn,markOut}=getPlayback(),view={frame,fps,markIn,markOut}
      views.set(instance,view);saveEditorView(viewKey,view)
    }
    const beforeUnload = (event: BeforeUnloadEvent) => {remember(); if (instance.hasPendingWork()) { event.preventDefault(); event.returnValue = '' } }
    const timer=setInterval(remember,1500)
    window.addEventListener('beforeunload', beforeUnload)
    return () => {
      cancelEditorDragFor(instance.store)
      disposed = true;remember();clearInterval(timer);if(!ownedSession)instance.dispose(); playback.pause(); playback.reset()
      window.removeEventListener('beforeunload', beforeUnload)
      releaseLeave(); releaseScrollbars?.(); releaseCommands(); releaseStore(); releaseRoot(); releasePreferences()
    }
  }, [projectId, ownedSession])
  useEffect(()=>{const snapshot=session?.store.getSnapshot().snapshot;if(snapshot)focusRequestedRange(snapshot.project as Project,initialT,initialEnd)},[session,initialT,initialEnd])
  useEffect(() => {
    if (!active) { playback.pause(); if(session)cancelEditorDragFor(session.store); return }
    editorEvents.addEventListener('keydown', onKeyDown)
    return () => editorEvents.removeEventListener('keydown', onKeyDown)
  }, [active,session])
  return <div ref={root} className={styles.workspace} data-biliup-editor tabIndex={-1} lang={language}>
    {session && <EditorStoreContext.Provider value={session.store}><WorkspaceContent session={session} active={active} /></EditorStoreContext.Provider>}
  </div>
}
function WorkspaceContent({ session, active }: { session: WorkspaceSession; active: boolean }) {
  useEditorLanguage();
  const app = useApp()
  useLayoutEffect(()=>{
    if(!active||!app.snapshot)return
    const project=app.snapshot.project as import('@/app/lib/editor/core/timeline/types').Project
    const timing=layoutOf(project)
    playback.setProject(timing.frames,project.canvas.fps,timing.end)
  },[active,app.snapshot?.project])
  const dir=app.snapshot?.dir,assetKey=renderAssetsKey(dir,app.brand,app.projectFonts)
  useEffect(()=>()=>{void syncKitFonts([])},[active,dir,session])
  useEffect(()=>{
    if(!active||!dir)return
    const controller=new AbortController(),{signal}=controller
    void loadRenderAssets(dir,assetKey).then(async assets=>{
      if(signal.aborted)return
      const errors=await syncKitFonts(assets.fonts.map((font,index)=>({...font,key:`${assetKey}:${index}`})),signal)
      if(signal.aborted)return
      const warnings=[...assets.warnings,...errors];if(warnings.length)session.store.set({brandError:warnings.join('；')})
      // Changing available font metrics must repaint paused text and component previews too.
      previewPlayback.seek(getPreviewPlayback().frame)
    }).catch(error=>{if(!signal.aborted)session.store.set({brandError:error instanceof Error?error.message:String(error)})})
    return ()=>controller.abort()
  },[active,dir,assetKey,session])
  if (!app.snapshot) return <div className="splash" role="status">{app.connectionMessage ?? editorText("正在打开剪辑工程…")}</div>
  return <>
    {active && <Editor snapshot={app.snapshot} />}
    {active && app.settingsOpen && <Settings />}
    {active && app.shortcutsOpen && <ShortcutsSheet />}
    {active && <SaveRecovery key={app.snapshot.dir} session={session} />}
    <LeaveDialog />
  </>
}
