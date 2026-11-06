// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.


export const ms3 = (s: number) => Math.round(s * 1000) / 1000;

/** A transcript segment: its span (seconds) and its words [first, end) (whisper's segments, D68). */
export interface SpeechSegment {
  readonly start: number;
  readonly end: number;
  readonly words: readonly [number, number];
}

type Span2 = { readonly start: number; readonly end: number };

/** A segment's edge counts only when at least this far (seconds) from the neighbouring word (speechWords). */
const SEGMENT_EDGE_GAP_S = 0.1;

/** The silence overlapping [start, end] the most, if one does. */
export function silenceIn(silences: ReadonlyArray<Span2>, start: number, end: number): Span2 | undefined {
  let best: Span2 | undefined;
  let overlap = 0;
  for (const s of silences) {
    const o = Math.min(s.end, end) - Math.max(s.start, start);
    if (o > overlap) {
      overlap = o;
      best = s;
    }
  }
  return best;
}

// ---- the speech envelope (D74) ----

/**
 * The loudness analyzer's speech envelope as cached: the RMS level of every 1/rate s of the sound
 * (mono), in dBFS, quantised to `step` dB above `db0` (0 = at or below db0) as one byte per window,
 * base64. 10 ms windows at 0.5 dB: 100 bytes a second, ~480 kB of JSON for an hour.
 */
export interface StoredEnvelope {
  readonly rate: number;
  readonly db0: number;
  readonly step: number;
  readonly data: string;
}
/** A decoded envelope: dBFS per window of 1/rate s. */
export interface SpeechEnvelope {
  readonly rate: number;
  readonly db: ArrayLike<number>;
}
export const ENVELOPE_RATE = 100;
const ENVELOPE_DB0 = -100;
const ENVELOPE_STEP = 0.5;

/** dBFS levels (one per window) as a StoredEnvelope. */
export function encodeEnvelope(db: ArrayLike<number>, rate = ENVELOPE_RATE): StoredEnvelope {
  let bin = "";
  for (let i = 0; i < db.length; i++) {
    const v = db[i]!;
    const q = Number.isFinite(v) ? Math.round((v - ENVELOPE_DB0) / ENVELOPE_STEP) : 0;
    bin += String.fromCharCode(Math.max(0, Math.min(255, q)));
  }
  return { rate, db0: ENVELOPE_DB0, step: ENVELOPE_STEP, data: btoa(bin) };
}

/** A cached envelope decoded (undefined when it isn't one). */
export function decodeEnvelope(value: unknown): SpeechEnvelope | undefined {
  const e = value as Partial<StoredEnvelope> | undefined;
  if (!e || typeof e !== "object" || typeof e.data !== "string" || !(typeof e.rate === "number" && e.rate > 0) || typeof e.db0 !== "number" || typeof e.step !== "number") return undefined;
  let bin: string;
  try {
    bin = atob(e.data);
  } catch {
    return undefined;
  }
  const db = new Float32Array(bin.length);
  for (let i = 0; i < bin.length; i++) db[i] = e.db0 + bin.charCodeAt(i) * e.step;
  return { rate: e.rate, db };
}

/** Speech is at most this far (dB) below the speech level (90th percentile): breaths and room tone are further down... */
const SPEECH_RANGE_DB = 25;
/** ...and at least this far above the noise floor (10th percentile: a music or noise bed). */
const ABOVE_FLOOR_DB = 10;
/** Speech ends after this much quiet (s): a stop consonant's closure (≤ ~0.1 s) doesn't end a word. */
const QUIET_HOLD_S = 0.1;
/** The sound moves a word's edge at most this far (s) past where the transcript and silences put it (whisper's sentence ends run up to ~0.3 s early). */
const MAX_EXTEND_S = 0.35;
/** A word whisper timed shorter than this (s) is a timestamp, not a span (fillers: "Um," at 4.02–4.02). */
const SHORT_WORD_S = 0.05;
/** Without an envelope, such a word is given this long (s), centred, between its neighbours; with one, its sound is looked for this far from the timestamp... */
const SHORT_WORD_SPAN_S = 0.2;
/** ...and may last this long on each side of it (a drawn-out "ummm"). */
const SHORT_WORD_MAX_S = 1;

const thresholds = new WeakMap<SpeechEnvelope, number>();
/** The level (dBFS) speech is above: the speech level less SPEECH_RANGE_DB, or the floor plus ABOVE_FLOOR_DB. */
export function speechThreshold(env: SpeechEnvelope): number {
  const known = thresholds.get(env);
  if (known !== undefined) return known;
  const sorted = Float32Array.from(env.db as ArrayLike<number>).sort();
  const at = (q: number) => sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * q))] ?? -Infinity;
  const thr = Math.max(at(0.9) - SPEECH_RANGE_DB, at(0.1) + ABOVE_FLOOR_DB);
  thresholds.set(env, thr);
  return thr;
}

/**
 * Whether an envelope tells speech from quiet: some windows above the speech threshold and some
 * below (a steady tone or music bed with no gaps has neither, so it says nothing about words).
 */
export function envelopeInformative(env: SpeechEnvelope | undefined): env is SpeechEnvelope {
  if (!env?.db.length) return false;
  const thr = speechThreshold(env);
  let loud = false;
  let quiet = false;
  for (let k = 0; k < env.db.length && !(loud && quiet); k++) env.db[k]! > thr ? (loud = true) : (quiet = true);
  return loud && quiet;
}

type SpeechOptions = { readonly segments?: ReadonlyArray<SpeechSegment> | undefined; readonly silences?: ReadonlyArray<Span2> | undefined; readonly seconds?: number | undefined; readonly envelope?: SpeechEnvelope | undefined };

/** speechWords' words, and which of their edges the sound measured (a silence, or the envelope going quiet). */
export interface HeardWords<W> {
  readonly words: Array<W>;
  /** Per word: its end / start is where the sound stops / starts (a cut there needs no margin into a removed neighbour). */
  readonly endMeasured: ReadonlyArray<boolean>;
  readonly startMeasured: ReadonlyArray<boolean>;
}

/**
 * Where each word is heard, as best the transcript and the sound tell (D74): whisper's word edges
 * are early at the end of a sentence (its last word's end is 0.15–0.65 s before the sound stops,
 * measured on real large-v3-turbo output), while its segments' edges (timestamp tokens) are close.
 * - A word whisper put wholly inside a silence (no sound there) moves to the silence's nearer edge
 *   (and the words after it start no earlier than it).
 * - A segment's last word ends no earlier than the segment and its first word starts no later than
 *   it (unless that edge is within SEGMENT_EDGE_GAP_S of the next / previous word: punctuation timed
 *   at the next word, D68).
 * - With the loudness analyzer's silences, a word before a silence lasts until it starts and a word
 *   after one starts when it ends.
 * - With its speech envelope (when it tells speech from quiet): a word's end follows the sound while
 *   it's above the speech threshold until QUIET_HOLD_S of quiet, its start back the same way, at most
 *   MAX_EXTEND_S, never into a silence or past a neighbour. When two words' sound runs together the
 *   gap between them is split at its quietest window. A word whisper gave no length (a timestamp,
 *   often a filler) takes the sound within SHORT_WORD_SPAN_S of it (up to SHORT_WORD_MAX_S long on
 *   each side); without the envelope it gets SHORT_WORD_SPAN_S around it.
 * Words with sound only get longer here; idempotent (running it on its own output changes nothing).
 */
export function speechEdges<W extends { readonly start: number; readonly end: number }>(words: ReadonlyArray<W>, { segments, silences, seconds, envelope }: SpeechOptions = {}): HeardWords<W> {
  // Copies with writable edges (W's own fields kept).
  const out = words.map((w) => ({ ...w })) as Array<W & { start: number; end: number }>;
  const n = out.length;
  const endMeasured = Array<boolean>(n).fill(false);
  const startMeasured = Array<boolean>(n).fill(false);
  if (!n) return { words: out, endMeasured, startMeasured };
  const limit = seconds ?? Infinity;
  // 1. Words the silences placed (no sound where whisper put them): judged on whisper's own span.
  const moved = new Set<number>();
  if (silences?.length) {
    out.forEach((w, i) => {
      const s = silences.find((x) => x.start <= w.start && w.end <= x.end && x.end - x.start > 0);
      if (!s || (w.start === w.end && (w.start === s.start || w.start === s.end))) return;
      const at = (w.start + w.end) / 2 - s.start < s.end - (w.start + w.end) / 2 ? s.start : s.end;
      w.start = at;
      w.end = at;
      moved.add(i);
    });
  }
  // In order: no word starts before the one before it ends (a moved word can land before its
  // predecessor's end, and push the ones after it).
  for (let i = 1; i < n; i++) {
    const w = out[i]!;
    if (!moved.has(i) && !moved.has(i - 1)) continue;
    if (w.start < out[i - 1]!.end) {
      w.start = out[i - 1]!.end;
      w.end = Math.max(w.end, w.start);
      moved.add(i);
    }
  }
  // 2. Segment edges.
  const lastAt = new Map<number, number>();
  const firstAt = new Map<number, number>();
  for (const s of segments ?? []) {
    const [a, b] = s.words;
    if (!(b > a) || a < 0 || b > n) continue;
    if (Number.isFinite(s.end)) lastAt.set(b - 1, Math.max(lastAt.get(b - 1) ?? -Infinity, s.end));
    if (Number.isFinite(s.start)) firstAt.set(a, Math.min(firstAt.get(a) ?? Infinity, s.start));
  }
  // A segment edge inside a silence is where the silence says the sound stops / starts.
  const outOfSilence = (t: number | undefined, side: "start" | "end") => {
    const s = t === undefined ? undefined : silences?.find((x) => x.start < t && t < x.end);
    return s ? (side === "end" ? s.start : s.end) : t;
  };
  for (let i = 0; i < n; i++) {
    const w = out[i]!;
    const nextStart = i + 1 < n ? out[i + 1]!.start : limit;
    const prevEnd = i > 0 ? out[i - 1]!.end : 0;
    const segEnd = outOfSilence(lastAt.get(i), "end");
    if (segEnd !== undefined && segEnd > w.end && nextStart - segEnd >= SEGMENT_EDGE_GAP_S) w.end = segEnd;
    const segStart = outOfSilence(firstAt.get(i), "start");
    if (segStart !== undefined && segStart < w.start && segStart - prevEnd >= SEGMENT_EDGE_GAP_S) w.start = segStart;
  }
  // 3. The silences in the gaps: before the first word, between words, after the last.
  if (silences?.length) {
    for (let i = -1; i < n; i++) {
      const gapStart = i >= 0 ? out[i]!.end : 0;
      const gapEnd = i + 1 < n ? out[i + 1]!.start : limit;
      if (!(gapEnd >= gapStart) || !Number.isFinite(gapEnd)) continue;
      const s = silenceIn(silences, gapStart, gapEnd);
      if (!s) continue;
      if (i >= 0) {
        if (s.start > out[i]!.end) out[i]!.end = Math.min(s.start, gapEnd);
        endMeasured[i] = true;
      }
      if (i + 1 < n) {
        if (s.end < out[i + 1]!.start) out[i + 1]!.start = Math.max(s.end, i >= 0 ? out[i]!.end : 0);
        startMeasured[i + 1] = true;
      }
    }
  }
  // 4. The sound.
  const short = out.map((w) => w.end - w.start < SHORT_WORD_S - 1e-9);
  if (envelopeInformative(envelope)) {
    const { rate, db } = envelope;
    const thr = speechThreshold(envelope);
    const loud = (k: number) => k >= 0 && k < db.length && db[k]! > thr;
    const end = db.length / rate;
    const silenceAfter = (t: number) => Math.min(...(silences ?? []).filter((x) => x.end > t + 1e-9).map((x) => Math.max(x.start, t)), Infinity);
    const silenceBefore = (t: number) => Math.max(...(silences ?? []).filter((x) => x.start < t - 1e-9).map((x) => Math.min(x.end, t)), -Infinity);
    // Ends, left to right.
    for (let i = 0; i < n; i++) {
      const w = out[i]!;
      if (endMeasured[i]) continue;
      const next = i + 1 < n ? out[i + 1]!.start : Math.min(limit, end);
      const cap = w.end + (short[i] ? SHORT_WORD_MAX_S : MAX_EXTEND_S);
      const silence = silenceAfter(w.end);
      const hold = Math.round(QUIET_HOLD_S * rate);
      // The quiet after the sound may run past the cap; sound past the cap may not.
      const bound = Math.min(next, cap + hold / rate, silence, end);
      let k = Math.floor(w.end * rate + 1e-6);
      // A timestamp in the quiet: its sound may start a little after it, so quiet before the first
      // loud window doesn't end it (not once its end follows sound: then it's an ordinary edge).
      const search = Math.round((short[i] && !loud(k - 1) ? SHORT_WORD_SPAN_S : QUIET_HOLD_S) * rate);
      let lastLoud = -1;
      let quiet = 0;
      let capped = false;
      for (; k < bound * rate - 1e-6 && quiet < (lastLoud < 0 ? search : hold); k++) {
        if (loud(k) && k >= cap * rate - 1e-6) {
          capped = true;
          break;
        }
        if (loud(k)) (lastLoud = k), (quiet = 0);
        else quiet++;
      }
      if (!capped && (quiet >= (lastLoud < 0 ? search : hold) || bound === silence || bound === end)) {
        // The sound went quiet (or a silence or the file's end came first): its end, measured.
        if (lastLoud >= 0) w.end = Math.max(w.end, Math.min(bound, (lastLoud + 1) / rate));
        endMeasured[i] = true;
      } else if (!capped && bound === next) {
        // The sound runs into the next word: split the gap at its quietest window.
        let best = -1;
        for (let j = Math.floor(w.end * rate + 1e-6); j < next * rate - 1e-6; j++) if (best < 0 || db[j]! < db[best]!) best = j;
        if (best >= 0) w.end = Math.max(w.end, Math.min(next, best / rate));
      }
      // Still loud at the cap: not this word's tail (music, another voice); left as it was.
    }
    // Starts, right to left.
    for (let i = n - 1; i >= 0; i--) {
      const w = out[i]!;
      if (startMeasured[i]) continue;
      const prev = i > 0 ? out[i - 1]!.end : 0;
      const cap = w.start - (short[i] ? SHORT_WORD_MAX_S : MAX_EXTEND_S);
      const silence = silenceBefore(w.start);
      const hold = Math.round(QUIET_HOLD_S * rate);
      const bound = Math.max(prev, cap - hold / rate, silence, 0);
      let k = Math.ceil(w.start * rate - 1e-6) - 1;
      const search = Math.round((short[i] && !loud(k + 1) ? SHORT_WORD_SPAN_S : QUIET_HOLD_S) * rate);
      let firstLoud = -1;
      let quiet = 0;
      let capped = false;
      for (; k >= bound * rate - 1e-6 && k >= 0 && quiet < (firstLoud < 0 ? search : hold); k--) {
        if (loud(k) && k < cap * rate - 1e-6) {
          capped = true;
          break;
        }
        if (loud(k)) (firstLoud = k), (quiet = 0);
        else quiet++;
      }
      if (!capped && (quiet >= (firstLoud < 0 ? search : hold) || bound === silence || bound === 0)) {
        if (firstLoud >= 0) w.start = Math.min(w.start, Math.max(bound, firstLoud / rate));
        startMeasured[i] = true;
      } else if (!capped && bound === prev && firstLoud >= 0) {
        // Runs into the word before (its end was split at the quietest window): start there or at the sound.
        w.start = Math.min(w.start, Math.max(prev, firstLoud / rate));
      }
    }
  } else {
    // No envelope: a timestamp gets SHORT_WORD_SPAN_S around it, between its neighbours, not into a silence.
    for (let i = 0; i < n; i++) {
      if (!short[i]) continue;
      const w = out[i]!;
      const mid = (w.start + w.end) / 2;
      const prevEnd = Math.max(i > 0 ? out[i - 1]!.end : 0, ...(silences ?? []).filter((x) => x.start < mid && x.end > mid - SHORT_WORD_SPAN_S / 2).map((x) => Math.min(x.end, mid)));
      const nextStart = Math.min(i + 1 < n ? out[i + 1]!.start : limit, ...(silences ?? []).filter((x) => x.end > mid && x.start < mid + SHORT_WORD_SPAN_S / 2).map((x) => Math.max(x.start, mid)));
      w.start = Math.min(w.start, Math.max(prevEnd, mid - SHORT_WORD_SPAN_S / 2));
      w.end = Math.max(w.end, Math.min(nextStart, mid + SHORT_WORD_SPAN_S / 2));
    }
  }
  for (const w of out) {
    w.start = ms3(w.start);
    w.end = ms3(Math.max(w.end, w.start));
  }
  return { words: out, endMeasured, startMeasured };
}

/** speechEdges' words (where each word is heard, D74). */
export const speechWords = <W extends { readonly start: number; readonly end: number }>(words: ReadonlyArray<W>, options: SpeechOptions = {}): Array<W> => speechEdges(words, options).words;
