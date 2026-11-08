// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { MAIN_ITEM_TYPES, mainStarts } from "@/app/lib/editor/core/timeline/rules";
import { layout } from "@/app/lib/editor/core/timeline/timeline";
import type { Project, Item, Op, Id, Track } from "@/app/lib/editor/core/timeline/types";
import { locate, lockProblem } from "@/app/lib/editor/core/timeline/arrange/retime";
import { mainSlot, takenIds, isFree, freeAudioTrack, uniqueIn, nextName, tryOps } from "@/app/lib/editor/core/timeline/arrange/place";

/** A take's picture (D215): the clip a "Put on Main" puts on the main track when `item` is one of its sounds. */
function takePicture(project: Pick<Project, "tracks" | "media">, item: Item): Item | undefined {
  if (MAIN_ITEM_TYPES.has(item.type)) return item;
  if (item.linkedTo) return undefined;
  if (!item.group) return undefined;
  const mates = project.tracks.flatMap((t) => (t.role === "main" ? [] : t.items.filter((i) => i.group === item.group && i.type === "clip" && i.id !== item.id).map((i) => ({ i, t }))));
  const named = (re: RegExp) => mates.find(({ i, t }) => re.test(t.name) || re.test(project.media.find((m) => m.id === i.media)?.name ?? ""))?.i;
  return named(/screen/i) ?? named(/camera/i) ?? mates[0]?.i;
}

export interface PutOnMainPlan {
  readonly ops: ReadonlyArray<Op>;
  readonly summary: string;
  /** The item that went on Main, and its index there. */
  readonly itemId: Id;
  readonly index: number;
  /** Its parts that now follow it (linkedTo). */
  readonly linked: ReadonlyArray<Id>;
}

/**
 * "Put on Main" (D215): a picture (clip, element, graphic or text) off the main track goes into the
 * main track at the cut nearest `frame` (default: where its middle is), which ripples what comes
 * after. A take's parts (its group: the microphone, this Mac's sound, the other picture) that play
 * over exactly its stretch become linked to it (D123's linkedTo, the in offsets kept): they stay on
 * their tracks, start where it starts on Main and follow its moves, trims, ripple, splits and
 * transcript cuts. A part whose track isn't free there goes on another free track of its kind (a new
 * one when none is). Group members that aren't parts (other lengths, text) stay as they are. A sound
 * of a take picks the take's picture (the screen, else the camera). One edit; the track the picture
 * leaves empty goes.
 */
export function planPutOnMain(project: Project, itemId: Id, at: { readonly frame?: number | undefined } = {}): PutOnMainPlan | { readonly problem: string } {
  const found = locate(project, itemId);
  if (!found) return { problem: "That item isn't in the project any more." };
  const main = project.tracks.find((t) => t.role === "main");
  if (!main) return { problem: "This project has no main track." };
  if (found.track.role === "main") return { problem: "It's already on the main track." };
  const picture = takePicture(project, found.item);
  if (!picture) return { problem: found.item.type === "captions" ? "Captions follow the speech: they stay on their own track." : "A sound stays on an audio track: put its picture on the main track." };
  const placed = locate(project, picture.id)!;
  const locked = lockProblem(placed.track, picture);
  if (locked) return { problem: locked };
  if (main.locked) return { problem: `The ${main.name} track is locked. Unlock it to add to it.` };
  const { starts } = layout(project);
  const start = starts.get(picture.id) ?? 0;
  const desired = at.frame !== undefined && Number.isFinite(at.frame) ? Math.max(0, at.frame) : start;
  const index = mainSlot(main.items, desired + picture.duration / 2);
  const newStart = index > 0 ? mainStarts(main)[index - 1]! + main.items[index - 1]!.duration : 0;
  const ops: Array<Op> = [];
  const before = main.items[index];
  if (before?.transitionIn) ops.push({ op: "set_transition", item: before.id, transition: null });
  ops.push({ op: "move_item", item: picture.id, track: main.id, index });
  if (picture.group || picture.linkedTo) ops.push({ op: "update_item", item: picture.id, set: { ...(picture.group ? { group: null } : {}), ...(picture.linkedTo ? { linkedTo: null } : {}) } });
  // The take's parts: same stretch, a clip or a sound, unlocked.
  const members = picture.group ? project.tracks.flatMap((t) => (t.role === "main" ? [] : t.items.filter((i) => i.group === picture.group && i.id !== picture.id).map((i) => ({ item: i, track: t })))) : [];
  const parts = members.filter(({ item, track }) => (item.type === "audio" || item.type === "clip") && (starts.get(item.id) ?? 0) === start && item.duration === picture.duration && !item.locked && !track.locked);
  const rest = members.filter((m) => !parts.includes(m));
  // Where each part goes: its own track when free at the new start, else another of its kind, else a new one.
  let tracks: Array<Track> = project.tracks.map((t) => ({ ...t, items: t.items.filter((i) => i.id !== picture.id && !parts.some((p) => p.item.id === i.id)) }));
  const taken = takenIds(project);
  const linked: Array<Id> = [];
  const span = picture.duration;
  for (const { item, track } of parts) {
    const audio = item.type === "audio";
    let target = tracks.find((t) => t.id === track.id)!;
    if (!isFree(target, newStart, span)) {
      const mi = tracks.findIndex((t) => t.role === "main");
      const other = audio ? freeAudioTrack({ tracks }, newStart, span) : tracks.slice(0, Math.max(0, mi)).reverse().find((t) => t.kind === "video" && t.role !== "main" && !t.locked && isFree(t, newStart, span));
      if (other) target = other;
      else {
        const id = uniqueIn(audio ? "audio-track" : "video-track", taken);
        const name = nextName({ tracks }, audio ? "Audio" : "Video");
        const index = audio ? tracks.length : Math.max(0, mi);
        ops.push({ op: "add_track", track: { id, kind: audio ? "audio" : "video", name }, index });
        target = { id, kind: audio ? "audio" : "video", name, items: [] };
        tracks.splice(index, 0, target);
      }
    }
    ops.push({ op: "move_item", item: item.id, ...(target.id !== track.id ? { track: target.id } : {}), start: newStart });
    ops.push({ op: "update_item", item: item.id, set: { linkedTo: picture.id, group: null } });
    const moved = { ...item, start: newStart };
    tracks = tracks.map((t) => (t.id === target.id ? { ...t, items: [...t.items, moved] } : t));
    linked.push(item.id);
  }
  // A group of one is no group.
  if (picture.group && rest.length === 1) ops.push({ op: "update_item", item: rest[0]!.item.id, set: { group: null } });
  // The track it leaves empty goes (as Bring forward / Send backward does).
  const left = tracks.find((t) => t.id === placed.track.id);
  if (left && !left.items.length && !left.locked && left.role !== "main") ops.push({ op: "delete_track", track: left.id });
  const check = tryOps(project, ops);
  if ("problem" in check) return check;
  return { ops, summary: `Put ${picture.name ?? picture.id} on Main`, itemId: picture.id, index, linked };
}
