// Adapted from Uncut agent/agentPlan.ts (D191/D296), MIT; licenses/uncut/LICENSE.
import type {BeatsResult} from "../agent/analysisTypes";
import {sourcePoints} from '../core/timeline/sourceSpans';
export interface GridItem {
  readonly id: string;
  readonly media: string;
  readonly start: number;
  readonly in: number;
  readonly duration: number;
  readonly speed: number;
  readonly loop: boolean;
  readonly sourceFrames: number | undefined;
}

/** D296: a drop by level: the low bands' 2 bars after the beat at least this many times the 2 bars before, */
export const DROP_LEVEL_RISE = 2;
/** that beat at least this many times the 2 beats before it (a step, not a ramp), */
export const DROP_STEP = 1.5;
/** and the bar after it at least this share of the loudest bar (the drop is the loud part). */
export const DROP_LOUD = 0.6;

const meanOf = (xs: ReadonlyArray<number>) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : 0);

/**
 * The drop. D296: by level when the analysis has beatLevels: the beat where the low bands (kick,
 * bass) step up: the 2 bars after it at least DROP_LEVEL_RISE times the 2 bars before, the beat
 * itself DROP_STEP times the 2 beats before it, and the bar after it loud (DROP_LOUD of the loudest
 * bar); the biggest such rise, any beat from the third bar on (a drop lands on a downbeat, so the
 * grid's downbeats follow it: `beat` is its index). Without levels (D295) by onsets: the downbeat after which
 * the next 2 bars carry the most onset strength vs the 2 bars before, at least 1.5 times. An even
 * build gets undefined either way. Seconds of the media.
 */
export function dropOf(beats: ReadonlyArray<number>, phase: number, onsets: ReadonlyArray<{ t: number; strength: number }>, levels?: ReadonlyArray<number>): { seconds: number; rise: number; beat: number } | undefined {
  if (beats.length < 16) return undefined;
  if (levels && levels.length === beats.length) {
    const n = levels.length;
    const bars = Array.from({ length: Math.max(0, n - 3) }, (_, i) => meanOf(levels.slice(i, i + 4)));
    const loudest = Math.max(...bars, 0);
    let best: { seconds: number; rise: number; beat: number } | undefined;
    for (let i = 8; i + 4 <= n; i++) {
      const before = meanOf(levels.slice(i - 8, i));
      const after = meanOf(levels.slice(i, Math.min(n, i + 8)));
      const rise = after / Math.max(before, 0.01);
      const step = levels[i]! / Math.max(meanOf(levels.slice(i - 2, i)), 0.01);
      if (rise >= DROP_LEVEL_RISE && step >= DROP_STEP && bars[i]! >= DROP_LOUD * loudest && (!best || rise > best.rise)) best = { seconds: beats[i]!, rise: Math.round(rise * 10) / 10, beat: i };
    }
    return best;
  }
  const energy = (from: number, to: number) => onsets.reduce((sum, o) => (o.t >= from && o.t < to ? sum + o.strength : sum), 0);
  let best: { seconds: number; rise: number; beat: number } | undefined;
  for (let i = phase + 8; i + 8 <= beats.length; i += 4) {
    const before = energy(beats[i - 8]!, beats[i]!);
    const after = energy(beats[i]!, i + 8 < beats.length ? beats[i + 8]! : Infinity);
    const rise = after / Math.max(before, 0.5);
    if (rise >= 1.5 && (!best || rise > best.rise)) best = { seconds: beats[i]!, rise: Math.round(rise * 10) / 10, beat: i };
  }
  return best;
}

/**
 * The downbeat phase (one beat in 4: the phase whose beats carry the most onset strength; D296: the
 * drop's, when there is one, since a drop lands on a downbeat) and the drop, in seconds of the media.
 */
export function downbeatsOf(result: BeatsResult): { phase: number; drop: ReturnType<typeof dropOf> } {
  const beatsS = result.beats ?? [];
  const onsets = result.onsets ?? [];
  // Onset strength near each beat (within a quarter beat).
  const half = result.bpm ? 60 / result.bpm / 4 : 0.1;
  const weight = beatsS.map((b) => onsets.reduce((sum, o) => (Math.abs(o.t - b) <= half ? sum + o.strength : sum), 0));
  let phase = 0;
  let best = -1;
  for (let p = 0; p < 4; p++) {
    const w = weight.reduce((sum, x, i) => (i % 4 === p ? sum + x : sum), 0);
    if (w > best) {
      best = w;
      phase = p;
    }
  }
  const drop = dropOf(beatsS, phase, onsets, result.beatLevels);
  return { phase: drop ? drop.beat % 4 : phase, drop };
}

/**
 * The beat grid in timeline frames (D191): beats, downbeats (one beat in 4: the phase whose beats
 * carry the most onset strength) and the strongest hits, inside the item; snaps of the given
 * frames to the nearest beat and downbeat.
 */
export function beatGrid(result: BeatsResult, item: GridItem, fps: number, range?: { start: number; end: number }, snap?: ReadonlyArray<number>) {
  const end = item.start + item.duration;
  const inItem = (f: number) => f >= item.start && f < end && (!range || (f >= range.start && f < range.end));
  // Keep events that round into the requested whole-frame window at either edge.
  const window = range ? {start: range.start - item.start - 0.5, end: range.end - item.start + 0.5} : undefined;
  const beatsS = result.beats ?? [];
  const onsets = result.onsets ?? [];
  const { phase, drop } = downbeatsOf(result);
  const beatSet = new Set<number>();
  const downbeatSet = new Set<number>();
  for (const point of sourcePoints(item, item.sourceFrames, beatsS, fps, window)) {
    const frame = Math.round(item.start + point.local);
    if (!inItem(frame)) continue;
    beatSet.add(frame);
    if (point.index % 4 === phase) downbeatSet.add(frame);
  }
  const beats = [...beatSet];
  const downbeats = [...downbeatSet];
  const byHit = new Map<number, number>();
  // Onsets retain their strength when repeated; one hit per frame keeps the strongest event.
  for (const point of sourcePoints(item, item.sourceFrames, onsets.map(onset => onset.t), fps, window)) {
    const frame = Math.round(item.start + point.local);
    const strength = Math.round(onsets[point.index]!.strength * 100) / 100;
    if (inItem(frame) && strength >= 0.6) byHit.set(frame, Math.max(strength, byHit.get(frame) ?? 0));
  }
  const hits = [...byHit].map(([frame, strength]) => ({frame, strength}))
    .sort((a, b) => b.strength - a.strength)
    .slice(0, 24)
    .sort((a, b) => a.frame - b.frame);
  const drops: Array<{frame: number; cycle: number; rise: number}> = [];
  if (drop) for (const point of sourcePoints(item, item.sourceFrames, [drop.seconds], fps, window)) {
    const frame = Math.round(item.start + point.local);
    if (inItem(frame) && drops.at(-1)?.frame !== frame) drops.push({frame, cycle: point.cycle, rise: drop.rise});
  }
  const nearest = (list: ReadonlyArray<number>, frame: number) => {
    let lo = 0;
    let hi = list.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (list[mid]! < frame) lo = mid + 1;
      else hi = mid;
    }
    const before = list[lo - 1];
    const after = list[lo];
    return before === undefined ? after : after === undefined || frame - before <= after - frame ? before : after;
  };
  const snaps = (snap ?? []).map((f) => {
    const beat = nearest(beats, f);
    const down = nearest(downbeats, f);
    return { frame: f, ...(beat !== undefined ? { beat, off: beat - f } : {}), ...(down !== undefined ? { downbeat: down } : {}) };
  });
  return {
    item: item.id,
    bpm: result.bpm ?? null,
    timelineBpm: result.bpm ? result.bpm * item.speed : null,
    confidence: result.confidence ?? 0,
    beatFrames: result.bpm ? Math.round((60 / result.bpm) * fps / item.speed * 100) / 100 : null,
    beatCount: beats.length,
    downbeatCount: downbeats.length,
    beats: beats.slice(0, 400),
    downbeats: downbeats.slice(0, 120),
    ...(beats.length > 400 || downbeats.length > 120 || drops.length > 120 ? {truncated:'Pass a smaller timeline range for the remaining beats, downbeats and drops.'}:{}),
    hits,
    dropCount: drops.length,
    drops: drops.slice(0, 120),
    ...(snaps.length ? { snaps } : {}),
    note: "Timeline frames after trimming, speed and looping. The range applies to every returned event, including drops; snaps use the full range even when its event lists are truncated. Put cuts and state changes on beats, big moments on downbeats, and start a whoosh or hit sound 1 to 2 frames before the frame it hits. off is how far the beat is from your frame. drops lists each played occurrence of the source's kick/bass rise (cycle is its source pass); an even track has none. Repeated events at the same whole frame are listed once.",
  };
}
