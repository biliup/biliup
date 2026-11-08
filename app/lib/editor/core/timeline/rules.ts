// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The timeline rules a schema can't express and that need no installed modules: item kinds per
// track, starts, media ranges, keys, fades, animations, transitions between main items, overlaps.
// Browser-safe (no node:fs, no ajv), so the renderer checks a drag's planned result with the same
// rules the server's validator uses (D52) before showing it as droppable.
import { handleAfter, handleBefore, overlaps } from "@/app/lib/editor/core/visual/transitions";
import type { Item, ItemType, Media, Problem, Project, Track } from "@/app/lib/editor/core/timeline/types";

const ITEM_TRACK_KINDS: Readonly<Record<ItemType, ReadonlyArray<Track["kind"]>>> = {
  clip: ["video"],
  component: ["video"],
  graphic: ["video"],
  text: ["text"],
  captions: ["text"],
  danmaku: ["text"],
  audio: ["audio"],
};

/**
 * D215: the main track (the spine) takes any picture: clips (video, stills, recordings), elements,
 * graphics and text. A text card there is a title clip: in order, ripple, transitions (it holds
 * its first and last look through a transition's overlap and needs no footage). Captions and sound
 * stay off it.
 */
export const MAIN_ITEM_TYPES: ReadonlySet<ItemType> = new Set<ItemType>(["clip", "component", "graphic", "text"]);

/** Whether an item of this type may go on this track (the rule item-track-kind checks). */
export const fitsTrackKind = (type: ItemType, track: Pick<Track, "kind" | "role">) => (track.role === "main" ? MAIN_ITEM_TYPES.has(type) : !!ITEM_TRACK_KINDS[type]?.includes(track.kind));

const err = (code: string, path: string, message: string): Problem => ({ code, path, message });

/**
 * D224: where a main-track item starts, from the one before it: its `gap` (empty frames, absent = 0)
 * after the previous item's end (frame 0 for the first). Every place that walks the main track adds
 * the gap this way.
 */
export const gapOf = (item: Pick<Item, "gap">): number => (typeof item.gap === "number" && item.gap > 0 ? item.gap : 0);

/** Start frame of each main-track item, in order (gaps included). */
export function mainStarts(track: Pick<Track, "items">): Array<number> {
  const out: Array<number> = [];
  let t = 0;
  for (const item of track.items) {
    t += gapOf(item);
    out.push(t);
    t += item.duration;
  }
  return out;
}

/** Where the main track ends (its last item's end, gaps included); 0 without a main track or items. */
export function mainEnd(project: Pick<Project, "tracks">): number {
  const main = project.tracks.find((t) => t.role === "main");
  if (!main?.items.length) return 0;
  const last = main.items.at(-1)!;
  return mainStarts(main).at(-1)! + last.duration;
}

/** Start frame of every item: main-track items play in order (D224: after their `gap`), free items carry `start`. */
export function resolveStarts(project: Pick<Project, "tracks">): Map<string, number> {
  const starts = new Map<string, number>();
  for (const track of project.tracks) {
    let t = 0;
    for (const item of track.items) {
      if (track.role === "main") {
        t += gapOf(item);
        starts.set(item.id, t);
        t += item.duration;
      } else {
        starts.set(item.id, item.start ?? 0);
      }
    }
  }
  return starts;
}

export function projectDuration(project: Pick<Project, "tracks">): number {
  const starts = resolveStarts(project);
  let end = 0;
  // Captions (D82) draw only where their speech plays: their window doesn't make the video longer.
  for (const track of project.tracks) for (const item of track.items) if (item.type !== "captions") end = Math.max(end, starts.get(item.id)! + item.duration);
  return end;
}

/** Problems with the timeline itself (see the header). Expects a project of the right shape. */
export function timelineProblems(project: Pick<Project, "tracks" | "media">): Array<Problem> {
  const errors: Array<Problem> = [];
  const media = new Map<string, Media>(project.media.map((m) => [m.id, m]));
  const starts = resolveStarts(project);
  const startOf = (item: Item) => starts.get(item.id)!;
  for (const track of project.tracks) {
    const tpath = `/tracks/${track.id}`;
    const main = track.role === "main";
    track.items.forEach((item, index) => {
      const ipath = `${tpath}/items/${item.id}`;
      if (!fitsTrackKind(item.type, track)) errors.push(err("item-track-kind", ipath, main ? `a ${item.type} item can't go on the main track` : `a ${item.type} item can't go on a ${track.kind} track`));
      if (main && item.start !== undefined) errors.push(err("main-item-start", ipath, "main-track items are placed by their order and gap, and must not have a start"));
      if (!main && item.gap !== undefined) errors.push(err("gap-track", ipath, "only main-track items have a gap (free items carry a start)"));
      if (!main && item.start === undefined) errors.push(err("missing-start", ipath, "items on free tracks need a start frame"));

      // Media references and ranges.
      const mediaId = item.type === "captions" ? item.source : item.media;
      if (mediaId !== undefined) {
        const m = media.get(mediaId);
        if (!m) errors.push(err("unknown-media", ipath, `media "${mediaId}" does not exist`));
        else {
          const needsAudio = item.type === "audio" || item.type === "captions";
          if (needsAudio && !(m.kind === "audio" || m.probe?.hasAudio)) errors.push(err("media-kind", ipath, `"${mediaId}" has no audio`));
          if (!needsAudio && m.kind === "audio") errors.push(err("media-kind", ipath, `"${mediaId}" is audio-only`));
          const used = (item.in ?? 0) + item.duration * (item.speed ?? 1);
          if (m.kind !== "image" && m.probe?.frames && !item.loop && !((item.type === "clip" || item.type === "danmaku") && item.hold) && item.type !== "captions" && used > m.probe.frames) {
            errors.push(err("media-range", ipath, `uses frames up to ${used} but "${mediaId}" has ${m.probe.frames}`));
          }
        }
      }

      for (const [k, a] of Object.entries(item.animation ?? {})) {
        if (a && a.duration > item.duration) errors.push(err("animation-too-long", `${ipath}/animation/${k}`, `${k} animation is longer than the item`));
      }
      for (const k of item.gainKeys ?? []) {
        if (k.frame > item.duration) errors.push(err("key-range", `${ipath}/gainKeys`, `key at ${k.frame} is past the item's end (${item.duration})`));
      }
      for (const k of item.keys ?? []) {
        if (k.frame > item.duration) errors.push(err("key-range", `${ipath}/keys`, `key at ${k.frame} is past the item's end (${item.duration})`));
      }
      if ((item.fadeIn ?? 0) + (item.fadeOut ?? 0) > item.duration) errors.push(err("fade-too-long", ipath, "fades are longer than the item"));

      // Transitions only between two main-track items, and no longer than either.
      if (item.transitionIn) {
        const tr = item.transitionIn;
        if (!main) errors.push(err("transition-track", ipath, "transitions only go between items on the main track"));
        else if (index === 0) errors.push(err("transition-first", ipath, "the first main-track item has nothing to transition from"));
        else if (gapOf(item) > 0) errors.push(err("transition-gap", ipath, "a transition can't cross a gap: the item must follow the one before it"));
        else {
          const prev = track.items[index - 1]!;
          if (tr.duration > Math.min(prev.duration, item.duration) / 2) {
            errors.push(err("transition-too-long", ipath, `a ${tr.duration}-frame transition needs both items to be at least ${tr.duration * 2} frames`));
          }
          // An overlapping transition is centred on the cut, so both clips need spare media
          // (handles) past the cut; a dip to black keeps them apart and needs none.
          const half = overlaps(tr.module) ? Math.ceil(tr.duration / 2) : 0;
          if (half > 0 && handleBefore(project.media, item) < half) {
            errors.push(err("transition-handles", ipath, `needs ${half} timeline frames of "${item.media}" before its in-point at its playback speed for the ${tr.module}`));
          }
          if (half > 0 && handleAfter(project.media, prev) < half) {
            errors.push(err("transition-handles", ipath, `"${prev.id}" needs ${half} timeline frames of "${prev.media}" after its out-point at its playback speed for the ${tr.module}`));
          }
        }
      }
    });

    // Free tracks: nothing may overlap on one track.
    if (!main) {
      const sorted = [...track.items].sort((a, b) => startOf(a) - startOf(b));
      for (let i = 1; i < sorted.length; i++) {
        const a = sorted[i - 1]!;
        const b = sorted[i]!;
        if (startOf(b) < startOf(a) + a.duration) errors.push(err("overlap", `${tpath}/items/${b.id}`, `"${b.id}" overlaps "${a.id}" on track "${track.id}"`));
      }
    }
  }
  return errors;
}
