import { editorRoot } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Tiles and browser files dropped on the lanes; transitions use the pointer gesture.
import type React from "react";
import type { Cut } from "@/app/lib/editor/core/visual/transitions";
import { actions, getState } from "@/app/lib/editor/state/store";
import { TEXT_PRESETS } from "@/app/lib/editor/state/project";
import { importDroppedFiles } from '@/app/lib/editor/state/store/server';
import { secondsFor } from "@/app/ui/editor/preview/components";
import { CUT_DROP_PX, endEditorDrag, hasEditorDrag, hasFiles, overEditorDrag, readEditorDrag } from "@/app/ui/editor/timeline/dnd";
import { HEAD_WIDTH } from "@/app/ui/editor/timeline/gestures/dragView";
import type { LaneGeometry } from "@/app/ui/editor/timeline/gestures/lanes";

export interface LaneDropCtx {
  readonly fps: number;
  readonly ppf: number;
  readonly online: boolean;
  readonly cuts: ReadonlyArray<Cut>;
  readonly lanesRef: React.RefObject<HTMLDivElement | null>;
  readonly bodyRef: React.RefObject<HTMLDivElement | null>;
  readonly frameAt: LaneGeometry["frameAt"];
  readonly laneHit: LaneGeometry["laneHit"];
  readonly dropLane: string | undefined;
  readonly dropSpan: { frame: number; frames: number } | undefined;
  readonly setDropLane: React.Dispatch<React.SetStateAction<string | undefined>>;
  readonly setDropSpan: React.Dispatch<React.SetStateAction<{ frame: number; frames: number } | undefined>>;
  readonly setDropping: React.Dispatch<React.SetStateAction<boolean>>;
}

export function makeLaneDrop(ctx: LaneDropCtx) {
  const { fps, ppf, online, cuts, lanesRef, bodyRef, frameAt, laneHit, dropLane, dropSpan, setDropLane, setDropSpan, setDropping } = ctx;
  /** The main cut nearest a pointer x, within CUT_DROP_PX. */
  const cutNear = (clientX: number) => {
    const f = frameAt(clientX);
    let best: (typeof cuts)[number] | undefined;
    for (const c of cuts) if (Math.abs(c.frame - f) * ppf <= CUT_DROP_PX && (!best || Math.abs(c.frame - f) < Math.abs(best.frame - f))) best = c;
    return best;
  };
  const overLanes = (x: number, y: number) => {
    const r = lanesRef.current?.getBoundingClientRect();
    const body = bodyRef.current?.getBoundingClientRect();
    return !!r && !!body && x >= Math.max(r.left, body.left + HEAD_WIDTH) && x <= Math.min(r.right, body.right) && y >= body.top && y <= body.bottom;
  };
  /** The timeline item under a pointer (an Effects tile dropped there goes on it, D136). */
  const itemAt = (x: number, y: number) => document.elementsFromPoint(x, y).find((el): el is HTMLElement => el instanceof HTMLElement && editorRoot().contains(el) && el.dataset.testid === "timeline-item")?.dataset.item;
  const onDragOver = (e: React.DragEvent) => {
    if (!online || !getState().biliupCanEdit) return;
    const types = [...e.dataTransfer.types];
    const value = overEditorDrag(e.dataTransfer);
    if (hasEditorDrag(types) && !value) return;
    if (value?.kind === 'effect') {
      if (!itemAt(e.clientX, e.clientY)) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
      return;
    }
    if (!value && !hasFiles(types)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    setDropping(true);
    const lane = value?.kind === 'media' || !value ? laneHit(e.clientY).track?.id : undefined;
    if (lane !== dropLane) setDropLane(lane);
    const tile = value?.kind === 'text' ? 3 : value?.kind === 'component' ? secondsFor(value.id) : undefined;
    const span = tile === undefined ? undefined : { frame: Math.max(0, Math.round(frameAt(e.clientX))), frames: Math.max(1, Math.round(tile * fps)) };
    if (span?.frame !== dropSpan?.frame || span?.frames !== dropSpan?.frames) setDropSpan(span);
  };
  const onDrop = (e: React.DragEvent) => {
    setDropping(false);
    setDropLane(undefined);
    setDropSpan(undefined);
    const value = readEditorDrag(e.dataTransfer);
    const types = [...e.dataTransfer.types];
    if (hasEditorDrag(types)) {
      e.preventDefault();
      if (value) endEditorDrag();
      if (!value) return;
    }
    if (!online || !getState().biliupCanEdit) { e.preventDefault(); return; }
    if (value?.kind === 'effect') {
      const id = itemAt(e.clientX, e.clientY);
      if (id) void actions.applyFx(value.id, id);
      return;
    }
    const frame = Math.max(0, Math.round(frameAt(e.clientX)));
    if (value?.kind === 'text') {
      const preset = TEXT_PRESETS.find(p => p.id === value.id);
      if (preset) void actions.addText(preset, { frame });
      return;
    }
    if (value?.kind === 'component') {
      void actions.addComponent(value.id, { frame, props: value.props });
      return;
    }
    const track = laneHit(e.clientY).track?.id;
    if (value?.kind === 'media') {
      void actions.dropMedia(value.id, { frame, track });
      return;
    }
    // Upload browser files and place them in the same import edit.
    const files = [...e.dataTransfer.files];
    if (files.length || hasFiles(types)) {
      e.preventDefault();
      importDroppedFiles(e.dataTransfer, { frame, fps, track });
      return;
    }
  };
  return { cutNear, overLanes, itemAt, onDragOver, onDrop };
}
