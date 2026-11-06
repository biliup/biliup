import { editorText } from '@/app/lib/editor/i18n';
import type { Marker } from '@/app/lib/editor/core/timeline/types';
import { layoutOf, type ProjectModel } from '@/app/lib/editor/state/project';
import type { Plan } from './base';

const frameOf = (frame: number) => Number.isSafeInteger(frame) && frame >= 0;
const change = (markers: ReadonlyArray<Marker>, summary: string): Plan => ({
  ops: [{ op: 'update_project', set: { markers } }], summary,
});

export function planAddMarker(project: ProjectModel, frame: number): Plan {
  const summary = editorText('Add marker');
  if (!frameOf(frame) || frame > layoutOf(project).end || project.markers?.some(marker => marker.frame === frame)) return { ops: [], summary };
  return change([...(project.markers ?? []), { frame }], summary);
}

/** Array positions belong to the captured project revision, never to a later save. */
export function planEditMarker(project: ProjectModel, index: number, set: Partial<Marker> | null): Plan {
  const summary = editorText(set === null ? 'Delete marker' : set.frame !== undefined ? 'Move marker' : 'Rename marker');
  const markers = project.markers ?? [], previous = markers[index];
  if (!previous || (set?.frame !== undefined && (!frameOf(set.frame) || set.frame > layoutOf(project).end))) return { ops: [], summary };
  if (set === null) return change(markers.filter((_, at) => at !== index), summary);
  const next = { ...previous, ...set };
  if (next.label !== undefined) {
    const label = next.label.trim();
    if (label) next.label = label;
    else delete next.label;
  }
  if (next.frame === previous.frame && next.label === previous.label) return { ops: [], summary };
  return change(markers.map((marker, at) => at === index ? next : marker), summary);
}
