import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import {matchesEditorAsset} from '@/app/lib/editor/asset-search';
import { editorEvents } from "@/app/lib/editor/environment";
import { editorRoot } from "@/app/lib/editor/environment";
import { isTextComposition } from '@/app/lib/editor/keyboard';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Transitions tab (D73, D133): every transition the preview and export draw (core TRANSITIONS),
// each a tile with an animated thumbnail drawn by the same core maths as the preview (incomingFx /
// outgoingFx, the iris radius, the mask outlines) that plays only while the tile is hovered or
// focused (a still mid-transition otherwise, D240), and a search. No kind filters or length chips
// (D240): a transition's length is set on the cut (Inspector, the marker's edges), and the length new
// ones get is in Settings › App. Drag a tile onto a cut between two main-track clips; or select a cut and click the tile; or
// + puts it on the cut nearest the playhead.
import { defaultFrames, incomingFx, irisRadius, maskClipPath, outgoingFx, TRANSITIONS, type TransitionFx, type TransitionInfo, type TransitionKind } from "@/app/lib/editor/core/visual/transitions";
import { Plus, X } from "lucide-react";
import React, { useEffect, useLayoutEffect, useRef, useState } from "react";

import { actions, useApp, type Project } from "@/app/lib/editor/state/store";
import { activeEditorDrag, beginEditorDrag, dropTransition, setTilePointer, subscribeEditorDrag } from "@/app/ui/editor/timeline/dnd";
import { usePlayback } from "@/app/ui/editor/preview/playback";
import { cutFit, findItem, secondsLabel } from "@/app/ui/editor/timeline/edit";
import { useTransitionSeconds } from "@/app/ui/editor/timeline/transitionLength";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

/** The thumbnail's own canvas (px): the shapes are drawn on it, then it's scaled into the tile. */
const THUMB = { width: 160, height: 90 } as const;
/** One loop: A held, the transition, B held (ms). */
const HOLD = 500;
const RUN = 1300;
const LOOP = 2 * HOLD + RUN;

// One clock for every tile, so 28 thumbnails share one animation frame.
const ticks = new Set<(now: number) => void>();
let raf = 0;
const loop = (now: number) => {
  for (const tick of ticks) tick(now);
  raf = ticks.size ? requestAnimationFrame(loop) : 0;
};
function onTick(tick: (now: number) => void) {
  ticks.add(tick);
  if (!raf) raf = requestAnimationFrame(loop);
  return () => void ticks.delete(tick);
}

const px = (n: number) => `${Math.round(n * 1000) / 1000}px`;
/** The style a clip gets from core's TransitionFx, as the preview applies it (transform about its centre, clip-path, blur). */
function styleOf(el: HTMLElement, fx: TransitionFx | undefined) {
  if (!fx) {
    el.style.visibility = "hidden";
    return;
  }
  el.style.visibility = "";
  el.style.opacity = String(fx.opacity);
  el.style.translate = fx.shift || fx.shiftY ? `${px(fx.shift ?? 0)} ${px(fx.shiftY ?? 0)}` : "";
  el.style.scale = fx.zoom !== undefined && fx.zoom !== 1 ? String(fx.zoom) : "";
  el.style.filter = fx.blur ? `blur(${px(fx.blur)})` : "";
  el.style.clipPath =
    fx.wipe !== undefined
      ? `inset(0 ${px((1 - fx.wipe) * THUMB.width)} 0 0)`
      : fx.iris !== undefined
        ? `circle(${px(irisRadius(fx.iris, THUMB))} at ${px(THUMB.width / 2)} ${px(THUMB.height / 2)})`
        : fx.mask
          ? maskClipPath(fx.mask.shape, fx.mask.p, THUMB)
          : "";
}

/**
 * Draws a transition at time `ms` of its loop on the thumbnail's two shots and colour layer: A, then
 * the transition at share p (linear, as core draws it), then B.
 */
function drawAt(t: TransitionInfo, ms: number, a: HTMLElement, b: HTMLElement, k: HTMLElement) {
  const at = ms % LOOP;
  const p = Math.min(1, Math.max(0, (at - HOLD) / RUN));
  const running = p > 0 && p < 1;
  if (!t.overlaps) {
    // A dip: A over the first half, B over the second, the colour layer up to the cut and back.
    styleOf(a, p < 0.5 ? { opacity: 1 } : undefined);
    styleOf(b, p >= 0.5 ? { opacity: 1 } : undefined);
  } else {
    styleOf(a, p < 1 ? (running ? outgoingFx(t.id, p, THUMB) : { opacity: 1 }) : undefined);
    styleOf(b, p > 0 ? (running ? incomingFx(t.id, p, THUMB) : { opacity: 1 }) : undefined);
  }
  k.style.background = t.color ?? "transparent";
  k.style.opacity = t.color && running ? String(1 - Math.abs(2 * p - 1)) : "0";
}

/** A tile's preview (D133, D240): a still mid-transition, looping from the start while `playing` (the tile hovered or focused); always the still with reduced motion. */
function TransitionThumb({ t, playing }: { t: TransitionInfo; playing: boolean }) {
  useEditorLanguage();
  const box = useRef<HTMLDivElement>(null);
  const stage = useRef<HTMLDivElement>(null);
  const a = useRef<HTMLSpanElement>(null);
  const b = useRef<HTMLSpanElement>(null);
  const k = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const fit = () => {
      if (stage.current) stage.current.style.scale = String(el.clientWidth / THUMB.width || 1);
    };
    fit();
    const observer = new ResizeObserver(fit);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    const draw = (ms: number) => a.current && b.current && k.current && drawAt(t, ms, a.current, b.current, k.current);
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    // The still: the middle of the transition (a dip's colour at its fullest).
    draw(HOLD + RUN * (t.overlaps ? 0.5 : 0.45));
    if (still || !playing) return;
    const start = performance.now();
    return onTick((now) => draw(now - start));
  }, [t, playing]);
  return (
    <div className="thumb tr-thumb" ref={box} data-testid="transition-thumb" data-playing={playing ? "true" : undefined}>
      <div className="tr-stage" ref={stage} style={{ width: THUMB.width, height: THUMB.height }}>
        {/* Two shots, A then B, and the colour of a dip or flash over them. */}
        <span className="tr-a" ref={a} aria-hidden="true">
          <i />
        </span>
        <span className="tr-b" ref={b} aria-hidden="true">
          <i />
        </span>
        <span className="tr-k" ref={k} aria-hidden="true" />
      </div>
    </div>
  );
}

/** The transitions matching a search (name or description words) and, optionally, a kind. */
export function filterTransitions(query: string, kind?: TransitionKind): Array<TransitionInfo> {
  return TRANSITIONS.filter((t) => (!kind || t.kind === kind) && matchesEditorAsset(query,t.name,t.description));
}

// A press that moves this far becomes a drag (a click puts the tile on the selected cut).
const DRAG_START_PX = 4;

/**
 * Drag a tile onto a cut with the pointer (F16, D154): past DRAG_START_PX the tile's thumbnail
 * follows the pointer as a chip, the timeline highlights the cut it would land on with a ghost of
 * its length, and releasing over the timeline puts it there (one edit); Esc cancels.
 */
function startTileDrag(e: React.PointerEvent<HTMLElement>, t: TransitionInfo, onDragged: () => void) {
  if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
  const root = editorRoot();
  const release = beginEditorDrag({kind:'transition',id:t.id});
  if (!release) return;
  const value = activeEditorDrag();
  if (!value) { release(); return; }
  const tile = e.currentTarget;
  const pointer = e.pointerId;
  const x0 = e.clientX;
  const y0 = e.clientY;
  let chip: HTMLElement | undefined;
  let finished = false;
  let unsubscribe = () => {};
  const move = (ev: PointerEvent) => {
    if (ev.pointerId !== pointer) return;
    if (activeEditorDrag() !== value) { cancel(); return; }
    if (!chip && Math.hypot(ev.clientX - x0, ev.clientY - y0) < DRAG_START_PX) return;
    if (!chip) {
      chip = document.createElement("div");
      chip.className = "drag-chip tile-drag";
      chip.dataset.testid = "transition-drag-chip";
      const thumb = tile.querySelector<HTMLElement>(".thumb");
      if (thumb) {
        const box = document.createElement("div");
        box.className = "drag-chip-box";
        const k = Math.min(1, 96 / Math.max(1, thumb.offsetWidth));
        box.style.width = `${Math.round(thumb.offsetWidth * k)}px`;
        box.style.height = `${Math.round(thumb.offsetHeight * k)}px`;
        const copy = thumb.cloneNode(true) as HTMLElement;
        copy.classList.add("drag-chip-thumb");
        copy.style.width = `${thumb.offsetWidth}px`;
        copy.style.height = `${thumb.offsetHeight}px`;
        copy.style.transform = `scale(${k})`;
        box.append(copy);
        chip.append(box);
      }
      const name = document.createElement("span");
      name.textContent = editorText(t.name);
      chip.append(name);
      root.append(chip);
      root.classList.add("tile-dragging");
    }
    chip.style.left = `${ev.clientX + 12}px`;
    chip.style.top = `${ev.clientY + 12}px`;
    setTilePointer({ x: ev.clientX, y: ev.clientY });
  };
  const end = (drop: PointerEvent | undefined) => {
    if (finished) return;
    finished = true;
    unsubscribe();
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    window.removeEventListener("pointercancel", pointerCancel);
    editorEvents.removeEventListener("keydown", key, true);
    if (chip) {
      onDragged();
      chip.remove();
      root.classList.remove("tile-dragging");
      if (drop && activeEditorDrag() === value) dropTransition(t.id, drop.clientX, drop.clientY);
    }
    release();
  };
  const up = (ev: PointerEvent) => { if (ev.pointerId === pointer) end(ev); };
  const cancel = () => end(undefined);
  const pointerCancel = (ev: PointerEvent) => { if (ev.pointerId === pointer) cancel(); };
  const key = (ev: KeyboardEvent) => {
    if (isTextComposition(ev) || ev.key !== "Escape") return;
    ev.preventDefault();
    ev.stopPropagation();
    cancel();
  };
  unsubscribe = subscribeEditorDrag(() => { if (activeEditorDrag() !== value) cancel(); });
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
  window.addEventListener("pointercancel", pointerCancel);
  editorEvents.addEventListener("keydown", key, true);
  return cancel;
}

export function TransitionsLibrary({ project }: { project: Project }) {
  useEditorLanguage();
  const { connection, biliupCanEdit } = useApp();
  const online = connection === "connected" && !!biliupCanEdit;
  const stopDrag = useRef<(() => void) | undefined>(undefined);
  const dragged = useRef(false);
  useEffect(() => () => stopDrag.current?.(), []);
  const fps = project.canvas.fps;
  const [query, setQuery] = useState("");
  // The tile whose thumbnail plays (hovered, else focused).
  const [hovered, setHovered] = useState<string | undefined>();
  const [focused, setFocused] = useState<string | undefined>();
  const playing = hovered ?? focused;
  // The length new transitions get (Settings › App › New items), else each one's own default.
  const seconds = useTransitionSeconds();
  const shown = filterTransitions(query);
  const apply = (t: TransitionInfo, e: React.MouseEvent, nearest: boolean) => {
    e.stopPropagation();
    (e.currentTarget as HTMLElement).blur();
    if (online) void actions.applyTransition(t.id, { nearest });
  };
  /** The length a tile puts on a cut: the chosen one for all (within its limits), else its own default. */
  const lengthOf = (t: TransitionInfo) => (seconds ? Math.min(t.maxFrames, Math.max(t.minFrames, Math.round(seconds * fps))) : defaultFrames(t, fps));
  // F9 (D153): with a cut selected, each tile says whether it fits there (and how long it can be).
  const cut = usePlayback((s) => s.cut);
  const cutPlaced = findItem(project, cut);
  const target = cutPlaced && cutPlaced.track.role === "main" && cutPlaced.index > 0 && !((cutPlaced.item.gap ?? 0) > 0) ? cutPlaced.item : undefined;
  /** The length a tile would get on the selected cut: the chosen one, else the one there now (kept on a change of type), else its default. */
  const wantedOn = (t: TransitionInfo) => (seconds ? lengthOf(t) : (target?.transitionIn?.duration ?? defaultFrames(t, fps)));
  return (
    <aside className="library" data-testid="transitions-library">
      <div className="lib-head">{editorText("Transitions")}</div>
      <div className="lib-section">
        <div className="music-filters">
          <div className="tr-search">
            <input className="field input" data-testid="transition-search" placeholder={editorText("Search transitions")} value={query} spellCheck={false} onChange={(e) => setQuery(e.target.value)} />
            {query && (
              <Tooltip label={editorText("Clear")}>
                <button className="icon-btn" data-testid="transition-search-clear" onClick={() => setQuery("")}>
                  <X />
                </button>
              </Tooltip>
            )}
          </div>
        </div>
        {shown.length === 0 && (
          <div className="muted small" data-testid="transition-empty">
            {editorText("No transition matches")}</div>
        )}
        <div className="grid2">
          {shown.map((t) => {
            const fit = target ? cutFit(project, target.id, t.id, wantedOn(t)) : undefined;
            return (
            <div
              key={t.id}
              className={`tile tr-tile ${fit && !fit.fits ? "unfit" : ""}`}
              data-testid="transition-tile"
              data-module={t.id}
              data-fit={fit ? (!fit.fits ? "no" : fit.clamped ? "clamped" : "yes") : undefined}
              role="button"
              tabIndex={0}
              aria-disabled={!online || (fit && !fit.fits)}
              draggable={false}
              title={fit && !fit.fits ? fit.problem : `${editorText(t.name)}: ${editorText(t.description)}`}
              onPointerEnter={() => setHovered(t.id)}
              onPointerLeave={() => setHovered((h) => (h === t.id ? undefined : h))}
              onFocus={() => setFocused(t.id)}
              onBlur={() => setFocused((f) => (f === t.id ? undefined : f))}
              onPointerDown={(e) => {
                stopDrag.current?.();
                dragged.current = false;
                if (online) stopDrag.current = startTileDrag(e, t, () => {
                  dragged.current = true;
                  setTimeout(() => { dragged.current = false; }, 0);
                });
              }}
              onClick={(e) => !dragged.current && apply(t, e, false)}
              onKeyDown={(e) => {
                // Keys on the + belong to it (its click), not to the tile as well.
                if (isTextComposition(e.nativeEvent) || e.target !== e.currentTarget || (e.key !== "Enter" && e.key !== " ")) return;
                e.preventDefault();
                if (online && !e.repeat) void actions.applyTransition(t.id);
              }}
            >
              <div className="tr-thumb-wrap">
                <TransitionThumb t={t} playing={playing === t.id} />
                <Tooltip label={editorText("Put a {0} on the cut nearest the playhead", { 0: editorText(t.name) })}>
                  <button className="tile-add" data-testid="transition-add" disabled={!online} onClick={(e) => apply(t, e, true)}>
                    <Plus />
                  </button>
                </Tooltip>
              </div>
              <div className="tile-name">{editorText(t.name)}</div>
              <div className="tile-sub" data-testid="transition-default">
                {fit && !fit.fits ? (
                  <span className="tr-unfit" data-testid="transition-unfit">
                    {fit.short}
                  </span>
                ) : fit?.clamped ? (
                  <span data-testid="transition-clamped" title={editorText("The clips at this cut leave room for {0}", { 0: secondsLabel(fit.max, fps) })}>
                    {secondsLabel(fit.max, fps)} {editorText(" max here")}</span>
                ) : (
                  secondsLabel(target ? wantedOn(t) : lengthOf(t), fps)
                )}
              </div>
            </div>
            );
          })}
        </div>
      </div>
    </aside>
  );
}
