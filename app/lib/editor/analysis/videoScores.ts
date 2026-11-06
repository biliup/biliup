// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Scores from the video analyzer's measurements (D67), pure: per second of a file and per file,
// each 0–1 (1 is best), plus the best stretch of a take. Kept apart from the measurements so the
// scoring can be tuned without analysing again (the cache holds measurements).
//
// - stability = 1 / (1 + (jitter / STABILITY_HALF)²): jitter is the camera's shake (RMS distance
//   between its path and its smoothed path, in frame widths); 0.3 % of the width scores 0.5.
// - sharpness = 1 / (1 + (max(0, blur − SHARP_BLUR) / SHARP_SPAN)²): blur is blurdetect's edge width
//   at 320 px; sharp footage measures about 4–6.5, a soft focus 8+, a heavy blur 11+.
// - exposure: 1 for a mean brightness in 0.25–0.75, falling to 0 at 0.03 / 0.97, times a penalty
//   for clipped pixels (near black or white) beyond 2 %, reaching 0 at 42 %.
// - clean = stability^0.3 · sharpness^0.5 · exposure^0.2 · (1 − frozen share). Sharpness weighs
//   most: shake can be stabilised (with a crop) and a dark shot lifted, a blurred one can't be
//   fixed. A component that couldn't be measured (no texture to track, nothing to measure blur on)
//   is left out and the others' weights renormalised.
import type { VideoResult, VideoWindow } from "./videoResult";

export const STABILITY_HALF = 0.003;
export const SHARP_BLUR = 5.5;
export const SHARP_SPAN = 3;
export const WEIGHTS = { stability: 0.3, sharpness: 0.5, exposure: 0.2 } as const;
/** A best stretch is this many seconds unless asked otherwise. */
export const DEFAULT_BEST_SECONDS = 5;

export interface Scores {
  readonly cleanScore: number;
  readonly stability: number | null;
  readonly sharpness: number | null;
  readonly exposure: number;
  /** Share of it that is frozen (0–1). */
  readonly frozen: number;
}

const clamp01 = (v: number) => Math.max(0, Math.min(1, v));
const r3 = (v: number) => Math.round(v * 1000) / 1000;

export const stabilityOf = (jitter: number | null) => (jitter === null ? null : 1 / (1 + (jitter / STABILITY_HALF) ** 2));
export const sharpnessOf = (blur: number | null) => (blur === null ? null : 1 / (1 + (Math.max(0, blur - SHARP_BLUR) / SHARP_SPAN) ** 2));
export function exposureOf(brightness: number, dark: number, bright: number): number {
  const level = brightness < 0.25 ? clamp01((brightness - 0.03) / 0.22) : brightness > 0.75 ? clamp01((0.97 - brightness) / 0.22) : 1;
  const clipped = clamp01((dark + bright - 0.02) / 0.4);
  return level * (1 - clipped);
}

/** clean from components (null ones left out, weights renormalised). */
export function cleanOf(stability: number | null, sharpness: number | null, exposure: number, frozen: number): number {
  const parts: Array<[number, number]> = [[exposure, WEIGHTS.exposure]];
  if (stability !== null) parts.push([stability, WEIGHTS.stability]);
  if (sharpness !== null) parts.push([sharpness, WEIGHTS.sharpness]);
  const total = parts.reduce((s, [, w]) => s + w, 0);
  const geo = Math.exp(parts.reduce((s, [v, w]) => s + w * Math.log(Math.max(1e-6, v)), 0) / total);
  return clamp01(geo * (1 - frozen));
}

/** One second's scores. */
export function windowScores(w: VideoWindow): Scores {
  const stability = stabilityOf(w.jitter);
  const sharpness = sharpnessOf(w.blur);
  const exposure = exposureOf(w.brightness, w.dark, w.bright);
  return { cleanScore: r3(cleanOf(stability, sharpness, exposure, w.frozen)), stability: stability === null ? null : r3(stability), sharpness: sharpness === null ? null : r3(sharpness), exposure: r3(exposure), frozen: r3(w.frozen) };
}

/** Frame-weighted means of the windows' scores (a component no window measured stays null). */
export function meanScores(windows: ReadonlyArray<VideoWindow>): Scores {
  let frames = 0;
  const sum = { clean: 0, stability: 0, stabilityN: 0, sharpness: 0, sharpnessN: 0, exposure: 0, frozen: 0 };
  for (const w of windows) {
    const s = windowScores(w);
    const n = w.frames;
    frames += n;
    sum.clean += s.cleanScore * n;
    sum.exposure += s.exposure * n;
    sum.frozen += s.frozen * n;
    if (s.stability !== null) {
      sum.stability += s.stability * n;
      sum.stabilityN += n;
    }
    if (s.sharpness !== null) {
      sum.sharpness += s.sharpness * n;
      sum.sharpnessN += n;
    }
  }
  if (!frames) return { cleanScore: 0, stability: null, sharpness: null, exposure: 0, frozen: 0 };
  return {
    cleanScore: r3(sum.clean / frames),
    stability: sum.stabilityN ? r3(sum.stability / sum.stabilityN) : null,
    sharpness: sum.sharpnessN ? r3(sum.sharpness / sum.sharpnessN) : null,
    exposure: r3(sum.exposure / frames),
    frozen: r3(sum.frozen / frames),
  };
}

/** The shots of a file (seconds): from 0 to the first cut, cut to cut, the last cut to the end. */
export function shotsOf(result: VideoResult): Array<{ start: number; end: number }> {
  const end = result.seconds ?? 0;
  const starts = [0, ...(result.cuts ?? []).map((c) => c.time).filter((t) => t > 0 && t < end)].sort((a, b) => a - b);
  return starts.map((start, i) => ({ start, end: starts[i + 1] ?? end })).filter((s) => s.end > s.start);
}

export interface BestStretch extends Scores {
  /** Seconds of the file [start, end). */
  readonly start: number;
  readonly end: number;
}

/**
 * The best `seconds`-long stretch (whole seconds of the file) by mean clean score that doesn't
 * cross a cut or a gap in the recording, so it's one continuous shot. Shots shorter than that: the best shot (or its best
 * part) as long as fits. Undefined for a file with no windows. Linear in the windows per length tried.
 */
export function bestStretch(result: VideoResult, seconds = DEFAULT_BEST_SECONDS): BestStretch | undefined {
  const windows = result.windows ?? [];
  const n = windows.length;
  if (!n) return undefined;
  // Neither a cut nor a gap in the recording (dropped frames, a visible jump) inside it.
  const barriers = [...(result.cuts ?? []).map((c) => c.time), ...(result.drops ?? []).map((d) => d.time)].sort((x, y) => x - y);
  const scores = windows.map(windowScores);
  // Per window, in one pass over the sorted barriers: one inside it (blocks any stretch holding
  // it), or one at its start (blocks a stretch that runs on from the window before), or a gap
  // in time before it (no frames in a second: the stretch wouldn't be continuous).
  const inner = new Uint8Array(n);
  const edge = new Uint8Array(n);
  let b = 0;
  for (let k = 0; k < n; k++) {
    const w = windows[k]!;
    if (k > 0 && w.start !== windows[k - 1]!.start + 1) edge[k] = 1;
    while (b < barriers.length && barriers[b]! < w.start - 0.05) b++;
    for (let j = b; j < barriers.length && barriers[j]! < w.end - 0.05; j++) {
      if (barriers[j]! <= w.start + 0.05) edge[k] = 1;
      else inner[k] = 1;
    }
  }
  // Prefix sums: clean scores, blocked windows, blocked edges.
  const cleanSum = new Float64Array(n + 1);
  const innerSum = new Uint32Array(n + 1);
  const edgeSum = new Uint32Array(n + 1);
  for (let k = 0; k < n; k++) {
    cleanSum[k + 1] = cleanSum[k]! + scores[k]!.cleanScore;
    innerSum[k + 1] = innerSum[k]! + inner[k]!;
    edgeSum[k + 1] = edgeSum[k]! + edge[k]!;
  }
  // Blocked: a barrier inside any of its windows, or at the start of (or a time gap before) any but its first.
  const crossesAt = (from: number, length: number) => innerSum[from + length]! - innerSum[from]! > 0 || edgeSum[from + length]! - edgeSum[from + 1]! > 0;
  for (let length = Math.max(1, Math.min(Math.round(seconds), n)); length >= 1; length--) {
    let best: { from: number; mean: number } | undefined;
    for (let from = 0; from + length <= n; from++) {
      if (crossesAt(from, length)) continue;
      const mean = (cleanSum[from + length]! - cleanSum[from]!) / length;
      if (!best || mean > best.mean + 1e-9) best = { from, mean };
    }
    if (best) {
      const part = windows.slice(best.from, best.from + length);
      const s = meanScores(part);
      return { ...s, start: r3(part[0]!.start), end: r3(part.at(-1)!.end) };
    }
  }
  return undefined;
}

export type RankBy = "clean" | "best" | "stability" | "sharpness" | "exposure";
export const RANK_BY: ReadonlyArray<RankBy> = ["clean", "best", "stability", "sharpness", "exposure"];

/** The number a take is ranked by (higher first); unmeasured components rank last. */
export function rankValue(scores: Scores, best: BestStretch | undefined, by: RankBy): number {
  if (by === "best") return best?.cleanScore ?? -1;
  if (by === "clean") return scores.cleanScore;
  return scores[by] ?? -1;
}
