// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Timeline maths shared by the preview, export and the agent's frame renders.
// Frame f means the moment f / fps. A main-track transition sits on its cut (transitions.ts says
// how each draws): an overlapping one (crossfade, wipe, iris, push, zoom) has the outgoing clip run half the
// transition past its end and the incoming one start half before its start, on top; a dip to
// black keeps the clips apart and puts a black layer over the main track across the cut.
import { sourceAt } from "@/app/lib/editor/core/audio/audio";
import { clipBox, EASES, moveBox, type ClipBox, type Ease } from "@/app/lib/editor/core/visual/box";
import { isSpringName, springEase } from "@/app/lib/editor/core/visual/spring";
import { fxBox, fxSkips } from "@/app/lib/editor/core/visual/fx";
import { captionAt } from "@/app/lib/editor/core/text/captions";
import { componentBox } from "@/app/lib/editor/core/visual/components";
import { BLUR_MOVE, BOUNCE_MOVE, LOOP_DEFAULTS, POP_MOVE, PULSE_MOVE, SCALE_MOVE, TYPEWRITER, WIGGLE_MOVE, WORD_BY_WORD, ZOOM_DEFAULTS, ZOOM_MOVE, type WordStyle } from "@/app/lib/editor/core/text/looks";
import { charCount, SOLID, wordCount, type Motion } from "@/app/lib/editor/core/visual/render";
import { dipColor, incomingFx, outgoingFx, overlaps, WIPE, type MaskShape, type TransitionFx } from "@/app/lib/editor/core/visual/transitions";
import { gapOf } from "@/app/lib/editor/core/timeline/rules";
import type { Canvas, Item, Media, Project, Track } from "@/app/lib/editor/core/timeline/types";

export interface Visual {
  readonly item: Item;
  readonly track: Track;
  readonly opacity: number;
  /** Offset and wipe from the item's in/out animation (opacity is already in `opacity`). */
  readonly motion: Motion;
  /** A wipe transition into it: the share of its width shown, from the left edge (0–1). Absent when whole. */
  readonly wipe?: number;
  /** An iris transition into it (D79): the share of the canvas's half diagonal a circle at its centre shows (0–1; transitions.ts irisRadius). Absent when whole. */
  readonly iris?: number;
  /** A reveal shape into it (D133, transitions.ts maskDistance): the incoming clip shows inside it at share p. Absent when whole. */
  readonly mask?: { readonly shape: MaskShape; readonly p: number };
  /**
   * A push or zoom transition moving it (D79): `shift` canvas px to the right, `zoom` its picture
   * scaled about its box's centre. Already in `box` when it has one; an item without one (text, a
   * background, a graphic) moves as the whole canvas. Absent when not moved.
   */
  readonly shift?: number;
  /** A vertical push or slide (D133): canvas px down, like `shift`. */
  readonly shiftY?: number;
  readonly zoom?: number;
  /** A blur dissolve (D133): Gaussian sigma in canvas px. */
  readonly blur?: number;
  /** Source time in seconds (0 for items without media); a looped clip wraps at its media's end. */
  readonly source: number;
  /**
   * A clip of video or still media, or a drawn component (D76): its box at this frame (D75,
   * box.ts). Its opacity is already in `opacity`. A wipe reveals the canvas from the left, whatever
   * the box.
   */
  readonly box?: ClipBox;
}

/** An SVG image (D76): drawn by Chromium at its box's size in the export (ffmpeg can't decode SVG), as an <img> in the preview. */
export const isSvg = (media: Pick<Media, "kind" | "path"> | undefined) => !!media && media.kind === "image" && /\.svg$/i.test(media.path);

/** The media a clip draws (video or still), if any. */
export function pictureMedia(project: Partial<Pick<Project, "media">>, item: Item): Media | undefined {
  if ((item.type !== "clip" && item.type !== "danmaku") || !item.media) return undefined;
  const media = project.media?.find((m) => m.id === item.media);
  return media && media.kind !== "audio" ? media : undefined;
}

/**
 * The frames an item is drawn over and how its transitions meet it. Off the main track it's just
 * its own frames. `half` and `nextHalf` are the overlaps at its cuts (an overlapping transition
 * into it, and out of it into the next item); `skipIn` / `skipOut` say a transition blends that
 * side, so the item's own in/out animation doesn't apply there.
 */
export interface Span {
  readonly from: number;
  readonly to: number;
  readonly half: number;
  readonly nextHalf: number;
  readonly skipIn: boolean;
  readonly skipOut: boolean;
  /** The transition into it is a wipe (it is revealed from the left over [from, from + 2·half)). */
  readonly wipe: boolean;
  /** The overlapping transitions' modules at its cuts: into it (when half > 0) and out of it (when nextHalf > 0). */
  readonly into?: string;
  readonly next?: string;
}

export function spanOf(track: Track, index: number, start: number): Span {
  const item = track.items[index]!;
  const main = track.role === "main";
  const into = main ? item.transitionIn : undefined;
  const out = main ? track.items[index + 1]?.transitionIn : undefined;
  const half = into && overlaps(into.module) ? into.duration / 2 : 0;
  const nextHalf = out && overlaps(out.module) ? out.duration / 2 : 0;
  return {
    from: start - half,
    to: start + item.duration + nextHalf,
    half,
    nextHalf,
    skipIn: !!into,
    skipOut: !!out,
    wipe: half > 0 && into!.module === WIPE,
    ...(half > 0 ? { into: into!.module } : {}),
    ...(nextHalf > 0 ? { next: out!.module } : {}),
  };
}

/** The share done of the transition into an item (`side` "in") or out of it at frame f: 0 → 1 over its frames, linear. */
export function transitionShare(span: Pick<Span, "from" | "to" | "half" | "nextHalf">, f: number, side: "in" | "out"): number {
  const clamp = (p: number) => Math.min(1, Math.max(0, p));
  return side === "in" ? clamp((f - span.from) / (2 * span.half)) : clamp((f - (span.to - 2 * span.nextHalf)) / (2 * span.nextHalf));
}

/**
 * What the overlapping transitions at an item's cuts do to it at frame f (D79; transitions.ts
 * incomingFx / outgoingFx): opacity, a wipe or iris reveal, a push shift or a zoom. Nothing on
 * frames outside them.
 */
export function transitionAt(span: Span, f: number, canvas: { readonly width: number; readonly height: number }): TransitionFx {
  let fx: TransitionFx = { opacity: 1 };
  if (span.half > 0) {
    const p = transitionShare(span, f, "in");
    if (p < 1) fx = incomingFx(span.into!, p, canvas);
  }
  if (span.nextHalf > 0) {
    const p = transitionShare(span, f, "out");
    if (p > 0) fx = { ...outgoingFx(span.next!, p, canvas), opacity: fx.opacity };
  }
  return fx;
}

/** The id of the black layer a dip to black into `itemId` draws (not a valid item id, so it can't collide). */
export const dipId = (itemId: string) => `dip:${itemId}`;

/** A dip to black's black layer over the main track: [cut − half, cut + half), opaque on the cut. */
export interface Dip {
  readonly item: Item;
  readonly cut: number;
  readonly half: number;
}

/**
 * The colour layers of the dips (to black, to white) and flashes on a main track, in cut order
 * (D133: the colour is the transition's; a flash overlaps the clips, a dip doesn't, the layer is
 * the same).
 */
export function dipsOf(track: Track, starts: ReadonlyMap<string, number>): Array<Dip> {
  if (track.role !== "main") return [];
  const out: Array<Dip> = [];
  track.items.forEach((item, index) => {
    const tr = item.transitionIn;
    const color = tr && dipColor(tr.module);
    if (index === 0 || !tr || !color || !(tr.duration > 0)) return;
    out.push({ item: { id: dipId(item.id), type: "component", component: SOLID, duration: tr.duration, props: { color } }, cut: starts.get(item.id)!, half: tr.duration / 2 });
  });
  return out;
}

/** A dip's opacity at frame f: up from 0 at cut − half to 1 on the cut, back to 0 at cut + half. */
export const dipOpacity = (dip: Pick<Dip, "cut" | "half">, f: number) => Math.min(1, Math.max(0, f < dip.cut ? (f - (dip.cut - dip.half)) / dip.half : 1 - (f - dip.cut) / dip.half));

/** Start frame of every item and the project length. */
export function layout(project: Pick<Project, "tracks">): { starts: Map<string, number>; duration: number } {
  const starts = new Map<string, number>();
  let duration = 1;
  for (const track of project.tracks) {
    let t = 0;
    for (const item of track.items) {
      // D224: a main item starts after its gap.
      if (track.role === "main") t += gapOf(item);
      const start = track.role === "main" ? t : item.start!;
      if (track.role === "main") t += item.duration;
      starts.set(item.id, start);
      // Captions (D82) draw only where their speech plays: their window doesn't make the video longer.
      if (item.type !== "captions") duration = Math.max(duration, start + item.duration);
    }
  }
  return { starts, duration };
}

/** Everything visible at frame f, bottom layer first, with opacity and source time in seconds. */
export function visualsAt(project: Pick<Project, "tracks" | "canvas" | "media" | "theme">, starts: ReadonlyMap<string, number>, f: number): Array<Visual> {
  const out: Array<Visual> = [];
  for (const track of [...project.tracks].reverse()) {
    if (track.hidden || track.kind === "audio") continue;
    track.items.forEach((item, i) => {
      if(item.type==='danmaku'&&!item.danmaku?.settings.enabled)return;
      // Captions (D82): the card at this frame, as the text item core textHtml draws.
      if (item.type === "captions") {
        const text = captionAt(project, item, f);
        if (text) out.push({ item: text, track, opacity: 1, motion: {}, source: 0 });
        return;
      }
      const start = starts.get(item.id)!;
      const span = spanOf(track, i, start);
      if (f < span.from || f >= span.to) return;
      // The overlapping transitions at its cuts: a crossfade ramps the opacity, a wipe or an iris
      // reveals, a push or zoom moves it (D73, D79).
      const fx = transitionAt(span, f, project.canvas);
      let opacity = fx.opacity;
      // A main-track transition owns the blend at its cut: the in/out animation on that side is skipped.
      const anim = animationAt(item, f - start, project.canvas, { skipIn: span.skipIn, skipOut: span.skipOut });
      opacity *= anim.opacity;
      const source = item.media ? sourceAt({ media: project.media ?? [], canvas: project.canvas }, item, f - start) : 0;
      const media = pictureMedia(project, item);
      // A zoom pulse (D136) is the clip's box zoom at this frame.
      const own = media ? (fxSkips(media) ? clipBox(item, project.canvas, media.probe, f - start) : fxBox(item, clipBox(item, project.canvas, media.probe, f - start), f - start, project.canvas.fps)) : componentBox(item, project.canvas, f - start);
      const box = own && moveBox(own, fx.shift, fx.zoom, fx.shiftY);
      if (box) opacity *= box.opacity;
      // A graphic (D78) has no box: transform.opacity alone applies.
      if (item.type === "graphic" && typeof item.transform?.opacity === "number") opacity *= Math.min(1, Math.max(0, item.transform.opacity));
      const moved = { ...(fx.shift ? { shift: fx.shift } : {}), ...(fx.shiftY ? { shiftY: fx.shiftY } : {}), ...(fx.zoom !== undefined && fx.zoom !== 1 ? { zoom: fx.zoom } : {}) };
      const extra = { ...(fx.mask ? { mask: fx.mask } : {}), ...(fx.blur ? { blur: fx.blur } : {}) };
      out.push({ item, track, opacity, motion: anim.motion, source, ...(fx.wipe !== undefined ? { wipe: fx.wipe } : {}), ...(fx.iris !== undefined ? { iris: fx.iris } : {}), ...moved, ...extra, ...(box ? { box } : {}) });
    });
    // Dips to black: a black layer over the main track's clips, under the tracks above it.
    for (const dip of dipsOf(track, starts)) {
      if (f >= dip.cut - dip.half && f < dip.cut + dip.half) out.push({ item: dip.item, track, opacity: dipOpacity(dip, f), motion: {}, source: 0 });
    }
  }
  return out;
}

// The built-in moves (spec/modules/builtin/move.*.json) the preview and export draw. word-by-word
// and zoom (D81) draw on text; on other items they show the item without motion.
export const DRAWN_MOVES = ["move.fade", "move.rise", "move.slide-left", "move.wipe", WORD_BY_WORD, ZOOM_MOVE, TYPEWRITER, BOUNCE_MOVE, BLUR_MOVE, SCALE_MOVE, POP_MOVE, PULSE_MOVE, WIGGLE_MOVE] as const;

/**
 * Whether an in/out move changes the item's opacity (linearly over its frames, as animationAt does):
 * fade, rise and slide-left; zoom on text unless params.fade is false (D81). The export draws those
 * as alpha ramps; the rest of a move is in the drawn look.
 */
export function moveFades(item: Partial<Pick<Item, "text">>, move: { readonly module: string; readonly params?: { readonly [key: string]: unknown } | undefined }): boolean {
  if (move.module === "move.fade" || move.module === "move.rise" || move.module === "move.slide-left") return true;
  // Bounce and blur fade linearly on text (D163); pop's quick fade is drawn in the HTML (motion.fade), not a ramp.
  if ((move.module === BOUNCE_MOVE || move.module === BLUR_MOVE) && typeof item.text === "string") return true;
  return move.module === ZOOM_MOVE && typeof item.text === "string" && move.params?.fade !== false;
}

const easeOut = (p: number) => 1 - (1 - p) ** 3;
/** Back-out: overshoots a little past 1 before settling (a pop, D163). */
const backOut = (p: number) => 1 + 2.70158 * (p - 1) ** 3 + 1.70158 * (p - 1) ** 2;
/** Bounce-out: lands and bounces three times, smaller each time (a bounce, D163). */
export function bounceOut(p: number): number {
  const n = 7.5625;
  const d = 2.75;
  if (p < 1 / d) return n * p * p;
  if (p < 2 / d) return n * (p -= 1.5 / d) * p + 0.75;
  if (p < 2.5 / d) return n * (p -= 2.25 / d) * p + 0.9375;
  return n * (p -= 2.625 / d) * p + 0.984375;
}
const clamp01 = (p: number) => Math.min(1, Math.max(0, p));

/**
 * A move's ease from its params.ease (D192): a cubic ease by name (linear, in, out, inOut) or a
 * spring preset (springEase: the spring fitted to the move's duration, overshoot included);
 * anything else (or none) is `fallback`, the move's own curve.
 */
export function moveEase(params: { readonly [key: string]: unknown } | undefined, fallback: (p: number) => number): (p: number) => number {
  const e = params?.ease;
  if (isSpringName(e)) return springEase(e);
  if (typeof e === "string" && Object.hasOwn(EASES, e)) return EASES[e as Ease];
  return fallback;
}

/**
 * A word-by-word move's word progress (D81): n words arrive one after another, each over 1/n of the
 * move; p is the move's share (0 → 1 in; 1 → 0 out, words leaving first-to-last). In, word i is at
 * clamp(p·n − i): none on the first frame, then ceil(local·n / duration) words have started.
 */
export function wordProgress(n: number, p: number, out: boolean): Array<number> {
  return Array.from({ length: n }, (_, i) => clamp01(out ? p * n - (n - 1 - i) : p * n - i));
}

/** A zoom move's scale (D81): `scale` → 1 over the move in, 1 → `scale` out, shaped by the ease (time share t). */
export function zoomScale(p: number, out: boolean, params: { readonly [key: string]: unknown } | undefined): number {
  const scale = typeof params?.scale === "number" && params.scale > 0 ? params.scale : ZOOM_DEFAULTS.scale;
  const ease = moveEase(params, EASES[ZOOM_DEFAULTS.ease]);
  return out ? 1 + (scale - 1) * ease(1 - p) : scale + (1 - scale) * ease(p);
}

/**
 * An item's in and out animation at `local` frames from its start. The in-animation runs from 0
 * at the first frame to 1 at `duration`; the out-animation from 1 to 0 on the item's last frame,
 * so a faded item is fully transparent on its first and last frame. Fade changes opacity
 * linearly; rise and slide-left also move (eased) by 4 % of the height / 5 % of the width; wipe
 * reveals the box from the left. On text (an item with `text`), word-by-word reveals the words in
 * turn and zoom scales the text about its centre, fading unless params.fade is false (D81).
 * params.ease (D192, moveEase) reshapes a move's motion (not its fade, which stays linear): a cubic
 * ease or a spring preset fitted to the move's duration, so rise, slide-left, zoom, scale, pop,
 * bounce and blur may overshoot and settle; wipe's reveal and pop's starting size stay in range.
 */
export function animationAt(item: Pick<Item, "duration" | "animation"> & Partial<Pick<Item, "text">>, local: number, canvas: Pick<Canvas, "width" | "height">, { skipIn = false, skipOut = false } = {}): { opacity: number; motion: Motion } {
  let opacity = 1;
  let dx = 0;
  let dy = 0;
  let reveal = 1;
  let scale = 1;
  let words: Array<number> | undefined;
  let wordStyle: WordStyle | undefined;
  let chars: number | undefined;
  let blur = 0;
  let rotate = 0;
  let fade = 1;
  const isText = typeof item.text === "string";
  const apply = (module: string, p: number, out: boolean, params: { readonly [key: string]: unknown } | undefined) => {
    const e = moveEase(params, easeOut)(p);
    if (module === "move.fade") opacity *= p;
    else if (module === "move.rise") {
      opacity *= p;
      dy += (out ? -1 : 1) * (1 - e) * canvas.height * 0.04;
    } else if (module === "move.slide-left") {
      opacity *= p;
      dx += (out ? -1 : 1) * (1 - e) * canvas.width * 0.05;
    } else if (module === "move.wipe") reveal = Math.min(reveal, clamp01(e));
    else if (module === ZOOM_MOVE && isText) {
      scale *= zoomScale(p, out, params);
      if (params?.fade !== false) opacity *= p;
    } else if (module === WORD_BY_WORD && isText) {
      const next = wordProgress(wordCount(item.text!), p, out);
      words = words ? words.map((w, i) => Math.min(w, next[i]!)) : next;
      if (!wordStyle) wordStyle = typeof params?.style === "string" && ["pop", "fade", "rise"].includes(params.style) ? (params.style as WordStyle) : "pop";
    } else if (module === TYPEWRITER && isText) {
      // D163: floor(p·n) characters show: none on the first frame in, all of them at the end.
      const n = charCount(item.text!);
      const shown = Math.floor(p * n + 1e-9);
      chars = chars === undefined ? shown : Math.min(chars, shown);
    } else if (module === BOUNCE_MOVE && isText) {
      opacity *= p;
      dy -= (1 - moveEase(params, bounceOut)(p)) * canvas.height * 0.08;
    } else if (module === BLUR_MOVE && isText) {
      opacity *= p;
      blur += Math.max(0, 1 - e) * 2.5;
    } else if (module === SCALE_MOVE && isText) scale *= Math.max(0, e);
    else if (module === POP_MOVE && isText) {
      scale *= Math.max(0, 0.4 + 0.6 * moveEase(params, backOut)(p));
      fade *= Math.min(1, p * 3);
    }
  };
  const a = item.animation;
  // Outside the item's own frames (a transition's pre-roll and tail) it holds its first / last
  // state; with skipIn/skipOut (the transition at that cut blends instead) it doesn't apply.
  const at = Math.max(0, Math.min(item.duration - 1, local));
  if (!skipIn && a?.in && a.in.duration > 0 && at < a.in.duration) apply(a.in.module, clamp01(at / a.in.duration), false, a.in.params);
  if (!skipOut && a?.out && a.out.duration > 0) {
    const left = item.duration - 1 - at;
    if (left < a.out.duration) apply(a.out.module, clamp01(left / a.out.duration), true, a.out.params);
  }
  // A loop (D163): one period every `duration` frames for the item's whole length (phase from the
  // frame number, so equal phases draw byte-equal HTML and the export dedupes them).
  if (a?.loop && a.loop.duration > 0 && isText) {
    const wave = Math.sin((2 * Math.PI * (at % a.loop.duration)) / a.loop.duration);
    const amount = typeof a.loop.params?.amount === "number" ? a.loop.params.amount : LOOP_DEFAULTS[a.loop.module] ?? 0;
    if (a.loop.module === PULSE_MOVE) scale *= 1 + amount * (1 - Math.cos((2 * Math.PI * (at % a.loop.duration)) / a.loop.duration)) / 2;
    else if (a.loop.module === WIGGLE_MOVE) rotate += amount * wave;
  }
  const motion: { dx?: number; dy?: number; reveal?: number; scale?: number; words?: Array<number>; wordStyle?: WordStyle; chars?: number; blur?: number; rotate?: number; fade?: number } = {};
  if (dx) motion.dx = dx;
  if (dy) motion.dy = dy;
  if (reveal < 1) motion.reveal = reveal;
  if (scale !== 1) motion.scale = scale;
  if (words && words.some((w) => w < 1)) {
    motion.words = words;
    motion.wordStyle = wordStyle ?? "pop";
  }
  if (chars !== undefined && chars < charCount(item.text!)) motion.chars = chars;
  if (blur > 0) motion.blur = Math.round(blur * 1000) / 1000;
  if (rotate) motion.rotate = Math.round(rotate * 1000) / 1000;
  if (fade < 1) motion.fade = fade;
  return { opacity, motion };
}
