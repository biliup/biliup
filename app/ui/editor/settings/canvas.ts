// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The project's canvas settings (G8, D119), pure: aspect presets and the edit a change makes.
import type { FieldSet, Item, Op } from "@/app/lib/editor/core/timeline/types";

import type { ProjectModel } from "@/app/lib/editor/state/project";
import type { Plan } from "@/app/ui/editor/timeline/edit";

export type Canvas = ProjectModel["canvas"];
export const FPS_CHOICES: ReadonlyArray<Canvas["fps"]> = [23.976, 24, 25, 29.97, 30, 50, 59.94, 60];
export const ASPECTS: ReadonlyArray<{ readonly id: string; readonly label: string; readonly width: number; readonly height: number }> = [
  { id: "16:9", label: "16:9", width: 1920, height: 1080 },
  { id: "9:16", label: "9:16", width: 1080, height: 1920 },
  { id: "1:1", label: "1:1", width: 1080, height: 1080 },
  { id: "4:5", label: "4:5", width: 1080, height: 1350 },
];
export const aspectOf = (c: Pick<Canvas, "width" | "height">) => ASPECTS.find((a) => a.width * c.height === a.height * c.width)?.id;
/** Even, 16…8192 (the schema's range; H.264 wants even sizes). */
export const canvasSide = (v: number) => Math.min(8192, Math.max(16, Math.round(v / 2) * 2));

/**
 * The edit for a canvas change. A frame rate change is refused while the timeline has items:
 * every time in a project is in frames, and retiming them all is not done here.
 * A new size (F10, D155) also rescales what's placed on the canvas, in the same edit (one ⌘Z),
 * unless `keepPositions`: see rescaleItem.
 */
export function planCanvas(project: ProjectModel, patch: Partial<Canvas>, { keepPositions = false }: { keepPositions?: boolean } = {}): Plan | { readonly problem: string } {
  const next = { ...project.canvas, ...patch };
  if (patch.width !== undefined || patch.height !== undefined || patch.fps !== undefined) next.matchFirstClip = false;
  if (patch.width !== undefined) next.width = canvasSide(patch.width);
  if (patch.height !== undefined) next.height = canvasSide(patch.height);
  if (patch.fps !== undefined && patch.fps !== project.canvas.fps && project.tracks.some((t) => t.items.length)) return { problem: "The frame rate can only change while the timeline is empty." };
  const same = next.width === project.canvas.width && next.height === project.canvas.height && next.fps === project.canvas.fps && next.background === project.canvas.background && next.matchFirstClip === project.canvas.matchFirstClip;
  if (same) return { ops: [], summary: "" };
  const what = patch.matchFirstClip !== undefined ? `match first video to ${next.matchFirstClip ? 'on' : 'off'}` : patch.fps !== undefined ? `frame rate to ${next.fps} fps` : patch.background !== undefined ? `background to ${next.background}` : `canvas to ${next.width}×${next.height}`;
  const resized = next.width !== project.canvas.width || next.height !== project.canvas.height;
  const items: Array<Op> = [];
  if (resized && !keepPositions) {
    for (const track of project.tracks) {
      for (const item of track.items) {
        const set = rescaleItem(item, project.canvas, next);
        if (set) items.push({ op: "update_item", item: item.id, set });
      }
    }
  }
  return { ops: [{ op: "update_project", set: { canvas: next } }, ...items], summary: `Changed ${what}${items.length ? `, ${items.length === 1 ? "1 item" : `${items.length} items`} rescaled` : ""}` };
}

const r = (n: number) => Math.round(n);
const r2 = (n: number) => Math.round(n * 100) / 100;

/**
 * What a canvas change from `from` to `to` does to one item (F10, D155), or undefined when nothing:
 * every position keeps its place relative to the frame (a box's centre goes to the same share of
 * the new width and height) and every size scales by k = the smaller of the two ratios, so what fit
 * still fits. A clip box that was the whole frame becomes the whole new frame. Clips and elements
 * without a box (full frame, or their natural size) need nothing; text gets its position, width
 * and font size (and outline width) scaled; box keys (x, y) move with the box.
 */
export function rescaleItem(item: Item, from: Pick<Canvas, "width" | "height">, to: Pick<Canvas, "width" | "height">): FieldSet | undefined {
  const sx = to.width / from.width;
  const sy = to.height / from.height;
  const k = Math.min(sx, sy);
  const t = item.transform;
  const set: Record<string, unknown> = {};
  if (item.type === "text") {
    if (t && (t.x !== undefined || t.y !== undefined || t.w !== undefined)) {
      const w = t.w !== undefined ? t.w * k : undefined;
      const x = t.x !== undefined ? (t.w !== undefined ? (t.x + t.w / 2) * sx - w! / 2 : t.x * sx) : undefined;
      set.transform = { ...t, ...(x !== undefined ? { x: r(x) } : {}), ...(t.y !== undefined ? { y: r(t.y * sy) } : {}), ...(w !== undefined ? { w: Math.max(1, r(w)) } : {}) };
    }
    const style = item.style;
    if (style && (typeof style.size === "number" || (style.stroke && typeof style.stroke.width === "number"))) {
      set.style = {
        ...style,
        ...(typeof style.size === "number" ? { size: Math.max(1, r(style.size * k)) } : {}),
        ...(style.stroke ? { stroke: { ...style.stroke, width: r2(style.stroke.width * k) } } : {}),
      };
    }
    return Object.keys(set).length ? set : undefined;
  }
  if (item.type !== "clip" && item.type !== "component") return undefined;
  if (!t || (t.w === undefined && t.h === undefined && t.x === undefined && t.y === undefined)) return undefined;
  const whole = (t.x ?? 0) === 0 && (t.y ?? 0) === 0 && t.w === from.width && t.h === from.height;
  if (whole) set.transform = { ...t, x: 0, y: 0, w: to.width, h: to.height };
  else {
    const w = t.w !== undefined ? t.w * k : undefined;
    const h = t.h !== undefined ? t.h * k : undefined;
    // The centre keeps its share of the frame (a box without x / y stays centred).
    const x = t.x !== undefined && t.w !== undefined ? (t.x + t.w / 2) * sx - w! / 2 : t.x !== undefined ? t.x * sx : undefined;
    const y = t.y !== undefined && t.h !== undefined ? (t.y + t.h / 2) * sy - h! / 2 : t.y !== undefined ? t.y * sy : undefined;
    set.transform = {
      ...t,
      ...(x !== undefined ? { x: r(x) } : {}),
      ...(y !== undefined ? { y: r(y) } : {}),
      ...(w !== undefined ? { w: Math.max(1, r(w)) } : {}),
      ...(h !== undefined ? { h: Math.max(1, r(h)) } : {}),
      ...(typeof t.radius === "number" ? { radius: r(t.radius * k) } : {}),
    };
  }
  if (item.keys?.length) {
    const w0 = t.w ?? 0;
    const h0 = t.h ?? 0;
    const nt = set.transform as { w?: number; h?: number };
    set.keys = item.keys.map((key) => ({
      ...key,
      ...(typeof key.x === "number" ? { x: r(whole ? key.x * sx : (key.x + w0 / 2) * sx - (nt.w ?? 0) / 2) } : {}),
      ...(typeof key.y === "number" ? { y: r(whole ? key.y * sy : (key.y + h0 / 2) * sy - (nt.h ?? 0) / 2) } : {}),
    }));
  }
  return set;
}
