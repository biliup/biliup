import type {ExportJob} from '../contracts'
import {editorText} from '../i18n'

export const ACTIVE_EXPORT_PHASES:ReadonlySet<ExportJob['phase']>=new Set(['queued','preparing','rendering'])
export const exportRunning=(job:ExportJob|undefined)=>!!job&&ACTIVE_EXPORT_PHASES.has(job.phase)

/** The toolbar and its dialog follow the same running job, including work started elsewhere. */
export function activeExport(jobs:ReadonlyArray<ExportJob>,dir:string,selected?:string){
  const active=jobs.filter(job=>job.projectDir===dir&&exportRunning(job))
  return active.find(job=>job.id===selected)??active[0]
}

/** The same preparation step is shown in the editor and the global output history. */
export function preparingStatus(job: Pick<ExportJob, 'step'>): { readonly text: string; readonly fraction: number | undefined } {
  const step=job.step
  const names={sound:'Preparing audio',mix:'Processing audio',loudness:'Measuring loudness',graphics:'Rendering graphics',shaders:'Rendering effects',encoder:'Preparing encoder'}
  const counted=step&&step.done!==undefined&&Number.isFinite(step.done)&&step.total!==undefined&&Number.isFinite(step.total)&&step.total>0
    ?{done:Math.max(0,Math.min(step.done,step.total)),total:step.total}:undefined
  const fraction=counted?counted.done/counted.total:undefined
  const label=step?.kind==='loudness'&&step.pass!==undefined
    ?editorText('Measuring loudness · pass {0}',{'0':step.pass})
    :editorText(step?names[step.kind]:'Preparing export')
  const amount=counted?(step?.kind==='mix'||step?.kind==='loudness'?` ${Math.floor(fraction!*100)}%`:` ${counted.done}/${counted.total}`):''
  return {text:label+amount,fraction}
}
