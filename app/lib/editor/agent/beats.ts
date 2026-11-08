// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// What the agent sees of the beats analyzer (D70), pure: media_analyze's "beats" answer. The cache
// holds seconds of the file; the agent gets project frames of the media (usable as in) beside them.
import type { BeatsResult } from "./analysisTypes";
import type { ToFrames } from "./video";

/** Without a range, the first this many beats are listed (the rest: pass a range). */
export const FIRST_BEATS = 32;
/** With a range, at most this many beats and onsets in it. */
export const MAX_BEATS_LISTED = 400;
export const MAX_ONSETS_LISTED = 200;

export const BEATS_NOTE =
  "tempo (bpm) and beats found from the sound's onsets on this server. confidence 0–1 is how clear the pulse is (above about 0.5: a steady beat; under 0.15 there's none to speak of and bpm is null). Beat frames are project frames of the media, the frame nearest each beat (use them as in): a clip of this media at timeline frame S with in I hits a beat b at S + (b − I) (at speed 1). beatFrames is the gap between beats in frames. Without range only the first beats come back; pass range (project frames of the media) for the beats and onsets inside it. Onsets are every hit (drums, notes), strength 0–1.";

/** media_analyze's answer for the beats analyzer. */
export function beatsOut(result: BeatsResult, cached: boolean, toFrames: ToFrames, range: { start: number; end: number } | undefined, fps: number) {
  if (!result.hasAudio) return { state: "done", cached, hasAudio: false, note: "This media has no sound, so there are no beats." };
  const beats = (result.beats ?? []).map((t) => ({ frame: toFrames(t, Math.round), seconds: t }));
  const onsets = result.onsets ?? [];
  const base = { state: "done", cached, hasAudio: true, seconds: result.seconds, bpm: result.bpm ?? null, confidence: result.confidence ?? 0 };
  if (result.bpm === null || result.bpm === undefined || !beats.length)
    return { ...base, beatCount: 0, onsetCount: onsets.length, note: "No steady beat found (speech, ambience or silence), so there are no beats to cut to." };
  const inRange = (frame: number) => !range || (frame >= range.start && frame < range.end);
  const listed = beats.filter((b) => inRange(b.frame));
  const cap = range ? MAX_BEATS_LISTED : FIRST_BEATS;
  const shown = listed.slice(0, cap);
  const out: Record<string, unknown> = {
    ...base,
    beatCount: beats.length,
    beatFrames: Math.round(((60 / result.bpm) * fps) * 100) / 100,
    firstBeat: beats[0],
    lastBeat: beats.at(-1),
    beats: {
      ...(range ? { range, count: listed.length } : {}),
      frames: shown.map((b) => b.frame),
      seconds: shown.map((b) => b.seconds),
      ...(listed.length > cap ? { truncated: range ? `Only the first ${cap} of ${listed.length} in the range; narrow the range for the rest.` : `The first ${cap} of ${beats.length}; pass range for the others.` } : {}),
    },
    onsetCount: onsets.length,
  };
  if (range) {
    const hits = onsets.map((o) => ({ frame: toFrames(o.t, Math.round), seconds: o.t, strength: o.strength })).filter((o) => inRange(o.frame));
    out.onsets = { count: hits.length, list: hits.slice(0, MAX_ONSETS_LISTED), ...(hits.length > MAX_ONSETS_LISTED ? { truncated: `Only the first ${MAX_ONSETS_LISTED} of ${hits.length}; narrow the range for the rest.` } : {}) };
  }
  return out;
}
