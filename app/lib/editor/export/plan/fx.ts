// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type ColourLut, shakeZoom, shakeExpr } from "@/app/lib/editor/core";
import type { ExportPlan, LayerFx } from "./types.ts";
import { n } from "./num.ts";
import { blurFilters, BLUR_STEPS } from "./layers.ts";

/** The colour tables (D136) a plan's layers use, each once: what the exporter writes as .cube files. */
export const planLuts = (plan: Pick<ExportPlan, "visuals">): Array<ColourLut> => [...new Map(plan.visuals.flatMap((l) => (l.kind === "video" || l.kind === "image") && l.fx?.lut ? [[l.fx.lut.key, l.fx.lut] as const] : [])).values()];

/** A file path as an ffmpeg filter argument: only plain path characters (the exporter's temp files), so nothing needs escaping. */
const filterPath = (path: string) => {
  if (!/^[\w/.\-]+$/.test(path)) throw new Error(`can't pass ${path} to ffmpeg's lut3d`);
  return path;
};

/**
 * Graph lines that run a clip's picture effects (D136) from `input` to `output`, in core's order:
 * made at the effects' size in 16-bit RGB with alpha, mirror (hflip), shake (zoomed in, then a window moved
 * frame by frame: core shakeExpr of the frame's moment, `n` counting the layer's frames from its
 * first, `f0`), pixelate (pixelize, block averages), colour (lut3d trilinear on core's table),
 * posterize (lutrgb floor), RGB split (rgbashift), sharpen (a 3×3 convolution, weights over 16),
 * glow (a thresholded copy blurred and screened on), back to 4:4:4 YUV with alpha, then blur (D133's
 * premultiplied gblur). The preview's shader (apps/web/src/preview/fx.ts) does the same.
 */
export function fxGraph(lf: LayerFx, input: string, output: string, fps: number, f0: number, luts?: ReadonlyMap<string, string>): Array<string> {
  const { fx, w, h, k } = lf;
  // 16-bit RGB with alpha: a still's transparency is carried through (every step leaves alpha as it is).
  const chain = [`scale=w=${w}:h=${h}:out_range=pc`, "format=gbrap16le"];
  if (fx.mirror) chain.push("hflip");
  if (fx.shake) {
    const z = shakeZoom(fx.shake.amount * k, w, h);
    const zw = Math.max(w, Math.round(w * z));
    const zh = Math.max(h, Math.round(h * z));
    const t = `(n+${f0 - lf.start})/${fps}`;
    const at = (base: number, d: string, max: number) => `'min(max(floor(${n(base)}+${d}+0.5),0),${max})'`;
    chain.push(`scale=w=${zw}:h=${zh}`, `crop=w=${w}:h=${h}:x=${at((zw - w) / 2, shakeExpr(fx.shake, "x", t, k), zw - w)}:y=${at((zh - h) / 2, shakeExpr(fx.shake, "y", t, k), zh - h)}:exact=1`);
  }
  if (fx.pixelate) {
    const b = Math.max(1, Math.round(fx.pixelate * k));
    if (b > 1) chain.push(`pixelize=w=${b}:h=${b}:mode=avg`);
  }
  if (lf.lut) {
    const file = luts?.get(lf.lut.key);
    if (!file) throw new Error(`no colour table for ${lf.lut.key}`);
    chain.push(`lut3d=file=${filterPath(file)}:interp=trilinear`);
  }
  if (fx.posterize) {
    const L = fx.posterize;
    const e = `floor(val*${L}/(maxval+1))*maxval/${L - 1}`;
    chain.push(`lutrgb=r='${e}':g='${e}':b='${e}'`);
  }
  if (fx.rgbSplit) {
    const d = Math.round(fx.rgbSplit * k);
    if (d > 0) chain.push(`rgbashift=rh=${d}:bh=${-d}:edge=smear`);
  }
  if (fx.sharpen) {
    const a = Math.round(fx.sharpen * 16);
    const m = `0 -${a} 0 -${a} ${16 + 4 * a} -${a} 0 -${a} 0`;
    chain.push(`convolution=0m='${m}':1m='${m}':2m='${m}':3m='0 0 0 0 1 0 0 0 0':0rdiv=0.0625:1rdiv=0.0625:2rdiv=0.0625:3rdiv=1`);
  }
  const back = ["scale=out_color_matrix=bt709:out_range=tv", "format=yuva444p", ...(fx.blur ? [blurFilters(fx.blur * k)] : [])];
  if (!fx.glow) return [`[${input}]${[...chain, ...back].join(",")}[${output}]`];
  const g = fx.glow;
  const T = `${n(g.threshold)}*maxval`;
  const e = `clip((val-${T})/(maxval-${T}),0,1)*${n(g.amount)}*maxval`;
  const sigma = Math.max(0.5, g.radius * k);
  return [
    `[${input}]${chain.join(",")},split=2[${output}ga][${output}gb]`,
    `[${output}gb]lutrgb=r='${e}':g='${e}':b='${e}',gblur=sigma=${n(sigma)}:sigmaV=${n(sigma)}:steps=${BLUR_STEPS}[${output}gc]`,
    `[${output}ga][${output}gc]blend=all_mode=screen:c3_expr=A,${back.join(",")}[${output}]`,
  ];
}
