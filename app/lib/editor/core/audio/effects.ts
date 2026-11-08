// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Track effects and the master limiter (phase 6 chunk 7, D84–D88): what a track's mixer.effects and
// the project's master.effects mean, as a list of processing stages both the preview (Web Audio
// nodes, apps/web/src/audio/audio.ts) and the export (ffmpeg filters, exportPlan.ts) build from, so the
// two hear the same thing.
//
// - EQ (audio.eq): Butterworth high-pass / low-pass, low shelf (slope 1), peak (Q), high shelf
//   (slope 1); the curves of the Audio EQ Cookbook, which is what the Web Audio spec's
//   BiquadFilterNode and ffmpeg's highpass / lowpass / equalizer / lowshelf / highshelf compute
//   (biquadResponseDb is the one oracle the tests check both against). A band at 0 dB is no stage.
// - Compressor (audio.compressor): hard knee, peak detection, stereo-linked. Preview:
//   DynamicsCompressorNode (its automatic make-up gain cancelled, makeupCompensationDb); export:
//   acompressor. The static curve is the same; the detectors' timing differs (documented in D84).
// - Voice (audio.voice-enhance): one slot standing for high-pass 80 Hz + presence peak + gentle
//   compression (voiceStages); the Mixer's Voice preset adds the same as an EQ and a compressor.
// - Noise removal (audio.denoise): ffmpeg afftdn in the export (D88); the preview runs core's
//   SpectralDenoiser (denoise.ts) in an AudioWorklet, fitted to afftdn (D246). Both start from the
//   track's measured noise floor (trackNoiseFloor).
// - Ducking (audio.duck) is gain automation, not a stage: ducking.ts.
// - The limiter (audio.limiter) belongs to the master: masterLimiter. On a track it does nothing.
//
// Order on a track (D84): items (their gain: volume, keys, fades, the track's volume, ducking) →
// the track's stages in list order → pan → the sum of tracks → master volume → limiter.
import { soundMedia } from "@/app/lib/editor/core/audio/audio";
import { itemEnvelope } from './envelope';
import { resolveStarts } from '../timeline/rules';
import type { SpeechEnvelope } from "@/app/lib/editor/core/text/transcript";
import type { ModuleUse, Project, Track } from "@/app/lib/editor/core/timeline/types";

export const EQ = "audio.eq";
export const COMPRESSOR = "audio.compressor";
export const VOICE = "audio.voice-enhance";
export const DENOISE = "audio.denoise";
export const DUCK = "audio.duck";
export const LIMITER = "audio.limiter";

/** The effects the Mixer offers on a track, in its menu's order. */
export const TRACK_EFFECTS = [EQ, COMPRESSOR, VOICE, DENOISE] as const;
/** Effects heard in the preview as in the export (modules_list: showsInPreview): all of them since D246. */
export const HEARD_EFFECTS: ReadonlyArray<string> = [EQ, COMPRESSOR, VOICE, DENOISE, DUCK, LIMITER];

export const EFFECT_NAMES: Readonly<Record<string, string>> = {
  [EQ]: "EQ",
  [COMPRESSOR]: "Compressor",
  [VOICE]: "Voice",
  [DENOISE]: "Noise removal",
  [DUCK]: "Ducking",
  [LIMITER]: "Limiter",
};
export const effectName = (module: string) => EFFECT_NAMES[module] ?? module;

export type BiquadType = "highpass" | "lowpass" | "lowshelf" | "peaking" | "highshelf";
/** A Butterworth high-/low-pass: Q = 1/√2 (linear; Web Audio takes it in dB: 20·log10 Q ≈ −3.01). */
export const BUTTERWORTH_Q = Math.SQRT1_2;

export type Stage =
  | { readonly kind: "biquad"; readonly type: BiquadType; readonly freq: number; readonly q: number; readonly gainDb: number }
  | { readonly kind: "compressor"; readonly thresholdDb: number; readonly ratio: number; readonly attackMs: number; readonly releaseMs: number; readonly makeupDb: number }
  /** floorDb: the noise floor afftdn starts from (dBFS RMS), from the track's played sound; absent = DENOISE_FLOOR_DB. */
  | { readonly kind: "denoise"; readonly amount: number; readonly floorDb?: number };

export const EQ_DEFAULTS = { lowGainDb: 0, lowHz: 120, midGainDb: 0, midHz: 1000, midQ: 1, highGainDb: 0, highHz: 8000 } as const;
export const COMPRESSOR_DEFAULTS = { thresholdDb: -18, ratio: 3, attackMs: 10, releaseMs: 150, makeupDb: 0 } as const;
export const DENOISE_DEFAULTS = { amount: 0.5 } as const;
export const VOICE_DEFAULTS = { amount: 0.5 } as const;

/** The Mixer's Voice preset (D84): an EQ and a compressor equal to Voice at its default amount. */
export const VOICE_PRESET: ReadonlyArray<ModuleUse> = [
  { module: EQ, params: { lowCutHz: 80, midGainDb: 4, midHz: 3500, midQ: 0.9 } },
  { module: COMPRESSOR, params: { thresholdDb: -20, ratio: 2.5, attackMs: 10, releaseMs: 150, makeupDb: 2 } },
];

const num = (v: unknown, fallback: number, lo: number, hi: number) => (typeof v === "number" && Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : fallback);
const opt = (v: unknown, lo: number, hi: number) => (typeof v === "number" && Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : undefined);

/** An EQ's params with defaults filled in (and clamped to the manifest's ranges). */
export function eqParams(params: ModuleUse["params"]) {
  const p = params ?? {};
  return {
    lowCutHz: opt(p.lowCutHz, 20, 1000),
    highCutHz: opt(p.highCutHz, 1000, 20000),
    lowGainDb: num(p.lowGainDb, EQ_DEFAULTS.lowGainDb, -24, 24),
    lowHz: num(p.lowHz, EQ_DEFAULTS.lowHz, 20, 1000),
    midGainDb: num(p.midGainDb, EQ_DEFAULTS.midGainDb, -24, 24),
    midHz: num(p.midHz, EQ_DEFAULTS.midHz, 100, 10000),
    midQ: num(p.midQ, EQ_DEFAULTS.midQ, 0.1, 10),
    highGainDb: num(p.highGainDb, EQ_DEFAULTS.highGainDb, -24, 24),
    highHz: num(p.highHz, EQ_DEFAULTS.highHz, 1000, 16000),
  };
}
export function compressorParams(params: ModuleUse["params"]) {
  const p = params ?? {};
  return {
    thresholdDb: num(p.thresholdDb, COMPRESSOR_DEFAULTS.thresholdDb, -60, 0),
    ratio: num(p.ratio, COMPRESSOR_DEFAULTS.ratio, 1, 20),
    attackMs: num(p.attackMs, COMPRESSOR_DEFAULTS.attackMs, 1, 500),
    releaseMs: num(p.releaseMs, COMPRESSOR_DEFAULTS.releaseMs, 10, 1000),
    makeupDb: num(p.makeupDb, COMPRESSOR_DEFAULTS.makeupDb, 0, 24),
  };
}
export const amountOf = (params: ModuleUse["params"], fallback = 0.5) => num(params?.amount, fallback, 0, 1);

function eqStages(params: ModuleUse["params"]): Array<Stage> {
  const p = eqParams(params);
  const out: Array<Stage> = [];
  if (p.lowCutHz !== undefined) out.push({ kind: "biquad", type: "highpass", freq: p.lowCutHz, q: BUTTERWORTH_Q, gainDb: 0 });
  if (p.lowGainDb !== 0) out.push({ kind: "biquad", type: "lowshelf", freq: p.lowHz, q: 1, gainDb: p.lowGainDb });
  if (p.midGainDb !== 0) out.push({ kind: "biquad", type: "peaking", freq: p.midHz, q: p.midQ, gainDb: p.midGainDb });
  if (p.highGainDb !== 0) out.push({ kind: "biquad", type: "highshelf", freq: p.highHz, q: 1, gainDb: p.highGainDb });
  if (p.highCutHz !== undefined) out.push({ kind: "biquad", type: "lowpass", freq: p.highCutHz, q: BUTTERWORTH_Q, gainDb: 0 });
  return out;
}

/** Voice at `amount` 0…1: high-pass 80 Hz, a presence peak (+2…+6 dB at 3.5 kHz), gentle compression (1.5…3.5 : 1). */
export function voiceStages(amount: number): Array<Stage> {
  const a = Math.min(1, Math.max(0, amount));
  const r = (x: number) => Math.round(x * 1000) / 1000;
  return [
    { kind: "biquad", type: "highpass", freq: 80, q: BUTTERWORTH_Q, gainDb: 0 },
    { kind: "biquad", type: "peaking", freq: 3500, q: 0.9, gainDb: r(2 + 4 * a) },
    { kind: "compressor", thresholdDb: -20, ratio: r(1.5 + 2 * a), attackMs: 10, releaseMs: 150, makeupDb: r(1 + 2 * a) },
  ];
}

/** The stages one effect entry stands for (none when bypassed, for ducking, a limiter or an unknown module). */
export function effectStages(fx: ModuleUse): Array<Stage> {
  if (fx.bypass) return [];
  switch (fx.module) {
    case EQ:
      return eqStages(fx.params);
    case COMPRESSOR: {
      const c = compressorParams(fx.params);
      return c.ratio === 1 && c.makeupDb === 0 ? [] : [{ kind: "compressor", ...c }];
    }
    case VOICE:
      return voiceStages(amountOf(fx.params, VOICE_DEFAULTS.amount));
    case DENOISE:
      return [{ kind: "denoise", amount: amountOf(fx.params, DENOISE_DEFAULTS.amount) }];
    default:
      return [];
  }
}

/**
 * A track's stages in list order. With `envelopes` (media id → loudness envelope), noise removal
 * gets the track's measured noise floor (trackNoiseFloor): the preview and the export call this
 * the same way, so both start from the same floor (D88, D246).
 */
export function trackStages(track: Track, project?: Pick<Project, 'tracks' | 'media' | 'canvas'>, envelopes?: ReadonlyMap<string, SpeechEnvelope | null>): Array<Stage> {
  const out: Array<Stage> = [];
  let floor: number | undefined;
  for (const fx of track.mixer?.effects ?? [])
    for (const s of effectStages(fx)) {
      if (s.kind === "denoise" && project && envelopes) out.push({ ...s, floorDb: (floor ??= trackNoiseFloor(project, track, envelopes).db) });
      else out.push(s);
    }
  return out;
}

/** The noise floor noise removal assumes when the track's sound hasn't been measured (dBFS RMS). */
export const DENOISE_FLOOR_DB = -40;
/**
 * A played item's noise floor (dBFS RMS): the 10th percentile of its audible envelope windows,
 * after item/track gain, fades and transition handles, plus 6 dB, within −80…−20.
 * The envelope is a 16 kHz mono decode, so
 * broadband noise reads about 5 dB under its full-band level, and afftdn removes noise up to about
 * its floor setting.
 */
type NoiseFloor = { readonly db: number; readonly measured: boolean };
type NoiseProject = Pick<Project, 'tracks' | 'media' | 'canvas'>;
const noiseFloors = new WeakMap<Track, { tracks: Project['tracks']; media: Project['media']; fps: number; envelopes: ReadonlyMap<string, SpeechEnvelope | null>; value: NoiseFloor }>();
/** The quietest measured item in the track; gaps and muted/held portions are not noise samples. */
export function trackNoiseFloor(project: NoiseProject, track: Track, envelopes: ReadonlyMap<string, SpeechEnvelope | null>): NoiseFloor {
  const known = noiseFloors.get(track);
  if (known && known.tracks === project.tracks && known.media === project.media && known.fps === project.canvas.fps && known.envelopes === envelopes) return known.value;
  const starts = resolveStarts(project);
  let floor: number | undefined;
  track.items.forEach((item, index) => {
    const env = item.media && soundMedia(project, item) ? envelopes.get(item.media) : undefined;
    if (!env?.db.length) return;
    // The analysis has half-dB precision. Fixed bins avoid sorting hours of repeated loop samples.
    const bins = new Uint32Array(481);
    let count = 0;
    for (const sample of itemEnvelope(project, track, index, starts.get(item.id) ?? 0, env)) {
      const db = sample.sourceDb + 20 * Math.log10(sample.gain);
      const bin = Math.max(0, Math.min(bins.length - 1, Math.round((db + 160) * 2)));
      bins[bin] = bins[bin]! + 1;
      count++;
    }
    if (!count) return;
    const target = Math.floor(count * 0.1);
    let sum = 0;
    for (let bin = 0; bin < bins.length; bin++) {
      sum += bins[bin]!;
      if (sum <= target) continue;
      const db = Math.min(-20, Math.max(-80, bin / 2 - 160 + 6));
      floor = floor === undefined ? db : Math.min(floor, db);
      break;
    }
  });
  const value = { db: floor ?? DENOISE_FLOOR_DB, measured: floor !== undefined };
  noiseFloors.set(track, { tracks: project.tracks, media: project.media, fps: project.canvas.fps, envelopes, value });
  return value;
}
/** Media on tracks with noise removal on (their envelopes give the noise floor, D88). */
export function denoiseMediaIds(project: Pick<Project, "tracks" | "media">): Array<string> {
  const ids = new Set<string>();
  for (const t of project.tracks) if ((t.mixer?.effects ?? []).some((fx) => fx.module === DENOISE && !fx.bypass)) for (const i of t.items) if (soundMedia(project, i) && i.media) ids.add(i.media);
  return [...ids].sort();
}

// ---- biquads: the Audio EQ Cookbook, as the Web Audio spec states it ----

export interface Biquad {
  readonly b0: number;
  readonly b1: number;
  readonly b2: number;
  readonly a1: number;
  readonly a2: number;
}

/** A biquad stage's coefficients (normalised by a0) at `sampleRate`. Q is linear for every type here. */
export function biquadCoefficients(stage: Extract<Stage, { kind: "biquad" }>, sampleRate: number): Biquad {
  const w0 = (2 * Math.PI * Math.min(stage.freq, sampleRate / 2 - 1)) / sampleRate;
  const cos = Math.cos(w0);
  const sin = Math.sin(w0);
  const A = 10 ** (stage.gainDb / 40);
  let b0: number, b1: number, b2: number, a0: number, a1: number, a2: number;
  switch (stage.type) {
    case "highpass":
    case "lowpass": {
      const alpha = sin / (2 * stage.q);
      const k = stage.type === "lowpass" ? 1 - cos : 1 + cos;
      b0 = k / 2;
      b1 = stage.type === "lowpass" ? k : -k;
      b2 = k / 2;
      a0 = 1 + alpha;
      a1 = -2 * cos;
      a2 = 1 - alpha;
      break;
    }
    case "peaking": {
      const alpha = sin / (2 * stage.q);
      b0 = 1 + alpha * A;
      b1 = -2 * cos;
      b2 = 1 - alpha * A;
      a0 = 1 + alpha / A;
      a1 = -2 * cos;
      a2 = 1 - alpha / A;
      break;
    }
    case "lowshelf":
    case "highshelf": {
      // Slope S = 1: alpha = sin(w0)/2 · √((A + 1/A)(1/S − 1) + 2) = sin(w0)/√2.
      const s = 2 * Math.sqrt(A) * (sin / Math.SQRT2);
      if (stage.type === "lowshelf") {
        b0 = A * (A + 1 - (A - 1) * cos + s);
        b1 = 2 * A * (A - 1 - (A + 1) * cos);
        b2 = A * (A + 1 - (A - 1) * cos - s);
        a0 = A + 1 + (A - 1) * cos + s;
        a1 = -2 * (A - 1 + (A + 1) * cos);
        a2 = A + 1 + (A - 1) * cos - s;
      } else {
        b0 = A * (A + 1 + (A - 1) * cos + s);
        b1 = -2 * A * (A - 1 + (A + 1) * cos);
        b2 = A * (A + 1 + (A - 1) * cos - s);
        a0 = A + 1 - (A - 1) * cos + s;
        a1 = 2 * (A - 1 - (A + 1) * cos);
        a2 = A + 1 - (A - 1) * cos - s;
      }
      break;
    }
  }
  return { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 };
}

/** The magnitude response (dB) of a biquad stage at `freq` Hz. */
export function biquadResponseDb(stage: Extract<Stage, { kind: "biquad" }>, sampleRate: number, freq: number): number {
  const { b0, b1, b2, a1, a2 } = biquadCoefficients(stage, sampleRate);
  const w = (2 * Math.PI * freq) / sampleRate;
  const c1 = Math.cos(w);
  const s1 = Math.sin(w);
  const c2 = Math.cos(2 * w);
  const s2 = Math.sin(2 * w);
  const nr = b0 + b1 * c1 + b2 * c2;
  const ni = -(b1 * s1 + b2 * s2);
  const dr = 1 + a1 * c1 + a2 * c2;
  const di = -(a1 * s1 + a2 * s2);
  return 10 * Math.log10((nr * nr + ni * ni) / (dr * dr + di * di));
}

/** The whole chain's EQ response (dB) at `freq`: the biquad stages summed (compressors and noise removal left out). */
export const stagesResponseDb = (stages: ReadonlyArray<Stage>, sampleRate: number, freq: number) =>
  stages.reduce((sum, s) => (s.kind === "biquad" ? sum + biquadResponseDb(s, sampleRate, freq) : sum), 0);

// ---- the compressor ----

/** The compressor's static curve: output level (dB) for a steady input level (dB), with make-up. Hard knee. */
export function compressorCurveDb(stage: Extract<Stage, { kind: "compressor" }>, inputDb: number): number {
  const over = inputDb - stage.thresholdDb;
  return (over > 0 ? stage.thresholdDb + over / stage.ratio : inputDb) + stage.makeupDb;
}

/**
 * The make-up gain Chromium's DynamicsCompressorNode adds by itself (it can't be turned off), in dB,
 * for a hard knee: (1 / fullRangeGain)^0.6 where fullRangeGain is the curve's output at 0 dBFS,
 * i.e. −0.6 · threshold · (1 − 1/ratio). The preview cancels it with a gain after the node (checked
 * in the app test against a tone below the threshold).
 */
export const makeupCompensationDb = (thresholdDb: number, ratio: number) => -0.6 * thresholdDb * (1 - 1 / ratio);

// ---- the master limiter (D86) ----

export const LIMITER_CEILING_DB = -1;
/**
 * The limiter works this far (dB) under its ceiling, in the export and the preview alike (D86): the
 * export's 4× oversampled alimiter and the AAC encode overshoot a little, so a ceiling of −1 dBTP
 * limits sample peaks at −2 dBFS (measured: a +12 dB hot mix lands at −1.0 to −1.3 dBTP on the AAC).
 */
export const LIMITER_MARGIN_DB = 1;
/**
 * The master limiter: on by default at −1 dBTP (no entry needed), changed or bypassed by an
 * audio.limiter entry in master.effects. Undefined when bypassed.
 */
export function masterLimiter(project: Pick<Project, "master">): { readonly ceilingDb: number } | undefined {
  const fx = project.master?.effects?.find((e) => e.module === LIMITER);
  if (!fx) return { ceilingDb: LIMITER_CEILING_DB };
  if (fx.bypass) return undefined;
  return { ceilingDb: num(fx.params?.ceilingDb, LIMITER_CEILING_DB, -12, 0) };
}

// ---- loudness targets (D87) ----

/** The export's loudness choices: a preset target, the project's own target, or the mix as it is. */
export type LoudnessChoice = "off" | "normal" | "quiet" | "broadcast" | "target";
export const LOUDNESS_PRESETS: ReadonlyArray<{ readonly id: Exclude<LoudnessChoice, "off" | "target">; readonly lufs: number; readonly name: string; readonly hint: string }> = [
  { id: "normal", lufs: -14, name: "Normal", hint: "YouTube, Instagram, TikTok" },
  { id: "quiet", lufs: -16, name: "Quieter", hint: "Podcasts, Apple" },
  { id: "broadcast", lufs: -23, name: "Broadcast", hint: "TV (EBU R128)" },
];
export const DEFAULT_TARGET_LUFS = -14;
export const TARGET_LUFS_RANGE = { min: -30, max: -5 } as const;
/** The project's loudness target (master.loudness.targetLufs), −14 LUFS when unset. */
export const projectTargetLufs = (project: Pick<Project, "master">) => project.master?.loudness?.targetLufs ?? DEFAULT_TARGET_LUFS;
/** The export choice a target stands for: its preset, or "target" for any other number. */
export const choiceOfTarget = (lufs: number): Exclude<LoudnessChoice, "off"> => LOUDNESS_PRESETS.find((p) => p.lufs === lufs)?.id ?? "target";
/** The LUFS an export choice normalises to (undefined: As mixed). "target" uses `targetLufs` (else −14). */
export function targetOfChoice(choice: LoudnessChoice, targetLufs?: number): number | undefined {
  if (choice === "off") return undefined;
  if (choice === "target") return num(targetLufs, DEFAULT_TARGET_LUFS, TARGET_LUFS_RANGE.min, TARGET_LUFS_RANGE.max);
  return LOUDNESS_PRESETS.find((p) => p.id === choice)!.lufs;
}
/** A target in words: "Normal (−14 LUFS)", "−18 LUFS". */
export const targetWords = (lufs: number) => {
  const preset = LOUDNESS_PRESETS.find((p) => p.lufs === lufs);
  const n = `${lufs < 0 ? "−" : ""}${Math.abs(Math.round(lufs * 10) / 10)} LUFS`;
  return preset ? `${preset.name} (${n})` : n;
};

// ---- words ----

const dbw = (db: number) => `${db > 0 ? "+" : db < 0 ? "−" : ""}${Math.abs(Math.round(db * 10) / 10)} dB`;
const hz = (f: number) => (f >= 1000 ? `${Math.round(f / 100) / 10} kHz` : `${Math.round(f)} Hz`);
/** A short line about an effect's settings, for the Mixer's slots and History. */
export function effectSummary(fx: ModuleUse, trackName?: (id: string) => string): string {
  switch (fx.module) {
    case EQ: {
      const p = eqParams(fx.params);
      const parts = [
        p.lowCutHz !== undefined ? `low cut ${hz(p.lowCutHz)}` : "",
        p.lowGainDb ? `low ${dbw(p.lowGainDb)}` : "",
        p.midGainDb ? `${dbw(p.midGainDb)} at ${hz(p.midHz)}` : "",
        p.highGainDb ? `high ${dbw(p.highGainDb)}` : "",
        p.highCutHz !== undefined ? `high cut ${hz(p.highCutHz)}` : "",
      ].filter(Boolean);
      return parts.length ? parts.join(", ") : "flat";
    }
    case COMPRESSOR: {
      const c = compressorParams(fx.params);
      return `${dbw(c.thresholdDb)}, ${Math.round(c.ratio * 10) / 10}:1${c.makeupDb ? `, make-up ${dbw(c.makeupDb)}` : ""}`;
    }
    case VOICE:
    case DENOISE:
      return `${Math.round(amountOf(fx.params) * 100)} %`;
    case DUCK: {
      const under = Array.isArray(fx.params?.under) ? (fx.params.under as ReadonlyArray<unknown>).filter((u): u is string => typeof u === "string") : [];
      return `under ${under.map((u) => trackName?.(u) ?? u).join(", ") || "?"}, ${dbw(num(fx.params?.amountDb, -12, -40, 0))}`;
    }
    case LIMITER:
      return `ceiling ${dbw(num(fx.params?.ceilingDb, LIMITER_CEILING_DB, -12, 0))}TP`;
    default:
      return "";
  }
}
