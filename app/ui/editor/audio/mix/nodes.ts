// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Web Audio node builders for the mix: effect stage chains (D84), noise removal's worklet (D246)
// and the master limiter (D86).
import { type Stage, makeupCompensationDb, LIMITER_MARGIN_DB, DENOISE_FLOOR_DB } from "@/app/lib/editor/core/audio/effects";
import { DENOISE_FFT } from "@/app/lib/editor/core/audio/denoise";
const denoiseWorkletUrl = '/editor-workers/denoise.js';

/**
 * Noise removal's AudioWorklet module (D246), loaded once per context. Until it has loaded a
 * denoise stage passes the sound through (its shape says "denoise…" so the chain is rebuilt once it
 * is ready); a context where it can't load keeps passing through and says so in the console.
 */
const worklets = new WeakMap<BaseAudioContext, { ready: boolean; promise: Promise<boolean> }>();
export function loadDenoise(ctx: BaseAudioContext): Promise<boolean> {
  const known = worklets.get(ctx);
  if (known) return known.promise;
  const entry = { ready: false, promise: Promise.resolve(false) };
  entry.promise = ctx.audioWorklet
    .addModule(denoiseWorkletUrl)
    .then(() => (entry.ready = true))
    .catch((error: unknown) => {
      console.warn(`noise removal can't play in the preview: ${error instanceof Error ? error.message : String(error)}`);
      return false;
    });
  worklets.set(ctx, entry);
  return entry.promise;
}
export const denoiseReady = (ctx: BaseAudioContext) => worklets.get(ctx)?.ready === true;

/**
 * Denoise worklets made and not yet ended (D262): a node leaves when its processor answers a
 * dispose with { ended: true }, i.e. its process() has returned false and the render thread let it
 * go. The app tests read the count (__biliupAudio.denoiseStats) to check that toggling noise removal
 * leaves none behind.
 */
const liveDenoise = new Set<AudioWorkletNode>();
let madeDenoise = 0;
export function denoiseStats(): { readonly live: number; readonly made: number } {
  // A closed context's processors are gone with it.
  for (const w of liveDenoise) if (w.context.state === "closed") liveDenoise.delete(w);
  return { live: liveDenoise.size, made: madeDenoise };
}

/**
 * How long a track's stages delay its sound on `ctx` (s): DENOISE_FFT samples per noise removal whose
 * worklet runs there (a pending one passes through), 0 without. The mix starts that track's sound
 * this much early so it lands with the picture and the other tracks (D262).
 */
export const stagesLatency = (stages: ReadonlyArray<Stage>, ctx: BaseAudioContext | undefined) =>
  ctx && denoiseReady(ctx) ? (stages.filter((s) => s.kind === "denoise").length * DENOISE_FFT) / ctx.sampleRate : 0;

/** dB → linear (no cutoff: for node parameters). */
const lin = (db: number) => 10 ** (db / 20);

/** The nodes of a chain of effect stages, and how to set their parameters without rebuilding. */
export interface StageChain {
  /** Kinds and types in order: a chain with the same shape is updated in place. */
  readonly shape: string;
  readonly first: AudioNode | undefined;
  readonly last: AudioNode | undefined;
  readonly nodes: ReadonlyArray<AudioNode>;
  /** Set every parameter from `stages` (same shape), gliding over `ramp` seconds (0: at once). */
  readonly set: (stages: ReadonlyArray<Stage>, ramp: number) => void;
  /** Compressors' gain reduction now (dB, ≤ 0), the deepest. */
  readonly reduction: () => number;
  /** Clear noise removal's buffers (a seek or a play start: no stale tail at the new position). */
  readonly reset: () => void;
  /** Disconnect every node and end noise removal's processors (D262): call once, when the chain is thrown away. */
  readonly dispose: () => void;
}
/** A chain's shape on `ctx`: a denoise stage reads "denoise…" until its worklet has loaded there. */
export const stageShape = (stages: ReadonlyArray<Stage>, ctx?: BaseAudioContext) =>
  stages.map((s) => (s.kind === "biquad" ? s.type : s.kind === "denoise" && !(ctx && denoiseReady(ctx)) ? "denoise…" : s.kind)).join(",");

/**
 * Web Audio nodes for effect stages (D84), in order, connected one to the next. Noise removal is an
 * AudioWorkletNode running core's SpectralDenoiser (D246) once its module has loaded (loadDenoise);
 * before that it passes through. Works on any context (the live mix, and the app tests'
 * OfflineAudioContext checks).
 */
export function buildStages(ctx: BaseAudioContext, stages: ReadonlyArray<Stage>): StageChain {
  type Part = { readonly stage: Stage["kind"]; readonly nodes: ReadonlyArray<AudioNode>; readonly set: (s: Stage, at: (p: AudioParam, v: number) => void) => void };
  const parts: Array<Part> = [];
  const denoisers: Array<AudioWorkletNode> = [];
  const denoise = denoiseReady(ctx);
  // Parts line up with the stages: a pending denoise stage has no nodes (and is skipped when wiring).
  for (const stage of stages) {
    if (stage.kind === "biquad") {
      const f = ctx.createBiquadFilter();
      f.type = stage.type;
      parts.push({
        stage: "biquad",
        nodes: [f],
        set: (s, at) => {
          if (s.kind !== "biquad") return;
          at(f.frequency, s.freq);
          // Web Audio: a high-/low-pass's Q is in dB (20·log10 of the cookbook's Q); a peak's is linear; shelves ignore it (slope 1).
          at(f.Q, s.type === "highpass" || s.type === "lowpass" ? 20 * Math.log10(s.q) : s.q);
          at(f.gain, s.gainDb);
        },
      });
    } else if (stage.kind === "compressor") {
      const c = ctx.createDynamicsCompressor();
      const g = ctx.createGain();
      c.knee.value = 0;
      c.connect(g);
      parts.push({
        stage: "compressor",
        nodes: [c, g],
        set: (s, at) => {
          if (s.kind !== "compressor") return;
          at(c.threshold, s.thresholdDb);
          at(c.ratio, s.ratio);
          at(c.attack, s.attackMs / 1000);
          at(c.release, s.releaseMs / 1000);
          at(g.gain, lin(s.makeupDb - makeupCompensationDb(s.thresholdDb, s.ratio)));
        },
      });
    } else if (stage.kind === "denoise") {
      if (!denoise) {
        parts.push({ stage: "denoise", nodes: [], set: () => {} });
        continue;
      }
      const w = new AudioWorkletNode(ctx, "biliup-denoise", { numberOfInputs: 1, numberOfOutputs: 1, outputChannelCount: [2], channelCount: 2, channelCountMode: "explicit", channelInterpretation: "speakers" });
      denoisers.push(w);
      // Counted on a live context only (an offline render ends with its context).
      if (typeof AudioContext !== "undefined" && ctx instanceof AudioContext) {
        liveDenoise.add(w);
        madeDenoise++;
      }
      w.port.onmessage = (e: MessageEvent<{ ended?: boolean }>) => {
        if (e.data?.ended) {
          liveDenoise.delete(w);
          w.port.onmessage = null;
        }
      };
      let last = "";
      parts.push({
        stage: "denoise",
        nodes: [w],
        set: (s) => {
          if (s.kind !== "denoise") return;
          const msg = { amount: s.amount, floorDb: s.floorDb ?? DENOISE_FLOOR_DB };
          const key = `${msg.amount},${msg.floorDb}`;
          if (key === last) return;
          last = key;
          w.port.postMessage(msg);
        },
      });
    }
  }
  const wired = parts.filter((p) => p.nodes.length);
  for (let i = 1; i < wired.length; i++) wired[i - 1]!.nodes.at(-1)!.connect(wired[i]!.nodes[0]!);
  const apply = (list: ReadonlyArray<Stage>, ramp: number) => {
    const shown = list;
    const now = ctx.currentTime;
    const at = (p: AudioParam, v: number) => {
      if (ramp <= 0) {
        p.cancelScheduledValues(now);
        p.value = v;
        return;
      }
      p.cancelScheduledValues(now);
      p.setValueAtTime(p.value, now);
      p.linearRampToValueAtTime(v, now + ramp);
    };
    shown.forEach((s, i) => parts[i]?.set(s, at));
  };
  apply(stages, 0);
  const compressors = parts.filter((p) => p.stage === "compressor").map((p) => p.nodes[0] as DynamicsCompressorNode);
  const nodes = parts.flatMap((p) => p.nodes);
  let disposed = false;
  return {
    shape: stageShape(stages, ctx),
    first: wired[0]?.nodes[0],
    last: wired.at(-1)?.nodes.at(-1),
    nodes,
    set: apply,
    reduction: () => compressors.reduce((m, c) => Math.min(m, c.reduction), 0),
    reset: () => {
      if (!disposed) for (const w of denoisers) w.port.postMessage({ reset: true });
    },
    dispose: () => {
      if (disposed) return;
      disposed = true;
      for (const w of denoisers) w.port.postMessage({ dispose: true });
      for (const n of nodes) n.disconnect();
    },
  };
}

/** The limiter's release (s) and attack (s): close to the export's alimiter (50 ms; its 5 ms lookahead is an attack here). */
const LIMITER_RELEASE_S = 0.05;
const LIMITER_ATTACK_S = 0.001;
/** A hard clip at ±ceiling (linear), as a WaveShaper curve: identity inside, flat outside (input past ±1 maps to the ends). */
function clipCurve(ceiling: number): Float32Array<ArrayBuffer> {
  const n = 4097;
  const curve = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const x = (i / (n - 1)) * 2 - 1;
    curve[i] = Math.max(-ceiling, Math.min(ceiling, x));
  }
  return curve;
}

/** The master limiter's nodes (D86): compressor → compensation gain → clip. */
export interface LimiterNodes {
  readonly input: DynamicsCompressorNode;
  readonly gain: GainNode;
  readonly clip: WaveShaperNode;
  ceilingDb: number;
}
export function buildLimiter(ctx: BaseAudioContext, ceilingDb: number): LimiterNodes {
  const input = ctx.createDynamicsCompressor();
  input.knee.value = 0;
  input.ratio.value = 20;
  input.attack.value = LIMITER_ATTACK_S;
  input.release.value = LIMITER_RELEASE_S;
  const gain = ctx.createGain();
  const clip = ctx.createWaveShaper();
  clip.oversample = "none";
  input.connect(gain);
  gain.connect(clip);
  const nodes: LimiterNodes = { input, gain, clip, ceilingDb };
  setLimiter(nodes, ceilingDb);
  return nodes;
}
/** The compressor works LIMITER_MARGIN_DB under the ceiling, as the export's alimiter does; the clip is at the ceiling itself. */
export function setLimiter(l: LimiterNodes, ceilingDb: number) {
  l.ceilingDb = ceilingDb;
  const threshold = ceilingDb - LIMITER_MARGIN_DB;
  l.input.threshold.value = threshold;
  l.gain.gain.value = lin(-makeupCompensationDb(threshold, 20));
  l.clip.curve = clipCurve(lin(ceilingDb));
}
