// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { applyEdit, createSession, EditError } from "@/app/lib/editor/core/timeline/apply-ops";
import { gapOf, mainStarts, timelineProblems } from "@/app/lib/editor/core/timeline/rules";
import type { Project, Track, Item, Media, Id, Frame, Op } from "@/app/lib/editor/core/timeline/types";

/** A still image placed on the timeline lasts this long. */
export const IMAGE_SECONDS = 5;

export const uniqueIn = (base: string, taken: Set<string>) => {
  for (let n = 1; ; n++) {
    const id = `${base}-${n}`;
    if (!taken.has(id)) {
      taken.add(id);
      return id;
    }
  }
};

/** Every id in use (media, tracks, items, scenes). */
export const takenIds = (project: Pick<Project, "media" | "tracks"> & Partial<Pick<Project, "scenes">>) =>
  new Set([...project.media.map((m) => m.id), ...project.tracks.flatMap((t) => [t.id, ...t.items.map((i) => i.id)]), ...(project.scenes ?? []).map((s) => s.id)]);

/** Free-track items other than `exclude`, as [start, end) in time order. */
export function busySpans(track: Track, exclude?: string): Array<{ start: number; end: number }> {
  return track.items
    .filter((i) => i.id !== exclude)
    .map((i) => ({ start: i.start ?? 0, end: (i.start ?? 0) + i.duration }))
    .sort((a, b) => a.start - b.start);
}

/** Whether [start, start + duration) is free on a free track (ignoring `exclude`). */
export const isFree = (track: Track, start: number, duration: number, exclude?: string) => busySpans(track, exclude).every((b) => start + duration <= b.start || start >= b.end);

/**
 * Where an item `duration` long can go on a free track nearest to `start` without overlapping
 * anything (D52). There is always room after the last item.
 */
export function nearestFreeSpot(track: Track, exclude: string | undefined, start: number, duration: number): number {
  const busy = busySpans(track, exclude);
  const fits = (s: number) => s >= 0 && busy.every((b) => s + duration <= b.start || s >= b.end);
  if (fits(start)) return start;
  const candidates = [0, ...busy.flatMap((b) => [b.start - duration, b.end])].filter(fits);
  return candidates.reduce((best, c) => (Math.abs(c - start) < Math.abs(best - start) ? c : best), Math.max(0, ...busy.map((b) => b.end)));
}

/** The main-track slot (index) a clip whose middle is at `middle` lands in (D52's reorder rule), among `items`. */
export function mainSlot(items: ReadonlyArray<Pick<Item, "duration" | "gap">>, middle: number): number {
  let t = 0;
  let index = 0;
  for (const other of items) {
    t += gapOf(other);
    if (t + other.duration / 2 < middle) index++;
    t += other.duration;
  }
  return index;
}

/** Which item type media of this kind becomes on a track of this kind, if any (video on an audio track: its sound). */
export function itemTypeFor(media: Pick<Media, "kind" | "probe">, track: Pick<Track, "kind">): "clip" | "audio" | undefined {
  if (track.kind === "video") return media.kind === "audio" ? undefined : "clip";
  if (track.kind === "audio") return media.kind === "audio" || (media.kind === "video" && media.probe?.hasAudio) ? "audio" : undefined;
  return undefined;
}

/** A media file to place: an entry of the project, or one being added in the same edit. */
export type PlaceMedia = Pick<Media, "id" | "kind" | "name" | "probe">;

export interface PlacedItem {
  readonly itemId: Id;
  readonly trackId: Id;
  readonly start: Frame;
}

export interface PlaceMediaPlan {
  readonly ops: ReadonlyArray<Op>;
  readonly placed: ReadonlyArray<PlacedItem>;
}

/** The length a media file gets on the timeline, in project frames (undefined: unknown, can't be placed). */
export const placedLength = (media: PlaceMedia, fps: number, imageSeconds = IMAGE_SECONDS) => (media.kind === "image" ? Math.max(1, Math.round(imageSeconds * fps)) : media.probe?.frames && media.probe.frames > 0 ? Math.floor(media.probe.frames) : undefined);

/**
 * The first unlocked, heard audio track (not main) free over [start, start + duration), or undefined
 * (D124: several sounds at once each get their own track; music, sfx and voice overlap freely).
 */
export function freeAudioTrack(project: Pick<Project, "tracks">, start: number, duration: number): Track | undefined {
  // Not a muted track, nor one a solo elsewhere silences: the new sound would be unheard.
  const solo = project.tracks.some((t) => t.mixer?.solo === true);
  return project.tracks.find((t) => t.kind === "audio" && t.role !== "main" && !t.locked && !t.mixer?.mute && (!solo || t.mixer?.solo === true) && isFree(t, start, duration));
}

/** The next "Audio" track name: Audio, Audio 2, … */
export const nextName = (project: Pick<Project, "tracks">, base: string) => {
  const used = new Set(project.tracks.map((t) => t.name));
  if (!used.has(base)) return base;
  for (let n = 2; ; n++) if (!used.has(`${base} ${n}`)) return `${base} ${n}`;
};

/**
 * The ops that put `media` on the timeline at `frame` (D122, D124), one file after another:
 * - on `track` when given and it takes the media (main: in the slot nearest the frame, the next
 *   files after it; a free track: at the frame, each on the nearest free spot after the previous);
 *   a video file on an audio track places its sound;
 * - otherwise sound goes on the first audio track free for its whole length at the frame (a new
 *   "Audio N" track at the bottom when none is), and pictures go on the main track when it's empty,
 *   else at the frame on the nearest video track above main that is free for the whole length (a new
 *   "Video N" track just above main when none is).
 * Nothing that is already there moves (except main's ripple when inserting into it). `taken` holds
 * every id in use and gets the new ones. Media of unknown length are refused.
 */
export function planPlaceMedia(project: Pick<Project, "tracks" | "canvas">, media: ReadonlyArray<PlaceMedia>, at: { readonly frame: Frame; readonly track?: Id | undefined; /** A photo's length (F30: the app's default; IMAGE_SECONDS when absent). */ readonly imageSeconds?: number | undefined }, taken: Set<string>): PlaceMediaPlan | { readonly problem: string } {
  const fps = project.canvas.fps;
  // The project as the ops so far leave it (tracks only, starts resolved for free tracks).
  let tracks: Array<Track> = project.tracks.map((t) => ({ ...t, items: [...t.items] }));
  const ops: Array<Op> = [];
  const placed: Array<PlacedItem> = [];
  let cursor = Math.max(0, Math.floor(at.frame));
  let mainIndex: number | undefined;
  const target = at.track !== undefined ? tracks.find((t) => t.id === at.track) : undefined;
  if (target?.locked) return { problem: `The ${target.name} track is locked. Unlock it to add to it.` };
  for (const m of media) {
    const length = placedLength(m, fps, at.imageSeconds);
    if (!length) return { problem: `${m.name} has no known length, so it can't go on the timeline.` };
    let track = target && itemTypeFor(m, target) ? target : undefined;
    const type = track ? itemTypeFor(m, track)! : m.kind === "audio" ? "audio" : "clip";
    if (!track) {
      const main = tracks.find((t) => t.role === "main");
      if (type === "audio") track = freeAudioTrack({ tracks }, cursor, length);
      else if (main && !main.locked && main.items.length === 0) track = main;
      else {
        const mi = tracks.findIndex((t) => t.role === "main");
        track = tracks.slice(0, Math.max(0, mi)).reverse().find((t) => t.kind === "video" && !t.locked && isFree(t, cursor, length));
      }
      if (!track) {
        const audio = type === "audio";
        const id = uniqueIn(audio ? "audio-track" : "video-track", taken);
        const name = nextName({ tracks }, audio ? "Audio" : "Video");
        const mi = tracks.findIndex((t) => t.role === "main");
        const index = audio ? tracks.length : Math.max(0, mi);
        ops.push({ op: "add_track", track: { id, kind: audio ? "audio" : "video", name }, index });
        track = { id, kind: audio ? "audio" : "video", name, items: [] };
        tracks.splice(index, 0, track);
      }
    }
    const itemId = uniqueIn(type === "audio" ? "audio" : "clip", taken);
    const body = { id: itemId, type, media: m.id, ...(m.kind === "image" ? {} : { in: 0 }), duration: length } as Item;
    let start: number;
    if (track.role === "main") {
      // The cut nearest the drop: after every clip whose middle is at or before it.
      if (mainIndex === undefined) mainIndex = mainSlot(track.items, cursor + 0.5);
      const index = mainIndex++;
      // Right after the item before it (D224: its gap included).
      start = index > 0 ? mainStarts(track)[index - 1]! + track.items[index - 1]!.duration : 0;
      // The clip it lands before now follows a new clip: the transition into it goes.
      const before = track.items[index];
      if (before?.transitionIn) ops.push({ op: "set_transition", item: before.id, transition: null });
      ops.push({ op: "insert_item", track: track.id, index, item: body });
      const updated: Track = { ...track, items: [...track.items.slice(0, index), body, ...track.items.slice(index)] };
      tracks = tracks.map((t) => (t.id === track!.id ? updated : t));
      cursor = start + length;
    } else {
      start = nearestFreeSpot(track, undefined, cursor, length);
      const item = { ...body, start } as Item;
      ops.push({ op: "insert_item", track: track.id, item });
      const updated: Track = { ...track, items: [...track.items, item] };
      tracks = tracks.map((t) => (t.id === track!.id ? updated : t));
      cursor = start + length;
    }
    placed.push({ itemId, trackId: track.id, start });
  }
  return { ops, placed };
}

/** The project after `ops`, checked with the timeline rules, or the first problem as a sentence. */
export function tryOps(project: Project, ops: ReadonlyArray<Op>): { readonly project: Project } | { readonly problem: string } {
  try {
    return { project: applyEdit(createSession(project), { actor: "user", ops }, timelineProblems).project };
  } catch (error) {
    const first = error instanceof EditError ? (error.details[0] as { message?: string } | undefined) : undefined;
    return { problem: first?.message ?? (error instanceof Error ? error.message : String(error)) };
  }
}
