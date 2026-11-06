// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Noise removal heard in the preview (D246; export: ffmpeg afftdn, D88). One spectral denoiser the
// preview runs in an AudioWorklet (apps/web/src/audio/mix/denoiseWorklet.ts) and the tests run in
// Node against afftdn (test/export/issue83-export.test.ts): a short-time Fourier transform (1024 samples, hop 256, √Hann in and out),
// a white noise floor at `floorDb` (the level afftdn's nf is given, dBFS RMS), and per bin a
// spectral-subtraction gain 1 − β·noise/power (power smoothed over time), never under the
// reduction afftdn's nr allows (6 + 24·amount dB), smoothed over time so it doesn't warble. Each
// channel is processed on its own, as afftdn does. Latency: one frame (1024 samples, ~21 ms).
//
// Self-contained on purpose (no imports, no module-level helpers): the worklet bundles it alone.

/** The FFT size, hop and the latency they add (samples). */
export const DENOISE_FFT = 1024;
export const DENOISE_HOP = 256;
/** The reduction (dB) afftdn is given for an amount 0…1 (the export's nr, D88). */
export const denoiseReductionDb = (amount: number) => 6 + 24 * Math.min(1, Math.max(0, amount));

export class SpectralDenoiser {
  private readonly n = DENOISE_FFT;
  private readonly hop = DENOISE_HOP;
  private readonly win: Float32Array;
  private readonly cos: Float32Array;
  private readonly sin: Float32Array;
  private readonly rev: Uint32Array;
  private readonly inBuf: Float32Array;
  private readonly outBuf: Float32Array;
  private readonly re: Float32Array;
  private readonly im: Float32Array;
  private readonly tr: Float32Array;
  private readonly ti: Float32Array;
  private readonly power: Float32Array;
  private readonly gain: Float32Array;
  private fill = 0;
  private outPos = 0;
  private noiseBin = 0;
  private minGain = 1;
  /** Over-subtraction and smoothing, fitted to afftdn (test/export/issue83-export.test.ts). */
  static readonly BETA = 1.6;
  static readonly POWER_SMOOTH = 0.55;
  static readonly GAIN_RELEASE = 0.5;

  constructor() {
    const n = this.n;
    this.win = new Float32Array(n);
    for (let i = 0; i < n; i++) this.win[i] = Math.sqrt(0.5 - 0.5 * Math.cos((2 * Math.PI * i) / n));
    this.cos = new Float32Array(n / 2);
    this.sin = new Float32Array(n / 2);
    for (let i = 0; i < n / 2; i++) {
      this.cos[i] = Math.cos((2 * Math.PI * i) / n);
      this.sin[i] = -Math.sin((2 * Math.PI * i) / n);
    }
    const bits = Math.log2(n);
    this.rev = new Uint32Array(n);
    for (let i = 0; i < n; i++) {
      let r = 0;
      for (let b = 0; b < bits; b++) r |= ((i >> b) & 1) << (bits - 1 - b);
      this.rev[i] = r;
    }
    this.inBuf = new Float32Array(n);
    this.outBuf = new Float32Array(n);
    this.re = new Float32Array(n);
    this.im = new Float32Array(n);
    this.tr = new Float32Array(n);
    this.ti = new Float32Array(n);
    this.power = new Float32Array(n / 2 + 1);
    this.gain = new Float32Array(n / 2 + 1).fill(1);
    this.set(0.5, -40);
  }

  /** The amount (0…1) and the noise floor (dBFS RMS of white noise, as afftdn's nf). */
  set(amount: number, floorDb: number) {
    const a = Math.min(1, Math.max(0, amount));
    this.minGain = 10 ** (-(6 + 24 * a) / 20);
    // White noise of variance σ² gives each bin E|X|² = σ² · Σ w² = σ² · n/2 under a √Hann window.
    this.noiseBin = 10 ** (Math.min(-20, Math.max(-80, floorDb)) / 10) * (this.n / 2);
  }

  /** Back to silence (a seek, a new chain). */
  reset() {
    this.inBuf.fill(0);
    this.outBuf.fill(0);
    this.power.fill(0);
    this.gain.fill(1);
    this.fill = 0;
    this.outPos = 0;
  }

  /** Denoise `input` into `output` (the same length, any length; delayed by DENOISE_FFT samples). */
  process(input: Float32Array, output: Float32Array) {
    const n = this.n;
    const hop = this.hop;
    for (let i = 0; i < input.length; i++) {
      // inBuf holds the last n samples; every hop a frame is analysed and overlap-added into outBuf.
      this.inBuf[n - hop + this.fill] = input[i]!;
      output[i] = this.outBuf[this.outPos]!;
      this.outPos++;
      this.fill++;
      if (this.fill === hop) {
        this.frame();
        this.inBuf.copyWithin(0, hop);
        this.fill = 0;
        this.outPos = 0;
      }
    }
  }

  private frame() {
    const n = this.n;
    const hop = this.hop;
    const { re, im, win, rev } = this;
    for (let i = 0; i < n; i++) {
      const j = rev[i]!;
      re[j] = this.inBuf[i]! * win[i]!;
      im[j] = 0;
    }
    this.fft();
    const beta = SpectralDenoiser.BETA;
    const ps = SpectralDenoiser.POWER_SMOOTH;
    const rel = SpectralDenoiser.GAIN_RELEASE;
    for (let k = 0; k <= n / 2; k++) {
      const p = re[k]! * re[k]! + im[k]! * im[k]!;
      const smooth = ps * this.power[k]! + (1 - ps) * p;
      this.power[k] = smooth;
      let g = smooth > 0 ? 1 - (beta * this.noiseBin) / smooth : 0;
      if (g < this.minGain) g = this.minGain;
      // Rises at once (speech onsets stay), falls smoothly (no musical noise).
      const prev = this.gain[k]!;
      if (g < prev) g = rel * prev + (1 - rel) * g;
      this.gain[k] = g;
      re[k] = re[k]! * g;
      im[k] = im[k]! * g;
      if (k > 0 && k < n / 2) {
        re[n - k] = re[k]!;
        im[n - k] = -im[k]!;
      }
    }
    // Inverse: conjugate, forward FFT (bit-reversed copy first), conjugate, scale.
    const { tr, ti } = this;
    tr.set(re);
    ti.set(im);
    for (let i = 0; i < n; i++) {
      const j = rev[i]!;
      re[j] = tr[i]!;
      im[j] = -ti[i]!;
    }
    this.fft();
    // √Hann in and out at hop n/4: the windows sum to 2.
    const scale = 1 / (n * 2);
    // Shift the output buffer by a hop and add this frame.
    this.outBuf.copyWithin(0, hop);
    this.outBuf.fill(0, n - hop);
    for (let i = 0; i < n; i++) this.outBuf[i] = this.outBuf[i]! + re[i]! * win[i]! * scale;
  }

  /** In-place radix-2 FFT of (re, im), already in bit-reversed order. */
  private fft() {
    const n = this.n;
    const { re, im, cos, sin } = this;
    for (let size = 2; size <= n; size <<= 1) {
      const half = size >> 1;
      const step = n / size;
      for (let start = 0; start < n; start += size) {
        for (let j = 0; j < half; j++) {
          const wr = cos[j * step]!;
          const wi = sin[j * step]!;
          const a = start + j;
          const b = a + half;
          const xr = re[b]! * wr - im[b]! * wi;
          const xi = re[b]! * wi + im[b]! * wr;
          re[b] = re[a]! - xr;
          im[b] = im[a]! - xi;
          re[a] = re[a]! + xr;
          im[a] = im[a]! + xi;
        }
      }
    }
  }
}
