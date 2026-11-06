// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { Item, ModuleUse } from "@/app/lib/editor/core/timeline/types";
import { type ClipBox, fitRect, moveBox, type Rect } from "@/app/lib/editor/core/visual/box";
import type { ColourSpec } from "@/app/lib/editor/core/visual/fx/colour";
import { type LookDef, lookInfo, type Adjust, LOOK, num, ADJUST, adjustOf, isNeutral, effectInfo, effectParams } from "@/app/lib/editor/core/visual/fx/defs";

// ---- a clip's effects, compiled ----

/** What a clip's effects draw, in the fixed order (see the header). Lengths in canvas px. */
export interface ClipFx {
  readonly colour?: ColourSpec;
  readonly posterize?: number;
  readonly mirror?: boolean;
  readonly shake?: { readonly amount: number; readonly speed: number };
  readonly pixelate?: number;
  readonly rgbSplit?: number;
  /** The sharpen kernel's side weight (in 1/16 steps: the export's convolution takes whole numbers over 16). */
  readonly sharpen?: number;
  readonly glow?: { readonly amount: number; readonly threshold: number; readonly radius: number };
  readonly blur?: number;
  readonly pulse?: { readonly amount: number; readonly period: number };
  readonly vignette?: { readonly amount: number; readonly size: number };
  readonly grain?: { readonly amount: number };
  readonly letterbox?: { readonly ratio: number };
}

const live = (item: Pick<Item, "effects">): ReadonlyArray<ModuleUse> => (Array.isArray(item.effects) ? item.effects.filter((e) => e && typeof e.module === "string" && !e.bypass) : []);

/** A look entry's definition: a built-in one, or the custom look it carries (a plugin's, D136). */
export function lookDefOf(params: unknown): LookDef | undefined {
  const p = (params && typeof params === "object" ? params : {}) as Record<string, unknown>;
  if (p.custom && typeof p.custom === "object") return p.custom as LookDef;
  return lookInfo(typeof p.look === "string" ? p.look : undefined)?.def;
}

/** Media looks and effects aren't drawn on (D138): SVG stills, which Chromium draws at their box's size in the export. */
export const fxSkips = (media: { readonly kind: string; readonly path?: string } | undefined) => !!media && media.kind === "image" && /\.svg$/i.test(media.path ?? "");

/** A clip's effects compiled to what the preview and export draw; undefined when nothing is drawn. */
export function clipFx(item: Pick<Item, "type" | "effects">): ClipFx | undefined {
  if (item.type !== "clip") return undefined;
  const entries = live(item);
  if (!entries.length) return undefined;
  const fx: { -readonly [K in keyof ClipFx]: ClipFx[K] } = {};
  const colour: { look?: ColourSpec["look"]; adjust?: Adjust; invert?: boolean } = {};
  for (const e of entries) {
    if (e.module === LOOK) {
      const def = lookDefOf(e.params);
      const p = (e.params ?? {}) as Record<string, unknown>;
      const intensity = num(p.intensity, 0, 1, 1);
      if (def && intensity > 0) colour.look = { def, intensity };
      continue;
    }
    if (e.module === ADJUST) {
      const a = adjustOf(e.params);
      if (!isNeutral(a)) colour.adjust = a;
      continue;
    }
    if (!effectInfo(e.module)) continue;
    const p = effectParams(e.module, e.params);
    switch (e.module) {
      case "effect.blur":
        fx.blur = p.amount!;
        break;
      case "effect.sharpen":
        fx.sharpen = Math.max(1, Math.round(p.amount! * 16)) / 16;
        break;
      case "effect.vignette":
        if (p.amount! > 0) fx.vignette = { amount: p.amount!, size: p.size! };
        break;
      case "effect.grain":
        if (p.amount! > 0) fx.grain = { amount: p.amount! };
        break;
      case "effect.glow":
        if (p.amount! > 0) fx.glow = { amount: p.amount!, threshold: p.threshold!, radius: p.radius! };
        break;
      case "effect.rgb-split":
        fx.rgbSplit = Math.round(p.amount!);
        break;
      case "effect.pixelate":
        fx.pixelate = Math.round(p.size!);
        break;
      case "effect.mirror":
        fx.mirror = true;
        break;
      case "effect.posterize":
        fx.posterize = Math.round(p.levels!);
        break;
      case "effect.invert":
        colour.invert = true;
        break;
      case "effect.shake":
        fx.shake = { amount: p.amount!, speed: p.speed! };
        break;
      case "effect.zoom-pulse":
        fx.pulse = { amount: p.amount!, period: p.period! };
        break;
      case "effect.letterbox":
        fx.letterbox = { ratio: p.ratio! };
        break;
    }
  }
  if (colour.look || colour.adjust || colour.invert) fx.colour = colour;
  return Object.keys(fx).length ? fx : undefined;
}

/** Whether the effects change the picture itself (the WebGL pass / the export's filters), not only overlays or the box. */
export const needsPicturePass = (fx: ClipFx | undefined) => !!fx && !!(fx.colour || fx.posterize || fx.mirror || fx.shake || fx.pixelate || fx.rgbSplit || fx.sharpen || fx.glow);
/** Whether anything is drawn over the picture (vignette, grain, letterbox). */
export const hasOverlay = (fx: ClipFx | undefined) => !!fx && !!(fx.vignette || fx.grain || fx.letterbox);

/**
 * The size (canvas px, whole) the effects work at: the picture at its fit in the box, before any
 * zoom (scale, keys, a pulse or a transition zoom it afterwards, effects and all).
 */
export function fxPictureSize(box: Pick<ClipBox, "x" | "y" | "w" | "h" | "fit">, media?: { width?: number | undefined; height?: number | undefined }): { w: number; h: number } {
  const r = fitRect(box, box.fit, media?.width && media?.height ? { width: media.width, height: media.height } : undefined);
  return { w: Math.max(1, Math.round(r.w)), h: Math.max(1, Math.round(r.h)) };
}

// ---- motion ----

/** How much a shake zooms the picture in so its edges never show: the offset at most `amount` px each way. */
export const shakeZoom = (amount: number, w: number, h: number) => 1 + (2.1 * amount) / Math.max(1, Math.min(w, h));

/** A shake's offset at a moment (s from the clip's start), px each way; |offset| ≤ amount. */
export function shakeAt(shake: NonNullable<ClipFx["shake"]>, t: number): { dx: number; dy: number } {
  const w = 2 * Math.PI * shake.speed * t;
  return {
    dx: shake.amount * (0.6 * Math.sin(w + 0.3) + 0.4 * Math.sin(2.37 * w + 1.7)),
    dy: shake.amount * (0.6 * Math.sin(1.13 * w + 2.1) + 0.4 * Math.sin(2.91 * w + 0.4)),
  };
}

/**
 * The same offset as an ffmpeg expression of `t` (a frame's moment from the clip's start, an
 * expression), px times `k` (output px per canvas px): what the export's crop moves by.
 */
export function shakeExpr(shake: NonNullable<ClipFx["shake"]>, axis: "x" | "y", t: string, k: number): string {
  const a = (shake.amount * k).toFixed(6);
  const w = `(${(2 * Math.PI * shake.speed).toFixed(9)}*(${t}))`;
  return axis === "x" ? `${a}*(0.6*sin(${w}+0.3)+0.4*sin(2.37*${w}+1.7))` : `${a}*(0.6*sin(1.13*${w}+2.1)+0.4*sin(2.91*${w}+0.4))`;
}

/**
 * Where the shaken picture's window sits in the zoomed picture (zw × zh, whole px) at a moment:
 * centred plus the offset, rounded (floor(x + 0.5), as the export's crop) and kept inside.
 */
export function shakeWindow(shake: NonNullable<ClipFx["shake"]>, t: number, w: number, h: number, zw: number, zh: number, k = 1): { x: number; y: number } {
  const o = shakeAt(shake, t);
  const at = (base: number, d: number, max: number) => Math.min(max, Math.max(0, Math.floor(base + d * k + 0.5)));
  return { x: at((zw - w) / 2, o.dx, zw - w), y: at((zh - h) / 2, o.dy, zh - h) };
}

/** The zoom a pulse adds at a moment (s from the clip's start): 1 at the start, 1 + amount half a period later. */
export const pulseZoom = (pulse: NonNullable<ClipFx["pulse"]>, t: number) => 1 + pulse.amount * (0.5 - 0.5 * Math.cos((2 * Math.PI * t) / pulse.period));

/** A clip's box with its zoom pulse (D136) at `local` frames from its start (the same box without one). */
export function fxBox(item: Pick<Item, "type" | "effects">, box: ClipBox, local: number, fps: number): ClipBox {
  if (item.type !== "clip" || !item.effects?.length) return box;
  const fx = clipFx(item);
  if (!fx?.pulse) return box;
  return moveBox(box, 0, pulseZoom(fx.pulse, local / fps));
}

// ---- overlays ----

const css = (n: number) => `${Math.round(n * 1000) / 1000}`;

/** The part of the box the picture covers (box-relative px): where vignette, grain and bars go. */
export function visibleRect(box: ClipBox): Rect {
  const x0 = Math.max(box.x, box.content.x);
  const y0 = Math.max(box.y, box.content.y);
  const x1 = Math.min(box.x + box.w, box.content.x + box.content.w);
  const y1 = Math.min(box.y + box.h, box.content.y + box.content.h);
  return { x: x0 - box.x, y: y0 - box.y, w: Math.max(0, x1 - x0), h: Math.max(0, y1 - y0) };
}

/**
 * Film grain: grey fractal noise, inline SVG (not a data: image, which the editor's content policy
 * doesn't load), one filter id per pattern so several clips share them.
 */
const grainSvg = (seed: number, opacity: number) =>
  `<svg width="100%" height="100%" style="position:absolute;inset:0;opacity:${css(opacity)}" aria-hidden="true"><filter id="biliup-grain-${seed}" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.65" numOctaves="2" seed="${seed}"/><feColorMatrix type="saturate" values="0"/></filter><rect width="100%" height="100%" filter="url(#biliup-grain-${seed})"/></svg>`;
/** Grain moves through this many patterns, a new one every GRAIN_HOLD frames. */
export const GRAIN_STATES = 6;
export const GRAIN_HOLD = 2;

/**
 * The overlays (vignette, grain, letterbox bars) as HTML positioned in the box (box-relative px,
 * over `rect`, the picture's visible part), at `local` frames from the clip's start (grain moves).
 * The preview puts it in the clip's box wrapper; the export draws it inside fxOverlayHtml's box.
 */
export function fxOverlayInner(fx: ClipFx, rect: Rect, local: number): string {
  if (!hasOverlay(fx) || rect.w <= 0 || rect.h <= 0) return "";
  const layers: Array<string> = [];
  if (fx.grain) {
    const phase = Math.floor(Math.max(0, local) / GRAIN_HOLD) % GRAIN_STATES;
    layers.push(grainSvg(phase + 1, 0.4 * fx.grain.amount));
  }
  if (fx.vignette) {
    // Dark at the corners: black rising from the inner radius out (a smoothstep in 8 stops).
    const inner = 0.15 + 0.6 * fx.vignette.size;
    const top = 0.9 * fx.vignette.amount;
    const stops = [`rgba(0,0,0,0) ${css(inner * 100)}%`];
    for (let i = 1; i <= 8; i++) {
      const r = inner + ((1 - inner) * i) / 8;
      const s = i / 8;
      stops.push(`rgba(0,0,0,${css(top * s * s * (3 - 2 * s))}) ${css(r * 100)}%`);
    }
    layers.push(`<div style="position:absolute;inset:0;background:radial-gradient(ellipse farthest-corner at 50% 50%, ${stops.join(", ")})"></div>`);
  }
  if (fx.letterbox) {
    const bar = Math.max(0, (rect.h - rect.w / fx.letterbox.ratio) / 2);
    if (bar > 0) layers.push(`<div style="position:absolute;left:0;right:0;top:0;height:${css(bar)}px;background:#000"></div><div style="position:absolute;left:0;right:0;bottom:0;height:${css(bar)}px;background:#000"></div>`);
  }
  if (!layers.length) return "";
  return `<div class="pv-fx" style="position:absolute;left:${css(rect.x)}px;top:${css(rect.y)}px;width:${css(rect.w)}px;height:${css(rect.h)}px;overflow:hidden;pointer-events:none">${layers.join("")}</div>`;
}

/** The overlays drawn on a canvas-size page for the export: the box (its corners rounded) holding fxOverlayInner. */
export function fxOverlayHtml(fx: ClipFx, box: ClipBox, local: number): string {
  const inner = fxOverlayInner(fx, visibleRect(box), local);
  if (!inner) return "<div></div>";
  return `<div style="position:absolute;left:${css(box.x)}px;top:${css(box.y)}px;width:${css(box.w)}px;height:${css(box.h)}px;overflow:hidden${box.radius > 0 ? `;border-radius:${css(box.radius)}px` : ""}">${inner}</div>`;
}
