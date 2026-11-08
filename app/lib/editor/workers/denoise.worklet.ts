// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Noise removal heard in the preview (D246): an AudioWorklet running core's SpectralDenoiser on each
// channel, the algorithm the tests fit to the export's afftdn (test/export/issue83-export.test.ts).
// Loaded by nodes.ts with addModule (Vite bundles this file and denoise.ts alone: no other imports).
// The amount and the track's noise floor arrive on the port: { amount, floorDb }; { reset: true }
// clears its buffers (a seek, a play start); { dispose: true } ends it (D262: its chain was thrown
// away): the next process() posts { ended: true } and returns false, so the render thread lets go of
// it. Latency: DENOISE_FFT samples (~21 ms at 48 kHz); the mix starts the track's sound that much
// early (audio.ts, D262).
import { SpectralDenoiser } from "@/app/lib/editor/core/audio/denoise";

declare function registerProcessor(name: string, ctor: unknown): void;
declare class AudioWorkletProcessor {
  readonly port: MessagePort;
}

class DenoiseProcessor extends AudioWorkletProcessor {
  private readonly ch = [new SpectralDenoiser(), new SpectralDenoiser()];
  private silence = new Float32Array(128);
  private disposed = false;
  constructor() {
    super();
    this.port.onmessage = (e: MessageEvent<{ amount?: number; floorDb?: number; reset?: boolean; dispose?: boolean }>) => {
      const m = e.data;
      if (m.dispose) this.disposed = true;
      if (m.reset) for (const d of this.ch) d.reset();
      if (typeof m.amount === "number" && typeof m.floorDb === "number") for (const d of this.ch) d.set(m.amount, m.floorDb);
    };
  }
  process(inputs: Array<Array<Float32Array>>, outputs: Array<Array<Float32Array>>) {
    if (this.disposed) {
      this.port.postMessage({ ended: true });
      this.port.onmessage = null;
      return false;
    }
    const input = inputs[0] ?? [];
    const output = outputs[0] ?? [];
    for (let c = 0; c < output.length; c++) {
      const src = input[c] ?? input[0];
      const d = this.ch[c] ?? this.ch[0]!;
      // No input connected (between items): keep feeding silence so the tail plays out.
      if (!src && this.silence.length !== output[c]!.length) this.silence = new Float32Array(output[c]!.length);
      d.process(src ?? this.silence, output[c]!);
    }
    return true;
  }
}
registerProcessor("biliup-denoise", DenoiseProcessor);
