// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { textLook } from "@/app/lib/editor/core/text/looks";
import { type RenderTarget, textPlacement, safeMargin, textDecor } from "@/app/lib/editor/core/visual/render";
import type { Item } from "@/app/lib/editor/core/timeline/types";
import { wideCharacterRatio } from '@/app/lib/editor/core/text/word-spacing';

export type CaptionMode = "line" | "word" | "single";
export const CAPTION_MODES: ReadonlyArray<CaptionMode> = ["line", "word", "single"];
export const CAPTION_MODE_NAMES: Readonly<Record<CaptionMode, string>> = { line: "Line by line", word: "Word by word", single: "One word at a time" };

/** One word of a captions item's snapshot: seconds of the media file, where it's heard. */
export interface CaptionWord {
  readonly text: string;
  readonly start: number;
  readonly end: number;
  /** A hesitation (um, uh, yyy): hidden when the item's hideFillers is on. */
  readonly filler?: boolean;
}

/** How words are grouped into cards. */
export interface CaptionLines {
  /** Characters per line (spaces included). */
  readonly maxChars?: number;
  /** Lines per card: 1 or 2. */
  readonly maxLines?: number;
  /** A card is at most this long (seconds, first word's start to last word's end). */
  readonly maxSeconds?: number;
  /** Words per line (no limit when absent). */
  readonly maxWords?: number;
  /** A gap between words at least this long (seconds) starts a new card. */
  readonly pause?: number;
}

export const CAPTION_DEFAULTS = { look: "text.caption", mode: "line" as CaptionMode, maxChars: 32, maxLines: 2, maxSeconds: 3, pause: 0.3 } as const;
/** The schema's bounds for `lines` (project.schema.json), checked by every planner. */
export const CAPTION_LINE_BOUNDS = {
  maxChars: { min: 4, max: 200, integer: true },
  maxLines: { min: 1, max: 2, integer: true },
  maxSeconds: { min: 0.1, max: 30, integer: false },
  maxWords: { min: 1, max: 1000, integer: true },
  pause: { min: 0, max: 10, integer: false },
} as const;
/** A card stays on screen at least this long (seconds), while the next card doesn't need the room. */
export const CAPTION_MIN_S = 0.7;
/** Words ending in a full stop that don't end a sentence. */
export const ABBREVIATIONS = new Set(["mr.", "mrs.", "ms.", "dr.", "prof.", "st.", "vs.", "e.g.", "i.e.", "etc.", "jr.", "sr.", "inc.", "approx.", "no.", "fig."]);
/** A card stays this long (seconds) after its last word ends... */
export const CAPTION_HOLD_S = 0.3;
/** ...and a gap shorter than this (seconds) to the next card is closed, so close cards don't flicker. */
export const CAPTION_BRIDGE_S = 0.5;
/** Word ends that close a sentence (a new card starts after them). */
export const SENTENCE_END = /[.?!…。！？]["'”’)\]）】》〉」』〕]*$/u;

/** A caption card: timeline frames [from, to), the clip it comes from and its words in lines. */
export interface CaptionCard {
  readonly from: number;
  readonly to: number;
  readonly clip: string;
  /** Its words in order: index into the snapshot, the text shown, timeline frames heard [from, to). */
  readonly words: ReadonlyArray<{ readonly index: number; readonly text: string; readonly from: number; readonly to: number }>;
  /** The card's lines as counts of its words (they add up to words.length). */
  readonly lines: ReadonlyArray<number>;
}

const num = (v: unknown, lo: number, hi: number, fallback: number) => (typeof v === "number" && Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : fallback);

/**
 * Characters per line when the item doesn't set them: what fits the text box at the look's size
 * with a conservative average glyph width (0.58 em for the sans looks, 0.62 for mono, 1 em for
 * full-width CJK characters weighted by their share of the transcript, wider in
 * capitals and heavy weights, plus tracking), less the box's padding and outline, at most
 * CAPTION_DEFAULTS.maxChars (a readable line), at least 4 (a single word longer than the line still breaks inside it). So a 4:5 or 9:16 canvas gets shorter
 * lines and a card never wraps past its lines.
 */
export function autoMaxChars(item: Pick<Item, "style" | "transform" | "words">, target: Pick<RenderTarget, "canvas" | "theme">): number {
  const style = { preset: CAPTION_DEFAULTS.look, ...(item.style ?? {}) } as NonNullable<Item["style"]>;
  const shell = { transform: item.transform, style };
  const place = textPlacement(shell, target);
  const look = textLook(style.preset);
  const width = place.w ?? target.canvas.width - place.x - safeMargin(target as RenderTarget);
  const decor = textDecor(shell, look, place.size);
  const inner = width - 2 * Math.max(0, decor.background?.padding ?? 0) - 2 * (decor.stroke?.width ?? 0);
  const font = String(style.font ?? look.font);
  const weight = Number(style.weight ?? look.weight);
  const latin = (/mono/i.test(font) ? 0.62 : 0.58) + ((style.uppercase ?? look.uppercase) ? 0.1 : 0) + (weight >= 700 ? 0.03 : 0);
  const wide = wideCharacterRatio((item.words??[]).map(word=>word.text));
  const glyph = latin+(1-latin)*wide+Math.max(0, Number(style.tracking ?? look.tracking) || 0);
  return Math.max(4, Math.min(CAPTION_DEFAULTS.maxChars, Math.floor(inner / (place.size * glyph))));
}

/** A captions item's grouping settings with the defaults filled in (characters per line from the canvas when given). */
export function captionSettings(item: Pick<Item, "lines" | "style" | "transform" | "words">, target?: Pick<RenderTarget, "canvas" | "theme">) {
  const l = (item.lines ?? {}) as CaptionLines;
  return {
    maxChars: Math.round(num(l.maxChars, 4, 200, target ? autoMaxChars(item, target) : CAPTION_DEFAULTS.maxChars)),
    maxLines: l.maxLines === 1 ? 1 : 2,
    maxSeconds: num(l.maxSeconds, 0.3, 30, CAPTION_DEFAULTS.maxSeconds),
    maxWords: typeof l.maxWords === "number" && l.maxWords >= 1 ? Math.round(l.maxWords) : Infinity,
    pause: num(l.pause, 0, 10, CAPTION_DEFAULTS.pause),
  };
}

export const captionMode = (item: Pick<Item, "mode">): CaptionMode => (item.mode === "word" || item.mode === "single" ? item.mode : "line");

/** The text a word shows: its correction, else the transcript's (trimmed); "" = hidden. */
export function captionWordText(item: Pick<Item, "words" | "edits">, index: number): string {
  const edit = item.edits?.findLast((e) => e.word === index);
  return (edit ? edit.text : (item.words?.[index]?.text ?? "")).trim();
}
