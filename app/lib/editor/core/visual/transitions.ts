// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The main-track transitions the preview and export draw (D73, D79), and how long one can be at a cut.
// Modules from plugins (not built in) have no known limits here; the server still checks theirs.
// Browser-safe: the renderer's Transitions panel and Inspector, the server's export and the
// agent's docs all read this list. The limits are the modules' manifests'
// (spec/modules/builtin/transition.*.json; test/core/transitions.test.ts keeps them in step).
//
// How each one draws (timeline.ts visualsAt via transitionAt; export follows it). Every one but dip
// to black overlaps: centred on the cut, the outgoing clip runs half the transition past its end and
// the incoming one starts half before its start, on top; video needs media handles on both sides
// and the sound crossfades over the same frames. p is the share of the transition done, linear in
// frames (0 at its first frame's moment, 1 at its end); none of them ease.
// - transition.crossfade: the incoming clip fades in (opacity p).
// - transition.wipe: the incoming clip, fully opaque, is shown from the left edge of the canvas up
//   to p of its width.
// - transition.iris (D79): the incoming clip, fully opaque, is shown inside a circle at the canvas's
//   centre whose radius grows from 0 to the half diagonal (p · hypot(W, H) / 2: the corners at 1).
// - transition.push-left (D79): both clips move left by p of the canvas width, side by side: the
//   outgoing one shifted −p·W, the incoming one +(1 − p)·W; both opaque.
// - transition.zoom (D79): the outgoing clip's picture grows from 1 to ZOOM_SCALE (linear in p)
//   about its box's centre (the canvas's for a full-frame clip), while the incoming clip fades in
//   over it exactly like a crossfade.
// - transition.dip-to-black: no overlap and no handles. A black layer over the main track fades
//   in over the first half (the outgoing clip's last frames) and out over the second (the
//   incoming clip's first); the cut frame is black. The sound dips too: the outgoing clip fades
//   out over its last half, the incoming one fades in over its first.
// D133 adds 22 more, all from the same few primitives described by TransitionInfo (kind + params):
// - Reveals (wipe left/up/down, split, split horizontal, box, diamond, diagonal, clock wipe, blinds,
//   circle close): the incoming clip, opaque, shown inside a mask shape (maskDistance; the preview
//   clips to maskRings' outline, the export multiplies maskGeq's coverage into the alpha).
// - Moves: push right/up/down (both clips, `enter` and `exit`), slide left/right/up/down (only the
//   incoming clip), zoom out (the incoming picture 1.25 → 1 while it fades in).
// - Blur: both clips blurred (Gaussian sigma up to 2 % of the canvas height at the cut) under a crossfade.
// - Dips: dip to white (like dip to black) and flash (a white layer peaking on the cut over a crossfade).
// A move (push, zoom) applies to an item's box (box.ts moveBox: clips and components); items
// without one (text, backgrounds, graphics) move as the whole canvas. Iris and wipe reveal the
// canvas, whatever the box. transition.cut and plugin transitions draw as a crossfade.
import type { Item, Media, Project, Track } from "@/app/lib/editor/core/timeline/types";

export interface TransitionInfo {
  readonly id: string;
  readonly name: string;
  /** Limits in frames, from the module's manifest. */
  readonly minFrames: number;
  readonly maxFrames: number;
  /** The length a new one gets, in seconds (then clamped to the limits and the cut). */
  readonly defaultSeconds: number;
  /** The clips overlap at the cut (half the transition each side), so video needs handles. */
  readonly overlaps: boolean;
  readonly description: string;
  /** How it draws (D133): a fade, a reveal (a mask over the incoming clip), a move, or a dip through a colour. */
  readonly kind: TransitionKind;
  /** The incoming clip fades in (opacity p). */
  readonly fades?: boolean;
  /** A reveal's shape (D133): the incoming clip, opaque, is shown inside it (wipe and iris keep their own D73/D79 paths). */
  readonly mask?: MaskShape;
  /** The incoming clip starts this share of the canvas away (x right, y down) and moves to its place. */
  readonly enter?: { readonly x: number; readonly y: number };
  /** The outgoing clip moves this share of the canvas away by the end. */
  readonly exit?: { readonly x: number; readonly y: number };
  /** The incoming picture's scale at the start (to 1 at the end), about its box's centre. */
  readonly inZoom?: number;
  /** The outgoing picture's scale at the end (from 1), about its box's centre. */
  readonly outZoom?: number;
  /** A blur dissolve's strongest blur, as a share of the canvas height (Gaussian sigma). */
  readonly blur?: number;
  /** A colour layer over the main track across the cut (dip to black / white, flash). */
  readonly color?: string;
}

export type TransitionKind = "fade" | "reveal" | "move" | "dip";

/** The reveal shapes drawn by one mask each (D133). */
export type MaskShape = "wipe-left" | "wipe-up" | "wipe-down" | "split-v" | "split-h" | "box" | "diamond" | "diagonal" | "circle-close" | "clock" | "blinds";

export const CROSSFADE = "transition.crossfade";
export const WIPE = "transition.wipe";
export const DIP_TO_BLACK = "transition.dip-to-black";
export const IRIS = "transition.iris";
export const PUSH_LEFT = "transition.push-left";
export const ZOOM = "transition.zoom";

/** How much larger the outgoing picture is at the end of a zoom transition (D79). */
export const ZOOM_SCALE = 1.2;

/** Blinds: the number of slats (D133). */
export const BLINDS = 6;

const t = (id: string, name: string, kind: TransitionKind, description: string, rest: Partial<TransitionInfo> = {}): TransitionInfo => ({
  id: `transition.${id}`,
  name,
  kind,
  description,
  minFrames: 4,
  maxFrames: 30,
  defaultSeconds: 0.5,
  overlaps: true,
  ...rest,
});

export const TRANSITIONS: ReadonlyArray<TransitionInfo> = [
  t("crossfade", "Crossfade", "fade", "The next clip fades in over this one.", { minFrames: 2, maxFrames: 60, fades: true }),
  t("dip-to-black", "Dip to black", "dip", "Fades to black and back up; the sound dips with it. Needs no spare footage.", { maxFrames: 90, defaultSeconds: 0.8, overlaps: false, color: "#000000" }),
  t("wipe", "Wipe", "reveal", "The next clip is revealed from the left edge."),
  t("iris", "Iris", "reveal", "The next clip opens in a growing circle from the centre.", { minFrames: 6 }),
  t("push-left", "Push", "move", "The next clip slides in from the right, pushing this one out to the left.", { enter: { x: 1, y: 0 }, exit: { x: -1, y: 0 } }),
  t("zoom", "Zoom", "fade", "This clip zooms in as the next one fades in over it.", { fades: true, outZoom: ZOOM_SCALE }),
  // D133
  t("dip-to-white", "Dip to white", "dip", "Fades to white and back down; the sound dips with it. Needs no spare footage.", { maxFrames: 90, defaultSeconds: 0.8, overlaps: false, color: "#ffffff" }),
  t("flash", "Flash", "dip", "A white flash at the cut while the next clip fades in.", { minFrames: 2, defaultSeconds: 0.3, fades: true, color: "#ffffff" }),
  t("blur", "Blur", "fade", "This clip blurs out as the next one fades in sharp.", { minFrames: 6, maxFrames: 60, fades: true, blur: 0.02 }),
  t("wipe-left", "Wipe left", "reveal", "The next clip is revealed from the right edge.", { mask: "wipe-left" }),
  t("wipe-up", "Wipe up", "reveal", "The next clip is revealed from the bottom edge.", { mask: "wipe-up" }),
  t("wipe-down", "Wipe down", "reveal", "The next clip is revealed from the top edge.", { mask: "wipe-down" }),
  t("push-right", "Push right", "move", "The next clip slides in from the left, pushing this one out to the right.", { enter: { x: -1, y: 0 }, exit: { x: 1, y: 0 } }),
  t("push-up", "Push up", "move", "The next clip slides in from below, pushing this one out the top.", { enter: { x: 0, y: 1 }, exit: { x: 0, y: -1 } }),
  t("push-down", "Push down", "move", "The next clip slides in from above, pushing this one out the bottom.", { enter: { x: 0, y: -1 }, exit: { x: 0, y: 1 } }),
  t("slide-left", "Slide left", "move", "The next clip slides in from the right over this one.", { enter: { x: 1, y: 0 } }),
  t("slide-right", "Slide right", "move", "The next clip slides in from the left over this one.", { enter: { x: -1, y: 0 } }),
  t("slide-up", "Slide up", "move", "The next clip slides in from below over this one.", { enter: { x: 0, y: 1 } }),
  t("slide-down", "Slide down", "move", "The next clip slides in from above over this one.", { enter: { x: 0, y: -1 } }),
  t("circle-close", "Circle close", "reveal", "This clip closes into a shrinking circle over the next one.", { minFrames: 6, mask: "circle-close" }),
  t("clock", "Clock wipe", "reveal", "The next clip is revealed by a hand sweeping clockwise from 12.", { minFrames: 6, mask: "clock" }),
  t("split", "Split", "reveal", "The next clip opens from a vertical line in the middle.", { mask: "split-v" }),
  t("split-horizontal", "Split horizontal", "reveal", "The next clip opens from a horizontal line in the middle.", { mask: "split-h" }),
  t("box", "Box", "reveal", "The next clip opens in a growing rectangle from the centre.", { mask: "box" }),
  t("diamond", "Diamond", "reveal", "The next clip opens in a growing diamond from the centre.", { minFrames: 6, mask: "diamond" }),
  t("diagonal", "Diagonal", "reveal", "The next clip is revealed by a diagonal edge from the top left.", { mask: "diagonal" }),
  t("blinds", "Blinds", "reveal", "The next clip appears through slats that open downwards.", { minFrames: 6, mask: "blinds" }),
  t("zoom-out", "Zoom out", "fade", "The next clip fades in while settling from 125 % to its size.", { fades: true, inZoom: 1.25 }),
];

/** The modules the preview and export draw as themselves. */
export const DRAWN_TRANSITIONS: ReadonlyArray<string> = TRANSITIONS.map((t) => t.id);

export const transitionInfo = (module: string | undefined) => TRANSITIONS.find((t) => t.id === module);

/**
 * Frame limits of every built-in transition module (their manifests'; a test keeps them equal), so
 * a length set by hand is clamped as the server checks it. transition.cut has none.
 */
export const TRANSITION_LIMITS: Readonly<Record<string, { readonly minFrames: number; readonly maxFrames: number }>> = Object.fromEntries(TRANSITIONS.map((t) => [t.id, { minFrames: t.minFrames, maxFrames: t.maxFrames }]));

/** Whether a transition overlaps the clips at its cut (all but the dips to black / white; unknown ones draw as a crossfade). */
export const overlaps = (module: string) => transitionInfo(module)?.overlaps ?? true;

/**
 * What an overlapping transition does to the clips at its cut at share p (D79, D133): `opacity`
 * multiplies the clip's; `wipe` is the share of the canvas width shown from the left, `iris` the
 * share of the half diagonal a centred circle shows, `mask` a reveal shape at its share (all absent
 * when whole); `shift` / `shiftY` move it right / down by canvas px and `zoom` scales its picture
 * about its box's centre (absent when not moved); `blur` is a Gaussian sigma in canvas px.
 */
export interface TransitionFx {
  readonly opacity: number;
  readonly wipe?: number;
  readonly iris?: number;
  readonly mask?: { readonly shape: MaskShape; readonly p: number };
  readonly shift?: number;
  readonly shiftY?: number;
  readonly zoom?: number;
  readonly blur?: number;
}

const NONE: TransitionFx = { opacity: 1 };
type Size = { readonly width: number; readonly height: number };

/** The incoming clip at share p (0 ≤ p < 1) of an overlapping `module` transition into it. */
export function incomingFx(module: string, p: number, canvas: Size): TransitionFx {
  if (module === WIPE) return { opacity: 1, wipe: p };
  if (module === IRIS) return { opacity: 1, iris: p };
  const info = transitionInfo(module);
  // transition.cut and plugins: a fade.
  if (!info) return { opacity: p };
  const fx: { -readonly [K in keyof TransitionFx]: TransitionFx[K] } = { opacity: info.fades ? p : 1 };
  if (info.mask) fx.mask = { shape: info.mask, p };
  if (info.enter) {
    if (info.enter.x) fx.shift = (1 - p) * info.enter.x * canvas.width;
    if (info.enter.y) fx.shiftY = (1 - p) * info.enter.y * canvas.height;
  }
  if (info.inZoom !== undefined) fx.zoom = info.inZoom + (1 - info.inZoom) * p;
  if (info.blur) fx.blur = (1 - p) * info.blur * canvas.height;
  return fx;
}

/** The outgoing clip at share p (0 < p ≤ 1) of an overlapping `module` transition out of it. */
export function outgoingFx(module: string, p: number, canvas: Size): TransitionFx {
  const info = transitionInfo(module);
  if (!info) return NONE;
  const fx: { -readonly [K in keyof TransitionFx]: TransitionFx[K] } = { opacity: 1 };
  if (info.exit) {
    if (info.exit.x) fx.shift = p * info.exit.x * canvas.width;
    if (info.exit.y) fx.shiftY = p * info.exit.y * canvas.height;
  }
  if (info.outZoom !== undefined) fx.zoom = 1 + (info.outZoom - 1) * p;
  if (info.blur) fx.blur = p * info.blur * canvas.height;
  return Object.keys(fx).length === 1 ? NONE : fx;
}

/** Whether the incoming clip fades in (its opacity ramps) through an overlapping `module` transition. */
export const fadesIn = (module: string) => {
  const info = transitionInfo(module);
  return info ? info.fades === true : true;
};

/** Whether an overlapping `module` transition moves or zooms the outgoing clip (push, zoom). */
export const movesOutgoing = (module: string) => {
  const info = transitionInfo(module);
  return !!info && (!!info.exit || info.outZoom !== undefined);
};

/** Whether an overlapping `module` transition moves or zooms the incoming clip (push, slide, zoom out). */
export const movesIncoming = (module: string) => {
  const info = transitionInfo(module);
  return !!info && (!!info.enter || info.inZoom !== undefined);
};

/** Whether a `module` transition reveals the incoming clip (wipe, iris, a mask shape): the reveal is dropped at its end. */
export const reveals = (module: string) => module === WIPE || module === IRIS || !!transitionInfo(module)?.mask;

/** The reveal shape of a `module` transition (not wipe or iris, which keep their own paths), if any. */
export const maskOf = (module: string | undefined) => transitionInfo(module)?.mask;

/** Whether a `module` transition blurs the clips at its cut. */
export const blurs = (module: string | undefined) => !!transitionInfo(module)?.blur;

/** The colour layer a `module` transition puts over the main track across its cut (dips, flash), if any. */
export const dipColor = (module: string | undefined) => transitionInfo(module)?.color;

/** The iris's radius in canvas px at share `iris` (the half diagonal at 1, so the corners are in). */
export const irisRadius = (iris: number, canvas: { readonly width: number; readonly height: number }) => (iris * Math.hypot(canvas.width, canvas.height)) / 2;

// ---- reveal masks (D133) ----
// Every shape is a signed distance d(x, y) in px from its edge (> 0 inside, where the incoming clip
// shows) at share p, on a W×H picture. The preview clips to the shape's outline (maskRings → a CSS
// path, antialiased by Chromium); the export and render_frames multiply a geq coverage
// clip(d / step + 0.5) into the layer's alpha (maskGeq: the same d as an ffmpeg expression).

type Ring = ReadonlyArray<readonly [number, number]>;
const TAU = 2 * Math.PI;

/** The signed distance (px, > 0 inside) of the point (x, y) from `shape`'s edge at share p on a W×H picture. */
export function maskDistance(shape: MaskShape, p: number, W: number, H: number, x: number, y: number): number {
  const dx = x - W / 2;
  const dy = y - H / 2;
  switch (shape) {
    case "wipe-left":
      return x - (1 - p) * W;
    case "wipe-up":
      return y - (1 - p) * H;
    case "wipe-down":
      return p * H - y;
    case "split-v":
      return (p * W) / 2 - Math.abs(dx);
    case "split-h":
      return (p * H) / 2 - Math.abs(dy);
    case "box":
      return Math.min((p * W) / 2 - Math.abs(dx), (p * H) / 2 - Math.abs(dy));
    case "diamond":
      return (2 * p - Math.abs(dx) / (W / 2) - Math.abs(dy) / (H / 2)) / Math.hypot(2 / W, 2 / H);
    case "diagonal":
      return (2 * p - x / W - y / H) / Math.hypot(1 / W, 1 / H);
    case "circle-close":
      return Math.hypot(dx, dy) - ((1 - p) * Math.hypot(W, H)) / 2;
    case "clock": {
      const th = (Math.atan2(dx, -dy) + TAU) % TAU;
      const a = TAU * p;
      const angle = th < a ? Math.min(th, a - th) : -Math.min(th - a, TAU - th);
      return Math.hypot(dx, dy) * Math.sin(Math.min(Math.PI / 2, Math.max(-Math.PI / 2, angle)));
    }
    case "blinds": {
      const bh = H / BLINDS;
      return p * bh - (((y % bh) + bh) % bh);
    }
  }
}

/** maskDistance as an ffmpeg expression: `P` the share, `x` / `y` the pixel centre's expressions, W×H the picture. */
export function maskGeq(shape: MaskShape, P: string, W: number, H: number, x: string, y: string): string {
  const dx = `(${x}-${W / 2})`;
  const dy = `(${y}-${H / 2})`;
  switch (shape) {
    case "wipe-left":
      return `(${x}-(1-${P})*${W})`;
    case "wipe-up":
      return `(${y}-(1-${P})*${H})`;
    case "wipe-down":
      return `(${P}*${H}-${y})`;
    case "split-v":
      return `(${P}*${W / 2}-abs${dx})`;
    case "split-h":
      return `(${P}*${H / 2}-abs${dy})`;
    case "box":
      return `min(${P}*${W / 2}-abs${dx},${P}*${H / 2}-abs${dy})`;
    case "diamond":
      return `((2*${P}-abs${dx}/${W / 2}-abs${dy}/${H / 2})/${Math.hypot(2 / W, 2 / H)})`;
    case "diagonal":
      return `((2*${P}-(${x})/${W}-(${y})/${H})/${Math.hypot(1 / W, 1 / H)})`;
    case "circle-close":
      return `(hypot(${dx},${dy})-(1-${P})*${Math.hypot(W, H) / 2})`;
    case "clock": {
      const th = `mod(atan2(${dx},-${dy})+2*PI,2*PI)`;
      const a = `(2*PI*${P})`;
      return `(hypot(${dx},${dy})*sin(clip(if(lt(${th},${a}),min(${th},${a}-${th}),-min(${th}-${a},2*PI-${th})),-PI/2,PI/2)))`;
    }
    case "blinds": {
      const bh = H / BLINDS;
      return `(${P}*${bh}-mod(${y},${bh}))`;
    }
  }
}

/**
 * The outline of `shape` at share p on a W×H picture, as rings (even-odd: a hole is a ring inside
 * another), in its px. Straight edges reach well past the picture, so a moved element is cut only
 * at the shape's real edges.
 */
export function maskRings(shape: MaskShape, p: number, W: number, H: number): Array<Ring> {
  const rect = (x0: number, y0: number, x1: number, y1: number): Ring => [
    [x0, y0],
    [x1, y0],
    [x1, y1],
    [x0, y1],
  ];
  const [L, T, R, B] = [-W, -H, 2 * W, 2 * H];
  const cx = W / 2;
  const cy = H / 2;
  switch (shape) {
    case "wipe-left":
      return [rect((1 - p) * W, T, R, B)];
    case "wipe-up":
      return [rect(L, (1 - p) * H, R, B)];
    case "wipe-down":
      return [rect(L, T, R, p * H)];
    case "split-v":
      return [rect(cx - (p * W) / 2, T, cx + (p * W) / 2, B)];
    case "split-h":
      return [rect(L, cy - (p * H) / 2, R, cy + (p * H) / 2)];
    case "box":
      return [rect(cx - (p * W) / 2, cy - (p * H) / 2, cx + (p * W) / 2, cy + (p * H) / 2)];
    case "diamond":
      return [
        [
          [cx, cy - p * H],
          [cx + p * W, cy],
          [cx, cy + p * H],
          [cx - p * W, cy],
        ],
      ];
    case "diagonal":
      return [
        [
          [-2 * W, -H],
          [W * (2 * p + 1), -H],
          [W * (2 * p - 2), 2 * H],
          [-2 * W, 2 * H],
        ],
      ];
    case "circle-close": {
      const r = ((1 - p) * Math.hypot(W, H)) / 2;
      const n = 120;
      const circle: Array<[number, number]> = [];
      for (let i = 0; i < n; i++) circle.push([cx + r * Math.cos((TAU * i) / n), cy + r * Math.sin((TAU * i) / n)]);
      return [rect(L, T, R, B), circle];
    }
    case "clock": {
      if (p >= 1) return [rect(L, T, R, B)];
      const far = 2 * (W + H);
      const a = TAU * p;
      const steps = Math.max(1, Math.ceil(a / (Math.PI / 36)));
      const fan: Array<[number, number]> = [[cx, cy]];
      for (let i = 0; i <= steps; i++) {
        const t = (a * i) / steps;
        fan.push([cx + far * Math.sin(t), cy - far * Math.cos(t)]);
      }
      return [fan];
    }
    case "blinds": {
      const bh = H / BLINDS;
      return Array.from({ length: BLINDS }, (_, k) => rect(L, k * bh, R, k * bh + p * bh));
    }
  }
}

/**
 * A CSS clip-path for `shape` at share p on the canvas, in an element's own px: `map` takes a
 * canvas point to the element's (identity for a full-canvas element).
 */
export function maskClipPath(shape: MaskShape, p: number, canvas: Size, map: (x: number, y: number) => readonly [number, number] = (x, y) => [x, y]): string {
  const r = (n: number) => Math.round(n * 1000) / 1000;
  const d = maskRings(shape, p, canvas.width, canvas.height)
    .map((ring) => `M${ring.map(([x, y]) => map(x, y).map(r).join(" ")).join("L")}Z`)
    .join("");
  return `path(evenodd, "${d}")`;
}

/** A transition's name for people ("Crossfade"), or its id when it isn't one the panel knows. */
export const transitionName = (module: string) => transitionInfo(module)?.name ?? module.replace(/^transition\./, "");

/** A new transition's length at this fps, before the cut's limits. */
export const defaultFrames = (info: TransitionInfo, fps: number) => Math.min(info.maxFrames, Math.max(info.minFrames, Math.round(info.defaultSeconds * fps)));

export type TransitionProblem =
  | { readonly problem: string; readonly reason: 'track' | 'first' | 'gap' | 'handles' }
  | { readonly problem: string; readonly reason: 'length'; readonly min: number };
export type TransitionLimits = { readonly min: number; readonly max: number } | TransitionProblem;

/**
 * How long a `module` transition into the main item `itemId` can be: the module's limits, at most
 * half the shorter clip (rules.ts: both clips at least twice its length), and for an overlapping
 * one the media handles (rules.ts: ceil(d/2) frames of video before the incoming clip's in-point
 * and after the outgoing one's out-point). A problem, as a sentence, when no length fits.
 */
export function transitionLimits(project: Pick<Project, "tracks" | "media">, itemId: string, module: string): TransitionLimits {
  const track = project.tracks.find((t) => t.role === "main" && t.items.some((i) => i.id === itemId));
  if (!track) return { problem: "Transitions go between two clips on the main track.", reason: 'track' };
  const index = track.items.findIndex((i) => i.id === itemId);
  if (index < 1) return { problem: "The first clip on the main track has nothing to transition from.", reason: 'first' };
  if ((track.items[index]!.gap ?? 0) > 0) return { problem: "There's a gap before this clip: close it (move the clip up to the one before) to put a transition here.", reason: 'gap' };
  const limits = TRANSITION_LIMITS[module];
  const prev = track.items[index - 1]!;
  const item = track.items[index]!;
  const min = limits?.minFrames ?? 1;
  let max = Math.min(limits?.maxFrames ?? Infinity, Math.floor(Math.min(prev.duration, item.duration) / 2));
  if (max < min) return { problem: `The clips at this cut are too short for a ${transitionName(module).toLowerCase()}: both need at least ${2 * min} frames.`, reason: 'length', min };
  if (overlaps(module)) {
    const before = handleBefore(project.media, item);
    const after = handleAfter(project.media, prev);
    max = Math.min(max, 2 * before, 2 * after);
    if (max < min) {
      const short = before < after ? `"${item.id}" starts at the beginning of its footage` : `"${prev.id}" ends at the end of its footage`;
      return { problem: `Not enough footage for a ${transitionName(module).toLowerCase()} here: ${short}. Trim the clips at this cut, or use Dip to black.`, reason: 'handles' };
    }
  }
  return { min, max };
}

const videoOf = (media: ReadonlyArray<Media>, item: Item) => {
  const m = item.media ? media.find((x) => x.id === item.media) : undefined;
  return m?.kind === "video" ? m : undefined;
};

/** Whole timeline frames before the in-point, at the item's speed (Infinity for non-video). */
export function handleBefore(media: ReadonlyArray<Media>, item: Item): number {
  return videoOf(media, item) ? Math.floor(Math.max(0, item.in ?? 0) / (item.speed ?? 1)) : Infinity;
}

/** Whole timeline frames after the out-point; negative for a hold beyond the source's end.
 * A loop can continue forwards through the transition, but cannot read before source frame zero. */
export function handleAfter(media: ReadonlyArray<Media>, item: Item): number {
  const m = videoOf(media, item);
  if (!m?.probe?.frames) return Infinity;
  if (item.loop) return Infinity;
  return Math.floor((m.probe.frames - (item.in ?? 0)) / (item.speed ?? 1) - item.duration);
}

/** A main-track cut: the clip after it (whose transitionIn it is) and the frame it's on. */
export interface Cut {
  readonly item: Item;
  readonly prev: Item;
  readonly index: number;
  readonly frame: number;
}

/** Every cut between two main-track items, in time order. Two items with a gap between them (D224) don't meet: no cut there. */
export function cutsOf(track: Track | undefined): Array<Cut> {
  if (!track || track.role !== "main") return [];
  const out: Array<Cut> = [];
  let t = 0;
  track.items.forEach((item, index) => {
    const gap = typeof item.gap === "number" && item.gap > 0 ? item.gap : 0;
    t += gap;
    if (index > 0 && gap === 0) out.push({ item, prev: track.items[index - 1]!, index, frame: t });
    t += item.duration;
  });
  return out;
}
