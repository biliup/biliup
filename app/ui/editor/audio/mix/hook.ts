// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { Stage } from "@/app/lib/editor/core/audio/effects";
import { buildStages, buildLimiter, loadDenoise, denoiseStats } from "@/app/ui/editor/audio/mix/nodes";
import { type CaptureResult, SILENT } from "@/app/ui/editor/audio/mix/levels";
import type { Mix } from "@/app/ui/editor/audio/audio";

// A read-only hook for the app tests (and for poking at the mix in DevTools): the live mix's
// level and gains. It changes nothing.
let current: Mix | undefined;
export const setCurrentMix = (mix: Mix | undefined) => {
  current = mix;
};
export const currentMix = () => current;
let capturing: { stop: () => CaptureResult } | undefined;
const b64 = (a: Float32Array) => {
  const bytes = new Uint8Array(a.buffer, a.byteOffset, a.byteLength);
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
};
(globalThis as { __biliupAudio?: unknown }).__biliupAudio = {
  level: () => current?.level() ?? { rms: 0, peak: 0 },
  levels: () => current?.levels() ?? { master: SILENT, tracks: {}, limiter: 0, reduction: {} },
  state: () => current?.state() ?? { state: "none", items: [] },
  // Test-only (D262): noise removal's processors made on the live context and still running (each chain
  // thrown away must end its own: toggling noise removal ten times leaves only the current one).
  denoiseStats: () => denoiseStats(),
  // Test-only (D83): record the master output; stop returns the samples (float32, base64 per
  // channel), the context time of the first one, and the frame drawn at each context time.
  startCapture: () => {
    capturing = current?.capture();
    return capturing !== undefined;
  },
  // Test-only (D84, D86): the preview's own node builders on an OfflineAudioContext. biquadResponse:
  // a biquad stage's magnitude response (dB) at `freqs` from BiquadFilterNode.getFrequencyResponse;
  // offlineTone: a sine of amplitude `amp` through `stages` (and the limiter at `ceilingDb`), the
  // output's RMS and peak over its second half.
  biquadResponse: (stage: Stage, freqs: ReadonlyArray<number>, sampleRate = 48000) => {
    const ctx = new OfflineAudioContext(1, 128, sampleRate);
    const node = buildStages(ctx, [stage]).first as BiquadFilterNode;
    const f = Float32Array.from(freqs);
    const mag = new Float32Array(f.length);
    node.getFrequencyResponse(f, mag, new Float32Array(f.length));
    return [...mag].map((m) => 20 * Math.log10(m));
  },
  offlineTone: async ({ stages = [], ceilingDb, amp, freq = 1000, seconds = 2, sampleRate = 48000 }: { stages?: ReadonlyArray<Stage>; ceilingDb?: number; amp: number; freq?: number; seconds?: number; sampleRate?: number }) => {
    const ctx = new OfflineAudioContext(2, Math.round(seconds * sampleRate), sampleRate);
    await loadDenoise(ctx);
    const osc = ctx.createOscillator();
    osc.frequency.value = freq;
    const g = ctx.createGain();
    g.gain.value = amp;
    osc.connect(g);
    let out: AudioNode = g;
    const chain = buildStages(ctx, stages);
    if (chain.first && chain.last) {
      out.connect(chain.first);
      out = chain.last;
    }
    if (ceilingDb !== undefined) {
      const l = buildLimiter(ctx, ceilingDb);
      out.connect(l.input);
      out = l.clip;
    }
    out.connect(ctx.destination);
    osc.start();
    const buf = await ctx.startRendering();
    const data = buf.getChannelData(0);
    let sum = 0;
    let peak = 0;
    const from = Math.floor(data.length / 2);
    for (let i = from; i < data.length; i++) {
      sum += data[i]! ** 2;
      peak = Math.max(peak, Math.abs(data[i]!));
    }
    return { rms: Math.sqrt(sum / (data.length - from)), peak };
  },
  // Test-only (D246): a mono signal (float32, base64) through `stages` on an OfflineAudioContext
  // (noise removal's worklet included), returned the same way (left channel).
  offlineRender: async ({ stages, input, sampleRate = 48000 }: { stages: ReadonlyArray<Stage>; input: string; sampleRate?: number }) => {
    const bytes = Uint8Array.from(atob(input), (c) => c.charCodeAt(0));
    const x = new Float32Array(bytes.buffer);
    const ctx = new OfflineAudioContext(2, x.length, sampleRate);
    const ready = await loadDenoise(ctx);
    const buf = ctx.createBuffer(1, x.length, sampleRate);
    buf.copyToChannel(x, 0);
    const src = ctx.createBufferSource();
    src.buffer = buf;
    const chain = buildStages(ctx, stages);
    if (chain.first && chain.last) {
      src.connect(chain.first);
      chain.last.connect(ctx.destination);
    } else src.connect(ctx.destination);
    src.start();
    const out = await ctx.startRendering();
    return { ready, output: b64(out.getChannelData(0)) };
  },
  stopCapture: () => {
    const r = capturing?.stop();
    capturing = undefined;
    if (!r || !r.blocks.length) return undefined;
    const n = r.blocks.reduce((s, b) => s + b.left.length, 0);
    const left = new Float32Array(n);
    const right = new Float32Array(n);
    let o = 0;
    for (const b of r.blocks) {
      left.set(b.left, o);
      right.set(b.right, o);
      o += b.left.length;
    }
    return { sampleRate: r.sampleRate, t0: r.blocks[0]!.t, left: b64(left), right: b64(right), marks: r.marks, latency: r.latency, sink: r.sink };
  },
};
