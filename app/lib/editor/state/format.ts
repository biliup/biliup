// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { Canvas as CoreCanvas } from "@/app/lib/editor/core/timeline/types";

/** Frames → HH:MM:SS, whole seconds (media durations). */
export const timecode = (frames: number, fps: number) => {
  const total = Math.floor(frames / fps);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(Math.floor(total / 3600))}:${pad(Math.floor(total / 60) % 60)}:${pad(total % 60)}`;
};

/** Frames → HH:MM:SS:FF, the transport's timecode (frame f is the moment f / fps, D38). */
export const frameTimecode = (frames: number, fps: number) => {
  const f = Math.max(0, Math.floor(frames));
  const pad = (n: number) => String(n).padStart(2, "0");
  // The frame within its second; at 29.97 (D209) a second starts at the first frame at or after it.
  const second = Math.floor(f / fps);
  return `${timecode(f, fps)}:${pad(f - Math.ceil(second * fps - 1e-9))}`;
};

/** Seconds → the ruler's label: "0s", "15s", "1:30". */
export const rulerLabel = (seconds: number) =>
  seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;

export type Canvas = CoreCanvas;

// D209 (issue #15): eight presets, then Custom.
export const CANVAS_PRESETS: ReadonlyArray<{ readonly id: string; readonly label: string; readonly ratio: string; readonly canvas: Canvas }> = [
  { id: "landscape", label: "Landscape", ratio: "16:9", canvas: { width: 1920, height: 1080, fps: 30 } },
  { id: "1080p60", label: "1080p 60", ratio: "16:9", canvas: { width: 1920, height: 1080, fps: 60 } },
  { id: "4k", label: "4K", ratio: "16:9", canvas: { width: 3840, height: 2160, fps: 60 } },
  { id: "cinema", label: "Cinema", ratio: "21:9", canvas: { width: 2560, height: 1080, fps: 24 } },
  { id: "vertical", label: "Vertical", ratio: "9:16", canvas: { width: 1080, height: 1920, fps: 30 } },
  { id: "4k-vertical", label: "4K vertical", ratio: "9:16", canvas: { width: 2160, height: 3840, fps: 30 } },
  { id: "social", label: "Social", ratio: "4:5", canvas: { width: 1080, height: 1350, fps: 30 } },
  { id: "square", label: "Square", ratio: "1:1", canvas: { width: 1080, height: 1080, fps: 30 } },
];

/** "29.97 fps", "30 fps". */
export const fpsLabel = (fps: number) => `${fps} fps`;
export const canvasLabel = (c: Canvas) => `${c.width}×${c.height}, ${fpsLabel(c.fps)}`;
