import type { Media, Project } from './core/timeline/types'
import { planReshoot, type Overrun } from './core/timeline/reshoot'
import { mediaUses } from './core/timeline/mediaPlan'
import { editorText as t } from './i18n'
import { createHistory, editHistory } from './history'
import { layout } from './core/timeline/timeline'
import { documentFingerprint, mediaForCanvas, type EditorDocument } from './project'

export interface PreparedReplacement {
  readonly token: string
  readonly mediaId: string
  readonly originalPath: string
  readonly media: Media
  readonly durationMs: number | null
}

/** Identify the submitted edit without registry facts the server refreshes during any save. */
export function replacementFingerprint(document:EditorDocument):string{
  return documentFingerprint({...document,
    project:{...document.project,media:document.project.media.map(({probe,...media})=>media)},
    sources:Object.fromEntries(Object.entries(document.sources).map(([id,source])=>[id,source.kind==='recording'
      ?{...source,fileName:'',state:'finished' as const,hasDanmaku:false}
      :{...source,fileName:''}])),
  })
}

/** The same media identity remains in every clip, element, mask and history entry. */
export function replacementPlan(project: Project, replacement: PreparedReplacement, overrun: Overrun) {
  const original = project.media.find(media => media.id === replacement.mediaId)
  if (!original || original.path !== replacement.originalPath) throw new Error(t('The source media changed. Choose the replacement again.'))
  if (original.kind !== replacement.media.kind) throw new Error(t('The replacement must have the same media type.'))
  const uses = mediaUses(project, original.id)
  if (uses.some(use => use.item.locked || use.track.locked)) throw new Error(t('Unlock the clips and tracks using this media before replacing it.'))
  const probe = mediaForCanvas(replacement.media, replacement.durationMs, project.canvas).probe ?? {}
  const plan = planReshoot(project, original.id, {
    path: replacement.media.path, probe, source: replacement.media.source ?? { module: 'import.file' },
  }, overrun)
  // Use the actual edit engine for linked sound, locks, sections and valid transitions.
  const after = editHistory(createHistory(project), { actor: 'user', ops: plan.ops }).current.project
  const items = new Map(after.tracks.flatMap(track => track.items.map(item => [item.id, item] as const)))
  const starts = layout(project).starts, nextStarts = layout(after).starts
  const changes: string[] = []
  for (const track of project.tracks) for (const item of track.items) {
    const next = items.get(item.id), name = item.name ?? item.id
    if (!next) {
      changes.push(t(item.type==='captions'?'Remove captions {0}: transcribe the replacement to make new captions.':item.type==='danmaku'?'Remove danmaku {0}: it belongs to the original recording.':'Remove {0}: the new file has no playable audio for this clip.',{0:name}));
      continue
    }
    if (next.duration < item.duration) changes.push(t('Shorten {0} by {1} s.', { 0: name, 1: ((item.duration - next.duration) / project.canvas.fps).toFixed(2) }))
    if (next.hold && !item.hold) changes.push(t('Hold the last frame for the remainder of {0}.', { 0: name }))
    if (starts.get(item.id) !== nextStarts.get(item.id)) changes.push(t('Move {0} with the changed timeline.', { 0: name }))
    if (item.linkedTo && !next.linkedTo) changes.push(t('Unlink {0} after its duration changes.', { 0: name }))
    if (item.transitionIn && !next.transitionIn) changes.push(t('Remove the transition into {0}: the replacement has no room for it.', { 0: name }))
    else if (item.transitionIn && next.transitionIn && next.transitionIn.duration < item.transitionIn.duration) changes.push(t('Shorten the transition into {0} to fit the replacement.', { 0: name }))
  }
  return { ops: plan.ops, changes, uses: uses.length, summary: t('Replace file: {0}', { 0: original.name }) }
}
