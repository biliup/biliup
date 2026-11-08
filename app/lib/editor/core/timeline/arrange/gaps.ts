// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Gaps on the main track (D224): main items keep their order, and an item's `gap` is the empty
// frames before it. Planners that must leave the other items where they are (a delete, a drag)
// work out the gaps that put each item back at its frame; the engine's ops stay plain (a bare
// delete_item still closes up, which is what ripple delete and cuts by text want).
import { gapOf, mainStarts } from "@/app/lib/editor/core/timeline/rules";
import type { Id, Item, Op, Track } from "@/app/lib/editor/core/timeline/types";

/**
 * update_item ops giving the main items in `order` the gaps that start each at `desired(item)`, or
 * right after the item before it when that's later (a later item is pushed, never overlapped).
 * An item whose gap is already right gets no op, except ids in `cleared` (their gap is gone after
 * an earlier op in the same edit, e.g. move_item): those get one whenever it isn't 0. Also returns
 * where each lands.
 */
export function gapOpsFor(order: ReadonlyArray<Item>, desired: (item: Item, index: number) => number, cleared: ReadonlySet<Id> = new Set()): { ops: Array<Op>; starts: Map<Id, number> } {
  const ops: Array<Op> = [];
  const starts = new Map<Id, number>();
  let end = 0;
  order.forEach((item, index) => {
    const start = Math.max(end, Math.round(desired(item, index)));
    const gap = start - end;
    const had = cleared.has(item.id) ? 0 : gapOf(item);
    if (gap !== had) ops.push({ op: "update_item", item: item.id, set: { gap: gap > 0 ? gap : null } });
    starts.set(item.id, start);
    end = start + item.duration;
  });
  return { ops, starts };
}

/** Each main item's start frame, by id. */
export function mainStartsById(track: Pick<Track, "items">): Map<Id, number> {
  const s = mainStarts(track);
  return new Map(track.items.map((item, i) => [item.id, s[i]!]));
}

/**
 * Delete main items and leave their space (D224): the items after them stay where they are.
 * The update_item ops to send after the delete_item ops.
 */
export function keepMainTimes(track: Pick<Track, "items">, gone: ReadonlySet<Id>): Array<Op> {
  const before = mainStartsById(track);
  return gapOpsFor(track.items.filter((i) => !gone.has(i.id)), (item) => before.get(item.id)!).ops;
}

/**
 * Close up after deleting main items (ripple delete, a cut by text): the deleted items' own frames
 * go and the rest moves left, but the empty space that was before a deleted item stays (it goes to
 * the next item that stays). The update_item ops to send after the delete_item ops.
 */
export function rippleMainGaps(track: Pick<Track, "items">, gone: ReadonlySet<Id>): Array<Op> {
  const ops: Array<Op> = [];
  let carried = 0;
  for (const item of track.items) {
    if (gone.has(item.id)) {
      carried += gapOf(item);
      continue;
    }
    if (carried > 0) ops.push({ op: "update_item", item: item.id, set: { gap: gapOf(item) + carried } });
    carried = 0;
  }
  return ops;
}

/**
 * Move some main items together by `shift` frames (issue #58, D229): a selection or a group that
 * holds main clips. The moved items keep their order and the spaces between them; the others stay
 * at their frames. The new order is by middles (moved items next to each other count as one block, so
 * nothing lands inside it; a block passes an item once its middle does, as one dragged clip does,
 * D224); an item that would land on the one before it is
 * pushed later (gapOpsFor). Returns the ops (transitions dropped at every cut whose clip before it
 * changed or that now follows a gap, the move_items that reorder, then the gaps) and where each
 * main item lands. No ops when nothing moves.
 */
export function shiftMainItems(track: Pick<Track, "items">, ids: ReadonlySet<Id>, shift: number): { ops: Array<Op>; starts: Map<Id, number> } {
  const before = mainStartsById(track);
  const desired = (item: Item) => before.get(item.id)! + (ids.has(item.id) ? shift : 0);
  const rank = new Map(track.items.map((item, i) => [item.id, i]));
  // Moved items next to each other move as one block: the others go before or after a block by
  // its middle, so they never land inside it.
  const middle = new Map<Id, number>();
  for (let i = 0; i < track.items.length; ) {
    const item = track.items[i]!;
    if (!ids.has(item.id)) {
      middle.set(item.id, desired(item) + item.duration / 2);
      i++;
      continue;
    }
    let j = i;
    while (j + 1 < track.items.length && ids.has(track.items[j + 1]!.id)) j++;
    const last = track.items[j]!;
    const mid = (desired(item) + desired(last) + last.duration) / 2;
    for (let k = i; k <= j; k++) middle.set(track.items[k]!.id, mid);
    i = j + 1;
  }
  const order = [...track.items].sort((a, b) => middle.get(a.id)! - middle.get(b.id)! || rank.get(a.id)! - rank.get(b.id)!);
  // Reorder: each moved item, in its new order, goes right after the item it now follows.
  const now = track.items.map((i) => i.id);
  const moves: Array<Op> = [];
  order.forEach((item, k) => {
    if (!ids.has(item.id)) return;
    const from = now.indexOf(item.id);
    now.splice(from, 1);
    const index = k === 0 ? 0 : now.indexOf(order[k - 1]!.id) + 1;
    now.splice(index, 0, item.id);
    if (index !== from) moves.push({ op: "move_item", item: item.id, index });
  });
  const cleared = new Set(moves.map((m) => (m as { item: Id }).item));
  const gaps = gapOpsFor(order, desired, cleared);
  // The cuts that change lose their transitions (D52), and so does any clip that now follows a gap.
  const prevBefore = new Map(track.items.map((item, i) => [item.id, track.items[i - 1]?.id]));
  let end = 0;
  const drops: Array<Op> = [];
  order.forEach((item, k) => {
    const start = gaps.starts.get(item.id)!;
    if (item.transitionIn && (start > end || prevBefore.get(item.id) !== order[k - 1]?.id)) drops.push({ op: "set_transition", item: item.id, transition: null });
    end = start + item.duration;
  });
  return { ops: [...drops, ...moves, ...gaps.ops], starts: gaps.starts };
}
