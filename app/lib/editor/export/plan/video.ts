// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import {applyLayerMasks} from './masks';
import type { Ramp, VisualLayer, Quality, Placement } from "./types.ts";
import { n } from "./num.ts";
import type { RasterOutput, Caps, VideoChoice } from "./codec.ts";
import type { AudioOptions } from "./audio.ts";

export const fadeFilters = (ramps: ReadonlyArray<Ramp>, fps: number) => ramps.map((r) => `fade=t=${r.type}:st=${n(Math.max(0, r.st) / fps)}:d=${n(r.d / fps)}:alpha=1`);

/**
 * For a video without colour tags (its height given), filters that make ffmpeg read it as the
 * preview shows it: The preview assumes BT.601 with SMPTE-C primaries below 720 lines (so the
 * primaries are converted to BT.709's) and BT.709 from 720 lines up. Tagged video: nothing.
 */
export const untaggedFilter = (height: number | undefined) =>
  height === undefined
    ? ""
    : height < 720
      ? "setparams=colorspace=smpte170m:color_primaries=smpte170m:color_trc=smpte170m:range=tv,colorspace=all=bt709:format=yuv420p,"
      : "setparams=colorspace=bt709:color_primaries=bt709:color_trc=bt709:range=tv,";

export interface CommandOptions {
  /** Video files without colour tags, with their height (see untaggedFilter). */
  readonly untagged?: ReadonlyMap<string, number> | undefined;
  readonly background: string;
  readonly rasters: ReadonlyMap<VisualLayer, RasterOutput>;
  readonly caps: Caps;
  readonly quality: Quality;
  /** F4 (D171): the codec and bitrate when not the preset's, and the output frame rate when not the plan's. */
  readonly video?: (VideoChoice & { readonly fps?: number | undefined }) | undefined;
  /** Mix processing shared with the loudness measurement passes. */
  readonly audio?: AudioOptions | undefined;
  /** A completed range mix, including its gain and effects, prepared before video encoding. */
  readonly preparedAudio?: string | undefined;
  readonly output: string;
  /** The colour tables (D136) by key (core colourLut): .cube files the exporter wrote. */
  readonly luts?: ReadonlyMap<string, string> | undefined;
  /** Shaded layers' pictures (D213): the file of the frames the rasteriser drew, by layer. */
  readonly shaded?: ReadonlyMap<VisualLayer, string> | undefined;
  /** A soft subtitle track (D105): an .srt file muxed as mov_text with an ISO 639-2 language tag. */
  readonly subtitles?: { readonly file: string; readonly language: string; /** Shown by default (when the captions aren't burned in). */ readonly show?: boolean | undefined } | undefined;
}

/**
 * One frame of a video file as the preview and the export show it at `source` seconds (D38): the
 * last source frame that starts at or before that moment. Seek up to a second early, shift the
 * timestamps so the moment is 0, and let fps (round=up) pick, then keep the first frame. Shared by
 * render_frames and contact sheets of media (D65); the export's own chain does the same per layer.
 */
export function sourceFramePick(source: number, fps: number, hold = false): { readonly seekArgs: ReadonlyArray<string>; readonly filter: string } {
  // A held clip (D94) seeks up to 2 s early and clones its last decoded frame, so the frame at or past
  // the stream's end (even when the probe overstates its length) is the last one, as the export shows.
  const pre = Math.min(hold ? 2 : 1, Math.max(0, source));
  return {
    seekArgs: ["-ss", n(source - pre), "-t", n(pre + 0.5)],
    filter: `${hold ? `tpad=stop_mode=clone:stop_duration=${n(pre + 1)},` : ""}setpts=PTS-${n(pre + 0.0001)}/TB,fps=${fps}:start_time=0:round=up,trim=end_frame=1,setpts=0`,
  };
}

// ---- boxes (D75) ----

/**
 * An ffmpeg expression for a per-frame value: values[i] when `v` (evaluated once, kept in slot
 * `slot`) is i; before 0 the first, past the end the last. Runs of one value collapse, and the
 * runs are searched as a balanced tree, so the expression stays shallow for long keyed stretches.
 */
export function frameLookup(values: ReadonlyArray<number>, v: string, slot = 9): string {
  const runs: Array<{ start: number; value: number }> = [];
  values.forEach((value, i) => {
    if (!runs.length || runs.at(-1)!.value !== value) runs.push({ start: i, value });
  });
  if (runs.length <= 1) return n(runs[0]?.value ?? 0);
  const tree = (lo: number, hi: number): string => {
    if (lo === hi) return n(runs[lo]!.value);
    const mid = (lo + hi + 1) >> 1;
    return `if(lt(ld(${slot}),${runs[mid]!.start}),${tree(lo, mid - 1)},${tree(mid, hi)})`;
  };
  return `st(${slot},${v});${tree(0, runs.length - 1)}`;
}

export const same = (values: ReadonlyArray<number>) => values.every((x) => x === values[0]);

/**
 * A rounded-corner mask for a w×h box (one grey frame: 255 inside, the corners' edges
 * antialiased like the preview's border-radius), as a source chain ending in `label`.
 */
export const cornerMask = (w: number, h: number, r: number, fps: number, label: string) =>
  `color=c=black:s=${w}x${h}:r=${fps},trim=end_frame=1,format=gray,geq=lum='255*clip(${r}+0.5-hypot(max(max(${r}-X-0.5,X+0.5-${w - r}),0),max(max(${r}-Y-0.5,Y+0.5-${h - r}),0)),0,1)'[${label}]`;

/**
 * Graph lines that turn the picture at `input` into its box at `output` (D75): scaled to the
 * picture's size (per frame when the scale is keyed: `n` counts the layer's frames), padded and/or
 * cropped to the box (transparent where a contained picture doesn't reach), and its corners
 * rounded with a mask (multiplied into any alpha the picture has). The box's size never changes.
 */
export function boxGraph(frames: ReadonlyArray<Placement>, input: string, output: string, fps: number): Array<string> {
  const p0 = frames[0]!;
  const { w, h, radius } = p0;
  const cws = frames.map((p) => p.cw);
  const chs = frames.map((p) => p.ch);
  const fixed = same(cws) && same(chs);
  // The layer's frames are numbered from 0 at its first (pts on the 1/fps grid) when this runs.
  const local = `floor(t*${fps}+0.5)`;
  const scale = fixed ? `scale=w=${p0.cw}:h=${p0.ch}` : `scale=w='${frameLookup(cws, local)}':h='${frameLookup(chs, local)}':eval=frame`;
  // 4:4:4 (no chroma subsampling), so odd sizes and offsets are exact; converted where it's overlaid.
  const chain = [`${scale}:out_color_matrix=bt709:out_range=tv`, "format=yuva444p"];
  // The area holding both the picture and the box, relative to the box's top-left.
  const minX = Math.min(0, ...frames.map((p) => p.cx));
  const minY = Math.min(0, ...frames.map((p) => p.cy));
  const pw = Math.max(w, ...frames.map((p) => p.cx + p.cw)) - minX;
  const ph = Math.max(h, ...frames.map((p) => p.cy + p.ch)) - minY;
  if (fixed) {
    if (pw > p0.cw || ph > p0.ch) chain.push(`pad=w=${pw}:h=${ph}:x=${p0.cx - minX}:y=${p0.cy - minY}:color=black@0`);
    if (pw > w || ph > h) chain.push(`crop=w=${w}:h=${h}:x=${-minX}:y=${-minY}:exact=1`);
  } else {
    // A keyed zoom stays centred on the box: pad around the picture's centre, crop the box out.
    // Same parity as the box, so the crop sits exactly on the centre.
    const PW = Math.max(w, ...cws) + 2 + ((Math.max(w, ...cws) - w) % 2);
    const PH = Math.max(h, ...chs) + 2 + ((Math.max(h, ...chs) - h) % 2);
    chain.push(`pad=w=${PW}:h=${PH}:x='floor((ow-iw)/2)':y='floor((oh-ih)/2)':color=black@0:eval=frame`, `crop=w=${w}:h=${h}:x=${Math.floor((PW - w) / 2)}:y=${Math.floor((PH - h) / 2)}:exact=1`);
  }
  if (radius <= 0) return [`[${input}]${chain.join(",")}[${output}]`];
  const id = output;
  return [
    `[${input}]${chain.join(",")},split=2[${id}c][${id}a]`,
    cornerMask(w, h, radius, fps, `${id}m`),
    `[${id}a]alphaextract[${id}e]`,
    `[${id}e][${id}m]blend=all_mode=multiply[${id}k]`,
    `[${id}c][${id}k]alphamerge[${output}]`,
  ];
}

/** The box's opacity as filters on the alpha plane: a constant, or a value per frame (sendcmd). */
export function alphaFilters(layer: VisualLayer, k: number, fps: number): Array<string> {
  if (!layer.alpha) return [];
  if (same(layer.alpha)) return [`lutyuv=a='val*${n(layer.alpha[0]!)}'`];
  const commands: Array<string> = [];
  layer.alpha.forEach((a, i) => {
    if (i > 0 && a === layer.alpha![i - 1]) return;
    commands.push(`${n(Math.max(0, layer.f0 + i - 0.5) / fps)} lutyuv@a${k} a val*${n(a)}`);
  });
  return [`sendcmd=c='${commands.join(";")}'`, `lutyuv@a${k}=a='val*${n(layer.alpha[0]!)}'`];
}

/** Where a boxed layer goes: overlay x and y, constant or per frame (from `t`, the frame time). */
export function placeAt(layer: VisualLayer, frames: ReadonlyArray<Placement>, fps: number): { x: string; y: string } {
  const xs = frames.map((p) => p.x);
  const ys = frames.map((p) => p.y);
  const frame = `floor(t*${fps}+0.5)-${layer.f0}`;
  return { x: same(xs) ? String(xs[0]) : `'${frameLookup(xs, frame)}'`, y: same(ys) ? String(ys[0]) : `'${frameLookup(ys, frame)}'` };
}

/** The ffmpeg graph and arguments for the whole export (the graph goes in a file: -/filter_complex). */
/**
 * A video layer's picture as the export decodes it (D38, D94, D75 review): its input arguments and
 * the filters from `[index:v:0]` to its frames on the layer's 1/fps grid (frames from f0). Shared
 * with the shader pass (D213), which decodes the same frames for the rasteriser.
 */
export function videoSource(layer: Extract<VisualLayer, { kind: "video" }>, index: number, opts: Pick<CommandOptions, "caps" | "untagged">, W: number, H: number, fps: number): { readonly inputs: Array<string>; readonly picked: Array<string> } {
  const frames = layer.f1 - layer.f0;
  const geo = layer.geometry?.frames;
  // Seek a second early and let fps pick, for each frame, the last source frame that starts at
  // or before its source time (round=up), like the preview's <video> (D38).
  // A held clip (D94) seeks up to 2 s early, so a probe that overstates the stream still leaves frames to clone.
  const pre = layer.cycle ? 0 : Math.min(layer.last !== undefined ? 2 : 1, layer.source);
  const length = layer.cycle ? layer.cycle.length / fps + 1 : pre + (frames / fps) * layer.speed + 1;
  const inputs = [...(layer.cycle ? [] : ["-ss", n(layer.source - pre)]), "-t", n(length), "-i", layer.file];
  const decoded = `[${index}:v:0]${untaggedFilter(opts.untagged?.get(layer.file))}setpts=(PTS-${n(pre + 0.0001)}/TB)/${n(layer.speed)}`;
  // A held clip (D94) shows its media's last frame for the rest of its length: the decoded frames
  // are padded with clones of the last one (before fps picks, so even a slice that starts at or
  // past the stream's end has a frame to show).
  const hold = layer.last !== undefined && !layer.cycle ? [`tpad=stop_mode=clone:stop_duration=${n(pre + (frames / fps) * layer.speed + 1)}`] : [];
  let picked = [decoded, ...hold, `fps=${fps}:start_time=0:round=up`, `trim=end_frame=${frames}`];
  if (layer.cycle) {
    // The media's frames once, made only as large as they're drawn, then repeated from `offset`.
    const { length: L, offset } = layer.cycle;
    const shrink = layer.masks ? `scale=w=${layer.masks.width}:h=${layer.masks.height}` : geo
      ? `scale=w=${Math.max(...geo.map((p) => p.cw))}:h=${Math.max(...geo.map((p) => p.ch))}`
      : `scale=w=${W}:h=${H}:force_original_aspect_ratio=decrease:force_divisible_by=2`;
    picked = [decoded, `fps=${fps}:start_time=0:round=up`, `trim=end_frame=${L}`, `${shrink}:out_color_matrix=bt709:out_range=tv`, `loop=loop=-1:size=${L}:start=0`, `trim=start_frame=${offset}:end_frame=${offset + frames}`, `settb=1/${fps}`, "setpts=N"];
  }
  return { inputs, picked };
}

/**
 * The ffmpeg arguments that decode a shaded layer's picture (D213) as the export would show it,
 * made at the effects' size (fxGraph's first step, in 8-bit RGBA), raw to stdout: one
 * w × h × 4 byte frame for each of the layer's frames.
 */
export function shaderSourceArgs(layer: VisualLayer, opts: Pick<CommandOptions, "caps" | "untagged">, W: number, H: number, fps: number): Array<string> {
  if ((layer.kind !== "video" && layer.kind !== "image") || !layer.fx) throw new Error(`${layer.item.id} has no picture to shade`);
  const frames = layer.f1 - layer.f0;
  const size = `scale=w=${layer.fx.w}:h=${layer.fx.h}:out_range=pc,format=rgba`;
  let inputs: Array<string>;
  let chain: Array<string>;
  const graph:string[]=[];
  if (layer.kind === "video") {
    const source = videoSource(layer, 0, opts, W, H, fps);
    inputs = source.inputs;
    chain = source.picked;
  } else {
    inputs = ["-f", "image2", "-pattern_type", "none", "-i", layer.file];
    chain = [`[0:v:0]loop=loop=${frames - 1}:size=1:start=0`, `settb=1/${fps}`, "setpts=N"];
  }
  chain=applyLayerMasks(layer,chain,inputs,graph,'masked',fps);
  graph.push(`${[...chain,size].join(',')}[out]`);
  return ["-hide_banner", "-nostdin", "-v", "error", ...inputs, "-filter_complex", graph.join(';'), "-map", "[out]", "-frames:v", String(frames), "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"];
}
