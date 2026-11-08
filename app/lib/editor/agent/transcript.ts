// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// What the agent sees of a transcript (D68), pure: transcript_read's answer (text lines or words,
// paged) and media_analyze's short "transcript" summary. The cache holds seconds of the file; the
// agent gets project frames of the media (usable as in): a word from the frame it starts in to
// the frame after it ends (it covers the word), a pause inward (both ends inside the gap), like
// the silences.
import { speechWords, type SpeechEnvelope } from "../core/text/transcript";
import { joinWords } from '@/app/lib/editor/core/text/word-spacing';

import { framesFor } from "./video";
import { fillersOf, pausesOf, type Filler, type Pause, type TranscriptResult } from "./analysisTypes";

/** Words per answer: text lines, or one object per word. */
export const MAX_TEXT_WORDS = 1500;
export const MAX_JSON_WORDS = 500;

export interface ReadOptions {
  readonly cached: boolean;
  readonly fps: number;
  /** The media's length in project frames. */
  readonly length: number | undefined;
  readonly range?: { readonly start: number; readonly end: number } | undefined;
  /** The first word index to list. */
  readonly from?: number;
  readonly format?: "text" | "words";
  /** The loudness analyzer's silences (seconds), when cached. */
  readonly silences?: ReadonlyArray<{ readonly start: number; readonly end: number }> | undefined;
  /** Its speech envelope (D74), when cached. */
  readonly envelope?: SpeechEnvelope | undefined;
  readonly maxWords?: number;
}

const secs = (s: number) => `${Math.round(s * 10) / 10} s`;

export const TRANSCRIPT_NOTE =
  "Frames are project frames of the media (use them as in): a word runs from start (the frame it begins in) to end (the frame after it), a pause from start to end inside the gap. Text lines start with the index of their first word (w12) and its frames; {braces} mark fillers, (pause 1.2 s) a gap of at least 0.3 s. fillers: kind hesitation (um, uh, yyy, eee: safe to cut) or maybe (discourse words like no, znaczy, you know: judge in context). pauses: silent says whether the sound is below the silence threshold there (from the loudness analysis; absent when it hasn't run); edge start/end is before the first or after the last word. Times come from the speech model and are good to about 0.1 s; look at the frames before a tight cut.";

/** transcript_read's answer for a finished transcript. */
export function transcriptOut(result: TranscriptResult, options: ReadOptions) {
  if (!result.hasAudio) return { state: "done", cached: options.cached, hasAudio: false, note: "This media has no sound, so there's nothing to transcribe." };
  const words = result.words ?? [];
  const segments = result.segments ?? [];
  const { fps, range } = options;
  const toFrames = framesFor(fps, options.length);
  const base = {
    state: "done",
    cached: options.cached,
    hasAudio: true,
    ...(result.seconds !== undefined ? { seconds: result.seconds } : {}),
    ...(result.language ? { language: result.language } : {}),
    ...(result.model ? { model: result.model } : {}),
    words: words.length,
    segments: segments.length,
  };
  if (!words.length) return { ...base, text: "", note: "No speech was found in it." };
  const fillers = fillersOf(words, result.language);
  // Where the words are heard (D74: whisper's edges widened to its segments, the silences and the
  // envelope), as the cut planner sees them: their frames, the range and the pauses all come from it.
  const heard = speechWords(words, { segments, silences: options.silences, seconds: result.seconds, envelope: options.envelope });
  const pauses = pausesOf(heard, result.seconds, options.silences, undefined, undefined, options.envelope);
  const inRange = (start: number, end: number) => !range || (end * fps > range.start && start * fps < range.end);
  const from = options.from ?? 0;
  const cap = options.maxWords ?? (options.format === "words" ? MAX_JSON_WORDS : MAX_TEXT_WORDS);
  const picked: Array<number> = [];
  let more: number | undefined;
  for (let i = from; i < words.length; i++) {
    if (!inRange(heard[i]!.start, heard[i]!.end)) continue;
    if (picked.length === cap) {
      more = i;
      break;
    }
    picked.push(i);
  }
  const first = picked[0];
  const last = picked.at(-1);
  const shown = new Set(picked);
  const fillerAt = new Map<number, Filler>();
  for (const f of fillers) for (let k = 0; k < f.n; k++) fillerAt.set(f.i + k, f);
  const wordStart = (i: number) => toFrames(heard[i]!.start, Math.floor);
  const wordEnd = (i: number) => Math.max(wordStart(i) + 1, toFrames(heard[i]!.end, Math.ceil));
  const pauseOut = (p: Pause) => ({ start: toFrames(p.start, Math.ceil), end: toFrames(p.end, Math.floor), startSeconds: p.start, endSeconds: p.end, after: p.after, ...(p.edge ? { edge: p.edge } : {}), ...(p.silent !== undefined ? { silent: p.silent } : {}) });
  // The pauses after a listed word, and the one before the first word with it.
  const listedPauses = pauses.filter((p) => shown.has(p.after) || (p.edge === "start" && shown.has(0)));
  const listedFillers = fillers.filter((f) => shown.has(f.i));
  const out: Record<string, unknown> = {
    ...base,
    ...(range ? { range } : {}),
    ...(first !== undefined ? { shown: { from: first, to: last } } : { shown: null }),
  };
  if (options.format === "words") {
    out.list = picked.map((i) => {
      const w = words[i]!;
      const f = fillerAt.get(i);
      return { i, text: w.text, start: wordStart(i), end: wordEnd(i), ...(w.p < 0.5 ? { unsure: true } : {}), ...(f ? { filler: f.kind } : {}) };
    });
  } else {
    const pauseAfter = new Map<number, Pause>();
    for (const p of listedPauses) if (p.edge !== "start") pauseAfter.set(p.after, p);
    const lines: Array<string> = [];
    const lead = listedPauses.find((p) => p.edge === "start");
    if (lead) lines.push(`(pause ${secs(lead.end - lead.start)} before the first word)`);
    for (const s of segments) {
      const ids = picked.filter((i) => i >= s.words[0] && i < s.words[1]);
      if (!ids.length) continue;
      const parts = ids.map((i) => {
        const f = fillerAt.get(i);
        let t = words[i]!.text;
        if (f && f.i === i) t = `{${t}`;
        if (f && f.i + f.n - 1 === i) t = `${t}}`;
        const p = pauseAfter.get(i);
        return p ? `${t} (pause ${secs(p.end - p.start)})` : t;
      });
      lines.push(`w${ids[0]} [${wordStart(ids[0]!)}–${wordEnd(ids.at(-1)!)}] ${joinWords(ids.map(id=>words[id]!.text),(_word,index)=>parts[index]!)}`);
    }
    out.text = lines.join("\n");
  }
  out.fillers = listedFillers.map((f) => ({ i: f.i, n: f.n, text: joinWords(words.slice(f.i, f.i + f.n).map((w) => w.text)), start: wordStart(f.i), end: wordEnd(f.i + f.n - 1), kind: f.kind }));
  out.pauses = listedPauses.map(pauseOut);
  if (more !== undefined) out.next = { from: more, note: `More words follow: call transcript_read again with from ${more} (and the same range).` };
  if (first === undefined) out.note = range ? "No words in that range." : "No words from there.";
  return out;
}

/** media_analyze's "transcript": a summary; transcript_read gives the words. */
export function transcriptSummary(result: TranscriptResult, cached: boolean, sound: { readonly silences?: ReadonlyArray<{ readonly start: number; readonly end: number }> | undefined; readonly envelope?: SpeechEnvelope | undefined } = {}) {
  if (!result.hasAudio) return { state: "done", cached, hasAudio: false, note: "This media has no sound, so there's nothing to transcribe." };
  const words = result.words ?? [];
  const fillers = fillersOf(words, result.language);
  const pauses = pausesOf(words, result.seconds, sound.silences, undefined, result.segments, sound.envelope);
  return {
    state: "done",
    cached,
    hasAudio: true,
    ...(result.seconds !== undefined ? { seconds: result.seconds } : {}),
    ...(result.language ? { language: result.language } : {}),
    ...(result.model ? { model: result.model } : {}),
    words: words.length,
    segments: result.segments?.length ?? 0,
    fillers: { hesitation: fillers.filter((f) => f.kind === "hesitation").length, maybe: fillers.filter((f) => f.kind === "maybe").length },
    pauses: pauses.length,
    pauseSeconds: Math.round(pauses.reduce((s, p) => s + p.end - p.start, 0) * 10) / 10,
    ...(words.length ? { opening: joinWords(words.slice(0, 30).map((w) => w.text)) } : {}),
  };
}
