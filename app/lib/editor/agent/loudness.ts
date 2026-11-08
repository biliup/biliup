// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type {LoudnessResult} from "./analysisTypes";
const MAX_SILENCES=200;
export function loudnessOut(result: LoudnessResult, cached: boolean, frames: (seconds: number, round: (x: number) => number) => number, range: { start: number; end: number } | undefined) {
  if (!result.hasAudio) return { state: "done", cached, hasAudio: false, note: "This media has no sound, so there's nothing to measure." };
  const all = (result.silence?.ranges ?? [])
    .map((r) => ({ start: frames(r.start, Math.ceil), end: frames(r.end, Math.floor), startSeconds: r.start, endSeconds: r.end }))
    .filter((r) => r.end > r.start && (!range || (r.start < range.end && r.end > range.start)));
  return {
    state: "done",
    cached,
    hasAudio: true,
    ...(result.seconds !== undefined ? { seconds: result.seconds } : {}),
    integratedLufs: result.integratedLufs ?? null,
    loudnessRangeLu: result.loudnessRangeLu ?? null,
    truePeakDbtp: result.truePeakDbtp ?? null,
    silence: {
      thresholdDb: result.silence?.thresholdDb,
      minSeconds: result.silence?.minSeconds,
      count: all.length,
      ...(range ? { range } : {}),
      ranges: all.slice(0, MAX_SILENCES),
      ...(all.length > MAX_SILENCES ? { truncated: `Only the first ${MAX_SILENCES} of ${all.length}; pass range for the rest.` } : {}),
    },
  };
}

