// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// What the agent sees of the video analyzer (D67), pure: media_analyze's "video" answer, a
// media_rank entry, and the cells of contact_sheet at "shots". Times are seconds of the file in the
// cache; the agent gets project frames of the media (usable as in) next to them.
import type { VideoResult } from "../analysis/videoResult";
import { bestStretch, DEFAULT_BEST_SECONDS, meanScores, shotsOf, windowScores, type BestStretch, type Scores } from "../analysis/videoScores";

/** At most this many seconds of per-second scores in one media_analyze answer (range for the rest). */
export const MAX_SECONDS_LISTED = 600;
/** At most this many cuts listed in one answer. */
export const MAX_CUTS_LISTED = 200;
/** media_rank ranks at most this many media per call. */
export const MAX_RANKED = 50;

/** Seconds of the file to project frames of the media, rounded inwards (a hair of slack for float noise), clamped to its length. */
export type ToFrames = (seconds: number, round: (x: number) => number) => number;
export const framesFor =
  (fps: number, length: number | undefined): ToFrames =>
  (seconds, round) => {
    const f = round(seconds * fps + (round === Math.ceil ? -1e-6 : 1e-6));
    return Math.max(0, length ? Math.min(length, f) : f);
  };

const r2 = (v: number | null) => (v === null ? null : Math.round(v * 100) / 100);
const r3 = (v: number) => Math.round(v * 1000) / 1000;

/** Quality bands for the stretches list. */
const band = (clean: number) => (clean >= 0.7 ? "good" : clean >= 0.4 ? "fair" : "poor");

const scoresOut = (s: Scores) => ({ cleanScore: s.cleanScore, stability: s.stability, sharpness: s.sharpness, exposure: s.exposure, frozen: s.frozen });

/** A best stretch in project frames of the media and seconds. */
export const bestOut = (best: BestStretch, toFrames: ToFrames) => ({
  start: toFrames(best.start, Math.ceil),
  end: toFrames(best.end, Math.floor),
  startSeconds: best.start,
  endSeconds: best.end,
  ...scoresOut(best),
});

/** The whole file's raw measures, averaged over its seconds: what the scores come from. */
function measures(result: VideoResult) {
  const ws = result.windows ?? [];
  const mean = (pick: (w: (typeof ws)[number]) => number | null) => {
    let s = 0;
    let n = 0;
    for (const w of ws) {
      const v = pick(w);
      if (v === null) continue;
      s += v * w.frames;
      n += w.frames;
    }
    return n ? s / n : null;
  };
  const shake = mean((w) => w.jitter);
  const pan = mean((w) => w.motion);
  const blur = mean((w) => w.blur);
  return {
    shakePercent: shake === null ? null : r3(shake * 100),
    panPerSecond: pan === null ? null : r3(pan),
    blur: blur === null ? null : r2(blur),
    brightness: r2(mean((w) => w.brightness)),
  };
}

/** media_analyze's answer for the video analyzer. */
export function videoOut(result: VideoResult, cached: boolean, toFrames: ToFrames, range: { start: number; end: number } | undefined, fps: number, bestSeconds = DEFAULT_BEST_SECONDS) {
  if (!result.hasVideo) return { state: "done", cached, hasVideo: false, note: "This media has no picture, so there's nothing to measure." };
  const windows = result.windows ?? [];
  const overlaps = (start: number, end: number) => !range || (start * fps < range.end && end * fps > range.start);
  const listed = windows.filter((w) => overlaps(w.start, w.end));
  // Seconds with no frames (a gap in a variable-rate file) have no window: the per-second arrays
  // hold null there, so index i is always second firstSecond + i.
  const firstSecond = listed[0]?.start ?? 0;
  const lastSecond = listed.at(-1)?.start ?? -1;
  const span = Math.max(0, Math.min(lastSecond - firstSecond + 1, MAX_SECONDS_LISTED));
  const shown = listed.filter((w) => w.start < firstSecond + span);
  const scores = shown.map(windowScores);
  const slots = <T>(pick: (i: number) => T) => {
    const out: Array<T | null> = Array.from({ length: span }, () => null);
    shown.forEach((w, i) => (out[w.start - firstSecond] = pick(i)));
    return out;
  };
  // Runs of consecutive seconds in the same quality band (a gap ends a run).
  const stretches: Array<{ start: number; end: number; startSeconds: number; endSeconds: number; quality: string; cleanScore: number }> = [];
  let runStart = 0;
  for (let i = 1; i <= shown.length; i++) {
    if (i < shown.length && shown[i]!.start === shown[i - 1]!.start + 1 && band(scores[i]!.cleanScore) === band(scores[runStart]!.cleanScore)) continue;
    const part = shown.slice(runStart, i);
    const s = meanScores(part);
    stretches.push({ start: toFrames(part[0]!.start, Math.ceil), end: toFrames(part.at(-1)!.end, Math.floor), startSeconds: part[0]!.start, endSeconds: part.at(-1)!.end, quality: band(scores[runStart]!.cleanScore), cleanScore: s.cleanScore });
    runStart = i;
  }
  const cuts = (result.cuts ?? []).filter((c) => !range || (c.time * fps >= range.start && c.time * fps < range.end));
  const frozen = (result.frozen ?? []).filter((f) => overlaps(f.start, f.end));
  const drops = (result.drops ?? []).filter((d) => !range || (d.time * fps >= range.start && d.time * fps < range.end));
  const best = bestStretch(result, bestSeconds);
  return {
    state: "done",
    cached,
    hasVideo: true,
    seconds: result.seconds,
    sourceFps: result.fps,
    ...scoresOut(meanScores(windows)),
    measures: measures(result),
    ...(best ? { best: bestOut(best, toFrames) } : {}),
    shots: shotsOf(result).length,
    cuts: {
      threshold: result.cutThreshold,
      count: cuts.length,
      list: cuts.slice(0, MAX_CUTS_LISTED).map((c) => ({ frame: toFrames(c.time, Math.ceil), seconds: c.time, score: c.score })),
      ...(cuts.length > MAX_CUTS_LISTED ? { truncated: `Only the first ${MAX_CUTS_LISTED} of ${cuts.length}; pass range for the rest.` } : {}),
    },
    frozen: frozen.map((f) => ({ start: toFrames(f.start, Math.ceil), end: toFrames(f.end, Math.floor), startSeconds: f.start, endSeconds: f.end })),
    drops: drops.slice(0, MAX_CUTS_LISTED).map((d) => ({ frame: toFrames(d.time, Math.ceil), seconds: d.time, missingSeconds: d.missing })),
    stretches,
    perSecond: {
      firstSecond,
      cleanScore: slots((i) => r2(scores[i]!.cleanScore)),
      stability: slots((i) => r2(scores[i]!.stability)),
      sharpness: slots((i) => r2(scores[i]!.sharpness)),
      exposure: slots((i) => r2(scores[i]!.exposure)),
      panPerSecond: slots((i) => r2(shown[i]!.motion)),
      ...(lastSecond - firstSecond + 1 > span ? { truncated: `Only ${span} of ${lastSecond - firstSecond + 1} seconds; pass range for the rest.` } : {}),
    },
  };
}

export const VIDEO_NOTE =
  "Scores are 0–1, higher is better. stability: no camera shake (shake left after removing smooth pans; shakePercent is its RMS size in % of the frame width). sharpness: in focus, not blurred (from the edge width, blur). exposure: neither too dark nor too bright, little clipping. frozen: share of it where the picture doesn't change. cleanScore combines them (sharpness weighs most, since blur can't be fixed; shake can be stabilised). Cuts are where a new shot starts: frame is its first project frame of the media (use it as in). drops are gaps in the recording (dropped frames; the picture jumps at frame), not cuts: avoid a clip across one. best is the cleanest stretch inside one shot, crossing no drop. stretches group seconds into good (≥ 0.7), fair and poor; perSecond lists each second from firstSecond (null: no frames in that second). Frames are project frames of the media; seconds are media time.";

/** A media_rank entry (rank filled in by the caller after sorting). */
export function rankEntry(media: { readonly id: string; readonly name: string }, result: VideoResult, cached: boolean, toFrames: ToFrames, bestSeconds: number) {
  const scores = meanScores(result.windows ?? []);
  const best = bestStretch(result, bestSeconds);
  return {
    scores,
    best,
    out: {
      media: media.id,
      name: media.name,
      cached,
      seconds: result.seconds,
      ...scoresOut(scores),
      measures: measures(result),
      shots: shotsOf(result).length,
      drops: result.drops?.length ?? 0,
      ...(best ? { best: bestOut(best, toFrames) } : {}),
    },
  };
}

/** contact_sheet at "shots": the shots overlapping `range` (project frames of the media), one cell each, at the shot's middle. */
export function shotCells(result: VideoResult, fps: number, length: number, range: { start: number; end: number } | undefined, max: number) {
  const toFrames = framesFor(fps, length);
  const all = shotsOf(result)
    .map((s) => ({ start: toFrames(s.start, Math.ceil), end: Math.min(length, Math.max(toFrames(s.end, Math.ceil), toFrames(s.start, Math.ceil) + 1)) }))
    .filter((s) => s.end > s.start && (!range || (s.start < range.end && s.end > range.start)));
  // More shots than cells: spread evenly over them.
  const picked = all.length <= max ? all : Array.from({ length: max }, (_, i) => all[Math.floor((i * all.length) / max)]!);
  return { total: all.length, shots: picked.map((s) => ({ ...s, frame: Math.min(length - 1, Math.floor((s.start + s.end - 1) / 2)) })) };
}
