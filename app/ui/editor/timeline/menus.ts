import { editorText } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// What the timeline's right-click menus hold (T7, D126): on items (the selection when the item is
// in it), on track heads and on empty timeline. Every entry is an existing action (one edit, one
// ⌘Z) or a UI step (rename in place, the Inspector's custom speed).
import { soundMedia } from "@/app/lib/editor/core/audio/audio";
import { planFitSoundToVideo } from '@/app/ui/editor/audio/sound';
import { canReplaceMedia, SPEED_PRESETS, speedLabel } from "@/app/lib/editor/core/timeline/arrange";
import { planLockRest, planMarkSection, planRemoveSection } from "@/app/lib/editor/core/timeline/sections";

import { clipboardItems, copySelection, cutSelection, deleteSelection, duplicate, paste } from "@/app/ui/editor/workspace/keys";
import { getPlayback, playback } from "@/app/ui/editor/preview/playback";
import { itemLabel, type ProjectModel, type Track } from "@/app/lib/editor/state/project";
import { actions, getState, projectOf } from "@/app/lib/editor/state/store";
import type { MenuItems } from "@/app/ui/editor/timeline/ContextMenu";
import { findItem, trackFlag, type Plan } from "@/app/ui/editor/timeline/edit";
import { keyHint } from "@/app/ui/editor/workspace/keymap";
import { timelineCommand } from '@/app/lib/editor/state/store/edits';

/** A menu plan uses the revision it was drawn from, even when another edit is queued. */
export function commitMenuEdit(project: ProjectModel, plan: Plan | { readonly problem: string }) {
  const snapshot = getState().snapshot;
  if (!snapshot || projectOf(snapshot) !== project) return;
  void timelineCommand(plan);
}

export interface MenuUi {
  /** Rename the item in place on its clip. */
  readonly renameItem: (id: string) => void;
  readonly renameTrack: (id: string) => void;
  /** The Inspector's custom speed field. */
  readonly customSpeed: (id: string) => void;
}

/** The ids a menu on `itemId` acts on: the selection when the item is in it, else just the item (which gets selected). */
export function menuSelection(itemId: string): ReadonlyArray<string> {
  const { selected, also } = getPlayback();
  const current = selected === undefined ? [] : [selected, ...also];
  if (current.includes(itemId)) return current;
  playback.select(itemId);
  return [itemId];
}

export function itemMenu(project: ProjectModel, itemId: string, ui: MenuUi, online: boolean): MenuItems {
  const placed = findItem(project, itemId);
  if (!placed) return [];
  const ids = menuSelection(itemId);
  const many = ids.length > 1;
  const { item, track, start } = placed;
  const locked = !!(track.locked || item.locked);
  const off = !online;
  const frame = getPlayback().frame;
  const media = item.media ? project.media.find((m) => m.id === item.media) : undefined;
  const timed = (item.type === "clip" || item.type === "audio") && media && media.kind !== "image";
  const sound = soundMedia(project, item);
  const speed = item.speed ?? 1;
  const others = project.media.filter(m => m.id !== item.media && canReplaceMedia(item, m));
  return [
    // D141: the item as a chip in the Agent tab's composer, with its time range.
    { id: "add-to-chat", label: many ? editorText('Add {0} items to chat', { 0: ids.length }) : editorText("Add to chat"), ...keyHint("addToChat"), run: () => ids.forEach((id) => actions.addItemToChat(id)) },
    "-",
    { id: "split", get label() { return editorText("Split"); }, ...keyHint('split'), disabled: off || many || locked || !(frame > start && frame < start + item.duration) || item.type === "captions", run: () => void actions.split(item.id, frame) },
    "-",
    { id: "copy", get label() { return editorText("Copy"); }, ...keyHint('copy'), disabled: item.type === "captions", run: () => void copySelection(ids) },
    { id: "cut", get label() { return editorText("Cut selection"); }, ...keyHint('cut'), disabled: off || locked || item.type === "captions", run: () => void cutSelection(ids) },
    { id: "paste", get label() { return editorText("Paste"); }, ...keyHint('paste'), disabled: off || !clipboardItems().length, run: () => void paste(frame) },
    { id: "duplicate", get label() { return editorText("Duplicate"); }, ...keyHint('duplicate'), disabled: off || locked || item.type === "captions", run: () => void duplicate(ids) },
    "-",
    { id: "delete", label: many ? editorText('Delete {0} items', { 0: ids.length }) : editorText("Delete"), ...keyHint('delete'), danger: true, disabled: off || locked, run: () => void deleteSelection(ids) },
    { id: "ripple-delete", get label() { return editorText("Ripple delete"); }, ...keyHint('rippleDelete'), danger: true, disabled: off || locked, run: () => void actions.rippleDelete(ids) },
    "-",
    // D215: a picture off Main goes on it (a take from any of its parts: its picture, sounds linked under it).
    ...(track.role !== "main" && item.type !== "captions" && !item.linkedTo && (item.type !== "audio" || !!item.group)
      ? [{ id: "put-on-main", get label() { return editorText("Put on Main"); }, disabled: off || locked || (item.type === "audio" && !takeHasPicture(project, item)), run: () => void actions.putOnMain(item.id) }]
      : []),
    ...(item.type === "clip" && media?.kind === "video" && media.probe?.hasAudio
      ? [{ id: "detach-audio", label: item.mute ? "Audio detached" : "Detach audio", disabled: off || many || locked || !sound, run: () => void actions.detachAudio(item.id) }]
      : []),
    ...(timed
      ? [
          {
            id: "speed",
            label: `${editorText('Speed')}${speed !== 1 ? `  ${speedLabel(speed)}` : ""}`,
            disabled: off || many || locked,
            submenu: [
              ...SPEED_PRESETS.map((s) => ({ id: `speed-${s}`, label: speedLabel(s), checked: Math.abs(s - speed) < 1e-6, run: () => void actions.setSpeed(item.id, s) })),
              "-" as const,
              { id: "speed-custom", get label() { return editorText("Custom…"); }, run: () => ui.customSpeed(item.id) },
              { id: "speed-pitch", get label() { return editorText("Keep pitch"); }, checked: item.keepPitch !== false, run: () => void actions.setSpeed(item.id, speed, item.keepPitch === false) },
            ],
          },
        ]
      : []),
    // F27 (D169): music trimmed or looped to the end of the picture, with a fade out.
    ...(item.type === "audio" && track.role !== "main" ? [{ id: "fit-to-video", get label() { return editorText("Fit to video"); }, disabled: off || many || locked, run: () => commitMenuEdit(project, planFitSoundToVideo(project, item.id)) }] : []),
    { id: "rename", get label() { return editorText("Rename"); }, disabled: off || many || locked || item.type === "captions", run: () => ui.renameItem(item.id) },
    ...(item.media && (item.type === "clip" || item.type === "audio")
      ? [
          {
            id: "replace-media",
            get label() { return editorText("Replace media"); },
            disabled: off || many || locked || !others.length,
            submenu: others.length ? others.map((m) => ({ id: `replace-${m.id}`, label: m.name, run: () => void actions.replaceMedia(item.id, m.id) })) : [{ id: "replace-none", get label() { return editorText("No other media"); }, disabled: true }],
          },
          { id: "show-in-media", get label() { return editorText("Show in Media"); }, disabled: !media, run: () => media && actions.showInMedia(media.id) },
        ]
      : []),
    "-",
    ...((item.type === "clip" && media?.kind === "video" && media.probe?.hasAudio) || item.type === "audio"
      ? [{ id: "mute", label: item.mute ? "Unmute" : "Mute", checked: !!item.mute, disabled: off || many || locked, run: () => void actions.toggleItem(item.id, "mute") }]
      : []),
    { id: "lock", label: item.locked ? "Unlock" : "Lock", checked: !!item.locked, disabled: off || many || track.locked, run: () => void actions.toggleItem(item.id, "locked") },
    // D157: a kicker and its title (or any items grouped here) move, trim and delete together.
    ...(item.group
      ? ["-" as const, { id: "ungroup", get label() { return editorText("Ungroup"); }, disabled: off || locked, run: () => void actions.grouping(ids, false) }]
      : many && track.role !== "main"
        ? ["-" as const, { id: "group", label: editorText('Group {0} items', { 0: ids.length }), disabled: off || locked, run: () => void actions.grouping(ids, true) }]
        : []),
  ];
}

/** A sound's group has a picture to put on Main (D215). */
const takeHasPicture = (project: ProjectModel, item: { readonly group?: string | undefined; readonly id: string }) => !!item.group && project.tracks.some((t) => t.role !== "main" && t.items.some((i) => i.group === item.group && i.id !== item.id && i.type === "clip"));

const KIND_NAMES = { video: "Video track", text: "Text track", audio: "Audio track" } as const;

export function trackMenu(project: ProjectModel, track: Track, ui: MenuUi, online: boolean): MenuItems {
  const off = !online;
  const audio = track.kind === "audio";
  const sound = audio || track.items.some((i) => soundMedia(project, i));
  return [
    // D264 (#77): the track as a chip in the Agent tab's composer.
    { id: "add-to-chat", get label() { return editorText("Add to chat"); }, run: () => void actions.addTrackToChat(track.id) },
    "-",
    { id: "track-rename", get label() { return editorText("Rename track"); }, disabled: off || track.locked, run: () => ui.renameTrack(track.id) },
    "-",
    ...(!audio ? [{ id: "track-hide", label: trackFlag(track, "hidden") ? "Show" : "Hide", checked: trackFlag(track, "hidden"), disabled: off, run: () => void actions.toggleTrack(track.id, "hidden") }] : []),
    ...(sound ? [{ id: "track-mute", label: trackFlag(track, "mute") ? "Unmute" : "Mute", checked: trackFlag(track, "mute"), disabled: off, run: () => void actions.toggleTrack(track.id, "mute") }] : []),
    ...(sound ? [{ id: "track-solo", get label() { return editorText("Solo"); }, checked: trackFlag(track, "solo"), disabled: off, run: () => void actions.toggleTrack(track.id, "solo") }] : []),
    { id: "track-lock", label: track.locked ? "Unlock" : "Lock", checked: !!track.locked, disabled: off, run: () => void actions.toggleTrack(track.id, "locked") },
    "-",
    { id: "track-select", get label() { return editorText("Select all on this track"); }, disabled: !track.items.length, run: () => playback.selectMany(track.items.filter((i) => i.type !== "captions").map((i) => i.id)) },
    ...addTrackEntries(project, off, track),
    ...(track.role !== "main"
      ? [{ id: "track-delete", get label() { return editorText("Delete track"); }, danger: true, disabled: off || track.locked || track.items.some((i) => i.locked), run: () => void actions.deleteTrack(track.id) }]
      : []),
  ];
}

function addTrackEntries(project: ProjectModel, off: boolean, near?: Track): MenuItems {
  return [
    {
      id: "add-track",
      get label() { return editorText("Add track"); },
      disabled: off,
      submenu: (["video", "text", "audio"] as const).map((kind) => ({ id: `add-track-${kind}`, label: editorText(KIND_NAMES[kind]), run: () => void actions.addTrack(kind, near?.id) })),
    },
  ];
}

/** Right-click on empty timeline (a lane's background or below the tracks) at `frame`. */
/** D183: right-click on the ruler or the playhead: that moment, or the In/Out range, into the chat. */
export function rulerMenu(project: ProjectModel, frame: number, marks: { readonly markIn?: number | undefined; readonly markOut?: number | undefined; readonly end: number }, online: boolean): MenuItems {
  const hasRange = marks.markIn !== undefined || marks.markOut !== undefined;
  const start = marks.markIn ?? 0;
  const end = marks.markOut ?? marks.end;
  const redo = (project.sections ?? []).some((s) => s.state === "redo");
  return [
    // Issue #118: start playback where the user pointed, not where the playhead was.
    { id: "play-from-here", get label() { return editorText("Play from here"); }, run: () => playback.playFrom(frame) },
    { id: "add-marker", get label() { return editorText("Add marker"); }, ...keyHint('addMarker'), disabled: !online || frame > marks.end || !!project.markers?.some(marker => marker.frame === frame), run: () => void actions.addMarker(frame) },
    "-",
    { id: "add-moment-to-chat", get label() { return editorText("Add this moment to chat"); }, run: () => actions.addMomentToChat(frame) },
    ...(hasRange ? [{ id: "add-range-to-chat", get label() { return editorText("Add range to chat"); }, disabled: end <= start, run: () => actions.addRangeToChat(start, end) }] : []),
    // Issue #118: In to Out locked (nothing there may move or change) or flagged for the agent to redo.
    ...(hasRange
      ? [
          "-" as const,
          { id: "lock-range", get label() { return editorText("Lock In to Out"); }, disabled: !online || end <= start, run: () => commitMenuEdit(project, planMarkSection(project, start, end, "locked")) },
          { id: "redo-range", get label() { return editorText("Mark In to Out as not OK"); }, disabled: !online || end <= start, run: () => commitMenuEdit(project, planMarkSection(project, start, end, "redo")) },
        ]
      : []),
    ...(redo ? [{ id: "lock-rest", get label() { return editorText("Lock everything not marked to redo"); }, disabled: !online, run: () => commitMenuEdit(project, planLockRest(project)) }] : []),
  ];
}

/** Issue #118: right-click a marked section on the ruler: unlock or unflag it, ask the agent about it, lock the rest. */
export function sectionMenu(project: ProjectModel, id: string, online: boolean): MenuItems {
  const s = project.sections?.find((x) => x.id === id);
  if (!s) return [];
  return [
    { id: "section-play", get label() { return editorText("Play from its start"); }, run: () => playback.playFrom(s.start) },
    { id: "section-add-to-chat", get label() { return editorText("Add to chat"); }, run: () => actions.addRangeToChat(s.start, s.end) },
    "-",
    s.state === "locked"
      ? { id: "section-unlock", get label() { return editorText("Unlock"); }, disabled: !online, run: () => commitMenuEdit(project, planRemoveSection(project, id)) }
      : { id: "section-unflag", get label() { return editorText("Mark as OK"); }, disabled: !online, run: () => commitMenuEdit(project, planRemoveSection(project, id)) },
    ...(s.state === "redo" ? [{ id: "section-lock-rest", get label() { return editorText("Lock everything not marked to redo"); }, disabled: !online, run: () => commitMenuEdit(project, planLockRest(project)) }] : []),
  ];
}

export function emptyMenu(project: ProjectModel, frame: number, track: Track | undefined, online: boolean): MenuItems {
  const off = !online;
  const label = track ? itemsLabel(project, track) : undefined;
  return [
    { id: "play-from-here", get label() { return editorText("Play from here"); }, run: () => playback.playFrom(frame) },
    "-",
    { id: "paste-here", get label() { return editorText("Paste here"); }, ...keyHint('paste'), disabled: off || !clipboardItems().length, run: () => void paste(frame) },
    { id: "split-playhead", get label() { return editorText("Split at the playhead"); }, ...keyHint('split'), disabled: off, run: () => void actions.split() },
    "-",
    { id: "select-all", get label() { return editorText("Select all"); }, ...keyHint('selectAll'), run: () => playback.selectMany(project.tracks.flatMap((t) => t.items.filter((i) => i.type !== "captions").map((i) => i.id))) },
    ...(track && label ? [{ id: "track-select", get label() { return editorText("Select all on this track"); }, run: () => playback.selectMany(track.items.filter((i) => i.type !== "captions").map((i) => i.id)) }] : []),
    "-",
    ...addTrackEntries(project, off, track),
  ];
}

const itemsLabel = (project: ProjectModel, track: Track) => (track.items.length ? itemLabel(project, track.items[0]!) : undefined);
