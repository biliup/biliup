// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Media-level edits (F8, F27; D167, D169): taking a media out of the project with everything on the
// timeline that uses it, and fitting a sound (music) to the video's length. Pure planners: the UI
// and the agent send their ops as one edit (one revision, one ⌘Z).
import { gapOf } from "@/app/lib/editor/core/timeline/rules";
import { rippleMainGaps } from "@/app/lib/editor/core/timeline/arrange/gaps";
import { sliceGainKeys } from '@/app/lib/editor/core/audio/audio';
import type { Id, Item, Op, Project, Track } from "@/app/lib/editor/core/timeline/types";

type MessageValues = Readonly<Record<string, string | number>>;
type Plan = { readonly ops: ReadonlyArray<Op>; readonly summary: string; readonly values?: MessageValues };
type Problem = { readonly problem: string; readonly values?: MessageValues };

/** Media ids an element's props refer to (`{ media: id }` anywhere inside, as validate's checkMediaRefs reads them). */
export function propMediaIds(value: unknown, into: Set<Id> = new Set()): Set<Id> {
  if (value && typeof value === "object") {
    const ref = value as { media?: unknown };
    if (typeof ref.media === "string" && Object.keys(value).length === 1) into.add(ref.media);
    for (const v of Object.values(value)) propMediaIds(v, into);
  }
  return into;
}

/** Whether an item uses a media: as its `media`, a captions' `source`, or in its props (an image frame's image). */
export const itemNeedsMedia = (item: Item, mediaId: Id) => item.media === mediaId || (item.type === "captions" && item.source === mediaId) || (item.props !== undefined && propMediaIds(item.props).has(mediaId));
export const itemUsesMedia = (item: Item, mediaId: Id) => itemNeedsMedia(item, mediaId) || propMediaIds(item.effects).has(mediaId);

/** The items that use a media: clips and sounds (`media`), captions made from it (`source`), elements showing it (props). */
export function mediaUses(project: Pick<Project, "tracks">, mediaId: Id): Array<{ readonly item: Item; readonly track: Track; readonly index: number }> {
  const out: Array<{ item: Item; track: Track; index: number }> = [];
  for (const track of project.tracks) {
    track.items.forEach((item, index) => {
      if (itemUsesMedia(item, mediaId)) out.push({ item, track, index });
    });
  }
  return out;
}

/** Every media id the timeline uses (the Media panel's "used" badge). */
export function usedMediaIds(project: Pick<Project, "tracks">): Set<Id> {
  const used = new Set<Id>();
  for (const track of project.tracks)
    for (const item of track.items) {
      if (item.media) used.add(item.media);
      if (item.type === "captions" && item.source) used.add(item.source);
      if (item.props) propMediaIds(item.props, used);
      if (item.effects) propMediaIds(item.effects, used);
    }
  return used;
}

/**
 * Remove a media from the project (F8): every item that uses it is deleted (on main the clips after
 * close up, and a clip that now follows another loses its transition in), then the media entry
 * goes. The file is never touched. A locked item or track that uses it refuses the whole thing.
 */
export function planRemoveMedia(project: Pick<Project, "tracks" | "media">, mediaId: Id): (Plan & { readonly items: number }) | Problem {
  const media = project.media.find((m) => m.id === mediaId);
  if (!media) return { problem: "That media isn't in the project any more." };
  const uses = mediaUses(project, mediaId);
  const locked = uses.find((u) => u.item.locked || u.track.locked);
  if (locked) return { problem: `${media.name} is used by a locked ${locked.track.locked ? "track" : "item"}. Unlock it to remove the media.` };
  const gone = new Set(uses.filter(u=>itemNeedsMedia(u.item,mediaId)).map(u=>u.item.id));
  const ops: Array<Op> = [];
  // An image used only by an effect does not own the video it covers.
  for(const {item} of uses){
    if(gone.has(item.id))continue;
    const effects=(item.effects??[]).flatMap(effect=>{
      if(!propMediaIds(effect.params).has(mediaId))return [effect];
      if(effect.module!=='effect.biliup-mask')return [];
      const regions=Array.isArray(effect.params?.regions)?effect.params.regions.filter(region=>!propMediaIds(region).has(mediaId)):[];
      return regions.length?[{...effect,params:{...effect.params,regions}}]:[];
    });
    ops.push({op:'update_item',item:item.id,set:{effects}});
  }
  // Main: the first clip that stays after a removed run now follows another clip (or starts the
  // track), so its transition in goes too.
  for (const track of project.tracks) {
    if (track.role !== "main") continue;
    track.items.forEach((item, index) => {
      if (!gone.has(item.id) || gone.has(track.items[index + 1]?.id ?? "")) return;
      const next = track.items[index + 1];
      if (next?.transitionIn) ops.push({ op: "set_transition", item: next.id, transition: null });
    });
  }
  for (const id of gone) ops.push({ op: "delete_item", item: id });
  // D224: closes up like ripple delete, but the space before a removed main clip stays.
  for (const track of project.tracks) if (track.role === "main") ops.push(...rippleMainGaps(track, gone));
  ops.push({ op: "delete_media", media: mediaId });
  const n = uses.length;
  return { ops, summary: `Remove ${media.name} from the project`, items: n };
}

/** Where the picture ends: the last frame of any video or text track (sound and captions don't count). */
export function videoEnd(project: Pick<Project, "tracks">): number {
  let end = 0;
  for (const track of project.tracks) {
    if (track.kind === "audio") continue;
    let t = 0;
    for (const item of track.items) {
      if (track.role === "main") t += gapOf(item);
      const start = track.role === "main" ? t : (item.start ?? 0);
      if (track.role === "main") t += item.duration;
      if (item.type !== "captions") end = Math.max(end, start + item.duration);
    }
  }
  return end;
}

/** The fade out Fit to video gives (as music added from the library gets): 2 s, at most a quarter of the item. */
export const fitFadeOut = (duration: number, fps: number) => Math.min(Math.round(2 * fps), Math.floor(duration / 4));

/**
 * Fit to video (F27): a sound runs from where it starts to the end of the picture, trimmed when the
 * media is longer, looped (the media starts over) when it is shorter, and fades out over the last
 * 2 s (a longer fade out stays within the new duration). One update_item. Refused when the sound starts at or past
 * the end of the picture, when another item on its track is in the way, or when it is locked.
 */
export function planFitToVideo(project: Pick<Project, "tracks" | "media" | "canvas">, itemId: Id): (Plan & { readonly mode: "trim" | "extend" | "loop" | "same" }) | Problem {
  const fps = project.canvas.fps;
  let found: { item: Item; track: Track } | undefined;
  for (const track of project.tracks) {
    const item = track.items.find((i) => i.id === itemId);
    if (item) found = { item, track };
  }
  if (!found) return { problem: "That item isn't in the project any more." };
  const { item, track } = found;
  if (item.type !== "audio" || track.kind !== "audio" || track.role === "main") return { problem: "Only a sound on an audio track can be fitted to the video." };
  if (item.locked || track.locked) return { problem: "The sound is locked. Unlock it to fit it to the video." };
  const media = project.media.find((m) => m.id === item.media);
  const length = media?.probe?.frames;
  if (!media || !length) return { problem: "The sound has no known length." };
  const start = item.start ?? 0;
  const end = videoEnd(project);
  if (end <= 0) return { problem: "There is no video to fit the sound to." };
  if (start >= end) return { problem: "The sound starts at or after the video ends." };
  const duration = end - start;
  const blocker = track.items.find((i) => i.id !== item.id && (i.start ?? 0) < end && (i.start ?? 0) + i.duration > start);
  if (blocker) return { problem: "Another sound on this track is in the way at {0} s.", values: { 0: ((blocker.start ?? 0) / fps).toFixed(1) } };
  const speed = item.speed ?? 1;
  const fits = (item.in ?? 0) + duration * speed <= length;
  const fadeOut = Math.min(duration, Math.max(item.fadeOut ?? 0, fitFadeOut(duration, fps)));
  const fadeIn = Math.min(item.fadeIn ?? 0, Math.max(0, duration - fadeOut));
  const set: Record<string, unknown> = { duration, fadeOut: fadeOut > 0 ? fadeOut : null, loop: fits ? null : true };
  if ((item.fadeIn ?? 0) !== fadeIn) set.fadeIn = fadeIn > 0 ? fadeIn : null;
  const keys = sliceGainKeys(item.gainKeys, 0, duration);
  if (item.gainKeys !== keys) set.gainKeys = keys ?? null;
  // A detached sound (D123) that is fitted on its own stops following its clip.
  if (item.linkedTo) set.linkedTo = null;
  const mode = fits ? (duration === item.duration ? "same" : duration < item.duration ? "trim" : "extend") : "loop";
  const changed = Object.entries(set).some(([k, v]) => (v === null ? (item as unknown as Record<string, unknown>)[k] !== undefined : (item as unknown as Record<string, unknown>)[k] !== v && JSON.stringify((item as unknown as Record<string, unknown>)[k]) !== JSON.stringify(v)));
  if (!changed) return { ops: [], summary: "Already fits the video", mode: "same" };
  const name = item.name ?? media.name;
  const summary = mode === "loop" ? "Loop {0} to video length ({1} s)"
    : mode === "trim" ? "Trim {0} to video length ({1} s)"
    : mode === "extend" ? "Extend {0} to video length ({1} s)"
    : "Fit {0} to video length ({1} s)";
  return { ops: [{ op: "update_item", item: item.id, set }], summary, values: { 0: name, 1: (duration / fps).toFixed(1) }, mode };
}
