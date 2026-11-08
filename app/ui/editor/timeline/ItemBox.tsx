import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// One item on a timeline lane (its box, name, length, trim and fade handles) and the volume line
// of items with sound (D83). Split out of Timeline.tsx.
import { VolumeX } from "lucide-react";
import React, { useEffect, useRef, useState } from "react";
import { speedLabel } from "@/app/lib/editor/core/timeline/arrange";
import { type CaptionCard } from "@/app/lib/editor/core/text/captions";
import { duckDbAt, duckKeysIn, type DuckKey } from "@/app/lib/editor/core/audio/ducking";
import { SOLID, solidBackground } from "@/app/lib/editor/core/visual/render";

import { dbLabel, envelopeDbAt, hasSound, keyIndexAt, levelY, planAddGainKey, planMoveGainKey, planRemoveGainKey, transitionFades, yLevel } from "@/app/ui/editor/audio/sound";
import { getPlayback, playback, usePlayback } from "@/app/ui/editor/preview/playback";
import { itemLabel, type Item, type ProjectModel } from "@/app/lib/editor/state/project";
import { actions, projectOf, thumbUrl } from "@/app/lib/editor/state/store";
import { usePoster, usePreviewVisible } from '@/app/ui/editor/preview/resources';
import { keyFrames } from "@/app/ui/editor/timeline/clip";
import { draftOf, findItem, visibleBeats } from "@/app/ui/editor/timeline/edit";
import { ItemWaveform } from "@/app/ui/editor/timeline/Waveform";
import { ClipFilmstrip } from "@/app/ui/editor/timeline/Filmstrip";
import { RenameField } from "@/app/ui/editor/timeline/RenameField";
import { currentEditorStore } from '@/app/lib/editor/state/store/core';
import { beginEditorGesture, editorGestureOrigin } from '../workspace/pointerGesture';
import { useProjectEdit } from '../workspace/projectEdit';

// A press has to move this far before it becomes a drag (a click only selects).
export const DRAG_PX = 3;

// Issue #118: how many times items were drawn, for the performance test (as the preview's __biliupPreview).
const timelineStats = { itemRenders: 0 };
(globalThis as { __biliupTimeline?: unknown }).__biliupTimeline = { stats: () => ({ ...timelineStats }) };

type BoxProps = Parameters<typeof ItemBoxView>[0];
/**
 * Issue #118: an item is drawn again only when something it shows changed, not on every render of
 * the timeline (a key typed in the chat, a job's progress, another item selected). The handlers
 * aren't compared: the timeline's read the latest state when they run.
 */
const sameBox = (a: BoxProps, b: BoxProps) => {
  for (const key of Object.keys({ ...a, ...b }) as Array<keyof BoxProps>) {
    const x = a[key];
    const y = b[key];
    if (x === y || (typeof x === "function" && typeof y === "function")) continue;
    if (key === "data" && x && y && Object.keys(x).length === Object.keys(y).length && Object.entries(x as object).every(([k, v]) => (y as Record<string, unknown>)[k] === v)) continue;
    return false;
  }
  return true;
};

/** One item on a lane: its box, name, length and (when it can be edited) trim handles at both edges. */
export const ItemBox = React.memo(ItemBoxView, sameBox);

function ItemBoxView({
  project,
  item,
  start,
  duration,
  ppf,
  beats,
  className,
  data,
  edges,
  onPointerDown,
  labelInset = 0,
  cards,
  envelope = "none",
  duck,
  renaming = false,
  onRenamed,
  onContextMenu,
  onDoubleClick,
}: {
  project: ProjectModel;
  item: Item;
  start: number;
  duration: number;
  ppf: number;
  /** The item's media's beats (seconds), when cached (D70). */
  beats?: ReadonlyArray<number> | undefined;
  className: string;
  data: Readonly<Record<string, string | number>>;
  edges: boolean;
  onPointerDown?: (e: React.PointerEvent) => void;
  /** Pixels the name starts in from the left (a transition marker sits over the start). */
  labelInset?: number;
  /** A captions item's cards (D82), drawn as blocks with their words. */
  cards?: ReadonlyArray<CaptionCard>;
  /** The volume line (D83) on items with sound: drawn, or drawn with keys to click and drag (the selected item). */
  envelope?: "none" | "show" | "edit";
  /** Its track's duck curve (D85, project frames): drawn as a second, dashed line under the volume line. */
  duck?: ReadonlyArray<DuckKey> | undefined;
  /** T7: its name is being edited in place. */
  renaming?: boolean;
  onRenamed?: () => void;
  onContextMenu?: (e: React.MouseEvent) => void;
  onDoubleClick?: () => void;
}) {
  useEditorLanguage();
  const media = item.media ? project.media.find((m) => m.id === item.media) : undefined;
  timelineStats.itemRenders++;
  const previewRef = React.useRef<HTMLDivElement>(null);
  const visible = usePreviewVisible(previewRef);
  const poster = usePoster(item.type === 'clip' && media && media.kind !== 'audio' ? thumbUrl(media.id, media.path) : undefined, visible).value?.src;
  const kind = item.type === "clip" ? "video" : item.type === "danmaku" ? "text" : item.type;
  const label = itemLabel(project, item);
  const sound = media && hasSound(project, item) ? media : undefined;
  const follows = sound ? transitionFades(project, item.id) : { in: false, out: false };
  const px = Math.max(4, duration * ppf - 2);
  const fadeIn = sound && !follows.in ? Math.min(px, (item.fadeIn ?? 0) * ppf) : 0;
  const fadeOut = sound && !follows.out ? Math.min(px, (item.fadeOut ?? 0) * ppf) : 0;
  // Beats as ticks along the top and bottom edges, unless too close together at this zoom.
  const ticks = sound && beats ? visibleBeats(project, item, beats, ppf) : [];
  // F5 (D148): the item's box keys as diamonds along its bottom edge; a click goes there.
  const marks = item.type === "captions" ? [] : keyFrames(item).filter((f) => f >= 0 && f <= duration);
  return (
    <div
      className={`clip ${kind} ${className}`}
      ref={previewRef}
      data-type={item.type}
      {...data}
      title={label}
      data-muted={item.mute ? "true" : undefined}
      data-speed={item.speed !== undefined && item.speed !== 1 ? item.speed : undefined}
      style={{ left: start * ppf, width: px }}
      onPointerDown={onPointerDown}
      onContextMenu={onContextMenu}
      onDoubleClick={onDoubleClick}
    >
      {/* Issue #53: a video clip's frames across its time; stills (and other items) keep the poster. */}
      {item.type === "clip" && media?.kind === "video" ? (
        <ClipFilmstrip item={item} media={media} fps={project.canvas.fps} duration={duration} ppf={ppf} poster={poster} />
      ) : (
        poster && <div className="strip" style={{ backgroundImage: `url("${poster}")` }} />
      )}
      {cards && (
        <div className="caption-cards" data-testid="caption-cards" data-count={cards.length}>
          {cards.map((c) => (
            <div key={`${c.from}-${c.words[0]!.index}`} className="caption-card" data-testid="caption-card" data-from={c.from} data-to={c.to} title={c.words.map((w) => w.text).join(" ")} style={{ left: (c.from - start) * ppf, width: Math.max(2, (c.to - c.from) * ppf - 1) }}>
              {c.words.map((w) => w.text).join(" ")}
            </div>
          ))}
        </div>
      )}
      {item.type === "component" && item.component === SOLID && <div className="strip solid-fill" data-testid="solid-fill" style={{ background: solidBackground(item, project) }} />}
      {sound && <ItemWaveform project={project} item={item} media={sound} start={start} duck={duck} width={px} compact={item.type !== "audio"} />}
      {ticks.length > 0 && (
        <svg className="beat-ticks" data-testid="beat-ticks" data-media={sound!.id} data-count={ticks.length} data-first={Math.round(start + ticks[0]!)} viewBox={`0 0 ${px} 100`} preserveAspectRatio="none">
          <path d={ticks.map((f) => `M${(f * ppf).toFixed(1)} 0V22M${(f * ppf).toFixed(1)} 78V100`).join("")} />
        </svg>
      )}
      {sound && (fadeIn > 0 || fadeOut > 0) && (
        // The fades as ramps: the shaded corner is what the fade takes away.
        <svg className="fade-shape" data-testid="fade-shape" viewBox={`0 0 ${px} 100`} preserveAspectRatio="none">
          {fadeIn > 0 && <path d={`M0 0 L${fadeIn} 0 L0 100 Z`} />}
          {fadeOut > 0 && <path d={`M${px} 0 L${px - fadeOut} 0 L${px} 100 Z`} />}
        </svg>
      )}
      {sound && envelope !== "none" && <Envelope project={project} item={item} px={px} ppf={ppf} edit={envelope === "edit"} duck={duck} start={start} />}
      {marks.length > 0 && (
        <div className="key-marks" data-testid="key-marks" data-count={marks.length}>
          {marks.map((f) => (
            <span
              key={f}
              className="key-mark"
              data-testid="key-mark"
              data-frame={Math.round(start + f)}
              title={editorText("Key at {0} s", { 0: (f / project.canvas.fps).toFixed(2) })}
              style={{ left: Math.min(px - 4, Math.max(4, f * ppf)) }}
              onPointerDown={(e) => {
                if (e.button !== 0) return;
                e.stopPropagation();
                playback.pause();
                if (getPlayback().selected !== item.id) playback.select(item.id);
                playback.seek(Math.round(start + f));
              }}
            />
          ))}
        </div>
      )}
      <span className="nm" style={labelInset ? { paddingLeft: Math.min(labelInset, Math.max(0, px - 30)) } : undefined}>
        {renaming ? <RenameField testId="item-rename" value={label} onDone={(name,origin) => (name !== undefined && name !== label && void actions.renameItem(item.id, name,origin), onRenamed?.())} /> : label}
      </span>
      {(item.mute || (item.speed !== undefined && item.speed !== 1)) && px > 44 && (
        <span className="clip-badge" data-testid="clip-badge">
          {item.speed !== undefined && item.speed !== 1 && speedLabel(item.speed)}
          {item.mute && <VolumeX />}
        </span>
      )}
      <span className="meta">{(duration / project.canvas.fps).toFixed(1)}{editorText("s")}</span>
      {edges && (
        <>
          <div className="edge start" data-edge="start" data-testid="trim-start" title={editorText("Drag to trim")} />
          <div className="edge end" data-edge="end" data-testid="trim-end" title={editorText("Drag to trim")} />
        </>
      )}
      {edges && sound && (
        <>
          {/* F27 (D169): 12 px squares (a wider hit area), clear of the 6 px trim edges. */}
          {!follows.in && <div className="fade-handle in" data-fade="in" data-testid="fade-in-handle" title={editorText("Drag to fade in")} style={{ left: Math.min(px - 13, Math.max(13, fadeIn)) }} />}
          {!follows.out && <div className="fade-handle out" data-fade="out" data-testid="fade-out-handle" title={editorText("Drag to fade out")} style={{ right: Math.min(px - 13, Math.max(13, fadeOut)) }} />}
        </>
      )}
    </div>
  );
}

/**
 * The volume line of an item with sound (D83): gainDb plus its gainKeys envelope (linear in dB
 * between keys, held outside them), on the level scale the Mixer's faders use (levelY: +12 dB at
 * the top, 0 dB a quarter down). On the selected item (when it is at least ENVELOPE_MIN_PX wide):
 * a press within 4 px of the line adds a key there with the line's value (dragging right away
 * adds it where the pointer lets go), dragging a key moves it in time and volume (one frame clear
 * of its neighbours), double-click removes it, and a click selects it so ⌫ removes it. One edit
 * per gesture; a drag plays as it moves (playback carries on). Keys are addressed by their index
 * in frame order. The trim edges and fade handles sit above the line, so they keep their presses.
 */
const ENVELOPE_MIN_PX = 40;
function Envelope({ project, item, px, ppf, edit: editable, duck, start }: { project: ProjectModel; item: Item; px: number; ppf: number; edit: boolean; duck?: ReadonlyArray<DuckKey> | undefined; start: number }) {
  useEditorLanguage();
  const edit = editable && px >= ENVELOPE_MIN_PX;
  const selectedKey = usePlayback((s) => (s.selected === item.id ? s.gainKey : undefined));
  // While a key is dragged: the item as it would be (from the plan's gainKeys).
  const [shown, setShown] = useState<Item | undefined>(undefined);
  const editor=useProjectEdit();
  const stop=useRef<(()=>void)|undefined>(undefined);
  const active=useRef<object|undefined>(undefined);
  const latestGesture=useRef<object|undefined>(undefined);
  useEffect(()=>()=>{stop.current?.();latestGesture.current=undefined},[]);
  useEffect(()=>{stop.current?.()},[project,edit]);
  const ref = useRef<SVGSVGElement>(null);
  const view = shown ?? item;
  const fps = project.canvas.fps;
  const keys = [...(view.gainKeys ?? [])].sort((a, b) => a.frame - b.frame);
  // A selected key that isn't there any more (undo, an agent edit) is deselected.
  const keyCount = (item.gainKeys ?? []).length;
  useEffect(() => {
    if (selectedKey !== undefined && selectedKey >= keyCount) playback.selectGainKey(item.id, undefined);
  }, [selectedKey, keyCount, item.id]);
  // The line: sampled every few pixels, plus each key, so the corners are exact. Issue #118: only
  // between the first and last key (outside them the level holds, a straight line), so a long item
  // with no keys is two points, not one every 3 px of its (zoomed) width.
  const step = Math.max(1, Math.round(3 / ppf));
  const frames = new Set<number>([0, view.duration, ...keys.map((k) => k.frame)]);
  if (keys.length > 1) for (let f = Math.max(0, keys[0]!.frame); f < Math.min(view.duration, keys.at(-1)!.frame); f += step) frames.add(f);
  const pts = [...frames].sort((a, b) => a - b).map((f) => `${(f * ppf).toFixed(1)},${(levelY(envelopeDbAt(view, f)) * 100).toFixed(2)}`);
  const d = `M${pts.join("L")}`;
  // The ducking (D85): the volume line with the duck curve added, where the curve dips inside the item.
  const duckKeys = duck ? duckKeysIn(duck, start, start + view.duration) : [];
  let duckPath: string | undefined;
  if (duckKeys.length) {
    const df = new Set(frames);
    for (let f = 0; f < view.duration; f += step) df.add(f);
    for (const k of duckKeys) if (k.frame - start > 0 && k.frame - start < view.duration) df.add(k.frame - start);
    duckPath = `M${[...df]
      .sort((a, b) => a - b)
      .map((f) => `${(f * ppf).toFixed(1)},${(levelY(envelopeDbAt(view, f) + duckDbAt(duck, start + f)) * 100).toFixed(2)}`)
      .join("L")}`;
  }

  /** Frame (from the item's start) and the item's dB at a pointer position. */
  const at = (ev: { clientX: number; clientY: number }) => {
    const r = ref.current!.getBoundingClientRect();
    return { frame: Math.max(0, Math.min(item.duration, Math.round((ev.clientX - r.left) / ppf))), db: yLevel((ev.clientY - r.top) / r.height), height: r.height };
  };
  /**
   * A press on the line (`key` undefined) or on a key: plan from the pointer on every move (draft
   * shown and heard), then save once against the revision where the gesture started.
   */
  const press = (e: React.PointerEvent, key?: { index: number; frame: number; db: number }) => {
    if (e.button !== 0||!edit) return;
    e.stopPropagation();
    e.preventDefault();
    stop.current?.();
    const origin=editorGestureOrigin();
    if(!origin||projectOf(origin.owner.getSnapshot().snapshot!)!==project)return;
    if (key) playback.selectGainKey(item.id, key.index);
    const x0 = e.clientX;
    const y0 = e.clientY;
    const start = at(e);
    let moved = false;
    const token={};active.current=token;latestGesture.current=token;
    /** The plan for the gesture so far, and the frame of the key it leaves (to select it). */
    const planFor = (base: ProjectModel, ev: { clientX: number; clientY: number }) => {
      if (!moved) return key ? undefined : { plan: planAddGainKey(base, item.id, start.frame), frame: start.frame };
      const p = at(ev);
      if (!key) return { plan: planAddGainKey(base, item.id, p.frame, p.db), frame: p.frame };
      const frame = key.frame + Math.round((ev.clientX - x0) / ppf);
      const db = yLevel(levelY(key.db) + (ev.clientY - y0) / p.height);
      return { plan: planMoveGainKey(base, item.id, key.index, frame, db), frame: undefined };
    };
    const move = (ev: PointerEvent) => {
      if (!moved && Math.abs(ev.clientX - x0) < DRAG_PX && Math.abs(ev.clientY - y0) < DRAG_PX) return;
      moved = true;
      const plan = planFor(project, ev)?.plan;
      if (!plan || "problem" in plan || !plan.ops.length){setShown(undefined);editor.cancel();return}
      const result = draftOf(project, plan.ops);
      if (!("project" in result)){setShown(undefined);editor.cancel();return}
      setShown(findItem(result.project, item.id)?.item);
      editor.preview(project,plan);
    };
    const clear = () => {
      if(active.current!==token)return;
      active.current=undefined;stop.current=undefined;
      setShown(undefined);
      editor.cancel();
    };
    const up = (ev: PointerEvent) => {
      const planned = planFor(project, ev);
      const plan = planned?.plan;
      if (!plan || ("ops" in plan && !plan.ops.length)) return clear();
      // The key that results (new or moved) is selected, so ⌫ removes it.
      const set = "ops" in plan ? (plan.ops[0] as { set?: { gainKeys?: ReadonlyArray<{ frame: number; db: number }> | null } }).set : undefined;
      const landed = key ? key.index : planned.frame !== undefined ? keyIndexAt(set?.gainKeys, planned.frame) : -1;
      void editor
        .commit(()=>plan,origin)
        .then((ok) => {
          if (ok && latestGesture.current===token && currentEditorStore()===origin.owner && getPlayback().selected===item.id && landed >= 0) playback.selectGainKey(item.id, landed);
        })
        .finally(clear);
    };
    const pointer=beginEditorGesture(e,{move,commit:(_from,next)=>up(next),cancel:clear});
    stop.current=()=>{pointer?.();clear()};
  };
  const remove = (e: React.MouseEvent, index: number) => {
    e.stopPropagation();
    stop.current?.();
    const origin=editorGestureOrigin();
    if(!edit||!origin||projectOf(origin.owner.getSnapshot().snapshot!)!==project)return;
    const token={};latestGesture.current=token;
    void editor.commit(current=>planRemoveGainKey(current,item.id,index),origin).then(ok=>{
      if(ok&&latestGesture.current===token&&currentEditorStore()===origin.owner&&getPlayback().selected===item.id)playback.selectGainKey(item.id,undefined);
    });
  };
  return (
    <>
      <svg className="envelope" data-testid="envelope" data-keys={keys.length} viewBox={`0 0 ${px} 100`} preserveAspectRatio="none" ref={ref}>
        {duckPath && <path className="env-duck" data-testid="duck-line" data-keys={duckKeys.length} d={duckPath} />}
        <path className="env-line" d={d} />
        {edit && <path className="env-hit" data-testid="envelope-line" d={d} onPointerDown={(e) => press(e)} />}
      </svg>
      {edit &&
        keys.map((k, index) => {
          const db = envelopeDbAt(view, k.frame);
          return (
            <div
              key={index}
              className={`env-key ${selectedKey === index ? "selected" : ""}`}
              data-testid="envelope-key"
              data-index={index}
              data-frame={k.frame}
              data-db={Math.round(db * 10) / 10}
              title={editorText("Volume key at {0} s: {1} dB (drag to move, double-click to remove)", { 0: (k.frame / fps).toFixed(2), 1: dbLabel(db) })}
              style={{ left: k.frame * ppf, top: `${levelY(db) * 100}%` }}
              onPointerDown={(e) => press(e, { index, frame: k.frame, db })}
              onDoubleClick={(e) => remove(e, index)}
            />
          );
        })}
    </>
  );
}
