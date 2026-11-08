import type {Project} from '../core/timeline/types'
/** Analysis is about source files, not just persistent media IDs. */
export const analysisVersions=(project:Pick<Project,'media'>)=>JSON.stringify(project.media.map(media=>[media.id,media.path]).sort((a,b)=>a[0].localeCompare(b[0])))
