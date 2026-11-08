// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { joinWords } from '@/app/lib/editor/core/text/word-spacing';
import { sliceGainKeys } from "@/app/lib/editor/core/audio/audio";
import type { Project, Item, Op, TimedModule } from "@/app/lib/editor/core/timeline/types";
import { type Pause, tailFrames, handleFrames } from "@/app/lib/editor/core/text/transcript/fillers";

export interface CutWord {
  readonly text: string;
  /** Seconds of the media file. */
  readonly start: number;
  readonly end: number;
}

interface Span {
  from: number;
  to: number;
  words: Array<number>;
  pauses: number;
  fillers: number;
}

/** A piece shorter than this (frames) is a sliver: a cut doesn't leave one between itself and an earlier cut. */
const MIN_PIECE = 3;
export const floorF = (x: number) => Math.floor(x + 1e-6);
export const ceilF = (x: number) => Math.ceil(x - 1e-6);
export const quote = (text: string, max = 48) => `"${text.length > max ? `${text.slice(0, max - 1)}…` : text}"`;
export const wordsText = (words: ReadonlyArray<CutWord>, ids: ReadonlyArray<number>) => joinWords(ids.map((i) => words[i]!.text.trim()));
export const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;
export const wordIds = (ids: ReadonlyArray<number>) => `w${ids[0]}${ids.length > 1 ? `–w${ids.at(-1)}` : ""}`;

/** An id nothing in the project (or `taken`) uses yet: base-1, base-2, … */
export function freeId(project: Project, base: string, taken: Set<string>) {
  const ids = new Set([...taken, ...project.media.map((m) => m.id), ...project.tracks.flatMap((t) => [t.id, ...t.items.map((i) => i.id)]), ...(project.scenes ?? []).map((s) => s.id)]);
  for (let n = 1; ; n++) {
    const id = `${base}-${n}`;
    if (!ids.has(id)) {
      taken.add(id);
      return id;
    }
  }
}

/** Runs of consecutive indices. */
export function runsOf(ids: Iterable<number>): Array<Array<number>> {
  const sorted = [...new Set(ids)].sort((a, b) => a - b);
  const runs: Array<Array<number>> = [];
  for (const i of sorted) {
    const last = runs.at(-1);
    if (last && last.at(-1) === i - 1) last.push(i);
    else runs.push([i]);
  }
  return runs;
}

/**
 * The stretches of the media to cut from one clip, in project frames of the media, merged. A word
 * counts as kept when it isn't removed and the clip still plays it (its middle frame); only kept
 * words get a handle. Words: a run of removed words, widened over neighbours that aren't kept but
 * of which the clip still plays a few frames (the sliver an earlier cut's handle left), from the frame its first word starts in to the frame
 * after its last one ends (the transcript's covering convention), pulled back to keep tailFrames after
 * the kept word before it and handleFrames before the kept word after it. So deleting touching words one by one ends like deleting them
 * together, and a selection across a clip seam leaves no slivers. Pauses: inward (both ends inside
 * the gap), keeping tailFrames after a kept word and handleFrames before one (none next to one that goes). Edge pauses
 * (before the first word, after the last) run to the media's start or end.
 */
export function spansFor(words: ReadonlyArray<CutWord>, removed: ReadonlySet<number>, fillerWords: ReadonlySet<number>, kept: (i: number) => boolean, sliver: (i: number) => boolean, pauses: ReadonlyArray<Pause>, fps: number, length: number, tight: Set<number>, measured: { readonly endMeasured: ReadonlyArray<boolean>; readonly startMeasured: ReadonlyArray<boolean> } = { endMeasured: [], startMeasured: [] }): Array<Span> {
  const n = words.length;
  // Kept after a kept word (its tail) and before one (its head), D74.
  const T = tailFrames(fps);
  const H = handleFrames(fps);
  const f = (s: number) => s * fps;
  const clamp = (v: number) => Math.min(length, Math.max(0, v));
  const spans: Array<Span> = [];
  // Words a removed run took with it (slivers of words mostly cut): no pause handle next to them.
  const absorbed = new Set<number>();
  for (const run of runsOf(removed)) {
    let a = run[0]!;
    let b = run.at(-1)!;
    while (a > 0 && !kept(a - 1) && sliver(a - 1)) a--;
    while (b + 1 < n && !kept(b + 1) && sliver(b + 1)) b++;
    let from = floorF(f(words[a]!.start));
    let to = ceilF(f(words[b]!.end));
    // A neighbour that's gone from the clip (an earlier cut took it): a sliver of the gap up to it
    // (under MIN_PIECE frames) goes too, so cutting words one by one leaves no blip between the cuts.
    if (a > 0 && !kept(a - 1) && !sliver(a - 1)) {
      const gapFrom = ceilF(f(words[a - 1]!.end));
      if (from - gapFrom < MIN_PIECE) from = Math.min(from, gapFrom);
    }
    if (b + 1 < n && !kept(b + 1) && !sliver(b + 1)) {
      const gapTo = floorF(f(words[b + 1]!.start));
      if (gapTo - to < MIN_PIECE) to = Math.max(to, gapTo);
    }
    // A kept word's edge the sound measured (D74: the envelope went quiet, or a silence): the
    // removed words' sound goes whole, the margin only takes the gap. Otherwise (whisper's edges,
    // or two words whose sound runs together) the kept word's margin wins.
    if (a > 0 && kept(a - 1)) {
      const keptEnd = ceilF(f(words[a - 1]!.end));
      from = measured.endMeasured[a - 1] ? Math.max(from, keptEnd) : Math.max(from, keptEnd + T);
    }
    if (b + 1 < n && kept(b + 1)) {
      const keptStart = floorF(f(words[b + 1]!.start));
      to = measured.startMeasured[b + 1] ? Math.min(to, keptStart) : Math.min(to, keptStart - H);
    }
    from = clamp(from);
    to = clamp(to);
    if (to - from < 1) {
      for (const i of run) tight.add(i);
      continue;
    }
    for (let k = a; k <= b; k++) absorbed.add(k);
    spans.push({ from, to, words: [...run], pauses: 0, fillers: run.filter((i) => fillerWords.has(i)).length });
  }
  // A pause keeps a handle next to a word that isn't removed and still has frames in the clip, even
  // when its middle frame doesn't play there: a pause aligned to a silence can take the middle of a
  // word whisper timed too late, and the next plan (Remove pauses again, after a cut) must not then
  // cut the handle that was left (D72).
  // A word a removed run took with it isn't near: a handle there would leave a sliver (D72 review).
  const near = (i: number) => kept(i) || (!removed.has(i) && !absorbed.has(i) && sliver(i));
  for (const p of pauses) {
    const before = p.after >= 0 && near(p.after) ? T : 0;
    const after = p.after + 1 < n && near(p.after + 1) ? H : 0;
    const from = clamp(p.edge === "start" ? 0 : ceilF(f(p.start)) + before);
    const to = clamp(p.edge === "end" ? length : floorF(f(p.end)) - after);
    if (to - from >= 1) spans.push({ from, to, words: [], pauses: 1, fillers: 0 });
  }
  spans.sort((x, y) => x.from - y.from);
  const merged: Array<Span> = [];
  for (const sp of spans) {
    const last = merged.at(-1);
    if (last && sp.from <= last.to) {
      last.to = Math.max(last.to, sp.to);
      last.words.push(...sp.words);
      last.pauses += sp.pauses;
      last.fillers += sp.fillers;
    } else merged.push({ ...sp, words: [...sp.words] });
  }
  return merged;
}

/** update_item fixes for a piece a cut made shorter: fades, animations and keys that no longer fit. */
export function fitPiece(item: Item): Op | undefined {
  const d = item.duration;
  const set: Record<string, unknown> = {};
  const fi = item.fadeIn ?? 0;
  const fo = item.fadeOut ?? 0;
  if (fi + fo > d) {
    const nfi = Math.min(fi, d);
    const nfo = Math.min(fo, d - nfi);
    if (nfi !== fi) set.fadeIn = nfi || null;
    if (nfo !== fo) set.fadeOut = nfo || null;
  }
  if (item.animation && Object.values(item.animation).some((a) => a && a.duration > d)) {
    const fitted: Record<string, TimedModule> = {};
    for (const [k, a] of Object.entries(item.animation)) if (a) fitted[k] = { ...a, duration: Math.min(a.duration, d) };
    set.animation = fitted;
  }
  if (item.keys?.some((k) => k.frame > d)) {
    const kept = item.keys.filter((k) => k.frame <= d);
    set.keys = kept.length ? kept : null;
  }
  if (item.gainKeys?.some((k) => k.frame > d)) {
    set.gainKeys = sliceGainKeys(item.gainKeys, 0, d);
  }
  return Object.keys(set).length ? { op: "update_item", item: item.id, set } : undefined;
}

/** Where each kept part of an item cut at `ranges` (local, sorted, merged) ends up: its id and local start (planTextCut's split order). */
export function keptPieces(item: Item, ranges: ReadonlyArray<[number, number]>, newIds: ReadonlyArray<string>): Array<{ id: string; from: number; to: number }> {
  const out: Array<{ id: string; from: number; to: number }> = [];
  let at = 0;
  let id = item.id;
  let k = 0;
  for (const [a, b] of ranges) {
    if (a > at) out.push({ id, from: at, to: a });
    if (a > 0 && b < item.duration) id = newIds[k++]!;
    at = b;
  }
  if (at < item.duration) out.push({ id, from: at, to: item.duration });
  return out;
}
