// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { layout } from "@/app/lib/editor/core/timeline/timeline";
import type { Project, Id, Item, Media, Op } from "@/app/lib/editor/core/timeline/types";
import { takenIds, tryOps, uniqueIn } from "@/app/lib/editor/core/timeline/arrange/place";
import { APPEARANCE_KEYS, locate, lockProblem, pieceAppearance, withLinkedAudio } from "@/app/lib/editor/core/timeline/arrange/retime";
import { EditError } from '@/app/lib/editor/core/timeline/apply-ops';
import { fitChangedTransitions } from './transitions';

/** Only picture media can replace a picture; audio replacements must actually carry sound. */
export const canReplaceMedia = (item: Item, media: Media) => item.type === 'clip' ? media.kind !== 'audio'
  : item.type === 'audio' && (media.kind === 'audio' || media.kind === 'video' && media.probe?.hasAudio === true);

/** Fit adjacent transitions to the new source and check the same linked-layer edit used on save. */
export function finishReplacement(project: Project, ops: ReadonlyArray<Op>): { ops: ReadonlyArray<Op> } | { problem: string } {
  try {
    const fitted = fitChangedTransitions(project, ops);
    const tried = tryOps(project, withLinkedAudio(project, fitted));
    return 'problem' in tried ? tried : { ops: fitted };
  } catch (error) {
    const detail = error instanceof EditError ? error.details[0] as { message?: string } | undefined : undefined;
    return { problem: detail?.message ?? (error instanceof Error ? error.message : String(error)) };
  }
}

/** Replace at the current speed; short footage trims the end or loops to fill the same time. */
export function planReplaceMedia(project: Project, itemId: Id, mediaId: Id, short: 'trim' | 'loop' = 'trim') {
  const found = locate(project, itemId), media = project.media.find(value => value.id === mediaId);
  if (!found || !media) return { problem: "That media isn't in the project any more." };
  const { item, track } = found, locked = lockProblem(track, item);
  if (locked) return { problem: locked };
  if (!canReplaceMedia(item, media)) return { problem: 'Choose video or an image for a picture, or media with sound for an audio clip.' };
  const still = media.kind === 'image', speed = still ? 1 : item.speed ?? 1, frames = media.probe?.frames;
  if (!still && (!frames || frames < 1)) return { problem: 'The replacement has no known duration.' };
  const loop = !still && (short === 'loop' ? frames! < item.duration * speed : item.loop === true);
  const duration = still || loop ? item.duration : Math.min(item.duration, Math.floor(frames! / speed));
  if (duration < 1) return { problem: 'The replacement is too short at this playback speed.' };
  const set: Record<string, unknown> = { media: media.id, in: still ? null : 0, duration, hold: null, loop: loop ? true : null };
  if (still) Object.assign(set, { speed: null, keepPitch: null, loop: null });
  if (duration !== item.duration) {
    const appearance = pieceAppearance(item, 0, duration, duration, project.canvas.fps, true, true);
    for (const key of APPEARANCE_KEYS) if (item[key] !== undefined) set[key] = appearance[key] ?? null;
  }
  const finished = finishReplacement(project, [{ op: 'update_item', item: item.id, set }]);
  return 'problem' in finished ? finished : { ...finished, summary: `Replace the media of ${itemId} with ${media.name}` };
}

export interface ReplaceRange {
  /** The clip or audio item a piece of which is replaced. */
  readonly item: Id;
  /** Timeline frames [from, to): the piece, inside the item. */
  readonly from: number;
  readonly to: number;
  /** The media that plays there instead, from frame `in` of it (default 0). */
  readonly media: Id;
  readonly in?: number | undefined;
  /** The new piece's id when it's split off (default: the item's id, numbered); a piece from the item's start keeps the item's id. */
  readonly id?: string | undefined;
}

/**
 * Replace a fragment (issue #118, an overwrite edit): frames [from, to) of a clip or audio item play
 * another media instead (from its frame `in`, at normal speed), and nothing on the timeline moves:
 * the item is split at both ends of the piece (where they're inside it) and the piece gets the new
 * media and in point, keeping the item's look (box, effects, volume, keys). The piece is exactly as
 * long as before, so on Main every later clip, and every other track, stays in sync with the song.
 */
export function planReplaceRange(project: Project, r: ReplaceRange): { readonly ops: ReadonlyArray<Op>; readonly piece: Id; readonly summary: string } | { readonly problem: string } {
  const found = locate(project, r.item);
  if (!found) return { problem: `There's no item "${r.item}".` };
  const { track, item } = found;
  if (item.type !== "clip" && item.type !== "audio") return { problem: "Only a clip's or an audio item's piece can be replaced." };
  const locked = lockProblem(track, item);
  if (locked) return { problem: locked };
  const media = project.media.find((m) => m.id === r.media);
  if (!media) return { problem: `There's no media "${r.media}".` };
  if (!canReplaceMedia(item, media)) return { problem: 'Choose video or an image for a picture, or media with sound for an audio clip.' };
  if (!Number.isInteger(r.from) || !Number.isInteger(r.to)) return { problem: "from and to are whole timeline frames." };
  const start = layout(project).starts.get(item.id) ?? 0;
  const end = start + item.duration;
  if (r.from < start || r.to > end || r.to <= r.from) return { problem: `The piece must be inside the item: frames ${start} to ${end}.` };
  const from = r.in ?? 0;
  if (!Number.isInteger(from) || from < 0) return { problem: "in is a whole frame of the media, from 0." };
  const length = r.to - r.from;
  const frames = media.probe?.frames;
  if (media.kind !== 'image' && (!frames || frames < 1)) return { problem: 'The replacement has no known duration.' };
  if (media.kind !== "image" && frames && from + length > frames) return { problem: `${media.name} is ${frames} frames long: from frame ${from} it can't fill ${length} frames. Start it at frame ${Math.max(0, frames - length)} or earlier.` };
  const taken = takenIds(project);
  if (r.id !== undefined && taken.has(r.id)) return { problem: `The id ${r.id} is taken.` };
  const ops: Array<Op> = [];
  let piece = item.id;
  if (r.from > start) {
    piece = r.id ?? uniqueIn(item.id, taken);
    ops.push({ op: "split_item", item: item.id, at: r.from - start, newId: piece });
  }
  if (r.to < end) ops.push({ op: "split_item", item: piece, at: length, newId: uniqueIn(item.id, taken) });
  ops.push({ op: "update_item", item: piece, set: { media: media.id, in: media.kind === "image" ? null : from, speed: null, keepPitch: null, loop: null, hold: null, name: null } });
  const finished = finishReplacement(project, ops);
  if ('problem' in finished) return finished;
  return { ...finished, piece, summary: `Replaced ${(length / project.canvas.fps).toFixed(1)} s with ${media.name}` };
}
