// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Pressing a transition marker: select its cut; dragging an edge sets its length (D73).
import type React from "react";
import { draftOf, findItem, lockOf, planSetTransition, secondsLabel, type Plan } from "@/app/ui/editor/timeline/edit";
import { playback } from "@/app/ui/editor/preview/playback";
import type { ProjectModel } from "@/app/lib/editor/state/project";
import { beginTimelineEdit } from './session';
import { DRAG_PX } from "@/app/ui/editor/timeline/ItemBox";
import type { DragView } from "@/app/ui/editor/timeline/gestures/dragView";
import type { LaneGeometry } from "@/app/ui/editor/timeline/gestures/lanes";
import { editorText } from '@/app/lib/editor/i18n';

export interface TransitionDragCtx {
  readonly project: ProjectModel;
  readonly fps: number;
  readonly online: boolean;
  readonly frameAt: LaneGeometry["frameAt"];
  readonly setDrag: React.Dispatch<React.SetStateAction<DragView | undefined>>;
  readonly stopDrag: React.RefObject<(() => void) | undefined>;
}

export function makeTransitionDrag(ctx: TransitionDragCtx) {
  const { project, fps, online, frameAt } = ctx;
  /**
   * Press on a transition marker: select its cut. From an edge, dragging changes its length,
   * centred on the cut (the edge follows the pointer, so the length is twice its distance from
   * the cut), clamped to what fits there; one edit on release ("Changed transition to 0.8 s").
   */
  const onTransitionDown = (e: React.PointerEvent, itemId: string, cutFrame: number) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    ctx.stopDrag.current?.();
    playback.selectCut(itemId);
    const edge = (e.target as HTMLElement).dataset.trEdge;
    const base = project;
    const placed = findItem(base, itemId);
    const tr = placed?.item.transitionIn;
    if (!edge || !placed || !tr || lockOf(placed) || !online) return;
    e.preventDefault();
    const x0 = e.clientX;
    const direction = edge === 'start' ? -1 : 1;
    const frame0 = frameAt(x0);
    let active = false;
    let plan: Plan | undefined;
    const compute = (ev: PointerEvent, against: ProjectModel) => {
      plan = undefined;
      const now = findItem(against, itemId)?.item.transitionIn;
      if (!now) return;
      // Markers have a minimum clickable width. Start from the real duration so grabbing a
      // short marker doesn't lengthen it, and crossing the cut cannot turn shrinking into growth.
      const wanted = Math.max(1, Math.round(tr.duration + 2 * direction * (frameAt(ev.clientX) - frame0)));
      const p = planSetTransition(against, itemId, { module: now.module, duration: wanted });
      if ("problem" in p) return gesture?.show({ itemId: `tr:${itemId}`, mode: "transition", view: against, refused: p.problem, tipAt: cutFrame, tip: p.problem });
      const draft = draftOf(against, p.ops);
      if (!("problem" in draft)) plan = p;
      const shown = ("project" in draft ? findItem(draft.project, itemId)?.item.transitionIn?.duration : undefined) ?? now.duration;
      gesture?.show({
        itemId: `tr:${itemId}`,
        mode: "transition",
        view: "problem" in draft ? against : draft.project,
        ...("problem" in draft ? { refused: draft.problem } : {}),
        tipAt: cutFrame + direction * shown / 2,
        tip: "problem" in draft ? draft.problem : editorText('{0} ({1} frames)', {0: secondsLabel(shown, fps), 1: shown}),
      });
    };
    const move = (ev: PointerEvent) => {
      if (!active && Math.abs(ev.clientX - x0) < DRAG_PX) return;
      if (!active) playback.pause();
      active = true;
      compute(ev, base);
    };
    const gesture=beginTimelineEdit(e,ctx,{
      move,
      release:(ev)=>{if(!active)return;compute(ev,base);return plan},
    });
  };
  return { onTransitionDown };
}
