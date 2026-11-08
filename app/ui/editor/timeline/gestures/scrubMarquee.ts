// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The ruler and empty timeline: scrubbing the playhead (D70 beat snap) and the marquee selection (T8).
import type React from "react";
import { primaryModifier, type Modifiers } from '@/app/lib/editor/keyboard';
import { getBeats } from "@/app/ui/editor/audio/beats";
import { SNAP_PX, snapPlayhead } from "@/app/ui/editor/timeline/edit";
import { getPlayback, playback } from "@/app/ui/editor/preview/playback";
import type { ProjectModel } from "@/app/lib/editor/state/project";
import { DRAG_PX } from "@/app/ui/editor/timeline/ItemBox";
import { type Marquee, selectionIds } from "@/app/ui/editor/timeline/gestures/dragView";
import { beginTimelineView } from './session';
import type { LaneGeometry } from "@/app/ui/editor/timeline/gestures/lanes";

export interface ScrubMarqueeCtx {
  readonly stopDrag: React.RefObject<(() => void) | undefined>;
  readonly project: ProjectModel;
  readonly ppf: number;
  readonly lanesRef: React.RefObject<HTMLDivElement | null>;
  readonly frameAt: LaneGeometry["frameAt"];
  readonly setScrubSnap: React.Dispatch<React.SetStateAction<number | undefined>>;
  readonly setMarquee: React.Dispatch<React.SetStateAction<Marquee | undefined>>;
}

export function makeScrubMarquee(ctx: ScrubMarqueeCtx) {
  const { project, ppf, lanesRef, frameAt, setScrubSnap, setMarquee } = ctx;
  /** Click or drag the ruler/playhead; snapping finds recording markers and visible beats. */
  const scrub = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    e.preventDefault();
    const seek = (ev: { clientX: number } & Modifiers) => {
      const frame = frameAt(ev.clientX);
      const snapped = getPlayback().snapping && !primaryModifier(ev) ? snapPlayhead(project, frame, { beats: getBeats().map, ppf }, SNAP_PX / ppf) : { frame };
      setScrubSnap(snapped.snapAt);
      playback.seek(snapped.frame);
    };
    const move = (ev: PointerEvent) => seek(ev);
    if(beginTimelineView(e,ctx,{move,finish:()=>setScrubSnap(undefined)})){
      playback.pause();
      seek(e);
    }
  };

  /**
   * Press on empty timeline (T8): a click moves the playhead there and clears the selection; a drag
   * draws a marquee and selects every item it touches (with ⇧ or ⌘, added to the selection).
   */
  const onEmptyDown = (e: React.PointerEvent, fromHeads = false) => {
    if (e.button !== 0) return;
    // From the track heads (F12): anywhere but their buttons, fields and sliders; the marquee starts at the lanes' visible left edge.
    if (fromHeads ? (e.target as HTMLElement).closest("button, input, select, textarea, .track-volume") : e.target !== e.currentTarget) return;
    e.preventDefault();
    const lanes = lanesRef.current;
    if (!lanes) return;
    const keep = e.shiftKey || primaryModifier(e) ? selectionIds() : [];
    const r0 = lanes.getBoundingClientRect();
    const headsRight = (e.currentTarget as HTMLElement).getBoundingClientRect().right;
    const start = { x: fromHeads ? Math.max(0, headsRight - r0.left) : e.clientX - r0.left, y: e.clientY - r0.top };
    let active = false;
    const hit = (m: Marquee) => {
      const r = lanes.getBoundingClientRect();
      const box = { left: Math.min(m.x0, m.x1) + r.left, right: Math.max(m.x0, m.x1) + r.left, top: Math.min(m.y0, m.y1) + r.top, bottom: Math.max(m.y0, m.y1) + r.top };
      const ids = [...lanes.querySelectorAll<HTMLElement>("[data-testid=timeline-item]")]
        .filter((el) => el.dataset.type !== "captions")
        .filter((el) => {
          const b = el.getBoundingClientRect();
          return b.right > box.left && b.left < box.right && b.bottom > box.top && b.top < box.bottom;
        })
        .map((el) => el.dataset.item!);
      playback.selectMany([...new Set([...keep, ...ids])]);
    };
    const move = (ev: PointerEvent) => {
      const r = lanes.getBoundingClientRect();
      const at = { x: ev.clientX - r.left, y: ev.clientY - r.top };
      if (!active && Math.abs(at.x - start.x) < DRAG_PX && Math.abs(at.y - start.y) < DRAG_PX) return;
      active = true;
      const m = { x0: start.x, y0: start.y, x1: Math.max(0, Math.min(r.width, at.x)), y1: Math.max(0, Math.min(r.height, at.y)) };
      setMarquee(m);
      hit(m);
    };
    if(beginTimelineView(e,ctx,{move,finish:()=>setMarquee(undefined)})&&!keep.length&&!fromHeads){
      playback.pause();
      playback.select(undefined);
      const frame=frameAt(e.clientX);
      const snapped=getPlayback().snapping&&!primaryModifier(e)?snapPlayhead(project,frame,{beats:getBeats().map,ppf},SNAP_PX/ppf):{frame};
      playback.seek(snapped.frame);
    }
  };
  return { scrub, onEmptyDown };
}
