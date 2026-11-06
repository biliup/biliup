// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { textLook } from "@/app/lib/editor/core/text/looks";
import { textPlacement, textDecor } from "@/app/lib/editor/core/visual/render";
import { editWords } from "@/app/lib/editor/core/text/transcript";
import { gapOf, resolveStarts } from "@/app/lib/editor/core/timeline/rules";
import type { Project, Item } from "@/app/lib/editor/core/timeline/types";
import { joinWords, textCharacters, wordSeparator } from '@/app/lib/editor/core/text/word-spacing';
import { captionSettings, SENTENCE_END, ABBREVIATIONS, type CaptionCard, captionWordText, CAPTION_HOLD_S, CAPTION_BRIDGE_S, CAPTION_MIN_S, captionMode, CAPTION_DEFAULTS, CAPTION_LINE_BOUNDS } from "@/app/lib/editor/core/text/captions/settings";

/** Word text made safe for textHtml's markup ([[accent]], **bold**, `mono` stay literal-ish). */
const plain = (text: string) => text.replace(/`/g, "'").replace(/\*\*+/g, "*").replace(/\[\[+/g, "[").replace(/\]\]+/g, "]").replace(/\s+/g, " ");

/** The items a captions item follows when it has `anyTrack` (D215): every clip or sound that plays its source, on any track, unmuted. */
function followedItems(project: Pick<Project, "tracks">, item: Item): Set<string> {
  return new Set(project.tracks.flatMap((t) => t.items.filter((i) => (i.type === "clip" || i.type === "audio") && i.media === item.source && !i.mute).map((i) => i.id)));
}

/**
 * The played words of the item's source, per clip in edit order (the editWords rule): through the
 * main track (a main clip's own sound or its linked microphone, D215), or with `anyTrack` (D215)
 * through every item that plays the source wherever it is.
 */
function heardByClip(project: Pick<Project, "tracks" | "canvas" | "media">, item: Item) {
  const words = item.words ?? [];
  const out: Array<{ clip: string; cycle: number; words: Array<{ index: number; from: number; to: number }> }> = [];
  if (!item.source || !words.length) return out;
  for (const w of editWords(project, new Map([[item.source, words]]), item.anyTrack ? followedItems(project, item) : undefined)) {
    const last = out.at(-1);
    if (last && last.clip === w.clip && last.cycle === w.cycle) last.words.push({ index: w.index, from: w.start, to: w.end });
    else out.push({ clip: w.clip, cycle: w.cycle, words: [{ index: w.index, from: w.start, to: w.end }] });
  }
  return out;
}

/**
 * Groups one clip's visible words into cards: a new card at a forced break, after a pause of at
 * least `pause`, after a sentence end, when the card would run past maxSeconds, or when the word
 * doesn't fit in maxLines lines of maxChars (and maxWords) — a word longer than a line gets its own.
 */
function group(words: ReadonlyArray<{ index: number; text: string; from: number; to: number; hidden: boolean }>, s: ReturnType<typeof captionSettings>, breaks: ReadonlySet<number>, fps: number) {
  const cards: Array<{ words: Array<(typeof words)[number]>; lines: Array<{ chars: number; count: number }> }> = [];
  let card: (typeof cards)[number] | undefined;
  // Where the last word heard ends, shown or not: a hidden filler is speech, not a pause.
  let heardEnd = -Infinity;
  for (const w of words) {
    const gap = (w.from - heardEnd) / fps;
    heardEnd = Math.max(heardEnd, w.to);
    if (w.hidden) continue;
    const prev = card?.words.at(-1);
    const sentence = !!prev && SENTENCE_END.test(prev.text) && !ABBREVIATIONS.has(prev.text.toLowerCase().replace(/["'”’)\]]+$/, ""));
    let fresh = !card || !prev || breaks.has(w.index) || gap >= s.pause - 1e-9 || sentence || (w.to - card.words[0]!.from) / fps > s.maxSeconds + 1e-9;
    if (!fresh && card) {
      const line = card.lines.at(-1)!;
      const added = textCharacters(w.text)+wordSeparator(prev!.text,w.text).length;
      const fits = line.chars + added <= s.maxChars && line.count + 1 <= s.maxWords;
      if (fits) {
        line.chars += added;
        line.count += 1;
        card.words.push(w);
        continue;
      }
      if (card.lines.length < s.maxLines) {
        card.lines.push({ chars: textCharacters(w.text), count: 1 });
        card.words.push(w);
        continue;
      }
      fresh = true;
    }
    card = { words: [w], lines: [{ chars: textCharacters(w.text), count: 1 }] };
    cards.push(card);
  }
  return cards;
}

const memo = new WeakMap<Item, { tracks: Project['tracks']; media: Project['media']; canvas: Project['canvas']; theme: Project['theme']; cards: ReadonlyArray<CaptionCard> }>();

/**
 * Every card of a captions item, in timeline order, inside its window [start, start + duration):
 * each from its first word's first frame to its last word's end plus CAPTION_HOLD_S, never into
 * the next card, and closed up to the next card when the gap would be under CAPTION_BRIDGE_S.
 * Hidden words (a correction to "", a filler with hideFillers) aren't in any card. Memoised per
 * item, tracks and source media, so the preview can ask on every frame.
 */
export function captionCards(project: Pick<Project, "tracks" | "canvas" | "media"> & Partial<Pick<Project, "theme">>, item: Item): ReadonlyArray<CaptionCard> {
  if (item.type !== "captions") return [];
  const fps = project.canvas.fps;
  const known = memo.get(item);
  if (known && known.tracks === project.tracks && known.media === project.media && known.canvas === project.canvas && known.theme === project.theme) return known.cards;
  const s = captionSettings(item, project);
  const breaks = new Set(item.breaks ?? []);
  const hideFillers = item.hideFillers === true;
  const raw: Array<{ clip: string; words: Array<{ index: number; text: string; from: number; to: number }>; lines: Array<number> }> = [];
  for (const run of heardByClip(project, item)) {
    const marked = run.words.map((w) => {
      const text = plain(captionWordText(item, w.index));
      return { ...w, text, hidden: !text || (hideFillers && item.words![w.index]!.filler === true && !item.edits?.some((e) => e.word === w.index)) };
    });
    for (const c of group(marked, s, breaks, fps)) raw.push({ clip: run.clip, words: c.words.map(({ hidden: _, ...w }) => w), lines: c.lines.map((l) => l.count) });
  }
  const hold = Math.round(CAPTION_HOLD_S * fps);
  const bridge = CAPTION_BRIDGE_S * fps;
  const least = Math.round(CAPTION_MIN_S * fps);
  // Captions made by "Make captions" cover the whole main track (coversMain); others draw in their window.
  const lo = item.coversMain ? -Infinity : (item.start ?? 0);
  const hi = item.coversMain ? Infinity : lo + item.duration;
  // Where each main clip ends, and whether the next clip plays the same media (a hold may run into it).
  const clipEnds = new Map<string, { end: number; sameNext: boolean }>();
  if (item.anyTrack) {
    // Each followed item's end, and whether another one plays on from there (D215).
    const starts = resolveStarts(project);
    const followed = followedItems(project, item);
    const spans = project.tracks.flatMap((tr) => tr.items.filter((i) => followed.has(i.id)).map((i) => ({ id: i.id, start: starts.get(i.id) ?? 0, end: (starts.get(i.id) ?? 0) + i.duration })));
    for (const sp of spans) clipEnds.set(sp.id, { end: sp.end, sameNext: spans.some((o) => o.id !== sp.id && o.start === sp.end) });
  } else {
    let t = 0;
    const main = project.tracks.find(t => t.role === 'main');
    main?.items.forEach((it, k) => {
      t += gapOf(it) + it.duration;
      const next = main.items[k + 1];
      // A gap after it (D224): nothing plays on from its end.
      clipEnds.set(it.id, { end: t, sameNext: next?.type === "clip" && next.media === item.source && gapOf(next) === 0 });
    });
  }
  const cards: Array<CaptionCard> = [];
  raw.forEach((c, i) => {
    const from = c.words[0]!.from;
    const lastEnd = Math.max(...c.words.map((w) => w.to));
    let to = Math.max(from + 1, lastEnd + hold, from + least);
    const next = raw[i + 1]?.words[0]?.from;
    if (next !== undefined && (next < to || next - to < bridge)) to = Math.max(next, from + 1);
    // Not past a cut into other footage: the hold stays inside the card's clip.
    const clip = clipEnds.get(c.clip);
    if (clip && !clip.sameNext) to = Math.max(from + 1, Math.min(to, clip.end));
    const a = Math.max(from, lo);
    const b = Math.min(to, hi);
    if (b > a) cards.push({ from: a, to: b, clip: c.clip, words: c.words, lines: c.lines });
  });
  memo.set(item, { tracks: project.tracks, media: project.media, canvas: project.canvas, theme: project.theme, cards });
  return cards;
}

/** The frames the captions draw over: first card's start to last card's end (undefined: none). */
export function captionExtent(project: Pick<Project, "tracks" | "canvas" | "media">, item: Item): { from: number; to: number } | undefined {
  const cards = captionCards(project, item);
  return cards.length ? { from: cards[0]!.from, to: cards.at(-1)!.to } : undefined;
}

/** The card showing at frame f (whole frames), with its position in the list. */
export function captionCardAt(project: Pick<Project, "tracks" | "canvas" | "media">, item: Item, f: number): { card: CaptionCard; n: number } | undefined {
  const cards = captionCards(project, item);
  const frame = Math.floor(f + 1e-9);
  let lo = 0;
  let hi = cards.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const c = cards[mid]!;
    if (frame < c.from) hi = mid - 1;
    else if (frame >= c.to) lo = mid + 1;
    else return { card: c, n: mid };
  }
  return undefined;
}

/** In word and single mode: the card's word speaking at frame f (from its start to the next word's start; the last one to the card's end). */
export function activeWord(card: CaptionCard, f: number): number {
  const frame = Math.floor(f + 1e-9);
  let k = 0;
  for (let i = 1; i < card.words.length; i++) if (card.words[i]!.from <= frame) k = i;
  return k;
}

/**
 * What a captions item shows at frame f, as a text item core textHtml draws (undefined: nothing):
 * the card's lines (\n between them), in word mode the speaking word as an [[accent]], in single
 * mode only that word. Its id is the captions item's, so clicking it in the preview selects the
 * captions. Placed with the captions' look and transform, bottom-anchored: a card with fewer lines
 * than maxLines moves down by the missing lines, so the last line always sits in the same place.
 */
export function captionAt(project: Pick<Project, "tracks" | "canvas" | "theme" | "media">, item: Item, f: number): Item | undefined {
  const at = captionCardAt(project, item, f);
  if (!at) return undefined;
  const { card } = at;
  const mode = captionMode(item);
  const k = mode === "line" ? -1 : activeWord(card, f);
  const karaoke = item.style?.karaoke === true;
  let text: string;
  if (mode === "single") text = card.words[k]!.text;
  else {
    const parts: Array<string> = [];
    let i = 0;
    for (const count of card.lines) {
      // A "]" right before the closing ]] would end the accent early: a zero-width space after each keeps them apart.
      const line = card.words.slice(i, i + count).map((w) => w.text.replace(/\]/g, "]\u200B"));
      // Karaoke (D166): every word said so far on the line is filled (one accent run), not only the speaking one.
      const upTo = karaoke ? Math.min(count, k + 1 - i) : 0;
      const words=card.words.slice(i,i+count).map(word=>word.text);
      if (upTo > 0) parts.push(joinWords(words,(_word,j)=>`${j===0?'[[':''}${line[j]}${j===upTo-1?']]':''}`));
      else parts.push(joinWords(words,(word,j)=>i+j===k?`[[${line[j]}]]`:word));
      i += count;
    }
    text = parts.join("\n");
  }
  const style = { preset: CAPTION_DEFAULTS.look, ...(item.style ?? {}) } as Item["style"] & {};
  const transform = { ...(item.transform ?? {}), ...captionBox(project, item) };
  return { id: item.id, type: "text", ...(item.name ? { name: item.name } : {}), start: card.from, duration: card.to - card.from, text, style, transform: { ...transform, y: transform.bottom }, anchor: "bottom" };
}

/**
 * Where a captions item's cards sit (canvas px): x and width from the look and transform, `y` the top
 * of a full card (maxLines lines) as the Inspector and canvas drag edit it, `bottom` its bottom edge.
 * Cards are drawn anchored at `bottom` (textHtml's anchor), so a card with fewer lines sits lower and
 * anything that wraps grows upward, never off the canvas's bottom.
 */
export function captionBox(project: Pick<Project, "canvas"> & Partial<Pick<Project, "theme">>, item: Pick<Item, "style" | "transform" | "lines">) {
  const style = { preset: CAPTION_DEFAULTS.look, ...(item.style ?? {}) } as NonNullable<Item["style"]>;
  const shell = { transform: item.transform, style };
  const place = textPlacement(shell, project);
  const look = textLook(style.preset);
  const lineHeight = typeof style.lineHeight === "number" ? style.lineHeight : look.lineHeight;
  const decor = textDecor(shell, look, place.size);
  const pad = decor.background && decor.background.wrap !== "lines" ? Math.max(0, decor.background.padding ?? 0) : 0;
  const height = captionSettings(item).maxLines * place.size * lineHeight + pad;
  return { x: place.x, y: place.y, ...(place.w !== undefined ? { w: place.w } : {}), height, bottom: place.y + height, size: place.size };
}

/** Pieces of the timeline where a captions item draws one unchanging look: per card, and per speaking word in word and single mode. */
export function captionSegments(project: Pick<Project, "tracks" | "canvas" | "media"> & Partial<Pick<Project, "theme">>, item: Item): Array<{ readonly from: number; readonly to: number; readonly text: Item }> {
  const out: Array<{ from: number; to: number; text: Item }> = [];
  const mode = captionMode(item);
  for (const card of captionCards(project, item)) {
    let from = card.from;
    let k = mode === "line" ? -1 : activeWord(card, from);
    for (let f = card.from + 1; f <= card.to; f++) {
      const now = f === card.to ? -2 : mode === "line" ? -1 : activeWord(card, f);
      if (now === k) continue;
      out.push({ from, to: f, text: captionAt(project, item, from)! });
      from = f;
      k = now;
    }
  }
  return out;
}

/** A `lines` value checked against the schema's bounds and keys (null for a field: its default, when `nulls`). */
export function checkCaptionLines(lines: unknown, nulls = false): { readonly lines: Record<string, number | null> } | { readonly problem: string } {
  if (typeof lines !== "object" || lines === null || Array.isArray(lines)) return { problem: "lines must be an object: maxChars, maxLines, maxSeconds, maxWords, pause." };
  const out: Record<string, number | null> = {};
  for (const [k, v] of Object.entries(lines)) {
    const b = CAPTION_LINE_BOUNDS[k as keyof typeof CAPTION_LINE_BOUNDS];
    if (!b) return { problem: `lines has no setting ${k} (maxChars, maxLines, maxSeconds, maxWords, pause).` };
    if (v === undefined) continue;
    if (v === null && nulls) {
      out[k] = null;
      continue;
    }
    if (typeof v !== "number" || !Number.isFinite(v) || (b.integer && !Number.isInteger(v)) || v < b.min || v > b.max || (k === "maxSeconds" && v <= 0)) {
      return { problem: k === "maxLines" ? "lines.maxLines must be 1 or 2." : `lines.${k} must be ${b.integer ? "a whole number" : "a number"} from ${b.min} to ${b.max}.` };
    }
    out[k] = v;
  }
  return { lines: out };
}
