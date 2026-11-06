import type {GenerateJob} from './contracts'
import type {Media} from './core/timeline/types'
import type {GenerateDraft} from './state/store/types'

/** Retry is an explicit new generation using the request shown on the completed job. */
export function generationDraft(job:GenerateJob):GenerateDraft {
  return {generator:job.generator,prompt:job.prompt,kind:job.kind,size:job.size,seconds:job.seconds}
}

/** Authored prompts and names stay literal; the saved media carries the generation settings. */
export function generatedDraft(media:Media):GenerateDraft {
  const source=media.source
  return {
    generator:String(source?.module??''),prompt:typeof source?.prompt==='string'?source.prompt:'',kind:media.kind,
    ...(typeof source?.size==='string'?{size:source.size}:{}),
    ...(typeof source?.seconds==='number'?{seconds:source.seconds}:{}),
  }
}

export function generationRunning(jobs:ReadonlyArray<GenerateJob>,draft:GenerateDraft){
  return jobs.some(job=>(job.phase==='running'||job.phase==='importing')&&job.generator===draft.generator
    &&job.prompt===draft.prompt.trim()&&job.kind===draft.kind&&job.size===draft.size&&job.seconds===draft.seconds)
}
