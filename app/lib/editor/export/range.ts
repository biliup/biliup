import {layout} from '../core/timeline/timeline'
import type {Project} from '../core/timeline/types'
export interface FrameRange { readonly from:number; readonly to:number }

/** Original timeline coordinates; a range limits rendering without editing the project. */
export function renderWindow(project:Pick<Project,'tracks'>,range?:FrameRange|null):FrameRange{
  const problem=range?exportRangeProblem(project,range):undefined
  if(problem)throw new Error(problem)
  return range??{from:0,to:layout(project).duration}
}

/** Video, sidecars and standalone subtitles must export exactly the displayed range. */
export function exportRangeProblem(project:Pick<Project,'tracks'>,range?:FrameRange):string|undefined{
  const end=layout(project).duration
  if(end<=0||!project.tracks.some(track=>track.items.length))return 'The timeline is empty. Add a clip before exporting.'
  if(range&&(!Number.isInteger(range.from)||!Number.isInteger(range.to)||range.from<0||range.to<=range.from||range.to>end))return 'The export range is outside the timeline.'
}
