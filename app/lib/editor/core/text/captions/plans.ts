// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { TEXT_STYLES } from "@/app/lib/editor/core/text/looks";
import { joinWords } from '@/app/lib/editor/core/text/word-spacing';
import { remapCaptionFixes } from './remake';
import { lookItem } from "@/app/lib/editor/core/text/textPlan";
import { type CutTranscript, type SpeechEnvelope, soundParts, fillersOf, speechWords } from "@/app/lib/editor/core/text/transcript";
import { mainEnd, projectDuration } from "@/app/lib/editor/core/timeline/rules";
import type { Op, Project, FieldSet, Item } from "@/app/lib/editor/core/timeline/types";
import { type CaptionMode, type CaptionLines, CAPTION_DEFAULTS, CAPTION_MODES, type CaptionWord, captionMode, CAPTION_MODE_NAMES } from "@/app/lib/editor/core/text/captions/settings";
import { checkCaptionLines, captionCards } from "@/app/lib/editor/core/text/captions/cards";
import { captionStyleDef, CAPTION_STYLES, captionStyleFields } from "@/app/lib/editor/core/text/captions/styles";

// ---- planning (the Transcript panel's "Make captions", the agent's captions_make, the Inspector) ----

export interface MakeCaptionsRequest {
  readonly media: string;
  /** The media's transcript (whisper's words, seconds of the file). */
  readonly transcript: CutTranscript;
  /** The loudness analyzer's silences and speech envelope, when known: words are stored where they're heard (D74). */
  readonly silences?: ReadonlyArray<{ readonly start: number; readonly end: number }> | undefined;
  readonly envelope?: SpeechEnvelope | undefined;
  readonly mode?: CaptionMode;
  /** A text-style module id (default text.caption). */
  readonly look?: string;
  /** A caption preset, instead of look. Applies to new or remade captions; an explicit mode wins. */
  readonly captionStyle?: string;
  readonly lines?: CaptionLines;
  readonly hideFillers?: boolean;
  /** The new item's id (default captions-1, …). */
  readonly id?: string;
}

export type MakeCaptionsPlan = { readonly ops: Array<Op>; readonly itemId: string; readonly summary: string; readonly remade: boolean; readonly words: number; readonly fixesDropped: number } | { readonly problem: string };

function freeId(project: Pick<Project, "media" | "tracks">, base: string, taken: ReadonlySet<string>) {
  const ids = new Set([...taken, ...project.media.map((m) => m.id), ...project.tracks.flatMap((t) => [t.id, ...t.items.map((i) => i.id)])]);
  for (let n = 1; ; n++) if (!ids.has(`${base}-${n}`)) return `${base}-${n}`;
}

/** The main track's length in frames (what new captions cover). */
const mainLength = (project: Pick<Project, "tracks">) => mainEnd(project);

/**
 * Captions for a media from its transcript: a new captions item covering the main track on a new
 * "Captions" text track on top (its own track, so it never overlaps titles), with the look's size,
 * weight and position (lower safe area). If the media already has captions they're remade in place:
 * fresh words, the window refitted to the main track, mode / look / lines as asked, and the
 * corrections and breaks kept where words can be matched unambiguously. One edit.
 */
export function planMakeCaptions(project: Pick<Project, "canvas" | "theme" | "tracks" | "media">, req: MakeCaptionsRequest): MakeCaptionsPlan {
  const media = project.media.find((m) => m.id === req.media);
  if (!media) return { problem: `There's no media "${req.media}".` };
  if (media.kind === "image" || !(media.kind === "audio" || media.probe?.hasAudio)) return { problem: `${media.name} has no sound, so there's nothing to caption.` };
  // Heard through Main (a main clip of it, or one whose linked microphone it is: D215), else
  // wherever it plays: then the captions follow those items (anyTrack).
  const main = project.tracks.find((t) => t.role === "main");
  const onMain = !!main?.items.some((i) => i.type === "clip" && (i.media === media.id || soundParts(project, i).some((p) => p.media === media.id && !p.mute)));
  const anywhere = project.tracks.some((t) => t.items.some((i) => (i.type === "clip" || i.type === "audio") && i.media === media.id));
  if (!onMain && !anywhere) return { problem: `${media.name} isn't on the timeline: captions follow the items that play it.` };
  const anyTrack = !onMain;
  const look = req.look ?? CAPTION_DEFAULTS.look;
  if (!TEXT_STYLES[look]) return { problem: `There's no text look called ${look}. modules_list with kind "text-style" lists them.` };
  if (req.look !== undefined && req.captionStyle !== undefined) return { problem: 'Choose either a text look or a caption style.' };
  const styled = req.captionStyle !== undefined ? captionStyleDef(req.captionStyle) : undefined;
  if (req.captionStyle !== undefined && !styled) return { problem: `There's no caption style "${req.captionStyle}". Styles: ${CAPTION_STYLES.map((c) => c.id).join(", ")}.` };
  if (req.mode !== undefined && !CAPTION_MODES.includes(req.mode)) return { problem: `mode must be one of ${CAPTION_MODES.join(", ")}.` };
  const t = req.transcript;
  if (!t.words.length) return { problem: `${media.name}'s transcript has no words.` };
  const hes = new Set(fillersOf(t.words, t.language).filter((f) => f.kind === "hesitation").flatMap((f) => Array.from({ length: f.n }, (_, k) => f.i + k)));
  const heard = speechWords(t.words, { segments: t.segments, silences: req.silences, seconds: t.seconds, envelope: req.envelope });
  const words: Array<CaptionWord> = heard.map((w, i) => ({ text: w.text.trim(), start: w.start, end: w.end, ...(hes.has(i) ? { filler: true } : {}) }));
  const duration = Math.max(1, anyTrack ? projectDuration(project) : mainLength(project));
  let lines: CaptionLines | undefined;
  if (req.lines !== undefined) {
    const checked = checkCaptionLines(req.lines);
    if ("problem" in checked) return checked;
    lines = checked.lines as CaptionLines;
  }
  const existing = project.tracks.flatMap((tr) => tr.items.map((i) => ({ item: i, track: tr }))).find((x) => x.item.type === "captions" && x.item.source === media.id);
  if (existing) {
    if (existing.track.locked || existing.item.locked) return { problem: `${media.name}'s captions are locked: unlock them to remake them.` };
    const fixes=remapCaptionFixes(existing.item,words);
    const set: Record<string, unknown> = { words, start: 0, duration, coversMain: true, ...(anyTrack ? { anyTrack: true } : existing.item.anyTrack ? { anyTrack: null } : {}) };
    const mode = req.mode ?? styled?.mode;
    if (mode !== undefined) set.mode = mode;
    if (styled) set.style = captionStyleFields(styled.id, project.canvas);
    else if (req.look !== undefined) {
      const fields = lookItem(TEXT_STYLES[look]!, project, duration);
      set.style = fields.style;
    }
    if (lines) set.lines = { ...(existing.item.lines ?? {}), ...lines };
    if (req.hideFillers !== undefined) set.hideFillers = req.hideFillers || null;
    const fixesDropped=fixes.dropped;
    if(existing.item.edits)set.edits=fixes.edits.length?fixes.edits:null;
    if(existing.item.breaks)set.breaks=fixes.breaks.length?fixes.breaks:null;
    const summary = fixesDropped ? `Remake captions (${fixesDropped} fix${fixesDropped === 1 ? "" : "es"} dropped)` : "Remake captions";
    return { ops: [{ op: "update_item", item: existing.item.id, set: set as FieldSet }], itemId: existing.item.id, summary, remade: true, words: words.length, fixesDropped };
  }
  const taken = new Set<string>();
  if (req.id !== undefined) {
    if (project.media.some((m) => m.id === req.id) || project.tracks.some((tr) => tr.id === req.id || tr.items.some((i) => i.id === req.id))) return { problem: `The id "${req.id}" is already used in this project.` };
    taken.add(req.id);
  }
  const trackId = freeId(project, "captions-track", taken);
  taken.add(trackId);
  const itemId = req.id ?? freeId(project, "captions", taken);
  const fields = lookItem(TEXT_STYLES[styled ? CAPTION_DEFAULTS.look : look]!, project, duration);
  const item: Item = {
    id: itemId,
    type: "captions",
    source: media.id,
    start: 0,
    duration,
    coversMain: true,
    ...(anyTrack ? { anyTrack: true } : {}),
    mode: req.mode ?? styled?.mode ?? CAPTION_DEFAULTS.mode,
    transform: fields.transform,
    style: (styled ? captionStyleFields(styled.id, project.canvas) : fields.style) as Item["style"],
    ...(lines && Object.keys(lines).length ? { lines } : {}),
    ...(req.hideFillers ? { hideFillers: true } : {}),
    words,
  };
  // An empty, unlocked captions track (its captions were deleted) is used again.
  const empty = project.tracks.find((tr) => tr.kind === "text" && tr.purpose === 'subtitles' && !tr.items.length && !tr.locked && !tr.hidden);
  const textTracks = project.tracks.filter((tr) => tr.kind === "text" && tr.purpose === 'subtitles').length;
  const ops: Array<Op> = empty
    ? [{ op: "insert_item", track: empty.id, item }]
    : [
        { op: "add_track", index: 0, track: { id: trackId, kind: "text", purpose: 'subtitles', name: textTracks ? `Captions ${textTracks + 1}` : "Captions" } } as Op,
        { op: "insert_item", track: trackId, item },
      ];
  return { ops, itemId, summary: "Make captions", remade: false, words: words.length, fixesDropped: 0 };
}

export interface CaptionEdit {
  /** Word corrections by index into the snapshot: text ("" hides the word), or null to go back to the transcript's. */
  readonly words?: ReadonlyArray<{ readonly index: number; readonly text: string | null }>;
  /** Start a new card at these words (true) or stop forcing one there (false). */
  readonly breaks?: ReadonlyArray<{ readonly index: number; readonly on: boolean }>;
  readonly mode?: CaptionMode;
  /** Grouping settings over the item's (null: back to the defaults). */
  readonly lines?: { readonly [K in keyof CaptionLines]?: CaptionLines[K] | null };
  readonly hideFillers?: boolean;
  /** Fit the window to the whole main track again. */
  readonly cover?: boolean;
}

const quote = (s: string) => `"${s.length > 30 ? `${s.slice(0, 29)}…` : s}"`;

/**
 * One captions edit as update_item (edits and breaks sorted, a correction equal to the
 * transcript's text dropped, empty lists removed), with the summary the History shows.
 */
export function planCaptionEdit(project: Pick<Project, "tracks">, itemId: string, edit: CaptionEdit): { readonly ops: Array<Op>; readonly summary: string } | { readonly problem: string } {
  const item = project.tracks.flatMap((t) => t.items).find((i) => i.id === itemId);
  if (!item || item.type !== "captions") return { problem: `"${itemId}" isn't a captions item.` };
  const words = item.words ?? [];
  const set: Record<string, unknown> = {};
  const labels: Array<string> = [];
  if (edit.words?.length) {
    const edits = new Map((item.edits ?? []).map((e) => [e.word, e.text]));
    for (const w of edit.words) {
      if (!Number.isInteger(w.index) || w.index < 0 || w.index >= words.length) return { problem: `There's no word ${w.index} in these captions (they have ${words.length}: w0 to w${Math.max(0, words.length - 1)}).` };
      const text = w.text === null ? null : w.text.replace(/\s+/g, " ").trim();
      if (text === null || text === words[w.index]!.text.trim()) edits.delete(w.index);
      else edits.set(w.index, text);
    }
    const list = [...edits].sort((a, b) => a[0] - b[0]).map(([word, text]) => ({ word, text }));
    const before = JSON.stringify(item.edits ?? []);
    if (JSON.stringify(list) !== before) {
      set.edits = list.length ? list : null;
      const one = edit.words.length === 1 ? edit.words[0]! : undefined;
      const restores = (w: { index: number; text: string | null }) => w.text === null || w.text.replace(/\s+/g, " ").trim() === words[w.index]!.text.trim();
      labels.push(
        one
          ? one.text === "" || one.text?.trim() === ""
            ? `Hide caption word ${quote(words[one.index]!.text.trim())}`
            : restores(one)
              ? `Restore caption word ${quote(words[one.index]!.text.trim())}`
              : `Fix caption word ${quote(one.text!.trim())}`
          : edit.words.every(restores)
            ? `Restore ${edit.words.length} caption words`
            : `Fix ${edit.words.length} caption words`,
      );
    }
  }
  if (edit.breaks?.length) {
    const breaks = new Set(item.breaks ?? []);
    for (const b of edit.breaks) {
      if (!Number.isInteger(b.index) || b.index < 0 || b.index >= words.length) return { problem: `There's no word ${b.index} in these captions.` };
      if (b.on) breaks.add(b.index);
      else breaks.delete(b.index);
    }
    const list = [...breaks].sort((a, b) => a - b);
    if (JSON.stringify(list) !== JSON.stringify(item.breaks ?? [])) {
      set.breaks = list.length ? list : null;
      labels.push(edit.breaks.length === 1 ? (edit.breaks[0]!.on ? "Start a caption card" : "Join caption cards") : "Change caption cards");
    }
  }
  if (edit.mode !== undefined) {
    if (!CAPTION_MODES.includes(edit.mode)) return { problem: `mode must be one of ${CAPTION_MODES.join(", ")}.` };
    if (edit.mode !== captionMode(item)) {
      set.mode = edit.mode;
      labels.push(`Captions ${CAPTION_MODE_NAMES[edit.mode].toLowerCase()}`);
    }
  }
  if (edit.lines) {
    const checked = checkCaptionLines(edit.lines, true);
    if ("problem" in checked) return checked;
    const next: Record<string, unknown> = { ...(item.lines ?? {}) };
    for (const [k, v] of Object.entries(checked.lines)) {
      if (v === null) delete next[k];
      else next[k] = v;
    }
    if (JSON.stringify(next) !== JSON.stringify(item.lines ?? {})) {
      set.lines = Object.keys(next).length ? next : null;
      labels.push("Change caption lines");
    }
  }
  if (edit.hideFillers !== undefined && edit.hideFillers !== (item.hideFillers === true)) {
    set.hideFillers = edit.hideFillers || null;
    labels.push(edit.hideFillers ? "Hide fillers in captions" : "Show fillers in captions");
  }
  if (edit.cover && !item.coversMain) {
    set.start = 0;
    set.duration = Math.max(1, mainLength(project));
    set.coversMain = true;
    labels.push("Fit captions to the edit");
  }
  if (!labels.length) return { ops: [], summary: "" };
  return { ops: [{ op: "update_item", item: itemId, set: set as FieldSet }], summary: labels.length === 1 ? labels[0]! : "Edit captions" };
}

/** Whether a captions item's window covers the whole main track (else the Inspector offers to fit it). */
export const captionsCoverMain = (project: Pick<Project, "tracks">, item: Pick<Item, "start" | "duration" | "coversMain">) => item.coversMain === true || ((item.start ?? 0) <= 0 && (item.start ?? 0) + item.duration >= mainLength(project));

/** For the agent and the Inspector: the cards as text with their word ids and timeline frames. */
export function captionCardList(project: Pick<Project, "tracks" | "canvas" | "media">, item: Item): Array<{ readonly from: number; readonly to: number; readonly words: string; readonly text: string }> {
  return captionCards(project, item).map((c) => {
    let i = 0;
    const lines = c.lines.map((n) => {
      const line = joinWords(c.words.slice(i, i + n).map((w) => w.text));
      i += n;
      return line;
    });
    const a = c.words[0]!.index;
    const b = c.words.at(-1)!.index;
    return { from: c.from, to: c.to, words: a === b ? `w${a}` : `w${a}–w${b}`, text: lines.join(" / ") };
  });
}
