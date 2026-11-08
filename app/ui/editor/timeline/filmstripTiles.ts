// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Issue #53: which filmstrip tile each cell of a clip on the timeline shows. A cell is a tile wide
// at the lane's height; it shows the frame of the media at the cell's left edge, following the
// clip's in point, speed, loop and hold the way the waveform does (Waveform.tsx).
import type { FilmstripLayout } from "@/app/lib/editor/contracts";

export interface TileClip {
  readonly in?: number | undefined;
  readonly duration: number;
  readonly speed?: number | undefined;
  readonly loop?: boolean | undefined;
}

/** The media time (s) at `x` px into a clip drawn `width` px wide. */
export function mediaTimeAt(clip: TileClip, fps: number, x: number, width: number, mediaSeconds: number): number {
  const from = (clip.in ?? 0) / fps;
  const length = (clip.duration * (clip.speed ?? 1)) / fps;
  let t = from + (width > 0 ? x / width : 0) * length;
  // Looped media starts over; held (or plain) media stays on its last frame (the tile clamp).
  if (clip.loop && mediaSeconds > 0 && t >= mediaSeconds) t -= Math.floor(t / mediaSeconds) * mediaSeconds;
  return t;
}

/** The tile showing media time `t`: the nearest frame taken, within the strip. */
export const tileAt = (layout: Pick<FilmstripLayout, "interval" | "count">, t: number) => Math.max(0, Math.min(layout.count - 1, Math.round(t / layout.interval)));

/** A cell's width (px) for tiles drawn `height` px high. */
export const cellWidth = (layout: Pick<FilmstripLayout, "tileWidth" | "tileHeight">, height: number) => Math.max(8, (layout.tileWidth * height) / layout.tileHeight);

/**
 * The cells of a clip `width` px wide that fall in [`left`, `right`) (px from the clip's left edge,
 * the part on screen), with one more on each side: index, left edge and tile.
 */
export function visibleCells(clip: TileClip, fps: number, layout: FilmstripLayout, width: number, height: number, left: number, right: number) {
  const w = cellWidth(layout, height);
  const total = Math.ceil(width / w);
  const first = Math.max(0, Math.floor(left / w) - 1);
  const last = Math.min(total - 1, Math.ceil(right / w));
  const cells: Array<{ index: number; x: number; tile: number }> = [];
  for (let i = first; i <= last; i++) cells.push({ index: i, x: i * w, tile: tileAt(layout, mediaTimeAt(clip, fps, i * w, width, layout.seconds)) });
  return { width: w, cells };
}
