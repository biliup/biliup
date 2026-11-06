import {Check,FolderOpen,Plus,RefreshCw} from 'lucide-react'
import type {GenerateJob} from '@/app/lib/editor/contracts'
import {generationDraft,generationRunning} from '@/app/lib/editor/generation'
import {editorText,useEditorLanguage} from '@/app/lib/editor/i18n'
import {actions,useApp,type Project} from '@/app/lib/editor/state/store'
import {GenerationJobRow} from './GenerationJobRow'
import {GenerationCost} from './GenerationCost'

/** The full generator and compact sound form expose the same job and result actions. */
export function GenerationStatus({jobs,project,online}:{jobs:ReadonlyArray<GenerateJob>;project:Project;online:boolean}){
  useEditorLanguage()
  const app=useApp(),running=jobs.filter(job=>job.phase==='running'||job.phase==='importing')
  const last=jobs.find(job=>job.phase==='done'||job.phase==='failed'||job.phase==='cancelled')
  const media=last?.mediaId?project.media.find(media=>media.id===last.mediaId):undefined
  const draft=last&&generationDraft(last)
  const generator=last&&app.generators?.find(generator=>generator.id===last.generator)
  const ready=generator?.ready
  const editable=online&&!!app.biliupCanEdit
  const canAgain=editable&&ready&&!app.generateStarting&&!!draft?.prompt&&!generationRunning(jobs,draft!)
  return <>
    {running.map(job=><GenerationJobRow key={job.id} job={job} online={online}/>)}
    {app.generateError&&<div className="inline-error small" role="alert" data-testid="generate-error">{app.generateError}</div>}
    {last&&!running.length&&(last.phase==='failed'||last.phase==='cancelled')&&<div className="gen-failed small" data-testid="generate-failed">
      <span className={last.phase==='failed'?'inline-error':'muted'}>{last.phase==='cancelled'?editorText('Cancelled'):last.error||editorText('Generation failed')}</span>
      <button className="link-btn" disabled={!canAgain} onClick={()=>void actions.startGenerate(draft!)}><RefreshCw/>{editorText('Generate again')}</button>
      <GenerationCost generator={generator} request={draft!}/>
    </div>}
    {last?.phase==='done'&&!running.length&&media&&<div className="gen-done small" data-testid="generate-done">
      <Check/><span>{media.name} {editorText(' is in Media.')}</span>
      <span className="gen-done-actions">
        <button className="link-btn" onClick={()=>actions.showInMedia(media.id)}><FolderOpen/>{editorText('Show in Media')}</button>
        <button className="link-btn" disabled={!editable} onClick={()=>void actions.addAtPlayhead(media.id)}><Plus/>{editorText('Add to timeline')}</button>
      </span>
    </div>}
  </>
}
