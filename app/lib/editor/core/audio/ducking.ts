// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Ducking, precomputed (phase 6 chunk 7, D85). A track with an audio.duck entry in its
// mixer.effects goes down by amountDb while any of its key tracks (params.under) has speech. The
// speech comes from the loudness analysis's envelope (D74: RMS dBFS every 10 ms) of the media on
// the key tracks, mapped through the timeline (each key item's span, in point, speed, loop, its own
// gain: a key item at −∞ or faded to 0 doesn't count). The result is a gain curve in dB, piecewise
// linear in project frames (duckKeys), which the preview adds to each item's gain on the ducked
// track (core itemGain's duckDb) and the export folds into the same volume expression: the same
// numbers, no live sidechain.
//
// Shape: speech windows above the threshold (thresholdDb, or automatically the speech level less
// 25 dB and at least 10 dB over the floor, as D74's word edges) are joined over gaps shorter than
// DUCK_HOLD_S, blips shorter than DUCK_MIN_S dropped, then joined over gaps shorter than attack +
// release (so one duck never starts before the last has recovered). Each run [s, e] becomes a
// trapezoid: 0 dB at s − attack, amountDb at s (the duck is down when the speech starts: the
// curve is known ahead), amountDb until e, back to 0 dB at e + release.
import { soundMedia, trackAudible } from "@/app/lib/editor/core/audio/audio";
import { itemEnvelope } from './envelope';
import { DUCK } from "@/app/lib/editor/core/audio/effects";
import { layout } from "@/app/lib/editor/core/timeline/timeline";
import { speechThreshold, type SpeechEnvelope } from "@/app/lib/editor/core/text/transcript";
import type { ModuleUse, Project, Track } from "@/app/lib/editor/core/timeline/types";

export const DUCK_DEFAULTS = { amountDb: -12, attackMs: 250, releaseMs: 600 } as const;
/** Gaps in the key's speech shorter than this (s) keep the duck down (between words and phrases). */
export const DUCK_HOLD_S = 0.3;
/** Speech shorter than this (s) doesn't duck (a click, a cough). */
export const DUCK_MIN_S = 0.05;

export interface DuckSettings {
  readonly under: ReadonlyArray<string>;
  readonly amountDb: number;
  readonly attackMs: number;
  readonly releaseMs: number;
  /** dBFS of the key media; undefined = automatic per media. */
  readonly thresholdDb?: number | undefined;
}

/** A point of the duck curve: project frame (may be fractional) and dB (≤ 0). */
export interface DuckKey {
  readonly frame: number;
  readonly db: number;
}

export interface DuckCurve {
  readonly settings: DuckSettings;
  /** The curve: piecewise linear in dB between keys, 0 dB outside them. */
  readonly keys: ReadonlyArray<DuckKey>;
  /** Key tracks that exist, aren't this track and are heard (muted or soloed-out keys duck nothing). */
  readonly keyTracks: ReadonlyArray<string>;
  /** Media on the key tracks whose loudness envelope isn't known yet (no analysis cached): they duck nothing until it is. */
  readonly missing: ReadonlyArray<string>;
}

const clamp = (v: unknown, fallback: number, lo: number, hi: number) => (typeof v === "number" && Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : fallback);

/** A duck entry's settings, defaults filled in. */
export function duckSettings(params: ModuleUse["params"]): DuckSettings {
  const p = params ?? {};
  const under = Array.isArray(p.under) ? [...new Set((p.under as ReadonlyArray<unknown>).filter((u): u is string => typeof u === "string"))] : [];
  const threshold = typeof p.thresholdDb === "number" && Number.isFinite(p.thresholdDb) ? Math.min(0, Math.max(-80, p.thresholdDb)) : undefined;
  return {
    under,
    amountDb: clamp(p.amountDb, DUCK_DEFAULTS.amountDb, -40, 0),
    attackMs: clamp(p.attackMs, DUCK_DEFAULTS.attackMs, 0, 2000),
    releaseMs: clamp(p.releaseMs, DUCK_DEFAULTS.releaseMs, 0, 5000),
    ...(threshold !== undefined ? { thresholdDb: threshold } : {}),
  };
}

/** The track's duck entry (the first one that isn't bypassed), if any. */
export const duckEntry = (track: Pick<Track, "mixer">) => track.mixer?.effects?.find((fx) => fx.module === DUCK && !fx.bypass);
/** The track's ducking settings, if it ducks. */
export function duckOf(track: Pick<Track, "mixer">): DuckSettings | undefined {
  const fx = duckEntry(track);
  return fx ? duckSettings(fx.params) : undefined;
}

/** The key tracks that count for a ducked track: they exist, aren't the track itself, and are heard. */
function keyTracksOf(project: Pick<Project, "tracks" | "media">, track: Track, settings: DuckSettings): Array<Track> {
  return settings.under.map((id) => project.tracks.find((t) => t.id === id)).filter((t): t is Track => !!t && t.id !== track.id && trackAudible(project, t));
}

/** Media whose envelopes the project's ducking needs (on the heard key tracks of every ducked track). */
export function duckMediaIds(project: Pick<Project, "tracks" | "media">): Array<string> {
  const ids = new Set<string>();
  for (const track of project.tracks) {
    const settings = duckOf(track);
    if (!settings) continue;
    for (const key of keyTracksOf(project, track, settings)) for (const item of key.items) if (soundMedia(project, item) && item.media) ids.add(item.media);
  }
  return [...ids].sort();
}

const round4 = (x: number) => Math.round(x * 1e4) / 1e4;

/**
 * The duck curve of a track (undefined when it doesn't duck). `envelopes` maps media ids to their
 * decoded envelope (null: analysed, no sound); a key media missing from it ducks nothing and is
 * listed in `missing`.
 */
export function duckCurve(
  project: Pick<Project, "tracks" | "media" | "canvas">,
  track: Track,
  envelopes: ReadonlyMap<string, SpeechEnvelope | null>,
  starts: ReadonlyMap<string, number> = layout(project).starts,
): DuckCurve | undefined {
  const settings = duckOf(track);
  if (!settings) return undefined;
  const fps = project.canvas.fps;
  const keys = keyTracksOf(project, track, settings);
  const missing = new Set<string>();
  // Speech as [start, end) in project seconds, on a 10 ms grid of project time.
  const runs: Array<[number, number]> = [];
  for (const key of keys) {
    key.items.forEach((item, index) => {
      const media = soundMedia(project, item);
      if (!media) return;
      const env = envelopes.get(media.id);
      if (env === undefined) return void missing.add(media.id);
      if (!env || !env.db.length) return;
      const thr = settings.thresholdDb ?? speechThreshold(env);
      const start = starts.get(item.id) ?? 0;
      let open: number | undefined;
      let end = 0;
      for (const sample of itemEnvelope(project, key, index, start, env)) {
        const on = sample.sourceDb >= thr;
        if (open !== undefined && (!on || sample.from > end + 1e-6)) {
          runs.push([open / fps, end / fps]);
          open = undefined;
        }
        if (on && open === undefined) open = sample.from;
        end = sample.to;
      }
      if (open !== undefined) runs.push([open / fps, end / fps]);
    });
  }
  const attack = settings.attackMs / 1000;
  const release = settings.releaseMs / 1000;
  const join = (list: Array<[number, number]>, gap: number) => {
    const out: Array<[number, number]> = [];
    for (const r of [...list].sort((a, b) => a[0] - b[0])) {
      const last = out.at(-1);
      if (last && r[0] - last[1] < gap - 1e-9) last[1] = Math.max(last[1], r[1]);
      else out.push([r[0], r[1]]);
    }
    return out;
  };
  const held = join(runs, DUCK_HOLD_S).filter(([s, e]) => e - s >= DUCK_MIN_S - 1e-9);
  const merged = join(held, Math.max(DUCK_HOLD_S, attack + release));
  const out: Array<DuckKey> = [];
  if (settings.amountDb < 0) {
    for (const [s, e] of merged) {
      out.push({ frame: round4((s - attack) * fps), db: 0 }, { frame: round4(s * fps), db: settings.amountDb }, { frame: round4(e * fps), db: settings.amountDb }, { frame: round4((e + release) * fps), db: 0 });
    }
  }
  return { settings, keys: out, keyTracks: keys.map((t) => t.id), missing: [...missing].sort() };
}

/** Every ducked track's curve, by track id. */
export function duckCurves(project: Pick<Project, "tracks" | "media" | "canvas">, envelopes: ReadonlyMap<string, SpeechEnvelope | null>): Map<string, DuckCurve> {
  const out = new Map<string, DuckCurve>();
  const { starts } = layout(project);
  for (const track of project.tracks) {
    const c = duckCurve(project, track, envelopes, starts);
    if (c) out.set(track.id, c);
  }
  return out;
}

/** The curve's dB at project frame `frame` (0 outside its keys; a step where two keys share a frame). */
export function duckDbAt(keys: ReadonlyArray<DuckKey> | undefined, frame: number): number {
  if (!keys?.length || frame < keys[0]!.frame || frame > keys.at(-1)!.frame) return 0;
  return pwlAt(keys, frame);
}

/** A piecewise-linear curve at `x`, held at its ends: the last key at or before x, towards the next (a step where two share a frame). */
export function pwlAt(keys: ReadonlyArray<DuckKey>, x: number): number {
  if (x < keys[0]!.frame) return keys[0]!.db;
  let lo = 0;
  let hi = keys.length;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (keys[mid]!.frame <= x) lo = mid;
    else hi = mid;
  }
  const a = keys[lo]!;
  const b = keys[lo + 1];
  return !b || b.frame === a.frame ? a.db : a.db + ((b.db - a.db) * (x - a.frame)) / (b.frame - a.frame);
}

/** The keys a span of project frames needs (those inside it and one either side), for drawing or an expression. */
export function duckKeysIn(keys: ReadonlyArray<DuckKey>, from: number, to: number): Array<DuckKey> {
  let i0 = keys.findIndex((k) => k.frame >= from);
  if (i0 < 0) return [];
  i0 = Math.max(0, i0 - 1);
  let i1 = keys.length - 1;
  for (let i = i0; i < keys.length; i++) if (keys[i]!.frame > to) {
    i1 = i;
    break;
  }
  const out = keys.slice(i0, i1 + 1);
  return out.some((k) => k.db !== 0) ? out : [];
}

/** What the duck needs said (the Mixer, the export plan's warnings, the agent): missing analysis, no key heard. */
export function duckNote(project: Pick<Project, "tracks" | "media">, track: Track, curve: DuckCurve | undefined): string | undefined {
  if (!curve) return undefined;
  const names = (ids: ReadonlyArray<string>) => ids.map((id) => project.media.find((m) => m.id === id)?.name ?? id).join(", ");
  if (!curve.keyTracks.length) return `${track.name} ducks under nothing that is heard (the key track is muted or soloed out).`;
  if (curve.missing.length) return `${track.name}'s ducking waits for the loudness of ${names(curve.missing)} to be measured.`;
  return undefined;
}
