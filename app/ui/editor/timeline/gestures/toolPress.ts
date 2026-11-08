// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Presses on the lanes under the Blade and Hand tools (issue #57, D230). The Select tool's presses
// go to the items and the empty timeline as before (itemDrag, scrubMarquee); the ruler always scrubs.
import type React from "react";
import { primaryModifier, type Modifiers } from '@/app/lib/editor/keyboard';
import { getBeats } from "@/app/ui/editor/audio/beats";
import { getPlayback } from "@/app/ui/editor/preview/playback";
import type { ProjectModel } from "@/app/lib/editor/state/project";
import { actions } from "@/app/lib/editor/state/store";
import { projectOf } from '@/app/lib/editor/state/store/core';
import { editorGestureOrigin } from '../../workspace/pointerGesture';
import { beginTimelineView } from './session';
import { bladeFrame, findItem, planBladeSplit, SNAP_PX } from "@/app/ui/editor/timeline/edit";
import { getTool } from "@/app/ui/editor/timeline/tools";
import type { LaneGeometry } from "@/app/ui/editor/timeline/gestures/lanes";

/** The blade's hover line: where a click would cut, on the lane of the item under the pointer. */
export interface BladeHover {
  readonly item: string;
  readonly frame: number;
  readonly snapAt?: number;
  readonly top: number;
  readonly height: number;
  /** Why a click wouldn't cut there (planBladeSplit's problem: a lock, a locked linked sound, a transition). */
  readonly refused?: string;
}

export interface ToolPressCtx {
  readonly stopDrag: React.RefObject<(() => void) | undefined>;
  readonly project: ProjectModel;
  readonly ppf: number;
  readonly online: boolean;
  readonly lanesRef: React.RefObject<HTMLDivElement | null>;
  readonly bodyRef: React.RefObject<HTMLDivElement | null>;
  readonly frameAt: LaneGeometry["frameAt"];
  readonly setBlade: React.Dispatch<React.SetStateAction<BladeHover | undefined>>;
  readonly setPanning: React.Dispatch<React.SetStateAction<boolean>>;
}

export function makeToolPress(ctx: ToolPressCtx) {
  const { project, ppf, online, lanesRef, bodyRef, frameAt, setBlade, setPanning } = ctx;
  const onRuler = (target: EventTarget | null) => target instanceof Element && target.closest(".tl-ruler") !== null;

  /** The item under the pointer and the frame the blade would cut it at. */
  const bladeAt = (e: { target: EventTarget | null; clientX: number } & Modifiers): BladeHover | undefined => {
    if (!(e.target instanceof Element) || onRuler(e.target)) return undefined;
    const el = e.target.closest<HTMLElement>("[data-testid=timeline-item]");
    const lane = e.target.closest<HTMLElement>("[data-testid=lane]");
    const id = el?.dataset.item;
    const placed = id ? findItem(project, id) : undefined;
    if (!placed || !lane || !lanesRef.current || placed.item.type === "captions") return undefined;
    const snapOn = getPlayback().snapping && !primaryModifier(e);
    const at = bladeFrame(project, placed.item.id, frameAt(e.clientX), snapOn ? { playhead: getPlayback().frame, threshold: SNAP_PX / ppf, beats: { beats: getBeats().map, ppf } } : undefined);
    if (!(placed.start < at.frame && at.frame < placed.start + placed.item.duration)) return undefined;
    const r = lane.getBoundingClientRect();
    const top = r.top - lanesRef.current.getBoundingClientRect().top;
    // D248: the line says what the click would do: planBladeSplit's refusal (locks, a locked linked sound, a transition).
    const plan = planBladeSplit(project, placed.item.id, at.frame);
    const refused = "problem" in plan ? plan.problem : undefined;
    return { item: placed.item.id, frame: at.frame, ...(at.snapAt !== undefined ? { snapAt: at.snapAt } : {}), top, height: r.height, ...(refused ? { refused } : {}) };
  };

  const onPointerMove = (e: React.PointerEvent) => {
    if (getTool() !== "blade") return setBlade(undefined);
    setBlade(bladeAt(e));
  };
  const onPointerLeave = () => setBlade(undefined);

  /** Capture phase: under Blade a press on an item cuts it; under Hand any press (but the ruler's) drags the view. */
  const onPointerDownCapture = (e: React.PointerEvent) => {
    const tool = getTool();
    if (e.button !== 0 || tool === "select" || onRuler(e.target)) return;
    if (tool === "blade") {
      const at = bladeAt(e);
      const onItem = e.target instanceof Element && e.target.closest("[data-testid=timeline-item]");
      if (!onItem) return;
      // An item, cut or not: the Select tool's press (select, move, trim) doesn't run.
      e.preventDefault();
      e.stopPropagation();
      if (!at || !online) return;
      ctx.stopDrag.current?.();
      const origin=editorGestureOrigin();
      if(!origin||projectOf(origin.owner.getSnapshot().snapshot!)!==project)return;
      void actions.itemEdit(current=>planBladeSplit(current,at.item,at.frame),{dir:origin.dir,baseRevision:origin.baseRevision});
      return;
    }
    const body = bodyRef.current;
    if (!body) return;
    e.preventDefault();
    e.stopPropagation();
    const x0 = e.clientX;
    const y0 = e.clientY;
    const left = body.scrollLeft;
    const top = body.scrollTop;
    const move = (ev: PointerEvent) => {
      body.scrollLeft = left - (ev.clientX - x0);
      body.scrollTop = top - (ev.clientY - y0);
    };
    if(beginTimelineView(e,ctx,{move,finish:()=>setPanning(false)}))setPanning(true);
  };
  return { onPointerDownCapture, onPointerMove, onPointerLeave };
}
