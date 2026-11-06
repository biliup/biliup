// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Marked sections (issue #118): stretches of the timeline the user locked, so that nothing playing
// there ever moves or changes (a music video's cuts stay on the song), and stretches flagged "not
// OK" for the agent to redo. applyEdit refuses any edit that would change what a locked section
// plays, by the user (a ripple trim or delete before it) or the agent alike; the planners here make
// the update_project edits the editor sends. Browser-safe.
import { keyDbAt } from "@/app/lib/editor/core/audio/audio";
import { captionCards } from "@/app/lib/editor/core/text/captions";
import { KEY_PROPS, keyAt } from "@/app/lib/editor/core/visual/box";
import { layout } from "@/app/lib/editor/core/timeline/timeline";
import type { Item, Op, Problem, Project, Section } from "@/app/lib/editor/core/timeline/types";

// Fields that say nothing about what plays (names, ids, grouping, the item's own lock) or that the
// content descriptor below gives in timeline frames instead (where it starts, how long, its in point).
const NOT_CONTENT = new Set(["id", "name", "group", "linkedTo", "locked", "start", "gap", "duration", "in", "keys", "gainKeys", "fadeIn", "fadeOut", "transitionIn", "animation"]);
// Captions: their words, corrections and card breaks are compared as the cards they make in the section.
const NOT_CAPTION_CONTENT = new Set([...NOT_CONTENT, "words", "edits", "breaks"]);

const round = (n: number) => Math.round(n * 1e4) / 1e4;

/**
 * Box keys (x, y, scale, opacity) and volume keys as the values they give on each frame of [a, b)
 * (timeline frames), so a split or trim outside the section (which moves, adds or drops keys there)
 * compares equal while the values in it are the same. Other keyed fields are compared as they are.
 */
function keyedValues(project: Project, item: Item, start: number, a: number, b: number) {
  const fps = project.canvas.fps;
  const keys = item.keys ?? [];
  const props = new Set(keys.flatMap((k) => Object.keys(k)).filter((p) => p !== "frame" && p !== "ease"));
  const box: Record<string, Array<number | null>> = {};
  for (const p of KEY_PROPS) {
    if (!props.has(p)) continue;
    const values: Array<number | null> = [];
    for (let f = a; f < b; f++) values.push(round(keyAt(keys, p, f - start, fps) ?? Number.NaN));
    box[p] = values;
  }
  const other = keys.filter((k) => Object.keys(k).some((p) => p !== "frame" && p !== "ease" && !(KEY_PROPS as ReadonlyArray<string>).includes(p))).map((k) => ({ ...k, frame: k.frame + start }));
  const gain: Array<number> = [];
  if (item.gainKeys?.length) for (let f = a; f < b; f++) gain.push(round(keyDbAt(item.gainKeys, f - start)));
  return { box, other, gain };
}

/**
 * What one item plays inside [from, to): its fields that change the picture or the sound, the part
 * of it in the section in timeline frames and the media frame it plays there, and its keys, fades,
 * moves and incoming transition at their timeline frames, so a trim, split or ripple elsewhere that
 * leaves the section as it was compares equal (a split before the section renames the half in it).
 */
function itemContent(project: Project, item: Item, track: string, start: number, from: number, to: number) {
  const rest: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(item)) if (!(item.type === "captions" ? NOT_CAPTION_CONTENT : NOT_CONTENT).has(k)) rest[k] = v;
  const a = Math.max(start, from);
  const b = Math.min(start + item.duration, to);
  const media = item.media ? project.media.find((m) => m.id === item.media) : undefined;
  const timed = item.type === "graphic" || (media ? media.kind !== "image" : item.in !== undefined);
  return {
    track,
    from: a,
    to: b,
    ...(timed ? { src: Math.round(((item.in ?? 0) + (a - start) * (item.speed ?? 1)) * 1000) / 1000 } : {}),
    rest,
    ...(item.type === "captions" ? { cards: cardsIn(project, item, from, to) } : {}),
    keys: keyedValues(project, item, start, a, b),
    // A fade, a move in or out and a transition only matter where they reach into the section.
    fadeIn: item.fadeIn && start + item.fadeIn > from ? [start, start + item.fadeIn] : undefined,
    fadeOut: item.fadeOut && start + item.duration - item.fadeOut < to ? [start + item.duration - item.fadeOut, start + item.duration] : undefined,
    animIn: item.animation?.in && start + item.animation.in.duration > from ? { ...item.animation.in, at: start } : undefined,
    animOut: item.animation?.out && start + item.duration - item.animation.out.duration < to ? { ...item.animation.out, at: start + item.duration } : undefined,
    animLoop: item.animation?.loop ? { ...item.animation.loop, at: start } : undefined,
    transitionIn: item.transitionIn && start - item.transitionIn.duration < to && start + item.transitionIn.duration > from ? { ...item.transitionIn, at: start } : undefined,
  };
}

/** A captions item's cards shown in [from, to) (timeline frames): their words, timing and lines. */
function cardsIn(project: Project, item: Item, from: number, to: number) {
  return captionCards(project, item)
    .filter((c) => c.from < to && c.to > from)
    .map((c) => ({ from: c.from, to: c.to, lines: c.lines, words: c.words.map((w) => [w.text, w.from, w.to]) }));
}

/** What plays in frames [from, to) on every track, as one string: equal strings, nothing there changed. */
export function sectionContent(project: Project, from: number, to: number): string {
  const parts = sectionItems(project, from, to).map(({ item, track, start }) => itemContent(project, item, track.id, start, from, to));
  parts.sort((a, b) => (a.track < b.track ? -1 : a.track > b.track ? 1 : a.from - b.from));
  return JSON.stringify(parts);
}

/** The items that play in frames [from, to), with their tracks and timeline starts. */
function sectionItems(project: Project, from: number, to: number) {
  const { starts } = layout(project);
  const out: Array<{ readonly item: Item; readonly track: Project["tracks"][number]; readonly start: number }> = [];
  for (const track of project.tracks) {
    for (const item of track.items) {
      const start = starts.get(item.id) ?? 0;
      // An incoming transition plays before its cut too; captions draw where their cards are
      // (over Main's clips when they cover it, whatever their own start and length say).
      const reach = item.transitionIn?.duration ?? 0;
      const outside = start - reach >= to || start + item.duration <= from;
      if (outside && !(item.type === "captions" && cardsIn(project, item, from, to).length)) continue;
      out.push({ item, track, start });
    }
  }
  return out;
}

/**
 * What else decides how [from, to) looks and sounds, outside its items: the tracks they're on (in
 * their order, shown or hidden, their mixer), which tracks are soloed, the files of the media they
 * play, and the canvas, theme, master and motion blur.
 */
function sectionContext(project: Project, from: number, to: number): string {
  const items = sectionItems(project, from, to);
  const tracks = new Set(items.map(({ track }) => track.id));
  const media = new Set(items.map(({ item }) => item.media).filter((m): m is string => !!m));
  return JSON.stringify({
    tracks: project.tracks.filter((t) => tracks.has(t.id)).map((t) => ({ id: t.id, hidden: t.hidden === true, mixer: t.mixer ? { ...t.mixer, solo: undefined, linkedTo: undefined } : undefined })),
    solo: project.tracks.filter((t) => t.mixer?.solo).map((t) => t.id),
    media: project.media.filter((m) => media.has(m.id)).map((m) => ({ id: m.id, kind: m.kind, path: m.path })),
    canvas: project.canvas,
    theme: project.theme,
    master: project.master,
    motionBlur: (project as { motionBlur?: unknown }).motionBlur,
  });
}

/**
 * Issue #118, the agent's edits only: the problem when one changes how a locked section looks or
 * sounds from outside its items (a track there hidden, muted, soloed against, turned down, given an
 * effect or moved in the stack; a file it plays relinked; the canvas, theme or master changed), else
 * undefined. The user's own such changes are theirs to make.
 */
export function lockedContextProblem(before: Project, after: Project): Problem | undefined {
  for (const s of stillLocked(before, after)) {
    if (sectionContext(before, s.start, s.end) === sectionContext(after, s.start, s.end)) continue;
    const fps = before.canvas.fps;
    return {
      code: "locked-section",
      path: `/sections/${s.id}`,
      message: `frames ${s.start}–${s.end} (${secs(s.start, fps)}–${secs(s.end, fps)}) are locked, and this would change how they look or sound (a track, the master, the canvas or a media file they use); leave those as they are, or ask the user to unlock it`,
    };
  }
  return undefined;
}

const secs = (frame: number, fps: number) => {
  const s = frame / fps;
  return `${Math.floor(s / 60)}:${(s % 60).toFixed(1).padStart(4, "0")}`;
};

/** The locked sections of `before` still locked over the same frames in `after` (an edit that unlocks one may change it). */
const stillLocked = (before: Project, after: Project) =>
  (before.sections ?? []).filter((s) => s.state === "locked" && (after.sections ?? []).some((a) => a.id === s.id && a.state === "locked" && a.start === s.start && a.end === s.end));

/**
 * The problem when going from `before` to `after` changes what a locked section plays (a clip in it
 * moved, trimmed, swapped or restyled, or everything after a ripple shifted into it), else undefined.
 */
export function lockedSectionProblem(before: Project, after: Project): Problem | undefined {
  for (const s of stillLocked(before, after)) {
    if (sectionContent(before, s.start, s.end) === sectionContent(after, s.start, s.end)) continue;
    const fps = before.canvas.fps;
    return {
      code: "locked-section",
      path: `/sections/${s.id}`,
      message: `frames ${s.start}–${s.end} (${secs(s.start, fps)}–${secs(s.end, fps)}) are locked, and this would move or change what plays there; edit outside it without shifting it (an overwrite, not a ripple), or unlock it first`,
    };
  }
  return undefined;
}

/**
 * Issue #118: the problem when the agent's edit unlocks, moves or resizes a locked section (only
 * the user unlocks), else undefined. It may add sections and clear or change its "not OK" ones.
 */
export function unlockProblem(before: Project, after: Project): Problem | undefined {
  const kept = stillLocked(before, after);
  const gone = (before.sections ?? []).find((s) => s.state === "locked" && !kept.includes(s));
  if (!gone) return undefined;
  return { code: "locked-section", path: `/sections/${gone.id}`, message: `only the user can unlock or change the locked section ${gone.id} (frames ${gone.start}–${gone.end}); ask them to` };
}

/** A new section id: "section-1", "section-2", … */
const newId = (project: Project, taken: Set<string>) => {
  for (let n = 1; ; n++) {
    const id = `section-${n}`;
    if (!taken.has(id) && !(project.sections ?? []).some((s) => s.id === id)) {
      taken.add(id);
      return id;
    }
  }
};

/** `sections` with [from, to) cut out of them (a section partly covered keeps what's left of it, on each side). */
function without(project: Project, sections: ReadonlyArray<Section>, from: number, to: number, taken: Set<string>): Array<Section> {
  const out: Array<Section> = [];
  for (const s of sections) {
    if (s.end <= from || s.start >= to) {
      out.push(s);
      continue;
    }
    if (s.start < from) out.push({ ...s, end: from });
    if (s.end > to) out.push({ ...s, id: s.start < from ? newId(project, taken) : s.id, start: to });
  }
  return out;
}

const edit = (sections: ReadonlyArray<Section>): ReadonlyArray<Op> => [{ op: "update_project", set: { sections: sections.length ? [...sections].sort((a, b) => a.start - b.start) : null } }];

/**
 * Lock frames [from, to), or flag them "not OK" for the agent to redo (`state` "redo", with a note):
 * one update_project; what it covers of other sections is taken out of them.
 */
export function planMarkSection(project: Project, from: number, to: number, state: Section["state"], note?: string): { readonly ops: ReadonlyArray<Op>; readonly id: string; readonly summary: string } | { readonly problem: string } {
  const a = Math.max(0, Math.round(Math.min(from, to)));
  const b = Math.round(Math.max(from, to));
  if (b <= a) return { problem: "Mark a stretch first: set In and Out (I and O) around it." };
  const taken = new Set<string>();
  const kept = without(project, project.sections ?? [], a, b, taken);
  const id = newId(project, taken);
  const text = note?.trim().slice(0, 500);
  const section: Section = { id, start: a, end: b, state, ...(text ? { note: text } : {}) };
  return { ops: edit([...kept, section]), id, summary: state === "locked" ? "Locked a section" : "Marked a section to redo" };
}

/** Unlock or unflag a section: it goes. */
export function planRemoveSection(project: Project, id: string): { readonly ops: ReadonlyArray<Op>; readonly summary: string } | { readonly problem: string } {
  const s = (project.sections ?? []).find((x) => x.id === id);
  if (!s) return { problem: "That section isn't marked any more." };
  return { ops: edit((project.sections ?? []).filter((x) => x.id !== id)), summary: s.state === "locked" ? "Unlocked a section" : "Removed a redo mark" };
}

/**
 * "Lock the rest": everything from 0 to the end of the timeline that isn't flagged for redo is
 * locked (the existing locks are kept as they are), so the agent can only change what was flagged.
 */
export function planLockRest(project: Project): { readonly ops: ReadonlyArray<Op>; readonly summary: string } | { readonly problem: string } {
  const end = layout(project).duration;
  const sections = [...(project.sections ?? [])].sort((a, b) => a.start - b.start);
  const out: Array<Section> = [...sections];
  const taken = new Set<string>();
  let at = 0;
  for (const s of [...sections, { start: end, end }]) {
    if (s.start > at) out.push({ id: newId(project, taken), start: at, end: s.start, state: "locked" });
    at = Math.max(at, s.end);
  }
  if (out.length === sections.length) return { problem: "Everything is marked already." };
  return { ops: edit(out), summary: "Locked everything not marked to redo" };
}

/** The sections for the agent, one line each (empty when there are none). */
export function sectionsNote(project: Project): string {
  const fps = project.canvas.fps;
  const lines = [...(project.sections ?? [])]
    .sort((a, b) => a.start - b.start)
    .map((s) => `${s.id}: frames ${s.start}–${s.end} (${secs(s.start, fps)}–${secs(s.end, fps)}) ${s.state === "locked" ? "LOCKED: nothing there may move or change; edits that would are refused" : `NOT OK, redo it${s.note ? `: ${s.note}` : ""}`}`);
  return lines.join("\n");
}
