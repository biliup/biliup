import type { SavedProject } from './api'
import type { EditorDocument } from './project'
import type { Item, Track } from './core/timeline/types'
import { layout } from './core/timeline/timeline'
import { editorText as t } from './i18n'

/** Exactly the local draft and server revision the user has reviewed. */
export interface SaveReview {
  readonly projectId: number
  readonly localRevision: number
  readonly localFingerprint: string
  readonly local: EditorDocument
  readonly remote: SavedProject
}
export interface SaveDifference {
  readonly kind: 'added' | 'removed' | 'changed'
  readonly label: string
}

function same(a: unknown, b: unknown): boolean {
  if (a === b) return true
  if (a === null || b === null || typeof a !== 'object' || typeof b !== 'object') return false
  if (Array.isArray(a) || Array.isArray(b)) return Array.isArray(a) && Array.isArray(b) && a.length === b.length && a.every((value, index) => same(value, b[index]))
  const left = a as Record<string, unknown>, right = b as Record<string, unknown>
  const keys = Object.keys(left)
  return keys.length === Object.keys(right).length && keys.every(key => Object.prototype.hasOwnProperty.call(right, key) && same(left[key], right[key]))
}

/** Shows how keeping the current edit would change the reviewed server version. */
export function saveDifferences(local: EditorDocument, remote: EditorDocument): SaveDifference[] {
  const rows: SaveDifference[] = []
  const project = local.project, server = remote.project
  const field = (label: string, a: unknown, b: unknown) => {
    if (!same(a, b)) rows.push({ kind: 'changed', label: t(label) })
  }
  field('Name', project.name, server.name)
  field('Canvas', project.canvas, server.canvas)
  field('Theme', project.theme, server.theme)
  field('Master audio', project.master, server.master)
  field('Markers', project.markers, server.markers)
  field('Scenes', project.scenes, server.scenes)
  field('Sections', project.sections, server.sections)
  field('Motion blur', project.motionBlur, server.motionBlur)
  field('Checks', project.checks, server.checks)
  field('Recording source', { origin: local.origin, markers: local.sourceMarkers }, { origin: remote.origin, markers: remote.sourceMarkers })
  field('Project warnings', local.warnings, remote.warnings)

  const collection = <T,>(left: ReadonlyMap<string, T>, right: ReadonlyMap<string, T>, label: (value: T) => string) => {
    for (const [id, value] of left) {
      if (!right.has(id)) rows.push({ kind: 'added', label: label(value) })
      else if (!same(value, right.get(id))) rows.push({ kind: 'changed', label: label(value) })
    }
    for (const [id, value] of right) if (!left.has(id)) rows.push({ kind: 'removed', label: label(value) })
  }
  const media = (document: EditorDocument) => new Map(document.project.media.map(value => [value.id, { media: value, source: document.sources[value.id] }]))
  collection(media(local), media(remote), value => t('Media: {0}', { 0: value.media.name }))
  const tracks = (values: ReadonlyArray<Track>) => new Map(values.map(({ items: _items, ...track }) => [track.id, track]))
  collection(tracks(project.tracks), tracks(server.tracks), track => t('Track: {0}', { 0: track.name }))
  field('Track order', project.tracks.map(track => track.id), server.tracks.map(track => track.id))
  const items = (document: EditorDocument) => {
    const starts = layout(document.project).starts
    return new Map(document.project.tracks.flatMap(track => track.items.map(item => [item.id, { item, track: track.id, start: starts.get(item.id) ?? item.start ?? 0 }] as const)))
  }
  collection(items(local), items(remote), ({ item }: { item: Item }) => t('Clip: {0}', { 0: item.name ?? item.id }))
  return rows
}
