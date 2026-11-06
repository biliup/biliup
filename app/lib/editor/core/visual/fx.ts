// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Clip looks and effects (D136, feedback G2): one definition per filter and effect, drawn by the
// preview (a WebGL pass over the clip's picture, apps/web/src/preview/fx.ts) and by the export
// (ffmpeg filters on the same picture, apps/server/src/export/exportPlan.ts) from the same numbers.
//
// A clip's `effects` (moduleUse entries, list order kept for the panel) can hold:
// - effect.look {look, intensity, custom?, label?}: a colour look (Warm, Film, B&W...). At most one.
// - effect.color {exposure, contrast, saturation, temperature, tint, highlights, shadows, vibrance}:
//   the Inspector's Adjust. At most one.
// - picture effects, on the clip's picture at its fitted size, in this fixed order whatever the
//   list order: mirror, shake, pixelate, then colour (look, adjust, invert: one 3D LUT core
//   computes), posterize, RGB split, sharpen, glow, blur (CSS blur / gblur, D133's maths).
// - overlay effects, drawn by Chromium on both sides over the picture inside the box: vignette,
//   film grain, letterbox bars (fxOverlayInner / fxOverlayHtml).
// - zoom pulse: the box's zoom, frame by frame (fxBox, through D75's keyed-scale path).
// effect.stabilize stays valid but isn't drawn.
//
// Colour: every pointwise change (look at its intensity, Adjust, invert) is a function on
// sRGB-encoded 0–1 RGB, sampled into a LUT_SIZE³ table: the preview reads it with trilinear
// interpolation in its shader, the export with lut3d interp=trilinear from a .cube of the same
// numbers, so both draw the same colours. Browser-safe and pure.
export { LOOK, ADJUST, STABILIZE, LUT_SIZE, ADJUST_INFO, ADJUST_KEYS, NO_ADJUST, adjustOf, isNeutral, LOOKS, lookInfo, EFFECTS, effectInfo, DRAWN_FX, effectParams } from "@/app/lib/editor/core/visual/fx/defs";
export type { AdjustKey, Adjust, AdjustInfo, Rgb3, Curve, LookDef, LookInfo, EffectParam, EffectGroup, EffectInfo } from "@/app/lib/editor/core/visual/fx/defs";
export { colourFn, colourLut, cubeText, lutAt, posterize } from "@/app/lib/editor/core/visual/fx/colour";
export type { ColourSpec, ColourLut } from "@/app/lib/editor/core/visual/fx/colour";
export { lookDefOf, fxSkips, clipFx, needsPicturePass, hasOverlay, fxPictureSize, shakeZoom, shakeAt, shakeExpr, shakeWindow, pulseZoom, fxBox, visibleRect, GRAIN_STATES, GRAIN_HOLD, fxOverlayInner, fxOverlayHtml } from "@/app/lib/editor/core/visual/fx/clipFx";
export type { ClipFx } from "@/app/lib/editor/core/visual/fx/clipFx";
export { planSetLook, planAdjust, adjustWord, planEffect, planRemoveEffect, planEffectBypass, fxWords } from "@/app/lib/editor/core/visual/fx/plans";
