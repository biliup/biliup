// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The clip box model (D75), shared by the preview, export and the agent's frame renders.
// A clip (video or still) draws inside a box on the canvas: transform {x, y, w, h} in canvas
// pixels, the box's top-left and size. `fit` places the picture in the box at scale 1: contain
// (all of it, inside the box), cover (fills the box, the overflow cut off) or fill (stretched to
// the box). `scale` zooms the picture about the box's centre; the box clips it, so a scale above 1
// is a zoom in (a Ken Burns when keyed) and the box itself never changes size. `opacity`
// multiplies the clip's opacity; `radius` rounds the box's corners (canvas pixels).
// Defaults: no w and h = the whole canvas; only one = the other from the media's aspect (else the
// canvas's); no x / y = centred on the canvas. So a clip without a transform is drawn as before
// D75: fitted to the full frame, contain.
// `keys` animate x, y, scale and opacity: [{frame, ease?, x?, y?, scale?, opacity?}], frames from
// the item's start. Each property uses only the keys that carry it: before the first it holds the
// first value, after the last the last; between two it moves from one to the next, shaped by the
// `ease` of the key it leaves (linear when absent; in, out and inOut are cubic). A property
// without keys takes the transform's value.
// A key's ease may also name a spring (D192, spring.ts): the move from that key to the next one is
// then a real-time spring that starts at the key's frame (seconds = frames / the canvas's fps, 30
// when a caller has none) and heads for the next key's value, settling on its own time (it may run
// past the next key's frame, overshoot included). With any spring among a property's keys its value
// is a sum: the first key's value plus, for each move between two keys, the step (next − this)
// times that move's progress (a spring's response; a cubic or linear ease's share of its frames,
// 0 before, 1 after). A later move that starts before an earlier spring settled adds on top, so
// the earlier one's rest still plays out: the motion stays continuous and keeps its velocity
// ("sum of springs" retargeting). Without springs keyAt is exactly the plain interpolation above.
import type { Canvas, Item, MediaProbe } from "@/app/lib/editor/core/timeline/types";
import { isSpringName, springAt, type SpringName } from "@/app/lib/editor/core/visual/spring";

export type Fit = "contain" | "cover" | "fill";
export type Ease = "linear" | "in" | "out" | "inOut";
/** What a key's ease may be: a cubic ease or a spring preset (D192). */
export type KeyEase = Ease | SpringName;
/** The fps keyed springs run at when the caller has no canvas fps (D192). */
export const DEFAULT_KEY_FPS = 30;
export type KeyProp = "x" | "y" | "scale" | "opacity";
export const KEY_PROPS: ReadonlyArray<KeyProp> = ["x", "y", "scale", "opacity"];
export const FITS: ReadonlyArray<Fit> = ["contain", "cover", "fill"];

/** The eases a key may name (cubic; the same curve as the text moves' ease-out). */
export const EASES: Readonly<Record<Ease, (p: number) => number>> = {
  linear: (p) => p,
  in: (p) => p ** 3,
  out: (p) => 1 - (1 - p) ** 3,
  inOut: (p) => (p < 0.5 ? 4 * p ** 3 : 1 - (-2 * p + 2) ** 3 / 2),
};

export interface Rect {
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly h: number;
}

type Key = NonNullable<Item["keys"]>[number];
export interface PropertyKey { readonly frame: number; readonly value: number; readonly ease: KeyEase }

/** The keys that carry `prop`, by frame (a later key at the same frame wins). */
export function keysOf(keys: Item["keys"], prop: KeyProp): Array<PropertyKey> {
  const out: Array<PropertyKey> = [];
  for (const k of [...(keys ?? [])].sort((a, b) => a.frame - b.frame)) {
    const value = (k as Key)[prop];
    if (typeof value !== "number" || !Number.isFinite(value)) continue;
    const ease: KeyEase = typeof k.ease === "string" && (Object.hasOwn(EASES, k.ease) || isSpringName(k.ease)) ? (k.ease as KeyEase) : "linear";
    if (out.length && out.at(-1)!.frame === k.frame) out[out.length - 1] = { frame: k.frame, value, ease };
    else out.push({ frame: k.frame, value, ease });
  }
  return out;
}

/** Whether any key of `keys` eases with a spring (D192). */
export const hasSpringKeys = (keys: Item["keys"]) => (keys ?? []).some((k) => isSpringName(k.ease));

/**
 * A keyed property at `local` frames from the item's start, or undefined when no key carries it.
 * `fps` times spring keys (D192; DEFAULT_KEY_FPS when not given); a pure function of `local`.
 */
export function keyAt(keys: Item["keys"], prop: KeyProp, local: number, fps: number = DEFAULT_KEY_FPS): number | undefined {
  return propertyKeyAt(keysOf(keys, prop), local, fps);
}

/** Evaluate an already sorted property curve, reused while sampling a cut's preserved motion. */
export function propertyKeyAt(list: ReadonlyArray<PropertyKey>, local: number, fps: number): number | undefined {
  if (!list.length) return undefined;
  if (local <= list[0]!.frame) return list[0]!.value;
  if (list.some((k) => isSpringName(k.ease))) {
    // Sum of moves (see the header): each move's step times its progress at `local`.
    const rate = Number.isFinite(fps) && fps > 0 ? fps : DEFAULT_KEY_FPS;
    let v = list[0]!.value;
    for (let i = 1; i < list.length; i++) {
      const a = list[i - 1]!;
      const b = list[i]!;
      if (local <= a.frame) break;
      const step = b.value - a.value;
      if (isSpringName(a.ease)) v += step * springAt(a.ease, (local - a.frame) / rate);
      else v += step * (local >= b.frame ? 1 : EASES[a.ease]((local - a.frame) / (b.frame - a.frame)));
    }
    return v;
  }
  for (let i = 1; i < list.length; i++) {
    const a = list[i - 1]!;
    const b = list[i]!;
    if (local < b.frame) return a.value + (b.value - a.value) * EASES[a.ease as Ease]((local - a.frame) / (b.frame - a.frame));
  }
  return list.at(-1)!.value;
}

/** A clip's box at one frame (canvas pixels). */
export interface ClipBox extends Rect {
  readonly fit: Fit;
  readonly scale: number;
  readonly opacity: number;
  /** Corner radius, at most half the box's shorter side. */
  readonly radius: number;
  /** Where the whole picture lands (fit, then scale about the box's centre); the box clips it. */
  readonly content: Rect;
}

type MediaSize = Pick<MediaProbe, "width" | "height"> | undefined;

/** The box's x, y, w and h with the defaults filled in, before keys (see the header). */
export function baseBox(item: Pick<Item, "transform">, canvas: Pick<Canvas, "width" | "height">, media?: MediaSize): Rect {
  const t = item.transform ?? {};
  const aspect = media?.width && media?.height ? media.width / media.height : canvas.width / canvas.height;
  const pos = (n: number | undefined) => (typeof n === "number" && Number.isFinite(n) ? n : undefined);
  const size = (n: number | undefined) => (typeof n === "number" && Number.isFinite(n) && n > 0 ? n : undefined);
  let w = size(t.w);
  let h = size(t.h);
  if (w === undefined && h === undefined) {
    w = canvas.width;
    h = canvas.height;
  } else if (w === undefined) w = h! * aspect;
  else if (h === undefined) h = w / aspect;
  return { x: pos(t.x) ?? (canvas.width - w) / 2, y: pos(t.y) ?? (canvas.height - h!) / 2, w, h: h! };
}

/** Where a picture of `media` size lands in `box` at scale 1 for a fit (no size known: the box). */
export function fitRect(box: Rect, fit: Fit, media?: MediaSize): Rect {
  if (fit === "fill" || !media?.width || !media?.height) return box;
  const k = (fit === "cover" ? Math.max : Math.min)(box.w / media.width, box.h / media.height);
  const w = media.width * k;
  const h = media.height * k;
  return { x: box.x + (box.w - w) / 2, y: box.y + (box.h - h) / 2, w, h };
}

const clampOpacity = (o: number) => Math.min(1, Math.max(0, o));

/** A clip's box at `local` frames from its start, keys applied. */
export function clipBox(item: Pick<Item, "transform" | "keys">, canvas: Pick<Canvas, "width" | "height"> & { readonly fps?: number }, media: MediaSize, local: number): ClipBox {
  const base = baseBox(item, canvas, media);
  const t = item.transform ?? {};
  const fps = canvas.fps ?? DEFAULT_KEY_FPS;
  const x = keyAt(item.keys, "x", local, fps) ?? base.x;
  const y = keyAt(item.keys, "y", local, fps) ?? base.y;
  const rawScale = keyAt(item.keys, "scale", local, fps) ?? (typeof t.scale === "number" && t.scale > 0 ? t.scale : 1);
  const scale = Math.max(0.001, rawScale);
  const opacity = clampOpacity(keyAt(item.keys, "opacity", local, fps) ?? (typeof t.opacity === "number" ? t.opacity : 1));
  const fit: Fit = t.fit && FITS.includes(t.fit) ? t.fit : "contain";
  const box = { x, y, w: base.w, h: base.h };
  const at1 = fitRect(box, fit, media);
  const cx = x + base.w / 2;
  const cy = y + base.h / 2;
  const content = { x: cx + (at1.x - cx) * scale, y: cy + (at1.y - cy) * scale, w: at1.w * scale, h: at1.h * scale };
  const r = (t as { radius?: unknown }).radius;
  const radius = typeof r === "number" && r > 0 ? Math.min(r, base.w / 2, base.h / 2) : 0;
  return { ...box, fit, scale, opacity, radius, content };
}

/** The box a clip without a transform gets: the whole canvas, contain, scale 1 (how clips drew before D75). */
export const isPlainBox = (box: ClipBox, canvas: Pick<Canvas, "width" | "height">) =>
  box.x === 0 && box.y === 0 && box.w === canvas.width && box.h === canvas.height && box.fit === "contain" && box.scale === 1 && box.radius === 0;

/** Whether an item has anything the box model draws differently from a plain clip at some frame. */
export const hasKeys = (item: Pick<Item, "keys">, prop?: KeyProp) => (prop ? keysOf(item.keys, prop).length > 0 : KEY_PROPS.some((p) => keysOf(item.keys, p).length > 0));

/** The whole canvas as a box (fill, scale 1): what a transition moves for an item without a box of its own (text, backgrounds, graphics). */
export const canvasBox = (canvas: Pick<Canvas, "width" | "height">): ClipBox => {
  const r = { x: 0, y: 0, w: canvas.width, h: canvas.height };
  return { ...r, fit: "fill", scale: 1, opacity: 1, radius: 0, content: r };
};

/**
 * A box moved by a transition (D79): `shift` canvas px to the right (push), the picture `zoom`
 * times larger about the box's centre (zoom: like a scale key, the box clips it). Size, radius and
 * opacity unchanged. The same box with no move.
 */
export function moveBox(box: ClipBox, shift = 0, zoom = 1, shiftY = 0): ClipBox {
  if (!shift && !shiftY && zoom === 1) return box;
  const cx = box.x + box.w / 2;
  const cy = box.y + box.h / 2;
  const c = box.content;
  return { ...box, x: box.x + shift, y: box.y + shiftY, scale: box.scale * zoom, content: { x: cx + (c.x - cx) * zoom + shift, y: cy + (c.y - cy) * zoom + shiftY, w: c.w * zoom, h: c.h * zoom } };
}
