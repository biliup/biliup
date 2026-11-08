// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The video analyzer's result (D67): per-second measurements and the cache shape check.
/** Bumped when the video analyzer's output changes, so older cache files are redone. */
export const VIDEO_VERSION = 2;

// ---- results ----

/** One second of the file [start, end), measured. */
export interface VideoWindow {
  readonly start: number;
  readonly end: number;
  /** Frames in it. */
  readonly frames: number;
  /** Camera shake: RMS distance (fraction of the frame width) between the camera's path and its smoothed path. null: no frame pair in it could be matched. */
  readonly jitter: number | null;
  /** The smoothed path's speed (frame widths per second): how fast the camera pans or travels. */
  readonly motion: number | null;
  /** How many of its frame pairs could be matched (0–1). */
  readonly tracked: number;
  /** blurdetect's mean blur at 320 px (higher is blurrier, about the width of an edge in pixels). */
  readonly blur: number | null;
  /** Mean luma 0–1. */
  readonly brightness: number;
  /** Share of pixels near black (≤ 16/255). */
  readonly dark: number;
  /** Share of pixels near white (≥ 245/255). */
  readonly bright: number;
  /** Share of it inside frozen stretches. */
  readonly frozen: number;
}

export interface VideoResult {
  readonly hasVideo: boolean;
  /** Seconds of picture decoded. */
  readonly seconds?: number;
  readonly frames?: number;
  /** Frames per second measured (frames over seconds). */
  readonly fps?: number;
  readonly cutThreshold?: number;
  /** Gaps in the recording (dropped frames: the picture jumps): the first frame after each (seconds) and how long is missing. */
  readonly drops?: ReadonlyArray<{ readonly time: number; readonly missing: number }>;
  /** Where a new shot starts (seconds of the file, the first frame of the new shot) and scdet's score. */
  readonly cuts?: ReadonlyArray<{ readonly time: number; readonly score: number }>;
  /** Frozen stretches (seconds of the file). */
  readonly frozen?: ReadonlyArray<{ readonly start: number; readonly end: number }>;
  /** Per second of the file. */
  readonly windows?: ReadonlyArray<VideoWindow>;
}

export const NO_VIDEO: VideoResult = { hasVideo: false };

const finite = (v: unknown) => typeof v === "number" && Number.isFinite(v);
const finiteOrNull = (v: unknown) => v === null || finite(v);

/** Whether a cached value has the video result's shape. */
export function isVideoResult(value: unknown): value is VideoResult {
  if (!value || typeof value !== "object") return false;
  const r = value as Record<string, unknown>;
  if (typeof r.hasVideo !== "boolean") return false;
  if (!r.hasVideo) return true;
  if (!finite(r.seconds) || !finite(r.frames) || !Array.isArray(r.cuts) || !Array.isArray(r.drops) || !Array.isArray(r.frozen) || !Array.isArray(r.windows)) return false;
  if (!r.drops.every((d) => d && finite(d.time) && finite(d.missing))) return false;
  if (!r.cuts.every((c) => c && finite(c.time) && finite(c.score))) return false;
  if (!r.frozen.every((f) => f && finite(f.start) && finite(f.end))) return false;
  return r.windows.every(
    (w) => w && finite(w.start) && finite(w.end) && finite(w.frames) && finiteOrNull(w.jitter) && finiteOrNull(w.motion) && finite(w.tracked) && finiteOrNull(w.blur) && finite(w.brightness) && finite(w.dark) && finite(w.bright) && finite(w.frozen),
  );
}
