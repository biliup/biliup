import { Flag } from 'lucide-react';
import { useEffect, useState, type Dispatch, type SetStateAction, type PointerEvent, type RefObject, type MouseEvent } from 'react';
import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { primaryModifier, isTextComposition } from '@/app/lib/editor/keyboard';
import { frameTimecode } from '@/app/lib/editor/state/format';
import { layoutOf, type ProjectModel } from '@/app/lib/editor/state/project';
import { actions, getState, projectOf } from '@/app/lib/editor/state/store';
import { getPlayback, playback } from '../preview/playback';
import { beginTimelineEdit } from './gestures/session';
import type { DragView } from './gestures/dragView';
import { draftOf, SNAP_PX, type BeatSnap, type Plan } from './edit';
import { nearestSnap, snapTargets } from './edit/snap';
import { planEditMarker } from './edit/markers';
import { openMenu, type MenuItems } from './ContextMenu';
import { commitMenuEdit } from './menus';
import { RenameField } from './RenameField';

interface Props {
  project: ProjectModel;
  view: ProjectModel;
  ppf: number;
  online: boolean;
  beats: BeatSnap;
  frameAt: (clientX: number) => number;
  stopDrag: RefObject<(() => void) | undefined>;
  setDrag: Dispatch<SetStateAction<DragView | undefined>>;
}

/** Project markers use the same ruler, edit queue and history as the rest of the timeline. */
export function Markers(props: Props) {
  useEditorLanguage();
  const { project, view, ppf, online, frameAt, stopDrag } = props;
  const [renaming, setRenaming] = useState<number>();
  useEffect(() => setRenaming(undefined), [project]);
  useEffect(() => () => stopDrag.current?.(), [project, ppf, stopDrag]);
  const current = () => {
    const snapshot = getState().snapshot;
    return !!snapshot && projectOf(snapshot) === project;
  };
  const seek = (frame: number) => {
    if (!current()) return;
    playback.hover(undefined);
    playback.pause();
    playback.seek(frame);
  };
  const rename = (index: number) => {
    stopDrag.current?.();
    if (online && current()) setRenaming(index);
  };
  const entries = (index: number): MenuItems => {
    const marker = project.markers?.[index];
    if (!marker) return [];
    return [
      { id: 'marker-seek', label: editorText('Go to marker'), run: () => seek(marker.frame) },
      { id: 'marker-chat', label: editorText('Add this moment to chat'), run: () => actions.addMomentToChat(marker.frame) },
      '-',
      { id: 'marker-rename', label: editorText('Rename marker'), disabled: !online, run: () => rename(index) },
      { id: 'marker-delete', label: editorText('Delete marker'), disabled: !online, danger: true, run: () => commitMenuEdit(project, planEditMarker(project, index, null)) },
    ];
  };
  const menu = (event: MouseEvent, index: number) => {
    event.preventDefault(); event.stopPropagation();
    stopDrag.current?.();
    const marker = project.markers?.[index];
    if (!marker || !current()) return;
    // Coincident imported markers keep their own names and can each be edited.
    const together = (project.markers ?? []).flatMap((other, at) => other.frame === marker.frame ? [at] : []);
    const items = together.length > 1 ? together.map(at => ({
      id: `marker-${at}`, label: project.markers![at]!.label || editorText('Marker {0}', { 0: at + 1 }), submenu: entries(at),
    })) : entries(index);
    openMenu(event.clientX, event.clientY, items, 'Marker');
  };
  const down = (event: PointerEvent<HTMLButtonElement>, index: number) => {
    if (event.button !== 0) return;
    event.stopPropagation(); event.preventDefault();
    stopDrag.current?.();
    const marker = project.markers?.[index];
    if (!marker || !current()) return;
    event.currentTarget.focus();
    seek(marker.frame);
    if (!online || renaming !== undefined) return;
    const pointerFrame = frameAt(event.clientX), startX = event.clientX;
    const end = layoutOf(project).end;
    const targets = snapTargets({ ...project, markers: project.markers!.filter((_, at) => at !== index) }, marker.frame, new Set(), props.beats).filter(frame => frame <= end);
    let moved = false, plan: Plan | undefined;
    let gesture: ReturnType<typeof beginTimelineEdit>;
    const move = (next: globalThis.PointerEvent) => {
      if (!moved && Math.abs(next.clientX - startX) < 3) return;
      moved = true;
      const desired = Math.max(0, Math.min(end, Math.round(marker.frame + frameAt(next.clientX) - pointerFrame)));
      const snap = getPlayback().snapping && !primaryModifier(next) ? nearestSnap([desired], { targets, threshold: SNAP_PX / ppf }) : undefined;
      const frame = snap?.at ?? desired;
      const nextPlan = planEditMarker(project, index, { frame });
      const draft = draftOf(project, nextPlan.ops);
      plan = 'problem' in draft ? undefined : nextPlan;
      gesture?.show({
        itemId: '', mode: 'marker',
        view: 'problem' in draft ? project : draft.project,
        tipAt: frame, tip: 'problem' in draft ? draft.problem : frameTimecode(frame, project.canvas.fps),
        ...('problem' in draft ? { refused: draft.problem } : {}),
        ...(snap ? { snapAt: snap.at } : {}),
      });
    };
    gesture = beginTimelineEdit(event, props, { move, release: next => { move(next); return plan; } });
  };

  return <>{(view.markers ?? []).map((marker, index) => {
    const label = marker.label || editorText('Marker {0}', { 0: index + 1 });
    const time = frameTimecode(marker.frame, project.canvas.fps);
    return <div className="tl-marker" key={index} style={{ left: marker.frame * ppf }} data-frame={marker.frame}>
      <button type="button" className="tl-marker-pin" title={`${label} · ${time}`} aria-label={`${label} · ${time}`}
        onPointerDown={event => down(event, index)} onClick={event => { event.stopPropagation(); if (!event.detail) seek(marker.frame); }}
        onDoubleClick={event => { event.stopPropagation(); rename(index); }} onContextMenu={event => menu(event, index)}
        onKeyDown={event => {
          if (isTextComposition(event.nativeEvent)) return;
          if (event.key === 'F2' || event.key === 'Delete' || event.key === 'Backspace') {
            event.preventDefault(); event.stopPropagation();
            if (!online || event.repeat) return;
            if (event.key === 'F2') rename(index);
            else commitMenuEdit(project, planEditMarker(project, index, null));
          }
        }}>
        <Flag aria-hidden="true" />
        <span className="tl-marker-label">{label} · {time}</span>
      </button>
      {renaming === index && <RenameField value={marker.label ?? ''} testId="rename-marker" onDone={(name, origin) => {
        setRenaming(undefined);
        if (name !== undefined && origin) void actions.itemEdit(() => planEditMarker(project, index, { label: name }), { dir: origin.dir, baseRevision: origin.baseRevision });
      }} />}
    </div>;
  })}</>;
}
