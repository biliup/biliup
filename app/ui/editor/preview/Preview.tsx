import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { editorDialogOpen, editorEvents } from "@/app/lib/editor/environment";
import { editorRoot } from "@/app/lib/editor/environment";
import { isTextComposition, primaryModifier } from '@/app/lib/editor/keyboard';
import {beginEditorGesture,editorGestureOrigin} from '../workspace/pointerGesture';
import {useProjectEdit} from '../workspace/projectEdit';
import {currentEditorStore,projectOf} from '@/app/lib/editor/state/store/core';
import { keyHint } from '@/app/ui/editor/workspace/keymap';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The preview and the transport under it. The preview draws the project at the playhead at the
// canvas's aspect ratio: media through <video> (D37; <img> for stills), seeked to the source time
// of the frame (D38), each in its box (D75: a wrapper at core's box that clips, rounds and fades
// it, the picture inside at core's content rect), backgrounds (component.solid) between them in
// track order, text from the editor core's textHtml on top (D39), the canvas colour where nothing is.
// Components are core's componentHtml at the frame (D76: the same HTML the export rasterises);
// graphic items are their documents in sandboxed iframes, render(t) called by postMessage (D78).
// The text can be selected, dragged and edited in place (D53); a selected clip or component can be
// dragged and resized by its corners (D75); a component dropped from Elements lands where it's dropped.
import { isDrawnComponent } from "@/app/lib/editor/core/visual/components";
import { RENDER_CSS } from "@/app/lib/editor/core/visual/render";
import React, { useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react";

import { findItem, lockOf, planLayer } from "@/app/ui/editor/timeline/edit";
import { getPlayback, playback, usePlayback } from "@/app/ui/editor/preview/playback";
import { type ProjectModel } from "@/app/lib/editor/state/project";
import { actions, getState, subscribeApp } from "@/app/lib/editor/state/store";
import { TEXT_PRESETS } from "@/app/lib/editor/state/project";
import { useProxiesOn } from "@/app/ui/editor/settings/proxies";
import { useScrubCacheOn } from "@/app/ui/editor/settings/scrub";
import { useGpuCompositorOn } from "@/app/ui/editor/settings/gpu";
import { requestTextFocus } from "@/app/ui/editor/inspector/focus";
import { planText, planTextScale, planTransform, snapBox, textBox, type Guide } from "@/app/ui/editor/preview/text";
import { clipState, planClipBox, planResetPosition, resizeBox, type Corner } from "@/app/ui/editor/timeline/clip";
import { Transport } from "@/app/ui/editor/preview/Transport";
import { useViewport } from "@/app/ui/editor/preview/viewport";
import { activeEditorDrag, endEditorDrag, hasEditorDrag, overEditorDrag, readEditorDrag, subscribeEditorDrag } from "@/app/ui/editor/timeline/dnd";
import { openMenu } from "@/app/ui/editor/timeline/ContextMenu";
import { copySelection, cutSelection, deleteSelection, duplicate } from "@/app/ui/editor/workspace/keys";
import { useFit } from "@/app/ui/editor/preview/hooks/useFit";
import { Compositor } from "@/app/ui/editor/preview/compositor/Compositor";
import { type InlineEdit, InlineEditor } from "@/app/ui/editor/preview/stage/InlineEditor";
import { dropGhostHtml } from "@/app/ui/editor/preview/stage/dropGhost";
import { ClipSelection, TextSelection } from "@/app/ui/editor/preview/stage/selection";
import { MaskSelection } from "@/app/ui/editor/preview/stage/MaskSelection";
import { AgentChangePill } from "@/app/ui/editor/preview/stage/AgentChangePill";
import { MediaSkim } from "@/app/ui/editor/preview/stage/MediaSkim";
import { RegionPins, RegionSelect } from "@/app/ui/editor/preview/stage/Region";

interface CanvasDrag {
  readonly id: string;
  /** A text move (D53), a clip move or a clip's corner (D75). */
  readonly kind: "text" | "clip" | Corner;
  /** Issue #72 (D242): a text's corner (kind "text"): the drag resizes it, from its drawn box (x, y, w, h). */
  readonly textCorner?: Corner;
  readonly pointer: number;
  readonly px: number;
  readonly py: number;
  readonly x: number;
  readonly y: number;
  /** The box in canvas pixels, for snapping. */
  readonly w: number;
  readonly h: number;
  /** How far the drawn box's top is below y (captions are drawn from their bottom edge, D82): snapping uses the drawn box. */
  readonly oy?: number;
  /** The playhead when it began: a keyed clip changes its keys there. */
  readonly frame: number;
  moved: boolean;
  pending?: boolean;
  to?: { x: number; y: number; w?: number; h?: number; k?: number };
}

/** Pointer travel (screen px) before a press on a text becomes a drag, and the snap distance. */
const DRAG_PX = 3;
const SNAP_SCREEN_PX = 8;

/**
 * Keyed on the project folder by its parent: another project gets a fresh preview. On the canvas
 * (D53): a click on a text selects it, dragging moves it (snapping to the centre lines and safe
 * margins, ⌘ to move freely; one edit on release), a double click edits it in place (Enter
 * saves, ⇧Enter is a new line, Esc cancels, clicking away saves).
 */
export function Stage({ project, dir }: { project: ProjectModel; dir: string }) {
  useEditorLanguage();
  const historyProject = usePlayback(s => s.draft?.kind === 'history' && s.draft.base === project ? s.draft.project : undefined);
  const historical = !!historyProject;
  const { width, height } = (historyProject ?? project).canvas;
  const { ref, size: fitted } = useFit(width, height);
  // Zoom and pan (G6): Fit is the fitted size; a zoom is canvas px per screen px.
  const view = useViewport(ref, fitted.w / width, {width,height});
  const size = view.zoom === "fit" ? fitted : { w: Math.round(width * view.scale), h: Math.round(height * view.scale) };
  // The full-window preview (G7b): Esc or the button again leaves it.
  const [full, setFull] = useState(false);
  const fullOwner=useRef<{root:HTMLElement;native:boolean}|undefined>(undefined);
  const leaveFull=()=>{
    const owner=fullOwner.current;fullOwner.current=undefined;setFull(false);
    if(owner&&document.fullscreenElement===owner.root)void document.exitFullscreen().catch(()=>{});
  };
  const toggleFull=()=>{
    if(fullOwner.current){leaveFull();return;}
    const owner={root:editorRoot(),native:false};fullOwner.current=owner;setFull(true);
    // Request while the button's user activation is live. CSS full preview also works when
    // native fullscreen is unavailable or refused by the embedding browser.
    if(document.fullscreenEnabled)void owner.root.requestFullscreen().then(()=>{
      if(fullOwner.current!==owner&&fullOwner.current?.root!==owner.root&&document.fullscreenElement===owner.root)void document.exitFullscreen().catch(()=>{});
    }).catch(()=>{});
  };
  useEffect(()=>{
    const changed=()=>{
      const owner=fullOwner.current;if(!owner)return;
      if(document.fullscreenElement===owner.root)owner.native=true;
      else if(owner.native){fullOwner.current=undefined;setFull(false)}
    };
    document.addEventListener('fullscreenchange',changed);
    return()=>{
      document.removeEventListener('fullscreenchange',changed);
      const owner=fullOwner.current;fullOwner.current=undefined;
      if(owner&&document.fullscreenElement===owner.root)void document.exitFullscreen().catch(()=>{});
    };
  },[]);
  useEffect(() => {
    if (!full) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || isTextComposition(e) || e.key !== "Escape" || editorDialogOpen() || (e.target as HTMLElement | null)?.closest?.(".pv-inline") || getState().maskEditing) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      leaveFull();
    };
    editorEvents.addEventListener("keydown", onKey, true);
    return () => editorEvents.removeEventListener("keydown", onKey, true);
  }, [full]);
  const frameRef = useRef<HTMLDivElement>(null);
  const mediaRef = useRef<HTMLDivElement>(null);
  const textRef = useRef<HTMLDivElement>(null);
  const compositor = useRef<Compositor | undefined>(undefined);
  const drag = useRef<CanvasDrag | undefined>(undefined);
  const stopDrag=useRef<(()=>void)|undefined>(undefined);
  const edit=useProjectEdit();
  useEffect(()=>()=>stopDrag.current?.(),[]);
  const [guides, setGuides] = useState<ReadonlyArray<Guide>>([]);
  // A tile dragged over the canvas (X2, D132): the element as it would land there, drawn faint with an outline.
  const [dropGhost, setDropGhost] = useState<string | undefined>(undefined);
  useEffect(() => subscribeEditorDrag(() => {
    if (!activeEditorDrag()) setDropGhost(undefined);
  }), []);
  const [editing, setEditing] = useState<InlineEdit | undefined>();
  // Issue #118: the Area tool (a rectangle over the picture into the chat).
  const [region, setRegion] = useState(false);
  const endRegion = useCallback(() => setRegion(false), []);
  useEffect(() => {
    if (!historical) return;
    stopDrag.current?.();
    setEditing(undefined); setDropGhost(undefined); setGuides([]); setRegion(false);
  }, [historical]);
  useEffect(() => {
    const c = new Compositor(dir, frameRef.current!, mediaRef.current!, textRef.current!);
    compositor.current = c;
    return () => {
      c.destroy();
      compositor.current = undefined;
    };
  }, [dir]);
  useEffect(() => compositor.current?.setProject(project), [project]);
  // D212: a proxy that's ready (or the setting turned) swaps the clip's element for the other file.
  const proxies = useSyncExternalStore(subscribeApp, () => getState().proxies);
  const useProxies = useProxiesOn();
  // D221: the scrub cache or the GPU compositor turned on or off draws again.
  const scrubCache = useScrubCacheOn();
  const gpu = useGpuCompositorOn();
  useEffect(() => compositor.current?.render(), [proxies, useProxies, scrubCache, gpu]);
  useEffect(() => compositor.current?.setEditing(editing?.id), [editing?.id]);
  // Canvas pixels per screen pixel, and its inverse: how the canvas-size layer is scaled.
  const scale = width / Math.max(1, size.w);
  const fit = size.w / width;
  useEffect(()=>{stopDrag.current?.()},[project,dir,scale,view.pan.x,view.pan.y]);

  /** The text item under an event, if it can be moved or edited by hand. */
  const editableText = (target: EventTarget, at?: { clientX: number; clientY: number }) => {
    const inside = (n: HTMLElement) => {
      const r = n.getBoundingClientRect();
      return !!at && at.clientX >= r.left && at.clientX <= r.right && at.clientY >= r.top && at.clientY <= r.bottom;
    };
    const el =
      (target as HTMLElement).closest<HTMLElement>(".pv-text[data-item]") ??
      (at ? document.elementsFromPoint(at.clientX, at.clientY).find((n): n is HTMLElement => n instanceof HTMLElement && n.matches(".pv-text[data-item]")) : undefined) ??
      // F20 (D158): a text mid-way through a reveal (clipped) still takes a double click inside its box; the selected one first.
      (at ? ([...editorRoot().querySelectorAll<HTMLElement>(".pv-text.sel-box[data-item]"), ...editorRoot().querySelectorAll<HTMLElement>(".pv-text[data-item]")].find(inside) ?? undefined) : undefined);
    const placed = findItem(project, el?.dataset.item);
    // Captions (D82) draw as text: they move like it, but their words are edited in the Inspector.
    if (!el || !placed || (placed.item.type !== "text" && placed.item.type !== "captions") || lockOf(placed)) return undefined;
    return { el, item: placed.item };
  };

  /** A clip or component that can be moved or resized by hand (D75, D76): selected, unlocked, drawn at the playhead. */
  const editableClip = (id: string | undefined) => {
    const placed = findItem(project, id);
    if (!placed || (placed.item.type !== "clip" && !isDrawnComponent(placed.item)) || lockOf(placed)) return undefined;
    const state = clipState(project, placed.item.id, getPlayback().frame);
    return state?.inside ? state : undefined;
  };

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (historical) return;
    // Presses in the in-place editor are its own (caret, selection).
    if (e.button !== 0 || (e.target as HTMLElement).closest(".pv-inline")) return;
    stopDrag.current?.();
    const target = e.target as HTMLElement;
    const before = getPlayback().selected;
    const corner = target.closest<HTMLElement>("[data-handle]")?.dataset.handle as Corner | undefined;
    if (!corner) playback.select(target.closest<HTMLElement>("[data-item]")?.dataset.item);
    const origin=editorGestureOrigin();
    if (editing || !origin || origin.dir !== dir || projectOf(origin.owner.getSnapshot().snapshot!)!==project) return;
    const start = { pointer: e.pointerId, px: e.clientX, py: e.clientY, frame: Math.round(getPlayback().frame), moved: false };
    // A clip's corner handle, or a press on the clip that was already selected: move or resize it.
    const clipId = corner ? before : target.closest<HTMLElement>(".pv-box[data-item], .pv-comp[data-item]")?.dataset.item;
    const clip = clipId !== undefined && clipId === before ? editableClip(clipId) : undefined;
    if (clip) {
      e.stopPropagation();
      beginDrag(e,{ ...start, id: clip.item.id, kind: corner ?? "clip", x: clip.box.x, y: clip.box.y, w: clip.box.w, h: clip.box.h });
      return;
    }
    // Issue #72 (D242): the selected text's corner resizes it.
    const textCorner = target.closest<HTMLElement>("[data-text-handle]")?.dataset.textHandle as Corner | undefined;
    if (textCorner) {
      const placed = findItem(project, before);
      const el = before === undefined ? null : textRef.current?.querySelector<HTMLElement>(`.pv-text[data-item="${CSS.escape(before)}"]`);
      if (!placed || placed.item.type !== "text" || lockOf(placed) || !el || el.offsetWidth <= 0) return;
      e.stopPropagation();
      beginDrag(e,{ ...start, id: placed.item.id, kind: "text", textCorner, x: el.offsetLeft, y: el.offsetTop, w: el.offsetWidth, h: el.offsetHeight });
      return;
    }
    const hit = editableText(e.target);
    if (!hit) return;
    const box = textBox(hit.item, project);
    beginDrag(e,{
      ...start,
      id: hit.item.id,
      kind: "text",
      x: box.x,
      y: box.y,
      // Laid out at canvas size, so its offset size is already in canvas pixels.
      w: hit.el.offsetWidth,
      h: hit.el.offsetHeight,
      // A captions card is drawn from its bottom edge: its drawn top isn't the item's y.
      oy: hit.item.type === "captions" ? hit.el.offsetTop - box.y : 0,
    });
  };

  /** The edit a drag makes, planned on `p`. */
  const dragPlan = (p: ProjectModel, d: CanvasDrag, to: NonNullable<CanvasDrag["to"]>) =>
    d.textCorner ? planTextScale(p, d.id, to.k ?? 1, d.textCorner, { x: d.x, y: d.y, w: d.w, h: d.h }) : d.kind === "text" ? planTransform(p, d.id, { x: to.x, y: to.y }) : d.kind === "clip" ? planClipBox(p, d.id, { x: to.x, y: to.y }, d.frame) : planClipBox(p, d.id, to, d.frame);

  const moveDrag = (e: PointerEvent,d:CanvasDrag,target:HTMLDivElement,finishing=false) => {
    if (drag.current!==d || d.pending) return;
    if (!finishing&&e.buttons===0) return stopDrag.current?.();
    const dx = e.clientX - d.px;
    const dy = e.clientY - d.py;
    if (!d.moved && Math.hypot(dx, dy) < DRAG_PX) return;
    if (!d.moved) {
      playback.pause();
      // Captured only once it's a drag: capturing on press would retarget the clicks (and the
      // double click that edits in place) to the layer.
      if(!finishing)target.setPointerCapture(e.pointerId);
    }
    d.moved = true;
    let shown: ReadonlyArray<Guide> = [];
    if (d.textCorner) {
      // Issue #72 (D242): the box keeps its shape (a text has no height of its own); its width's factor is the font size's.
      const r = resizeBox({ x: d.x, y: d.y, w: d.w, h: d.h }, d.textCorner, dx * scale, dy * scale, false);
      d.to = { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.w), h: Math.round(r.h), k: r.w / d.w };
    } else if (d.kind === "text" || d.kind === "clip") {
      let x = Math.round(d.x + dx * scale);
      let y = Math.round(d.y + dy * scale);
      if (!primaryModifier(e) && getPlayback().snapping) {
        const oy = d.oy ?? 0;
        const snapped = snapBox({ x, y: y + oy, w: d.w, h: d.h }, project, SNAP_SCREEN_PX * scale);
        x = snapped.x;
        y = snapped.y - oy;
        shown = snapped.guides;
      }
      d.to = { x, y };
    } else {
      // A corner (D75): the opposite corner stays, the box keeps its shape unless ⇧ is held.
      const r = resizeBox({ x: d.x, y: d.y, w: d.w, h: d.h }, d.kind, dx * scale, dy * scale, e.shiftKey);
      d.to = { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.w), h: Math.round(r.h) };
    }
    edit.preview(project,dragPlan(project,d,d.to));
    setGuides(shown);
  };

  const beginDrag=(event:React.PointerEvent<HTMLDivElement>,d:CanvasDrag)=>{
    const target=event.currentTarget;
    drag.current=d;
    const release=()=>{if(target.hasPointerCapture(d.pointer))target.releasePointerCapture(d.pointer)};
    const clear=()=>{
      if(drag.current!==d)return;
      drag.current=undefined;stopDrag.current=undefined;
      release();setGuides([]);edit.cancel();
    };
    const pointer=beginEditorGesture(event,{
      move:next=>moveDrag(next,d,target),
      cancel:clear,
      commit:(origin,next)=>{
        moveDrag(next,d,target,true);
        if(drag.current!==d||!d.moved||!d.to){clear();return}
        const to=d.to;d.pending=true;
        release();setGuides([]);
        void edit.commit(current=>dragPlan(current,d,to),origin).finally(clear);
      },
    });
    if(pointer)stopDrag.current=()=>{pointer();clear()};else clear();
  };

  const onDoubleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (historical) return;
    stopDrag.current?.();
    const origin=editorGestureOrigin();
    if(!origin||origin.dir!==dir||projectOf(origin.owner.getSnapshot().snapshot!)!==project)return;
    const hit = editableText(e.target, e);
    const snapshot = getState().snapshot;
    // F23 (D162): an element (lower third...) has no in-place editor: its main text field in the Inspector takes the focus.
    const comp = hit ? undefined : findItem(project, (e.target as HTMLElement).closest<HTMLElement>(".pv-comp[data-item]")?.dataset.item);
    if (comp && isDrawnComponent(comp.item)) {
      playback.pause();
      playback.select(comp.item.id);
      actions.openPanel("inspector");
      requestTextFocus(comp.item.id);
      return;
    }
    if (!hit || hit.item.type !== "text" || !snapshot || snapshot.dir !== dir) return;
    playback.pause();
    setEditing({ id: hit.item.id, css: hit.el.getAttribute("style") ?? "", text: hit.item.text ?? "", origin });
  };

  /** A component dragged from Elements (D76): dropped on the canvas, it's added at the playhead, centred where it lands. */
  /** The clip on the canvas under a pointer (an Effects tile dropped there goes on it, D136): the topmost box there. */
  const clipAt = (x: number, y: number) => document.elementsFromPoint(x, y).find((el): el is HTMLElement => el instanceof HTMLElement && !!frameRef.current?.contains(el) && el.matches(".pv-box[data-item]"))?.dataset.item;
  const onDragOver = (e: React.DragEvent<HTMLDivElement>) => {
    if (historical) return;
    if (getState().connection !== 'connected' || !getState().biliupCanEdit) return;
    const value = overEditorDrag(e.dataTransfer);
    if (value?.kind === 'effect') {
      if (!clipAt(e.clientX, e.clientY)) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
      return;
    }
    if (value?.kind !== 'component' && value?.kind !== 'text') return;
    const rect = frameRef.current?.getBoundingClientRect();
    if (!rect) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    const at = { x: (e.clientX - rect.left) * scale, y: (e.clientY - rect.top) * scale };
    const ghost = dropGhostHtml(project, value.kind === 'text' ? { text: value.id } : { component: value.id, props: value.props }, at);
    if (ghost !== dropGhost) setDropGhost(ghost);
  };
  const onDragLeave = (e: React.DragEvent<HTMLDivElement>) => {
    if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setDropGhost(undefined);
  };
  const onDrop = (e: React.DragEvent<HTMLDivElement>) => {
    if (historical) { e.preventDefault(); return; }
    setDropGhost(undefined);
    if (!hasEditorDrag([...e.dataTransfer.types])) return;
    e.preventDefault();
    const value = readEditorDrag(e.dataTransfer);
    if (!value) return;
    endEditorDrag();
    if (getState().connection !== 'connected' || !getState().biliupCanEdit) return;
    if (value.kind === 'effect') {
      const id = clipAt(e.clientX, e.clientY);
      if (id) void actions.applyFx(value.id, id);
      return;
    }
    const rect = frameRef.current?.getBoundingClientRect();
    if (!rect) return;
    const at = { x: (e.clientX - rect.left) * scale, y: (e.clientY - rect.top) * scale };
    // A Text tab preset (D80): added at the playhead, where it's dropped.
    if (value.kind === 'text') {
      const preset = TEXT_PRESETS.find(p => p.id === value.id);
      if (preset) void actions.addText(preset, { at });
      return;
    }
    if (value.kind === 'component') void actions.addComponent(value.id, { at, props: value.props });
  };

  const finishEdit = (text: string | undefined) => {
    const edit = editing;
    setEditing(undefined);
    if (!edit || text === undefined || getPlayback().draft?.kind === 'history' || currentEditorStore()!==edit.origin.owner) return;
    void actions.itemEdit((p) => planText(p, edit.id, text), edit.origin);
  };

  return (
    <main className={`stage ${full ? "fullscreen" : ""}`} data-testid="stage" data-full={full ? "true" : undefined}>
      <style>{RENDER_CSS}</style>
      <div className={`stage-canvas ${view.hand ? "hand" : ""}`} ref={ref} data-testid="stage-canvas" data-zoom={view.zoom === "fit" ? "fit" : view.scale.toFixed(3)} onPointerDownCapture={(e) => view.handDown(e)} onMouseDownCapture={(e) => e.button === 1 && e.preventDefault()} onAuxClick={(e) => e.button === 1 && e.preventDefault()}>
        <div
          className="frame"
          ref={frameRef}
          data-testid="preview-frame"
          data-frame="0"
          style={{ width: size.w, height: size.h, transform: view.pan.x || view.pan.y ? `translate(${view.pan.x}px, ${view.pan.y}px)` : undefined }}
          onPointerDown={onPointerDown}
          onContextMenu={(e) => {
            if (historical) { e.preventDefault(); return; }
            // D141: right-click a picture or a text on the canvas: Add to chat (with the frame it
            // was picked at), and the usual Copy, Duplicate and Delete.
            const id = (e.target as HTMLElement).closest<HTMLElement>("[data-item]")?.dataset.item;
            if (!id || !findItem(project, id)) return;
            e.preventDefault();
            playback.select(id);
            const frame = Math.floor(getPlayback().frame);
            const off = getState().connection !== "connected" || !getState().biliupCanEdit;
            // F15 (D151): also Split at the playhead, Detach audio, the layer order and Reset position.
            const placed = findItem(project, id)!;
            const { item, track, start } = placed;
            const locked = !!(track.locked || item.locked) || off;
            const media = item.media ? project.media.find((m) => m.id === item.media) : undefined;
            const layered = track.role !== "main" && item.type !== "captions";
            const forward = layered ? planLayer(project, id, -1) : undefined;
            const backward = layered ? planLayer(project, id, 1) : undefined;
            const reset = planResetPosition(project, id);
            openMenu(e.clientX, e.clientY, [
              { id: "add-to-chat", get label() { return editorText("Add to chat"); }, run: () => actions.addItemToChat(id, frame) },
              "-",
              { id: "split", get label() { return editorText("Split at playhead"); }, ...keyHint('split'), disabled: locked || !(frame > start && frame < start + item.duration), run: () => void actions.split(id, frame) },
              ...(item.type === "clip" && media?.kind === "video" && media.probe?.hasAudio ? [{ id: "detach-audio", label: item.mute ? "Audio detached" : "Detach audio", disabled: locked || !!item.mute, run: () => void actions.detachAudio(id) }] : []),
              "-",
              { id: "bring-forward", get label() { return editorText("Bring forward"); }, disabled: locked || !forward || "problem" in forward, run: () => void actions.layer(id, -1) },
              { id: "send-backward", get label() { return editorText("Send backward"); }, disabled: locked || !backward || "problem" in backward, run: () => void actions.layer(id, 1) },
              { id: "reset-position", get label() { return editorText("Reset position"); }, disabled: locked || "problem" in reset, run: () => void actions.resetPosition(id) },
              "-",
              { id: "copy", get label() { return editorText("Copy"); }, ...keyHint('copy'), run: () => void copySelection([id]) },
              { id: "cut", get label() { return editorText("Cut selection"); }, ...keyHint('cut'), disabled: locked, run: () => void cutSelection([id]) },
              { id: "duplicate", get label() { return editorText("Duplicate"); }, ...keyHint('duplicate'), disabled: locked, run: () => void duplicate([id]) },
              { id: "delete", get label() { return editorText("Delete"); }, ...keyHint('delete'), danger: true, disabled: locked, run: () => void deleteSelection([id]) },
            ], "Canvas item");
          }}
          onLostPointerCapture={e=>{if(drag.current?.pointer===e.pointerId&&!drag.current.pending)stopDrag.current?.()}}
          onDoubleClick={onDoubleClick}
          // Issue #61 (D233): nothing on the canvas starts a native drag (an <img>'s ghost copy
          // would cancel the pointer drag that moves the layer); the in-place editor's text may.
          onDragStart={(e) => !(e.target as HTMLElement).closest?.(".pv-inline") && e.preventDefault()}
          onDragOver={onDragOver}
          onDragLeave={onDragLeave}
          onDrop={onDrop}
        >
          {/* Everything is laid out at canvas size and scaled down as one (D56): the text's
              cqh sizes, shadows and line breaks resolve exactly as in the export's canvas-size
              page, whatever the preview's size (the system font's tracking depends on size). */}
          <div className="pv-canvas" style={{ width, height, transform: `scale(${fit})`, ["--pv-scale" as string]: fit || 1 }}>
            <div className="pv-layer" ref={mediaRef} data-testid="preview-media" />
            <div className="pv-layer pv-texts" ref={textRef} data-testid="preview-text" />
            <div className="pv-layer pv-overlay">
            {guides.map((g) => (
              <div
                key={`${g.axis}-${g.at}`}
                className={`pv-guide ${g.axis} ${g.kind}`}
                data-testid="canvas-guide"
                data-axis={g.axis}
                data-kind={g.kind}
                style={g.axis === "x" ? { left: `${(g.at / width) * 100}%` } : { top: `${(g.at / height) * 100}%` }}
              />
            ))}
            {dropGhost && <div className="pv-drop-ghost" data-testid="drop-ghost" dangerouslySetInnerHTML={{ __html: dropGhost }} />}
            {!historical && editing && <InlineEditor key={editing.id} edit={editing} onDone={finishEdit} />}
            {!historical && <ClipSelection project={project} />}
            {!historical && !editing && <TextSelection layer={textRef} project={project} />}
            {!historical && !editing && <MaskSelection project={project} dir={dir} disabled={region||view.hand}/>} 
            </div>
          </div>
          {!historical && <MediaSkim />}
          {!historical && <RegionPins project={project} />}
          {!historical && region && <RegionSelect project={project} onDone={endRegion} />}
        </div>
      </div>
      {!historical && <AgentChangePill fps={project.canvas.fps} />}
      <Transport project={historyProject ?? project} view={view} full={full} onFull={toggleFull} region={region} onRegion={historical ? undefined : () => setRegion(!region)} />
    </main>
  );
}
