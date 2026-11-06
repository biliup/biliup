// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.


export const LOOK = "effect.look";
export const ADJUST = "effect.color";
export const STABILIZE = "effect.stabilize";

/** Grid points per side of the colour table (33³ is the usual .cube size). */
export const LUT_SIZE = 33;

// ---- Adjust ----

export type AdjustKey = "exposure" | "contrast" | "saturation" | "temperature" | "tint" | "highlights" | "shadows" | "vibrance";
export type Adjust = Readonly<Record<AdjustKey, number>>;

export interface AdjustInfo {
  readonly key: AdjustKey;
  readonly name: string;
  readonly min: number;
  readonly max: number;
}

/** The Inspector's Adjust rows, in order. Exposure is in stops; the rest −1 to 1 (shown as −100 to 100). */
export const ADJUST_INFO: ReadonlyArray<AdjustInfo> = [
  { key: "exposure", name: "Exposure", min: -2, max: 2 },
  { key: "contrast", name: "Contrast", min: -1, max: 1 },
  { key: "saturation", name: "Saturation", min: -1, max: 1 },
  { key: "temperature", name: "Temperature", min: -1, max: 1 },
  { key: "tint", name: "Tint", min: -1, max: 1 },
  { key: "highlights", name: "Highlights", min: -1, max: 1 },
  { key: "shadows", name: "Shadows", min: -1, max: 1 },
  { key: "vibrance", name: "Vibrance", min: -1, max: 1 },
];
export const ADJUST_KEYS: ReadonlyArray<AdjustKey> = ADJUST_INFO.map((a) => a.key);
export const NO_ADJUST: Adjust = { exposure: 0, contrast: 0, saturation: 0, temperature: 0, tint: 0, highlights: 0, shadows: 0, vibrance: 0 };

export const clamp01 = (v: number) => (v <= 0 ? 0 : v >= 1 ? 1 : v);
export const num = (v: unknown, min: number, max: number, fallback: number) => (typeof v === "number" && Number.isFinite(v) ? Math.min(max, Math.max(min, v)) : fallback);

/** An Adjust from params (missing or bad values are 0, out of range clamped). */
export function adjustOf(params: unknown): Adjust {
  const p = (params && typeof params === "object" ? params : {}) as Record<string, unknown>;
  const out = { ...NO_ADJUST } as Record<AdjustKey, number>;
  for (const a of ADJUST_INFO) out[a.key] = num(p[a.key], a.min, a.max, 0);
  return out;
}
export const isNeutral = (a: Adjust) => ADJUST_KEYS.every((k) => a[k] === 0);

// ---- looks ----

export type Rgb3 = readonly [number, number, number];
/** A tone curve: points (x, y) in 0–1, x rising; a smooth monotone curve through them. */
export type Curve = ReadonlyArray<readonly [number, number]>;

/**
 * A look as data (so a plugin can contribute one, D136): applied in this order — channel mixer
 * (rows give r, g, b from r, g, b), mono (mix to luma), Adjust, the master curve then the per-channel
 * curves, split toning (a colour added in the shadows and in the highlights), fade (blacks lifted).
 */
export interface LookDef {
  readonly mixer?: ReadonlyArray<number>;
  readonly mono?: number;
  readonly adjust?: Partial<Record<AdjustKey, number>>;
  readonly curve?: Curve;
  readonly red?: Curve;
  readonly green?: Curve;
  readonly blue?: Curve;
  readonly shadowTint?: { readonly color: Rgb3; readonly amount: number };
  readonly highlightTint?: { readonly color: Rgb3; readonly amount: number };
  readonly fade?: number;
}

export interface LookInfo {
  /** The value of effect.look's `look` param. */
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly def: LookDef;
}

const S_CURVE: Curve = [
  [0, 0],
  [0.25, 0.2],
  [0.5, 0.5],
  [0.75, 0.8],
  [1, 1],
];

/** The built-in looks, in the panel's order. */
export const LOOKS: ReadonlyArray<LookInfo> = [
  { id: "natural", name: "Natural", description: "A touch more contrast and colour, nothing that shows", def: { adjust: { contrast: 0.06, vibrance: 0.3, saturation: 0.04 }, curve: [[0, 0], [0.25, 0.235], [0.75, 0.775], [1, 1]] } },
  { id: "warm", name: "Warm", description: "Golden, sunlit tones", def: { adjust: { temperature: 0.45, tint: -0.04, saturation: 0.06 } } },
  { id: "cool", name: "Cool", description: "Blue, crisp tones", def: { adjust: { temperature: -0.45, tint: 0.03 } } },
  { id: "vintage", name: "Vintage", description: "Faded warm print with soft blacks", def: { mono: 0.25, adjust: { temperature: 0.3, contrast: -0.1 }, blue: [[0, 0.12], [1, 0.88]], red: [[0, 0.04], [0.5, 0.54], [1, 1]], fade: 0.06 } },
  { id: "film", name: "Film", description: "Gentle S-curve, teal shadows, warm highlights", def: { curve: [[0, 0.04], [0.25, 0.22], [0.5, 0.5], [0.75, 0.8], [1, 0.97]], shadowTint: { color: [0.1, 0.45, 0.5], amount: 0.12 }, highlightTint: { color: [1, 0.75, 0.5], amount: 0.1 }, adjust: { saturation: -0.08 } } },
  { id: "teal-orange", name: "Teal & Orange", description: "Teal shadows and orange highlights, the blockbuster look", def: { shadowTint: { color: [0, 0.55, 0.65], amount: 0.28 }, highlightTint: { color: [1, 0.6, 0.25], amount: 0.24 }, adjust: { contrast: 0.12, saturation: 0.1 } } },
  { id: "bw", name: "B&W", description: "Black and white", def: { mono: 1, adjust: { contrast: 0.1 } } },
  { id: "noir", name: "Noir", description: "Hard black and white with deep shadows", def: { mono: 1, curve: [[0, 0], [0.3, 0.14], [0.7, 0.88], [1, 1]], adjust: { exposure: -0.2 } } },
  { id: "fade", name: "Fade", description: "Lifted blacks and quiet colour", def: { fade: 0.14, adjust: { saturation: -0.25, contrast: -0.15 } } },
  { id: "vivid", name: "Vivid", description: "Bold, saturated colour", def: { adjust: { saturation: 0.35, vibrance: 0.3, contrast: 0.15 } } },
  { id: "moody", name: "Moody", description: "Darker, cooler, less colour", def: { adjust: { exposure: -0.25, saturation: -0.2, contrast: 0.1, temperature: -0.15 }, shadowTint: { color: [0.1, 0.2, 0.45], amount: 0.15 } } },
  { id: "sepia", name: "Sepia", description: "Old photograph brown", def: { mixer: [0.393, 0.769, 0.189, 0.349, 0.686, 0.168, 0.272, 0.534, 0.131], adjust: { contrast: 0.05 } } },
  { id: "cinematic", name: "Cinematic", description: "Crushed contrast with a teal and amber split", def: { curve: S_CURVE, shadowTint: { color: [0, 0.4, 0.5], amount: 0.16 }, highlightTint: { color: [1, 0.7, 0.35], amount: 0.14 }, fade: 0.03, adjust: { saturation: -0.05 } } },
  { id: "pastel", name: "Pastel", description: "Soft, bright and light", def: { adjust: { contrast: -0.25, saturation: -0.15, exposure: 0.2 }, fade: 0.1 } },
  { id: "golden", name: "Golden hour", description: "Late sun, warm highlights", def: { adjust: { temperature: 0.65, saturation: 0.1 }, highlightTint: { color: [1, 0.7, 0.3], amount: 0.18 } } },
  { id: "arctic", name: "Arctic", description: "Icy blue and bright", def: { adjust: { temperature: -0.6, exposure: 0.1, saturation: -0.1, highlights: 0.15 } } },
];

export const lookInfo = (id: string | undefined) => LOOKS.find((l) => l.id === id);

// ---- effects ----

export interface EffectParam {
  readonly key: string;
  readonly name: string;
  readonly min: number;
  readonly max: number;
  readonly step: number;
  readonly default: number;
  readonly unit?: string;
}

export type EffectGroup = "picture" | "overlay" | "motion";

export interface EffectInfo {
  /** Its module id (spec/modules/builtin/<id>.json). */
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly group: EffectGroup;
  readonly params: ReadonlyArray<EffectParam>;
}

/** The built-in effects, in the panel's order. Lengths are canvas pixels. */
export const EFFECTS: ReadonlyArray<EffectInfo> = [
  { id: "effect.blur", name: "Blur", description: "Soft focus over the whole picture", group: "picture", params: [{ key: "amount", name: "Amount", min: 1, max: 60, step: 1, default: 8, unit: "px" }] },
  { id: "effect.sharpen", name: "Sharpen", description: "Crisper edges", group: "picture", params: [{ key: "amount", name: "Amount", min: 0.1, max: 2, step: 0.05, default: 0.6 }] },
  { id: "effect.vignette", name: "Vignette", description: "Darker corners", group: "overlay", params: [{ key: "amount", name: "Amount", min: 0, max: 1, step: 0.01, default: 0.5 }, { key: "size", name: "Size", min: 0, max: 1, step: 0.01, default: 0.5 }] },
  { id: "effect.grain", name: "Film grain", description: "Moving film grain", group: "overlay", params: [{ key: "amount", name: "Amount", min: 0, max: 1, step: 0.01, default: 0.35 }] },
  { id: "effect.glow", name: "Glow", description: "Bright parts bloom", group: "picture", params: [{ key: "amount", name: "Amount", min: 0, max: 1, step: 0.01, default: 0.5 }, { key: "threshold", name: "Threshold", min: 0, max: 0.95, step: 0.01, default: 0.65 }, { key: "radius", name: "Radius", min: 2, max: 60, step: 1, default: 16, unit: "px" }] },
  { id: "effect.rgb-split", name: "RGB split", description: "Red and blue pulled apart", group: "picture", params: [{ key: "amount", name: "Amount", min: 1, max: 40, step: 1, default: 6, unit: "px" }] },
  { id: "effect.pixelate", name: "Pixelate", description: "Big square pixels", group: "picture", params: [{ key: "size", name: "Size", min: 2, max: 80, step: 1, default: 16, unit: "px" }] },
  { id: "effect.mirror", name: "Mirror", description: "Flipped left to right", group: "picture", params: [] },
  { id: "effect.posterize", name: "Posterize", description: "Few flat colour bands", group: "picture", params: [{ key: "levels", name: "Levels", min: 2, max: 16, step: 1, default: 5 }] },
  { id: "effect.invert", name: "Invert", description: "Negative colours", group: "picture", params: [] },
  { id: "effect.shake", name: "Shake", description: "Handheld camera shake", group: "picture", params: [{ key: "amount", name: "Amount", min: 1, max: 80, step: 1, default: 12, unit: "px" }, { key: "speed", name: "Speed", min: 0.5, max: 8, step: 0.1, default: 3, unit: "Hz" }] },
  { id: "effect.zoom-pulse", name: "Zoom pulse", description: "Zooms in and out on a beat", group: "motion", params: [{ key: "amount", name: "Amount", min: 0.02, max: 0.4, step: 0.01, default: 0.08 }, { key: "period", name: "Period", min: 0.25, max: 4, step: 0.05, default: 1, unit: "s" }] },
  { id: "effect.letterbox", name: "Letterbox", description: "Black cinema bars", group: "overlay", params: [{ key: "ratio", name: "Ratio", min: 1.6, max: 3, step: 0.01, default: 2.39 }] },
];

export const effectInfo = (id: string | undefined) => EFFECTS.find((e) => e.id === id);
/** Every module id a clip's effects can draw (LOOK, ADJUST and EFFECTS). */
export const DRAWN_FX: ReadonlySet<string> = new Set([LOOK, ADJUST, ...EFFECTS.map((e) => e.id)]);

/** An effect's params with its defaults filled in and every value clamped. */
export function effectParams(id: string, params: unknown): Record<string, number> {
  const p = (params && typeof params === "object" ? params : {}) as Record<string, unknown>;
  const out: Record<string, number> = {};
  for (const d of effectInfo(id)?.params ?? []) out[d.key] = num(p[d.key], d.min, d.max, d.default);
  return out;
}
