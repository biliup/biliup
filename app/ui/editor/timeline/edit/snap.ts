// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Snapping for the timeline planners (edit.ts): beats (D70), snap targets and the nearest snap.
import { soundMedia } from "@/app/lib/editor/core/audio/audio";
import { sourcePoints } from '@/app/lib/editor/core/timeline/sourceSpans';
import { type ProjectModel, type Item, type Track, layoutOf } from "@/app/lib/editor/state/project";
import { SNAP_PX, type Placed, findItem } from "@/app/ui/editor/timeline/edit/base";

// ---- beats (D70) ----

/** Beat times (seconds of each media file) of the media whose beats are cached, by media id. */
export type BeatMap = ReadonlyMap<string, ReadonlyArray<number>>;
/** The beats a snap may use, and the zoom (beats closer than MIN_BEAT_PX on screen aren't targets). */
export interface BeatSnap {
  readonly beats: BeatMap;
  /** Pixels per frame. */
  readonly ppf: number;
}
/** Beats drawn and snapped to only when at least this far apart on screen: closer, every drop would stick to one. */
export const MIN_BEAT_PX = 2 * SNAP_PX;

/**
 * Where an item's beats fall, in whole frames from the item's start (in [0, duration)): a
 * beat at t seconds of the media is at (t·fps − in) / speed, repeated every media length when
 * the item loops. Only clips and audio items with sound have beats.
 */
export function beatsInItem(project: ProjectModel, item: Item, beats: ReadonlyArray<number>): Array<number> {
  if (!beats.length || !soundMedia(project, item)) return [];
  const media = project.media.find((m) => m.id === item.media);
  const frames = new Set<number>();
  for (const point of sourcePoints(item, media?.probe?.frames, beats, project.canvas.fps)) {
    const frame = Math.round(point.local);
    if (frame >= 0 && frame < item.duration) frames.add(frame);
  }
  return [...frames];
}

/** The median gap between neighbouring values (frames), 0 with fewer than two. */
const medianGap = (frames: ReadonlyArray<number>) => {
  if (frames.length < 2) return 0;
  const gaps = frames.slice(1).map((f, i) => f - frames[i]!).sort((a, b) => a - b);
  return gaps[gaps.length >> 1]!;
};

/** An item's beats as drawn: local frames, or none when they'd be closer than `minPx` apart at this zoom. */
export function visibleBeats(project: ProjectModel, item: Item, beats: ReadonlyArray<number> | undefined, ppf: number, minPx = MIN_BEAT_PX): Array<number> {
  const local = beats ? beatsInItem(project, item, beats) : [];
  return local.length > 1 && medianGap(local) * ppf < minPx ? [] : local;
}

/** Timeline frames (whole) of the beats of every item `include` accepts, at a zoom where they're far enough apart. */
export function beatTargets(project: ProjectModel, snap: BeatSnap | undefined, include: (item: Item, track: Track, index: number) => boolean = () => true): Array<number> {
  if (!snap || !snap.beats.size) return [];
  const { starts } = layoutOf(project);
  const out: Array<number> = [];
  for (const track of project.tracks) {
    track.items.forEach((item, index) => {
      const beats = item.media ? snap.beats.get(item.media) : undefined;
      if (!beats || !include(item, track, index)) return;
      const start = starts.get(item.id) ?? 0;
      for (const f of visibleBeats(project, item, beats, snap.ppf)) out.push(Math.round(start + f));
    });
  }
  return out;
}

/**
 * The trimmed item's own beats that stay put while an edge moves, over all of its media the edge
 * can reach (dragging the end out past the last beat it plays now still finds the next one): the
 * end edge, and the start edge of a free-track item (start and in move together). A main clip
 * trimmed from the start keeps its start and shifts its content, so its beats move: none.
 */
function ownTrimBeats(project: ProjectModel, placed: Placed, edge: "start" | "end", snap: BeatSnap | undefined, atFrame: number): Array<number> {
  const { item, start, track } = placed;
  const beats = item.media ? snap?.beats.get(item.media) : undefined;
  if (!snap || !beats || (edge === "start" && track.role === "main")) return [];
  const speed = item.speed ?? 1;
  const from = item.in ?? 0;
  const length = project.media.find((m) => m.id === item.media)?.probe?.frames;
  const spacing = beats.length > 1 ? medianGap(beats) * project.canvas.fps / speed : item.loop && length ? length / speed : 0;
  if (spacing > 0 && spacing * snap.ppf < MIN_BEAT_PX) return [];
  const margin = SNAP_PX / snap.ppf + 1;
  // Search around the dragged edge, including later loops, without enumerating all earlier passes.
  const reach = item.loop || !length ? Math.max(item.duration, atFrame - start + margin) : Math.max(item.duration, Math.floor((length - from) / speed));
  const virtual: Item = edge === "end" ? { ...item, duration: reach } : { ...item, in: 0, duration: item.duration + from / speed };
  const base = edge === "end" ? start : start - from / speed;
  if (!soundMedia(project, item)) return [];
  const frames = new Set<number>();
  for (const point of sourcePoints(virtual, length, beats, project.canvas.fps, { start: atFrame - base - margin, end: atFrame - base + margin })) {
    const frame = Math.round(base + point.local);
    if (frame >= 0) frames.add(frame);
  }
  return [...frames];
}

/** Snap the playhead to a recording/project marker or a visible beat. */
export function snapPlayhead(project: ProjectModel, frame: number, snap: BeatSnap | undefined, threshold: number): { frame: number; snapAt?: number } {
  let best: number | undefined;
  for (const t of [...(project.markers ?? []).map(marker => marker.frame), ...beatTargets(project, snap)]) if (Math.abs(t - frame) <= threshold && (best === undefined || Math.abs(t - frame) < Math.abs(best - frame))) best = t;
  return best === undefined ? { frame } : { frame: best, snapAt: best };
}

/** Snap targets in frames: 0, the playhead, markers, every other item's start and end, and the beats of other items (D70). `exclude`: one id or several (a group move). */
export function snapTargets(project: ProjectModel, playhead: number, exclude: string | ReadonlySet<string>, beats?: BeatSnap): Array<number> {
  const { starts } = layoutOf(project);
  const skip = typeof exclude === "string" ? new Set([exclude]) : exclude;
  const targets = new Set([0, Math.floor(playhead), ...(project.markers ?? []).map((m) => m.frame)]);
  for (const track of project.tracks) {
    for (const item of track.items) {
      if (skip.has(item.id)) continue;
      const s = starts.get(item.id) ?? 0;
      targets.add(s);
      targets.add(s + item.duration);
    }
  }
  // The moved item's own beats move with it.
  for (const b of beatTargets(project, beats, (item) => !skip.has(item.id))) targets.add(b);
  return [...targets];
}

/**
 * Snap targets for trimming an edge: as snapTargets, minus the edge's own position (on main the
 * neighbour's edge sits there, which would pull every small trim back to nothing) and minus the
 * main clips after this one, which ripple with the edge. Beats (D70) of items that stay put count,
 * the trimmed item's own included (trimming music to end on its beat): only a main clip trimmed
 * from the start shifts its own beats (its start stays, its content moves), so those don't.
 */
export function trimSnapTargets(project: ProjectModel, playhead: number, itemId: string, edge: "start" | "end", beats: BeatSnap | undefined, atFrame: number): Array<number> {
  const placed = findItem(project, itemId);
  if (!placed) return [];
  const own = edge === "start" ? placed.start : placed.start + placed.item.duration;
  const { starts } = layoutOf(project);
  const targets = new Set([0, Math.floor(playhead), ...(project.markers ?? []).map((m) => m.frame)]);
  const ripples = (track: Track, index: number) => track.role === "main" && placed.track.role === "main" && index > placed.index;
  for (const track of project.tracks) {
    track.items.forEach((item, index) => {
      if (item.id === itemId) return;
      if (ripples(track, index)) return;
      const s = starts.get(item.id) ?? 0;
      targets.add(s);
      targets.add(s + item.duration);
    });
  }
  for (const b of beatTargets(project, beats, (item, track, index) => !ripples(track, index) && item.id !== itemId)) targets.add(b);
  for (const b of ownTrimBeats(project, placed, edge, beats, atFrame)) targets.add(b);
  targets.delete(own);
  return [...targets];
}

export interface Snap {
  readonly targets: ReadonlyArray<number>;
  /** In frames. */
  readonly threshold: number;
}

/** The nearest target within the threshold of any of `edges` (frames); the shift to apply and where. */
export function nearestSnap(edges: ReadonlyArray<number>, snap: Snap | undefined): { shift: number; at: number } | undefined {
  if (!snap) return undefined;
  let best: { shift: number; at: number } | undefined;
  for (const edge of edges) {
    for (const t of snap.targets) {
      const shift = t - edge;
      if (Math.abs(shift) <= snap.threshold && (!best || Math.abs(shift) < Math.abs(best.shift))) best = { shift, at: t };
    }
  }
  return best;
}

/**
 * Where the blade (issue #57, D230) cuts `itemId` when pressed at `frame`: the nearest snap target
 * within `snap.threshold` frames (the playhead, 0, markers, every other item's edges, beats: as a
 * move snaps), else the frame rounded. The item's own edges aren't targets (there's nothing to cut
 * there). `snap` undefined: snapping is off.
 */
export function bladeFrame(project: ProjectModel, itemId: string, frame: number, snap: { readonly playhead: number; readonly threshold: number; readonly beats?: BeatSnap } | undefined): { frame: number; snapAt?: number } {
  if (!snap) return { frame: Math.round(frame) };
  const hit = nearestSnap([frame], { targets: snapTargets(project, snap.playhead, itemId, snap.beats), threshold: snap.threshold });
  return hit ? { frame: hit.at, snapAt: hit.at } : { frame: Math.round(frame) };
}
