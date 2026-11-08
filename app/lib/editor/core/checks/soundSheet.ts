// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { duckEntry } from "@/app/lib/editor/core/audio/ducking";
import { bundledSfxOf } from "@/app/lib/editor/core/audio/library";
import type { Media, Item, Track, Project, Id } from "@/app/lib/editor/core/timeline/types";
import { type CheckContext, audible, isSpeechTrack, isMusicTrack, soundOf, MUSIC_NAME, startOf, textLabel, checkContext, secs } from "@/app/lib/editor/core/checks/context";

// ---- sound design (D207, issue #31) ----
//
// What the evidence showed (argent-cloud-2: 16 sound effects in 21.5 s from 7 files, a UI click 7
// times, also as "typing" and on every text line, two booms, a slow lo-fi loop under a punchy cut)
// as objective rules over the timeline alone: how many effects play in any 10 s, the same file over
// and over, big hits, effects at the music's level or above, effects that land on nothing that
// changes on screen, clicks on text, meme sounds, and a slow track under fast cuts. The sound sheet
// is the same analysis as lines, for the critic, which can't hear (it reads the sheet blindfold).

/** At most this many sound effects start in any 10 s (fewer in a shorter video, pro rata, at least 2). */
export const SFX_PER_10S = 4;
/** The same effect file this many times or more reads as a loop. */
export const SFX_REPEAT_MAX = 3;
/** Big hits (impacts, booms): at most one in any 15 s. */
export const SFX_HITS_PER_15S = 1;
/** An effect sits at least this many dB under the music it plays over (−6 to −12 dB). */
export const SFX_UNDER_MUSIC_DB = 6;
/** Where an effect's fix puts it: this many dB under the music. */
export const SFX_FIX_UNDER_DB = 9;
/** A visual event this close (s) before an effect's start, or this close after, counts as its hit. */
export const SFX_SYNC_BEFORE_S = 0.1;
export const SFX_SYNC_AFTER_S = 0.2;
/** Under this tempo a track is slow (bpm), and cuts closer than this (s) on average are fast. */
export const SLOW_BPM = 95;
export const FAST_CUT_S = 2;

/**
 * D217 (issue #44): how strict the sound rule is, from the brief's video type and skills
 * (core skills soundProfileFor). strict: a promo or product video, or one that asks for subtle sound:
 * the numbers above, and density and level fail a critic round; standard: the same numbers as
 * findings; loose: a meme, gaming, social or music edit that may want dense, loud effects (much looser
 * numbers, still findings; no click or meme notes); ui (D297): a motion film, one soft effect per real
 * cursor press, drag or morph, on every beat if the picture changes on every beat, the same click
 * as often as it's pressed, under the music, never a blocker; music: the user asked for the music
 * only (T2, A2r8: "Music only, no added sound effects"), so the sound is the music, judged on its
 * sync with the picture, and an added effect is a finding, a silent press never one; off: no
 * sound-effect limits at all.
 */
export type SoundProfile = "strict" | "standard" | "loose" | "ui" | "music" | "off";
export const SOUND_PROFILES: ReadonlyArray<SoundProfile> = ["strict", "standard", "loose", "ui", "music", "off"];
export interface SoundLimits {
  /** Effects starting in any 10 s. */
  readonly per10s: number;
  /** The same file this many times or more is a finding. */
  readonly repeatMax: number;
  /** Big hits in any 15 s. */
  readonly hitsPer15s: number;
  /** An effect sits at least this many dB under the music (negative: it may be that much over it). */
  readonly underDb: number;
  /** Clicks on text and meme sounds are noted. */
  readonly taste: boolean;
}
export const SOUND_LIMITS: Readonly<Record<Exclude<SoundProfile, "off">, SoundLimits>> = {
  strict: { per10s: SFX_PER_10S, repeatMax: SFX_REPEAT_MAX, hitsPer15s: SFX_HITS_PER_15S, underDb: SFX_UNDER_MUSIC_DB, taste: true },
  standard: { per10s: SFX_PER_10S, repeatMax: SFX_REPEAT_MAX, hitsPer15s: SFX_HITS_PER_15S, underDb: SFX_UNDER_MUSIC_DB, taste: true },
  loose: { per10s: 12, repeatMax: 8, hitsPer15s: 4, underDb: -3, taste: false },
  ui: { per10s: 24, repeatMax: 1000, hitsPer15s: 2, underDb: SFX_UNDER_MUSIC_DB, taste: true },
  music: { per10s: 0, repeatMax: SFX_REPEAT_MAX, hitsPer15s: 0, underDb: SFX_UNDER_MUSIC_DB, taste: true },
};
/** The profile in a few words (the Brief card, project_check, the critic's sheet). */
export function soundProfileText(p: SoundProfile): string {
  if (p === "off") return "off: no limits on sound effects";
  const l = SOUND_LIMITS[p];
  const level = l.underDb > 0 ? `${l.underDb} dB under the music` : `up to ${-l.underDb} dB over the music`;
  if (p === "music") return "music only (the user's request): no sound effects; the sound is the music, judged on its tempo fitting the cutting and the cuts and big moments landing on its beats; an added effect is a finding, a press or a move without one never is";
  if (p === "ui") return `ui: one soft effect per real cursor press, drag or morph (at most ${l.per10s} in any 10 s), ${level}, no meme sounds; a finding, never a blocker`;
  return `${p}: at most ${l.per10s} effects in any 10 s, ${level}${p === "strict" ? "; too many or too loud fails a review" : "; too many or too loud is a finding"}`;
}

const SFX_NAME = /\b(sfx|fx|whoosh|swoosh|swish|whip|click|tap|pop|boom|hit|impact|riser|rise|ding|chime|glitch|notification|swipe|thud|stinger|sting|blip|beep|typing|keyboard)\b/i;
const SFX_TRACK = /\b(sfx|fx|effects?|sound effects?)\b/i;
const MEME_NAME = /\b(bruh|airhorn|air horn|scratch|trombone|crickets|laugh|laughter|applause|vine|fahh?|oof|meme)\b/i;
const HIT_NAME = /\b(boom|impact|hit|thud|slam|bass drop)\b/i;
const fileName = (m: Media) => m.path.replace(/\\/g, "/").split("/").pop() ?? "";

export type SfxKind = "whoosh" | "riser" | "ui" | "hit" | "meme" | "other";

/** One sound effect on the timeline. */
export interface PlacedSfx {
  readonly item: Item;
  readonly track: Track;
  readonly start: number;
  readonly media: Media;
  /** The sound's name in messages: the bundled effect's id, else the media's name. */
  readonly label: string;
  /** What gives the sound its identity (the same file in numbered copies is one). */
  readonly sound: string;
  readonly kind: SfxKind;
  /** item gain + track gain, dB. */
  readonly gain: number;
  /** From Biliup's pack (levelled to −20 LUFS, peaks under −1 dBFS, D208), so its level against the music is known. */
  readonly bundled: boolean;
}

/** A sound effect's kind, from the bundled pack's category or the file's name. */
function sfxKind(m: Media): SfxKind {
  const b = bundledSfxOf(m);
  const c = b?.category;
  const name = `${fileName(m)} ${m.name}`;
  if (c === "meme" || c === "crowd" || MEME_NAME.test(name)) return c === "meme" && b?.id === "dramatic-boom" ? "hit" : "meme";
  if (c === "whoosh" || c === "transition" || c === "swipe" || c === "glitch" || /whoosh|swoosh|swish|whip|swipe|transition|glitch/i.test(name)) return "whoosh";
  if (c === "riser" || /riser|rise|build/i.test(name)) return "riser";
  if (c === "impact" || HIT_NAME.test(name)) return "hit";
  if (c === "ui" || c === "notification" || c === "pop" || c === "success" || c === "typing" || /click|tap|pop|ding|chime|blip|beep|notification|typing|keyboard/i.test(name)) return "ui";
  return "other";
}

/**
 * D306: a keyboard or typing sound (the bundled pack's typing category, or its name): it belongs on
 * typing, so landing on a text being typed is right, not a click on text. Allowed unless the user's
 * request says music only (the music profile drops every effect).
 */
export const keyboardSound = (s: Pick<PlacedSfx, "media">) => {
  const b = bundledSfxOf(s.media);
  return b?.category === "typing" || /\b(typing|keyboard|keys|keystrokes?|key ?(press|stroke)e?s?|typewriter)\b/i.test(`${fileName(s.media)} ${s.media.name}`.replace(/[-_.]/g, " "));
};

export const trackGain = (t: Track) => t.mixer?.gainDb ?? 0;

/** A tempo written in a sound's name or file name ("Lofi loop 75 BPM", "beat-120bpm.mp3"), else undefined. */
export function bpmInName(m: Pick<Media, "name" | "path">): number | undefined {
  const hit = /(?:^|[^0-9])(\d{2,3})\s*-?\s*bpm\b/i.exec(`${m.name} ${m.path.replace(/\\/g, "/").split("/").pop() ?? ""}`);
  const bpm = hit ? Number(hit[1]) : NaN;
  return bpm >= 40 && bpm <= 240 ? bpm : undefined;
}

/** Music items (files named like music, long sounds on a music track), with their gain and the tempo its name gives. */
export function musicItems(ctx: CheckContext) {
  const out: Array<{ item: Item; track: Track; start: number; media: Media; gain: number; bpm: number | null | undefined }> = [];
  for (const track of ctx.project.tracks) {
    if (track.kind !== "audio" || !audible(ctx, track) || isSpeechTrack(ctx, track)) continue;
    const musicTrack = isMusicTrack(ctx, track);
    for (const item of track.items) {
      const m = soundOf(ctx, item);
      if (!m || item.type !== "audio" || bundledSfxOf(m)) continue;
      // D296: a long sound on a plain audio track ("Audio", a file named after its song) is music too,
      // unless it's named like an effect (and anything over 15 s: no effect is that long); before, a song not called "music" was neither music nor an effect.
      const effectNamed = SFX_NAME.test(fileName(m)) || SFX_NAME.test(m.name) || MEME_NAME.test(fileName(m)) || SFX_TRACK.test(track.name);
      if (MUSIC_NAME.test(fileName(m)) || ((musicTrack || !effectNamed) && item.duration > 6 * ctx.fps) || item.duration > 15 * ctx.fps) out.push({ item, track, start: startOf(ctx, item), media: m, gain: (item.gainDb ?? 0) + trackGain(track), bpm: bpmInName(m) });
    }
  }
  return out;
}

/** The sound effects: bundled ones, files or tracks named like effects, short sounds off the music and voice. */
export function sfxItems(ctx: CheckContext): Array<PlacedSfx> {
  const music = new Set(musicItems(ctx).map((m) => m.item.id));
  const out: Array<PlacedSfx> = [];
  for (const track of ctx.project.tracks) {
    if (track.kind !== "audio" || !audible(ctx, track) || isSpeechTrack(ctx, track)) continue;
    for (const item of track.items) {
      const m = soundOf(ctx, item);
      if (!m || item.type !== "audio" || music.has(item.id)) continue;
      const bundled = bundledSfxOf(m);
      const named = SFX_NAME.test(fileName(m)) || SFX_NAME.test(m.name) || MEME_NAME.test(fileName(m)) || SFX_TRACK.test(track.name);
      if (!bundled && !named && item.duration > 3 * ctx.fps) continue;
      out.push({ item, track, start: startOf(ctx, item), media: m, label: bundled?.id ?? m.name, sound: bundled?.id ?? m.path, kind: sfxKind(m), gain: (item.gainDb ?? 0) + trackGain(track), bundled: !!bundled });
    }
  }
  return out.sort((a, b) => a.start - b.start);
}

/** Something that changes on screen: a cut, a text or graphic arriving or leaving, a keyframe. */
export interface VisualEvent {
  readonly frame: number;
  readonly what: string;
  /** A text arriving (a click on it isn't a click the viewer sees). */
  readonly text: boolean;
  /**
   * A component or graphic animates inside itself (a tap indicator, a cursor in a render(t)
   * graphic): Biliup can't see when, so the event spans until here (a component's first second, a
   * graphic's whole length) and an effect anywhere in it counts as landing.
   */
  readonly until?: number;
}

export function visualEvents(ctx: CheckContext): Array<VisualEvent> {
  const out: Array<VisualEvent> = [];
  for (const track of ctx.project.tracks) {
    if (track.hidden || track.kind === "audio") continue;
    for (const item of track.items) {
      if (item.type === "captions") continue;
      const s = startOf(ctx, item);
      const name = item.type === "text" ? `"${textLabel(item.text)}"` : (item.name ?? item.id);
      const text = item.type === "text";
      if (s > 0) out.push({ frame: s, what: track.role === "main" ? `the cut to ${name}` : `${name} appearing`, text });
      if (item.type === "component" || item.type === "graphic") out.push({ frame: s, until: item.type === "graphic" ? s + item.duration : s + Math.min(item.duration, ctx.fps), what: `${name}'s own animation`, text: false });
      if (item.animation?.in?.duration) out.push({ frame: s + item.animation.in.duration, what: `${name} landing`, text });
      for (const k of item.keys ?? []) if (k.frame > 0) out.push({ frame: s + k.frame, what: `${name}'s move at its key`, text: false });
      const end = s + item.duration;
      if (end < ctx.end && track.role !== "main") out.push({ frame: end, what: `${name} leaving`, text });
    }
  }
  return out.sort((a, b) => a.frame - b.frame);
}

/** The visual events an effect lands on: whooshes over their first second, risers at their end, the rest at their start. */
export function hitsOf(ctx: CheckContext, s: PlacedSfx, events: ReadonlyArray<VisualEvent>) {
  const before = Math.round(SFX_SYNC_BEFORE_S * ctx.fps);
  const after = Math.max(2, Math.round(SFX_SYNC_AFTER_S * ctx.fps));
  const end = s.start + s.item.duration;
  const [a, b] = s.kind === "whoosh" ? [s.start - before, s.start + Math.min(s.item.duration, ctx.fps)] : s.kind === "riser" ? [end - Math.round(0.4 * ctx.fps), end + after] : [s.start - before, s.start + after];
  return events.filter((e) => (e.until !== undefined ? e.frame <= b && e.until >= a : e.frame >= a && e.frame <= b));
}

/** The most effects that start in any `window` frames, and where that window begins. */
export function busiest(starts: ReadonlyArray<number>, window: number) {
  let best = { count: 0, from: 0, to: 0 };
  for (let i = 0, j = 0; i < starts.length; i++) {
    while (starts[i]! - starts[j]! >= window) j++;
    if (i - j + 1 > best.count) best = { count: i - j + 1, from: starts[j]!, to: starts[i]! };
  }
  return best;
}

/** The music playing at a frame (the loudest, if several). */
export const musicAt = (music: ReturnType<typeof musicItems>, frame: number) => music.filter((m) => frame >= m.start && frame < m.start + m.item.duration).sort((a, b) => b.gain - a.gain)[0];

export const dbText = (db: number) => `${db > 0 ? "+" : db < 0 ? "−" : ""}${Math.abs(Math.round(db * 10) / 10)} dB`;

/**
 * The sound as lines (D207): music, voices and every effect with its time, kind, level, what it
 * lands on, then the counts. The critic reads it (it can't hear), and so can the agent.
 */
export function soundSheet(project: Project, options: { readonly speechMedia?: Iterable<Id>; readonly sound?: SoundProfile } = {}): Array<string> {
  const ctx = checkContext(project, options);
  const fps = ctx.fps;
  const lines: Array<string> = [];
  const music = musicItems(ctx);
  for (const m of music) lines.push(`music ${secs(m.start, fps)} to ${secs(m.start + m.item.duration, fps)}: ${m.media.name}${m.bpm ? `, ${m.bpm} BPM` : m.bpm === null ? ", no steady beat" : ""}, at ${dbText(m.gain)}${m.item.fadeIn ? ", fades in" : ""}${m.item.fadeOut ? ", fades out" : ""}${duckEntry(m.track) ? ", ducked under the voice" : ""}`);
  for (const track of ctx.project.tracks) {
    if (!isSpeechTrack(ctx, track) || !audible(ctx, track)) continue;
    const spoken = track.items.filter((i) => soundOf(ctx, i));
    if (spoken.length) lines.push(`voice on ${track.name}: ${spoken.map((i) => `${secs(startOf(ctx, i), fps)} to ${secs(startOf(ctx, i) + i.duration, fps)}`).join(", ")}`);
  }
  const sfx = sfxItems(ctx);
  const events = visualEvents(ctx);
  for (const s of sfx) {
    const hits = hitsOf(ctx, s, events);
    const under = musicAt(music, s.start);
    lines.push(`sound effect ${secs(s.start, fps)} (f ${s.start}): ${s.label} (${s.kind}), ${secs(s.item.duration, fps)}, at ${dbText(s.gain)}${under ? ` (${dbText(s.gain - under.gain)} against the music)` : ""}; ${hits.length ? `lands on ${hits.slice(0, 2).map((e) => `${e.what} (f ${e.frame})`).join(" and ")}` : "lands on nothing that changes on screen"}`);
  }
  const length = Math.max(1, ctx.end);
  if (!music.length && !sfx.length) lines.push("No music and no sound effects.");
  else lines.push(`${sfx.length} sound effect${sfx.length === 1 ? "" : "s"} in ${secs(length, fps)} (${(Math.round((sfx.length / (length / fps)) * 100) / 10).toFixed(1)} per 10 s; ${ctx.sound === "off" ? "no limit for this video" : ctx.sound === "music" ? "the user asked for the music only, so none are wanted" : `the bar is at most ${SOUND_LIMITS[ctx.sound].per10s}`}), ${new Set(sfx.map((s) => s.sound)).size} different`);
  lines.push(`Sound profile ${soundProfileText(ctx.sound)}.`);
  return lines;
}

/** The sound rule's findings that fail a critic round (density and level) under the strict profile, by key. */
export const SOUND_HARD = /^sound:(density|loud):/;
/** D217: whether a sound finding fails a critic round: density or level, and only under the strict profile. */
export const soundHard = (key: string, profile: SoundProfile) => profile === "strict" && SOUND_HARD.test(key);
