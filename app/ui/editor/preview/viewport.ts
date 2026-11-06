// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The preview's view (G6, D120): zoom (Fit, or canvas pixels per screen pixel: 0.5, 1, 2…) and
// pan, plus the full-window preview (G7b). Only how the editor shows the canvas: the project and
// the export never see it.
import type React from "react";
import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react";
import {beginEditorViewGesture} from '../workspace/pointerGesture';

export type PreviewZoom = "fit" | number;
export const ZOOM_STEPS: ReadonlyArray<number> = [0.5, 1, 2];
export const MIN_ZOOM = 0.05;
export const MAX_ZOOM = 8;

export interface Pan {
  readonly x: number;
  readonly y: number;
}

/**
 * The pan that keeps the canvas point under (cx, cy) (relative to the view's centre) in place when
 * the scale goes from `from` to `to`.
 */
export function zoomAround(pan: Pan, from: number, to: number, cx: number, cy: number): Pan {
  const k = to / from;
  return { x: cx - (cx - pan.x) * k, y: cy - (cy - pan.y) * k };
}

/** Keep at least `keep` px of the picture inside the view. */
export function clampPan(pan: Pan, picture: { w: number; h: number }, view: { w: number; h: number }, keep = 48): Pan {
  const mx = Math.max(0, (picture.w + view.w) / 2 - keep);
  const my = Math.max(0, (picture.h + view.h) / 2 - keep);
  return { x: Math.min(mx, Math.max(-mx, pan.x)), y: Math.min(my, Math.max(-my, pan.y)) };
}

/**
 * Wheel and hand gestures on the preview's area: ⌘ or ctrl + wheel (a trackpad pinch arrives as
 * ctrl + wheel) zooms around the pointer, and is never Electron's page zoom; a plain wheel pans
 * while zoomed in; with the hand on, a drag pans. `fitScale` is the scale Fit would use.
 */
export function useViewport(area: RefObject<HTMLDivElement | null>, fitScale: number, picture:{width:number;height:number}) {
  const [zoom, setZoomState] = useState<PreviewZoom>("fit");
  const [pan, setPan] = useState<Pan>({ x: 0, y: 0 });
  const [hand, setHand] = useState(false);
  const stop=useRef<(()=>void)|undefined>(undefined);
  useEffect(()=>()=>stop.current?.(),[]);
  useEffect(()=>{stop.current?.()},[hand,fitScale,picture.width,picture.height]);
  const scale = zoom === "fit" ? fitScale : zoom;
  const live = useRef({ scale, zoom, pan, fitScale, picture });
  live.current = { scale, zoom, pan, fitScale, picture };
  const bounded=(next:Pan,scale=live.current.scale)=>{
    const el=area.current,canvas=live.current.picture;
    return el?clampPan(next,{w:canvas.width*scale,h:canvas.height*scale},{w:el.clientWidth,h:el.clientHeight}):next;
  };
  const movePan=(next:Pan,scale=live.current.scale)=>{
    const pan=bounded(next,scale);live.current={...live.current,pan};setPan(pan);
  };
  useLayoutEffect(()=>{
    const el=area.current;if(!el)return;
    const clamp=()=>{
      const previous=live.current.pan,next=bounded(previous);
      if(next.x!==previous.x||next.y!==previous.y)movePan(next);
    };
    clamp();const observer=new ResizeObserver(clamp);observer.observe(el);
    return()=>observer.disconnect();
  },[area,fitScale,picture.width,picture.height]);

  const setZoom = (z: PreviewZoom) => {
    if(z!=='fit'&&!Number.isFinite(z))return;
    stop.current?.();
    const zoom=z==='fit'?z:Math.min(MAX_ZOOM,Math.max(MIN_ZOOM,z));
    const scale=zoom==='fit'?live.current.fitScale:zoom;
    movePan(zoom==='fit'?{x:0,y:0}:live.current.pan,scale);
    live.current={...live.current,zoom,scale};setZoomState(zoom);
  };

  useEffect(() => {
    const el = area.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      stop.current?.();
      const now = live.current;
      const unit=e.deltaMode===1?16:e.deltaMode===2?el.clientHeight:1;
      if (e.metaKey || e.ctrlKey) {
        e.preventDefault();
        if (!(now.fitScale > 0)) return;
        const r = el.getBoundingClientRect();
        const factor = Math.exp(-e.deltaY * unit * (e.ctrlKey && !e.metaKey ? 0.01 : 0.002));
        const to = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, now.scale * factor));
        const cx = e.clientX - (r.left + r.width / 2);
        const cy = e.clientY - (r.top + r.height / 2);
        movePan(zoomAround(now.pan, now.scale, to, cx, cy),to);
        live.current={...live.current,zoom:to,scale:to};
        setZoomState(to);
        return;
      }
      if (now.zoom === "fit") return;
      e.preventDefault();
      movePan({x:now.pan.x-e.deltaX*unit,y:now.pan.y-e.deltaY*unit});
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [area]);

  /**
   * A press with the hand on, or a middle-button press (U10) at any zoom: drag pans. Returns true
   * when it took the press. Only the view moves; the project never changes.
   */
  const handDown = (e: React.PointerEvent<HTMLElement>) => {
    if (!(e.button === 1 || (hand && e.button === 0))) return false;
    e.preventDefault();
    e.stopPropagation();
    stop.current?.();
    const start = { x: e.clientX, y: e.clientY, pan: live.current.pan };
    const target = e.currentTarget;
    const move = (ev: PointerEvent) => movePan({ x: start.pan.x + ev.clientX - start.x, y: start.pan.y + ev.clientY - start.y });
    // The release lands the pan where the pointer let go: a busy frame (the GPU compositor) can
    // coalesce the last pointermove away before the pointerup arrives.
    const finish = () => {
      stop.current=undefined;
      target.classList.remove("panning");
      target.removeEventListener('lostpointercapture',cancel);
      if(target.hasPointerCapture(e.pointerId))target.releasePointerCapture(e.pointerId);
    };
    const cancel=()=>stop.current?.();
    const pointer=beginEditorViewGesture(e,{move,commit:next=>{move(next);finish()},cancel:finish});
    if(!pointer)return true;
    stop.current=pointer;
    target.addEventListener('lostpointercapture',cancel);
    target.setPointerCapture(e.pointerId);
    target.classList.add('panning');
    return true;
  };

  return { zoom, scale, pan, hand, setZoom, setHand, handDown };
}
