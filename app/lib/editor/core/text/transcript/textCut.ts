// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { applyEdit, createSession, EditError } from "@/app/lib/editor/core/timeline/apply-ops";
import { rippleMainGaps, withLinkedAudio } from "@/app/lib/editor/core/timeline/arrange";
import { mainEnd, resolveStarts, projectDuration, timelineProblems } from "@/app/lib/editor/core/timeline/rules";
import type { Op, Project, Item, Track } from "@/app/lib/editor/core/timeline/types";
import { type CutWord, ceilF, plural, floorF, spansFor, freeId, keptPieces, runsOf, quote, wordsText, wordIds, fitPiece } from "@/app/lib/editor/core/text/transcript/cutSpans";
import { sourceSpans, sourceRange, type SourceSpan } from '@/app/lib/editor/core/timeline/sourceSpans';
import { type SpeechSegment, type SpeechEnvelope, speechEdges } from "@/app/lib/editor/core/text/transcript/speech";
import { fillersOf, type Pause, pausesOf, tailFrames, handleFrames } from "@/app/lib/editor/core/text/transcript/fillers";
import { soundParts } from "@/app/lib/editor/core/text/transcript/editWords";

export interface CutTranscript {
  readonly words: ReadonlyArray<CutWord>;
  /** The spoken language (fillers are recognised for en and pl). */
  readonly language?: string | undefined;
  /** The file's length in seconds (for the pause after the last word). */
  readonly seconds?: number | undefined;
  /** Whisper's segments (their edges extend the words at the ends of a sentence, speechWords). */
  readonly segments?: ReadonlyArray<SpeechSegment> | undefined;
}

export interface TextCutRequest {
  /**
   * Word index ranges to remove, both ends included (transcript_read's w12 is index 12). With
   * `clip`, only from that main-track clip (the panel: where the words were selected); otherwise
   * from every clip that plays them. `cycle` restricts a named clip to one pass through a
   * looped source; without it, all occurrences are removed.
   */
  readonly remove?: ReadonlyArray<{ readonly from: number; readonly to: number; readonly clip?: string | undefined; readonly cycle?: number }>;
  /** Remove pauses at least `min` seconds long (keeping tailFrames after a kept word and handleFrames before one). */
  readonly pauses?: { readonly min: number };
  /** Remove the hesitation fillers (um, uh, yyy, eee); "maybe" fillers are never removed here. */
  readonly fillers?: boolean;
  /** The loudness analyzer's silences (seconds), when known: pauses take their edges (as transcript_read does). */
  readonly silences?: ReadonlyArray<{ readonly start: number; readonly end: number }> | undefined;
  /** The loudness analyzer's speech envelope, when known: words end where their sound does (D74). */
  readonly envelope?: SpeechEnvelope | undefined;
  /** Only these main-track clips (the panel cuts where the words were selected); default: every main clip using the media. */
  readonly clips?: ReadonlyArray<string>;
  /**
   * D215: cut these items instead of the main track's clips (any track: a Voiceover recording, a
   * take's microphone; transcriptScope gives them). A range's `clip` names one of them. An item
   * linked to a clip cuts that clip (it follows); an item on a free track is cut in place and its
   * later pieces close up, its group's other parts get the same cuts; nothing else on its track moves.
   */
  readonly items?: ReadonlyArray<string>;
}

/** One occurrence of a source stretch cut from the timeline. */
export interface TextCut {
  readonly cycle: number;
  /** Project frames of the media: [from, to). */
  readonly from: number;
  readonly to: number;
  /** The removed words: the first and last index, and their text ("" for a pause). */
  readonly words?: readonly [number, number];
  readonly text: string;
  readonly pauses: number;
  readonly fillers: number;
  /** Timeline frames it removed (all clips together) and the clips it came out of. */
  readonly frames: number;
  readonly clips: ReadonlyArray<string>;
}

export interface TextCutPlan {
  /** One edit: send all of it as one project.edit / agent edit, so one undo restores it. */
  readonly ops: ReadonlyArray<Op>;
  readonly summary: string;
  readonly cuts: ReadonlyArray<TextCut>;
  /** Timeline frames the main track got shorter by. */
  readonly framesRemoved: number;
  /** The project's length before and after, in frames. */
  readonly before: number;
  readonly after: number;
  /** What was asked for but not done, and other things worth saying (sentences). */
  readonly notes: ReadonlyArray<string>;
  /** The project after the edit (checked with the timeline rules). */
  readonly project: Project;
}

/**
 * Cut words (ranges of word indices), pauses (at least `min` seconds) and/or hesitation fillers of
 * a media file out of the edit: from every main-track clip that uses the media (or whose sound part
 * does, D215: a take's microphone linked to its picture; or only `request.clips`, or a range's own
 * `clip`), respecting each clip's in-point, length and speed. A cut ripples the main track, so the
 * clips after it close up (D224: gaps before them stay); items on other tracks don't move. With
 * `request.items` (D215) those items are cut instead, wherever they are: one linked to a clip cuts
 * that clip; one on a free track is cut in place and its later pieces close up by what was cut, as
 * do the same stretches of its group's other parts (a take's camera and screen); nothing else on
 * its track moves. Each clip's stretches are merged and snapped to frames (words cover their
 * frames, pauses go inward, tailFrames stay after and handleFrames before every word that's kept), a cut
 * never leaves an empty clip, and transitions at the cuts that change are removed (D52). All of it
 * is one edit. Words that aren't in the edit any more (already cut) change nothing and come back
 * as a note.
 */
export function planTextCut(project: Project, mediaId: string, transcript: CutTranscript, request: TextCutRequest): TextCutPlan | { readonly problem: string } {
  const media = project.media.find((m) => m.id === mediaId);
  if (!media) return { problem: `There's no media "${mediaId}" in this project.` };
  if (!request.remove?.length && !request.pauses && !request.fillers) return { problem: "Say what to remove: word ranges, pauses or fillers." };
  const main = project.tracks.find((t) => t.role === "main");
  if (!main) return { problem: "This project has no main track." };
  const itemMode = request.items !== undefined;
  if (!itemMode && main.locked) return { problem: "The main track is locked. Unlock it to cut by text." };
  const fps = project.canvas.fps;
  // Where the words are heard (whisper's words widened to its segments' edges and the silences, D74):
  // every rule below (kept, handles, pauses) works on these.
  const heard = speechEdges(transcript.words, { segments: transcript.segments, silences: request.silences, seconds: transcript.seconds, envelope: request.envelope });
  const { words } = heard;
  const n = words.length;
  const length = media.probe?.frames ?? ceilF((transcript.seconds ?? words.at(-1)?.end ?? 0) * fps);
  const notes: Array<string> = [];

  // What's asked: words for every clip, words for one clip, fillers, pauses.
  const everywhere = new Set<number>();
  const perClipWords = new Map<string, Set<number>>();
  const perCycleWords = new Map<string, Map<number, Set<number>>>();
  for (const r of request.remove ?? []) {
    if (!Number.isInteger(r.from) || !Number.isInteger(r.to) || r.from < 0 || r.to < r.from) return { problem: `A word range must go from a word to the same or a later one (got ${r.from}–${r.to}).` };
    if (r.to >= n) return { problem: `The transcript has ${plural(n, "word")} (w0–w${n - 1}); there's no w${r.to}.` };
    if (r.cycle !== undefined && (!Number.isSafeInteger(r.cycle) || r.cycle < 0 || r.clip === undefined)) return { problem: 'A loop occurrence needs a clip and a non-negative integer cycle.' };
    let into: Set<number>;
    if (r.cycle !== undefined) {
      const cycles = perCycleWords.get(r.clip!) ?? new Map<number, Set<number>>();
      perCycleWords.set(r.clip!, cycles);
      into = cycles.get(r.cycle) ?? new Set<number>();
      cycles.set(r.cycle, into);
    } else into = r.clip === undefined ? everywhere : (perClipWords.get(r.clip) ?? perClipWords.set(r.clip, new Set()).get(r.clip)!);
    for (let i = r.from; i <= r.to; i++) into.add(i);
  }
  const fillerWords = new Set<number>();
  if (request.fillers) {
    if (transcript.language !== "en" && transcript.language !== "pl") notes.push(`Fillers are only recognised in English and Polish (this transcript is ${transcript.language ?? "in an unknown language"}).`);
    for (const fl of fillersOf(words, transcript.language)) if (fl.kind === "hesitation") for (let k = 0; k < fl.n; k++) (everywhere.add(fl.i + k), fillerWords.add(fl.i + k));
  }
  let pauses: Array<Pause> = [];
  if (request.pauses) {
    const min = request.pauses.min;
    if (!(typeof min === "number" && Number.isFinite(min) && min > 0)) return { problem: "pauses.min must be a number of seconds above 0." };
    pauses = pausesOf(words, transcript.seconds, request.silences, Math.max(min, 0.05), undefined, request.envelope);
  }

  const only = request.clips ? new Set(request.clips) : undefined;
  const starts = resolveStarts(project);
  const before = projectDuration(project);
  const mainBefore = mainEnd(project);
  const where = new Map(project.tracks.flatMap((t) => t.items.map((i, index) => [i.id, { item: i, track: t, index }] as const)));
  // The items whose words are measured (`src`: they play the media) and the item each cut goes into
  // (`target`: the same, or the clip a linked sound follows / a main clip's sound part speaks for).
  // `owner` is the id the panel's words name (a range's `clip`).
  type Source = { readonly src: Item; readonly target: Item; readonly track: Track; readonly owner: string };
  const sources: Array<Source> = [];
  if (!itemMode) {
    for (const c of main.items) {
      if (c.type !== "clip" || (only && !only.has(c.id) && !perClipWords.has(c.id) && !perCycleWords.has(c.id))) continue;
      if (c.media === mediaId) sources.push({ src: c, target: c, track: main, owner: c.id });
      else {
        const part = soundParts(project, c).find((p) => p.media === mediaId);
        if (part) sources.push({ src: part, target: c, track: main, owner: c.id });
      }
    }
  } else {
    for (const id of new Set(request.items)) {
      const at = where.get(id);
      if (!at || (at.item.type !== "clip" && at.item.type !== "audio") || at.item.media !== mediaId) continue;
      // A linked item cuts its clip (it follows): they share their frames (start, length, speed).
      const link = at.item.linkedTo ? where.get(at.item.linkedTo) : undefined;
      const linked = link && link.item.type === "clip" && (starts.get(link.item.id) ?? 0) === (starts.get(at.item.id) ?? 0) && link.item.duration === at.item.duration && (link.item.speed ?? 1) === (at.item.speed ?? 1);
      const target = linked ? link : at;
      // One source per target (a muted clip and its detached sound in the same scope).
      if (sources.some((s) => s.target.id === target.item.id)) continue;
      sources.push({ src: at.item, target: target.item, track: target.track, owner: at.item.id });
    }
  }
  const lockedOf = (s: Source) => s.target.locked || s.track.locked;
  const lockedClips = sources.filter(lockedOf);
  if (lockedClips.length) notes.push(`${lockedClips.map((s) => s.target.id).join(", ")} ${lockedClips.length === 1 ? "is" : "are"} locked, so nothing was cut from ${lockedClips.length === 1 ? "it" : "them"}.`);
  const cuttable = sources.filter((s) => !lockedOf(s));
  const touches = (span: SourceSpan, i: number) => floorF(words[i]!.start * fps) < span.to && ceilF(words[i]!.end * fps) > span.from;
  const plays = (span: SourceSpan, i: number) => {
    const mid = ((words[i]!.start + words[i]!.end) / 2) * fps;
    return mid >= span.from && mid < span.to;
  };
  const passes = new Map<Source, ReadonlyArray<SourceSpan>>();
  for (const source of cuttable) {
    const delta = (starts.get(source.src.id) ?? 0) - (starts.get(source.target.id) ?? 0);
    passes.set(source, [...sourceSpans(source.src, media.probe?.frames, { start: -delta, end: source.target.duration - delta })]);
  }

  // Report each occurrence separately, including repeated words/pauses in a looping clip.
  const byStretch = new Map<string, { cycle: number; from: number; to: number; words: Set<number>; pauses: number; fillers: number; frames: number; clips: Array<string> }>();
  const perClip = new Map<string, Array<[number, number]>>();
  const tight = new Set<number>();
  const cutWords = new Set<number>();
  const slowWords = new Set<number>();
  for (const source of cuttable) {
    const { src: item, target, owner } = source;
    const delta = (starts.get(item.id) ?? 0) - (starts.get(target.id) ?? 0);
    const ranges: Array<[number, number]> = [];
    for (const pass of passes.get(source)!) {
      const removed = new Set([...everywhere, ...(perClipWords.get(owner) ?? []), ...(perCycleWords.get(owner)?.get(pass.cycle) ?? [])]);
      if (!removed.size && !pauses.length) continue;
      const kept = (i: number) => !removed.has(i) && plays(pass, i);
      for (const sp of spansFor(words, removed, fillerWords, kept, (i) => touches(pass, i), pauses, fps, length, tight, heard)) {
        const [a, b] = sourceRange(item, pass, sp.from, sp.to);
        if (b <= a) {
          if ((item.speed ?? 1) !== 1) for (const i of sp.words) if (plays(pass, i)) slowWords.add(i);
          continue;
        }
        ranges.push([a + delta, b + delta]);
        for (const i of sp.words) cutWords.add(i);
        // Report source-file frames, not the unwrapped time of a later loop occurrence.
        const inF = item.in ?? 0;
        const speed = item.speed ?? 1;
        const mFrom = Math.max(0, Math.round(inF + a * speed - pass.offset));
        const mTo = Math.min(length, Math.round(inF + b * speed - pass.offset));
        const stretch = sp.words.length ? `w${[...new Set(sp.words)].sort((x, y) => x - y).join(",")}` : `${sp.from}-${sp.to}`;
        const key = JSON.stringify([target.id, pass.cycle, stretch]);
        const entry = byStretch.get(key) ?? byStretch.set(key, { cycle: pass.cycle, from: mFrom, to: mTo, words: new Set(), pauses: sp.pauses, fillers: sp.fillers, frames: 0, clips: [] }).get(key)!;
        entry.from = Math.min(entry.from, mFrom);
        entry.to = Math.max(entry.to, mTo);
        for (const i of sp.words) entry.words.add(i);
        entry.frames += b - a;
        if (!entry.clips.includes(target.id)) entry.clips.push(target.id);
      }
    }
    // Touching local ranges (rounding at a speed) are one cut.
    const merged: Array<[number, number]> = [];
    for (const r of ranges.sort((x, y) => x[0] - y[0])) {
      const last = merged.at(-1);
      if (last && r[0] <= last[1]) last[1] = Math.max(last[1], r[1]);
      else merged.push([r[0], r[1]]);
    }
    if (merged.length) perClip.set(target.id, merged);
  }

  // The timeline stretches cut, merged: free pieces after one close up by its length (D215).
  const removedSpans: Array<[number, number]> = [];
  for (const { target } of cuttable) for (const [a, b] of perClip.get(target.id) ?? []) removedSpans.push([(starts.get(target.id) ?? 0) + a, (starts.get(target.id) ?? 0) + b]);
  removedSpans.sort((x, y) => x[0] - y[0]);
  const union: Array<[number, number]> = [];
  for (const r of removedSpans) {
    const last = union.at(-1);
    if (last && r[0] <= last[1]) last[1] = Math.max(last[1], r[1]);
    else union.push([r[0], r[1]]);
  }
  const removedBefore = (t: number) => union.reduce((sum, [a, b]) => sum + (b <= t ? b - a : 0), 0);
  // A target's group's other parts (a take's camera, screen, other sound) get the same timeline cuts.
  const cutIds = new Set(cuttable.filter((s) => perClip.has(s.target.id)).map((s) => s.target.id));
  const followers: Array<{ item: Item; track: Track }> = [];
  for (const id of cutIds) {
    const g = where.get(id)!.item.group;
    if (!g) continue;
    for (const track of project.tracks) {
      if (track.role === "main") continue;
      for (const item of track.items) {
        if (item.group !== g || cutIds.has(item.id) || followers.some((f) => f.item.id === item.id) || (item.linkedTo && cutIds.has(item.linkedTo))) continue;
        if (item.type !== "clip" && item.type !== "audio") continue;
        const s0 = starts.get(item.id) ?? 0;
        const local: Array<[number, number]> = [];
        for (const [a, b] of union) {
          const from = Math.max(a, s0) - s0;
          const to = Math.min(b, s0 + item.duration) - s0;
          if (to > from) local.push([from, to]);
        }
        if (!local.length) continue;
        if (item.locked || track.locked) return { problem: 'A grouped clip is locked. Unlock it before cutting the transcript.' };
        perClip.set(item.id, local);
        followers.push({ item, track });
      }
    }
  }

  const ops: Array<Op> = [];
  const drops = new Set<string>();
  const taken = new Set<string>();
  const pieces: Array<string> = [];
  const moves: Array<Op> = [];
  const targets = [...cuttable.map((s) => ({ item: s.target, track: s.track })), ...followers];
  for (const { item, track } of targets) {
    const ranges = perClip.get(item.id);
    if (!ranges) continue;
    // What's kept of the clip, for the transitions at its ends.
    const keptParts: Array<[number, number]> = [];
    let at = 0;
    for (const [a, b] of ranges) {
      if (a > at) keptParts.push([at, a]);
      at = b;
    }
    if (at < item.duration) keptParts.push([at, item.duration]);
    if (track.role === "main") {
      const index = main.items.indexOf(item);
      const next = main.items[index + 1];
      const first = keptParts[0];
      const last = keptParts.at(-1);
      if (item.transitionIn && !(first && first[0] === 0 && first[1] - first[0] >= 2 * item.transitionIn.duration)) drops.add(item.id);
      if (next?.transitionIn && !(last && last[1] === item.duration && last[1] - last[0] >= 2 * next.transitionIn.duration)) drops.add(next.id);
    }
    // Right to left, so the left part keeps its id and its local frames. A new id only for a real
    // split (a cut inside the clip), handed out left to right (clip-2 before clip-3 on the timeline).
    const base = item.id.replace(/-\d+$/, "") || "clip";
    const newIds = ranges.filter(([a, b]) => a > 0 && b < item.duration).map(() => freeId(project, base, taken));
    const idsInOrder = [...newIds];
    let end = item.duration;
    const clipOps: Array<Op> = [];
    for (const [a, b] of [...ranges].reverse()) {
      if (b >= end) {
        if (a === 0) clipOps.push({ op: "delete_item", item: item.id });
        else clipOps.push({ op: "trim_item", item: item.id, edge: "end", frames: end - a });
      } else if (a === 0) {
        clipOps.push({ op: "trim_item", item: item.id, edge: "start", frames: b });
      } else {
        const newId = newIds.pop()!;
        clipOps.push({ op: "split_item", item: item.id, at: b, newId }, { op: "trim_item", item: item.id, edge: "end", frames: b - a });
        pieces.push(newId);
      }
      end = a;
    }
    ops.push(...clipOps);
    if (keptParts.length) pieces.push(item.id);
    // Off main (D215): each kept piece moves left by what was cut before it.
    if (track.role !== "main") {
      const s0 = starts.get(item.id) ?? 0;
      for (const piece of keptPieces(item, ranges, idsInOrder)) {
        const at0 = s0 + piece.from;
        const to = at0 - removedBefore(at0);
        if (to !== at0) moves.push({ op: "move_item", item: piece.id, start: Math.max(0, to) });
      }
    }
  }
  // D224: a cut closes up (ripple), but the empty space before a main clip it removes whole
  // stays, before the next clip that stays (which then follows a gap: its transition goes).
  const goneMain = new Set(ops.flatMap((o) => (o.op === "delete_item" && main.items.some((i) => i.id === o.item) ? [o.item] : [])));
  const gapOps = rippleMainGaps(main, goneMain);
  for (const o of gapOps) if (o.op === "update_item" && main.items.find((i) => i.id === o.item)?.transitionIn) drops.add(o.item);
  // A transition goes before the ops that cut its clip (it may be deleted).
  ops.unshift(...[...drops].map((id): Op => ({ op: "set_transition", item: id, transition: null })));
  ops.push(...gapOps, ...moves);

  // Asked-for words that weren't cut, and why: not played by any clip (already cut), too tight
  // between the words around them, or too short at a clip's speed.
  const asked = new Map<number, Array<SourceSpan>>();
  const allPasses = [...passes.values()].flat();
  for (const i of everywhere) asked.set(i, allPasses);
  for (const [clip, ids] of perClipWords) for (const i of ids) asked.set(i, [...(asked.get(i) ?? []), ...cuttable.filter((c) => c.owner === clip).flatMap(c => passes.get(c) ?? [])]);
  for (const [clip, cycles] of perCycleWords) for (const [cycle, ids] of cycles) for (const i of ids) {
    asked.set(i, [...(asked.get(i) ?? []), ...cuttable.filter(c => c.owner === clip).flatMap(c => (passes.get(c) ?? []).filter(pass => pass.cycle === cycle))]);
  }
  for (const run of runsOf([...asked.keys()].filter((i) => !cutWords.has(i)))) {
    const where = `${quote(wordsText(words, run))} (${wordIds(run)})`;
    if (!run.some((i) => asked.get(i)!.some((pass) => plays(pass, i)))) notes.push(`${where} isn't in the edit${lockedClips.length ? " (or only in a locked clip)" : " any more (already cut)"}, so nothing changed there.`);
    else if (run.some((i) => slowWords.has(i))) notes.push(`${where} is too short to cut at the clip's speed: it lasts less than a frame there.`);
    else if (run.some((i) => tight.has(i))) notes.push(`${where} runs into the words around it, so it can't be cut without clipping them.`);
  }
  if (request.pauses && ![...byStretch.values()].some((c) => c.pauses)) {
    if (!pauses.length || !cuttable.length) notes.push("There are no pauses that long in the edit.");
    else notes.push(`The ${plural(pauses.length, "pause")} of that length ${pauses.length === 1 ? "isn't" : "aren't"} in the edit, or leave nothing to cut after keeping ${tailFrames(fps)} frames after the words and ${handleFrames(fps)} before them.`);
  }
  if (!sources.length) notes.push(itemMode ? `None of those items plays ${media.name}.` : `No clip on the main track uses ${media.name}${only ? " (among the clips asked for)" : ""}.`);
  if (!itemMode) {
    const followed = new Set(sources.map((s) => s.src.id));
    const elsewhere = project.tracks.filter((t) => t.role !== "main").flatMap((t) => t.items.filter((i) => i.media === mediaId && !followed.has(i.id) && !(i.linkedTo && cutIds.has(i.linkedTo))).map((i) => i.id));
    if (elsewhere.length && perClip.size) notes.push(`${elsewhere.join(", ")} on other tracks also ${elsewhere.length === 1 ? "uses" : "use"} ${media.name} and weren't cut: cut ${elsewhere.length === 1 ? "it" : "them"} by item.`);
  }

  let draft: Project;
  try {
    // Linked items follow their clips in the draft as the server applies them (D123), so a plan
    // chained on this one sees them where they'll be; the ops sent stay the plain ones.
    draft = applyEdit(createSession(project), { actor: "user", ops }).project;
    // Shorter pieces: fades, animations and keys that no longer fit are shortened in the same edit.
    for (const id of pieces) {
      const piece = draft.tracks.flatMap((t) => t.items).find((i) => i.id === id);
      const fix = piece && fitPiece(piece);
      if (fix) ops.push(fix);
    }
    draft = applyEdit(createSession(project), { actor: "user", ops: withLinkedAudio(project, ops) }, timelineProblems).project;
  } catch (error) {
    const first = error instanceof EditError ? (error.details[0] as { message?: string } | undefined) : undefined;
    return { problem: `That cut doesn't work: ${first?.message ?? (error as Error).message}.` };
  }
  const mainCut = mainBefore - mainEnd(draft);
  const framesRemoved = mainCut > 0 ? mainCut : union.reduce((sum, [a, b]) => sum + b - a, 0);
  const after = projectDuration(draft);
  // Free-track items after the first cut don't move with the main track: worth a look.
  if (mainCut > 0) {
    let firstCut = Infinity;
    for (const { target } of cuttable) {
      const r = perClip.get(target.id);
      if (r && where.get(target.id)?.track.role === "main") firstCut = Math.min(firstCut, (starts.get(target.id) ?? 0) + r[0]![0]);
    }
    const moved = new Set([...followers.map((f) => f.item.id), ...cutIds]);
    const shifted = project.tracks.filter((t) => t.role !== "main").flatMap((t) => t.items.filter((i) => !moved.has(i.id) && !(i.linkedTo && moved.has(i.linkedTo)) && (i.start ?? 0) + i.duration > firstCut));
    if (shifted.length) notes.push(`Items on other tracks (${shifted.slice(0, 4).map((i) => i.id).join(", ")}${shifted.length > 4 ? ", …" : ""}) didn't move with the main track; check they still line up.`);
  }

  const cuts: Array<TextCut> = [...byStretch.values()]
    .sort((x, y) => x.from - y.from)
    .map((c) => {
      const ids = [...c.words].sort((x, y) => x - y);
      return { cycle: c.cycle, from: c.from, to: c.to, ...(ids.length ? { words: [ids[0]!, ids.at(-1)!] as const } : {}), text: wordsText(words, ids), pauses: c.pauses, fillers: c.fillers, frames: c.frames, clips: c.clips };
    });
  const wordCount = cuts.reduce((sum, cut) => sum + (cut.words ? cut.words[1] - cut.words[0] + 1 : 0), 0);
  const pauseCount = cuts.reduce((m, c) => m + c.pauses, 0);
  const fillerCount = cuts.reduce((m, c) => m + c.fillers, 0);
  const secs = `${(framesRemoved / fps).toFixed(1)} s`;
  const onlyPauses = !request.remove?.length && !request.fillers;
  const onlyFillers = !request.remove?.length && !request.pauses;
  const summary = !ops.length
    ? ""
    : onlyPauses
      ? `Removed ${plural(pauseCount, "pause")} (${secs})`
      : onlyFillers
        ? `Removed ${plural(fillerCount, "filler")} (${secs})`
        : cuts.length === 1 && !pauseCount && cuts[0]!.text
          ? `Cut ${quote(cuts[0]!.text, 40)} (${secs})`
          : `Cut ${[wordCount ? plural(wordCount, "word") : "", pauseCount ? plural(pauseCount, "pause") : ""].filter(Boolean).join(" and ")} (${secs})`;
  if (!ops.length && !notes.length) notes.push(onlyFillers ? "There are no hesitation fillers in the edit." : "Nothing to cut.");
  return { ops, summary, cuts, framesRemoved, before, after, notes, project: draft };
}
