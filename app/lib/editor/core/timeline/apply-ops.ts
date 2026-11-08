// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Reference engine for edit ops: applies them atomically, records who made
// them, and refuses an agent edit that would overwrite newer user edits (D6).
import { sliceGainKeys } from "@/app/lib/editor/core/audio/audio";
import { DEFAULT_KEY_FPS, hasSpringKeys, KEY_PROPS, keyAt, keysOf, propertyKeyAt } from "@/app/lib/editor/core/visual/box";
import { isSpringName, springSettleTime } from "@/app/lib/editor/core/visual/spring";
import { itemUsesMedia } from "@/app/lib/editor/core/timeline/mediaPlan";
import {sliceMaskEffects} from './masks';
import { lockedContextProblem, lockedSectionProblem, unlockProblem } from "@/app/lib/editor/core/timeline/sections";
import type { Edit, FieldSet, Id, Item, LogEntry, Mutable, Op, Problem, Project, Session, Track } from "@/app/lib/editor/core/timeline/types";

type Draft = Mutable<Project>;
type DraftTrack = Mutable<Track>;
type DraftItem = Mutable<Item>;

export class EditError extends Error {
  details: ReadonlyArray<unknown>;
  constructor(message: string, details: ReadonlyArray<unknown> = []) {
    super(message);
    this.name = "EditError";
    this.details = details;
  }
}

export class EditConflict extends EditError {
  constructor(touched: ReadonlyArray<string>) {
    super(`Conflicts with newer edits to: ${touched.join(", ")}`, touched);
    this.name = "EditConflict";
  }
}

export function createSession(project: Project): Session {
  return { project: structuredClone(project), revision: 0, log: [] };
}

/** What an op touches, for conflict checks. */
function touches(op: Op): Array<string> {
  if (op.op === "insert_item") return [op.item.id];
  if ("item" in op && op.item) return [op.item];
  if (op.op === "add_track") return [`track:${op.track.id}`];
  if ("track" in op && op.track) return [`track:${op.track}`];
  if (op.op === "add_media") return [`media:${op.media.id}`];
  if (op.op === "update_media") return [`media:${op.media}`];
  if (op.op === "delete_media") return [`media:${op.media}`];
  return ["project"];
}

function locate(project: Draft, itemId: Id): { track: DraftTrack; index: number; item: DraftItem } {
  for (const track of project.tracks) {
    const index = track.items.findIndex((i) => i.id === itemId);
    if (index !== -1) return { track, index, item: track.items[index]! };
  }
  throw new EditError(`Item "${itemId}" does not exist`);
}

function findTrack(project: Draft, trackId: Id): DraftTrack {
  const track = project.tracks.find((t) => t.id === trackId);
  if (!track) throw new EditError(`Track "${trackId}" does not exist`);
  return track;
}

// Keys that would reach an object's prototype instead of setting a field. Ops come from
// clients and the agent, so they are refused rather than trusted.
const UNSAFE_KEYS = new Set(["__proto__", "constructor", "prototype"]);

function merge(target: object, set: FieldSet) {
  const fields = target as Record<string, unknown>;
  for (const k of Object.keys(set)) {
    if (UNSAFE_KEYS.has(k)) throw new EditError(`"${k}" can't be set`);
  }
  for (const [k, v] of Object.entries(set)) {
    if (v === null) delete fields[k];
    else fields[k] = v;
  }
}

/**
 * Locked tracks and items can't be edited, by the user or the agent (D52). update_track stays
 * allowed, so a locked track can still be unlocked, hidden or muted.
 */
function guard(track: DraftTrack, item?: DraftItem) {
  if (track.locked) throw new EditError(`Track "${track.name ?? track.id}" is locked: unlock it to edit it`);
  if (item?.locked) throw new EditError(`"${item.id}" is locked: unlock it to edit it`);
}

/** The item an op works on, refused if it (or its track) is locked. */
function editable(project: Draft, itemId: Id) {
  const found = locate(project, itemId);
  guard(found.track, found.item);
  return found;
}

/** Clips and audio play from a time in their media; still images and items without media don't. */
function hasSourceTime(project: Draft, item: DraftItem) {
  // A graphic's time runs from its own start (D78): a trimmed start or a split's second half goes on from where it was.
  if (item.type === "graphic") return true;
  if (!item.media) return item.in !== undefined;
  return project.media.find((m) => m.id === item.media)?.kind !== "image";
}

/** An item's animations, each no longer than the item. */
function fitAnimation<T extends { duration: number }>(animation: { in?: T; out?: T; loop?: T }, duration: number) {
  for (const a of Object.values(animation)) if (a) a.duration = Math.min(a.duration, duration);
  return animation;
}

function applyOp(project: Draft, op: Op) {
  switch (op.op) {
    case "insert_item": {
      const track = findTrack(project, op.track);
      guard(track);
      const item = structuredClone(op.item) as DraftItem;
      if (track.role === "main") track.items.splice(op.index ?? track.items.length, 0, item);
      else track.items.push(item);
      return;
    }
    case "update_item": {
      // Locking or unlocking an item is the one content change a locked item takes; its gap (D224)
      // is its position, which may shift like a ripple shifts it (D52), so a delete before it works.
      const onlyLock = Object.keys(op.set).every((k) => k === "locked" || k === "gap");
      const {item} = onlyLock ? locate(project, op.item) : editable(project, op.item);
      const before = {...item};
      merge(item, op.set);
      if (!Object.hasOwn(op.set, 'effects') && (item.duration !== before.duration || item.speed !== before.speed)) {
        const sourceEnd = Math.max(1, Math.round(item.duration * (item.media === before.media ? (item.speed ?? 1) / (before.speed ?? 1) : 1)));
        const effects = sliceMaskEffects(before, 0, sourceEnd, item.duration, project.canvas.fps);
        if (effects) item.effects = structuredClone(effects) as DraftItem['effects'];
        else delete item.effects;
      }
      return;
    }
    case "trim_item": {
      // Box keys (D75) keep their values at a cut: a key goes on the cut with each keyed property's value there.
      const { item } = editable(project, op.item);
      if (item.duration - op.frames < 1) throw new EditError(`Trimming ${op.frames} frames would leave "${item.id}" empty`);
      const keys = sliceItemKeys(item, op.edge === 'start' ? op.frames : 0, item.duration - (op.edge === 'end' ? op.frames : 0), project.canvas.fps);
      const effects = sliceMaskEffects(item, op.edge === 'start' ? op.frames : 0, item.duration - (op.edge === 'end' ? op.frames : 0), item.duration - op.frames, project.canvas.fps);
      if (effects) item.effects = structuredClone(effects) as DraftItem['effects'];
      else delete item.effects;
      item.duration -= op.frames;
      if (op.edge === "start") {
        // The source moves with the start, at the clip's speed. A still image has no source time.
        if (item.type === "graphic") {
          // A graphic is lengthened at its start too: its time can't go before its own 0, so it clamps there (not an error).
          const at = Math.max(0, (item.in ?? 0) + Math.round(op.frames * (item.speed ?? 1)));
          if (at > 0 || item.in !== undefined) item.in = at;
        } else if (hasSourceTime(project, item)) item.in = (item.in ?? 0) + Math.round(op.frames * (item.speed ?? 1));
        if (item.start !== undefined) item.start += op.frames;
        if (item.in !== undefined && item.in < 0) throw new EditError(`"${item.id}" can't extend before the start of its media`);
      }
      // Both edges use the same cut-point values as replacement, speed changes and export ranges.
      for (const key of ['keys', 'gainKeys'] as const) {
        if (keys[key]?.length) Object.assign(item, { [key]: structuredClone(keys[key]) });
        else delete item[key];
      }
      return;
    }
    case "split_item": {
      const { track, index, item } = editable(project, op.item);
      if (op.at < 1 || op.at >= item.duration) throw new EditError(`Split point ${op.at} is outside "${item.id}"`);
      const second = structuredClone(item);
      const firstEffects = sliceMaskEffects(item, 0, op.at, op.at, project.canvas.fps);
      const secondEffects = sliceMaskEffects(item, op.at, item.duration, item.duration - op.at, project.canvas.fps);
      if (firstEffects) item.effects = structuredClone(firstEffects) as DraftItem['effects'];
      else delete item.effects;
      if (secondEffects) second.effects = structuredClone(secondEffects) as DraftItem['effects'];
      else delete second.effects;
      second.id = op.newId;
      second.duration = item.duration - op.at;
      if (hasSourceTime(project, item)) second.in = (item.in ?? 0) + Math.round(op.at * (item.speed ?? 1));
      if (item.start !== undefined) second.start = item.start + op.at;
      delete second.transitionIn;
      // D224: the second half follows the first; the gap stays before the first.
      delete second.gap;
      item.duration = op.at;
      // The first half keeps how the item comes in, the second how it goes out; keys go to the
      // half they fall in (a key on the cut goes to both, so each half keeps its value there).
      if (item.animation) {
        const { in: animIn, out, loop } = item.animation;
        const first = { ...(animIn ? { in: animIn } : {}), ...(loop ? { loop } : {}) };
        const last = { ...(out ? { out } : {}), ...(loop ? { loop } : {}) };
        if (Object.keys(first).length) item.animation = fitAnimation(first, item.duration);
        else delete item.animation;
        if (Object.keys(last).length) second.animation = fitAnimation(last, second.duration);
        else delete second.animation;
      }
      delete item.fadeOut;
      delete second.fadeIn;
      if (item.fadeIn !== undefined) item.fadeIn = Math.min(item.fadeIn, item.duration);
      if (second.fadeOut !== undefined) second.fadeOut = Math.min(second.fadeOut, second.duration);
      if (item.keys) {
        const keys = withCutKey(item.keys, op.at, project.canvas.fps, op.at + second.duration);
        second.keys = keys.filter((k) => k.frame >= op.at).map((k) => ({ ...k, frame: k.frame - op.at }));
        item.keys = keys.filter((k) => k.frame <= op.at);
      }
      if (item.gainKeys?.length) {
        second.gainKeys = structuredClone(sliceGainKeys(item.gainKeys, op.at, op.at + second.duration)) as DraftItem['gainKeys'];
        item.gainKeys = structuredClone(sliceGainKeys(item.gainKeys, 0, op.at)) as DraftItem['gainKeys'];
      }
      for (const half of [item, second]) {
        if (half.keys?.length === 0) delete half.keys;
        if (half.gainKeys?.length === 0) delete half.gainKeys;
      }
      track.items.splice(index + 1, 0, second);
      return;
    }
    case "move_item": {
      const { track, index, item } = editable(project, op.item);
      const target = op.track ? findTrack(project, op.track) : track;
      guard(target);
      track.items.splice(index, 1);
      // D224: a moved item leaves its gap behind (a planner sets one after the move when it wants it).
      delete item.gap;
      if (target.role === "main") {
        delete item.start;
        target.items.splice(op.index ?? target.items.length, 0, item);
      } else {
        if (op.start !== undefined) item.start = op.start;
        if (item.start === undefined) throw new EditError(`Moving "${item.id}" to a free track needs a start`);
        delete item.transitionIn;
        target.items.push(item);
      }
      return;
    }
    case "delete_item": {
      const { track, index } = editable(project, op.item);
      track.items.splice(index, 1);
      return;
    }
    case "set_transition": {
      const { item } = editable(project, op.item);
      if (op.transition === null) delete item.transitionIn;
      else item.transitionIn = structuredClone(op.transition) as DraftItem["transitionIn"];
      return;
    }
    case "add_track":
      project.tracks.splice(op.index ?? project.tracks.length, 0, { items: [], ...(structuredClone(op.track) as Partial<DraftTrack>) } as DraftTrack);
      return;
    case "update_track": {
      const track = findTrack(project, op.track);
      // Items change through item ops (so locks, ids and the log see them); an id is a reference.
      for (const key of ["items", "id"]) if (key in op.set) throw new EditError(`A track's ${key} can't be changed with update_track`);
      if (track.locked && ("kind" in op.set || "role" in op.set)) guard(track);
      merge(track, op.set);
      return;
    }
    case "delete_track": {
      const track = findTrack(project, op.track);
      guard(track);
      for (const item of track.items) guard(track, item);
      project.tracks = project.tracks.filter((t) => t.id !== op.track);
      return;
    }
    case "add_media":
      project.media.push(structuredClone(op.media) as Draft["media"][number]);
      return;
    case "update_media": {
      const media = project.media.find((m) => m.id === op.media);
      if (!media) throw new EditError(`No media "${op.media}"`);
      for (const k of Object.keys(op.set)) if (!["path", "name", "probe", "source"].includes(k)) throw new EditError(`update_media can't set "${k}"`);
      if ("path" in op.set && (typeof op.set.path !== "string" || !op.set.path)) throw new EditError("update_media: path must be a non-empty string");
      merge(media, structuredClone(op.set));
      return;
    }
    case "delete_media": {
      // F8 (D167): only once nothing uses it (delete those items first, in the same edit). The file stays.
      if (!project.media.some((m) => m.id === op.media)) throw new EditError(`No media "${op.media}"`);
      const user = project.tracks.flatMap((t) => t.items).find((i) => itemUsesMedia(i as Item, op.media));
      if (user) throw new EditError(`Media "${op.media}" is still used by "${user.id}"`);
      project.media = project.media.filter((m) => m.id !== op.media);
      return;
    }
    case "update_project": {
      // A2: motionBlur merges field by field (the motion-film skill raises samples alone; on and shutter stay).
      const set = structuredClone(op.set) as Record<string, unknown>;
      const blur = set.motionBlur;
      if (blur && typeof blur === "object") {
        const next: Record<string, unknown> = { ...((project as { motionBlur?: object }).motionBlur ?? {}) };
        for (const [k, v] of Object.entries(blur)) {
          if (UNSAFE_KEYS.has(k)) throw new EditError(`"${k}" can't be set`);
          if (v === null) delete next[k];
          else next[k] = v;
        }
        set.motionBlur = Object.keys(next).length ? next : null;
      }
      merge(project, set as FieldSet);
      return;
    }
    default:
      throw new EditError(`Unknown op "${(op as { op: unknown }).op}"`);
  }
}

/**
 * Apply one edit (a list of ops) atomically. Returns a new session; the old one is untouched.
 * baseRevision is the revision the actor last saw. If the agent's ops touch something the
 * user changed after that, the whole edit is refused with EditConflict.
 */
export function applyEdit(session: Session, { actor, ops, baseRevision = session.revision, turnId, summary, at }: Edit, validate?: (project: Project) => ReadonlyArray<Problem>): Session {
  if (!["user", "agent"].includes(actor)) throw new EditError(`Unknown actor "${actor}"`);
  if (baseRevision < session.revision) {
    const mine = new Set(ops.flatMap(touches));
    const newer = session.log.filter((e) => e.revision > baseRevision && e.actor !== actor).flatMap((e) => e.ops.flatMap(touches));
    const clash = [...new Set(newer.filter((id) => mine.has(id)))];
    if (clash.length) throw new EditConflict(clash);
  }
  const project = structuredClone(session.project) as Draft;
  for (const op of ops) applyOp(project, op);
  // Issue #118: what a locked section plays never changes (a ripple from before it included), whoever edits.
  // The agent also can't unlock one, or change its tracks, media files, canvas or master.
  const locked = session.project.sections?.length
    ? ((actor === "agent" ? unlockProblem(session.project, project as Project) : undefined) ?? lockedSectionProblem(session.project, project as Project) ?? (actor === "agent" ? lockedContextProblem(session.project, project as Project) : undefined))
    : undefined;
  if (locked) throw new EditError("The edit would change a locked section", [locked]);
  if (validate) {
    const errors = validate(project);
    if (errors.length) throw new EditError("The edit would make the project invalid", errors);
  }
  const revision = session.revision + 1;
  const entry: Mutable<LogEntry> = { revision, at: at ?? new Date().toISOString(), actor, ops: structuredClone(ops) as Array<Mutable<Op>> };
  if (turnId) entry.turnId = turnId;
  if (summary) entry.summary = summary;
  return { project, revision, log: [...session.log, entry] };
}

type Keys = NonNullable<Item["keys"]>;
/** Keep the same motion/envelope over a source-relative piece, including values at its cuts. */
export function sliceItemKeys(item:Item,from:number,to:number,fps:number):Pick<Item,'keys'|'gainKeys'>{
  if(from===0&&to===item.duration)return {
    ...(item.keys?{keys:item.keys}:{}),...(item.gainKeys?{gainKeys:item.gainKeys}:{}),
  };
  const keys=item.keys?.length?withCutKey(withCutKey(item.keys,from,fps,Math.max(item.duration,to)),to,fps,Math.max(item.duration,to))
    .filter(key=>key.frame>=from&&key.frame<=to).map(key=>({...key,frame:key.frame-from})):undefined;
  const gainKeys=sliceGainKeys(item.gainKeys,from,to);
  return {...(keys?.length?{keys}:{}),...(gainKeys?.length?{gainKeys}:{})};
}
/**
 * Box keys (D75) prepared for a cut at `at`: each keyed property gets its value there on a key at
 * `at`, so neither side jumps; and when the segment crossing the cut is eased (in / out / inOut),
 * all animated properties are sampled together into linear keys, so both halves keep the same
 * motion frame for frame. Ease belongs to a whole key: filling just one property at another
 * property's key would give it the other curve's ease and change the motion.
 * Spring keys (D192) reach past their segment (a spring runs on until it settles, and later keys
 * add to it), so a cut while one is moving first bakes every keyed property into linear keys on
 * every frame from the first key until the last spring has settled (bakeMotion).
 */
function withCutKey(input: Keys, at: number, fps: number = DEFAULT_KEY_FPS, end = Infinity): Keys {
  const keys = bakeMotion(input, at, fps, end);
  const add = new Map<number, Record<string, number>>();
  const put = (frame: number, p: string, v: number) => add.set(frame, { ...(add.get(frame) ?? {}), [p]: Math.round(v * 1e6) / 1e6 });
  for (const p of KEY_PROPS) {
    const list = keysOf(keys, p);
    if (!list.length || list.some((k) => k.frame === at)) continue;
    put(at, p, keyAt(keys, p, at, fps)!);
  }
  if (!add.size) return keys;
  const out = keys.map((k) => ({ ...k }));
  for (const [frame, values] of add) {
    const i = out.findLastIndex((k) => k.frame === frame);
    if (i !== -1) out[i] = { ...out[i]!, ...values };
    else out.push({ frame, ...values });
  }
  // A baked segment moves linearly from frame to frame: the key it starts from mustn't ease it again.
  return out.sort((a, b) => a.frame - b.frame);
}

/**
 * Eased properties share a key's ease, so sample all channels when inserting a cut inside a
 * curve. Springs also need their complete settling tail. Values at original timeline frames are
 * unchanged; intermediate subframes use linear interpolation. Never sample beyond the item.
 */
function bakeMotion(keys: Keys, at: number, fps: number, limit: number): Keys {
  const curves = KEY_PROPS.map(prop => ({ prop, points: keysOf(keys, prop) })).filter(curve => curve.points.length);
  const spring = hasSpringKeys(keys);
  const eased = curves.some(({ points }) => points.slice(0, -1).some(point => point.ease !== 'linear'));
  const partial = curves.some(({ points }) => at > points[0]!.frame && at < points.at(-1)!.frame && !points.some(point => point.frame === at));
  if (!spring && (!eased || !partial)) return keys;
  const rate = Number.isFinite(fps) && fps > 0 ? fps : DEFAULT_KEY_FPS;
  const frames = keys.map((k) => k.frame);
  const first = Math.min(...frames);
  // Settled to a millionth of each step, so holding the last baked value is no visible jump.
  const settle = Math.max(...keys.map((k) => (isSpringName(k.ease) ? springSettleTime(k.ease, 1e-6) : 0)));
  const end = Math.min(Math.max(...frames) + Math.ceil(settle * rate), Math.max(limit, ...frames));
  if (at <= first || at >= end) return keys;
  const original = new Map(keys.map(key => [key.frame, key]));
  const out: Array<Keys[number]> = [];
  for (let f = first; f <= end; f++) {
    const values: Record<string, number> = {};
    for (const { prop, points } of curves) values[prop] = Math.round(propertyKeyAt(points, f, rate)! * 1e6) / 1e6;
    const own = original.get(f);
    out.push({ ...(own ?? {}), frame: f, ...values, ease: "linear" });
  }
  return out;
}
