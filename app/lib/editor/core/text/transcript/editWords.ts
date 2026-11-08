// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { gapOf, resolveStarts } from "@/app/lib/editor/core/timeline/rules";
import type { Project, Item } from "@/app/lib/editor/core/timeline/types";
import { type CutWord, floorF, ceilF } from "@/app/lib/editor/core/text/transcript/cutSpans";
import { captureKind } from '@/app/lib/editor/core/media/capture';
import { sourceSpans } from '@/app/lib/editor/core/timeline/sourceSpans';

/** The words under the main track in edit order, as the Transcript panel lists them (what you'd hear). */
export interface EditWord {
  /**
   * The item it's heard in (main mode: the main-track clip; with a scope, the scoped item), and the
   * word's index in its media's transcript.
   */
  readonly clip: string;
  readonly media: string;
  readonly index: number;
  /** The pass through a looped source; distinct occurrences share their transcript word index. */
  readonly cycle: number;
  /** Timeline frames it's heard over: [start, end). */
  readonly start: number;
  readonly end: number;
  /** D215: the linked or grouped sound it's heard in when that isn't `clip` itself (a take's microphone under its picture on Main). */
  readonly via?: string;
}

/**
 * D215: the sound parts of an item, best first: the items linked to it (D123 linkedTo: a detached
 * sound, a take's microphone under its picture on Main), then its group's (a take still on overlay
 * tracks), clips and sounds only, never `item` itself. A captured microphone comes before shared
 * audio and the other picture. Track and item names do not identify the source.
 */
export function soundParts(project: Pick<Project, "tracks" | "media">, item: Pick<Item, "id" | "group">): Array<Item> {
  const { byLink, byGroup } = partsIndex(project);
  const linked: Array<{ item: Item; rank: number }> = [];
  const add = (list: ReadonlyArray<{ item: Item; rank: number }> | undefined, tie: number) => {
    for (const { item: i, rank } of list ?? []) if (i.id !== item.id && !linked.some((x) => x.item.id === i.id)) linked.push({ item: i, rank: tie + rank });
  };
  add(byLink.get(item.id), 0);
  if (item.group) add(byGroup.get(item.group), 4);
  return linked.sort((a, b) => a.rank - b.rank).map((x) => x.item);
}

/** Both track links and media origins belong to the immutable project snapshot. */
const partsIndexes = new WeakMap<Pick<Project, "tracks" | "media">, { byLink: Map<string, Array<{ item: Item; rank: number }>>; byGroup: Map<string, Array<{ item: Item; rank: number }>> }>();
function partsIndex(project: Pick<Project, "tracks" | "media">) {
  let index = partsIndexes.get(project);
  if (index) return index;
  index = { byLink: new Map(), byGroup: new Map() };
  const microphones = new Set(project.media.filter(m => captureKind(m) === 'voiceover').map(m => m.id));
  for (const track of project.tracks) {
    if (track.role === "main") continue;
    for (const i of track.items) {
      if (i.type !== "audio" && i.type !== "clip") continue;
      const entry = { item: i, rank: i.media && microphones.has(i.media) ? 0 : i.type === 'audio' ? 1 : 2 };
      if (i.linkedTo) (index.byLink.get(i.linkedTo) ?? index.byLink.set(i.linkedTo, []).get(i.linkedTo)!).push(entry);
      if (i.group) (index.byGroup.get(i.group) ?? index.byGroup.set(i.group, []).get(i.group)!).push(entry);
    }
  }
  partsIndexes.set(project, index);
  return index;
}

/**
 * D215: the item whose sound is `item`'s speech: a sound part with a different file (a take's
 * microphone: soundParts) when one has sound, else the item itself when its file has sound (a muted
 * clip's detached sound plays the same file, so it's the same words), else undefined.
 */
export function speechItem(project: Pick<Project, "tracks" | "media">, item: Item): Item | undefined {
  const audible = (i: Item) => {
    const m = i.media ? project.media.find((x) => x.id === i.media) : undefined;
    return !!m && m.kind !== "image" && (m.kind === "audio" || m.probe?.hasAudio === true);
  };
  const part = soundParts(project, item).find((p) => p.media !== item.media && audible(p));
  if (part) return part;
  if ((item.type === "clip" || item.type === "audio") && audible(item)) return item;
  return soundParts(project, item).find(audible);
}

/** D215: what the Transcript tab and transcript_edit work on for an item off the main track. */
export interface TranscriptScope {
  /** The media whose transcript it is (a take's microphone for its camera). */
  readonly media: string;
  /** The items that play it there, in timeline order (cut by text together). */
  readonly items: ReadonlyArray<string>;
  /** The item that plays it (the scope's first). */
  readonly anchor: string;
}

/**
 * D215: the transcript scope of an item: undefined when it's heard through the main track (on
 * Main, or linked to a main clip), else the media its speech is (speechItem) and every item playing
 * that media on the same track or in the same group (the pieces a cut left, a take's microphone).
 */
export function transcriptScope(project: Pick<Project, "tracks" | "media">, itemId: string): TranscriptScope | undefined {
  const at = project.tracks.flatMap((t) => t.items.map((i) => ({ item: i, track: t }))).find((x) => x.item.id === itemId);
  if (!at || at.track.role === "main") return undefined;
  const mainIds = new Set(project.tracks.filter((t) => t.role === "main").flatMap((t) => t.items.map((i) => i.id)));
  if (at.item.linkedTo && mainIds.has(at.item.linkedTo)) return undefined;
  const sound = speechItem(project, at.item);
  if (!sound?.media) return undefined;
  if (sound.linkedTo && mainIds.has(sound.linkedTo)) return undefined;
  const track = project.tracks.find((t) => t.items.some((i) => i.id === sound.id))!;
  const starts = resolveStarts(project);
  const plays = (i: Item) => (i.type === "clip" || i.type === "audio") && i.media === sound.media;
  const own = new Set(track.items.filter(plays).map((i) => i.id));
  const groups = new Set(project.tracks.flatMap((t) => t.items.filter((i) => own.has(i.id) && i.group).map((i) => i.group!)));
  const items = project.tracks.flatMap((t) => (t.role === "main" ? [] : t.items.filter((i) => plays(i) && (own.has(i.id) || (i.group !== undefined && groups.has(i.group))))));
  items.sort((a, b) => (starts.get(a.id) ?? 0) - (starts.get(b.id) ?? 0));
  return { media: sound.media, items: items.map((i) => i.id), anchor: sound.id };
}

/**
 * Every word heard on the main track, clip by clip in edit order: a word belongs to a clip when its
 * middle is inside the stretch of the media the clip plays; its timeline frames are where its
 * covering frames land (clipped to the clip). `transcripts` maps media ids to their words. A main
 * clip whose sound part has another transcribed file (D215: a take's microphone linked to its
 * camera) is heard through that part (`via`). With `scope` (D215), the items named there instead,
 * on any track, each at its own start, in timeline order (a word two of them play at once is listed
 * once).
 */
export function editWords(project: Pick<Project, "tracks" | "canvas" | "media">, transcripts: ReadonlyMap<string, ReadonlyArray<CutWord>>, scope?: ReadonlySet<string>): Array<EditWord> {
  const fps = project.canvas.fps;
  const main = project.tracks.find((t) => t.role === "main");
  const out: Array<EditWord> = [];
  // Each word list's middles, and whether they only go up (whisper's do): then each clip finds its
  // first word by binary search and stops at the first one past its stretch, so a long edit of many
  // clips over an hour of words costs O(clips · log words + words heard), not O(clips · words).
  const mids = new Map<ReadonlyArray<CutWord>, { mid: Float64Array; sorted: boolean }>();
  const midsOf = (words: ReadonlyArray<CutWord>) => {
    let m = mids.get(words);
    if (!m) {
      const mid = Float64Array.from(words, (w) => (w.start + w.end) / 2);
      let sorted = true;
      for (let i = 1; i < mid.length && sorted; i++) if (mid[i]! < mid[i - 1]!) sorted = false;
      m = { mid, sorted };
      mids.set(words, m);
    }
    return m;
  };
  /** The words `src` plays, placed from timeline frame `t` (clipped to it), listed under `clip`. */
  const walk = (src: Item, t: number, clip: string, window?: { start: number; end: number }) => {
    const words = src.media ? transcripts.get(src.media) : undefined;
    if (!words) return;
    const inF = src.in ?? 0;
    const speed = src.speed ?? 1;
    const { mid, sorted } = midsOf(words);
    const length = project.media.find(m => m.id === src.media)?.probe?.frames;
    for (const span of sourceSpans(src, length, window)) {
      const lo = span.from / fps;
      const hi = span.to / fps;
      let first = 0;
      if (sorted) {
        let a = 0;
        let b = mid.length;
        while (a < b) {
          const m = (a + b) >> 1;
          if (mid[m]! < lo) a = m + 1;
          else b = m;
        }
        first = a;
      }
      for (let index = first; index < words.length; index++) {
        const w = words[index]!;
        const c = mid[index]!;
        if (c >= hi) {
          if (sorted) break;
          continue;
        }
        if (c < lo) continue;
        const start = Math.max(span.start, Math.floor((floorF(w.start * fps) + span.offset - inF) / speed));
        const end = Math.min(span.end, Math.max(start + 1, Math.ceil((ceilF(w.end * fps) + span.offset - inF) / speed)));
        if (end > start) out.push({ clip, media: src.media!, index, cycle: span.cycle, start: t + start, end: t + end, ...(src.id !== clip ? { via: src.id } : {}) });
      }
    }
  };
  if (scope) {
    const starts = resolveStarts(project);
    const items = project.tracks.flatMap((t) => t.items.filter((i) => scope.has(i.id) && (i.type === "clip" || i.type === "audio")));
    items.sort((a, b) => (starts.get(a.id) ?? 0) - (starts.get(b.id) ?? 0));
    for (const item of items) walk(item, starts.get(item.id) ?? 0, item.id);
    out.sort((a, b) => a.start - b.start || a.end - b.end);
    // A word two items play at the same moment (a muted clip and its detached sound) is heard once.
    const kept: Array<EditWord> = [];
    const lastHeard = new Map<string, Map<number, EditWord>>();
    for (const w of out) {
      const byWord = lastHeard.get(w.media) ?? new Map<number, EditWord>();
      lastHeard.set(w.media, byWord);
      const previous = byWord.get(w.index);
      if (previous && w.start < previous.end) continue;
      kept.push(w);
      byWord.set(w.index, w);
    }
    return kept;
  }
  if (!main) return out;
  let t = 0;
  let starts: Map<string, number> | undefined;
  for (const item of main.items) {
    t += gapOf(item);
    if (item.type === "clip") {
      // A sound part with another transcribed file (a take's microphone) speaks for the clip.
      const part = soundParts(project, item).find((p) => p.media !== undefined && p.media !== item.media && transcripts.has(p.media));
      if (part) {
        starts ??= resolveStarts(project);
        const partStart = starts.get(part.id) ?? t;
        walk(part, partStart, item.id, { start: t - partStart, end: t + item.duration - partStart });
      } else if (item.media) walk(item, t, item.id);
    }
    t += item.duration;
  }
  return out;
}
