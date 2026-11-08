// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import {planMasks} from './masks';
import {maskRegions} from '../../core/timeline/masks';
import { type Media, type SpeechEnvelope, type Project, layout, type Item, itemMotionBlur, subFrameTimes, isSubtitleTrack, SOLID, isDrawnComponent, spanOf, maskOf, transitionAt, blurs, IRIS, reveals, movesIncoming, movesOutgoing, moveBox, canvasBox, isSvg, clipBox, graphicDocument, graphicLocal, fxBox, clipFx, fxSkips, clipShader, needsPicturePass, isPlainBox, baseBox, fitRect, colourLut, sourceAt, hasOverlay, fxOverlayHtml, animationAt, componentBox, componentParts, textHtml, solidHtml, wrapComponent, dipsOf, mixNote, duckCurves, trackAudible, duckNote, soundMedia, trackStages, panGains, trackPan, audioSpan, duckKeysIn, heldPast, canvasBackground, masterGain, masterLimiter } from "@/app/lib/editor/core";
import { type ExportPlan, mediaToken, type VisualLayer, type LayerFx, type Ramp, type AudioLayer, type GainModel } from "./types.ts";
import { capStates, opacityRamps, splitAt, placementOf, svgClipHtml, splitLoop } from "./layers.ts";
import { mergeableCues, captionLayers, subtitleLayers } from "./captions.ts";
import { constantGain } from "./gain.ts";
import {exportRangeProblem,type FrameRange} from '../range';
import {windowLayer} from './window';

// ---- the plan ----

export interface PlanOptions {
  /** Read this interval of the original timeline; do not trim or retime saved items. */
  readonly range?: FrameRange;
  /** The file of a media item (relative paths resolved against the project folder). */
  readonly mediaPath: (media: Media) => string;
  readonly width: number;
  readonly height: number;
  /** A graphic item's file contents (D78), by its src; undefined when it can't be read. */
  readonly graphic?: ((src: string) => string | undefined) | undefined;
  /**
   * The loudness envelopes (D74) of the media on ducking key tracks (D85), by media id (null:
   * analysed, no sound). A key media missing here ducks nothing, with a warning.
   */
  readonly envelopes?: ReadonlyMap<string, SpeechEnvelope | null> | undefined;
  /** Draw captions and Subtitles tracks into the picture (D105; default true). Off: a soft track or a file carries them instead. */
  readonly burnCaptions?: boolean | undefined;
  /** D291: motion blur on graphics (the project's samples and shutter; an item's motionBlur false opts out). Default off. */
  readonly motionBlur?: boolean | undefined;
}

/** Everything the export draws and plays, or why there's nothing to export. */
export function planExport(project: Project, { mediaPath, width, height, graphic, envelopes, burnCaptions = true, motionBlur = false, range }: PlanOptions): ExportPlan | { readonly problem: string } {
  const fps = project.canvas.fps;
  const { starts, duration } = layout(project);
  if (!project.tracks.some((t) => t.items.length)) return { problem: "The timeline is empty: there's nothing to export yet." };
  const problem=exportRangeProblem(project,range);
  if(problem)return {problem};
  const window=range??{from:0,to:duration};
  const mediaOf = (item: Item) => (item.media ? project.media.find((m) => m.id === item.media) : undefined);
  const files = new Map<string, { media: Media; path: string }>();
  const fileOf = (media: Media) => {
    const path = mediaPath(media);
    files.set(media.id, { media, path });
    return path;
  };
  const aspect = project.canvas.width / project.canvas.height;
  const warnings: Array<string> = [];
  // Components' media references: a placeholder each (the exporter swaps in the file as a data: URL).
  const target = { ...project, mediaUrl: (id: string) => {
    const m = project.media.find((x) => x.id === id);
    if (!m || m.kind !== "image") return undefined;
    fileOf(m);
    return mediaToken(id);
  } };
  const name = (item: Item) => `"${item.name ?? item.id}"`;
  const fills = (media: Media) => !!media.probe?.width && !!media.probe?.height && Math.abs(media.probe.width / media.probe.height - aspect) < 0.002;

  const visuals: Array<VisualLayer> = [];
  for (const track of [...project.tracks].reverse()) {
    if (track.hidden || track.kind === "audio") continue;
    const subtitles = isSubtitleTrack(track) ? mergeableCues(track, starts) : undefined;
    track.items.forEach((item, i) => {
      if(item.type==='danmaku'&&!item.danmaku?.settings.enabled)return;
      const media = mediaOf(item);
      const isMedia = (item.type === "clip" || item.type === "danmaku") && media && media.kind !== "audio";
      const isSolid = item.type === "component" && item.component === SOLID;
      const isText = item.type === "text" && typeof item.text === "string";
      const isComponent = isDrawnComponent(item);
      const isGraphic = item.type === "graphic" && typeof item.src === "string";
      if (item.type === "captions") {
        if (burnCaptions) visuals.push(...captionLayers(project, item, track, window.to, warnings, window.from));
        return;
      }
      // A Subtitles track's cues (D105): its plain ones drawn together (below), all of them off without burn-in.
      if (subtitles && isText && (!burnCaptions || subtitles.includes(item))) {
        if (burnCaptions && item === subtitles[0]) visuals.push(...subtitleLayers(project, track, subtitles, starts, window.to, warnings, window.from));
        return;
      }
      if (!isMedia && !isSolid && !isText && !isComponent && !isGraphic) return;
      const start = starts.get(item.id)!;
      const span = spanOf(track, i, start);
      const f0 = Math.max(window.from, Math.ceil(span.from));
      const f1 = Math.min(window.to, Math.ceil(span.to));
      if (f1 <= f0) return;
      if (isMedia && item.loop && media.kind !== 'image' && !media.probe?.frames) throw new Error('Looped media needs a measured duration before export.');
      const ramps = opacityRamps(item, start, span);
      const reveal = { st: span.from, d: 2 * span.half };
      const shape = span.half > 0 ? maskOf(span.into) : undefined;
      // The transitions at its cuts (D79, D133): what they do at each frame, and where its layer is cut.
      const fxAt = (f: number) => transitionAt(span, f, project.canvas);
      // A blur dissolve (D133): sigma per frame in output px (core's blur is in canvas px).
      const sigmas = blurs(span.into) || blurs(span.next) ? Array.from({ length: f1 - f0 }, (_, k) => ((fxAt(f0 + k).blur ?? 0) * height) / project.canvas.height) : undefined;
      const base = { item, track, f0, f1, ramps, ...(span.wipe ? { wipe: reveal } : {}), ...(span.into === IRIS ? { iris: reveal } : {}), ...(shape ? { mask: { shape, ...reveal } } : {}), ...(sigmas?.some((b) => b > 0) ? { blur: sigmas } : {}) };
      const inEnd = span.half > 0 ? Math.ceil(span.from + 2 * span.half) : undefined;
      const revealEnd = inEnd !== undefined && reveals(span.into!) ? inEnd : undefined;
      const moveIn = inEnd !== undefined && movesIncoming(span.into!) ? inEnd : undefined;
      const outStart = span.nextHalf > 0 ? Math.ceil(span.to - 2 * span.nextHalf) : undefined;
      const moves = [moveIn, outStart !== undefined && movesOutgoing(span.next!) ? outStart : undefined].filter((c): c is number => c !== undefined);
      const always = [inEnd !== undefined && blurs(span.into) ? inEnd : undefined, outStart !== undefined && blurs(span.next) ? outStart : undefined].filter((c): c is number => c !== undefined);
      const split = (layer: VisualLayer, plain?: (f: number) => boolean) => splitAt(layer, { moves, always, ...(revealEnd !== undefined ? { revealEnd } : {}), ...(plain ? { plain } : {}) }, fps);
      // Text, backgrounds and graphics have no box: a push or zoom moves the canvas (core canvasBox).
      const kx = width / project.canvas.width;
      const ky = height / project.canvas.height;
      const canvasMoves = Array.from({ length: f1 - f0 }, (_, k) => {
        const fx = fxAt(f0 + k);
        return fx.shift || fx.shiftY || (fx.zoom !== undefined && fx.zoom !== 1) ? moveBox(canvasBox(project.canvas), fx.shift, fx.zoom, fx.shiftY) : undefined;
      });
      const canvasGeometry = canvasMoves.some(Boolean) ? { geometry: { frames: canvasMoves.map((b) => placementOf(b ?? canvasBox(project.canvas), kx, ky, canvasBox(project.canvas))) } } : {};
      const canvasStill = (f: number) => !canvasMoves[f - f0];
      if (isMedia && isSvg(media) && !maskRegions(item).length) {
        // An SVG (D76): Chromium draws it in its box, one PNG per distinct box (keys).
        fileOf(media);
        const looks: Array<string> = [];
        for (let f = f0; f < f1; f++) {
          const fx = fxAt(f);
          const box = moveBox(clipBox(item, project.canvas, media.probe, f - start), fx.shift, fx.zoom, fx.shiftY);
          looks.push(svgClipHtml(item, box, mediaToken(media.id), box.opacity));
        }
        // Only its box changes (keys): drawn on every frame, like a clip's, never capped.
        const { states, frames } = capStates(looks, Infinity);
        visuals.push(...split({ ...base, kind: "raster", states, frames }));
        return;
      }
      if (isGraphic) {
        const html = graphic?.(item.src!);
        if (html === undefined) {
          warnings.push(`${name(item)}: ${item.src} couldn't be read, so it isn't drawn.`);
          return;
        }
        // Every frame drawn (D291); before its start and past its end it holds its first and last.
        // Its own frames run from its `in` (a range export's slice, a split's second half).
        const frames = Array.from({ length: f1 - f0 }, (_, k) => graphicLocal(item, f0 + k - start));
        // Motion blur (D291): each frame's sub-frames; a held frame stays sharp.
        const blur = itemMotionBlur(item, project, motionBlur);
        const motion = blur ? { ...blur, ts: frames.map((local, k) => subFrameTimes(local, fps, blur, local !== (item.in ?? 0) + f0 + k - start)) } : undefined;
        const opacity = typeof item.transform?.opacity === "number" ? Math.min(1, Math.max(0, item.transform.opacity)) : 1;
        visuals.push(...split({ ...base, ...canvasGeometry, ...(opacity !== 1 ? { alpha: Array.from({ length: f1 - f0 }, () => opacity) } : {}), kind: "graphic", doc: graphicDocument(html, project.canvas,30000), frames, ...(motion ? { motion } : {}) }, canvasStill));
        return;
      }
      if (isMedia) {
        const path = fileOf(media);
        const masks=planMasks(item,media,project.media,fileOf,start);
        // The box at every frame (D75), moved by a push or zoom (D79): plain frames (no transform,
        // no move) keep the full-frame chain.
        // A zoom pulse (D136) is in the box too.
        const boxes = Array.from({ length: f1 - f0 }, (_, k) => {
          const fx = fxAt(f0 + k);
          const box=clipBox(item, project.canvas, media.probe, f0 + k - start);
          return moveBox(fxSkips(media)?box:fxBox(item,box,f0 + k - start,fps), fx.shift, fx.zoom, fx.shiftY);
        });
        // Picture effects (D136) run before the box, so a clip with them always has one.
        const effects = fxSkips(media)?undefined:clipFx(item);
        // A shader effect (D213) is a picture pass too (SVG stills take none, D138).
        const shader = fxSkips(media) ? undefined : clipShader(item);
        const picture = needsPicturePass(effects) || effects?.blur !== undefined || !!shader;
        const plainAt = boxes.map((b) => !picture && isPlainBox(b, project.canvas));
        const rest = baseBox(item, project.canvas, media.probe);
        const placements = boxes.map((b) => placementOf(b, kx, ky, rest));
        const geometry = plainAt.every(Boolean) ? {} : { geometry: { frames: placements } };
        const alpha = boxes.some((b) => b.opacity !== 1) ? { alpha: boxes.map((b) => b.opacity) } : {};
        let layerFx: { fx?: LayerFx } = {};
        if (picture && (effects || shader)) {
          // The size the effects work at, as the preview's (core fxPictureSize): the picture's fit before
          // any zoom (a scale, keys, a pulse or a transition zoom scale it after); its placement's size
          // when it's never zoomed (the same within a pixel, so the box's own scale passes it through).
          const fixed = boxes.every((b) => b.scale === 1) && placements.every((p) => p.cw === placements[0]!.cw && p.ch === placements[0]!.ch);
          const fit = fitRect(rest, boxes[0]!.fit, media.probe);
          const w = fixed ? placements[0]!.cw : Math.max(1, Math.round(fit.w * kx));
          const h = fixed ? placements[0]!.ch : Math.max(1, Math.round(fit.h * ky));
          layerFx = { fx: { fx: effects ?? {}, start, w, h, k: ky, ...(effects?.colour ? { lut: colourLut(effects.colour) } : {}), ...(shader ? { shader } : {}) } };
        }
        const layer: VisualLayer =
          media.kind === "image"
            ? { ...base, ...alpha, kind: "image", file: path, fills: item.type!=="danmaku"&&fills(media), ...geometry, ...layerFx, ...(masks?{masks}:{}) }
            : { ...base, ...alpha, kind: "video", file: path, source: sourceAt(project, item, f0 - start), speed: item.speed ?? 1, fills: item.type!=="danmaku"&&fills(media), ...geometry, ...layerFx, ...(masks?{masks}:{}), ...(item.hold && !item.loop && media.probe?.frames ? { last: (media.probe.frames - 1) / fps } : {}) };
        // Cut at the transitions first, then each piece into passes through its media (D79 review): a
        // long looped run keeps one looped input (the loop filter) and a transition's few frames seek.
        for (const piece of split(layer, (f) => plainAt[f - f0]!)) visuals.push(...splitLoop(piece, project, start, { width, height }));
        // Vignette, grain and letterbox bars (D136): Chromium draws them over the picture in its box
        // (core fxOverlayHtml, as the preview does), a layer of their own right above the clip with
        // its opacity, transitions and blur.
        if (effects && hasOverlay(effects)) {
          const looks = boxes.map((b, k) => fxOverlayHtml(effects, b, f0 + k - start));
          const { states, frames } = capStates(looks);
          visuals.push(...split({ ...base, ...alpha, kind: "raster", states, frames }));
        }
        return;
      }
      // Text moves (rise, slide, wipe) are drawn by Chromium frame by frame; opacity is a ramp.
      // A component's look (a terminal typing), its box (keys) and its box's opacity too.
      // A component's body (its animation: a terminal typing) is a look per change; its box (position,
      // size, opacity, scale, the move's offset) changes on every frame it changes, like text's motion.
      const looks: Array<string> = [];
      const wrappers: Array<string> = [];
      for (let f = f0; f < f1; f++) {
        const local = f - start;
        const motion = isText || isComponent ? animationAt(item, local, project.canvas, { skipIn: span.skipIn, skipOut: span.skipOut }).motion : {};
        if (isComponent) {
          // Its box moved by a push or zoom (D79), drawn into the HTML like its keys.
          const own = componentBox(item, project.canvas, local);
          const fx = fxAt(f);
          const parts = componentParts(item, target, local, { opacity: own?.opacity ?? 1, motion, fps, ...(own ? { box: moveBox(own, fx.shift, fx.zoom, fx.shiftY) } : {}) })!;
          wrappers.push(parts.wrapper);
          looks.push(parts.body);
        } else looks.push(isText ? textHtml(item, project, { motion }) : solidHtml(item, project));
      }
      let html = looks;
      if (isComponent) {
        const bodies = capStates(looks);
        html = wrappers.map((w, i) => wrapComponent(w, bodies.states[bodies.frames[i]!]!));
      }
      // Text's looks (its moves: rise, slide, wipe, word by word, zoom; D81): every one drawn (D291).
      const { states, frames } = capStates(html);
      visuals.push(...split({ ...base, ...(isComponent ? {} : canvasGeometry), kind: "raster", states, frames }, canvasStill));
    });
    // Dips to black: core's black layer over the main track, fading in to the cut and out after it.
    for (const dip of dipsOf(track, starts)) {
      const f0 = Math.max(window.from, Math.ceil(dip.cut - dip.half));
      const f1 = Math.min(window.to, Math.ceil(dip.cut + dip.half));
      if (f1 <= f0) continue;
      const ramps: Array<Ramp> = [
        { type: "in", st: dip.cut - dip.half, d: dip.half },
        { type: "out", st: dip.cut, d: dip.half },
      ];
      visuals.push({ item: dip.item, track, f0, f1, ramps, kind: "raster", states: [solidHtml(dip.item, project)], frames: Array.from({ length: f1 - f0 }, () => 0) });
    }
  }

  // Solo, mutes or a silent master that leave little or nothing audible (D83): the job says so.
  const note = mixNote(project);
  if (note) warnings.push(note);
  const audio: Array<AudioLayer> = [];
  const ducks = duckCurves(project, envelopes ?? new Map());
  const buses: Array<ExportPlan["buses"][number]> = [];
  for (const track of project.tracks) {
    if (!trackAudible(project, track)) continue;
    const curve = ducks.get(track.id);
    const duckWarning = duckNote(project, track, curve);
    if (duckWarning && track.items.some((i) => soundMedia(project, i))) warnings.push(duckWarning.replace("waits for", "isn't applied: it needs"));
    // Noise removal (D88) starts from the track's measured noise floor, as the preview's does (D246).
    const stages = trackStages(track, project, envelopes ?? new Map());
    if (stages.length && track.items.some((i) => soundMedia(project, i))) buses.push({ track: track.id, stages, pan: panGains(trackPan(track)) });
    track.items.forEach((item, index) => {
      const media = soundMedia(project, item);
      if (!media) return;
      const start = starts.get(item.id) ?? 0;
      const span = audioSpan(project, track, index, start);
      // Audio is cropped after its buses and limiter, so their state and tempo/filter
      // lookahead match the original timeline on both sides of the selected interval.
      const from = Math.max(0, span.from), to = Math.min(duration, span.to);
      if(to<=from)return;
      if(item.loop&&!media.probe?.frames)throw new Error('Looped media needs a measured duration before export.');
      const duck = curve ? duckKeysIn(curve.keys, span.from, span.to).map((k) => ({ frame: k.frame - start, db: k.db })) : [];
      const gain: GainModel = {
        ...(duck.length ? { duck } : {}),
        baseDb: (item.gainDb ?? 0) + (track.mixer?.gainDb ?? 0),
        keys: [...(item.gainKeys ?? [])].sort((a, b) => a.frame - b.frame),
        duration: item.duration,
        half: span.half,
        nextHalf: span.nextHalf,
        // A dip to black dips the sound: core fades it over half the dip instead of the item's own fade.
        fadeIn: span.dipIn > 0 ? span.dipIn : (item.fadeIn ?? 0),
        fadeOut: span.dipOut > 0 ? span.dipOut : (item.fadeOut ?? 0),
      };
      if (constantGain(gain) === 0) return;
      // A held clip (D94) is silent past its media's end: a piece lying wholly there has no sound.
      if (heldPast(project, item, from - start)) return;
      const source = Math.max(0, sourceAt(project, item, from - start));
      const length = item.loop ? media.probe?.frames : undefined;
      const cycle = length ? { cycle: { samples: Math.max(1, Math.round((length * 48000) / fps)), offset: Math.round(source * 48000) } } : {};
      audio.push({ item, track: track.id, file: fileOf(media), from, to, localStart: from - start, source, speed: item.speed ?? 1, gain, pan: panGains(trackPan(track)), ...cycle });
    });
  }

  return {
    fps,
    canvas: { width: project.canvas.width, height: project.canvas.height },
    width,
    height,
    frames: window.to-window.from,
    backgroundHtml: `<div style="position:absolute;inset:0;background:${canvasBackground(project)}"></div>`,
    visuals:visuals.map(layer=>windowLayer(layer,window.from)),
    audio,
    audioFrames:duration,
    audioWindow:window,
    masterGain: masterGain(project),
    buses,
    limiter: masterLimiter(project),
    files: [...files.values()],
    warnings,
  };
}
