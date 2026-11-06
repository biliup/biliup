// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Subtitle files (D105), browser-safe. Writing: the cues are the cards the captions draw (core
// captionCards: the same grouping, hold, bridge and window; hidden words and fillers left out,
// corrections applied; word and single modes export their cards as line cards) plus the text items
// of "Subtitles" tracks (an imported .srt / .vtt), in timeline (post-cut) time at the project's fps,
// merged by time into one list (one file, one soft track). Reading: a .srt or .vtt becomes plain
// text items on a new "Subtitles" text track at the cues' timeline frames (planImportSubtitles), so
// D82's captions model (a media's transcript mapped through the main track) is left as it is.
import { TEXT_STYLES } from "@/app/lib/editor/core/text/looks";
import { joinWords } from '@/app/lib/editor/core/text/word-spacing';
import { lookItem } from "@/app/lib/editor/core/text/textPlan";
import { captionBox, captionCards, CAPTION_DEFAULTS } from "@/app/lib/editor/core/text/captions";
import { layout } from "@/app/lib/editor/core/timeline/timeline";
import { mainEnd } from "@/app/lib/editor/core/timeline/rules";
import type { Item, Op, Project, Track } from "@/app/lib/editor/core/timeline/types";

export type SubtitleFormat = "srt" | "vtt";
export const SUBTITLE_FORMATS: ReadonlyArray<SubtitleFormat> = ["srt", "vtt"];

/** A cue: timeline frames [from, to) and its text (lines separated by \n, no markup). */
export interface SubtitleCue {
  readonly from: number;
  readonly to: number;
  readonly text: string;
}

/** A cue read from a file: seconds and text. */
export interface ParsedCue {
  readonly start: number;
  readonly end: number;
  readonly text: string;
}

type SubtitleProblem = { readonly problem: string; readonly values?: Readonly<Record<string, string | number>> };

/** A text track that holds subtitles (made by an import): its plain text items are cues. */
export const isSubtitleTrack = (track: Pick<Track, "kind" | "purpose">) => track.kind === "text" && track.purpose === "subtitles";

/** A subtitles-track text item the export may draw as part of one layer: no motion, keys or opacity of its own. */
export const isPlainCue = (item: Item) =>
  item.type === "text" && typeof item.text === "string" && !item.animation && !item.keys?.length && !item.effects?.length && (item.transform?.opacity === undefined || item.transform.opacity === 1);

/** Text markup (**bold**, [[accent]], `mono`) as plain text. */
const unmarkup = (text: string) => text.replace(/\*\*(.+?)\*\*/gs, "$1").replace(/\[\[(.+?)\]\]/gs, "$1").replace(/`([^`]*)`/g, "$1").replace(/​/g, "");

/**
 * Every cue the project's subtitles hold, in timeline order: the cards of each captions item and
 * the text items of each Subtitles track, on tracks that aren't hidden, clipped to the edit's length.
 */
export function subtitleCues(project: Project): Array<SubtitleCue> {
  const { starts, duration } = layout(project);
  const out: Array<SubtitleCue> = [];
  const push = (from: number, to: number, text: string) => {
    const a = Math.max(0, from);
    const b = Math.min(duration, to);
    const t = text.split("\n").map((l) => l.replace(/\s+/g, " ").trim()).filter(Boolean).join("\n");
    if (b > a && t) out.push({ from: a, to: b, text: t });
  };
  for (const track of project.tracks) {
    if (track.hidden) continue;
    for (const item of track.items) {
      if (item.type === "captions") {
        for (const card of captionCards(project, item)) {
          let i = 0;
          const lines = card.lines.map((n) => {
            const line = joinWords(card.words.slice(i, i + n).map((w) => w.text));
            i += n;
            return line;
          });
          push(card.from, card.to, lines.join("\n"));
        }
      } else if (isSubtitleTrack(track) && item.type === "text" && typeof item.text === "string") {
        const start = starts.get(item.id) ?? item.start ?? 0;
        push(start, start + item.duration, unmarkup(item.text));
      }
    }
  }
  // Stable: cues starting together keep their track order.
  return out.map((c, k) => ({ c, k })).sort((a, b) => a.c.from - b.c.from || a.k - b.k).map((x) => x.c);
}

/**
 * Cues that never overlap: wherever cues overlap, one cue with their texts stacked (in the list's
 * order), split at every start and end, touching pieces with the same text joined. An mp4's soft
 * subtitle track (mov_text) shows one sample at a time, so overlapping cues would cut each other off.
 */
export function flattenCues(cues: ReadonlyArray<SubtitleCue>): Array<SubtitleCue> {
  return [...flattenedCues(cues)];
}

/** Stream completed intervals so imports can bound expanded overlapping text before allocating it all. */
function* flattenedCues(cues: ReadonlyArray<SubtitleCue>): Generator<SubtitleCue> {
  const edges=cues.flatMap((cue,index)=>cue.to>cue.from?[
    {frame:cue.from,index,enter:true},{frame:cue.to,index,enter:false},
  ]:[]).sort((a,b)=>a.frame-b.frame);
  // A long recording can have thousands of cues. Visit only boundaries and active cues,
  // keeping the original track/cue order even when the input isn't ordered by start time.
  const active:number[]=[];
  const position=(index:number)=>{
    let lo=0,hi=active.length;
    while(lo<hi){const middle=(lo+hi)>>>1;if(active[middle]!<index)lo=middle+1;else hi=middle}
    return lo;
  };
  let last: SubtitleCue | undefined;
  for(let i=0;i<edges.length;){
    const from=edges[i]!.frame;
    do{
      const edge=edges[i]!,at=position(edge.index);
      if(edge.enter)active.splice(at,0,edge.index);
      else if(active[at]===edge.index)active.splice(at,1);
      i++;
    }while(i<edges.length&&edges[i]!.frame===from);
    if(i===edges.length)break;
    const to=edges[i]!.frame;
    const text=active.map(index=>cues[index]!.text).join("\n");
    if (!text) continue;
    if (last && last.to === from && last.text === text) last = { ...last, to };
    else {
      if (last) yield last;
      last = { from, to, text };
    }
  }
  if (last) yield last;
}

/** Frames as a file's clock: HH:MM:SS,mmm (SRT) or HH:MM:SS.mmm (VTT), to the nearest millisecond. */
export function subtitleTime(frames: number, fps: number, format: SubtitleFormat): string {
  const ms = Math.max(0, Math.round((frames * 1000) / fps));
  const pad = (n: number, w = 2) => String(n).padStart(w, "0");
  const h = Math.floor(ms / 3_600_000);
  const m = Math.floor(ms / 60_000) % 60;
  const s = Math.floor(ms / 1000) % 60;
  return `${pad(h)}:${pad(m)}:${pad(s)}${format === "srt" ? "," : "."}${pad(ms % 1000, 3)}`;
}

/** A cue's text made safe for the format: no blank lines (they end a cue), no "-->", and in VTT & < > as entities. */
function cueText(text: string, format: SubtitleFormat): string {
  const lines = text
    .split("\n")
    .map((l) => l.trim().replace(/-->/g, "->"))
    .filter(Boolean);
  const body = lines.join("\n");
  return format === "vtt" ? body.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;") : body;
}

/** The cues as a subtitle file (SRT numbered from 1; VTT with its WEBVTT header). Ends with a newline. */
export function writeSubtitles(cues: ReadonlyArray<SubtitleCue>, fps: number, format: SubtitleFormat): string {
  const blocks = cues.map((c, i) => {
    const time = `${subtitleTime(c.from, fps, format)} --> ${subtitleTime(c.to, fps, format)}`;
    return format === "srt" ? `${i + 1}\n${time}\n${cueText(c.text, "srt")}` : `${time}\n${cueText(c.text, "vtt")}`;
  });
  return format === "srt" ? (blocks.length ? `${blocks.join("\n\n")}\n` : "") : `WEBVTT\n\n${blocks.map((b) => `${b}\n\n`).join("")}`;
}
export const writeSrt = (cues: ReadonlyArray<SubtitleCue>, fps: number) => writeSubtitles(cues, fps, "srt");
export const writeVtt = (cues: ReadonlyArray<SubtitleCue>, fps: number) => writeSubtitles(cues, fps, "vtt");

/** The format of a file name by its extension (undefined: neither). */
export const subtitleFormatOf = (name: string): SubtitleFormat | undefined => {
  const ext = /\.([^./\\]+)$/.exec(name)?.[1]?.toLowerCase();
  return ext === "srt" || ext === "vtt" ? ext : undefined;
};

// ---- languages (the soft track's tag) ----

/** Languages the export dialog offers for a soft subtitle track: ISO 639-1 code, the mp4's ISO 639-2 tag, the name. */
export const SUBTITLE_LANGUAGES: ReadonlyArray<{ readonly code: string; readonly tag: string; readonly name: string }> = [
  { code: "en", tag: "eng", name: "English" },
  { code: "pl", tag: "pol", name: "Polish" },
  { code: "de", tag: "ger", name: "German" },
  { code: "fr", tag: "fre", name: "French" },
  { code: "es", tag: "spa", name: "Spanish" },
  { code: "it", tag: "ita", name: "Italian" },
  { code: "pt", tag: "por", name: "Portuguese" },
  { code: "nl", tag: "dut", name: "Dutch" },
  { code: "sv", tag: "swe", name: "Swedish" },
  { code: "no", tag: "nor", name: "Norwegian" },
  { code: "da", tag: "dan", name: "Danish" },
  { code: "fi", tag: "fin", name: "Finnish" },
  { code: "cs", tag: "cze", name: "Czech" },
  { code: "uk", tag: "ukr", name: "Ukrainian" },
  { code: "ru", tag: "rus", name: "Russian" },
  { code: "tr", tag: "tur", name: "Turkish" },
  { code: "ja", tag: "jpn", name: "Japanese" },
  { code: "ko", tag: "kor", name: "Korean" },
  { code: "zh", tag: "chi", name: "Chinese" },
  { code: "hi", tag: "hin", name: "Hindi" },
  { code: "ar", tag: "ara", name: "Arabic" },
];

/** A language code (en, eng, en-US) as the mp4's three-letter tag; "und" when it isn't known. */
export function subtitleLanguageTag(code: string | undefined): string {
  const c = (code ?? "").trim().toLowerCase().split(/[-_]/)[0] ?? "";
  const hit = SUBTITLE_LANGUAGES.find((l) => l.code === c || l.tag === c);
  if (hit) return hit.tag;
  return /^[a-z]{3}$/.test(c) ? c : "und";
}

// ---- reading .srt / .vtt ----

/** At most this many cues in a file. */
export const MAX_SUBTITLE_CUES = 20000;

/** A subtitle file's bytes as text: UTF-8 (with or without a BOM), UTF-16 with a BOM, else Windows-1252 (older .srt files). */
export function decodeSubtitleBytes(bytes: Uint8Array): string {
  if (bytes[0] === 0xff && bytes[1] === 0xfe) return new TextDecoder("utf-16le").decode(bytes.subarray(2));
  if (bytes[0] === 0xfe && bytes[1] === 0xff) return new TextDecoder("utf-16be").decode(bytes.subarray(2));
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return new TextDecoder("windows-1252").decode(bytes);
  }
}

const TIME = /^(?:(\d+):)?(\d{1,2}):(\d{1,2})(?:[,.](\d{1,3}))?$/;
const seconds = (t: string): number | undefined => {
  const m = TIME.exec(t.trim());
  if (!m) return undefined;
  const [, h, mm, ss, frac] = m;
  if (Number(mm) > 59 && h !== undefined) return undefined;
  if (Number(ss) > 59) return undefined;
  const value = Number(h ?? 0) * 3600 + Number(mm) * 60 + Number(ss) + (frac ? Number(frac.padEnd(3, "0")) / 1000 : 0);
  return Number.isFinite(value) ? value : undefined;
};
const ENTITIES: Readonly<Record<string, string>> = { amp: "&", lt: "<", gt: ">", nbsp: " ", quot: '"', apos: "'", lrm: "", rlm: "" };
/** A cue's text as plain lines: tags (<i>, <c.x>, <v Name>, <00:01.000>) and {\an8}-style overrides dropped, entities decoded. */
const plainCue = (lines: ReadonlyArray<string>) =>
  lines
    .join("\n")
    .replace(/<br\s*\/?>/gi, "\n")
    .split("\n")
    .map((l) =>
      l
        .replace(/<[^>]*>/g, "")
        .replace(/\{\\[^}]*\}/g, "")
        .replace(/&(#(?:x[0-9a-f]+|[0-9]+)|[a-z]+);/gi, (all, e: string) => {
          if (e[0] === "#") {
            const n = e[1] === "x" || e[1] === "X" ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10);
            return Number.isFinite(n) && n > 0 && n < 0x110000 && !(n >= 0xd800 && n <= 0xdfff) ? String.fromCodePoint(n) : "";
          }
          return ENTITIES[e.toLowerCase()] ?? all;
        })
        .replace(/\s+/g, " ")
        .trim(),
    )
    .filter(Boolean)
    .join("\n");

/**
 * The cues of a .srt or .vtt file's text, in time order, or why it can't be read (a sentence).
 * `name` (the file name) picks the format by its extension; text starting with WEBVTT is read as
 * VTT whatever the name. Blocks without a timing line (VTT's NOTE, STYLE, REGION, a stray number)
 * are skipped; a timing line Biliup can't read refuses the file with its line number; cues with no
 * text or that end before they start are left out.
 */
export function parseSubtitles(text: string, name = "subtitles.srt"): { readonly cues: Array<ParsedCue>; readonly format: SubtitleFormat; readonly skipped: number } | SubtitleProblem {
  const body = text.replace(/^﻿/, "").replace(/\r\n?/g, "\n");
  const ext = subtitleFormatOf(name);
  const vtt = /^WEBVTT(?:[ \t].*)?(?:\n|$)/.test(body);
  if (ext === "vtt" && !vtt) return { problem: '{0} must start with a WEBVTT header.', values: {0:name} };
  if (!body.trim()) return { problem: '{0} is empty.', values: {0:name} };
  const format: SubtitleFormat = vtt ? "vtt" : "srt";
  const lines = body.split("\n");
  const cues: Array<ParsedCue> = [];
  let skipped = 0;
  let i = vtt ? 1 : 0;
  // VTT: the header block (WEBVTT and any lines after it up to a blank line) isn't a cue.
  if (vtt) while (i < lines.length && lines[i]!.trim() !== "") i++;
  while (i < lines.length) {
    while (i < lines.length && lines[i]!.trim() === "") i++;
    const block: Array<{ line: string; n: number }> = [];
    while (i < lines.length && lines[i]!.trim() !== "") block.push({ line: lines[i]!, n: ++i });
    if (!block.length) break;
    if (vtt && /^(?:NOTE(?:[ \t]|$)|STYLE[ \t]*$|REGION[ \t]*$)/.test(block[0]!.line)) continue;
    const timing = block.findIndex((b) => b.line.includes("-->"));
    if (timing < 0 || timing > 1) {
      if (timing > 1) return { problem: 'Invalid subtitle at line {0}: the time must follow the cue identifier.', values: {0:block[timing]!.n} };
      continue;
    }
    const t = block[timing]!;
    const [left, rightAll] = t.line.split("-->");
    const right = (rightAll ?? "").trim().split(/\s+/)[0] ?? "";
    const start = seconds(left ?? "");
    const end = seconds(right);
    if (start === undefined || end === undefined) return { problem: 'Invalid subtitle time at line {0}: {1}', values: {0:t.n,1:t.line.trim().slice(0,60)} };
    const cueTextLines = block.slice(timing + 1).map((b) => b.line);
    const content = plainCue(cueTextLines);
    if (!content || end <= start) {
      skipped++;
      continue;
    }
    cues.push({ start, end, text: content });
    if (cues.length > MAX_SUBTITLE_CUES) return { problem: 'A subtitle file can contain up to {0} cues.', values: {0:MAX_SUBTITLE_CUES} };
  }
  if (!cues.length) return { problem: skipped ? 'No subtitle cues have text and a valid time range.' : 'No SRT or WebVTT subtitle cues were found.' };
  cues.sort((a, b) => a.start - b.start);
  return { cues, format, skipped };
}

// ---- importing ----

export type ImportSubtitlesPlan =
  | { readonly ops: Array<Op>; readonly trackId: string; readonly cues: number; readonly clipped: number; readonly skipped: number; readonly summary: string }
  | SubtitleProblem;

/** Text made safe for textHtml's markup (as captions do), keeping line breaks. */
const markupSafe = (text: string) => text.replace(/`/g, "'").replace(/\*\*+/g, "*").replace(/\[\[+/g, "[").replace(/\]\]+/g, "]");

/**
 * Subtitles read from a file as plain text items on a new "Subtitles" text track on top (one item
 * per cue, at its timeline frames plus the import offset). Cues past the end of the main
 * track are left out and one running past it is shortened (when the main track has clips; text
 * items would otherwise make the video longer). Overlaps use the same interval flattening as a
 * soft subtitle track: stack the active texts and retain each cue's start and end. Each
 * sits with its last line where a caption's last line sits (the text.caption look). One edit.
 */
export function planImportSubtitles(project: Pick<Project, "canvas" | "theme" | "tracks" | "media">, cues: ReadonlyArray<ParsedCue>, name: string, offset = 0): ImportSubtitlesPlan {
  if(!Number.isSafeInteger(offset)||offset<0)return {problem:'Subtitle placement must be a non-negative timeline frame.'};
  const fps = project.canvas.fps;
  const mainLength = mainEnd(project);
  const limit = mainLength > 0 ? mainLength : Infinity;
  let clipped = 0;
  let skipped = 0;
  const timed: Array<SubtitleCue> = [];
  for (const c of [...cues].sort((a, b) => a.start - b.start)) {
    const from = offset + Math.round(c.start * fps);
    let to = Math.max(from + 1, offset + Math.round(c.end * fps));
    if(!Number.isSafeInteger(from)||!Number.isSafeInteger(to))return {problem:'Subtitle placement exceeds the timeline range.'};
    if (from >= limit) {
      skipped++;
      continue;
    }
    if (to > limit) { to = limit; clipped++; }
    const text = markupSafe(c.text);
    timed.push({ from, to, text });
  }
  if (!timed.length) return { problem: 'Every subtitle cue starts after the end of the edit ({0} s).', values: {0:(mainLength / fps).toFixed(1)} };
  const frames: Array<SubtitleCue> = [];
  const encoder = new TextEncoder();
  let bytes = 0;
  for (const cue of flattenedCues(timed)) {
    bytes += encoder.encode(cue.text).byteLength;
    if (bytes > 2 * 1024 * 1024) return {problem:'Overlapping subtitles produce too much text. Import a smaller section.'};
    frames.push(cue);
  }
  const ids = new Set([...project.media.map((m) => m.id), ...project.tracks.flatMap((t) => [t.id, ...t.items.map((i) => i.id)])]);
  const nextIds = new Map<string, number>();
  const fresh = (base: string) => {
    for (let n = nextIds.get(base) ?? 1; ; n++) {
      const id = `${base}-${n}`;
      if (!ids.has(id)) {
        ids.add(id);
        nextIds.set(base, n + 1);
        return id;
      }
    }
  };
  const trackId = fresh("subtitles-track");
  const existing = project.tracks.filter((t) => isSubtitleTrack(t)).length;
  const look = lookItem(TEXT_STYLES[CAPTION_DEFAULTS.look]!, project, 1);
  // Where a caption card's bottom sits, and one line's height: a cue of n lines ends on that bottom.
  const two = captionBox(project, { style: look.style as Item["style"], transform: look.transform });
  const one = captionBox(project, { style: look.style as Item["style"], transform: look.transform, lines: { maxLines: 1 } });
  const perLine = two.height - one.height;
  const pad = one.height - perLine;
  const ops: Array<Op> = [{ op: "add_track", index: 0, track: { id: trackId, kind: "text", purpose: "subtitles", name: existing ? `Subtitles ${existing + 1}` : "Subtitles" } } as Op];
  for (const f of frames) {
    const n = f.text.split("\n").length;
    const y = Math.round(two.bottom - n * perLine - pad);
    const item: Item = { id: fresh("sub"), type: "text", start: f.from, duration: f.to - f.from, text: f.text, style: look.style as Item["style"], transform: { ...look.transform, y } };
    ops.push({ op: "insert_item", track: trackId, item });
  }
  return { ops, trackId, cues: frames.length, clipped, skipped, summary: `Import subtitles ${name}` };
}
