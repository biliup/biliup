// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Components by hand (D76): pure planners for the Elements tab (add one at the playhead, or where
// it's dropped on the canvas or the timeline), the generic params Inspector and the device frame's
// footage. Like every other editor (D52, D53), each gesture is one project.edit: validated, one
// ⌘Z, labelled in History and listed in the agent's next note.
import { mainEnd as mainEndOf } from "@/app/lib/editor/core/timeline/rules";
import {planReplaceMedia} from '@/app/lib/editor/core/timeline/arrange';
import { STICKER_ART } from "@/app/lib/editor/core/visual/stickers";
import { alignScreen, componentDefaults, componentInfo, defaultComponentBox, componentRest, deviceSpec, isDrawnComponent, naturalSize, planDeviceFrame, screenKeys, screenTransform, type Device } from "@/app/lib/editor/core/visual/components";
import type { Item, Op } from "@/app/lib/editor/core/timeline/types";

import { findItem, lockOf, type Plan } from "@/app/ui/editor/timeline/edit";
import { displayTracks, layoutOf, uniqueId, type Planned, type ProjectModel, type Track } from "@/app/lib/editor/state/project";
import { planUpdate } from "@/app/ui/editor/preview/text";

type Problem = { readonly problem: string };

/** How long a new component lasts: a countdown, lower third, quote or checklist 5 s, an end card, callout, subscribe or social button 3 s, a sticker 2 s, the rest 4 s. */
const SECONDS: Readonly<Record<string, number>> = { "component.end-card": 3, "component.subscribe": 3, "component.social-button": 3, "component.sticker": 2, "component.callout": 3, "component.countdown": 5, "component.lower-third": 5, "component.quote": 5, "component.checklist": 5 };
export const secondsFor = (id: string) => SECONDS[id] ?? 4;

/**
 * A component at frame `frame` (D76): its manifest's default props and size. `at` (canvas px, a
 * drop on the canvas) centres it there; otherwise it's centred (an end card fills the canvas).
 * Where: an end card at the end of the main track goes on main (a card in the sequence); anything
 * else on the top-most free video track under the text tracks (else a new "Overlay" track there),
 * so it draws over the footage and backgrounds and under the text. A device frame comes alone
 * (its Inspector puts footage in it).
 */
export function planAddComponent(project: ProjectModel, componentId: string, frame: number, { at, props }: { at?: { x: number; y: number }; props?: Record<string, unknown> } = {}): Planned {
  const c = componentInfo(componentId);
  if (!c) return { problem: "Biliup can't draw that component yet." };
  const fps = project.canvas.fps;
  const start = Math.max(0, Math.floor(frame));
  const duration = Math.max(1, Math.round(secondsFor(c.id) * fps));
  if (c.id === "component.device-frame") {
    const first = planDeviceFrame(project, { start, duration });
    if ("problem" in first) return first;
    let plan = first;
    if (at) {
      const size = naturalSize({ component: c.id, props: { device: "phone" } }, project.canvas);
      const h = project.canvas.height * 0.86;
      const w = (h * size.width) / size.height;
      const placed = planDeviceFrame(project, { start, duration, box: { x: Math.round(at.x - w / 2), y: Math.round(at.y - h / 2), h } });
      if (!("problem" in placed)) plan = placed;
    }
    return { ops: plan.ops, summary: "Add device frame", itemId: plan.frameId, start };
  }
  const mainIndex = project.tracks.findIndex((t) => t.role === "main");
  const main = project.tracks[mainIndex];
  if (!main) return { problem: "This project has no main track." };
  const taken = new Set<string>();
  const itemId = uniqueId(project, c.id.replace(/^component\./, ""), taken);
  taken.add(itemId);
  // Dropped on the canvas: centred on the drop point; else where it goes on this canvas's aspect (F7, D152).
  let transform: Item["transform"] = defaultComponentBox(c.id, project, { ...componentDefaults(c.id), ...(props ?? {}) });
  if (at && !c.block.fullFrame) {
    const size = transform ? { width: transform.w!, height: transform.h! } : naturalSize({ component: c.id, props: { ...componentDefaults(c.id), ...(props ?? {}) } }, project.canvas);
    transform = { x: Math.round(at.x - size.width / 2), y: Math.round(at.y - size.height / 2), w: Math.round(size.width), h: Math.round(size.height) };
  }
  // A sticker from the set (D165) comes with its own props over the defaults, and its name.
  const name = typeof props?.art === "string" ? STICKER_ART[props.art]?.name : undefined;
  const item: Item = { id: itemId, type: "component", component: c.id, duration, props: { ...componentDefaults(c.id), ...(props ?? {}) }, ...(name ? { name } : {}), ...(transform ? { transform } : {}) };
  const summary = props?.emoji ? `Add sticker ${String(props.emoji)}` : name ? `Add sticker ${name.toLowerCase()}` : `Add ${c.name.toLowerCase()}`;
  const mainEnd = mainEndOf(project);
  if (c.block.fullFrame && start === mainEnd && !main.locked) return { ops: [{ op: "insert_item", track: main.id, item }], summary, itemId, start };
  const ops: Array<Op> = [];
  const free = (t: Track) => t.items.every((i) => (i.start ?? 0) + i.duration <= start || start + duration <= (i.start ?? 0));
  const lastText = project.tracks.slice(0, mainIndex).reduce((n, t, i) => (t.kind === "text" ? i : n), -1);
  const track = project.tracks.slice(lastText + 1, mainIndex).find((t) => t.kind === "video" && !t.locked && free(t));
  let trackId = track?.id;
  if (!trackId) {
    trackId = uniqueId(project, "overlay", taken);
    const n = displayTracks(project).filter((t) => t.kind === "video" && t.role !== "main" && t.name.startsWith("Overlay")).length;
    ops.push({ op: "add_track", index: lastText + 1, track: { id: trackId, kind: "video", name: n ? `Overlay ${n + 1}` : "Overlay" } });
  }
  ops.push({ op: "insert_item", track: trackId, item: { ...item, start } });
  return { ops, summary, itemId, start };
}

/**
 * An imported HTML graphic registered in the project's graphics library: a graphic item
 * (full canvas, drawn in the graphics sandbox) with its props, at the playhead,
 * on the first free overlay track under the text (else a new Overlay track), as components go.
 */
export function planAddGraphic(project: ProjectModel, graphic: { readonly src: string; readonly name: string; readonly props: Record<string, unknown>; readonly idBase: string }, frame: number): Planned {
  const mainIndex = project.tracks.findIndex((t) => t.role === "main");
  if (mainIndex < 0) return { problem: "This project has no main track." };
  const start = Math.max(0, Math.floor(frame));
  const duration = Math.max(1,Math.round(3 * project.canvas.fps));
  const taken = new Set<string>();
  const itemId = uniqueId(project, graphic.idBase, taken);
  taken.add(itemId);
  const ops: Array<Op> = [];
  const free = (t: Track) => t.items.every((i) => (i.start ?? 0) + i.duration <= start || start + duration <= (i.start ?? 0));
  const lastText = project.tracks.slice(0, mainIndex).reduce((n, t, i) => (t.kind === "text" ? i : n), -1);
  let trackId = project.tracks.slice(lastText + 1, mainIndex).find((t) => t.kind === "video" && !t.locked && free(t))?.id;
  if (!trackId) {
    trackId = uniqueId(project, "overlay", taken);
    const n = displayTracks(project).filter((t) => t.kind === "video" && t.role !== "main" && t.name.startsWith("Overlay")).length;
    ops.push({ op: "add_track", index: lastText + 1, track: { id: trackId, kind: "video", name: n ? `Overlay ${n + 1}` : "Overlay" } });
  }
  ops.push({ op: "insert_item", track: trackId, item: { id: itemId, type: "graphic", src: graphic.src, name: graphic.name, start, duration, props: graphic.props } as Item });
  return { ops, summary: `Add ${graphic.name.toLowerCase()}`, itemId, start };
}

/** A component's props replaced whole (update_item replaces nested objects), labelled for History. */
export function planComponentProps(project: ProjectModel, itemId: string, props: Record<string, unknown>, summary: string): Plan | Problem {
  const placed = findItem(project, itemId);
  if (!placed || !isDrawnComponent(placed.item)) return { problem: "That component isn't in the project any more." };
  if (JSON.stringify(props) === JSON.stringify(placed.item.props ?? {})) return { ops: [], summary: "" };
  return planUpdate(project, itemId, { props }, summary);
}

/**
 * The clip showing a device frame's footage: a clip on a video track under the frame's that
 * overlaps it in time, preferably with its box on the frame's screen, else one with the frame's
 * start and length (as insert_device_frame and the Inspector put it there, before the frame moved).
 * The nearest track down wins.
 */
export function screenClipOf(project: ProjectModel, frameId: string): { item: Item; track: Track } | undefined {
  const frame = findItem(project, frameId);
  if (!frame || frame.item.component !== "component.device-frame") return undefined;
  const t = screenTransform(frame.item, project.canvas);
  if (!t) return undefined;
  const { starts } = layoutOf(project);
  const from = frame.start;
  const to = frame.start + frame.item.duration;
  const index = project.tracks.indexOf(frame.track);
  const candidates: Array<{ item: Item; track: Track }> = [];
  // Stay below this frame and stop at an intervening main track, before unrelated backgrounds.
  for (const track of project.tracks.slice(index + 1)) {
    if (track.role === "main") break;
    if (track.kind !== "video") continue;
    for (const item of track.items) {
      const s = starts.get(item.id) ?? 0;
      if (item.type === "clip" && item.transform && s < to && s + item.duration > from) candidates.push({ item, track });
    }
  }
  const on = (i: Item) => i.transform?.x === t.x && i.transform?.y === t.y && i.transform?.w === t.w && i.transform?.h === t.h;
  return candidates.find((c) => on(c.item)) ?? candidates.find((c) => (starts.get(c.item.id) ?? 0) === from && c.item.duration === frame.item.duration && c.item.transform?.fit === "cover");
}

/**
 * Footage in a device frame (D76): `mediaId` on its screen (a new clip under the frame, its box the
 * screen, cover; or the existing screen clip switched to it), or null to take the footage out. A
 * video shorter than the frame loops.
 */
export function planScreenFootage(project: ProjectModel, frameId: string, mediaId: string | null): Plan | Problem {
  const frame = findItem(project, frameId);
  if (!frame || frame.item.component !== "component.device-frame") return { problem: "That device frame isn't in the project any more." };
  const locked = lockOf(frame);
  if (locked) return { problem: locked };
  const existing = screenClipOf(project, frameId);
  if (mediaId === null) {
    if (!existing) return { ops: [], summary: "" };
    const locked = lockOf(existing);
    if (locked) return { problem: locked };
    return { ops: [{ op: "delete_item", item: existing.item.id }], summary: "Remove the device frame's footage" };
  }
  const media = project.media.find((m) => m.id === mediaId);
  if (!media || media.kind === "audio") return { problem: "Pick a video or an image for the screen." };
  if (existing) {
    const plan = planReplaceMedia(project, existing.item.id, media.id, 'loop');
    return 'problem' in plan ? plan : { ...plan, summary: "Change the device frame's footage" };
  }
  const frames = media.kind === "video" ? media.probe?.frames : undefined;
  if (media.kind === "video" && (!Number.isSafeInteger(frames) || frames! < 1)) return { problem: 'The replacement has no known duration.' };
  const loop = frames !== undefined && frames < frame.item.duration;
  const t = screenTransform(frame.item, project.canvas)!;
  const taken = new Set<string>();
  const clipId = uniqueId(project, "screen", taken);
  taken.add(clipId);
  const keys = screenKeys(frame.item, project.canvas);
  const clip: Item = { id: clipId, type: "clip", media: media.id, start: frame.start, duration: frame.item.duration, ...(media.kind === "video" ? { in: 0 } : {}), ...(loop ? { loop: true } : {}), transform: t, ...(keys ? { keys } : {}) };
  const index = project.tracks.indexOf(frame.track);
  const mainIndex = project.tracks.findIndex((tr) => tr.role === "main");
  const free = (tr: Track) => tr.kind === "video" && tr.role !== "main" && !tr.locked && tr.items.every((i) => (i.start ?? 0) + i.duration <= frame.start || frame.start + frame.item.duration <= (i.start ?? 0));
  const below = project.tracks.slice(index + 1, mainIndex === -1 ? undefined : mainIndex).find(free);
  const ops: Array<Op> = [];
  let trackId = below?.id;
  if (!trackId) {
    trackId = uniqueId(project, "screen-track", taken);
    ops.push({ op: "add_track", index: index + 1, track: { id: trackId, kind: "video", name: "Screen" } });
  }
  ops.push({ op: "insert_item", track: trackId, item: clip });
  return { ops, summary: "Put footage in the device frame" };
}

/** The screen clip back on the frame's screen (after the frame moved or was resized). */
export function planAlignScreen(project: ProjectModel, frameId: string): Plan | Problem {
  const frame = findItem(project, frameId);
  const clip = screenClipOf(project, frameId);
  if (!frame || !clip) return { problem: "This device frame has no footage under it." };
  const locked = lockOf(frame);
  if (locked) return { problem: locked };
  const set = alignScreen(frame.item, clip.item, project.canvas);
  if (!set) return { problem: "This device frame has no footage under it." };
  return planUpdate(project, clip.item.id, set, "Fit footage to the device frame's screen");
}

/**
 * Another device for a frame: the box keeps its height and centre and takes the new device's
 * shape, and its footage clip follows the new screen; one edit.
 */
export function planDevice(project: ProjectModel, frameId: string, device: Device): Plan | Problem {
  const frame = findItem(project, frameId);
  if (!frame || frame.item.component !== "component.device-frame") return { problem: "That device frame isn't in the project any more." };
  const locked = lockOf(frame);
  if (locked) return { problem: locked };
  const props: Record<string, unknown> = { ...(frame.item.props ?? {}), device };
  if (device !== "browser") delete props.url;
  const t = frame.item.transform ?? {};
  const spec = deviceSpec(device);
  // Same height and centre: from the box the frame has now, written whole.
  const rest = componentRest(frame.item, project.canvas)!;
  const w = (rest.h * spec.size.width) / spec.size.height;
  const transform = { ...t, x: Math.round(rest.x + rest.w / 2 - w / 2), y: Math.round(rest.y), w: Math.round(w), h: Math.round(rest.h) };
  const shift = transform.x - rest.x;
  const keys = frame.item.keys?.map(key => typeof key.x === 'number' ? { ...key, x: key.x + shift } : key);
  const next: Item = { ...frame.item, props, transform, ...(keys ? { keys } : {}) };
  const ops: Array<Op> = [{ op: "update_item", item: frameId, set: { props, transform, ...(keys ? { keys } : {}) } }];
  const clip = screenClipOf(project, frameId);
  if (clip) {
    const locked = lockOf(clip);
    if (locked) return { problem: locked };
    const set = alignScreen(next, clip.item, project.canvas);
    if (set) ops.push({ op: "update_item", item: clip.item.id, set });
  }
  return { ops, summary: `Set device frame to ${device}` };
}
