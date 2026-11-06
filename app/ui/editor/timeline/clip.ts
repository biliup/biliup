// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// A clip's box on the canvas (phase 6 chunk 1, D75): pure planners for the Inspector's "Position
// & size" group, the canvas drag and resize handles and the keyframe diamonds. Like the text ones
// (D53), every gesture is one update_item with the complete transform / keys built from the item
// as it is when the edit runs, so it is validated, undoes in one step, is labelled in History and
// reaches the agent's next note.
// Keys: a property with keys is edited at the playhead (the key there changes, or a key is added
// there); a property without keys edits the transform. The diamonds add or remove a key at the
// playhead.
import { baseBox, clipBox, EASES, keysOf, type ClipBox, type KeyEase, type Fit, type KeyProp, type Rect } from "@/app/lib/editor/core/visual/box";
import { isSpringName } from "@/app/lib/editor/core/visual/spring";
import { componentBox, componentInfo, componentRest, isDrawnComponent } from "@/app/lib/editor/core/visual/components";
import { pictureMedia } from "@/app/lib/editor/core/timeline/timeline";
import type { FieldSet, Item } from "@/app/lib/editor/core/timeline/types";

import { findItem, type Plan } from "@/app/ui/editor/timeline/edit";
import { planUpdate } from "@/app/ui/editor/preview/text";
import type { ProjectModel } from "@/app/lib/editor/state/project";

type Problem = { readonly problem: string };
type Key = NonNullable<Item["keys"]>[number];

/** What the box editors change: the box (canvas px), the fit, zoom, opacity and corner radius. */
export interface BoxPatch {
  readonly x?: number;
  readonly y?: number;
  readonly w?: number;
  readonly h?: number;
  readonly fit?: Fit;
  readonly scale?: number;
  readonly opacity?: number;
  readonly radius?: number;
}

export const PROP_NAMES: Readonly<Record<KeyProp, string>> = { x: "X", y: "Y", scale: "scale", opacity: "opacity" };
const FIT_NAMES: Readonly<Record<Fit, string>> = { contain: "contain", cover: "cover", fill: "fill" };

/** Whether an item draws in a box: a clip of video or still media, or a drawn component (D76). */
export const isBoxed = (project: ProjectModel, item: Item) => !!pictureMedia(project, item) || isDrawnComponent(item);

/** What a boxed item is called in History labels: "clip", or the component's name ("end card"). */
export const boxNoun = (item: Item) => componentInfo(item.component)?.name.toLowerCase() ?? "clip";

/** A boxed item's box now (a clip's or a component's): its rest box (no keys) and its box at the playhead, with the local frame. */
export function clipState(project: ProjectModel, itemId: string, frame: number): { item: Item; start: number; local: number; inside: boolean; rest: Rect; box: ClipBox } | undefined {
  const placed = findItem(project, itemId);
  if (!placed) return undefined;
  const media = pictureMedia(project, placed.item);
  if (!media && !isDrawnComponent(placed.item)) return undefined;
  const local = Math.round(frame - placed.start);
  const inside = local >= 0 && local < placed.item.duration;
  // A keyed edit with the playhead off the clip goes on its nearest frame (never past its last).
  const at = Math.min(placed.item.duration - 1, Math.max(0, local));
  if (!media) return { item: placed.item, start: placed.start, local: at, inside, rest: componentRest(placed.item, project.canvas)!, box: componentBox(placed.item, project.canvas, at)! };
  return { item: placed.item, start: placed.start, local: at, inside, rest: baseBox(placed.item, project.canvas, media.probe), box: clipBox(placed.item, project.canvas, media.probe, at) };
}

const secs = (frames: number, fps: number) => `${(frames / fps).toFixed(1)} s`;
const round2 = (n: number) => Math.round(n * 100) / 100;
const KEYED: ReadonlyArray<KeyProp> = ["x", "y", "scale", "opacity"];

/** The keys with `prop` set to `value` at `frame` (the key there changes, or one is added). */
function setKey(keys: ReadonlyArray<Key>, prop: KeyProp, frame: number, value: number): Array<Key> {
  const out = keys.map((k) => ({ ...k }));
  const at = out.findLastIndex((k) => k.frame === frame);
  if (at !== -1) out[at] = { ...out[at]!, [prop]: value };
  else out.push({ frame, [prop]: value });
  return out.sort((a, b) => a.frame - b.frame);
}

/** The keys without `prop` at `frame`; a key left with no property goes. */
function removeKey(keys: ReadonlyArray<Key>, prop: KeyProp, frame: number): Array<Key> {
  return keys
    .map((k) => {
      if (k.frame !== frame || !(prop in k)) return k;
      const { [prop]: _gone, ...rest } = k;
      return rest as Key;
    })
    .filter((k) => KEYED.some((p) => typeof k[p] === "number"));
}

/** A transform without its defaults (scale 1, opacity 1, radius 0, fit contain), or null when nothing is left. */
function tidy(t: Record<string, unknown>): Record<string, unknown> | null {
  const out = { ...t };
  if (out.scale === 1) delete out.scale;
  if (out.opacity === 1) delete out.opacity;
  if (out.radius === 0) delete out.radius;
  if (out.fit === "contain") delete out.fit;
  return Object.keys(out).length ? out : null;
}

const sameJson = (a: unknown, b: unknown) => JSON.stringify(a ?? null) === JSON.stringify(b ?? null);

/**
 * Change a clip's box. x, y, w and h are written together (the box the Inspector shows, rounded to
 * whole pixels) once any of them changes; a keyed x, y, scale or opacity changes its key at the
 * playhead (adding one there if needed) instead of the transform. Values are clamped to what the
 * schema takes (size ≥ 1, scale > 0, opacity 0–1, radius ≥ 0).
 */
export function planClipBox(project: ProjectModel, itemId: string, patch: BoxPatch, frame: number, summary?: string): Plan | Problem {
  const s = clipState(project, itemId, frame);
  if (!s) return { problem: "That clip isn't in the project any more." };
  const { item, local, rest } = s;
  const noun = boxNoun(item);
  const t: Record<string, unknown> = { ...(item.transform ?? {}) };
  let keys: Array<Key> = [...(item.keys ?? [])];
  const keyed = (p: KeyProp) => keysOf(item.keys, p).length > 0;
  const clean = { ...patch };
  if (clean.w !== undefined) clean.w = Math.max(1, Math.round(clean.w));
  if (clean.h !== undefined) clean.h = Math.max(1, Math.round(clean.h));
  if (clean.x !== undefined) clean.x = Math.round(clean.x);
  if (clean.y !== undefined) clean.y = Math.round(clean.y);
  if (clean.scale !== undefined) clean.scale = Math.min(20, Math.max(0.05, round2(clean.scale)));
  if (clean.opacity !== undefined) clean.opacity = Math.min(1, Math.max(0, round2(clean.opacity)));
  if (clean.radius !== undefined) clean.radius = Math.max(0, Math.round(clean.radius));
  // The box as written: its rest position and size, whole pixels.
  if (["x", "y", "w", "h"].some((k) => clean[k as keyof BoxPatch] !== undefined)) {
    t.x = Math.round(rest.x);
    t.y = Math.round(rest.y);
    t.w = Math.round(rest.w);
    t.h = Math.round(rest.h);
  }
  let atKey = false;
  for (const [k, v] of Object.entries(clean)) {
    if (v === undefined) continue;
    if ((KEYED as ReadonlyArray<string>).includes(k) && keyed(k as KeyProp)) {
      keys = setKey(keys, k as KeyProp, local, v as number);
      atKey = true;
    } else t[k] = v;
  }
  const transform = tidy(t);
  const set: FieldSet = {
    ...(sameJson(transform, item.transform) ? {} : { transform }),
    ...(sameJson(keys, item.keys ?? []) ? {} : { keys: keys.length ? keys : null }),
  };
  const words = summary ?? describePatch(clean, noun);
  return planUpdate(project, itemId, set, atKey ? `${words} (key at ${secs(local, project.canvas.fps)})` : words);
}

function describePatch(p: BoxPatch, noun: string): string {
  if (p.fit) return `Fit ${noun}: ${FIT_NAMES[p.fit]}`;
  if (p.w !== undefined || p.h !== undefined) return `Resize ${noun}`;
  if (p.x !== undefined || p.y !== undefined) return `Move ${noun}`;
  if (p.scale !== undefined) return `Zoom ${noun} to ${Math.round(p.scale * 100)}%`;
  if (p.opacity !== undefined) return `Set ${noun} opacity to ${Math.round(p.opacity * 100)}%`;
  if (p.radius !== undefined) return p.radius > 0 ? `Round ${noun} corners (${p.radius} px)` : `Square ${noun} corners`;
  return `Change ${noun} box`;
}

/** Whether `prop` has a key at the playhead ("on"), keys elsewhere ("keyed") or none. */
export function keyState(item: Item, prop: KeyProp, local: number): "on" | "keyed" | "none" {
  const list = keysOf(item.keys, prop);
  if (!list.length) return "none";
  return list.some((k) => k.frame === local) ? "on" : "keyed";
}

/**
 * The diamond: a key for `prop` at the playhead with the value it has there now, or, if there is
 * one, remove it. The first key of a property starts from the transform's value, so nothing jumps.
 */
export function planToggleKey(project: ProjectModel, itemId: string, prop: KeyProp, frame: number): Plan | Problem {
  const s = clipState(project, itemId, frame);
  if (!s) return { problem: "That clip isn't in the project any more." };
  if (!s.inside) return { problem: "Move the playhead over the clip to add a key." };
  const { item, local, box } = s;
  const fps = project.canvas.fps;
  if (keyState(item, prop, local) === "on") {
    const keys = removeKey(item.keys ?? [], prop, local);
    return planUpdate(project, itemId, { keys: keys.length ? keys : null }, `Remove ${PROP_NAMES[prop]} key at ${secs(local, fps)}`);
  }
  const value = prop === "x" ? Math.round(box.x) : prop === "y" ? Math.round(box.y) : prop === "scale" ? round2(box.scale) : round2(box.opacity);
  return planUpdate(project, itemId, { keys: setKey(item.keys ?? [], prop, local, value) }, `Add ${PROP_NAMES[prop]} key at ${secs(local, fps)}`);
}

/** Back to the full frame, contain, no zoom, no keys. */
export function planResetBox(project: ProjectModel, itemId: string): Plan | Problem {
  const placed = findItem(project, itemId);
  if (!placed) return { problem: "That clip isn't in the project any more." };
  const set: FieldSet = { ...(placed.item.transform ? { transform: null } : {}), ...(placed.item.keys ? { keys: null } : {}) };
  return planUpdate(project, itemId, set, `Reset ${boxNoun(placed.item)} position and size`);
}

/** Loop the clip's media (it starts over when the clip is longer), or not. */
export function planLoop(project: ProjectModel, itemId: string, loop: boolean): Plan | Problem {
  const placed = findItem(project, itemId);
  if (!placed) return { problem: "That clip isn't in the project any more." };
  if ((placed.item.loop === true) === loop) return { ops: [], summary: "" };
  return planUpdate(project, itemId, { loop: loop ? true : null }, loop ? "Loop clip" : "Stop looping clip");
}

/** Hold the media's last frame when the clip is longer than it (a freeze, D94), or not. */
export function planHold(project: ProjectModel, itemId: string, hold: boolean): Plan | Problem {
  const placed = findItem(project, itemId);
  if (!placed) return { problem: "That clip isn't in the project any more." };
  if ((placed.item.hold === true) === hold) return { ops: [], summary: "" };
  return planUpdate(project, itemId, { hold: hold ? true : null }, hold ? "Hold the last frame" : "Stop holding the last frame");
}

export type Corner = "nw" | "ne" | "sw" | "se";

/**
 * A corner handle dragged by (dx, dy) canvas pixels from the box `from`: the opposite corner stays
 * put; the box keeps its shape unless `free` (⇧), at least 16 px.
 */
export function resizeBox(from: Rect, corner: Corner, dx: number, dy: number, free: boolean): Rect {
  const left = corner === "nw" || corner === "sw";
  const top = corner === "nw" || corner === "ne";
  let w = Math.max(16, from.w + (left ? -dx : dx));
  let h = Math.max(16, from.h + (top ? -dy : dy));
  if (!free) {
    // The larger change (either way) wins, the other follows the box's shape.
    const kw = w / from.w;
    const kh = h / from.h;
    const k = Math.abs(kw - 1) >= Math.abs(kh - 1) ? kw : kh;
    w = Math.max(16, from.w * k);
    h = Math.max(16, from.h * k);
  }
  return { x: left ? from.x + from.w - w : from.x, y: top ? from.y + from.h - h : from.y, w, h };
}

// ---- keyframes on the timeline and their eases (Fable QA round 2, F5, D148) ----

export const EASE_NAMES: ReadonlyArray<readonly [KeyEase, string]> = [
  ["linear", "Linear"],
  ["in", "Ease in"],
  ["out", "Ease out"],
  ["inOut", "Ease in-out"],
  ["spring", "Spring"],
  ["spring.snap", "Spring: snap"],
  ["spring.settle", "Spring: settle"],
  ["spring.land", "Spring: land"],
  ["spring.bounce", "Spring: bounce"],
];

/** The frames (from the item's start) where an item has box keys, each once, in order. */
export function keyFrames(item: Pick<Item, "keys">): Array<number> {
  return [...new Set((item.keys ?? []).filter((k) => KEYED.some((p) => typeof k[p] === "number")).map((k) => k.frame))].sort((a, b) => a - b);
}

/** The ease a key at `local` leaves with (the motion to the next key), or undefined when there is no key there. */
export function keyEase(item: Pick<Item, "keys">, local: number): KeyEase | undefined {
  const at = (item.keys ?? []).findLast((k) => k.frame === local && KEYED.some((p) => typeof k[p] === "number"));
  if (!at) return undefined;
  return isSpringName(at.ease) || (typeof at.ease === "string" && Object.hasOwn(EASES, at.ease)) ? at.ease as KeyEase : "linear";
}

/**
 * The project frame of the previous (dir −1) or next (+1) key from `frame`: of the given items (the
 * selection), or of every item with keys when none is given. Undefined when there is none that way.
 */
export function nearbyKey(project: ProjectModel, frame: number, dir: -1 | 1, ids?: ReadonlyArray<string>): number | undefined {
  const f = Math.round(frame);
  let best: number | undefined;
  const items = ids?.length ? ids.map((id) => findItem(project, id)).filter((p) => p !== undefined) : project.tracks.flatMap((t) => t.items.map((i) => findItem(project, i.id)!));
  for (const p of items) {
    for (const k of keyFrames(p.item)) {
      const at = p.start + k;
      if (dir < 0 ? at < f && (best === undefined || at > best) : at > f && (best === undefined || at < best)) best = at;
    }
  }
  return best;
}

/** Set the ease of the key at `local` (every property keyed there): how it moves on to the next key. */
export function planKeyEase(project: ProjectModel, itemId: string, local: number, ease: KeyEase): Plan | Problem {
  const placed = findItem(project, itemId);
  if (!placed) return { problem: "That clip isn't in the project any more." };
  const keys = placed.item.keys ?? [];
  if (!keyFrames(placed.item).includes(local)) return { problem: "Move the playhead onto a key to set its ease." };
  if (keyEase(placed.item, local) === ease) return { ops: [], summary: "" };
  const next = keys.map((k) => {
    if (k.frame !== local) return k;
    const { ease: _old, ...rest } = k;
    return (ease === "linear" ? rest : { ...rest, ease }) as Key;
  });
  const name = EASE_NAMES.find(([e]) => e === ease)![1].toLowerCase();
  return planUpdate(project, itemId, { keys: next }, `Set the key at ${secs(local, project.canvas.fps)} to ${name}`);
}

/** Reset position (F15, D151): a boxed item back to the full frame (planResetBox); a text or graphic back to where its look puts it (no x, y or width). */
export function planResetPosition(project: ProjectModel, itemId: string): Plan | Problem {
  const placed = findItem(project, itemId);
  if (!placed) return { problem: "That item isn't in the project any more." };
  if (isBoxed(project, placed.item)) return planResetBox(project, itemId);
  const { x: _x, y: _y, w: _w, ...rest } = placed.item.transform ?? {};
  if (!placed.item.transform || (_x === undefined && _y === undefined && _w === undefined)) return { problem: "It's already where it starts." };
  return planUpdate(project, itemId, { transform: Object.keys(rest).length ? rest : null }, `Reset the position of ${placed.item.name ?? itemId}`);
}
