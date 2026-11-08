// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { Project, Item, Canvas, Op, Media, Track, FieldSet } from "@/app/lib/editor/core/timeline/types";
import { type Device, screenRect, deviceOf, DEVICES, deviceSpec } from "@/app/lib/editor/core/visual/components/device";
import { componentBox } from "@/app/lib/editor/core/visual/components/registry";

// ---- device frame planning ----

export interface DeviceFrameRequest {
  /** The footage (video or image media) for the screen; without it, only the frame. */
  readonly media?: string | undefined;
  readonly device?: Device | undefined;
  readonly start: number;
  readonly duration: number;
  /** Where in the media the clip starts (project frames). */
  readonly in?: number | undefined;
  /** The frame's box in canvas px: any of x, y and h (w follows the device's shape); default centred, 86 % of the canvas height (phones). */
  readonly box?: { readonly x?: number | undefined; readonly y?: number | undefined; readonly h?: number | undefined } | undefined;
  readonly frameId?: string | undefined;
  readonly clipId?: string | undefined;
  readonly color?: string | undefined;
  readonly url?: string | undefined;
  /** Footage shorter than the frame holds its last frame (D94) instead of looping. */
  readonly hold?: boolean | undefined;
}

type Problem = { readonly problem: string };

const allIds = (project: Pick<Project, "media" | "tracks" | "scenes">) =>
  new Set([...project.media.map((m) => m.id), ...project.tracks.flatMap((t) => [t.id, ...t.items.map((i) => i.id)]), ...(project.scenes ?? []).map((s) => s.id)]);
const freshId = (ids: Set<string>, base: string) => {
  for (let n = 1; ; n++) {
    if (!ids.has(`${base}-${n}`)) {
      ids.add(`${base}-${n}`);
      return `${base}-${n}`;
    }
  }
};

/** The footage clip's transform for a frame item: its screen rect at the frame's rest box (D75 box, cover, the screen's corners). */
export function screenTransform(frame: Item, canvas: Pick<Canvas, "width" | "height">): { x: number; y: number; w: number; h: number; fit: "cover"; radius?: number } | undefined {
  const box = componentBox({ ...frame, keys: undefined }, canvas, 0);
  if (!box) return undefined;
  const r = screenRect(deviceOf(frame.props ?? {}), box.w, box.h);
  // The frame's transform.scale draws it scaled about its centre: the screen scales with it.
  const s = box.scale;
  const cx = box.x + box.w / 2;
  const cy = box.y + box.h / 2;
  const radius = Math.round(r.radius * s);
  return { x: Math.round(cx + (box.x + r.x - cx) * s), y: Math.round(cy + (box.y + r.y - cy) * s), w: Math.round(r.w * s), h: Math.round(r.h * s), fit: "cover", ...(radius > 0 ? { radius } : {}) };
}

/** The frame's x / y keys moved onto its screen (so the footage moves with the frame), or none. */
export function screenKeys(frame: Item, canvas: Pick<Canvas, "width" | "height">): Item["keys"] | undefined {
  const keys = (frame.keys ?? []).filter((k) => typeof k.x === "number" || typeof k.y === "number" || typeof k.opacity === "number");
  if (!keys.length) return undefined;
  // A key moves the frame's box; the screen keeps its offset in it (rest box → rest screen).
  const box = componentBox({ ...frame, keys: undefined }, canvas, 0)!;
  const t = screenTransform(frame, canvas)!;
  const dx = t.x - box.x;
  const dy = t.y - box.y;
  return keys.map((k) => ({ frame: k.frame, ...(k.ease ? { ease: k.ease } : {}), ...(typeof k.x === "number" ? { x: Math.round(k.x + dx) } : {}), ...(typeof k.y === "number" ? { y: Math.round(k.y + dy) } : {}), ...(typeof k.opacity === "number" ? { opacity: k.opacity } : {}) }));
}

/**
 * A device frame and the footage under it, as one edit (D76): the clip on a free video track
 * between main and the text tracks (the nearest free one above main, else a new one), its box on
 * the frame's screen (cover, the screen's corners), and the bezel on a free video track above the
 * clip's (else a new one right above it), both over [start, start + duration). The frame is
 * centred at 86 % of the canvas height (a browser at 80 % of its width) unless `box` says otherwise.
 */
export function planDeviceFrame(project: Pick<Project, "canvas" | "media" | "tracks" | "scenes">, req: DeviceFrameRequest): { ops: Array<Op>; frameId: string; clipId?: string } | Problem {
  const { canvas } = project;
  const device = req.device && DEVICES.includes(req.device) ? req.device : "phone";
  if (!Number.isInteger(req.start) || req.start < 0) return { problem: "start must be a frame (0 or more)." };
  if (!Number.isInteger(req.duration) || req.duration < 1) return { problem: "duration must be at least one frame." };
  const media: Media | undefined = req.media !== undefined ? project.media.find((m) => m.id === req.media) : undefined;
  if (req.media !== undefined && !media) return { problem: `media "${req.media}" does not exist.` };
  if (media && media.kind === "audio") return { problem: `"${media.id}" is audio-only: a device frame needs video or an image.` };
  const ids = allIds(project);
  for (const id of [req.frameId, req.clipId]) if (id !== undefined && ids.has(id)) return { problem: `id "${id}" is already used.` };
  const frameId = req.frameId ?? freshId(ids, "device");
  ids.add(frameId);
  const clipId = media ? (req.clipId ?? freshId(ids, "screen")) : undefined;
  if (clipId) ids.add(clipId);

  const spec = deviceSpec(device);
  const aspect = spec.size.width / spec.size.height;
  let h = req.box?.h ?? (device === "browser" ? (canvas.width * 0.8) / aspect : canvas.height * 0.86);
  if (device === "browser" && req.box?.h === undefined) h = Math.min(h, canvas.height * 0.86);
  const w = h * aspect;
  const x = req.box?.x ?? (canvas.width - w) / 2;
  const y = req.box?.y ?? (canvas.height - h) / 2;
  const props: Record<string, unknown> = { device, ...(req.color ? { color: req.color } : {}), ...(req.url && device === "browser" ? { url: req.url } : {}) };
  const frame: Item = { id: frameId, type: "component", component: "component.device-frame", start: req.start, duration: req.duration, transform: { x: Math.round(x), y: Math.round(y), w: Math.round(w), h: Math.round(h) }, props };

  // Tracks: video tracks between the lowest text track above main and main (the project lists
  // tracks top to bottom; the first draws on top), nearest main first.
  const mainIndex = project.tracks.findIndex((t) => t.role === "main");
  if (mainIndex === -1) return { problem: "This project has no main track." };
  const free = (t: Track) => !t.locked && t.kind === "video" && t.role !== "main" && t.items.every((i) => (i.start ?? 0) + i.duration <= req.start || req.start + req.duration <= (i.start ?? 0));
  const lastText = project.tracks.slice(0, mainIndex).reduce((n, t, i) => (t.kind === "text" ? i : n), -1);
  const region = project.tracks.map((t, i) => ({ t, i })).slice(lastText + 1, mainIndex).reverse();
  const ops: Array<Op> = [];
  const newTrack = (index: number, name: string) => {
    const id = freshId(ids, "overlay");
    ops.push({ op: "add_track", index, track: { id, kind: "video", name } });
    return id;
  };
  const names = new Set(project.tracks.map((t) => t.name));
  const trackName = (base: string) => {
    for (let n = 1; ; n++) {
      const name = n === 1 ? base : `${base} ${n}`;
      if (!names.has(name)) {
        names.add(name);
        return name;
      }
    }
  };
  if (!media) {
    const spot = region.find(({ t }) => free(t));
    const track = spot ? spot.t.id : newTrack(mainIndex, trackName("Device"));
    ops.push({ op: "insert_item", track, item: frame });
    return { ops, frameId };
  }
  const t = screenTransform(frame, canvas)!;
  // Footage shorter than the frame (from its in-point) loops, as the Inspector's does.
  const from = req.in ?? 0;
  if (media.kind === "video" && media.probe?.frames !== undefined && from >= media.probe.frames) return { problem: `in (${from}) is past the end of "${media.id}" (${media.probe.frames} frames).` };
  const loop = media.kind === "video" && media.probe?.frames !== undefined && from + req.duration > media.probe.frames;
  const clip: Item = {
    id: clipId!,
    type: "clip",
    media: media.id,
    start: req.start,
    duration: req.duration,
    ...(media.kind === "video" ? { in: from } : {}),
    ...(loop ? (req.hold ? { hold: true } : { loop: true }) : {}),
    transform: t,
  };
  const clipSpot = region.find(({ t }) => free(t));
  let clipTrack: string;
  let bezelTrack: string;
  if (clipSpot) {
    clipTrack = clipSpot.t.id;
    // A free track above the clip's, still under the text.
    const above = region.filter(({ i }) => i < clipSpot.i).find(({ t }) => free(t));
    bezelTrack = above ? above.t.id : newTrack(clipSpot.i, trackName("Device"));
  } else {
    // Both new, right above main: the bezel first (on top), then the screen.
    clipTrack = newTrack(mainIndex, trackName("Screen"));
    bezelTrack = newTrack(mainIndex, trackName("Device"));
  }
  ops.push({ op: "insert_item", track: clipTrack, item: clip }, { op: "insert_item", track: bezelTrack, item: frame });
  return { ops, frameId, clipId: clipId! };
}

/** Re-align a frame's footage clip with its screen (after the frame moved or changed device): the update for the clip. */
export function alignScreen(frame: Item, clip: Item, canvas: Pick<Canvas, "width" | "height">): FieldSet | undefined {
  const t = screenTransform(frame, canvas);
  if (!t) return undefined;
  // The frame's x / y / opacity keys replace the clip's; its own scale keys (a zoom inside the screen) stay.
  const own = (clip.keys ?? []).filter((k) => typeof k.scale === "number").map((k) => ({ frame: k.frame, ...(k.ease ? { ease: k.ease } : {}), scale: k.scale }));
  const keys = [...(screenKeys(frame, canvas) ?? []), ...own].sort((a, b) => a.frame - b.frame);
  return { transform: { ...t, ...(typeof clip.transform?.opacity === "number" ? { opacity: clip.transform.opacity } : {}), ...(typeof clip.transform?.scale === "number" ? { scale: clip.transform.scale } : {}) }, keys: keys.length ? keys : null };
}
