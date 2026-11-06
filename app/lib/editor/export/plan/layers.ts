// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type Item, type Span, fadesIn, moveFades, type ClipBox, type Rect, type Project, sourceAt } from "@/app/lib/editor/core";
import type { Ramp, LayerBase, Placement, VisualLayer, ExportPlan } from "./types.ts";
import { n } from "./num.ts";

// ---- opacity ----


/**
 * The opacity ramps of an item as visualsAt computes its opacity: a crossfade's (0 at `from` → 1
 * at start + half, a zoom's too; a wipe, iris or push changes no opacity), then the in and out animations (core
 * moveFades: fade, rise, slide-left and a fading text zoom change opacity; wipe and word by word don't), skipped on a side where a main-track transition blends.
 */
export function opacityRamps(item: Item, start: number, span: Pick<Span, "half" | "skipIn" | "skipOut" | "into">): Array<Ramp> {
  const ramps: Array<Ramp> = [];
  // A crossfade or zoom fades the incoming clip in; a wipe, iris or push doesn't (core incomingFx).
  if (span.half > 0 && fadesIn(span.into ?? "")) ramps.push({ type: "in", st: start - span.half, d: 2 * span.half });
  const a = item.animation;
  if (!span.skipIn && a?.in && a.in.duration > 0 && moveFades(item, a.in)) ramps.push({ type: "in", st: start, d: a.in.duration });
  if (!span.skipOut && a?.out && a.out.duration > 0 && moveFades(item, a.out)) ramps.push({ type: "out", st: start + item.duration - 1 - a.out.duration, d: a.out.duration });
  return ramps;
}

/** The box's opacity at frame f (1 without one). */
export const alphaAt = (layer: Pick<LayerBase, "alpha" | "f0">, f: number) => layer.alpha?.[f - layer.f0] ?? 1;

/**
 * A core box in output pixels: `kx`, `ky` output pixels per canvas pixel. The box's size and the
 * picture inside it are rounded where the box is without keys (`base`, core baseBox), every edge
 * on its own (not x and w), so two boxes sharing an edge meet on one pixel, a picture that covers
 * its box still covers it, and a box that moves (x / y keys) keeps one size. x and y are the box's
 * position at this frame.
 */
export function placementOf(box: ClipBox, kx: number, ky: number, base: Rect = box): Placement {
  const x0 = Math.round(base.x * kx);
  const y0 = Math.round(base.y * ky);
  const w = Math.max(1, Math.round((base.x + base.w) * kx) - x0);
  const h = Math.max(1, Math.round((base.y + base.h) * ky) - y0);
  // The picture as it sits in the box, moved with the box to where the box is without keys.
  const c = { x: box.content.x - box.x + base.x, y: box.content.y - box.y + base.y, w: box.content.w, h: box.content.h };
  const cx = Math.round(c.x * kx) - x0;
  const cy = Math.round(c.y * ky) - y0;
  const cw = Math.max(1, Math.round((c.x + c.w) * kx) - x0 - cx);
  const ch = Math.max(1, Math.round((c.y + c.h) * ky) - y0 - cy);
  return { x: Math.round(box.x * kx), y: Math.round(box.y * ky), w, h, cx, cy, cw, ch, radius: Math.min(Math.round(box.radius * Math.min(kx, ky)), Math.floor(Math.min(w, h) / 2)) };
}

/** The share of a wiped layer shown at frame f (core visualsAt's `wipe`; 1 when whole). */
export const wipeAt = (wipe: VisualLayer["wipe"], f: number) => (wipe ? Math.min(1, Math.max(0, (f - wipe.st) / wipe.d)) : 1);

/** An iris's mask is computed at 1/this of the output size, then scaled up (D79 review: geq per pixel is costly). */
export const IRIS_MASK_STEP = 4;

/** gblur's passes for a blur dissolve (D133): more is closer to a true Gaussian (Chromium's CSS blur). */
export const BLUR_STEPS = 3;

/** The transparent margin a blurred layer gets (px, even): 3 sigma, where a Gaussian has all but 0.3 % of its weight. */
export const blurPad = (sigma: number) => 2 * Math.ceil((3 * sigma + 1) / 2);

/** A blur dissolve's filters at one sigma (output px), as the export's (render_frames). */
export const blurFilters = (sigma: number) => {
  const pad = blurPad(sigma);
  return `pad=w=iw+${2 * pad}:h=ih+${2 * pad}:x=${pad}:y=${pad}:color=black@0,premultiply=inplace=1,gblur=sigma=${n(sigma)}:steps=${BLUR_STEPS},unpremultiply=inplace=1,crop=w=iw-${2 * pad}:h=ih-${2 * pad}:x=${pad}:y=${pad}`;
};

/** The share of the half diagonal an iris shows at frame f (core visualsAt's `iris`; 1 when whole). */
export const irisAt = (iris: VisualLayer["iris"], f: number) => (iris ? Math.min(1, Math.max(0, (f - iris.st) / iris.d)) : 1);

/** The ramps' opacity at frame f, as ffmpeg's fade computes it (the tests compare it with visualsAt). */
export const rampsAt = (ramps: ReadonlyArray<Ramp>, f: number) =>
  ramps.reduce((o, r) => {
    const p = Math.min(1, Math.max(0, (f - r.st) / r.d));
    return o * (r.type === "in" ? p : 1 - p);
  }, 1);

/**
 * The frames [from, to) of a layer (from f0 ≤ from < to ≤ f1) as a layer of their own: per-frame
 * data sliced, a video's source time moved on. `source` overrides that (a looped clip's next pass).
 */
function sliceLayer(layer: VisualLayer, from: number, to: number, fps: number, source?: number): VisualLayer {
  const a = from - layer.f0;
  const b = to - layer.f0;
  const common = { f0: from, f1: to, ...(layer.alpha ? { alpha: layer.alpha.slice(a, b) } : {}), ...(layer.blur ? { blur: layer.blur.slice(a, b) } : {}), ...(layer.geometry ? { geometry: { frames: layer.geometry.frames.slice(a, b) } } : {}) };
  if (layer.kind === "raster") return { ...layer, ...common, ...compactStates(layer.states, layer.frames.slice(a, b)) };
  if (layer.kind === "graphic") return { ...layer, ...common, frames: layer.frames.slice(a, b), ...(layer.motion ? { motion: { ...layer.motion, ts: layer.motion.ts.slice(a, b) } } : {}) };
  if (layer.kind === "video" && layer.cycle) return { ...layer, ...common, cycle: { ...layer.cycle, offset: (layer.cycle.offset + a) % layer.cycle.length }, source: (((layer.cycle.offset + a) % layer.cycle.length) / fps) };
  if (layer.kind === "video") return { ...layer, ...common, source: Math.min(source ?? layer.source + (a * layer.speed) / fps, layer.last ?? Infinity) };
  return { ...layer, ...common };
}

/** A slice's looks: only the ones its frames use (in first-use order), the frames renumbered (D79 review). */
export function compactStates(states: ReadonlyArray<string>, frames: ReadonlyArray<number>): { states: Array<string>; frames: Array<number> } {
  const index = new Map<number, number>();
  const kept: Array<string> = [];
  const renumbered = frames.map((k) => {
    let j = index.get(k);
    if (j === undefined) {
      j = kept.length;
      index.set(k, j);
      kept.push(states[k]!);
    }
    return j;
  });
  return { states: kept, frames: renumbered };
}

/** The distinct looks the rasteriser draws for a plan: each raster state once, however many layers or slices share it. */
export const rasterLooks = (plan: Pick<ExportPlan, "visuals">) => [...new Set(plan.visuals.flatMap((l) => (l.kind === "raster" ? l.states : [])))];

/**
 * Where an item's layer is cut so a transition's frames are a layer of their own (D73, D79): the
 * end of a wipe or iris into it (`revealEnd`: the reveal is dropped after it; always cut) and the
 * end of a push into it or the start of a push or zoom out of it (`moves`: cut only where the
 * geometry stops or starts being needed, so a boxed clip isn't cut for nothing). The
 * pad/crop/mask/box work runs only over those frames.
 */
interface Cuts {
  readonly revealEnd?: number;
  /** Always cut here too (D133: where a blur starts or stops). */
  readonly always?: ReadonlyArray<number>;
  readonly moves: ReadonlyArray<number>;
  /** The item's frames whose box is plain (no geometry needed), when it has geometry. */
  readonly plain?: (f: number) => boolean;
}

export function splitAt(layer: VisualLayer, cuts: Cuts, fps: number): Array<VisualLayer> {
  const allPlain = (a: number, b: number) => {
    for (let f = a; f < b; f++) if (!cuts.plain!(f)) return false;
    return true;
  };
  const inside = (c: number) => c > layer.f0 && c < layer.f1;
  const reveal = [...(cuts.revealEnd !== undefined && inside(cuts.revealEnd) && (layer.wipe || layer.iris || layer.mask) ? [cuts.revealEnd] : []), ...(layer.blur ? (cuts.always ?? []).filter(inside) : [])];
  const candidates = [...new Set([...reveal, ...cuts.moves.filter(inside)])].sort((a, b) => a - b);
  // A move's cut is kept only where the frames on its two sides differ in needing geometry.
  const kept: Array<number> = [];
  for (const [i, c] of candidates.entries()) {
    if (reveal.includes(c)) {
      kept.push(c);
      continue;
    }
    if (!layer.geometry || !cuts.plain) continue;
    const before = kept.at(-1) ?? layer.f0;
    const after = candidates[i + 1] ?? layer.f1;
    if (allPlain(before, c) !== allPlain(c, after)) kept.push(c);
  }
  const points = [layer.f0, ...kept, layer.f1];
  const out: Array<VisualLayer> = [];
  for (let i = 0; i + 1 < points.length; i++) {
    const [a, b] = [points[i]!, points[i + 1]!];
    let slice = points.length === 2 ? layer : sliceLayer(layer, a, b, fps);
    if (cuts.revealEnd !== undefined && a >= cuts.revealEnd && (slice.wipe || slice.iris || slice.mask)) {
      const { wipe, iris, mask, ...rest } = slice;
      slice = rest as VisualLayer;
    }
    if (slice.blur && slice.blur.every((b) => b === 0)) {
      const { blur, ...rest } = slice;
      slice = rest as VisualLayer;
    }
    if (slice.geometry && cuts.plain) {
      let plain = true;
      for (let f = a; f < b && plain; f++) plain = cuts.plain(f);
      if (plain) {
        const { geometry, ...rest } = slice;
        slice = rest as VisualLayer;
      }
    }
    out.push(slice);
  }
  return out;
}

/**
 * A looped video clip as one layer per pass through its media (each from the source time core
 * gives its first frame), so every pass is decoded from a plain seek rather than a looped input.
 */
/** Passes a looped clip may take as inputs of their own; more go through the loop filter (one input). */
export const MAX_LOOP_INPUTS = 8;
/** The most memory (bytes) the loop filter may hold for one looped clip; above it, one input per pass. */
const MAX_LOOP_BUFFER = 1.5e9;

export function splitLoop(layer: VisualLayer, project: Project, start: number, size: { width: number; height: number }): Array<VisualLayer> {
  if (layer.kind !== "video" || !layer.item.loop) return [layer];
  const fps = project.canvas.fps;
  const length = layer.item.media ? project.media.find((m) => m.id === layer.item.media)?.probe?.frames : undefined;
  if (!length) return [layer];
  const out: Array<VisualLayer> = [];
  const pass = (f: number) => Math.floor(((layer.item.in ?? 0) + (f - start) * layer.speed) / length);
  // Many passes at speed 1: one input, the media decoded once and repeated (memory permitting).
  const geo = layer.geometry?.frames;
  const pixels = layer.masks ? layer.masks.width*layer.masks.height : geo ? Math.max(...geo.map((p) => p.cw)) * Math.max(...geo.map((p) => p.ch)) : size.width * size.height;
  if (layer.speed === 1 && pass(layer.f1 - 1) - pass(layer.f0) + 1 > MAX_LOOP_INPUTS && length * pixels * 1.5 <= MAX_LOOP_BUFFER) {
    const offset = (((layer.item.in ?? 0) + layer.f0 - start) % length + length) % length;
    return [{ ...layer, source: offset / fps, cycle: { length, offset } }];
  }
  let from = layer.f0;
  for (let f = layer.f0 + 1; f <= layer.f1; f++) {
    if (f < layer.f1 && pass(f) === pass(from)) continue;
    out.push(sliceLayer(layer, from, f, fps, sourceAt(project, layer.item, from - start)));
    from = f;
  }
  return out;
}

/**
 * Looks per frame, at most `cap` distinct: when there are more, frames are held (every k-th
 * frame's look is shown for k frames, the smallest k that fits). Returns the looks, each frame's
 * index and the hold (1 = every frame drawn). The export no longer caps (D291: the rasteriser
 * streams every frame), so by default every look is kept.
 */
export function capStates(looks: ReadonlyArray<string>, cap = Infinity): { states: Array<string>; frames: Array<number>; hold: number } {
  for (let hold = 1; ; hold++) {
    const states: Array<string> = [];
    const index = new Map<string, number>();
    const frames: Array<number> = [];
    for (let i = 0; i < looks.length; i++) {
      const html = looks[i - (i % hold)]!;
      let k = index.get(html);
      if (k === undefined) {
        k = states.length;
        states.push(html);
        index.set(html, k);
      }
      frames.push(k);
    }
    if (states.length <= cap || hold >= looks.length) return { states, frames, hold };
  }
}

/** An SVG image clip's box as HTML (D76): the picture at its content rect in a box that clips and rounds it, Chromium-drawn at canvas size. */
export function svgClipHtml(item: Item, box: ClipBox, src: string, opacity: number): string {
  const r = (n: number) => `${Math.round(n * 1000) / 1000}px`;
  const c = box.content;
  return `<div data-item="${item.id}" style="position:absolute;left:${r(box.x)};top:${r(box.y)};width:${r(box.w)};height:${r(box.h)};overflow:hidden;opacity:${Math.round(opacity * 10000) / 10000}${box.radius > 0 ? `;border-radius:${r(box.radius)}` : ""}"><img src="${src}" alt="" style="position:absolute;display:block;left:${r(c.x - box.x)};top:${r(c.y - box.y)};width:${r(c.w)};height:${r(c.h)};object-fit:${box.fit}"></div>`;
}
