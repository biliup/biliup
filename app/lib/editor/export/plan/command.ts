// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { maskGeq } from "@/app/lib/editor/core";
import type { ExportPlan } from "./types.ts";
import { type CommandOptions, fadeFilters, alphaFilters, boxGraph, videoSource, same, placeAt } from "./video.ts";
import { fxGraph } from "./fx.ts";
import { blurPad, BLUR_STEPS, IRIS_MASK_STEP } from "./layers.ts";
import { n } from "./num.ts";
import { audioGraph } from "./audio.ts";
import { videoCodec } from "./codec.ts";
import {applyLayerMasks} from './masks';

export function exportCommand(plan: ExportPlan, opts: CommandOptions): { inputs: Array<string>; graph: string; videoArgs: Array<string>; outputArgs: Array<string> } {
  const { width: W, height: H, fps } = plan;
  const inputs: Array<string> = ["-i", opts.background];
  const graph: Array<string> = [];
  const toYuv = (alpha: boolean) => `scale=w=${W}:h=${H}:out_color_matrix=bt709:out_range=tv,format=${alpha ? "yuva420p" : "yuv420p"}`;
  graph.push(`[0:v]${toYuv(false)},loop=loop=${plan.frames - 1}:size=1:start=0,settb=1/${fps},setpts=N[b0]`);
  let base = "b0";
  plan.visuals.forEach((layer, k) => {
    const index = inputs.filter(value=>value==='-i').length;
    const frames = layer.f1 - layer.f0;
    // Partial ramps crossing output zero have already been sampled into layer.alpha.
    const fades = fadeFilters(layer.ramps, fps);
    const opacity = alphaFilters(layer, k, fps);
    const geo = layer.geometry?.frames;
    let chain: Array<string>;
    if ((layer.kind === "video" || layer.kind === "image") && layer.fx?.shader) {
      const shaded = opts.shaded?.get(layer);
      // A shader effect (D213): the picture is the frames the rasteriser's WebGPU drew (shadeLayers,
      // at the effects' size), then the other picture effects and the box as for any clip.
      if (!shaded || !geo) throw new Error(`no shader frames for ${layer.item.id}`);
      inputs.push("-i", shaded);
      graph.push(`[${index}:v:0]trim=end_frame=${frames},settb=1/${fps},setpts=N[e${k}]`, ...fxGraph(layer.fx, `e${k}`, `p${k}`, fps, layer.f0, opts.luts), ...boxGraph(geo, `p${k}`, `q${k}`, fps));
      chain = [`[q${k}]setpts=PTS-STARTPTS+${layer.f0}`, ...fades, ...opacity];
    } else if (layer.kind === 'image' && layer.masks) {
      inputs.push('-f','image2','-pattern_type','none','-i',layer.file);
      const picked=applyLayerMasks(layer,[`[${index}:v:0]loop=loop=${frames-1}:size=1:start=0`,`settb=1/${fps}`,'setpts=N'],inputs,graph,`mask${k}`,fps);
      if(geo&&layer.fx){
        graph.push(`${picked.join(',')}[e${k}]`,...fxGraph(layer.fx,`e${k}`,`p${k}`,fps,layer.f0,opts.luts),...boxGraph(geo,`p${k}`,`q${k}`,fps));
        chain=[`[q${k}]setpts=PTS-STARTPTS+${layer.f0}`,...fades,...opacity];
      }else if(geo){
        graph.push(`${picked.join(',')}[p${k}]`,...boxGraph(geo,`p${k}`,`q${k}`,fps));
        chain=[`[q${k}]setpts=PTS-STARTPTS+${layer.f0}`,...fades,...opacity];
      }else{
        chain=[...picked,`scale=w=${W}:h=${H}:force_original_aspect_ratio=decrease:force_divisible_by=2:out_color_matrix=bt709:out_range=tv`,'format=yuva420p',...(layer.fills?[]:[`pad=${W}:${H}:(ow-iw)/2:(oh-ih)/2:color=black@0`]),`setpts=PTS-STARTPTS+${layer.f0}`,...fades,...opacity];
      }
    } else if (layer.kind === "video") {
      const source = videoSource(layer, index, opts, W, H, fps);
      inputs.push(...source.inputs);
      const picked = applyLayerMasks(layer,source.picked,inputs,graph,`mask${k}`,fps);
      if (geo && layer.fx) {
        // Picture effects (D136) on the picture, then its box.
        graph.push(`${picked.join(",")}[e${k}]`, ...fxGraph(layer.fx, `e${k}`, `p${k}`, fps, layer.f0, opts.luts), ...boxGraph(geo, `p${k}`, `q${k}`, fps));
        chain = [`[q${k}]setpts=PTS-STARTPTS+${layer.f0}`, ...fades, ...opacity];
      } else if (geo) {
        // A box (D75): the picture made into its box, then placed by the overlay.
        graph.push(`${picked.join(",")}[p${k}]`, ...boxGraph(geo, `p${k}`, `q${k}`, fps));
        chain = [`[q${k}]setpts=PTS-STARTPTS+${layer.f0}`, ...fades, ...opacity];
      } else {
        const alpha = !layer.fills || fades.length > 0 || !!layer.wipe || !!layer.iris || !!layer.mask || opacity.length > 0;
        chain = [
          ...picked,
          `scale=w=${W}:h=${H}:force_original_aspect_ratio=decrease:force_divisible_by=2:out_color_matrix=bt709:out_range=tv`,
          `format=${alpha ? "yuva420p" : "yuv420p"}`,
          ...(layer.fills ? [] : [`pad=${W}:${H}:(ow-iw)/2:(oh-ih)/2:color=black@0`]),
          `setpts=PTS-STARTPTS+${layer.f0}`,
          ...fades,
          ...opacity,
        ];
      }
    } else if (layer.kind === "image") {
      inputs.push("-f", "image2", "-pattern_type", "none", "-i", layer.file);
      const looped = [`loop=loop=${frames - 1}:size=1:start=0`, `settb=1/${fps}`];
      if (geo && layer.fx) {
        // Picture effects (D136): once on the still when nothing in them moves, else per frame.
        if (layer.fx.fx.shake) graph.push(`[${index}:v:0]${looped.join(",")},setpts=N[e${k}]`, ...fxGraph(layer.fx, `e${k}`, `p${k}`, fps, layer.f0, opts.luts), ...boxGraph(geo, `p${k}`, `q${k}`, fps));
        else graph.push(...fxGraph(layer.fx, `${index}:v:0`, `e${k}`, fps, layer.f0, opts.luts), `[e${k}]${looped.join(",")},setpts=N[p${k}]`, ...boxGraph(geo, `p${k}`, `q${k}`, fps));
        chain = [`[q${k}]setpts=PTS-STARTPTS+${layer.f0}`, ...fades, ...opacity];
      } else if (geo && same(geo.map((p) => p.cw)) && same(geo.map((p) => p.ch))) {
        // A still in a box of one size: made into its box once, then repeated.
        graph.push(...boxGraph(geo, `${index}:v:0`, `q${k}`, fps));
        chain = [`[q${k}]${looped.join(",")}`, `setpts=N+${layer.f0}`, ...fades, ...opacity];
      } else if (geo) {
        // A keyed zoom: repeated first, then sized frame by frame.
        graph.push(`[${index}:v:0]${looped.join(",")},setpts=N[p${k}]`, ...boxGraph(geo, `p${k}`, `q${k}`, fps));
        chain = [`[q${k}]setpts=PTS-STARTPTS+${layer.f0}`, ...fades, ...opacity];
      } else {
        chain = [
          `[${index}:v:0]scale=w=${W}:h=${H}:force_original_aspect_ratio=decrease:force_divisible_by=2:out_color_matrix=bt709:out_range=tv`,
          "format=yuva420p",
          ...(layer.fills ? [] : [`pad=${W}:${H}:(ow-iw)/2:(oh-ih)/2:color=black@0`]),
          ...looped,
          `setpts=N+${layer.f0}`,
          ...fades,
          ...opacity,
        ];
      }
    } else {
      // Rasters (text, backgrounds, components, SVG) and graphics: PNGs the rasteriser drew.
      const raster = opts.rasters.get(layer);
      if (!raster) throw new Error(`no raster for ${layer.item.id}`);
      let source: Array<string>;
      if (raster.kind === "still") {
        inputs.push("-f", "image2", "-pattern_type", "none", "-i", raster.png);
        source = [`[${index}:v:0]${toYuv(true)}`, `loop=loop=${frames - 1}:size=1:start=0`, `settb=1/${fps}`];
      } else {
        // A sequence, or a streamed layer's video (D291): one picture per frame either way, its
        // timestamps made again from the frame count (the video's are rounded to its container's).
        if (raster.kind === "video") inputs.push("-i", raster.file);
        else inputs.push("-f", "image2", "-framerate", String(fps), "-start_number", "0", "-i", raster.pattern);
        source = [`[${index}:v:0]${toYuv(true)}`, `trim=end_frame=${frames}`, `settb=1/${fps}`];
      }
      if (geo) {
        // Pushed or zoomed (D79): the canvas-size picture made into its moved box, then placed.
        graph.push(`${source.join(",")},setpts=N[p${k}]`, ...boxGraph(geo, `p${k}`, `q${k}`, fps));
        chain = [`[q${k}]setpts=PTS-STARTPTS+${layer.f0}`, ...fades, ...opacity];
      } else chain = [...source, `setpts=N+${layer.f0}`, ...fades, ...opacity];
    }
    // A wipe: the layer shows only its left X pixels (X even, the chroma grid): padded with W
    // transparent pixels on the left, a W-wide window at X holds the hidden part then the shown
    // one, and overlaying it X − W to the left puts the shown part back where it was. Both
    // expressions see the same frame time, so they agree on X.
    let at = "";
    let fmt = "auto";
    if (layer.blur) {
      // A blur dissolve (D133): gblur's sigma set frame by frame (sendcmd), as the preview's CSS
      // blur draws it: on premultiplied colour, with transparency past the layer's edges (padded
      // by 3 sigma, cropped back), so its edges soften into what's under it.
      const pad = blurPad(Math.max(...layer.blur));
      const commands: Array<string> = [];
      layer.blur.forEach((b, i) => {
        if (i > 0 && b === layer.blur![i - 1]) return;
        // sigmaV too: gblur copies sigma into it only when it starts.
        const at = n(Math.max(0, layer.f0 + i - 0.5) / fps);
        commands.push(`${at} gblur@g${k} sigma ${n(b)}`, `${at} gblur@g${k} sigmaV ${n(b)}`);
      });
      chain.push(
        "format=yuva444p",
        `pad=w=iw+${2 * pad}:h=ih+${2 * pad}:x=${pad}:y=${pad}:color=black@0`,
        "premultiply=inplace=1",
        `sendcmd=c='${commands.join(";")}'`,
        `gblur@g${k}=sigma=${n(layer.blur[0]!)}:sigmaV=${n(layer.blur[0]!)}:steps=${BLUR_STEPS}`,
        "unpremultiply=inplace=1",
        `crop=w=iw-${2 * pad}:h=ih-${2 * pad}:x=${pad}:y=${pad}`,
      );
    }
    if (geo && (layer.wipe || layer.iris || layer.mask)) {
      // A boxed layer is put on a transparent canvas first: a wipe or iris reveals the canvas.
      const place = placeAt(layer, geo, fps);
      graph.push(`${chain.join(",")}[x${k}]`);
      graph.push(`color=c=black@0:s=${W}x${H}:r=${fps},format=yuva444p,trim=end_frame=${frames},setpts=N+${layer.f0}[c${k}]`);
      graph.push(`[c${k}][x${k}]overlay=x=${place.x}:y=${place.y}:format=yuv444:eof_action=pass:repeatlast=0[y${k}]`);
      chain = [`[y${k}]null`];
    } else if (geo) {
      const place = placeAt(layer, geo, fps);
      // Overlaid in 4:4:4: on 4:2:0 an odd x or y would snap to even (2-pixel steps on a slow pan).
      at = `:x=${place.x}:y=${place.y}`;
      fmt = "yuv444";
    }
    if (layer.iris) {
      // An iris (D79): a disc at the canvas's centre, radius share · half diagonal (output px; edge
      // pixels by coverage, as the preview's clip-path antialiases), multiplied into the alpha. The
      // mask's frames carry the layer's times, so geq's T is the project frame's moment.
      const { st, d } = layer.iris;
      const r = `${n(Math.hypot(W, H) / 2)}*clip((T*${fps}+0.0001-${n(st)})/${n(d)},0,1)`;
      graph.push(`${chain.join(",")},split=2[ic${k}][ia${k}]`);
      // Computed at 1/IRIS_MASK_STEP of the size (geq is costly per pixel: ~16× less work), a soft
      // edge one coarse pixel wide around the radius, then scaled up bilinearly on the same grid
      // (coarse pixel i covers output pixels [i·s, (i+1)·s)) and cropped: the half-covered contour
      // stays on the radius.
      const step = IRIS_MASK_STEP;
      const mw = Math.ceil(W / step);
      const mh = Math.ceil(H / step);
      graph.push(
        `color=c=black:s=${mw}x${mh}:r=${fps},format=gray,trim=end_frame=${frames},settb=1/${fps},setpts=N+${layer.f0},geq=lum='255*clip((${r}-hypot((X+0.5)*${step}-${n(W / 2)},(Y+0.5)*${step}-${n(H / 2)}))/${step}+0.5,0,1)',scale=w=${mw * step}:h=${mh * step}:flags=bilinear,crop=w=${W}:h=${H}:x=0:y=0[im${k}]`,
      );
      graph.push(`[ia${k}]alphaextract[ie${k}]`, `[ie${k}][im${k}]blend=all_mode=multiply[ik${k}]`, `[ic${k}][ik${k}]alphamerge[ir${k}]`);
      chain = [`[ir${k}]null`];
    } else if (layer.mask) {
      // A reveal shape (D133): core's signed distance as a coverage mask, made like the iris's (at
      // 1/IRIS_MASK_STEP of the size, a soft edge one coarse pixel wide, scaled up on the same grid).
      const { shape, st, d } = layer.mask;
      const step = IRIS_MASK_STEP;
      const mw = Math.ceil(W / step);
      const mh = Math.ceil(H / step);
      const P = `clip((T*${fps}+0.0001-${n(st)})/${n(d)},0,1)`;
      const sd = maskGeq(shape, P, W, H, `(X+0.5)*${step}`, `(Y+0.5)*${step}`);
      graph.push(`${chain.join(",")},split=2[ic${k}][ia${k}]`);
      graph.push(
        `color=c=black:s=${mw}x${mh}:r=${fps},format=gray,trim=end_frame=${frames},settb=1/${fps},setpts=N+${layer.f0},geq=lum='255*clip(${sd}/${step}+0.5,0,1)',scale=w=${mw * step}:h=${mh * step}:flags=bilinear,crop=w=${W}:h=${H}:x=0:y=0[im${k}]`,
      );
      graph.push(`[ia${k}]alphaextract[ie${k}]`, `[ie${k}][im${k}]blend=all_mode=multiply[ik${k}]`, `[ic${k}][ik${k}]alphamerge[ir${k}]`);
      chain = [`[ir${k}]null`];
    }
    if (layer.wipe) {
      const x = `2*floor(${W}*clip((t*${fps}+0.0001-${n(layer.wipe.st)})/${n(layer.wipe.d)},0,1)/2)`;
      chain.push(`pad=${2 * W}:${H}:${W}:0:color=black@0`, `crop=w=${W}:h=${H}:x='${x}':y=0`);
      at = `:x='${x}-${W}':y=0`;
    }
    graph.push(`${chain.join(",")}[l${k}]`);
    graph.push(`[${base}][l${k}]overlay=eof_action=pass:format=${fmt}:repeatlast=0${at}[b${k + 1}]`);
    base = `b${k + 1}`;
  });
  // fps on frames already on the 1/fps grid passes them through one to one; it gives the stream its rate.
  // F4 (D171): another output rate resamples the finished picture (the edit is timed in the plan's frames).
  const outFps = opts.video?.fps !== undefined && opts.video.fps > 0 && Math.abs(opts.video.fps - fps) > 1e-6 ? opts.video.fps : undefined;
  graph.push(`[${base}]format=yuv420p,fps=${fps},trim=end_frame=${plan.frames}${outFps ? `,fps=${n(outFps)}` : ""}[vout]`);
  const audioInput = inputs.filter(value => value === '-i').length;
  if (opts.preparedAudio) {
    inputs.push('-i', opts.preparedAudio);
    graph.push(`[${audioInput}:a:0]aformat=sample_fmts=fltp:channel_layouts=stereo[aout]`);
  } else {
    const audio = audioGraph(plan, audioInput, opts.audio);
    inputs.push(...audio.inputs);
    graph.push(...audio.filters);
  }
  // The soft subtitle track (D105): the last input, its own stream (cue times are the edit's).
  const subtitleInput = inputs.filter((a) => a === "-i").length;
  if (opts.subtitles) inputs.push("-f", "srt", "-i", opts.subtitles.file);
  const videoArgs = videoCodec(opts.quality, opts.caps, plan, outFps ?? fps, opts.video);
  const outputArgs = [
    "-map",
    "[vout]",
    "-map",
    "[aout]",
    ...(opts.subtitles ? ["-map", `${subtitleInput}:s:0`, "-c:s", "mov_text", "-metadata:s:s:0", `language=${opts.subtitles.language}`, "-disposition:s:0", opts.subtitles.show ? "default" : "0"] : []),
    ...videoArgs,
    "-pix_fmt",
    "yuv420p",
    "-colorspace",
    "bt709",
    "-color_primaries",
    "bt709",
    "-color_trc",
    "bt709",
    "-color_range",
    "tv",
    // The graph already makes one frame per 1/fps; ffmpeg's own constant-frame-rate pass would
    // shift the picture by a frame when inputs are seeked (seen on the DJI clip), so it's off.
    "-fps_mode",
    "passthrough",
    "-c:a",
    "aac",
    "-b:a",
    opts.quality === "draft" ? "128k" : "192k",
    "-ar",
    "48000",
    "-t",
    n(plan.frames / fps),
    "-movflags",
    "+faststart",
    "-f",
    "mp4",
    opts.output,
  ];
  return { inputs, graph: graph.join(";\n"), videoArgs, outputArgs };
}

/**
 * A problem ffmpeg reported but didn't stop for (it exits 0): a scaler or filter that failed on
 * some frames leaves a picture missing (seen with too many inputs at once). The export fails then.
 */
export function silentFailure(stderr: string): string | undefined {
  const line = stderr.split("\n").find((l) => /\[swscaler[^\]]*\].*(error|fail|cannot|invalid)/i.test(l) || /Error while (filtering|processing)|Failed to inject frame/i.test(l));
  return line?.trim();
}
