// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { LIMITER_MARGIN_DB } from "@/app/lib/editor/core";
import type { ExportPlan } from "./types.ts";
import { type AudioOptions, audioCommand } from "./audio.ts";

/**
 * A measuring pass (audio only, nothing written): loudnorm's JSON for `target` at the end of the
 * chain. Without `options` it measures the mix as it goes into the limiter (the first pass, D87);
 * with a gain it measures what the limiter puts out (the check when the limiter will work).
 */
export function loudnessCommand(plan: ExportPlan, target: number, options: AudioOptions = { noLimiter: true }) {
  // ebur128 (integrated loudness and true peak, D87) and loudnorm's JSON (the bypassed limiter's two-pass loudnorm) in one pass.
  return audioCommand(plan, { ...options, tail: `ebur128=peak=true:framelog=quiet,loudnorm=I=${target}:TP=-1:LRA=11:print_format=json` }, ["-map", "[aout]", "-f", "null", "-"]);
}

/** ebur128's summary from a measuring pass: integrated loudness (LUFS) and true peak (dBTP); undefined for silence or none. */
export function parseEbur128(stderr: string): { readonly lufs: number; readonly tp: number } | undefined {
  const at = stderr.lastIndexOf("Summary:");
  if (at < 0) return undefined;
  const tail = stderr.slice(at);
  const i = /I:\s+(-?[\d.]+|-inf) LUFS/.exec(tail);
  const tp = /Peak:\s+(-?[\d.]+|-inf) dBFS/.exec(tail);
  const lufs = i && i[1] !== "-inf" ? Number(i[1]) : -Infinity;
  if (!(lufs > -70)) return undefined;
  return { lufs, tp: tp && tp[1] !== "-inf" ? Number(tp[1]) : -Infinity };
}

/** At most this many measuring passes through the limiter (D87): the gain is corrected (bracketed) until the target holds within LOUDNESS_TOLERANCE_LU. */
export const LOUDNESS_CHECKS = 6;
export const LOUDNESS_TOLERANCE_LU = 0.2;
/** The loudness gain is kept within ±this (dB). */
const MAX_LOUDNESS_GAIN_DB = 40;

export interface LoudnessMeasure {
  readonly input_i: string;
  readonly input_tp: string;
  readonly input_lra: string;
  readonly input_thresh: string;
  readonly target_offset: string;
}

/** loudnorm's JSON from the first pass's stderr, or undefined (e.g. silence: nothing to normalise). */
export function parseLoudness(stderr: string): LoudnessMeasure | undefined {
  const m = /\{[^{}]*"input_i"[^{}]*\}/s.exec(stderr);
  if (!m) return undefined;
  try {
    const j = JSON.parse(m[0]) as LoudnessMeasure;
    return Number.isFinite(Number(j.input_i)) && Number(j.input_i) > -70 ? j : undefined;
  } catch {
    return undefined;
  }
}

/** The second pass's loudnorm filter, linear (one gain for the whole mix) when the measure allows it. */
export const loudnormFilter = (target: number, m: LoudnessMeasure) =>
  `loudnorm=I=${target}:TP=-1:LRA=11:measured_I=${m.input_i}:measured_TP=${m.input_tp}:measured_LRA=${m.input_lra}:measured_thresh=${m.input_thresh}:offset=${m.target_offset}:linear=true:print_format=none`;

/**
 * The loudness step of an export (D87, D86), given a way to run a measuring pass (its stderr):
 * with the limiter on, one gain to the target before it (ebur128's integrated loudness of the mix
 * going into the limiter). When the peaks will make the limiter work (true peak + gain over its
 * limit), the result through the limiter is measured and the gain corrected (bracketing the target,
 * at most LOUDNESS_CHECKS passes) until it lands within LOUDNESS_TOLERANCE_LU; the gain used is the
 * closest one measured. With the limiter bypassed:
 * two-pass loudnorm as before D86. Nothing to do for silence.
 */
export async function planLoudness(plan: ExportPlan, target: number, measure: (pass: ReturnType<typeof loudnessCommand>, name: string) => Promise<string>): Promise<AudioOptions> {
  if (!plan.audio.length) return {};
  const stderr = await measure(loudnessCommand(plan, target), "loudness.txt");
  const norm = parseLoudness(stderr);
  if (!plan.limiter) return norm ? { loudnorm: loudnormFilter(target, norm) } : {};
  const first = parseEbur128(stderr);
  if (!first) return {};
  const clampGain = (g: number) => Math.round(Math.min(MAX_LOUDNESS_GAIN_DB, Math.max(-MAX_LOUDNESS_GAIN_DB, g)) * 100) / 100;
  let gainDb = clampGain(target - first.lufs);
  if (first.tp + gainDb > plan.limiter.ceilingDb - LIMITER_MARGIN_DB) {
    // Every gain tried is measured through the limiter; the closest one measured is kept (never an
    // unmeasured step). Loudness rises with the gain, but slowly and unevenly while the limiter flattens
    // peaks, so the steps bracket the target: below and above, then interpolate inside the bracket
    // (bisect when interpolation would stall at an edge).
    let best: { gain: number; err: number } | undefined;
    let lo: { gain: number; lufs: number } | undefined;
    let hi: { gain: number; lufs: number } | undefined;
    let last: { gain: number; lufs: number } | undefined;
    for (let k = 0; k < LOUDNESS_CHECKS; k++) {
      const check = parseEbur128(await measure(loudnessCommand(plan, target, { gainDb }), `loudness-check-${k}.txt`));
      if (!check) break;
      const err = target - check.lufs;
      if (!best || Math.abs(err) < Math.abs(best.err)) best = { gain: gainDb, err };
      if (Math.abs(err) <= LOUDNESS_TOLERANCE_LU) break;
      const point = { gain: gainDb, lufs: check.lufs };
      if (err > 0 && (!lo || gainDb > lo.gain)) lo = point;
      if (err < 0 && (!hi || gainDb < hi.gain)) hi = point;
      let next: number;
      if (lo && hi) {
        next = lo.gain + ((target - lo.lufs) * (hi.gain - lo.gain)) / (hi.lufs - lo.lufs);
        const span = hi.gain - lo.gain;
        if (!(next > lo.gain + 0.1 * span && next < hi.gain - 0.1 * span)) next = (lo.gain + hi.gain) / 2;
      } else {
        // Not bracketed yet: step along the slope seen so far (at least the full error: the limiter only slows loudness down).
        const slope = last && gainDb !== last.gain ? Math.min(1, Math.max(0.1, (check.lufs - last.lufs) / (gainDb - last.gain))) : 1;
        next = gainDb + err / slope;
      }
      last = point;
      gainDb = clampGain(next);
    }
    if (best) gainDb = best.gain;
  }
  return { gainDb };
}

/**
 * The gain (dB) the export's loudness step would give the mix for `target` (D246: the preview plays
 * at the target with it): planLoudness's gain with the limiter on; with it bypassed, target − I of
 * the first pass (loudnorm's linear gain; where loudnorm would turn dynamic to hold its true peak
 * the preview is louder than the export: a known limit). Also the mix's own integrated loudness.
 */
export async function loudnessGain(plan: ExportPlan, target: number, measure: (pass: ReturnType<typeof loudnessCommand>, name: string) => Promise<string>): Promise<{ readonly gainDb?: number; readonly lufs?: number }> {
  let first: string | undefined;
  const once = async (pass: ReturnType<typeof loudnessCommand>, name: string) => {
    const stderr = await measure(pass, name);
    first ??= stderr;
    return stderr;
  };
  const result = await planLoudness(plan, target, once);
  const mix = first !== undefined ? parseEbur128(first) : undefined;
  const lufs = mix ? { lufs: mix.lufs } : {};
  if (result.gainDb !== undefined) return { gainDb: result.gainDb, ...lufs };
  if (result.loudnorm && mix) return { gainDb: Math.round(Math.min(MAX_LOUDNESS_GAIN_DB, Math.max(-MAX_LOUDNESS_GAIN_DB, target - mix.lufs)) * 100) / 100, ...lufs };
  return lufs;
}
