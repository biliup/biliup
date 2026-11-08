// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { isDrawnComponent } from "@/app/lib/editor/core/visual/components";
import React, { useState, useLayoutEffect } from "react";
import { findItem, lockOf } from "@/app/ui/editor/timeline/edit";
import { usePlayback } from "@/app/ui/editor/preview/playback";
import {useApp} from '@/app/lib/editor/state/store';
import {RECORDING_MASK,type EditorMaskRegion} from '@/app/lib/editor/recording-effects';
import type { ProjectModel } from "@/app/lib/editor/state/project";
import { clipState, type Corner } from "@/app/ui/editor/timeline/clip";

/**
 * The selected clip's box on the canvas (D75): an outline and four corner handles (drag one to
 * resize; the clip itself drags to move). Follows the playhead (keyed x / y) and drafts.
 */
export function ClipSelection({ project }: { project: ProjectModel }) {
  const {maskEditing,connection,biliupCanEdit}=useApp();
  const selected = usePlayback((s) => s.selected);
  const frame = usePlayback((s) => Math.round(s.frame));
  const draft = usePlayback((s) => s.draft);
  const shown = draft && draft.base === project ? draft.project : project;
  const placed = findItem(shown, selected);
  if (!placed || (placed.item.type !== "clip" && !isDrawnComponent(placed.item)) || lockOf(placed)) return null;
  const mask=placed.item.effects?.find(effect=>effect.module===RECORDING_MASK&&!effect.bypass);
  if(mask&&maskEditing?.itemId===placed.item.id&&(maskEditing.drawing||((mask.params?.regions??[]) as EditorMaskRegion[]).some(region=>region.id===maskEditing.regionId)))return null;
  const state = clipState(shown, placed.item.id, frame);
  if (!state?.inside) return null;
  const { x, y, w, h } = state.box;
  const corners: ReadonlyArray<readonly [Corner, number, number]> = [
    ["nw", x, y],
    ["ne", x + w, y],
    ["sw", x, y + h],
    ["se", x + w, y + h],
  ];
  return (
    <>
      <div className="pv-clip-sel" data-testid="clip-selection" data-item={placed.item.id} style={{ left: x, top: y, width: w, height: h }} />
      {connection==='connected'&&biliupCanEdit&&corners.map(([c, cx, cy]) => (
        <div key={c} className="pv-handle" data-testid="clip-handle" data-handle={c} style={{ left: cx, top: cy }} />
      ))}
    </>
  );
}

/**
 * The selected text's handles on the canvas (F20, D158): four corner marks around the drawn text
 * (measured from the text layer, which lays out at canvas size), so a text just added from the Text
 * tab shows as selected. The text itself drags, and a double click edits it; a corner resizes a text
 * item (issue #72, D242: its font size, the opposite corner staying). Captions only show them.
 */
export function TextSelection({ layer, project }: { layer: React.RefObject<HTMLDivElement | null>; project: ProjectModel }) {
  const {connection,biliupCanEdit}=useApp();
  const selected = usePlayback((s) => s.selected);
  const frame = usePlayback((s) => Math.round(s.frame));
  const draft = usePlayback((s) => s.draft);
  const [box, setBox] = useState<{ x: number; y: number; w: number; h: number } | undefined>();
  useLayoutEffect(() => {
    let raf = 0;
    const measure = () => {
      const el = selected === undefined ? null : layer.current?.querySelector<HTMLElement>(`.pv-text.sel-box[data-item="${CSS.escape(selected)}"]`);
      const next = el && el.offsetWidth > 0 ? { x: el.offsetLeft, y: el.offsetTop, w: el.offsetWidth, h: el.offsetHeight } : undefined;
      setBox((prev) => (prev && next && prev.x === next.x && prev.y === next.y && prev.w === next.w && prev.h === next.h) || (!prev && !next) ? prev : next);
    };
    measure();
    // The text layer is redrawn after this render: measure again once it has been.
    raf = requestAnimationFrame(measure);
    return () => cancelAnimationFrame(raf);
  }, [selected, frame, layer, project, draft]);
  const placed = findItem(draft && draft.base === project ? draft.project : project, selected);
  const resizable = connection==='connected'&&biliupCanEdit&&placed?.item.type === "text" && placed.item.anchor !== "bottom" && !lockOf(placed);
  if (!box) return null;
  const corners = [
    ["nw", box.x, box.y],
    ["ne", box.x + box.w, box.y],
    ["sw", box.x, box.y + box.h],
    ["se", box.x + box.w, box.y + box.h],
  ] as const;
  return (
    <>
      {corners.map(([c, cx, cy]) => (
        <div key={c} className="pv-handle text" data-testid="text-handle" data-item={selected} data-corner={c} data-text-handle={resizable ? c : undefined} style={{ left: cx, top: cy }} />
      ))}
    </>
  );
}
