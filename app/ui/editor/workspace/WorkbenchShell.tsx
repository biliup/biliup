'use client'
import {createContext,useContext,useState,type ReactNode} from 'react'
import {createPortal} from 'react-dom'
import {useSearchParams} from 'next/navigation'
import useSWR,{mutate} from 'swr'
import {Clapperboard,Lock,Unlock,X} from 'lucide-react'
import {fetcher} from '@/app/lib/api-streamer'
import {projectsUrl,saveProject,type SavedProject} from '@/app/lib/editor/api'
import {DEFAULT_CANVAS,emptyProject} from '@/app/lib/editor/project'
import {canvasSide,FPS_CHOICES,type Canvas} from '../settings/canvas'
import {CANVAS_PRESETS} from '@/app/lib/editor/state/format'
import {NumInput} from '../inspector/controls'
import type {EditorSession} from '@/app/lib/editor/session'
import {projectEditorHref} from '@/app/lib/workbench'
import {editorText as t,useEditorLanguage,getEditorLanguage,setEditorLanguage} from '@/app/lib/editor/i18n'
import styles from '../workspace.module.css'

const Context=createContext<{toolbar:HTMLElement|null;project?:EditorSession;projectId:number|null;visible:boolean}>({toolbar:null,projectId:null,visible:true})
export const useWorkbench=()=>useContext(Context)
export function WorkbenchToolbar({children}:{children:ReactNode}){const {toolbar}=useWorkbench();return toolbar?createPortal(children,toolbar):null}

/** The application owns project selection; this header only identifies and operates the editor. */
export function WorkbenchShell({children,project,hasEditor}:{children:ReactNode;project?:EditorSession;hasEditor:boolean}){
 useEditorLanguage();const params=useSearchParams(),projectId=project?.id??null
 const [toolbar,setToolbar]=useState<HTMLDivElement|null>(null)
 const {data:saved}=useSWR<SavedProject>(projectId?`${projectsUrl}/${projectId}`:null,fetcher)
 const showingEditor=hasEditor
 return <Context.Provider value={{toolbar,project,projectId,visible:showingEditor&&!params.has('new')}}>
  <div className={styles.workspace} data-biliup-workbench>
   <div className="workbench-shell">
    <header className="topbar workbench-header" data-testid="workbench-header">
     <span className="workbench-title" title={saved?.name}>{saved?.name??t('Workbench')}</span>
     <div className="topbar-spacer"/>
     <div className="workbench-editor-actions" ref={setToolbar} hidden={!showingEditor}/>
     {!showingEditor&&<select className="input" aria-label={t('Language')} value={getEditorLanguage()} onChange={e=>setEditorLanguage(e.target.value==='en'?'en':'zh-CN')}><option value="zh-CN">简体中文</option><option value="en">English</option></select>}
    </header>
    <div className="workbench-editor" hidden={!showingEditor} inert={!showingEditor}>{children}</div>
    {!showingEditor&&<main className="workspace-empty"><Clapperboard size={36}/><h2>{t('Choose a project in the sidebar')}</h2></main>}
    {params.has('new')&&<CreateProject/>}
   </div>
  </div>
 </Context.Provider>
}

function CreateProject(){
 const params=useSearchParams(),[name,setName]=useState(''),[busy,setBusy]=useState(false),[error,setError]=useState('')
 const [canvas,setCanvas]=useState<Canvas>(DEFAULT_CANVAS),[match,setMatch]=useState(false),[locked,setLocked]=useState(true),[ratio,setRatio]=useState(16/9)
 const preset=CANVAS_PRESETS.find(value=>value.canvas.width===canvas.width&&value.canvas.height===canvas.height&&value.canvas.fps===canvas.fps)?.id??'custom'
 const size=(axis:'width'|'height',value:number|undefined)=>{if(value===undefined)return;const side=canvasSide(value);setCanvas(current=>({...current,[axis]:side,...(locked?(axis==='width'?{height:canvasSide(side/ratio)}:{width:canvasSide(side*ratio)}):{})}))}
 const close=()=>{const p=new URLSearchParams(params.toString());p.delete('new');window.history.replaceState(null,'',`/workbench?${p}`)}
 return <div className="modal-layer"><form className="modal source-export new-project-dialog" role="dialog" aria-modal="true" aria-label={t('New project')} onSubmit={async e=>{e.preventDefault();if(busy)return;setBusy(true);setError('');try{
   const saved=await saveProject(emptyProject(name||t('Untitled project'),{...canvas,matchFirstClip:match}));await mutate(projectsUrl)
   window.history.pushState(null,'',projectEditorHref(saved.id))
 }catch(e){setError(String(e));setBusy(false)}}}>
  <header className="source-section"><h2>{t('New project')}</h2><button type="button" className="icon-btn" disabled={busy} aria-label={t('Cancel')} onClick={close}><X/></button></header>
  <label>{t('Name')}<input autoFocus className="input workbench-name-input" disabled={busy} value={name} maxLength={256} placeholder={t('Untitled project')} onChange={e=>setName(e.target.value)}/></label>
  <div className="group"><label className="check-row"><input type="checkbox" checked={match} disabled={busy} onChange={e=>setMatch(e.target.checked)}/>{t('Match first video')}</label></div>
  <div className="group"><label className="field-row"><span>{t('Canvas preset')}</span><select className="field input" value={preset} disabled={busy||match} onChange={e=>{const next=CANVAS_PRESETS.find(value=>value.id===e.target.value);if(next){setCanvas(current=>({...current,...next.canvas}));setRatio(next.canvas.width/next.canvas.height)}}}>
   {CANVAS_PRESETS.map(value=><option key={value.id} value={value.id}>{t(value.label)} · {value.canvas.width}×{value.canvas.height} · {value.canvas.fps} fps</option>)}
   <option value="custom" disabled>{t('Custom')}</option>
  </select></label></div>
  <div className="group"><div className="group-label">{t('Resolution')}</div><div className="field-row">
   <NumInput testId="new-project-width" label="W" ariaLabel={t('Width')} value={canvas.width} min={16} max={8192} step={2} disabled={busy||match} onCommit={value=>size('width',value)}/>
   <button type="button" className="icon-btn" aria-label={t('Lock aspect ratio')} aria-pressed={locked} disabled={busy||match} onClick={()=>{setLocked(!locked);setRatio(canvas.width/canvas.height)}}>{locked?<Lock/>:<Unlock/>}</button>
   <NumInput testId="new-project-height" label="H" ariaLabel={t('Height')} value={canvas.height} min={16} max={8192} step={2} disabled={busy||match} onCommit={value=>size('height',value)}/>
  </div></div>
  <div className="group"><label className="field-row"><span>{t('Frame rate')}</span><select className="field input" disabled={busy||match} value={canvas.fps} onChange={e=>setCanvas(current=>({...current,fps:Number(e.target.value) as Canvas['fps']}))}>{FPS_CHOICES.map(fps=><option key={fps} value={fps}>{fps} {t(' fps')}</option>)}</select></label></div>
  <div className="group"><label className="field-row"><span>{t('Background')}</span><input type="color" disabled={busy} value={canvas.background??'#000000'} onChange={e=>setCanvas(current=>({...current,background:e.target.value}))}/></label></div>
  {error&&<p role="alert">{error}</p>}<footer className="source-actions"><button type="button" className="btn" disabled={busy} onClick={close}>{t('Cancel')}</button><button type="submit" className="btn primary" disabled={busy}>{t('Create project')}</button></footer>
 </form></div>
}
