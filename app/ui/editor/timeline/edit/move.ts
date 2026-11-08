// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Move planners (edit.ts): one item, onto a new track, a group, and the layer order (F15).
import { gapOpsFor, mainSlot, mainStartsById, nearestFreeSpot, planPutOnMain, shiftMainItems } from "@/app/lib/editor/core/timeline/arrange";
import type { Op } from "@/app/lib/editor/core/timeline/types";
import { type ProjectModel, uniqueId, type Item, type Track } from "@/app/lib/editor/state/project";
import { type Plan, findItem, lockOf, fitsTrack, lockedProblem, dropTransitions, draftOf, TRACK_KINDS, type Placed } from "@/app/ui/editor/timeline/edit/base";
import { type Snap, nearestSnap } from "@/app/ui/editor/timeline/edit/snap";
import { editorText } from "@/app/lib/editor/i18n";

export interface MovePlan extends Plan {
  /** Where the item lands: its track, and its start (free) or index (main). */
  readonly track: string;
  readonly start?: number;
  readonly index?: number;
  /** The frame it snapped to, for the snap line. */
  readonly snapAt?: number;
}

/**
 * Move an item to `desiredStart` on track `trackId` (its own track, or a compatible one).
 * Free tracks: the start snaps (either edge), then moves to the nearest free spot if something
 * is in the way. Main (D224): the start snaps too, the item goes into the slot its middle is
 * over and lands at that frame (or right after the clip before it), with gaps around it; the
 * others stay put unless it lands on them (they're pushed later). No ops when it would land where
 * it is.
 */
export function planMove(project: ProjectModel, itemId: string, trackId: string, desiredStart: number, snap?: Snap): MovePlan | { readonly problem: string } {
  const placed = findItem(project, itemId);
  if (!placed) return { problem: "That item isn't in the project any more." };
  const locked = lockOf(placed);
  if (locked) return { problem: locked };
  const target = project.tracks.find((t) => t.id === trackId);
  if (!target) return { problem: "That track isn't in the project any more." };
  if (!fitsTrack(placed.item, target)) return { problem: `A ${placed.item.type} item can't go on the ${target.name} track.` };
  if (target.locked) return { problem: lockedProblem(target) };
  const { item, track } = placed;
  const onTrack = target.id === track.id ? {} : { track: target.id };
  if (target.role === "main") {
    // D224: main clips go where they're dropped (snapped like any item) and leave their space;
    // the others stay where they are, unless the clip lands on one: then it and the ones after it
    // that it reaches are pushed later. The order is the slot its middle is over among the others
    // (the index is after the item is taken out: move_item removes it first).
    let start = Math.max(0, Math.round(desiredStart));
    const snapped = nearestSnap([start, start + item.duration], snap);
    if (snapped) start = Math.max(0, start + snapped.shift);
    const before = mainStartsById(target);
    const restItems = target.items.filter((i) => i.id !== item.id);
    // The others where they are now (with the clip taken out: their frames, not their gaps).
    let end = 0;
    const rest = restItems.map((i) => {
      const s = before.get(i.id)!;
      const placedRest = { ...i, gap: s - end };
      end = s + i.duration;
      return placedRest;
    });
    const index = mainSlot(rest, start + item.duration / 2);
    const order = [...restItems.slice(0, index), item, ...restItems.slice(index)];
    const gaps = gapOpsFor(order, (i) => (i.id === item.id ? start : before.get(i.id)!), new Set([item.id]));
    // Not before the clip it follows: then it butts up to it.
    const landed = gaps.starts.get(item.id)!;
    const snapAt = snapped && landed === start ? snapped.at : undefined;
    const at = { track: target.id, index, start: landed, ...(snapAt !== undefined ? { snapAt } : {}) };
    if (target.id === track.id && index === placed.index && landed === placed.start) return { ops: [], summary: "", ...at };
    // The cuts the clip leaves and joins change, so the transitions at them go (D52): its own,
    // the one into the clip after it, and the one into the clip it lands before; and any clip that
    // now follows a gap (the rules refuse a transition across one).
    const following = new Set(gaps.ops.flatMap((o) => (o.op === "update_item" && (o.set as { gap?: unknown }).gap !== null ? [o.item] : [])));
    const cut = [item, track.role === "main" ? track.items[placed.index + 1] : undefined, restItems[index], ...restItems.filter((i) => following.has(i.id))];
    // Same track, same order: only the gaps change (no move_item, so the clip keeps its links).
    const moveOp: Array<Op> = target.id === track.id && index === placed.index ? [] : [{ op: "move_item", item: item.id, ...onTrack, index }];
    const cleared = moveOp.length ? gaps.ops : gapOpsFor(order, (i) => (i.id === item.id ? landed : before.get(i.id)!)).ops;
    return { ops: [...dropTransitions(cut), ...moveOp, ...cleared], summary: `Move ${item.id}`, ...at };
  }
  let start = Math.max(0, Math.round(desiredStart));
  const snapped = nearestSnap([start, start + item.duration], snap);
  if (snapped) start = Math.max(0, start + snapped.shift);
  const landing = nearestFreeSpot(target, item.id, start, item.duration);
  const snapAt = snapped && landing === start ? snapped.at : undefined;
  if (target.id === track.id && landing === placed.start) return { ops: [], summary: "", track: target.id, start: landing, ...(snapAt !== undefined ? { snapAt } : {}) };
  // Off main: the clip after it loses the transition from it (its own goes with the move).
  const leaving = track.role === "main" ? dropTransitions([track.items[placed.index + 1]]) : [];
  return {
    ops: [...leaving, { op: "move_item", item: item.id, ...onTrack, start: landing }],
    summary: `Move ${item.id}`,
    track: target.id,
    start: landing,
    ...(snapAt !== undefined ? { snapAt } : {}),
  };
}

/** Where a dragged item lands when it's dropped above the top lane or below the bottom one: a new track there (D122). */
export type NewTrackSide = "top" | "bottom";

/**
 * Move an item onto a new track (D122): dragged above the top lane a new track of its kind goes on
 * top of the others; below the bottom lane, under them (a video or text track just above the audio
 * tracks, an audio track at the very bottom). `{ index }` (F6, D149): the new track goes at that
 * index of project.tracks (the timeline passes it when the item is dragged onto a lane that can't
 * take it, e.g. a clip from Main onto a text track: a new video track under that text track). It
 * keeps its start (snapped like a move).
 */
export function planMoveToNewTrack(project: ProjectModel, itemId: string, side: NewTrackSide | { readonly index: number }, desiredStart: number, snap?: Snap): MovePlan | { readonly problem: string } {
  const placed = findItem(project, itemId);
  if (!placed) return { problem: "That item isn't in the project any more." };
  const locked = lockOf(placed);
  if (locked) return { problem: locked };
  const { item, track } = placed;
  const kind = TRACK_KINDS[item.type];
  if (item.type === "captions") return { problem: "Captions stay on their own track." };
  let start = Math.max(0, Math.round(desiredStart));
  const snapped = nearestSnap([start, start + item.duration], snap);
  if (snapped) start = Math.max(0, start + snapped.shift);
  const id = uniqueId(project, kind === "audio" ? "audio-track" : kind === "text" ? "text-track" : "video-track");
  const used = new Set(project.tracks.map((t) => t.name));
  const base = editorText(track.purpose === "subtitles" ? "Subtitles" : kind === "audio" ? "Audio" : kind === "text" ? "Text" : "Video");
  let name = base;
  for (let n = 2; used.has(name); n++) name = `${base} ${n}`;
  // Audio tracks always show under the rest, so "top" for audio is just above the first audio track.
  const firstAudio = project.tracks.findIndex((t) => t.kind === "audio");
  const lastVisual = project.tracks.reduce((n, t, i) => (t.kind !== "audio" ? i : n), -1);
  const index =
    typeof side === "object"
      ? Math.max(0, Math.min(project.tracks.length, side.index))
      : kind === "audio"
        ? side === "top"
          ? firstAudio === -1
            ? project.tracks.length
            : firstAudio
          : project.tracks.length
        : side === "top"
          ? 0
          : lastVisual + 1;
  const leaving = track.role === "main" ? dropTransitions([track.items[placed.index + 1]]) : [];
  return {
    ops: [{ op: "add_track", track: { id, kind, name, ...(track.purpose ? {purpose:track.purpose} : {}) }, index }, ...leaving, { op: "move_item", item: item.id, track: id, start }],
    summary: `Move ${item.id} to a new track`,
    track: id,
    start,
    ...(snapped ? { snapAt: snapped.at } : {}),
  };
}

/**
 * Where a new track goes when an item is dragged onto a lane that can't take it (F6, D149): a
 * picture (clip, element, graphic) onto a text track lands on a new video track right under it (so
 * the text stays over it); a text onto a video track, on a new text track right over it. Nothing
 * for audio, captions, audio lanes or a lane that takes the item (that's a plain move).
 */
export function newTrackIndexOver(project: ProjectModel, item: Item, over: Track): number | undefined {
  const kind = TRACK_KINDS[item.type];
  if (item.type === "captions" || kind === "audio" || over.kind === "audio") return undefined;
  if (fitsTrack(item, over) && !over.locked) return undefined;
  const at = project.tracks.findIndex((t) => t.id === over.id);
  if (at === -1) return undefined;
  return kind === "text" ? at : at + 1;
}

export interface GroupMovePlan extends Plan {
  readonly snapAt?: number;
  /** The frames the group moved by (after snapping). */
  readonly delta: number;
}

/**
 * Move several selected items together (T8, D126): every one by the same number of frames (snapped
 * when any of their edges comes near a target that isn't in the group) and the same number of lanes
 * (`laneOffset`, in the timeline's lane order `lanes`), so they keep their offsets in time and
 * across tracks. Each free item must land on a free, compatible, unlocked track where nothing
 * outside the group is in the way; captions never move. Main clips in the group (issue #58, D229)
 * move along in time, staying on Main in their order with the spaces between them (core
 * shiftMainItems: the other main clips stay put unless one lands on them); then the group moves in
 * time only (the lane offset is ignored). Items linked to a main clip of the group (D123, D215)
 * get no op of their own: they follow their clip, still linked.
 */
export function planGroupMove(project: ProjectModel, ids: ReadonlyArray<string>, delta: number, laneOffset: number, lanes: ReadonlyArray<string>, snap?: Snap): GroupMovePlan | { readonly problem: string } {
  const placed = [...new Set(ids)].map((id) => findItem(project, id)).filter((p): p is Placed => p !== undefined && p.item.type !== "captions");
  if (!placed.length) return { problem: "Nothing to move." };
  for (const p of placed) {
    const locked = lockOf(p);
    if (locked) return { problem: locked };
  }
  const mains = placed.filter((p) => p.track.role === "main");
  const lanesBy = mains.length ? 0 : laneOffset;
  let shift = Math.round(delta);
  const minStart = Math.min(...placed.map((p) => p.start));
  shift = Math.max(shift, -minStart);
  const snapped = nearestSnap(placed.flatMap((p) => [p.start + shift, p.start + p.item.duration + shift]), snap);
  if (snapped) shift = Math.max(-minStart, shift + snapped.shift);
  // D215: a group dragged onto the main track goes on it (a take: its picture on Main, its sounds
  // linked under it) at the cut nearest where it's dropped, instead of being refused.
  const ontoMain = lanesBy ? placed.find((p) => {
    const from = lanes.indexOf(p.track.id);
    return from !== -1 && project.tracks.find((t) => t.id === lanes[from + lanesBy])?.role === "main";
  }) : undefined;
  if (ontoMain) {
    const put = planPutOnMain(project, ontoMain.item.id, { frame: ontoMain.start + shift });
    if ("problem" in put) return put;
    return { ops: put.ops, summary: put.summary, delta: shift };
  }
  const ops: Array<Op> = [];
  if (mains.length && shift !== 0) ops.push(...shiftMainItems(mains[0]!.track, new Set(mains.map((p) => p.item.id)), shift).ops);
  // Linked to a main clip that moves: withLinkedAudio carries it (a move_item would unlink it).
  const follows = (p: Placed) => typeof p.item.linkedTo === "string" && mains.some((m) => m.item.id === p.item.linkedTo);
  // D248: the free items land among the others where they'll be once the main clips have moved:
  // the sounds that follow them (selected or not, and those of main clips pushed later) included,
  // so nothing is refused for a sound that's moving away, and nothing lands on one that's coming
  // (withLinkedAudio would unlink it rather than fail).
  const projected = ops.length ? draftOf(project, ops) : { project };
  if ("problem" in projected) return projected;
  const moving = new Set(placed.filter((p) => p.track.role !== "main" && !follows(p)).map((p) => p.item.id));
  const landing = new Map<string, Array<{ start: number; end: number }>>();
  for (const p of placed) {
    if (p.track.role === "main" || follows(p)) continue;
    const from = lanes.indexOf(p.track.id);
    const toId = lanesBy && from !== -1 ? lanes[from + lanesBy] : p.track.id;
    const target = project.tracks.find((t) => t.id === toId);
    if (!target) return { problem: "There's no track there for all of them." };
    if (target.role === "main") return { problem: "A group can't be dropped on the main track: move clips onto it one at a time." };
    if (!fitsTrack(p.item, target)) return { problem: `A ${p.item.type} item can't go on the ${target.name} track.` };
    if (target.locked) return { problem: lockedProblem(target) };
    const start = p.start + shift;
    const end = start + p.item.duration;
    const there = projected.project.tracks.find((t) => t.id === target.id)?.items ?? target.items;
    const outside = there.filter((i) => !moving.has(i.id)).every((i) => end <= (i.start ?? 0) || start >= (i.start ?? 0) + i.duration);
    const mine = landing.get(target.id) ?? [];
    if (!outside || mine.some((b) => end > b.start && start < b.end)) return { problem: `Something on the ${target.name} track is in the way.` };
    mine.push({ start, end });
    landing.set(target.id, mine);
    if (shift !== 0 || target.id !== p.track.id) ops.push({ op: "move_item", item: p.item.id, ...(target.id !== p.track.id ? { track: target.id } : {}), start });
  }
  return { ops, summary: `Move ${placed.length} items`, delta: shift, ...(snapped && shift - Math.round(delta) === snapped.shift ? { snapAt: snapped.at } : {}) };
}

/**
 * Bring forward / Send backward (F15, D151): a picture or text on the canvas moves one layer up
 * (dir −1, towards the top of the track list, drawn over more) or down (+1). When the next track
 * that way is of its kind, unlocked, not main and free over its time, it moves there; otherwise it
 * goes onto a new track of its kind just past that track (so it passes it). A track it leaves empty
 * goes. Main clips stay where they are (the main track is the base), and so do captions and sounds.
 */
export function planLayer(project: ProjectModel, itemId: string, dir: -1 | 1): MovePlan | { readonly problem: string } {
  const placed = findItem(project, itemId);
  if (!placed) return { problem: "That item isn't in the project any more." };
  const locked = lockOf(placed);
  if (locked) return { problem: locked };
  const { item, track, start } = placed;
  if (track.role === "main") return { problem: "Clips on the main track are the base layer: drag one up off Main to put it over the others." };
  if (item.type === "captions" || track.kind === "audio") return { problem: "Only pictures and text have layers." };
  // Visual tracks in drawing order (top first), as the timeline lists them.
  const visual = project.tracks.filter((t) => t.kind !== "audio");
  const at = visual.findIndex((t) => t.id === track.id);
  const next = visual[at + dir];
  const alone = track.items.length === 1;
  if (!next && alone) return { problem: dir < 0 ? "It's already on top." : "It's already at the bottom." };
  const end = start + item.duration;
  const free = (t: Track) => t.items.every((i) => i.id === item.id || (i.start ?? 0) + i.duration <= start || end <= (i.start ?? 0));
  const gone: Array<Op> = alone ? [{ op: "delete_track", track: track.id }] : [];
  const word = dir < 0 ? "forward" : "backward";
  if (next && next.purpose === track.purpose && fitsTrack(item, next) && !next.locked && next.role !== "main" && free(next)) {
    return { ops: [{ op: "move_item", item: item.id, track: next.id, start }, ...gone], summary: `${dir < 0 ? "Bring" : "Send"} ${item.name ?? item.id} ${word}`, track: next.id, start };
  }
  const kind = TRACK_KINDS[item.type];
  const id = uniqueId(project, kind === "text" ? "text-track" : "video-track");
  const used = new Set(project.tracks.map((t) => t.name));
  const base = editorText(track.purpose === "subtitles" ? "Subtitles" : kind === "text" ? "Text" : "Video");
  let name = base;
  for (let n = 2; used.has(name); n++) name = `${base} ${n}`;
  // Past `next` (or at the very top / bottom of the visual tracks when there is none that way).
  const pivot = next ?? track;
  const pivotIndex = project.tracks.findIndex((t) => t.id === pivot.id);
  const index = dir < 0 ? (next ? pivotIndex : 0) : pivotIndex + 1;
  return {
    ops: [{ op: "add_track", track: { id, kind, name, ...(track.purpose ? {purpose:track.purpose} : {}) }, index }, { op: "move_item", item: item.id, track: id, start }, ...gone],
    summary: `${dir < 0 ? "Bring" : "Send"} ${item.name ?? item.id} ${word}`,
    track: id,
    start,
  };
}
