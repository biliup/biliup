// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Trim, split and delete planners (edit.ts), ripple delete and group trims.
import { overlaps } from "@/app/lib/editor/core/visual/transitions";
import { busySpans, rippleMainGaps } from "@/app/lib/editor/core/timeline/arrange";
import type { Op } from "@/app/lib/editor/core/timeline/types";
import { type Track, type ProjectModel, uniqueId } from "@/app/lib/editor/state/project";
import { type Plan, findItem, type Placed, lockOf, dropTransitions, draftOf, withGroups } from "@/app/ui/editor/timeline/edit/base";
import { type Snap, nearestSnap } from "@/app/ui/editor/timeline/edit/snap";

/** Free-track items other than `exclude`, in time order (array order on a free track isn't time). */
const others = (track: Track, exclude: string) => busySpans(track, exclude);

/**
 * Ripple delete (T7): delete the items and close the gaps they leave. On main the clips after them
 * move left by their length (D224: a gap that was before a deleted clip stays, before the next
 * clip); on a free track the later items on the same track move left by the deleted length
 * (as long as nothing else was in the gap, which never happens: items there can't overlap).
 */
export function planRippleDelete(project: ProjectModel, ids: ReadonlyArray<string>): Plan | { readonly problem: string } {
  const placed = [...new Set(ids)].map((id) => findItem(project, id)).filter((p): p is Placed => p !== undefined);
  if (!placed.length) return { problem: "Select an item to delete it." };
  for (const p of placed) {
    const locked = lockOf(p);
    if (locked) return { problem: locked };
  }
  const gone = new Set(placed.map((p) => p.item.id));
  const ops: Array<Op> = [];
  const nextOnMain = placed.filter((p) => p.track.role === "main").map((p) => p.track.items[p.index + 1]).filter((i) => i && !gone.has(i.id));
  ops.push(...dropTransitions(nextOnMain));
  for (const p of placed) ops.push({ op: "delete_item", item: p.item.id });
  const main = project.tracks.find((t) => t.role === "main");
  if (main) ops.push(...rippleMainGaps(main, gone));
  // Free tracks: each later item moves left by the deleted length before it.
  for (const track of project.tracks) {
    if (track.role === "main") continue;
    const removed = placed.filter((p) => p.track.id === track.id);
    if (!removed.length) continue;
    for (const item of track.items) {
      if (gone.has(item.id)) continue;
      // Retained linked layers follow their leader through withLinkedAudio. Moving them
      // explicitly here would detach them and ripple their timing a second time.
      if (item.linkedTo && !gone.has(item.linkedTo)) continue;
      const start = item.start ?? 0;
      const back = removed.filter((r) => r.start + r.item.duration <= start).reduce((n, r) => n + r.item.duration, 0);
      if (back > 0) {
        if (item.locked || track.locked) return { problem: "A locked item after it can't move to close the gap." };
        ops.push({ op: "move_item", item: item.id, start: start - back });
      }
    }
  }
  return { ops, summary: placed.length > 1 ? `Ripple delete ${placed.length} items` : `Ripple delete ${placed[0]!.item.id}` };
}

export interface TrimPlan extends Plan {
  /** The item's start and length after the trim, as the timeline shows them. */
  readonly start: number;
  readonly duration: number;
  readonly snapAt?: number;
}

/**
 * Drag an edge by `delta` frames (positive: to the right). Clamped to at least one frame, to
 * the media's length for video and audio (left edge: `in` can't go below 0; right edge: not past
 * the last frame), to frame 0 and the neighbours on a free track. On main the start stays and
 * the clips after it ripple. The moving edge snaps first, then is clamped.
 */
export function planTrim(project: ProjectModel, itemId: string, edge: "start" | "end", delta: number, snap?: Snap): TrimPlan | { readonly problem: string } {
  const placed = findItem(project, itemId);
  if (!placed) return { problem: "That item isn't in the project any more." };
  const locked = lockOf(placed);
  if (locked) return { problem: locked };
  const { item, track, start } = placed;
  const end = start + item.duration;
  const main = track.role === "main";
  const media = item.media ? project.media.find((m) => m.id === item.media) : undefined;
  const busy = main ? [] : others(track, item.id);
  let moved = Math.round(delta);
  const snapped = nearestSnap([edge === "start" ? start + moved : end + moved], snap);
  if (snapped) moved += snapped.shift;
  // How short the item may get: one frame, its animations and fades, a transition at either cut
  // needs both clips twice its length. The edit engine preserves key values at either cut.
  const next = main ? track.items[placed.index + 1] : undefined;
  const own = main ? (item.transitionIn?.duration ?? 0) : 0;
  const into = next?.transitionIn?.duration ?? 0;
  // The media an overlapping transition needs past the cut (a dip to black needs none).
  const ownHandle = main && item.transitionIn && overlaps(item.transitionIn.module) ? Math.ceil(own / 2) : 0;
  const intoHandle = next?.transitionIn && overlaps(next.transitionIn.module) ? Math.ceil(into / 2) : 0;
  const anims = Object.values(item.animation ?? {}).map((a) => a?.duration ?? 0);
  const minDuration = Math.max(1, 2 * own, 2 * into, (item.fadeIn ?? 0) + (item.fadeOut ?? 0), ...anims);
  const speed = item.speed ?? 1;
  // Video and audio play from a time in their media; still images and text can grow freely.
  const timed = media ? media.kind !== "image" : item.in !== undefined;
  const frames = timed ? media?.probe?.frames : undefined;
  let lo: number;
  let hi: number;
  if (edge === "start") {
    // moved > 0 shortens from the left. A transition into this clip needs media before its in-point.
    hi = item.duration - minDuration;
    lo = -Infinity;
    if (timed) lo = Math.max(lo, Math.ceil(((media?.kind === "video" ? Math.ceil(ownHandle * speed) : 0) - (item.in ?? 0)) / speed));
    if (!main) lo = Math.max(lo, -start, ...busy.filter((b) => b.end <= start).map((b) => b.end - start));
  } else {
    // A transition out of this clip needs media after its out-point.
    lo = minDuration - item.duration;
    hi = Infinity;
    // A held clip (D94) may run on past its media, unless an overlapping transition out of it needs media there.
    if (frames && !item.loop && !(item.hold && intoHandle === 0)) hi = Math.floor((frames - (item.in ?? 0)) / speed - item.duration - (media?.kind === "video" ? intoHandle : 0));
    if (!main) hi = Math.min(hi, ...busy.filter((b) => b.start >= end).map((b) => b.start - end));
  }
  // Already outside the limits (e.g. a project edited elsewhere): don't make it worse.
  lo = Math.min(lo, 0);
  hi = Math.max(hi, 0);
  const clamped = Math.min(hi, Math.max(lo, moved));
  const snapAt = snapped && clamped === moved ? snapped.at : undefined;
  const trim = edge === "start" ? clamped : -clamped;
  const after = { start: edge === "start" && !main ? start + clamped : start, duration: item.duration - trim, ...(snapAt !== undefined ? { snapAt } : {}) };
  if (clamped === 0) return { ops: [], summary: "", ...after };
  return { ops: [{ op: "trim_item", item: item.id, edge, frames: trim }], summary: `Trim ${item.id}`, ...after };
}

export interface SplitPlan extends Plan {
  readonly newId: string;
}

/**
 * Split at the playhead (D52): the selected item if the playhead is inside it (not on its first
 * frame), otherwise the main-track clip under the playhead. With an item selected that the
 * playhead isn't over, nothing is split (and the user is told), so a split never lands on
 * something the user didn't mean.
 */
export function planSplit(project: ProjectModel, selected: string | undefined, frame: number): SplitPlan | { readonly problem: string } {
  const f = Math.floor(frame);
  const inside = (p: Placed) => p.start < f && f < p.start + p.item.duration;
  // Selected captions (D82) aren't split: they follow the main track, so S splits the clip there.
  const picked = findItem(project, selected);
  const sel = picked?.item.type === "captions" ? undefined : picked;
  let target: Placed | undefined;
  if (sel) {
    if (!inside(sel)) return { problem: "Move the playhead over the selected item to split it." };
    target = sel;
  } else {
    const main = project.tracks.find((t) => t.role === "main");
    const item = main?.items.find((i) => inside(findItem(project, i.id)!));
    target = item && findItem(project, item.id);
    if (!target) return { problem: "Put the playhead over a clip on the main track (or select an item) to split it." };
  }
  return splitAt(project, target, f, "Move the playhead further from it.");
}

/** Split `target` at frame `f` (inside it): locks and transitions checked (planSplit, planBladeSplit). */
function splitAt(project: ProjectModel, target: Placed, f: number, further: string): SplitPlan | { readonly problem: string } {
  const locked = lockOf(target);
  if (locked) return { problem: locked };
  // D248: a linked sound (or picture) splits with its clip (D123); a locked one would stay whole,
  // still following only the first half, so the cut is refused and says why.
  for (const t of project.tracks) {
    for (const i of t.items) {
      if (!("linkedTo" in i) || i.linkedTo !== target.item.id || (!t.locked && !i.locked)) continue;
      const what = i.type === "audio" ? "linked sound" : "linked clip";
      return { problem: t.locked ? `Its ${what} is on the ${t.name} track, which is locked. Unlock it to cut the clip.` : `Its ${what} is locked. Unlock it to cut the clip.` };
    }
  }
  // A transition at either cut needs the half next to it to be at least twice its length.
  if (target.track.role === "main") {
    const at = f - target.start;
    const own = target.item.transitionIn?.duration ?? 0;
    const into = target.track.items[target.index + 1]?.transitionIn?.duration ?? 0;
    if (at < 2 * own || target.item.duration - at < 2 * into) return { problem: `That's too close to a transition to split there. ${further}` };
  }
  const newId = uniqueId(project, target.item.id.replace(/-\d+$/, "") || "item");
  return { ops: [{ op: "split_item", item: target.item.id, at: f - target.start, newId }], summary: `Split ${target.item.id}`, newId };
}

/**
 * The blade (issue #57, D230): cut the item that was pressed at `frame` (whole frames; snap it
 * first with bladeFrame). One split_item, so one edit and one ⌘Z; a linked sound splits with its
 * clip (D123). Locked tracks and items refuse; on an item's edge there's nothing to cut (no ops).
 * Captions follow the speech and aren't cut.
 */
export function planBladeSplit(project: ProjectModel, itemId: string, frame: number): SplitPlan | { readonly problem: string } {
  const target = findItem(project, itemId);
  if (!target) return { problem: "That item isn't in the project any more." };
  if (target.item.type === "captions") return { problem: "Captions follow the speech: cut the clip under them instead." };
  const locked = lockOf(target);
  if (locked) return { problem: locked };
  const f = Math.round(frame);
  if (!(target.start < f && f < target.start + target.item.duration)) return { ops: [], summary: "", newId: "" };
  return splitAt(project, target, f, "Cut further from it.");
}

/**
 * Trim a group (D157): the dragged item's edge moves by `delta` (planTrim, snapping and limits as
 * for one item), then every other member's same edge by the same frames, each within its own
 * limits; one edit. No ops when nothing moves.
 */
export function planGroupTrim(project: ProjectModel, itemId: string, edge: "start" | "end", delta: number, snap?: Snap): TrimPlan | { readonly problem: string } {
  const first = planTrim(project, itemId, edge, delta, snap);
  if ("problem" in first || !first.ops.length) return first;
  const placed = findItem(project, itemId)!;
  const moved = edge === "end" ? first.duration - placed.item.duration : first.start - placed.start;
  let current = draftOf(project, first.ops);
  if ("problem" in current) return first;
  const ops = [...first.ops];
  for (const id of withGroups(project, [itemId]).filter((id) => id !== itemId)) {
    const p = planTrim(current.project, id, edge, moved);
    if ("problem" in p || !p.ops.length) continue;
    const next = draftOf(current.project, p.ops);
    if ("problem" in next) continue;
    current = next;
    ops.push(...p.ops);
  }
  return { ...first, ops, summary: ops.length > first.ops.length ? `Trim group ${placed.item.group}` : first.summary };
}
