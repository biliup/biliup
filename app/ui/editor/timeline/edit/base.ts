// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Shared bits of the timeline planners (edit.ts): finding an item, track fit and locks, the plan shape, drafts.
import { applyEdit, createSession, EditError } from "@/app/lib/editor/core/timeline/apply-ops";
import { fitsTrackKind, timelineProblems } from "@/app/lib/editor/core/timeline/rules";
import { withLinkedAudio } from "@/app/lib/editor/core/timeline/arrange";
import { editorText } from "@/app/lib/editor/i18n";
import type { Op } from "@/app/lib/editor/core/timeline/types";
import { type Item, type Track, type ProjectModel, layoutOf } from "@/app/lib/editor/state/project";

/** How close (in pixels) an edge must come to a snap target to stick to it. */
export const SNAP_PX = 6;

export interface Placed {
  readonly item: Item;
  readonly track: Track;
  /** Index in the track's items (array order; only main's order means time). */
  readonly index: number;
  readonly start: number;
}

export function findItem(project: ProjectModel, id: string | undefined): Placed | undefined {
  if (id === undefined) return undefined;
  const { starts } = layoutOf(project);
  for (const track of project.tracks) {
    const index = track.items.findIndex((i) => i.id === id);
    if (index !== -1) return { item: track.items[index]!, track, index, start: starts.get(id) ?? 0 };
  }
  return undefined;
}

// The kind of track each item type lives on off the main track (a new track for it gets this kind).
export const TRACK_KINDS: Readonly<Record<Item["type"], Track["kind"]>> = { clip: "video", component: "video", graphic: "video", text: "text", captions: "text", audio: "audio", danmaku:"text" };
export const homeKind = (item: Pick<Item, "type">): Track["kind"] => TRACK_KINDS[item.type];
/** Whether an item may go on a track (the validator's rule, core rules.ts): D215, the main track takes any picture, text too. */
export const fitsTrack = (item: Item, track: Track) => fitsTrackKind(item.type, track);

export const lockedProblem = (track: Track) => `The ${track.name} track is locked. Unlock it to edit it.`;

/** Why an item can't be edited by hand, if it can't. */
export function lockOf(placed: Pick<Placed, 'track' | 'item'>): string | undefined {
  if (placed.track.locked) return lockedProblem(placed.track);
  if (placed.item.locked) return "That item is locked.";
  return undefined;
}

/** set_transition null for each of these main items that has a transition in. */
export const dropTransitions = (items: ReadonlyArray<Item | undefined>): Array<Op> =>
  [...new Set(items)].filter((i): i is Item => !!i?.transitionIn).map((i) => ({ op: "set_transition", item: i.id, transition: null }));

export interface Plan {
  readonly ops: ReadonlyArray<Op>;
  readonly summary: string;
}

/** "0.5 s", "1.25 s": a transition's length for labels. */
export const secondsLabel = (frames: number, fps: number) => `${Math.round((frames / fps) * 100) / 100} s`;

/**
 * The project after `ops`, checked with the same timeline rules as the server (core rules.ts), for
 * previewing a drag and checking an action before it is sent: a plan the server would refuse is
 * never shown as droppable. The problem is a sentence for the user.
 */
export function draftOf(project: ProjectModel, ops: ReadonlyArray<Op>): { readonly project: ProjectModel } | { readonly problem: string } {
  if (!ops.length) return { project };
  try {
    // D123: linked sounds follow their clips, as the server applies it.
    return { project: applyEdit(createSession(project), { actor: "user", ops: withLinkedAudio(project, ops) }, timelineProblems).project };
  } catch (error) {
    const first = error instanceof EditError ? (error.details[0] as { message?: string } | undefined) : undefined;
    return { problem: editorText(first?.message ?? (error instanceof Error ? error.message : String(error))) };
  }
}

// ---- groups (F19, D157): a kicker and its title move, trim and delete together ----

/** `ids` with every item that shares a group with one of them (the given ones first, in order). */
export function withGroups(project: ProjectModel, ids: ReadonlyArray<string>): Array<string> {
  const groups = new Set(ids.map((id) => findItem(project, id)?.item.group).filter((g): g is string => !!g));
  const out = [...ids];
  if (!groups.size) return out;
  for (const t of project.tracks) for (const i of t.items) if (i.group && groups.has(i.group) && !out.includes(i.id)) out.push(i.id);
  return out;
}
