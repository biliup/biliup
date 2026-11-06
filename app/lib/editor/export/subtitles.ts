import {subtitleCues} from '../core/text/subtitles'
import type {Project} from '../core/timeline/types'
import {exportRangeProblem,type FrameRange} from './range'

/** Sidecars, soft subtitles and standalone downloads use the video's timeline cut. */
export function subtitlesForExport(project:Project,range?:FrameRange){
  const problem=exportRangeProblem(project,range)
  if(problem)return {problem}
  const cues=subtitleCues(project)
  // Clip the existing cards, without regenerating their words or line breaks at the range edge.
  return {cues:range?cues.flatMap(cue=>{
    const from=Math.max(range.from,cue.from),to=Math.min(range.to,cue.to)
    return to>from?[{...cue,from:from-range.from,to:to-range.from}]:[]
  }):cues,fps:project.canvas.fps}
}
