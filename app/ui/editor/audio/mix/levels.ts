// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Meter levels (RMS and peak per channel) and the shapes the mix reports: its state and the test capture.
/** A channel's level over the analyser's window: RMS and peak, linear (1 = full scale). */
export interface Level {
  readonly rms: number;
  readonly peak: number;
}
/** Left and right levels of a strip. */
export interface StereoLevel {
  readonly left: Level;
  readonly right: Level;
}
export interface Levels {
  readonly master: StereoLevel;
  readonly tracks: Readonly<Record<string, StereoLevel>>;
  /** The master limiter's gain reduction now (dB, ≤ 0; 0 when off or idle), D86. */
  readonly limiter: number;
  /** Each bus's compressors' gain reduction now (dB, ≤ 0), D84. */
  readonly reduction: Readonly<Record<string, number>>;
}
export const SILENT: StereoLevel = { left: { rms: 0, peak: 0 }, right: { rms: 0, peak: 0 } };

/** What the test hook and the timeline can read about the mix. */
export interface MixState {
  readonly state: AudioContextState | "none";
  readonly items: ReadonlyArray<{ readonly id: string; readonly gain: number; readonly playing: boolean; readonly own: boolean }>;
}

export interface Capture {
  readonly node: ScriptProcessorNode;
  readonly sink: GainNode;
  readonly blocks: Array<{ t: number; left: Float32Array; right: Float32Array }>;
}
export interface CaptureResult {
  readonly sampleRate: number;
  readonly blocks: ReadonlyArray<{ readonly t: number; readonly left: Float32Array; readonly right: Float32Array }>;
  readonly marks: ReadonlyArray<{ readonly t: number; readonly frame: number }>;
  /** The context's output + base latency (s) and its sink ("none" in background test runs). */
  readonly latency: number;
  readonly sink: string;
}

/** The context's output device: "default", a device id, or "none" (AudioContext.sinkId; not in TS's DOM types yet). */
export const sinkOf = (ctx: AudioContext) => {
  const id = (ctx as AudioContext & { sinkId?: string | { type: string } }).sinkId;
  return id === undefined ? "default" : typeof id === "string" ? id || "default" : id.type;
};

const buffers = new WeakMap<AnalyserNode, Float32Array<ArrayBuffer>>();
/** RMS and peak of an analyser's window. */
export function levelOf(analyser: AnalyserNode): Level {
  let buf = buffers.get(analyser);
  if (!buf) buffers.set(analyser, (buf = new Float32Array(analyser.fftSize)));
  analyser.getFloatTimeDomainData(buf);
  let sum = 0;
  let peak = 0;
  for (const v of buf) {
    sum += v * v;
    peak = Math.max(peak, Math.abs(v));
  }
  return { rms: Math.sqrt(sum / buf.length), peak };
}
