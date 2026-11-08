// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type Rgb3, type Curve, clamp01, type Adjust, type LookDef, adjustOf, num, isNeutral, LUT_SIZE } from "@/app/lib/editor/core/visual/fx/defs";

// ---- colour maths ----

const LUMA: Rgb3 = [0.2126, 0.7152, 0.0722];
const luma = (c: Rgb3) => LUMA[0] * c[0] + LUMA[1] * c[1] + LUMA[2] * c[2];
const toLinear = (v: number) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
const toSrgb = (v: number) => (v <= 0.0031308 ? v * 12.92 : 1.055 * v ** (1 / 2.4) - 0.055);
type Px = [number, number, number];

/** A monotone cubic (Fritsch–Carlson) through a curve's points, as a function on 0–1. */
function curveFn(curve: Curve | undefined): ((x: number) => number) | undefined {
  if (!curve || curve.length < 2) return undefined;
  const pts = [...curve].filter((p) => Number.isFinite(p[0]) && Number.isFinite(p[1])).sort((a, b) => a[0] - b[0]);
  if (pts.length < 2) return undefined;
  const n = pts.length;
  const xs = pts.map((p) => clamp01(p[0]));
  const ys = pts.map((p) => clamp01(p[1]));
  const d: Array<number> = [];
  for (let i = 0; i < n - 1; i++) d.push(xs[i + 1]! > xs[i]! ? (ys[i + 1]! - ys[i]!) / (xs[i + 1]! - xs[i]!) : 0);
  const m: Array<number> = [d[0]!];
  for (let i = 1; i < n - 1; i++) m.push(d[i - 1]! * d[i]! <= 0 ? 0 : (d[i - 1]! + d[i]!) / 2);
  m.push(d[n - 2]!);
  for (let i = 0; i < n - 1; i++) {
    if (d[i] === 0) {
      m[i] = 0;
      m[i + 1] = 0;
      continue;
    }
    const a = m[i]! / d[i]!;
    const b = m[i + 1]! / d[i]!;
    const s = a * a + b * b;
    if (s > 9) {
      const t = 3 / Math.sqrt(s);
      m[i] = t * a * d[i]!;
      m[i + 1] = t * b * d[i]!;
    }
  }
  return (x: number) => {
    if (x <= xs[0]!) return ys[0]!;
    if (x >= xs[n - 1]!) return ys[n - 1]!;
    let i = 0;
    while (i < n - 2 && x > xs[i + 1]!) i++;
    const h = xs[i + 1]! - xs[i]!;
    if (h <= 0) return ys[i]!;
    const t = (x - xs[i]!) / h;
    const t2 = t * t;
    const t3 = t2 * t;
    return clamp01((2 * t3 - 3 * t2 + 1) * ys[i]! + (t3 - 2 * t2 + t) * h * m[i]! + (-2 * t3 + 3 * t2) * ys[i + 1]! + (t3 - t2) * h * m[i + 1]!);
  };
}

/** Adjust on one pixel (in place). */
function applyAdjust(c: Px, a: Adjust) {
  if (a.exposure || a.temperature || a.tint) {
    // Exposure and white balance in linear light: stops, then red/blue and green/magenta gains.
    const k = 2 ** a.exposure;
    const gains: Rgb3 = [1 + 0.3 * a.temperature + 0.06 * a.tint, 1 - 0.2 * a.tint, 1 - 0.3 * a.temperature + 0.06 * a.tint];
    for (let i = 0; i < 3; i++) c[i] = clamp01(toSrgb(Math.min(1, toLinear(c[i]!) * k * gains[i]!)));
  }
  if (a.highlights || a.shadows) {
    // Tone by luma: shadows lift or sink the dark third, highlights the bright third.
    const L = luma(c);
    const dL = 0.25 * (a.shadows * 6.75 * L * (1 - L) * (1 - L) + a.highlights * 6.75 * L * L * (1 - L));
    for (let i = 0; i < 3; i++) c[i] = clamp01(c[i]! + dL);
  }
  if (a.contrast) for (let i = 0; i < 3; i++) c[i] = clamp01(0.5 + (c[i]! - 0.5) * (1 + a.contrast));
  if (a.saturation || a.vibrance) {
    // Vibrance boosts the quiet colours more than the loud ones.
    const L = luma(c);
    const s = Math.max(c[0], c[1], c[2]) - Math.min(c[0], c[1], c[2]);
    const f = Math.max(0, 1 + a.saturation + a.vibrance * (1 - s) * (1 - s));
    for (let i = 0; i < 3; i++) c[i] = clamp01(L + (c[i]! - L) * f);
  }
}

/** A look's function on one pixel (in place). */
function lookFn(def: LookDef): (c: Px) => void {
  const adjust = def.adjust ? adjustOf(def.adjust) : undefined;
  const master = curveFn(def.curve);
  const per = [curveFn(def.red), curveFn(def.green), curveFn(def.blue)];
  const mixer = def.mixer && def.mixer.length === 9 && def.mixer.every(Number.isFinite) ? def.mixer : undefined;
  const mono = num(def.mono, 0, 1, 0);
  const fade = num(def.fade, 0, 0.5, 0);
  const tint = (t: LookDef["shadowTint"]) => (t && Array.isArray(t.color) && t.color.length === 3 ? { color: t.color.map((v) => num(v, 0, 1, 0)) as unknown as Rgb3, amount: num(t.amount, 0, 1, 0) } : undefined);
  const lo = tint(def.shadowTint);
  const hi = tint(def.highlightTint);
  return (c) => {
    if (mixer) {
      const [r, g, b] = c;
      for (let i = 0; i < 3; i++) c[i] = clamp01(mixer[i * 3]! * r + mixer[i * 3 + 1]! * g + mixer[i * 3 + 2]! * b);
    }
    if (mono) {
      const L = luma(c);
      for (let i = 0; i < 3; i++) c[i] = c[i]! + (L - c[i]!) * mono;
    }
    if (adjust) applyAdjust(c, adjust);
    if (master) for (let i = 0; i < 3; i++) c[i] = master(c[i]!);
    for (let i = 0; i < 3; i++) if (per[i]) c[i] = per[i]!(c[i]!);
    if (lo || hi) {
      // Split toning: towards the shadow colour where it's dark, the highlight colour where it's bright (soft-light-like, luma kept roughly).
      const L = luma(c);
      const ws = lo ? lo.amount * (1 - L) * (1 - L) : 0;
      const wh = hi ? hi.amount * L * L : 0;
      for (let i = 0; i < 3; i++) {
        let v = c[i]!;
        if (lo) v += ws * (lo.color[i]! - 0.5);
        if (hi) v += wh * (hi.color[i]! - 0.5);
        c[i] = clamp01(v);
      }
    }
    if (fade) for (let i = 0; i < 3; i++) c[i] = fade + c[i]! * (1 - fade);
  };
}

/** What a clip's colour is made of: a look at an intensity, an Adjust, invert. Undefined parts are off. */
export interface ColourSpec {
  readonly look?: { readonly def: LookDef; readonly intensity: number };
  readonly adjust?: Adjust;
  readonly invert?: boolean;
}

/** The colour function (sRGB-encoded 0–1 in and out): the look mixed in at its intensity, then Adjust, then invert. */
export function colourFn(spec: ColourSpec): (r: number, g: number, b: number) => Rgb3 {
  const look = spec.look && spec.look.intensity > 0 ? { fn: lookFn(spec.look.def), t: Math.min(1, spec.look.intensity) } : undefined;
  const adjust = spec.adjust && !isNeutral(spec.adjust) ? spec.adjust : undefined;
  return (r, g, b) => {
    const c: Px = [clamp01(r), clamp01(g), clamp01(b)];
    if (look) {
      const src: Rgb3 = [c[0], c[1], c[2]];
      look.fn(c);
      for (let i = 0; i < 3; i++) c[i] = src[i]! + (c[i]! - src[i]!) * look.t;
    }
    if (adjust) applyAdjust(c, adjust);
    if (spec.invert) for (let i = 0; i < 3; i++) c[i] = 1 - c[i]!;
    return [clamp01(c[0]), clamp01(c[1]), clamp01(c[2])];
  };
}

/** The colour table: LUT_SIZE³ RGB triples, red changing fastest (the .cube order). */
export interface ColourLut {
  readonly key: string;
  readonly size: number;
  readonly data: Float32Array;
}

const LUTS = new Map<string, ColourLut>();

/** A colour spec's table, cached by its key (several clips with one look share it). */
export function colourLut(spec: ColourSpec, size = LUT_SIZE): ColourLut {
  const key = `${size}:${JSON.stringify(spec)}`;
  const known = LUTS.get(key);
  if (known) return known;
  const fn = colourFn(spec);
  const data = new Float32Array(size * size * size * 3);
  let k = 0;
  for (let b = 0; b < size; b++)
    for (let g = 0; g < size; g++)
      for (let r = 0; r < size; r++) {
        const c = fn(r / (size - 1), g / (size - 1), b / (size - 1));
        data[k++] = c[0];
        data[k++] = c[1];
        data[k++] = c[2];
      }
  const lut = { key: lutHash(key), size, data };
  if (LUTS.size > 64) LUTS.clear();
  LUTS.set(key, lut);
  return lut;
}

/** A short stable name for a table (its file name in the export). */
function lutHash(s: string) {
  let h1 = 0x811c9dc5;
  let h2 = 0x01000193;
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    h1 = Math.imul(h1 ^ c, 0x01000193) >>> 0;
    h2 = Math.imul(h2 ^ c, 0x5bd1e995) >>> 0;
  }
  return `${h1.toString(16).padStart(8, "0")}${h2.toString(16).padStart(8, "0")}`;
}

/** A table as a .cube file (what ffmpeg's lut3d reads). */
export function cubeText(lut: ColourLut): string {
  const lines = [`TITLE "biliup ${lut.key}"`, `LUT_3D_SIZE ${lut.size}`, "DOMAIN_MIN 0 0 0", "DOMAIN_MAX 1 1 1"];
  for (let i = 0; i < lut.data.length; i += 3) lines.push(`${lut.data[i]!.toFixed(6)} ${lut.data[i + 1]!.toFixed(6)} ${lut.data[i + 2]!.toFixed(6)}`);
  return `${lines.join("\n")}\n`;
}

/** A table read the way lut3d interp=trilinear and the preview's shader read it. */
export function lutAt(lut: ColourLut, r: number, g: number, b: number): Rgb3 {
  const n = lut.size - 1;
  const at = (v: number) => {
    const x = clamp01(v) * n;
    const i = Math.min(n - 1, Math.floor(x));
    return [i, x - i] as const;
  };
  const [ri, rf] = at(r);
  const [gi, gf] = at(g);
  const [bi, bf] = at(b);
  const idx = (x: number, y: number, z: number) => ((z * lut.size + y) * lut.size + x) * 3;
  const out: Px = [0, 0, 0];
  for (let ch = 0; ch < 3; ch++) {
    const v = (x: number, y: number, z: number) => lut.data[idx(x, y, z) + ch]!;
    const c00 = v(ri, gi, bi) * (1 - rf) + v(ri + 1, gi, bi) * rf;
    const c10 = v(ri, gi + 1, bi) * (1 - rf) + v(ri + 1, gi + 1, bi) * rf;
    const c01 = v(ri, gi, bi + 1) * (1 - rf) + v(ri + 1, gi, bi + 1) * rf;
    const c11 = v(ri, gi + 1, bi + 1) * (1 - rf) + v(ri + 1, gi + 1, bi + 1) * rf;
    out[ch] = (c00 * (1 - gf) + c10 * gf) * (1 - bf) + (c01 * (1 - gf) + c11 * gf) * bf;
  }
  return out;
}

/** Posterize on one channel value (0–1): `levels` flat bands, as the shader's floor and the export's lutrgb compute it. */
export const posterize = (v: number, levels: number) => Math.min(levels - 1, Math.floor(clamp01(v) * levels)) / (levels - 1);
