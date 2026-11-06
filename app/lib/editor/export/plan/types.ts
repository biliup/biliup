// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type ExportQuality, type Item, type Track, type MaskShape, type ClipFx, type ColourLut, type ShaderUse, type DuckKey, type Stage, type Media, RENDER_CSS } from "@/app/lib/editor/core";

export type Quality = ExportQuality;
import type {MaskLayer} from './masks';
import type {FrameRange} from '../range';


/** A linear opacity ramp on a layer, in project frames: fade in from 0 at `st` to 1 at st + d, or out. */
export interface Ramp {
  readonly type: "in" | "out";
  readonly st: number;
  readonly d: number;
}

/**
 * A boxed clip at one frame, in output pixels (D75): the box (edges rounded to whole pixels, so
 * boxes that share an edge meet without a seam) and the picture inside it (its size and its
 * top-left relative to the box's, edges rounded the same way, so a picture that covers the box
 * covers it exactly). The box's size and radius never change over a layer; x, y and the picture do.
 */
export interface Placement {
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly h: number;
  readonly cx: number;
  readonly cy: number;
  readonly cw: number;
  readonly ch: number;
  readonly radius: number;
}

export interface LayerBase {
  readonly masks?: MaskLayer;
  readonly item: Item;
  readonly track: Track;
  /** First frame drawn and the frame after the last (integers). */
  readonly f0: number;
  readonly f1: number;
  readonly ramps: ReadonlyArray<Ramp>;
  /** A wipe transition into it: shown from the left edge, 0 at frame `st` to the whole width at st + d. */
  readonly wipe?: { readonly st: number; readonly d: number };
  /** An iris transition into it (D79): shown inside a circle at the canvas's centre, radius 0 at frame `st` to the half diagonal at st + d. */
  readonly iris?: { readonly st: number; readonly d: number };
  /** A reveal shape into it (D133): shown inside core's maskDistance shape, share 0 at frame `st` to 1 at st + d. */
  readonly mask?: { readonly shape: MaskShape; readonly st: number; readonly d: number };
  /** A blur dissolve (D133): the Gaussian sigma in output px for each frame from f0; absent when never blurred. */
  readonly blur?: ReadonlyArray<number>;
  /**
   * Its box for each frame from f0 (D75): a clip's (absent: the plain full frame, drawn as before
   * D75), or the canvas moved by a push or zoom for text, backgrounds and graphics (D79).
   */
  readonly geometry?: Geometry;
  /** The box's opacity (transform.opacity, opacity keys) for each frame from f0; absent when always 1. */
  readonly alpha?: ReadonlyArray<number>;
}

/**
 * A clip's picture effects (D136): what they draw (core clipFx), the clip's start (frame), the size
 * (output px) the picture is made before them, and output px per canvas px. They run on the picture
 * before its box (fxGraph); the box (geometry) is always there when they are.
 */
export interface LayerFx {
  readonly fx: ClipFx;
  readonly start: number;
  readonly w: number;
  readonly h: number;
  readonly k: number;
  /** The colour table (core colourLut), when the colour changes. */
  readonly lut?: ColourLut;
  /**
   * A shader effect (D213), drawn first, on the picture at w×h: the exporter draws its frames with
   * the rasteriser's WebGPU (shadeLayers) and the layer's picture comes from them (CommandOptions.shaded).
   */
  readonly shader?: ShaderUse;
}

/** A clip's box for each frame from f0 (absent: the plain full frame, drawn as before D75). */
export interface Geometry {
  readonly frames: ReadonlyArray<Placement>;
}

export type VisualLayer =
  | (LayerBase & {
      readonly kind: "video";
      readonly file: string;
      /** Source time (s) of frame f0, and source seconds per project second. */
      readonly source: number;
      readonly speed: number;
      /** The picture fills the canvas exactly (same aspect), so it needs no bars. */
      readonly fills: boolean;
      /**
       * A looped clip played from one input (D75 review): the media's first `length` project frames
       * are decoded once and repeated (ffmpeg's loop filter), frame f0 showing frame `offset` of them.
       */
      readonly cycle?: { readonly length: number; readonly offset: number };
      /** A held clip (D94): the source time (s) of its media's last frame, where its source stops. */
      readonly last?: number;
      readonly fx?: LayerFx;
    })
  | (LayerBase & { readonly kind: "image"; readonly file: string; readonly fills: boolean; readonly fx?: LayerFx })
  | (LayerBase & {
      readonly kind: "raster";
      /** HTML of each distinct look, at canvas size. */
      readonly states: ReadonlyArray<string>;
      /** For each frame from f0, the index of its look. */
      readonly frames: ReadonlyArray<number>;
    })
  | (LayerBase & {
      readonly kind: "graphic";
      /** The graphic's document (core graphicDocument), drawn at canvas size. */
      readonly doc: string;
      /** For each frame from f0, the frame from the item's start drawn there (held frames repeat one). */
      readonly frames: ReadonlyArray<number>;
      /** D291: motion blur: each frame the average of motion.samples pictures, drawn at ts[i] (seconds from the item's start, core subFrameTimes). */
      readonly motion?: { readonly samples: number; readonly shutter: number; readonly ts: ReadonlyArray<ReadonlyArray<number>> };
    });

/** The placeholder a component's media reference becomes in the plan's HTML; the exporter swaps in a data: URL. */
export const MEDIA_TOKEN = "biliup-media:";
export const mediaToken = (id: string) => `${MEDIA_TOKEN}${id}`;

/** What itemGain needs of an item, kept so the volume expression can be built (and checked). */
export interface GainModel {
  /** item.gainDb + the track's mixer.gainDb. */
  readonly baseDb: number;
  readonly keys: ReadonlyArray<{ readonly frame: number; readonly db: number }>;
  readonly duration: number;
  readonly half: number;
  readonly nextHalf: number;
  readonly fadeIn: number;
  readonly fadeOut: number;
  /** The track's ducking over this item (D85): core's duck keys, frames from the item's start; absent when none. */
  readonly duck?: ReadonlyArray<DuckKey>;
}

export interface AudioLayer {
  readonly item: Item;
  /** The item's track id (a track with effects is summed on its own bus first). */
  readonly track: string;
  readonly file: string;
  /** The span it sounds over, in (possibly fractional) project frames. */
  readonly from: number;
  readonly to: number;
  /** Local frame of the item at from, including any transition preroll clipped at project zero. */
  readonly localStart: number;
  /** Source time (s) at `from`, and the speed. */
  readonly source: number;
  readonly speed: number;
  /**
   * A looped item: the media's first `samples` samples (48 kHz: exactly
   * its length in project frames) repeated with aloop, starting `offset` samples in, so passes stay
   * on the picture's frames instead of drifting at the file's real length. Source looping and
   * offset trimming precede speed/pitch processing.
   */
  readonly cycle?: { readonly samples: number; readonly offset: number };
  readonly gain: GainModel;
  /** The track's pan as left / right factors on the stereo sound (core panGains, D83); 1/1 at centre. */
  readonly pan: { readonly left: number; readonly right: number };
}

export interface ExportPlan {
  readonly fps: number;
  readonly canvas: { readonly width: number; readonly height: number };
  /** Output size. */
  readonly width: number;
  readonly height: number;
  /** Frames in the export. */
  readonly frames: number;
  /** The canvas colour as HTML, drawn under everything. */
  readonly backgroundHtml: string;
  readonly visuals: ReadonlyArray<VisualLayer>;
  readonly audio: ReadonlyArray<AudioLayer>;
  /** Process the original mix before selecting output, preserving stateful effect history. */
  readonly audioFrames: number;
  readonly audioWindow: FrameRange;
  /** The master volume after the tracks are summed (core masterGain, D83), as a linear factor. */
  readonly masterGain: number;
  /**
   * Tracks with effects (D84): their items are summed first, run through `stages` in order, then
   * panned (`pan`, as each item would have been). Other tracks' items go straight into the mix.
   */
  readonly buses: ReadonlyArray<{ readonly track: string; readonly stages: ReadonlyArray<Stage>; readonly pan: { readonly left: number; readonly right: number } }>;
  /** The master limiter (D86), last in the chain; absent when bypassed. */
  readonly limiter?: { readonly ceilingDb: number } | undefined;
  /** Every media file the export reads. */
  readonly files: ReadonlyArray<{ readonly media: Media; readonly path: string }>;
  /** What the export draws differently from the preview (a capped animation), as sentences. */
  readonly warnings: ReadonlyArray<string>;
}

/** The CSS every rasterised layer gets: the preview's (D53). */
export const RASTER_CSS = RENDER_CSS;
