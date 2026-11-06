// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Pressing and dragging timeline items: select (⇧ range, ⌘ toggle), move, trim, fade, group moves (D52, D122, D157).
import type React from "react";
import { primaryModifier } from '@/app/lib/editor/keyboard';
import { getBeats } from "@/app/ui/editor/audio/beats";
import { planFade } from "@/app/ui/editor/audio/sound";
import { draftOf, findItem, lockOf, newTrackIndexOver, withGroups, withOverlayRipple, planGroupMove, planGroupTrim, planMove, planMoveToNewTrack, planTrim, SNAP_PX, snapTargets, trimSnapTargets, type Plan } from "@/app/ui/editor/timeline/edit";
import { frameTimecode } from "@/app/lib/editor/state/format";
import { getPlayback, playback } from "@/app/ui/editor/preview/playback";
import type { Item, Layout, ProjectModel, Track } from "@/app/lib/editor/state/project";
import { beginTimelineEdit } from './session';
import { DRAG_PX } from "@/app/ui/editor/timeline/ItemBox";
import { type DragView, selectionIds } from "@/app/ui/editor/timeline/gestures/dragView";
import type { LaneGeometry } from "@/app/ui/editor/timeline/gestures/lanes";

export interface ItemDragCtx {
  readonly project: ProjectModel;
  readonly fps: number;
  readonly ppf: number;
  readonly online: boolean;
  readonly starts: Layout["starts"];
  readonly tracks: ReadonlyArray<Track>;
  readonly laneOrder: ReadonlyArray<string>;
  readonly lanesRef: React.RefObject<HTMLDivElement | null>;
  readonly laneEls: LaneGeometry["laneEls"];
  readonly laneHit: LaneGeometry["laneHit"];
  readonly laneAt: LaneGeometry["laneAt"];
  readonly laneIndexAt: LaneGeometry["laneIndexAt"];
  readonly setDrag: React.Dispatch<React.SetStateAction<DragView | undefined>>;
  readonly stopDrag: React.RefObject<(() => void) | undefined>;
}

export function makeItemDrag(ctx: ItemDragCtx) {
  const { project, fps, ppf, online, starts, tracks, laneOrder, lanesRef, laneEls, laneHit, laneAt, laneIndexAt } = ctx;
  /**
   * Press on an item: select it (⇧: a range, ⌘: add or remove it). Moving past DRAG_PX starts a
   * move (or a trim, from an edge handle; a group move when it's one of several selected); each
   * pointer move re-plans against the project as it was at the press and redraws (once per
   * animation frame); the release sends the plan as one edit, based on that revision.
   */
  const onItemDown = (e: React.PointerEvent, item: Item, track: Track) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    ctx.stopDrag.current?.();
    // ⌘-click adds or removes the item; ⌘ held while dragging only turns snapping off.
    const toggle = primaryModifier(e);
    const toggleIt = () => {
      const ids = selectionIds();
      playback.selectMany(ids.includes(item.id) ? ids.filter((i) => i !== item.id) : [...ids, item.id]);
    };
    if (e.shiftKey) {
      const anchor = findItem(project, getPlayback().selected);
      const here = findItem(project, item.id);
      if (!anchor || !here) return playback.select(item.id);
      // A range: everything between the two in time, on the lanes between them.
      const from = Math.min(anchor.start, here.start);
      const to = Math.max(anchor.start + anchor.item.duration, here.start + here.item.duration);
      const [l0, l1] = [laneOrder.indexOf(anchor.track.id), laneOrder.indexOf(here.track.id)].sort((a, b) => a - b);
      const ids = [anchor.item.id];
      for (const t of tracks.slice(l0, l1 + 1))
        for (const i of t.items) {
          const s = starts.get(i.id) ?? 0;
          if (i.type !== "captions" && i.id !== anchor.item.id && s < to && s + i.duration > from) ids.push(i.id);
        }
      playback.selectMany(ids);
      return;
    }
    // D157: an item in a group brings its group along (selected, moved and trimmed together).
    const before = selectionIds();
    const mates = withGroups(project, [item.id]);
    const current = toggle ? before : before.includes(item.id) ? withGroups(project, before) : mates;
    const grouped = !toggle && current.length > 1 && current.includes(item.id);
    // Just its own group: its edges trim the whole group.
    const groupOnly = grouped && mates.length > 1 && current.length === mates.length && current.every((id) => mates.includes(id));
    if (!grouped && !toggle) playback.select(item.id);
    else if (grouped && (!before.includes(item.id) || current.length !== before.length)) playback.selectMany([item.id, ...current.filter((id) => id !== item.id)]);
    const found = findItem(project, item.id);
    const press = found && { ...found, base: project };
    // Captions (D82) follow the speech on the main track: selected here, edited in the Inspector.
    if (!press || lockOf(press) || !online || item.type === "captions") {
      if (toggle) toggleIt();
      return;
    }
    e.preventDefault();
    const edge = grouped && !groupOnly ? undefined : ((e.target as HTMLElement).dataset.edge as "start" | "end" | undefined);
    // A fade handle (top corners of an item with sound): drag sets the fade's length.
    const fade = grouped ? undefined : ((e.target as HTMLElement).dataset.fade as "in" | "out" | undefined);
    const x0 = e.clientX;
    const lane0 = laneIndexAt(e.clientY);
    const pressEdge = edge === "end" ? press.start + press.item.duration : press.start;
    let active = false;
    let plan: Plan | undefined;
    let raf = 0;
    let last: PointerEvent | undefined;
    /** Plan from the pointer against `base`, and draw it (or why it can't be dropped there). */
    const compute = (ev: PointerEvent, base: ProjectModel) => {
      plan = undefined;
      const placed = findItem(base, item.id);
      if (!placed) return;
      const delta = (ev.clientX - x0) / ppf;
      const snapOn = getPlayback().snapping && !primaryModifier(ev);
      const threshold = SNAP_PX / ppf;
      const beats = { beats: getBeats().map, ppf };
      if (grouped && !edge) {
        const lane = laneIndexAt(ev.clientY);
        const offset = lane === -1 || lane0 === -1 ? 0 : lane - lane0;
        const ids = current;
        const snap = snapOn ? { targets: snapTargets(base, getPlayback().frame, new Set(ids), beats), threshold } : undefined;
        const p = planGroupMove(base, ids, delta, offset, laneOrder, snap);
        const draft = "problem" in p ? p : draftOf(base, p.ops);
        if (!("problem" in draft) && !("problem" in p)) plan = p;
        const tipAt = Math.max(0, placed.start + ("problem" in p ? delta : p.delta));
        gesture?.show({
          itemId: item.id,
          mode: "group",
          group: ids,
          view: "problem" in draft ? base : draft.project,
          ...(!("problem" in p) && p.snapAt !== undefined ? { snapAt: p.snapAt } : {}),
          ...("problem" in draft ? { refused: draft.problem } : {}),
          tipAt,
          tip: "problem" in draft ? draft.problem : `${ids.length} items, ${frameTimecode(Math.round(tipAt), fps)}`,
        });
        return;
      }
      if (fade) {
        const x = (ev.clientX - (lanesRef.current?.getBoundingClientRect().left ?? 0)) / ppf;
        const frames = fade === "in" ? x - placed.start : placed.start + placed.item.duration - x;
        const p = planFade(base, item.id, fade, frames);
        if ("problem" in p) return gesture?.show({itemId:item.id,mode:'fade',view:base,refused:p.problem,tipAt:placed.start,tip:p.problem});
        const draft = draftOf(base, p.ops);
        if (!("problem" in draft)) plan = p;
        const shownItem = ("project" in draft ? findItem(draft.project, item.id)?.item : undefined) ?? placed.item;
        const length = (fade === "in" ? shownItem.fadeIn : shownItem.fadeOut) ?? 0;
        gesture?.show({
          itemId: item.id,
          mode: "fade",
          view: "problem" in draft ? base : draft.project,
          ...("problem" in draft ? { refused: draft.problem } : {}),
          tipAt: fade === "in" ? placed.start + length : placed.start + placed.item.duration - length,
          tip: "problem" in draft ? draft.problem : `Fade ${fade} ${(length / fps).toFixed(1)}s`,
        });
        return;
      }
      if (edge) {
        // Plan the edge against the same project that the user pressed.
        const own = edge === "end" ? placed.start + placed.item.duration : placed.start;
        const snap = snapOn ? { targets: trimSnapTargets(base, getPlayback().frame, item.id, edge, beats, pressEdge + delta), threshold } : undefined;
        const trimmed = groupOnly ? planGroupTrim(base, item.id, edge, pressEdge + delta - own, snap) : planTrim(base, item.id, edge, pressEdge + delta - own, snap);
        if ("problem" in trimmed) return gesture?.show({itemId:item.id,mode:edge,view:base,refused:trimmed.problem,tipAt:Math.max(0,pressEdge+delta),tip:trimmed.problem});
        // F11 (D156): overlays over a main clip follow its footage.
        const p = placed.track.role === "main" && getPlayback().linkOverlays ? withOverlayRipple(base, trimmed) : trimmed;
        const draft = draftOf(base, p.ops);
        if (!("problem" in draft)) plan = p;
        // A main clip trimmed from the left keeps its start (the rest ripples on release); while
        // dragging, its left edge follows the pointer so the user sees what they cut.
        const mainLeft = edge === "start" && placed.track.role === "main";
        const shown = mainLeft ? { start: placed.start + placed.item.duration - p.duration, duration: p.duration } : undefined;
        gesture?.show({
          itemId: item.id,
          mode: edge,
          view: mainLeft || "problem" in draft ? base : draft.project,
          ...(shown ? { ghost: shown } : {}),
          ...(p.snapAt !== undefined ? { snapAt: p.snapAt } : {}),
          ...("problem" in draft ? { refused: draft.problem } : {}),
          tipAt: edge === "start" ? (shown?.start ?? p.start) : p.start + p.duration,
          tip: "problem" in draft ? draft.problem : `${(p.duration / fps).toFixed(2)}s`,
        }, "problem" in draft ? base : draft.project);
        return;
      }
      const desired = press.start + delta;
      const snap = snapOn ? { targets: snapTargets(base, getPlayback().frame, item.id, beats), threshold } : undefined;
      // Past the top or bottom lane: onto a new track there (D122). Onto a lane that can't take it
      // (a clip from Main onto a text track): a new track next to that lane (F6, D149).
      const hit = laneHit(ev.clientY);
      const side = hit.side;
      const between = !side && hit.track && hit.track.id !== placed.track.id ? newTrackIndexOver(base, placed.item, hit.track) : undefined;
      const lanesTop = lanesRef.current?.getBoundingClientRect().top ?? 0;
      const laneRect = (id: string) => laneEls().find((l) => l.dataset.track === id)?.getBoundingClientRect();
      const lineY = (): number => {
        const lanes = laneEls();
        if (side === "top") return (lanes[0]?.getBoundingClientRect().top ?? lanesTop) - lanesTop;
        if (side === "bottom") return (lanes.at(-1)?.getBoundingClientRect().bottom ?? lanesTop) - lanesTop - 2;
        const r = laneRect(hit.track!.id);
        return r ? (placed.item.type === "text" ? r.top : r.bottom) - lanesTop - 1 : 0;
      };
      const p = side ? planMoveToNewTrack(base, item.id, side, desired, snap) : between !== undefined ? planMoveToNewTrack(base, item.id, { index: between }, desired, snap) : planMove(base, item.id, laneAt(ev.clientY, item) ?? placed.track.id, desired, snap);
      const newTrack = side || between !== undefined ? { side: side ?? ("between" as const), y: lineY() } : undefined;
      if ("problem" in p) return gesture?.show({ itemId: item.id, mode: "move", view: base, refused: p.problem, tipAt: Math.max(0, desired), tip: p.problem });
      const draft = draftOf(base, p.ops);
      if (!("problem" in draft)) plan = p;
      gesture?.show({
        itemId: item.id,
        mode: "move",
        view: "problem" in draft ? base : draft.project,
        ...(newTrack ? { newTrack } : { float: { track: p.track, start: Math.max(0, desired) } }),
        ...(p.snapAt !== undefined ? { snapAt: p.snapAt } : {}),
        ...("problem" in draft ? { refused: draft.problem } : {}),
        tipAt: Math.max(0, desired),
        tip: "problem" in draft ? draft.problem : newTrack ? `New track, ${frameTimecode(p.start ?? 0, fps)}` : p.start !== undefined ? frameTimecode(p.start, fps) : `Position ${p.index! + 1}`,
      });
    };
    const move = (ev: PointerEvent) => {
      if (!active && Math.abs(ev.clientX - x0) < DRAG_PX && Math.abs(ev.clientY - e.clientY) < DRAG_PX) return;
      if (!active) {
        playback.pause();
        if (toggle) playback.select(item.id);
      }
      active = true;
      last = ev;
      if (!raf) {
        raf = requestAnimationFrame(() => {
          raf = 0;
          if (last) compute(last, press.base);
        });
      }
    };
    const gesture = beginTimelineEdit(e,ctx,{
      move,
      stop:()=>{cancelAnimationFrame(raf);raf=0},
      release:(ev)=>{
        // A click on one of several selected items selects just it.
        if(!active){
          if(toggle)toggleIt();
          else if(grouped)mates.length>1?playback.selectMany(mates):playback.select(item.id);
          return;
        }
        compute(ev,press.base);
        return plan;
      },
    });
  };
  return { onItemDown };
}
