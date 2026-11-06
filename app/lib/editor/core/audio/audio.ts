// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Audio maths shared by the preview's playback mix and export (D39, D54): which items sound at
// project frame f, from where in their media, and how loud. Like visualsAt, frame f is the moment
// f / fps and may be fractional (the playback clock is).
//
// Loudness of an item at f, as a linear amplitude factor:
//   dB   = item.gainDb + gainKeys envelope at f + track.mixer.gainDb   (each 0 when unset)
//   gain = dbToGain(dB) × fade in × fade out × transition
// - gainKeys are an envelope in dB on top of gainDb: linear in dB between keys, held before the
//   first and after the last (frames from the item's start).
// - Fades are linear in amplitude (ffmpeg afade's default "tri" curve): 0 at the item's start →
//   1 after fadeIn frames; 1 at fadeOut frames before its end → 0 at its end.
// - A main-track transition crossfades the sound too, linearly over the transition's frames
//   (centred on the cut, like the picture): the outgoing clip goes 1 → 0 while the incoming one
//   goes 0 → 1. The item's own fade on that side is skipped, as the picture skips its animation.
//   A dip to black (transitions.ts) dips the sound instead: the outgoing clip fades out over its
//   last half of the transition, the incoming one fades in over its first half (no overlap).
// - A muted track is silent (gain 0); if any track is soloed, only soloed tracks sound. A hidden
//   track only hides the picture: its clips still sound (mute is the sound's switch).
// - dB at or below SILENCE_DB is silence (the volume slider's "−∞").
//
// Ducking (D85, ducking.ts) is a dB curve per track in project frames, added to each item's dB.
// Track effects (D84, effects.ts) process each track's sum before its pan; the master limiter
// (D86) comes after the master volume. Those are nonlinear and live in the preview's graph and the
// export's filters; the gains here stay linear.
//
// After the item gains (D83): each track's sum is panned (track mixer.pan, panGains below), the
// tracks are summed (no normalising) and the sum goes through the master gain (project
// master.gainDb, masterGain below; its own −60 dB cutoff). Both are linear, so the export pans each
// item's layer by its track's pan and applies the master volume once after amix: the same sound.
import { overlaps } from "@/app/lib/editor/core/visual/transitions";
import type { Item, Project, Track } from "@/app/lib/editor/core/timeline/types";

export const SILENCE_DB = -60;
export const MAX_DB = 12;

export const dbToGain = (db: number) => (db <= SILENCE_DB ? 0 : 10 ** (db / 20));
export const gainToDb = (gain: number) => (gain <= 0 ? -Infinity : 20 * Math.log10(gain));

const clamp01 = (p: number) => Math.min(1, Math.max(0, p));

export interface Audible {
  readonly item: Item;
  readonly track: Track;
  /** Linear amplitude factor (see the header); 0 while muted. */
  readonly gain: number;
  /** Where in the media, in seconds. */
  readonly source: number;
  /** The track is muted (or another is soloed): listed so playback can keep it warm, but silent. */
  readonly muted: boolean;
}

/** The gainKeys envelope in dB at `local` frames from the item's start. */
export function keyDbAt(keys: Item["gainKeys"], local: number): number {
  if (!keys?.length) return 0;
  const sorted = [...keys].sort((a, b) => a.frame - b.frame);
  if (local <= sorted[0]!.frame) return sorted[0]!.db;
  for (let i = 1; i < sorted.length; i++) {
    const a = sorted[i - 1]!;
    const b = sorted[i]!;
    if (local <= b.frame) return b.frame === a.frame ? b.db : a.db + ((b.db - a.db) * (local - a.frame)) / (b.frame - a.frame);
  }
  return sorted.at(-1)!.db;
}

/** Preserve the envelope on [from, to], including a cut through a ramp. */
export function sliceGainKeys(keys: Item['gainKeys'], from: number, to: number): Item['gainKeys'] {
  if (!keys?.length) return undefined;
  if (from === 0 && keys.every(key => key.frame <= to)) return keys;
  return [
    { frame: 0, db: keyDbAt(keys, from) },
    ...keys.filter(key => key.frame > from && key.frame < to).sort((a, b) => a.frame - b.frame).map(key => ({ ...key, frame: key.frame - from })),
    { frame: to - from, db: keyDbAt(keys, to) },
  ];
}

/** Fade in × fade out at `local` frames from the item's start (skipping a side a transition blends). */
export function fadeAt(item: Pick<Item, "duration" | "fadeIn" | "fadeOut">, local: number, { skipIn = false, skipOut = false } = {}): number {
  let g = 1;
  if (!skipIn && item.fadeIn && item.fadeIn > 0) g *= clamp01(local / item.fadeIn);
  if (!skipOut && item.fadeOut && item.fadeOut > 0) g *= clamp01((item.duration - local) / item.fadeOut);
  return g;
}

/** The item's media, if the item can make sound: an audio item, or a clip whose media has audio; never a muted one (D123: a clip whose sound was detached). */
export function soundMedia(project: Pick<Project, "media">, item: Item) {
  if ((item.type !== "clip" && item.type !== "audio") || item.mute) return undefined;
  const media = item.media ? project.media.find((m) => m.id === item.media) : undefined;
  if (!media || media.kind === "image") return undefined;
  if (media.kind === "audio" || media.probe?.hasAudio) return media;
  return undefined;
}

/**
 * The frames an item sounds over: its own, plus half an overlapping transition on either side of a
 * main-track cut (the outgoing clip plays on into the incoming one, which starts early). A sound
 * detached from that same source retains those transitions while its link and timing still match.
 * `dipIn` /
 * `dipOut` are the fade lengths of a dip to black at its start / end (half the dip; no overlap).
 */
type MainItem = { track: Track; index: number; start: number };
const mainItemsCache = new WeakMap<Project['tracks'], ReadonlyMap<string, MainItem>>();
function mainItems(tracks: Project['tracks']): ReadonlyMap<string, MainItem> {
  const known = mainItemsCache.get(tracks);
  if (known) return known;
  const items = new Map<string, MainItem>();
  for (const track of tracks) if (track.role === 'main') {
    let start = 0;
    track.items.forEach((item, index) => {
      start += item.gap ?? 0;
      items.set(item.id, { track, index, start });
      start += item.duration;
    });
  }
  mainItemsCache.set(tracks, items);
  return items;
}

export function audioSpan(project: Pick<Project, 'tracks'>, track: Track, index: number, start: number): { from: number; to: number; half: number; nextHalf: number; dipIn: number; dipOut: number } {
  const item = track.items[index]!;
  const main = track.role === "main";
  let into = main ? item.transitionIn : undefined;
  let out = main ? track.items[index + 1]?.transitionIn : undefined;
  if (!main && item.type === 'audio' && item.linkedTo) {
    const parent = mainItems(project.tracks).get(item.linkedTo);
    const picture = parent?.track.items[parent.index];
    // A separately recorded microphone or retained cutaway soundtrack owns its own envelope.
    // Only the actual detached source follows the picture's sound transitions.
    if (parent && picture?.type === 'clip' && picture.media === item.media
      && parent.start === (item.start ?? 0) && (picture.in ?? 0) === (item.in ?? 0)
      && (picture.speed ?? 1) === (item.speed ?? 1) && !!picture.loop === !!item.loop) {
      into = picture.transitionIn;
      // A held picture may be longer than its sound; do not move the later picture cut onto it.
      if (picture.duration === item.duration) out = parent.track.items[parent.index + 1]?.transitionIn;
    }
  }
  const half = into && overlaps(into.module) ? into.duration / 2 : 0;
  const nextHalf = out && overlaps(out.module) ? out.duration / 2 : 0;
  const dipIn = into && !overlaps(into.module) ? into.duration / 2 : 0;
  const dipOut = out && !overlaps(out.module) ? out.duration / 2 : 0;
  return { from: start - half, to: start + item.duration + nextHalf, half, nextHalf, dipIn, dipOut };
}

/** Where in its media an item is `local` frames after its start, in seconds (looped audio wraps). */
export function sourceAt(project: Pick<Project, "media" | "canvas">, item: Item, local: number): number {
  const fps = project.canvas.fps;
  let frames = (item.in ?? 0) + local * (item.speed ?? 1);
  const length = item.loop || item.hold ? project.media.find((m) => m.id === item.media)?.probe?.frames : undefined;
  if (length && length > 0 && frames >= length) frames = item.loop ? frames % length : length - 1;
  return frames / fps;
}

/** A held clip (D94) past its media's end: its picture is the last frame and it makes no sound. */
export function heldPast(project: Pick<Project, "media">, item: Item, local: number): boolean {
  if (!item.hold || item.loop || item.type !== "clip") return false;
  const length = project.media.find((m) => m.id === item.media)?.probe?.frames;
  return !!length && (item.in ?? 0) + local * (item.speed ?? 1) >= length;
}

/** Whether a track holds anything that can make sound (a clip with sound or an audio item). */
export const trackHasSound = (project: Pick<Project, "media">, track: Track) => track.items.some((item) => soundMedia(project, item) !== undefined);

/**
 * Whether a track sounds at all: not muted, and soloed if any track that can sound is (anySolo).
 * Mute wins: a soloed track that is also muted stays silent (and still silences the others). The export follows the same
 * rule, so it is what the preview plays (D83).
 */
export function trackAudible(project: Pick<Project, "tracks" | "media">, track: Track): boolean {
  if (track.mixer?.mute) return false;
  return !anySolo(project) || track.mixer?.solo === true;
}

/**
 * The pan law (D83), one law for every source after it is made stereo (mono plays at full level on
 * both channels, as before): θ = (pan + 1)·π/4; the near side stays at 1 and the far side follows
 * √2·cos θ (left) / √2·sin θ (right), so left = min(1, √2·cos θ), right = min(1, √2·sin θ). It never
 * boosts: centre is exactly unity on both channels (a pan of 0 sounds exactly as before pan
 * existed, and a pan can't push a full-scale source into clipping); −1 silences the right and keeps
 * the left at 0 dB; ±0.5 takes the far side down 5.3 dB. On a mono source (L = R) the far side
 * follows the equal-power curve with the near side held; on a stereo source it is a balance: each
 * side keeps its own channel, the far one fading out (no folding of one side into the other).
 */
export function panGains(pan: number): { readonly left: number; readonly right: number } {
  const p = Math.min(1, Math.max(-1, Number.isFinite(pan) ? pan : 0));
  if (p === 0) return { left: 1, right: 1 };
  const theta = ((p + 1) * Math.PI) / 4;
  // Exact at the ends (cos(π/2) isn't quite 0 in floating point); the near side is exactly 1.
  if (p < 0) return { left: 1, right: p === -1 ? 0 : Math.min(1, Math.SQRT2 * Math.sin(theta)) };
  return { left: p === 1 ? 0 : Math.min(1, Math.SQRT2 * Math.cos(theta)), right: 1 };
}

/** A track's pan, −1 (left) … 1 (right); 0 when unset. */
export const trackPan = (track: Pick<Track, "mixer">) => Math.min(1, Math.max(-1, track.mixer?.pan ?? 0));

/** The master volume (D83): project master.gainDb as a linear factor, applied after the tracks are summed. */
export const masterGain = (project: Pick<Project, "master">) => dbToGain(project.master?.gainDb ?? 0);

/**
 * Whether any track that can make sound is soloed (then only soloed, unmuted tracks sound:
 * trackAudible). A solo on a track with nothing to play (a text track, an audio track whose items
 * were deleted) is ignored, so it can't silence everything unseen.
 */
export const anySolo = (project: Pick<Project, "tracks" | "media">) => project.tracks.some((t) => t.mixer?.solo === true && trackHasSound(project, t));

/** The tracks that are heard: they can make sound and trackAudible says so (mute, solo). */
export const audibleTracks = (project: Pick<Project, "tracks" | "media">) => project.tracks.filter((t) => trackHasSound(project, t) && trackAudible(project, t));

const names = (tracks: ReadonlyArray<Track>) => {
  const list = tracks.map((t) => t.name);
  return list.length <= 1 ? (list[0] ?? "") : `${list.slice(0, -1).join(", ")} and ${list.at(-1)}`;
};
/**
 * What the mixer's solo, mutes and master volume leave audible, as a sentence for the Mixer, the
 * export dialog and the export plan's warnings (so export jobs and the agent see it too); undefined
 * when nothing needs saying (no solo, something plays, the master isn't silent).
 */
export function mixNote(project: Pick<Project, "tracks" | "media" | "master">): string | undefined {
  const sounding = project.tracks.filter((t) => trackHasSound(project, t));
  if (!sounding.length) return undefined;
  if (masterGain(project) === 0) return "Nothing will be heard: the master volume is at −∞.";
  const audible = audibleTracks(project);
  if (!audible.length) {
    const soloed = sounding.filter((t) => t.mixer?.solo === true);
    return soloed.length ? `Nothing will be heard: ${names(soloed)} ${soloed.length === 1 ? "is" : "are"} soloed but muted.` : "Nothing will be heard: every track with sound is muted.";
  }
  if (anySolo(project)) return `Solo is on: only ${names(audible)} will be heard, in the preview and the export.`;
  return undefined;
}

/**
 * An item's gain at `local` frames from its start (see the header), before mute and solo.
 * `duckDb` is the track's ducking at that moment (D85, ducking.ts duckDbAt), added to the combined
 * dB like a volume key (so the −60 dB cutoff covers it too).
 */
export function itemGain(project: Pick<Project, 'tracks'>, track: Track, index: number, local: number, duckDb = 0): number {
  const item = track.items[index]!;
  const { half, nextHalf, dipIn, dipOut } = audioSpan(project, track, index, 0);
  const db = (item.gainDb ?? 0) + keyDbAt(item.gainKeys, Math.min(item.duration, Math.max(0, local))) + (track.mixer?.gainDb ?? 0) + duckDb;
  let gain = dbToGain(db) * fadeAt(item, local, { skipIn: half > 0 || dipIn > 0, skipOut: nextHalf > 0 || dipOut > 0 });
  if (dipIn > 0) gain *= clamp01(local / dipIn);
  if (dipOut > 0) gain *= clamp01((item.duration - local) / dipOut);
  if (half > 0 && local < half) gain *= clamp01((local + half) / (2 * half));
  if (nextHalf > 0 && local > item.duration - nextHalf) gain *= clamp01((item.duration + nextHalf - local) / (2 * nextHalf));
  return gain;
}

/** A track's duck curve's dB at a project frame (D85); the preview and the export pass ducking.ts's. */
export type DuckAt = (trackId: string, frame: number) => number;

/** Everything that sounds at frame f, with its gain and source time, in track order. `duck`: the ducking (D85). */
export function audibleAt(project: Pick<Project, "tracks" | "media" | "canvas">, starts: ReadonlyMap<string, number>, f: number, duck?: DuckAt): Array<Audible> {
  const out: Array<Audible> = [];
  for (const track of project.tracks) {
    const on = trackAudible(project, track);
    track.items.forEach((item, index) => {
      if (!soundMedia(project, item)) return;
      const start = starts.get(item.id) ?? 0;
      const { from, to } = audioSpan(project, track, index, start);
      if (f < from || f >= to) return;
      const local = f - start;
      // A held clip (D94) past its media's end makes no sound (the preview stops its element).
      if (heldPast(project, item, local)) return;
      out.push({ item, track, gain: on ? itemGain(project, track, index, local, duck?.(track.id, f) ?? 0) : 0, source: sourceAt(project, item, local), muted: !on });
    });
  }
  return out;
}

/**
 * The whole chain for one item at `local` frames from its start, per output channel (D83): item
 * gain (volume, keys, fades, transitions, track volume) × the track's pan × the master volume, and
 * 0 when the track is muted or another is soloed. What reaches the speakers (and the export's mix)
 * for each channel of the item's sound, made stereo.
 */
export function outputGains(project: Pick<Project, "tracks" | "media" | "master">, track: Track, index: number, local: number, duckDb = 0): { readonly left: number; readonly right: number } {
  if (!trackAudible(project, track)) return { left: 0, right: 0 };
  const g = itemGain(project, track, index, local, duckDb) * masterGain(project);
  const pan = panGains(trackPan(track));
  return { left: g * pan.left, right: g * pan.right };
}
