// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { applyEdit, createSession, EditError, sliceItemKeys } from "@/app/lib/editor/core/timeline/apply-ops";
import {sliceMaskEffects} from '../masks';
import { layout } from "@/app/lib/editor/core/timeline/timeline";
import {timelineProblems} from '@/app/lib/editor/core/timeline/rules';
import { audioSpan, soundMedia, sliceGainKeys } from "@/app/lib/editor/core/audio/audio";
import type { Project, Id, Track, Item, Op } from "@/app/lib/editor/core/timeline/types";
import { freeAudioTrack, isFree, uniqueIn, nextName, tryOps, takenIds } from "@/app/lib/editor/core/timeline/arrange/place";
import { fitChangedTransitions } from './transitions';

export const locate = (project: Pick<Project, "tracks">, id: Id) => {
  for (const track of project.tracks) {
    const index = track.items.findIndex((i) => i.id === id);
    if (index !== -1) return { track, index, item: track.items[index]! };
  }
  return undefined;
};
export const lockProblem = (track: Track, item: Item) => (track.locked ? `The ${track.name} track is locked. Unlock it to edit it.` : item.locked ? "That item is locked." : undefined);

export interface DetachAudioPlan {
  readonly ops: ReadonlyArray<Op>;
  readonly itemId: Id;
  readonly trackId: Id;
  readonly summary: string;
}

/**
 * Detach audio (D123): a clip's sound becomes its own audio item, the clip muted, in one edit.
 * The audio item plays the same media from the same point at the same speed and pitch, starts
 * where the clip starts and takes over the clip's volume, fades and volume keys (the clip's are
 * removed: it's silent now); a held clip's sound stops at its media's end. It goes on the first
 * audio track free for that stretch, else on a new "Audio N" track. Captions and transcript cuts
 * keep following the clip on main (its picture), not the detached sound.
 */
export function planDetachAudio(project: Pick<Project, "tracks" | "media" | "canvas">, itemId: Id, taken: Set<string>, options: { readonly id?: Id; readonly track?: Id } = {}): DetachAudioPlan | { readonly problem: string } {
  const found = locate(project, itemId);
  if (!found) return { problem: "That item isn't in the project any more." };
  const { track, item } = found;
  if (item.type !== "clip") return { problem: "Only a clip's sound can be detached." };
  if (item.mute) return { problem: "This clip's sound is already off (detached or muted)." };
  const media = soundMedia(project, item);
  if (!media) return { problem: "This clip has no sound to detach." };
  const locked = lockProblem(track, item);
  if (locked) return { problem: locked };
  const { starts } = layout(project);
  const start = starts.get(item.id) ?? 0;
  const speed = item.speed ?? 1;
  const from = item.in ?? 0;
  const frames = media.probe?.frames;
  // A held clip (D94) is silent past its media's end, so its sound stops there.
  let duration = item.duration;
  if (frames && !item.loop && from + duration * speed > frames) duration = Math.max(1, Math.floor((frames - from) / speed));
  const ops: Array<Op> = [];
  let target = options.track !== undefined ? project.tracks.find((t) => t.id === options.track) : freeAudioTrack(project, start, duration);
  if (options.track !== undefined) {
    if (!target || target.kind !== "audio" || target.role === "main") return { problem: `There's no audio track "${options.track}".` };
    if (target.locked) return { problem: `The ${target.name} track is locked.` };
    if (!isFree(target, start, duration)) return { problem: `The ${target.name} track isn't free there.` };
  }
  if (!target) {
    const id = uniqueIn("audio-track", taken);
    const name = nextName(project, "Audio");
    ops.push({ op: "add_track", track: { id, kind: "audio", name } });
    target = { id, kind: "audio", name, items: [] };
  }
  const id = options.id ?? uniqueIn("audio", taken);
  if (options.id) taken.add(options.id);
  const clamp = (n: number | undefined) => (n === undefined ? undefined : Math.min(n, duration));
  const fadeIn = clamp(item.fadeIn);
  const fadeOut = clamp(item.fadeOut);
  const fades = (fadeIn ?? 0) + (fadeOut ?? 0) > duration ? {} : { ...(fadeIn ? { fadeIn } : {}), ...(fadeOut ? { fadeOut } : {}) };
  const gainKeys = sliceGainKeys(item.gainKeys, 0, duration);
  const audio: Item = {
    id,
    type: "audio",
    media: media.id,
    in: from,
    start,
    duration,
    ...(speed !== 1 ? { speed } : {}),
    ...(item.keepPitch === false ? { keepPitch: false } : {}),
    ...(item.loop ? { loop: true } : {}),
    ...(item.gainDb !== undefined ? { gainDb: item.gainDb } : {}),
    ...fades,
    ...(gainKeys?.length ? { gainKeys } : {}),
    linkedTo: item.id,
  };
  ops.push({ op: "insert_item", track: target.id, item: audio });
  ops.push({ op: "update_item", item: item.id, set: { mute: true, gainDb: null, fadeIn: null, fadeOut: null, gainKeys: null } });
  return { ops, itemId: id, trackId: target.id, summary: `Detach audio from ${item.name ?? media.name}` };
}

/** Speed presets the editor offers (D125). */
export const SPEED_PRESETS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3, 4] as const;
export const SPEED_RANGE = { min: 0.1, max: 10 } as const;

/** "2×", "0.5×", "1.25×" */
export const speedLabel = (speed: number) => `${Math.round(speed * 100) / 100}×`;

/**
 * Change a clip's or an audio item's speed (D125): it keeps playing the same stretch of its media,
 * so its length on the timeline becomes duration × old speed / new speed (a main clip's neighbours
 * ripple; on a free track it must still fit before the next item). Volume and box keys and
 * animations scale with it, fades are kept within the new length. `keepPitch` false lets the
 * pitch follow the speed (absent: unchanged). Checked with the timeline rules.
 */
export function planSetSpeed(project: Project, itemId: Id, speed: number, keepPitch?: boolean): { readonly ops: ReadonlyArray<Op>; readonly summary: string; readonly duration: number } | { readonly problem: string } {
  const found = locate(project, itemId);
  if (!found) return { problem: "That item isn't in the project any more." };
  const { track, item } = found;
  if (item.type !== "clip" && item.type !== "audio") return { problem: "Only clips and audio items have a speed." };
  const media = project.media.find((m) => m.id === item.media);
  if (!media || media.kind === "image") return { problem: "A still picture has no speed." };
  const locked = lockProblem(track, item);
  if (locked) return { problem: locked };
  if (!Number.isFinite(speed) || speed < SPEED_RANGE.min || speed > SPEED_RANGE.max) return { problem: `Pick a speed from ${SPEED_RANGE.min}× to ${SPEED_RANGE.max}×.` };
  const old = item.speed ?? 1;
  const next = Math.round(speed * 1000) / 1000;
  if (next === old) {
    if (keepPitch === undefined || keepPitch === (item.keepPitch !== false)) return {ops: [], summary: '', duration: item.duration};
    return {ops: [{op: 'update_item', item: item.id, set: {keepPitch: keepPitch ? null : false}}], summary: keepPitch ? 'Keep pitch' : 'Pitch follows speed', duration: item.duration};
  }
  const ratio = old / next;
  let duration = Math.max(1, Math.round(item.duration * ratio));
  const frames = media.probe?.frames;
  const from = item.in ?? 0;
  if (frames && !item.loop && !(item.type === "clip" && item.hold) && from + duration * next > frames) duration = Math.max(1, Math.floor((frames - from) / next));
  const set: Record<string, unknown> = { speed: next === 1 ? null : next, duration };
  if (keepPitch !== undefined) set.keepPitch = keepPitch ? null : false;
  const appearance=pieceAppearance(item,0,Math.max(1,Math.round(duration*next/old)),duration,project.canvas.fps,true,true);
  for(const key of APPEARANCE_KEYS)if(item[key]!==undefined)set[key]=appearance[key]??null;
  let ops: ReadonlyArray<Op> = [{ op: "update_item", item: item.id, set }];
  try {
    if (track.role === 'main') ops = fitChangedTransitions(project, ops);
  } catch (error) {
    return { problem: error instanceof Error ? error.message : String(error) };
  }
  const draft = tryOps(project, ops);
  if ("problem" in draft) return /overlaps/.test(draft.problem) ? { problem: "There's no room on this track for it at that speed: move the next item first." } : draft;
  return { ops, summary: `Speed ${speedLabel(next)}`, duration };
}

export const APPEARANCE_KEYS=['keys','gainKeys','animation','fadeIn','fadeOut','effects'] as const;
/** Rounding a speed change can put keys on the same frame. Merge them, keeping the later value. */
function scaledKeys<T extends {readonly frame:number}>(keys:ReadonlyArray<T>,ratio:number,duration:number):T[]{
  const frames=new Map<number,T>();
  for(const key of [...keys].sort((a,b)=>a.frame-b.frame)){
    const frame=Math.min(duration,Math.max(0,Math.round(key.frame*ratio)));
    frames.set(frame,{...frames.get(frame),...key,frame});
  }
  return [...frames.values()];
}
export function pieceAppearance(item:Item,from:number,to:number,duration:number,fps:number,first:boolean,last:boolean):Pick<Item,typeof APPEARANCE_KEYS[number]>{
  const ratio=duration/Math.max(1,to-from),sliced=sliceItemKeys(item,from,to,fps);
  const animation:NonNullable<Item['animation']>=Object.fromEntries(Object.entries(item.animation??{})
    .filter(([kind,value])=>value&&(kind!=='in'||first)&&(kind!=='out'||last))
    .map(([kind,value])=>[kind,{...value!,duration:Math.max(1,Math.min(duration,Math.round(value!.duration*ratio)))}]));
  const fadeIn=first&&item.fadeIn?Math.min(item.fadeIn,duration):0;
  const fadeOut=last&&item.fadeOut?Math.min(item.fadeOut,duration-fadeIn):0;
  return {
    ...(item.effects?{effects:sliceMaskEffects(item,from,to,duration,fps)}:{}),
    ...(sliced.keys?.length?{keys:scaledKeys(sliced.keys,ratio,duration)}:{}),
    ...(sliced.gainKeys?.length?{gainKeys:scaledKeys(sliced.gainKeys,ratio,duration)}:{}),
    ...(Object.keys(animation).length?{animation}:{}),...(fadeIn?{fadeIn}:{}),...(fadeOut?{fadeOut}:{}),
  };
}

/** Ops that move, cut or retime an item itself (a linked sound touched like this goes its own way). */
const RETIME_KEYS = new Set(["start", "in", "duration", "speed", "keepPitch", "media", "loop"]);
const retimes = (op: Op): string | undefined => {
  if (op.op === "move_item" || op.op === "trim_item" || op.op === "split_item") return op.item;
  if (op.op === "update_item" && Object.keys(op.set).some((k) => RETIME_KEYS.has(k))) return op.item;
  return undefined;
};

/**
 * Linked sound (D123), and linked parts of a take (D215): an audio item or an off-main clip with
 * `linkedTo` follows its clip until the user moves, trims, splits or retimes the linked item itself.
 * This returns `ops` plus the ops that keep every linked item in step with what `ops` did to its
 * clip: the same start, length and speed, its in point moved by as much as the clip's (a detached
 * sound plays the clip's own media from the same point; a take's microphone keeps its own offset
 * into its file), a piece per piece when the clip was split or cut by its transcript (the words cut
 * from the picture are cut from the sound too), gone when the clip is gone. An item the ops touch
 * directly is unlinked unless this edit explicitly sets its link. If keeping them in step would
 * collide with another item or change a locked layer, the edit is refused atomically. Pure; run where
 * edits are applied (the server) and drafted (the renderer).
 */
export function withLinkedAudio(project: Project, ops: ReadonlyArray<Op>): ReadonlyArray<Op> {
  const linked = project.tracks.flatMap((t) => (t.role === "main" ? [] : t.items.filter((i) => (i.type === "audio" || i.type === "clip" || i.type === "danmaku") && typeof i.linkedTo === "string").map((i) => ({ item: i, track: t }))));
  if (!linked.length || !ops.length) return ops;
  let after: Project;
  try {
    after = applyEdit(createSession(project), { actor: "user", ops }).project;
  } catch {
    return ops;
  }
  // Where each item came from: a split's new half descends from the item split.
  const origin = new Map<string, string>();
  const originOf = (id: string): string => {
    const o = origin.get(id);
    return o === undefined || o === id ? id : originOf(o);
  };
  for (const op of ops) if (op.op === "split_item") origin.set(op.newId, op.item);
  const touched = new Set(ops.map(retimes).filter((id): id is string => !!id));
  const explicitLinks = new Set(ops.flatMap((op) => op.op === "update_item" && Object.hasOwn(op.set, "linkedTo") ? [op.item] : []));
  const { starts } = layout(after);
  const beforeStarts = layout(project).starts;
  const beforeItems = new Map(project.tracks.flatMap((t) => t.items.map((i) => [i.id, i] as const)));
  const afterItems = new Map(after.tracks.flatMap((t) => t.items.map((i) => [i.id, { item: i, track: t }] as const)));
  const taken = takenIds(after);
  const extra: Array<Op> = [];
  const unlink: Array<Op> = [];
  for (const { item: a, track: beforeTrack } of linked) {
    // The linked item itself was moved, trimmed, split or retimed: it's on its own now (its halves too).
    const own = [...afterItems.keys()].filter((id) => originOf(id) === a.id);
    if (touched.has(a.id) || [...touched].some((id) => originOf(id) === a.id)) {
      for (const id of own) if (!explicitLinks.has(id) && afterItems.get(id)?.item.linkedTo !== undefined) unlink.push({ op: "update_item", item: id, set: { linkedTo: null } });
      continue;
    }
    const now = afterItems.get(a.id);
    // A direct unlink/relink is a user/assistant edit, not something to repair.
    if (!now || now.item.linkedTo !== a.linkedTo || explicitLinks.has(a.id)) continue;
    const clip = a.linkedTo!;
    const clipBefore = beforeItems.get(clip);
    const pieces = [...afterItems.values()]
      .filter(({ item }) => item.type === "clip" && originOf(item.id) === clip)
      .sort((x, y) => (starts.get(x.item.id) ?? 0) - (starts.get(y.item.id) ?? 0));
    if (now.track.locked || now.item.locked) {
      const leader = pieces.length === 1 ? pieces[0]!.item : undefined;
      const timing = (item: Item) => [item.in ?? 0, item.duration, item.speed ?? 1, item.keepPitch !== false, item.media, item.loop === true, item.hold === true];
      const before = clipBefore && timing(clipBefore);
      const beforeSound = audioSpan(project, beforeTrack, beforeTrack.items.indexOf(a), beforeStarts.get(a.id) ?? 0);
      const afterSound = audioSpan(after, now.track, now.track.items.indexOf(now.item), starts.get(a.id) ?? 0);
      const soundChanged = beforeSound.half !== afterSound.half || beforeSound.nextHalf !== afterSound.nextHalf || beforeSound.dipIn !== afterSound.dipIn || beforeSound.dipOut !== afterSound.dipOut;
      const changed = !before || !leader || leader.id !== clip || starts.get(clip) !== beforeStarts.get(clip) || timing(leader).some((value, index) => value !== before[index]) || soundChanged;
      if (changed) {
        const message = "A linked layer is locked. Unlock it before editing its video.";
        throw new EditError(message, [{ code: "linked-locked", path: `/tracks/${now.track.id}/items/${a.id}`, message }]);
      }
      continue;
    }
    if (!pieces.length) {
      // Its clip is gone (deleted, or every word of it cut): so is the linked item.
      extra.push({ op: "delete_item", item: a.id });
      continue;
    }
    const media = after.media.find((m) => m.id === a.media);
    const frames = media?.kind === "image" ? undefined : media?.probe?.frames;
    // The linked item's in point moves with the clip's (equal for a detached sound: D123 unchanged).
    const offset = (a.in ?? 0) - (clipBefore?.in ?? a.in ?? 0);
    const isClip = a.type === "clip" || a.type === "danmaku";
    const aStart = beforeStarts.get(a.id) ?? 0;
    pieces.forEach(({ item: d }, i) => {
      let speed = d.speed ?? 1;
      let from = Math.max(0, (d.in ?? 0) + offset);
      let duration = d.duration;
      let start = starts.get(d.id) ?? 0;
      let keepPitch = d.keepPitch;
      let loop = media?.kind !== 'image' && d.loop === true;
      let linkedTo: string | undefined = d.id;
      // Issue #118: a piece that plays another media now (a replaced fragment): the linked item
      // plays on there as it did before (a cutaway over the same sound), no longer following it.
      if (clipBefore && d.media !== clipBefore.media) {
        // Recorded comments describe their own source, unlike an intentionally retained
        // soundtrack. A replaced picture must not carry comments from another recording.
        if (a.type === 'danmaku') {
          if (i === 0) extra.push({ op: 'delete_item', item: a.id });
          return;
        }
        const s = Math.max(start, aStart);
        const e = Math.min(start + d.duration, aStart + a.duration);
        if (e <= s) {
          if (i === 0) extra.push({ op: "delete_item", item: a.id });
          return;
        }
        speed = a.speed ?? 1;
        from = (a.in ?? 0) + Math.round((s - aStart) * speed);
        start = s;
        duration = e - s;
        keepPitch = a.keepPitch;
        loop = a.loop === true;
        linkedTo = undefined;
      } else if (frames && !loop && !(isClip && a.hold) && from + duration * speed > frames) duration = Math.max(1, Math.floor((frames - from) / speed));
      const several = pieces.length > 1;
      const want = { start, in: from, duration, speed: speed === 1 ? undefined : speed, keepPitch: keepPitch === false ? false : undefined, loop: loop ? true : undefined, linkedTo };
      const cur=now.item;
      if(i===0&&!several&&cur.start===want.start&&(cur.in??0)===want.in&&cur.duration===want.duration&&cur.speed===want.speed&&cur.keepPitch===want.keepPitch&&cur.loop===want.loop&&cur.linkedTo===want.linkedTo)return;
      // Source time identifies the part of the linked item's own motion/volume
      // envelope. Use the ordinary split interpolation, then rescale for speed.
      const localFrom=Math.round((from-(a.in??0))/(a.speed??1));
      const localTo=localFrom+Math.max(1,Math.round(duration*speed/(a.speed??1)));
      const appearance=pieceAppearance(now.item,localFrom,localTo,duration,project.canvas.fps,i===0,i===pieces.length-1);
      if (i === 0) {
        const set:Record<string,unknown>={start,in:from,duration,speed:want.speed??null,keepPitch:want.keepPitch??null,loop:want.loop??null,linkedTo:linkedTo??null};
        for(const key of APPEARANCE_KEYS)if(cur[key]!==undefined||appearance[key]!==undefined)set[key]=appearance[key]??null;
        extra.push({
          op: "update_item",
          item: a.id,
          set,
        });
        return;
      }
      // Keep the complete audio/visual item (including group, source effects and
      // danmaku settings); only its local timing and link belong to the new piece.
      const {fadeIn:_fi,fadeOut:_fo,keys:_k,gainKeys:_g,animation:_a,effects:_fx,speed:_s,keepPitch:_p,loop:_loop,linkedTo:_link,...rest}=now.item;
      const id = uniqueIn(a.id.replace(/-\d+$/, "") || a.type, taken);
      extra.push({
        op: "insert_item",
        track: now.track.id,
        item: { ...rest,id,in:from,start,duration,...appearance,...(want.speed?{speed:want.speed}:{}),...(want.keepPitch===false?{keepPitch:false}:{}),...(want.loop?{loop:true}:{}),...(linkedTo?{linkedTo}:{}) },
      });
    });
  }
  const synced = [...ops, ...unlink, ...extra];
  if (!extra.length) return synced;
  try {
    applyEdit(createSession(project), { actor: "user", ops: synced }, timelineProblems);
    return synced;
  } catch (error) {
    if (error instanceof EditError && error.details.some((detail) => (detail as { code?: string })?.code === "overlap")) {
      const message = "A linked layer would overlap another clip. Make room on its track or unlink it first.";
      throw new EditError(message, error.details.map((detail) => {
        const problem = detail as { code?: string; path?: string; message?: string };
        return problem.code === "overlap" ? { ...problem, code: "linked-overlap", message } : problem;
      }));
    }
    throw error;
  }
}
