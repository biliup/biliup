// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The project as the renderer reads it, the timeline maths it borrows from the editor core (D39: the
// preview, export and agent share one implementation), and the edit ops the editor sends for
// "add to timeline". The ops go through project.edit, so the server validates them and they undo.
import { mainEnd as mainEndOf } from "@/app/lib/editor/core/timeline/rules";
import { SOLID } from "@/app/lib/editor/core/visual/render";
import { TEXT_STYLES, type TextPreset } from "@/app/lib/editor/core/text/looks";
import { planInsertText } from "@/app/lib/editor/core/text/textPlan";
import { componentInfo } from "@/app/lib/editor/core/visual/components";
import { layout } from "@/app/lib/editor/core/timeline/timeline";
import { planPlaceMedia, takenIds } from "@/app/lib/editor/core/timeline/arrange";
import { fitFadeOut, videoEnd } from "@/app/lib/editor/core/timeline/mediaPlan";
import { defaultFrames, defaultSeconds } from "@/app/ui/editor/settings/defaults";
import {editorText} from '@/app/lib/editor/i18n';

import type { Item, Media, Op, Project, Track } from "@/app/lib/editor/core/timeline/types";

// Preview, export and editor actions use the same in-project timeline types.
export type { Item, Track };
export type MediaItem = Media;
export type ProjectModel = Project;

export interface Layout {
  readonly starts: Map<string, number>;
  /** Where the last item ends, in frames; 0 for an empty project. */
  readonly end: number;
  /** Frames playback runs over: [0, frames - 1]. At least 1, so an empty project has frame 0. (The playhead can also rest at `end`.) */
  readonly frames: number;
}

// Snapshots are immutable, so the layout of one project object never changes.
const layouts = new WeakMap<ProjectModel, Layout>();
export function layoutOf(project: ProjectModel): Layout {
  let l = layouts.get(project);
  if (!l) {
    const { starts, duration } = layout(project);
    const end = project.tracks.some((t) => t.items.length) ? duration : 0;
    l = { starts, end, frames: Math.max(1, end) };
    layouts.set(project, l);
  }
  return l;
}

/**
 * Tracks top to bottom as the timeline shows them (D28, CapCut order): the visual tracks in
 * project order, which is also how the preview stacks them (core visualsAt draws the first track
 * on top), then audio. New overlay and text tracks go in front of Main, so they sit above it.
 */
export const displayTracks = (project: ProjectModel) => [
  ...project.tracks.filter((t) => t.kind !== "audio"),
  ...project.tracks.filter((t) => t.kind === "audio"),
];

/** An id no media, track, item or scene uses yet: "clip-1", "clip-2", … */
export function uniqueId(project: ProjectModel, base: string, taken = new Set<string>()) {
  const ids = new Set([
    ...taken,
    ...project.media.map((m) => m.id),
    ...project.tracks.flatMap((t) => [t.id, ...t.items.map((i) => i.id)]),
    ...(project.scenes ?? []).map((s) => s.id),
  ]);
  for (let n = 1; ; n++) if (!ids.has(`${base}-${n}`)) return `${base}-${n}`;
}

/** `show`: where the playhead goes after the add (default `start`), e.g. past a text's in-move so it's on screen (F20). */
export type Planned = { readonly ops: ReadonlyArray<Op>; readonly summary: string; readonly itemId: string; readonly start: number; readonly show?: number } | { readonly problem: string };

/**
 * "Add to timeline" for a library tile. Video and images are appended to the end of the magnetic
 * main track (D17: no gaps, nothing shifts); audio goes on the first unlocked audio track (one is
 * created if there is none) at the playhead if the track is free there for the sound's whole
 * length, else after its last item (D54).
 */
export function planAddMedia(project: ProjectModel, mediaId: string, frame = 0): Planned {
  const media = project.media.find((m) => m.id === mediaId);
  if (!media) return { problem: "That media isn't in the project any more." };
  const fps = project.canvas.fps;
  const frames = media.probe?.frames;
  if (media.kind !== "image" && !frames) return { problem: `${media.name} has no known length, so it can't go on the timeline.` };
  if (media.kind === "audio") return planAudio(project, media, frames!, frame);
  const main = project.tracks.find((t) => t.role === "main");
  if (!main) return { problem: "This project has no main track." };
  if (main.locked) return { problem: "The main track is locked. Unlock it to add clips." };
  const mainEnd = mainEndOf(project);
  const itemId = uniqueId(project, "clip");
  const duration = media.kind === "image" ? defaultFrames("still", fps) : frames!;
  return {
    ops: [{ op: "insert_item", track: main.id, item: { id: itemId, type: "clip", media: media.id, in: 0, duration } }],
    summary: `Add ${media.name} to the timeline`,
    itemId,
    start: mainEnd,
  };
}

/**
 * "Add at playhead" for a generated result (D99): a sound goes where "Add to timeline" puts it (the
 * playhead on the first free audio track); a picture (the default photo length, F30) or a video starts at the playhead on the
 * nearest video track above the main track that is free for its whole length, else on a new
 * "Generated" track just above the main track. Nothing on the main track moves.
 */
export function planAddAtPlayhead(project: ProjectModel, mediaId: string, frame: number): Planned {
  const media = project.media.find((m) => m.id === mediaId);
  if (!media) return { problem: "That media isn't in the project any more." };
  const fps = project.canvas.fps;
  const frames = media.probe?.frames;
  if (media.kind !== "image" && !frames) return { problem: `${media.name} has no known length, so it can't go on the timeline.` };
  if (media.kind === "audio") return planAudio(project, media, frames!, frame);
  const { starts } = layoutOf(project);
  const start = Math.max(0, Math.floor(frame));
  const duration = media.kind === "image" ? defaultFrames("still", fps) : frames!;
  const mainIndex = project.tracks.findIndex((t) => t.role === "main");
  const above = project.tracks.slice(0, Math.max(0, mainIndex)).reverse();
  const free = (t: Track) => t.items.every((i) => (starts.get(i.id) ?? 0) + i.duration <= start || start + duration <= (starts.get(i.id) ?? 0));
  const ops: Array<Op> = [];
  const taken = new Set<string>();
  let track = above.find((t) => t.kind === "video" && !t.locked && free(t));
  if (!track) {
    const id = uniqueId(project, "generated", taken);
    taken.add(id);
    ops.push({ op: "add_track", track: { id, kind: "video", name: "Generated" }, index: Math.max(0, mainIndex) });
    track = { id, kind: "video", name: "Generated", items: [] };
  }
  const itemId = uniqueId(project, "clip", taken);
  ops.push({ op: "insert_item", track: track.id, item: { id: itemId, type: "clip", media: media.id, in: 0, start, duration } });
  return { ops, summary: `Add ${media.name} at the playhead`, itemId, start };
}

/**
 * An audio item for `media` (D124): at the playhead on the first unlocked audio track that is free
 * there for the whole sound, else on a new "Audio N" track, so music, effects and voice overlap.
 */
function planAudio(project: ProjectModel, media: Media, _length: number, frame: number): Planned {
  return planDropMedia(project, media.id, { frame });
}

/**
 * Media dropped on the timeline (D122): at the drop frame on the lane it's over when that lane takes
 * it (main: the slot nearest the drop; a free track: the nearest free spot; a video file on an audio
 * lane: its sound), else where core planPlaceMedia puts it. One edit.
 */
export function planDropMedia(project: ProjectModel, mediaId: string, at: { readonly frame: number; readonly track?: string | undefined }): Planned {
  const media = project.media.find((m) => m.id === mediaId);
  if (!media) return { problem: "That media isn't in the project any more." };
  const plan = planPlaceMedia(project, [media], { ...at, imageSeconds: defaultSeconds("still") }, takenIds(project));
  if ("problem" in plan) return plan;
  const first = plan.placed[0]!;
  return { ops: plan.ops, summary: `Add ${media.name} to the timeline`, itemId: first.itemId, start: first.start };
}

/** Music shorter than this before the video ends isn't worth adding (D91, D134). */
const MIN_MUSIC_SECONDS = 3;

/**
 * A sound from the library (D134) on the timeline, in one edit: its add_media with its credits in
 * `source` (unless the same library sound is in the project already) and an audio item at the
 * playhead on the first unlocked audio track free for its stretch, else a new track (D124: sounds
 * overlap on tracks of their own). Music plays from the playhead to the video's end at most, with
 * fades; a sound effect goes in whole, without fades.
 */
export function planAddLibrarySound(project: ProjectModel, media: Media, music: boolean, frame: number): Planned {
  const fps = project.canvas.fps;
  const frames = media.probe?.frames;
  if (media.kind !== 'audio' || frames === undefined || !Number.isSafeInteger(frames) || frames < 1) return { problem: 'This sound has no usable audio duration.' };
  if (!Number.isFinite(frame)) return { problem: 'The playhead position is invalid.' };
  const ops: Array<Op> = [];
  const taken = new Set<string>([media.id]);
  const existing=project.media.find(entry=>entry.id===media.id);
  // A file imported through Media keeps its registered identity and gains its library credits.
  if(!existing)ops.push({op:'add_media',media});
  else if(existing.source!==media.source||existing.name!==media.name)ops.push({op:'update_media',media:media.id,set:{source:media.source,name:media.name}});
  const { starts } = layoutOf(project);
  const end = videoEnd(project);
  const start = Math.max(0, Math.floor(frame));
  let duration = frames;
  if (music && end > 0) {
    duration = Math.min(frames, end - start);
    if (duration < MIN_MUSIC_SECONDS * fps) {
      return { problem: start >= end ? 'The playhead is at the end of the video. Move it to where the music should start, then add it.'
        : frames < MIN_MUSIC_SECONDS * fps ? 'This audio is shorter than 3 seconds. Add it as a sound effect.'
        : editorText('Only {0} s of the video are left after the playhead. Move it earlier to add music there.',{0:((end-start)/fps).toFixed(1)}) };
    }
  }
  const freeOn = (t: Track) => t.items.every((i) => (starts.get(i.id) ?? 0) + i.duration <= start || start + duration <= (starts.get(i.id) ?? 0));
  let target = project.tracks.find((t) => t.kind === "audio" && t.role !== "main" && !t.locked && freeOn(t));
  if (!target) {
    const name = editorText(music ? "Music" : "Sound effects");
    const id = uniqueId(project, music ? "music" : "sfx", taken);
    taken.add(id);
    ops.push({ op: "add_track", track: { id, kind: "audio", name } });
    target = { id, kind: "audio", name, items: [] };
  }
  const fadeIn = music ? Math.min(Math.round(fps), Math.floor(duration / 4)) : 0;
  const fadeOut = music ? fitFadeOut(duration, fps) : 0;
  const itemId = uniqueId(project, music ? "audio" : "sound", taken);
  ops.push({ op: "insert_item", track: target.id, item: { id: itemId, type: "audio", media: media.id, in: 0, duration, start, ...(fadeIn > 0 ? { fadeIn } : {}), ...(fadeOut > 0 ? { fadeOut } : {}) } });
  return { ops, summary: editorText(music ? 'Add {0} from the music library' : 'Add {0} from the sound library', { 0: media.name }), itemId, start };
}

/**
 * The Text tab's presets (D80): core TEXT_PRESETS, built-in text-style looks from their manifests
 * (the three from before first, same ids), the same looks the renderer falls back to.
 */
export { TEXT_PRESETS } from "@/app/lib/editor/core/text/looks";
export type { TextPreset } from "@/app/lib/editor/core/text/looks";

/**
 * A text item in a preset's look at the playhead, the default text length long (F30, 3 s), on the top-most text track that is
 * free for that whole stretch (core planInsertText; the looks it comes with on the next free
 * ones). If none is, a new text track goes on top (above every other track). One edit either
 * way, so one undo removes it all.
 */
export function planAddText(project: ProjectModel, preset: TextPreset, frame: number, at?: { readonly x: number; readonly y: number }): Planned {
  const start = Math.max(0, Math.floor(frame));
  const duration = defaultFrames("text", project.canvas.fps);
  // Dropped on the canvas: its first line centred on the drop point's height; a look that spans
  // the safe area keeps its x and width, others start at the drop point.
  const look = TEXT_STYLES[preset.module];
  const y = at && look ? Math.round(at.y - (project.canvas.height * look.size * look.lineHeight) / 2) : undefined;
  const transform = at && look ? (look.span ? { y: y! } : { x: Math.round(at.x), y: y! }) : undefined;
  const plan = planInsertText(project, { look: preset.module, text: preset.text, start, duration, idBase: preset.id, ...(transform ? { transform } : {}) });
  if ("problem" in plan) return plan;
  // F20 (D158): the playhead goes where the new text is fully on screen (after its in-move).
  const added = plan.ops.find((o) => o.op === "insert_item" && o.item.id === plan.itemId) as { item: Item } | undefined;
  const settle = Math.min(duration - 1, added?.item.animation?.in?.duration ?? 0);
  return { ops: plan.ops, summary: `Add ${preset.label.toLowerCase()}`, itemId: plan.itemId, start, ...(settle > 0 ? { show: start + settle } : {}) };
}

/** A background's colours in words: "Background #0B1D3A", "Gradient #0B1D3A → #3B1E54". */
const colourName = (value: unknown) => (typeof value === "string" ? value : "default");
export const backgroundLabel = (item: Item) =>
  item.props?.to !== undefined ? `${item.props?.gradient === "radial" ? "Radial gradient" : "Gradient"} ${colourName(item.props?.color)} → ${colourName(item.props.to)}` : `Background ${colourName(item.props?.color)}`;

/** What a timeline item is called: its text, a background's colour, its media's name, or its id. */
export function itemLabel(project: ProjectModel, item: Item) {
  if (item.type === "text" && typeof item.text === "string") return item.text.replace(/\[\[|\]\]|\*\*|`/g, "").replace(/\n/g, " ");
  if (item.type === "component" && item.component === SOLID && !item.name) return backgroundLabel(item);
  // A component (D76): its name, and its title or first line when it has one.
  const component = item.type === "component" && !item.name ? componentInfo(item.component) : undefined;
  if (component) {
    const p = item.props ?? {};
    const words = [p.title, p.code, Array.isArray(p.lines) ? (p.lines[0] as { text?: unknown } | undefined)?.text : undefined, p.device].find((v): v is string => typeof v === "string" && v.trim() !== "");
    return words ? `${component.name}: ${words}` : component.name;
  }
  if (item.type === "graphic" && !item.name && item.src) return `Graphic: ${item.src.replace(/^graphics\//, "")}`;
  if (item.type === "captions" && !item.name) {
    const source = project.media.find((m) => m.id === item.source);
    return source ? `Captions of ${source.name}` : "Captions";
  }
  const media = item.media ? project.media.find((m) => m.id === item.media) : undefined;
  return item.name ?? media?.name ?? item.id;
}
