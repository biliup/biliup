// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Overlays following main-track changes (F11) and grouping (D157), for the timeline planners (edit.ts).
import type { Op } from "@/app/lib/editor/core/timeline/types";
import { type ProjectModel, layoutOf, type Track } from "@/app/lib/editor/state/project";
import { type Plan, draftOf, withGroups, findItem, type Placed, lockOf } from "@/app/ui/editor/timeline/edit/base";

/**
 * Overlays follow the footage under them (F11, D156). When `plan` changes main clips' lengths (a
 * speed, a trim), every item on another track that starts over a main clip keeps its moment of
 * that clip's footage: its new start is where that source frame plays now (clamped into the clip
 * when the frame was trimmed away); one that starts after the main track's end keeps its distance
 * from that end. Items keep their lengths; captions (they follow the speech already), sounds linked
 * to a clip (D123) and locked items stay. If the moves don't fit (an overlap), the plan is returned
 * as it was. The moves join the plan's ops: one edit, one ⌘Z.
 */
export function withOverlayRipple<P extends Plan>(project: ProjectModel, plan: P): P {
  if (!plan.ops.length) return plan;
  const draft = draftOf(project, plan.ops);
  if ("problem" in draft) return plan;
  const after = draft.project;
  const mainBefore = project.tracks.find((t) => t.role === "main");
  const mainAfter = after.tracks.find((t) => t.role === "main");
  if (!mainBefore || !mainAfter) return plan;
  const before = layoutOf(project).starts;
  const now = layoutOf(after).starts;
  const endOf = (track: Track, starts: ReadonlyMap<string, number>) => track.items.reduce((n, i) => Math.max(n, (starts.get(i.id) ?? 0) + i.duration), 0);
  const end0 = endOf(mainBefore, before);
  const end1 = endOf(mainAfter, now);
  const changed = mainBefore.items.some((i) => {
    const j = mainAfter.items.find((x) => x.id === i.id);
    return !j || j.duration !== i.duration || (j.speed ?? 1) !== (i.speed ?? 1) || (j.in ?? 0) !== (i.in ?? 0) || before.get(i.id) !== now.get(i.id);
  });
  if (!changed) return plan;
  /** Where frame `f` of the old timeline plays in the new one. */
  const map = (f: number): number => {
    for (const clip of mainBefore.items) {
      const s0 = before.get(clip.id) ?? 0;
      if (f < s0 || f >= s0 + clip.duration) continue;
      const next = mainAfter.items.find((x) => x.id === clip.id);
      if (!next) return f;
      const s1 = now.get(next.id) ?? 0;
      // The source frame under f, and where it is in the clip now.
      const source = (clip.in ?? 0) + (f - s0) * (clip.speed ?? 1);
      const local = (source - (next.in ?? 0)) / (next.speed ?? 1);
      return s1 + Math.min(next.duration - 1, Math.max(0, Math.round(local)));
    }
    return f >= end0 ? f + (end1 - end0) : f;
  };
  const moves: Array<Op> = [];
  const touched = new Set(plan.ops.flatMap((o) => ("item" in o && typeof o.item === "string" ? [o.item] : [])));
  for (const track of after.tracks) {
    if (track.role === "main" || track.locked) continue;
    for (const item of track.items) {
      if (item.type === "captions" || item.linkedTo || item.locked || touched.has(item.id)) continue;
      const s = item.start ?? 0;
      const to = Math.max(0, map(s));
      if (to !== s) moves.push({ op: "move_item", item: item.id, start: to });
    }
  }
  if (!moves.length) return plan;
  const ops = [...plan.ops, ...moves];
  if ("problem" in draftOf(project, ops)) return plan;
  return { ...plan, ops, summary: `${plan.summary} (${moves.length === 1 ? "1 overlay follows" : `${moves.length} overlays follow`})` };
}

/** Group the selected free-track items, or ungroup the group of `ids` (D157). */
export function planGrouping(project: ProjectModel, ids: ReadonlyArray<string>, group: boolean): Plan | { readonly problem: string } {
  const placed = withGroups(project, ids).map((id) => findItem(project, id)).filter((p): p is Placed => p !== undefined);
  for (const p of placed) {
    const locked = lockOf(p);
    if (locked) return { problem: locked };
  }
  if (group) {
    const free = placed.filter((p) => p.track.role !== "main" && p.item.type !== "captions");
    if (free.length < 2) return { problem: "Select two or more items off the main track to group them." };
    const id = `group-${free[0]!.item.id}`;
    return { ops: free.map((p) => ({ op: "update_item", item: p.item.id, set: { group: id } }) as Op), summary: `Group ${free.length} items` };
  }
  const grouped = placed.filter((p) => p.item.group);
  if (!grouped.length) return { problem: "That isn't a group." };
  return { ops: grouped.map((p) => ({ op: "update_item", item: p.item.id, set: { group: null } }) as Op), summary: `Ungroup ${grouped.length} items` };
}
