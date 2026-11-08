// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { trackAudible } from "@/app/lib/editor/core/audio/audio";
import { duckEntry } from "@/app/lib/editor/core/audio/ducking";
import { textLook } from "@/app/lib/editor/core/text/looks";
import { wordCount, textPlacement } from "@/app/lib/editor/core/visual/render";
import { layout } from "@/app/lib/editor/core/timeline/timeline";
import type { Op, Id, Project, Media, Item, Track } from "@/app/lib/editor/core/timeline/types";
import type { SoundProfile } from "@/app/lib/editor/core/checks/soundSheet";

export type CheckSeverity = "warn" | "info";

/** A suggested fix: ops for one project.edit / timeline_edit, and its History label. */
export interface CheckFix {
  readonly summary: string;
  readonly ops: ReadonlyArray<Op>;
}

/** One finding. */
export interface Check {
  /** The rule's stable id (what checks.disabled lists). */
  readonly id: string;
  /** Unique among the findings: the rule id plus what it's about. */
  readonly key: string;
  readonly severity: CheckSeverity;
  /** One plain sentence. */
  readonly message: string;
  readonly itemIds?: ReadonlyArray<Id>;
  /** Where to look (the playhead goes there). */
  readonly frame?: number;
  /** The stretch it's about, [start, end) in frames. */
  readonly range?: { readonly start: number; readonly end: number };
  readonly fix?: CheckFix;
  /** What to do when there's no fix (or as well as it). */
  readonly hint?: string;
}

/** What a rule gets: the project and things every rule would otherwise recompute. */
export interface CheckContext {
  readonly project: Project;
  readonly fps: number;
  readonly starts: ReadonlyMap<Id, number>;
  /** The video's length in frames (0 when nothing is on the timeline). */
  readonly end: number;
  /** Text items on shown text tracks, with their track and start, in start order. */
  readonly texts: ReadonlyArray<Placed>;
  /** Canvas px are judged at a 1080 px shorter side: this is the canvas's shorter side / 1080. */
  readonly scale: number;
  readonly media: ReadonlyMap<Id, Media>;
  /** Media known to carry speech (options.speechMedia, plus captions' sources). */
  readonly speech: ReadonlySet<Id>;
  /** trackAudible by track id, filled as rules ask. */
  readonly audible: Map<Id, boolean>;
  /** D217: how strict the sound rule is (the brief's video type and skills; strict when unknown). */
  readonly sound: SoundProfile;
}

export interface Placed {
  readonly item: Item;
  readonly track: Track;
  readonly start: number;
}

export interface CheckRule {
  readonly id: string;
  readonly title: string;
  readonly description: string;
  readonly severity: CheckSeverity;
  readonly check: (ctx: CheckContext) => ReadonlyArray<Omit<Check, "id" | "severity"> & { readonly severity?: CheckSeverity }>;
}

// ---- helpers ----

export const secs = (frames: number, fps: number) => `${(Math.round((frames / fps) * 10) / 10).toFixed(1)} s`;
/** A text's words for messages: markup stripped, one line, at most 32 characters. */
export const textLabel = (text: string | undefined) => {
  const plain = (text ?? "").replace(/\[\[(.*?)\]\]/gs, "$1").replace(/\*\*(.+?)\*\*/gs, "$1").replace(/`([^`]+)`/g, "$1").replace(/\s+/g, " ").trim();
  return plain.length > 32 ? `${plain.slice(0, 31).trimEnd()}…` : plain;
};
export const locked = (p: Placed) => p.track.locked === true || p.item.locked === true;
/** Frames a text is fully on screen: its length less its in and out moves. */
export const holdOf = (item: Item) => Math.max(0, item.duration - (item.animation?.in?.duration ?? 0) - (item.animation?.out?.duration ?? 0));
/** Frames a text needs to be read twice: words ÷ 3 s, at least 2 s. */
export const readFrames = (text: string | undefined, fps: number) => Math.ceil(Math.max(2, wordCount(text ?? "") / 3) * fps);
export const startOf = (ctx: CheckContext, item: Item) => ctx.starts.get(item.id) ?? 0;
/** Room after a free-track item before the next item on its track (Infinity when none). */
function roomAfter(ctx: CheckContext, p: Placed) {
  const end = p.start + p.item.duration;
  let next = Infinity;
  for (const other of p.track.items) {
    const s = startOf(ctx, other);
    if (other.id !== p.item.id && s >= end) next = Math.min(next, s);
  }
  return next - end;
}
/** A fix that makes a text `extra` frames longer, if its track has room and it doesn't make the video longer. */
export function lengthen(ctx: CheckContext, p: Placed, extra: number): CheckFix | undefined {
  if (locked(p) || p.track.role === "main" || extra <= 0) return undefined;
  if (roomAfter(ctx, p) < extra || p.start + p.item.duration + extra > ctx.end) return undefined;
  const set: Record<string, unknown> = { duration: p.item.duration + extra };
  return { summary: `Hold "${textLabel(p.item.text)}" for ${secs(holdOf(p.item) + extra, ctx.fps)}`, ops: [{ op: "update_item", item: p.item.id, set }] };
}

// Phrases that could sit under any product (VIDEO-BEST-PRACTICES 2b), matched as whole words, any case.
export const FILLER_PHRASES = [
  "seamlessly",
  "seamless",
  "effortlessly",
  "effortless",
  "unlock",
  "unlocks",
  "unleash",
  "the future of",
  "game-changer",
  "game changer",
  "revolutionize",
  "revolutionary",
  "supercharge",
  "next-level",
  "next level",
  "cutting-edge",
  "like never before",
  "elevate your",
  "empower",
  "whatever the",
  "in today's world",
] as const;
export const FILLER_RE = new RegExp(`(?<![\\p{L}\\p{N}])(${FILLER_PHRASES.map((p) => p.replace(/ /g, "\\s+")).join("|")})(?![\\p{L}\\p{N}])`, "giu");

// Dots between words: "Swift · Kotlin", "fast • simple", "one * two" (never **bold**). A dot at
// the start of a line is a bullet: the list keeps its lines and loses the bullets. A lone * is left
// alone in code (inside backticks, on a mono text, after import / export / select, or between
// numbers: "2 * 3").
const DOT = "[·•∙⋅]";
const BULLET_RE = new RegExp(`^([ \\t]*)${DOT}[ \\t]*`, "gm");
const DOT_RE = new RegExp(`(\\S)[ \\t]*${DOT}[ \\t]*(?=\\S)`, "g");
const STAR_RE = /(\S+)[ \t]+\*[ \t]+(?=(\S))/g;
const CODE_WORD = /^(import|export|select|from|count|return|where|and|or)$/i;

/** The text with its dots between words turned into commas and its bullets dropped, or undefined when it has none. */
export function fixDots(text: string, mono = false): string | undefined {
  // Code spans (`…`) are kept as they are.
  const parts = text.split(/(`[^`\n]*`)/);
  let lineStart = true;
  const out = parts.map((part, i) => {
    if (i % 2 === 1) {
      lineStart = false;
      return part;
    }
    let s = part;
    // Bullets only where a line really starts (not right after a code span).
    s = s.replace(BULLET_RE, (m, indent: string, offset: number) => (offset === 0 && !lineStart ? m : indent));
    s = s.replace(DOT_RE, "$1, ");
    if (!mono)
      s = s.replace(STAR_RE, (m, before: string, after: string) => {
        const word = before.replace(/^.*?([\p{L}\p{N}_]+)[^\p{L}\p{N}_]*$/u, "$1");
        if (/\d$/.test(before) && /^\d/.test(after)) return m;
        if (CODE_WORD.test(word) || /^[A-Z]{2,}$/.test(word)) return m;
        return `${before}, `;
      });
    if (part.length) lineStart = part.endsWith("\n");
    return s;
  });
  const fixed = out.join("");
  return fixed === text ? undefined : fixed;
}

/** Each line's trailing period (not an ellipsis), after markup that closes the line. */
const PERIOD_RE = /(?<!\.)\.((?:\]\]|\*\*|`)*)[ \t]*$/gm;
/** Abbreviations whose period stays ("Expo, RN, etc."). */
const ABBREVIATION = /(?:^|[^\p{L}])(etc|e\.g|i\.e|vs|inc|ltd|co|jr|sr|approx|corp)$/iu;
/** The text without its lines' trailing periods, or undefined when none has one. */
export function fixPeriods(text: string): string | undefined {
  const fixed = text.replace(PERIOD_RE, (m, close: string, offset: number) => (ABBREVIATION.test(text.slice(0, offset).split("\n").at(-1)!) ? m : close));
  return fixed === text ? undefined : fixed;
}

export const MUSIC_NAME = /\b(music|song|bed|lo-?fi|cozy|beat|soundtrack)\b/i;
const SPEECH_NAME = /\b(voice|vo|voiceover|voice-over|narration|narrator|speech|talk|talking|interview|dialog(ue)?|podcast)\b/i;
export const mediaOf = (ctx: CheckContext, item: Item) => (item.media ? ctx.media.get(item.media) : undefined);
export const isMusicTrack = (ctx: CheckContext, track: Track) =>
  track.kind === "audio" &&
  (!!duckEntry(track) ||
    MUSIC_NAME.test(track.name) ||
    track.items.some((i) => {
      const m = mediaOf(ctx, i);
      // The file's name, not the display name (G4: renaming a media item doesn't change what it is).
      return !!m && MUSIC_NAME.test(m.path.replace(/\\/g, "/").split("/").pop() ?? "");
    }));
/** soundMedia (audio.ts) through the context's media map. */
export function soundOf(ctx: CheckContext, item: Item) {
  if ((item.type !== "clip" && item.type !== "audio") || item.mute) return undefined;
  const m = mediaOf(ctx, item);
  return m && m.kind !== "image" && (m.kind === "audio" || m.probe?.hasAudio) ? m : undefined;
}
export const sounds = (ctx: CheckContext, track: Track) => track.items.some((i) => soundOf(ctx, i) !== undefined);
/** trackAudible (mute, solo), once per track. */
export function audible(ctx: CheckContext, track: Track) {
  let a = ctx.audible.get(track.id);
  if (a === undefined) {
    a = trackAudible(ctx.project, track);
    ctx.audible.set(track.id, a);
  }
  return a;
}
/**
 * A track that carries speech, for ducking: named like a voice (or holding media named like one),
 * or the main track when its sound is known to be speech (in `speechMedia`, else captions made from it).
 */
export function isSpeechTrack(ctx: CheckContext, track: Track) {
  if (isMusicTrack(ctx, track)) return false;
  if (SPEECH_NAME.test(track.name) || track.items.some((i) => SPEECH_NAME.test(mediaOf(ctx, i)?.name ?? ""))) return true;
  if (track.role === "main") return track.items.some((i) => !!i.media && ctx.speech.has(i.media));
  return false;
}
/** A fade made with volume keys (D83): the envelope starts (or ends) about silent. */
export const keyFade = (item: Item, side: "in" | "out") => {
  const keys = [...(item.gainKeys ?? [])].sort((a, b) => a.frame - b.frame);
  const k = side === "in" ? keys[0] : keys.at(-1);
  return !!k && k.db <= -40;
};

/**
 * The family a text draws with. The built-in looks use both the system sans ("sans") and the
 * bundled DM Sans (D77): with no theme font they count as one family, the app's own pair (D90).
 */
export function familyOf(project: Project, item: Item) {
  const token = item.style?.font ?? textLook(item.style?.preset).font;
  const themed = project.theme?.fonts?.[token];
  const name = themed || token;
  const mono = token === "mono" || /mono|code|courier|menlo|consol/i.test(name);
  const house = !themed && (token === "sans" || token === "DM Sans");
  return { key: house ? "sans" : name, name: house ? "DM Sans" : name, token, mono };
}

/** The look whose texts don't count as crowding (D96). */
export const FOOTNOTE_LOOK = "text.footnote";
/** A footnote for the crowding rule (D96): in the Footnote look and no bigger than the look's own size. */
export const isFootnote = (ctx: CheckContext, item: Item) => item.style?.preset === FOOTNOTE_LOOK && textPlacement(item, ctx.project).size <= textLook(FOOTNOTE_LOOK).size * ctx.project.canvas.height + 0.5;

/** What every rule gets (projectChecks, soundSheet). */
export function checkContext(project: Project, options: { readonly speechMedia?: Iterable<Id>; readonly sound?: SoundProfile } = {}): CheckContext {
  const { starts, duration } = layout(project);
  const end = project.tracks.some((t) => t.items.some((i) => i.type !== "captions")) ? duration : 0;
  const texts: Array<Placed> = [];
  const speech = new Set<Id>(options.speechMedia ?? []);
  for (const track of project.tracks) {
    for (const item of track.items) if (item.type === "captions" && item.source) speech.add(item.source);
    if (track.hidden || track.kind !== "text") continue;
    for (const item of track.items) if (item.type === "text") texts.push({ item, track, start: starts.get(item.id) ?? 0 });
  }
  texts.sort((a, b) => a.start - b.start);
  return { project, fps: project.canvas.fps, starts, end, texts, scale: Math.min(project.canvas.width, project.canvas.height) / 1080, media: new Map(project.media.map((m) => [m.id, m])), speech, audible: new Map(), sound: options.sound ?? "strict" };
}
