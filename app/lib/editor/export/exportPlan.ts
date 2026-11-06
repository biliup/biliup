// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// What an export draws and plays, and the ffmpeg command that makes it (D39, D55). Pure: no
// processes, no files; the export service (exporter.ts) rasterises the text, runs ffmpeg and
// reports progress. Everything here follows the preview's maths in the editor core:
// - Picture: every layer visualsAt would return, composited bottom-up in visualsAt's order
//   (tracks from the bottom, items in order), from frame ceil(from) to ceil(to) of its span.
//   Footage and stills without a box (D75: no transform, or the full frame contain at scale 1) are
//   scaled to fit like the preview's object-fit: contain, over transparent bars. A boxed clip is
//   scaled to its picture's size, padded / cropped to its box, its corners rounded (a mask), and
//   overlaid at the box's position; per-frame sizes (keyed scale) and positions (keyed x / y) are
//   lookups on the frame number, and keyed opacity is sent to colorchannelmixer frame by frame
//   (sendcmd), all from core's boxes. A looped clip is one layer per pass through its media. Text and backgrounds are PNGs Chromium drew from core's textHtml / solidHtml with
//   RENDER_CSS; a text whose look moves (rise, slide, wipe, word by word, zoom: D81) gets one PNG per distinct state.
//   Components (D76) are PNGs of core's componentHtml, one per distinct look (an animated one, a
//   terminal typing, gets one per frame its look changes; D291: never held, the rasteriser streams
//   them raw to ffmpeg), their box's opacity baked into the HTML. SVG images are drawn by
//   Chromium at their box's size (ffmpeg can't decode SVG). A graphic item (D78) is its document
//   drawn frame by frame (render(t) called), every frame, with motion blur's sub-frames when on (D291). Captions (D82) are one raster
//   layer per captions item from its first card to its last: core captionAt's text item at each
//   frame drawn by textHtml (an empty state between cards), one PNG per distinct card / speaking
//   word. Not capped: each card has at most its words + 1 looks, so a long talk is many small sets.
//   Opacity (a transition's crossfade, fade/rise/slide in and out) is linear in frames, so it is
//   ffmpeg's fade on the layer's alpha, evaluated at the same frames (time mode). A dip to black
//   is core's black layer over the main track (a raster with two ramps); a wipe crops the
//   incoming layer from the left at the same frames (pad + crop + overlay x, per frame); an iris
//   multiplies a growing disc (a per-frame geq mask) into its alpha (D79). A push or zoom moves the
//   item's box (core moveBox: per-frame boxes as for keys); text, backgrounds and graphics get the
//   canvas as a box. The frames a transition reveals or moves are a layer of their own, so the rest
//   of the item keeps the plain chain.
// - Sound: every item audibleAt would return, from its media at the same source time, with the
//   gain of core itemGain (combined dB with the −60 dB cutoff, gainKeys, fades, transition
//   crossfades) as one volume expression per item, panned by its track (D83), mixed with amix
//   normalize=0 (D54), then the master volume (D83). Ducking (D85) is core's duck curve folded into
//   each item's volume expression on the ducked track (the same dB the preview adds). A track with
//   effects (D84: EQ, compressor, voice, noise removal) is summed on its own first (amix over a
//   silent base), run through its stages in list order (core trackStages → stageFilter) and panned
//   after them; then the tracks are summed. After the master volume: the loudness gain (D87) and
//   the master limiter last (D86: alimiter at 4× oversampling, a small true-peak margin).
//   Range exports process that original audio history before cropping samples; nonzero starts
//   prepare a selected PCM track before video encoding. Loudness measurement covers only output.
// - A frame f is the moment f / fps (D38); a clip shows the source frame on screen at its source
//   time, as the preview's <video> does (the last frame that starts at or before it).
import { outputSize } from "@/app/lib/editor/core";
export { MEDIA_TOKEN, mediaToken, RASTER_CSS } from "./plan/types.ts";
export type { Quality, Ramp, Placement, LayerFx, Geometry, VisualLayer, GainModel, AudioLayer, ExportPlan } from "./plan/types.ts";
export { opacityRamps, alphaAt, placementOf, wipeAt, IRIS_MASK_STEP, BLUR_STEPS, blurPad, blurFilters, irisAt, rampsAt, compactStates, rasterLooks, MAX_LOOP_INPUTS, capStates, svgClipHtml } from "./plan/layers.ts";
export { gainAt, constantGain, AUDIO_BLOCK, gainExpr, pwlExpr } from "./plan/gain.ts";
export { planExport } from "./plan/planExport.ts";
export type { PlanOptions } from "./plan/planExport.ts";
export { CAPTION_GAP_HTML, MAX_CAPTION_LAYERS, CAPTION_LOOKS_WARN, captionLayers, subtitleLayers, mergeableCues } from "./plan/captions.ts";
export { graphFileArgs, canHevc, usesHardware, videoCodec, hardwareEncoderFailed } from "./plan/codec.ts";
export type { RasterOutput, Caps, VideoChoice } from "./plan/codec.ts";
export { LIMITER_MARGIN_DB, LIMITER_ATTACK_MS, LIMITER_RELEASE_MS, limiterFilter, stageFilter, audioGraph } from "./plan/audio.ts";
export type { AudioGraph, AudioOptions } from "./plan/audio.ts";
export { untaggedFilter, sourceFramePick, frameLookup, cornerMask, boxGraph, videoSource, shaderSourceArgs } from "./plan/video.ts";
export type { CommandOptions } from "./plan/video.ts";
export { planLuts, fxGraph } from "./plan/fx.ts";
export { exportCommand, silentFailure } from "./plan/command.ts";
export { loudnessCommand, parseEbur128, LOUDNESS_CHECKS, LOUDNESS_TOLERANCE_LU, parseLoudness, loudnormFilter, planLoudness, loudnessGain } from "./plan/loudness.ts";
export type { LoudnessMeasure } from "./plan/loudness.ts";
export { outputSize };
