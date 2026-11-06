// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Wheel and pinch on the timeline (T2, D126): ⌘ + wheel or a trackpad pinch (it arrives as ctrl +
// wheel) zooms around the pointer, keeping the frame under it in place; a plain wheel scrolls (the
// browser's own scrolling, vertical, or horizontal with a sideways swipe); ⇧ + wheel scrolls
// sideways with a mouse wheel too. A native, non-passive listener, so the page never zooms.
import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";

import { getPlayback, playback } from "@/app/ui/editor/preview/playback";

/** The zoom a wheel step asks for: a pinch (small deltas) and a mouse wheel (±100) both feel even. */
export function wheelZoom(zoom: number, deltaY: number, deltaMode = 0): number {
  const px = deltaMode === 1 ? deltaY * 16 : deltaMode === 2 ? deltaY * 400 : deltaY;
  return zoom * Math.exp(-Math.max(-200, Math.min(200, px)) * 0.006);
}

/**
 * Where the lanes must scroll so the frame that was under the pointer stays there after the zoom:
 * `pointerX` is the pointer's x from the lanes' visible left edge.
 */
export const anchoredScroll = (frame: number, ppf: number, pointerX: number) => Math.max(0, frame * ppf - pointerX);

export function useTimelineWheel(bodyRef: RefObject<HTMLDivElement | null>, headWidth: number, ppf: number) {
  const ppfRef = useRef(ppf);
  ppfRef.current = ppf;
  // The zoom anchor: the frame under the pointer and the pointer's x in the lanes, until the new ppf is drawn.
  const anchor = useRef<{ frame: number; x: number } | undefined>(undefined);
  useEffect(() => {
    const el = bodyRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const rect = el.getBoundingClientRect();
        const x = Math.max(0, e.clientX - rect.left - headWidth);
        anchor.current = { frame: (el.scrollLeft + x) / ppfRef.current, x };
        playback.setZoom(wheelZoom(getPlayback().zoom, e.deltaY, e.deltaMode));
        return;
      }
      // ⇧ + a mouse wheel: sideways (a trackpad's own sideways swipe already scrolls the browser's way).
      if (e.shiftKey && Math.abs(e.deltaX) < Math.abs(e.deltaY)) {
        e.preventDefault();
        el.scrollLeft += e.deltaY;
      }
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [bodyRef, headWidth]);
  useLayoutEffect(() => {
    const el = bodyRef.current;
    const a = anchor.current;
    if (!el || !a) return;
    anchor.current = undefined;
    el.scrollLeft = anchoredScroll(a.frame, ppf, a.x);
  }, [ppf, bodyRef]);
}
