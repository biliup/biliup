// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Motion blur in the export (D291): a frame is the average of `samples` pictures drawn at moments
// spread over the open shutter, `shutter` degrees of the frame interval (180°: half of it, a film
// camera's; 90° by default, see MOTION_BLUR_DEFAULTS), opening at the frame's own time, each at the
// middle of its equal slot of the shutter (D309: so a finer grid covers the same span, see
// blurLevels); one sample is no blur. The export draws graphics this way (the rasteriser streams each sub-frame,
// ffmpeg averages them on premultiplied colour). The preview draws no blur.
import type { Item, Project } from "@/app/lib/editor/core/timeline/types";

export interface MotionBlur {
  /** Pictures averaged per frame. */
  readonly samples: number;
  /** Degrees of the frame interval the sub-frames spread over (360 = the whole interval). */
  readonly shutter: number;
}

/**
 * What an export uses without a project setting: 4 sub-frames over a 90° shutter. Round 2's A1 (12
 * sub-frames, the old 180° default) smeared a camera push into the footage and a UI morph into
 * unreadable ghosts; rendered side by side, 90° kept the move smooth with a quarter-frame trail and
 * the text readable, where 180° doubled the smear and 4 sub-frames over 180° showed stepped copies.
 * Graphics are UI and type moving fast under a camera, not film footage: the shorter shutter suits them.
 */
export const MOTION_BLUR_DEFAULTS: MotionBlur = { samples: 4, shutter: 90 };
export const MOTION_BLUR_SAMPLES = { min: 2, max: 16 } as const;

/** Whether an export blurs by default: the project's `on`, else on for Standard and High, off for Draft. */
export const motionBlurDefault = (project: Pick<Project, "motionBlur">, quality: "draft" | "standard" | "high") => project.motionBlur?.on ?? quality !== "draft";

/** The project's blur (its samples and shutter, or the defaults), clamped to what the export takes. */
export function projectMotionBlur(project: Pick<Project, "motionBlur">): MotionBlur {
  const s = project.motionBlur?.samples;
  const sh = project.motionBlur?.shutter;
  return {
    samples: typeof s === "number" && Number.isFinite(s) ? Math.min(MOTION_BLUR_SAMPLES.max, Math.max(MOTION_BLUR_SAMPLES.min, Math.round(s))) : MOTION_BLUR_DEFAULTS.samples,
    shutter: typeof sh === "number" && Number.isFinite(sh) ? Math.min(360, Math.max(1, sh)) : MOTION_BLUR_DEFAULTS.shutter,
  };
}

/** An item's blur in an export with blur `on` (the export's switch): none when it's off or the item opts out (motionBlur false). */
export const itemMotionBlur = (item: Pick<Item, "motionBlur">, project: Pick<Project, "motionBlur">, on: boolean): MotionBlur | undefined => (on && item.motionBlur !== false ? projectMotionBlur(project) : undefined);

/**
 * The moments (seconds) drawn for frame `frame` (a number of frames from the item's start, at
 * `fps`): sub-frame j of n at (frame + (j + ½)·(shutter/360)/n) / fps, j = 0…n−1 (the middle of
 * each of n equal slots of the open shutter, D309). Without blur, the frame's
 * own moment. `still` (a held frame: before the item starts or after it ends) gives the frame's own
 * moment n times, so a held frame stays sharp.
 */
export function subFrameTimes(frame: number, fps: number, blur?: MotionBlur, still = false): Array<number> {
  if (!blur || blur.samples <= 1) return [frame / fps];
  const open = blur.shutter / 360;
  return Array.from({ length: blur.samples }, (_, j) => (still ? frame : frame + ((j + 0.5) * open) / blur.samples) / fps);
}

/**
 * Adaptive sub-frames (D309): a frame whose sub-frames are far apart (a fast move: text or UI tens
 * of px from one sub-frame to the next) averages into N stacked copies, not a smear. The rasteriser
 * compares each pair of consecutive sub-frames (blurStep on blurSignature) and, while most pairs
 * still have a change no small move explains (blurMoving), draws the frame again on the next finer
 * grid (blurLevels: the same slots split, so the same span and centre; each grid in time order) up
 * to BLUR_REFINE.max, and averages the finest grid it completed. A calm frame keeps the project's
 * samples. Refining is optional: a grid that fails or would run past the frame's time budget is
 * dropped and the last complete one kept.
 */
export const BLUR_REFINE = {
  /** The most sub-frames a frame gets by refining (a project's own samples above it stay as they are). */
  max: 32,
  /** A signature point stands for a block this many px square. */
  block: 4,
  /**
   * A point changed by more than this (0–255, luma or alpha) beyond what its neighbours in the other
   * sub-frame span (±1 block: a move of up to ~a block) is a change no small move explains.
   */
  tolerance: 40,
  /** A cell's side in points (32 px): the busiest cell counts. */
  cell: 8,
  /**
   * Unexplained points in one cell from which sub-frames are doubled: a 4 px line that jumped (8
   * points down a cell), a glyph's strokes; not an edge sliding a few px (explained) nor a speck.
   */
  points: 4,
} as const;

/** A sub-frame's picture reduced for blurStep: luma and alpha a point per BLUR_REFINE.block px block, with each point's 3×3 range. */
export interface BlurSignature {
  readonly cols: number;
  readonly rows: number;
  readonly luma: Uint8Array;
  readonly alpha: Uint8Array;
  readonly lumaLo: Uint8Array;
  readonly lumaHi: Uint8Array;
  readonly alphaLo: Uint8Array;
  readonly alphaHi: Uint8Array;
}

/** The 3×3 neighbourhood's lowest and highest of `v` (cols × rows); a plane all one value (an opaque picture's alpha) is its own range. */
function range3(v: Uint8Array, cols: number, rows: number): { lo: Uint8Array; hi: Uint8Array } {
  const first = v[0]!;
  let flat = true;
  for (let k = 1; k < v.length && flat; k++) flat = v[k] === first;
  if (flat) return { lo: v, hi: v };
  const rowLo = new Uint8Array(v.length);
  const rowHi = new Uint8Array(v.length);
  for (let y = 0; y < rows; y++) {
    const r = y * cols;
    for (let x = 0; x < cols; x++) {
      const k = r + x;
      const m = v[k]!;
      const l = x > 0 ? v[k - 1]! : m;
      const n = x < cols - 1 ? v[k + 1]! : m;
      rowLo[k] = l < m ? (l < n ? l : n) : m < n ? m : n;
      rowHi[k] = l > m ? (l > n ? l : n) : m > n ? m : n;
    }
  }
  const lo = new Uint8Array(v.length);
  const hi = new Uint8Array(v.length);
  for (let k = 0; k < v.length; k++) {
    const up = k >= cols ? k - cols : k;
    const down = k + cols < v.length ? k + cols : k;
    const a = rowLo[up]!, b = rowLo[k]!, c = rowLo[down]!;
    lo[k] = a < b ? (a < c ? a : c) : b < c ? b : c;
    const d = rowHi[up]!, e = rowHi[k]!, f = rowHi[down]!;
    hi[k] = d > e ? (d > f ? d : f) : e > f ? e : f;
  }
  return { lo, hi };
}

/** A BGRA bitmap (premultiplied, width × height) reduced to its signature. */
export function blurSignature(bgra: Uint8Array, width: number, height: number): BlurSignature {
  const b = BLUR_REFINE.block;
  const cols = Math.ceil(width / b);
  const rows = Math.ceil(height / b);
  const luma = new Uint8Array(cols * rows);
  const alpha = new Uint8Array(cols * rows);
  // Two pixels a block, at (1, 1) and (3, 3) (an eighth of them, cheap enough for every sub-frame;
  // a 2 px line still lands on one), averaged.
  const at = (x: number, y: number) => (Math.min(y, height - 1) * width + Math.min(x, width - 1)) * 4;
  for (let by = 0, k = 0; by < rows; by++) {
    for (let bx = 0; bx < cols; bx++, k++) {
      const i = at(bx * b + 1, by * b + 1);
      const j = at(bx * b + 3, by * b + 3);
      luma[k] = (bgra[i]! + 5 * bgra[i + 1]! + 2 * bgra[i + 2]! + bgra[j]! + 5 * bgra[j + 1]! + 2 * bgra[j + 2]!) >> 4;
      alpha[k] = (bgra[i + 3]! + bgra[j + 3]!) >> 1;
    }
  }
  const l = range3(luma, cols, rows);
  const a = range3(alpha, cols, rows);
  return { cols, rows, luma, alpha, lumaLo: l.lo, lumaHi: l.hi, alphaLo: a.lo, alphaHi: a.hi };
}

/**
 * How far apart two sub-frames are: the most points in one BLUR_REFINE.cell² cell whose value in
 * one lies outside the other's 3×3 range by more than BLUR_REFINE.tolerance (either way round, luma
 * or alpha), i.e. changes no move of about a block explains. 0 for the same picture or a small move.
 */
export function blurStep(a: BlurSignature, b: BlurSignature): number {
  if (a.cols !== b.cols || a.rows !== b.rows) return Infinity;
  const c = BLUR_REFINE.cell;
  const tol = BLUR_REFINE.tolerance;
  const cx = Math.ceil(a.cols / c);
  const counts = new Uint16Array(cx * Math.ceil(a.rows / c));
  const { luma: al, alpha: aa, lumaLo: all, lumaHi: alh, alphaLo: aal, alphaHi: aah } = a;
  const { luma: bl, alpha: ba, lumaLo: bll, lumaHi: blh, alphaLo: bal, alphaHi: bah } = b;
  for (let y = 0, k = 0; y < a.rows; y++) {
    const row = ((y / c) | 0) * cx;
    for (let x = 0; x < a.cols; x++, k++) {
      const lb = bl[k]!;
      const la = al[k]!;
      const ab = ba[k]!;
      const aA = aa[k]!;
      if (lb > alh[k]! + tol || lb < all[k]! - tol || la > blh[k]! + tol || la < bll[k]! - tol || ab > aah[k]! + tol || ab < aal[k]! - tol || aA > bah[k]! + tol || aA < bal[k]! - tol) counts[row + ((x / c) | 0)]!++;
    }
  }
  let worst = 0;
  for (const n of counts) worst = Math.max(worst, n);
  return worst;
}

/** How many consecutive pairs of `sigs` (sub-frames in time order) are far apart: blurStep at least BLUR_REFINE.points. */
export function blurFarPairs(sigs: ReadonlyArray<BlurSignature>): number {
  let far = 0;
  for (let j = 1; j < sigs.length; j++) if (blurStep(sigs[j - 1]!, sigs[j]!) >= BLUR_REFINE.points) far++;
  return far;
}

/**
 * Whether a frame drawn from `n` sub-frames, `far` of its gaps far apart (blurFarPairs), is a move
 * to halve again: most gaps far. A move spreads over every gap; a jump at one moment (something
 * popping in, text snapping a pixel on a slow drift) stays in one gap however often it's halved.
 */
export const blurMoving = (far: number, n: number) => far * 2 > n - 1;

/**
 * The finer grids a frame drawn at `ts` (subFrameTimes: n slot middles, an even grid) is refined
 * through, each in time order: n slots split into 4n, then 2× more each time, up to `max` (a grid
 * of n ≥ max has none). Every grid is the middles of equal slots of the same span, so the mean of
 * any of them covers the same shutter with the same centre.
 */
export function blurLevels(ts: ReadonlyArray<number>, max: number = BLUR_REFINE.max): Array<Array<number>> {
  const n = ts.length;
  if (n < 2) return [];
  const w = ts[1]! - ts[0]!;
  const start = ts[0]! - w / 2;
  const span = w * n;
  const out: Array<Array<number>> = [];
  for (let N = Math.min(max, 4 * n); N > (out.at(-1)?.length ?? n); N = Math.min(max, N * 2)) out.push(Array.from({ length: N }, (_, j) => start + ((j + 0.5) * span) / N));
  return out;
}

/** Whether `ts` is an even grid of distinct times (what doubling assumes): a held frame's repeated moment isn't. */
export function blurRefinable(ts: ReadonlyArray<number>): boolean {
  if (ts.length < 2) return false;
  const step = ts[1]! - ts[0]!;
  if (!(step > 0)) return false;
  return ts.every((t, j) => Math.abs(t - (ts[0]! + j * step)) <= step * 1e-6);
}
