import { editorText } from '@/app/lib/editor/i18n';
import type { ProjectModel } from '@/app/lib/editor/state/project';
import { findItem, lockOf, type Plan } from './base';

export type NameTarget = { readonly kind: 'project' } | { readonly kind: 'item' | 'track' | 'media'; readonly id: string };

export function readName(project: ProjectModel, target: NameTarget): string | undefined {
  if (target.kind === 'project') return project.name;
  if (target.kind === 'item') { const placed = findItem(project, target.id); return placed ? placed.item.name ?? '' : undefined; }
  if (target.kind === 'track') return project.tracks.find(track => track.id === target.id)?.name;
  return project.media.find(media => media.id === target.id)?.name;
}

/** Project settings, timeline, library and inspector share name rules and edit operations. */
export function planRename(project: ProjectModel, target: NameTarget, name: string): Plan | {readonly problem: string} {
  const before = readName(project, target);
  if (before === undefined) return {problem: editorText('This item is no longer in the project.')};
  const next = name.trim();
  if (!next && target.kind !== 'item') return {problem: editorText('Enter a name.')};
  if (next === before) return {ops: [], summary: ''};
  if (target.kind === 'project') return {ops: [{op: 'update_project', set: {name: next}}], summary: editorText('Rename project to {0}', {0: next})};
  if (target.kind === 'item') {
    const locked = lockOf(findItem(project, target.id)!);
    if (locked) return {problem: locked};
    return {ops: [{op: 'update_item', item: target.id, set: {name: next || null}}], summary: next ? editorText('Rename clip to {0}', {0: next}) : editorText('Use the media name')};
  }
  if (target.kind === 'track') return {ops: [{op: 'update_track', track: target.id, set: {name: next}}], summary: editorText('Rename track to {0}', {0: next})};
  return {ops: [{op: 'update_media', media: target.id, set: {name: next}}], summary: editorText('Rename media to {0}', {0: next})};
}
