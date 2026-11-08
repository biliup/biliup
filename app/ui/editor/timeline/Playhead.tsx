// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { useEffect, useRef, type RefObject } from "react";

import { getPlayback, usePlayback } from "@/app/ui/editor/preview/playback";

/**
 * The playhead: its own component, so playback re-renders only this. It lives in the ruler (sticky
 * at the top of the timeline, D115), so its handle stays in view when the tracks scroll; the line
 * reaches down over the visible tracks (--tl-view-h). A press on the handle is the ruler's scrub.
 */
export function Playhead({ ppf, bodyRef, headWidth, bodyWidth }: { ppf: number; bodyRef: RefObject<HTMLDivElement | null>; headWidth: number; bodyWidth: number }) {
  const frame = usePlayback((s) => s.frame);
  const x = frame * ppf;
  // Issue #118: the body's scroll position as its scroll events tell it. Reading scrollLeft (or
  // clientWidth) on every frame of playback made the browser lay out the whole timeline each time.
  const scrolled = useRef(0);
  useEffect(() => {
    const body = bodyRef.current;
    if (!body) return;
    scrolled.current = body.scrollLeft;
    const onScroll = () => (scrolled.current = body.scrollLeft);
    body.addEventListener("scroll", onScroll, { passive: true });
    return () => body.removeEventListener("scroll", onScroll);
  }, [bodyRef]);
  // While playing, keep the playhead in view.
  useEffect(() => {
    const body = bodyRef.current;
    if (!body || !getPlayback().playing) return;
    const view = bodyWidth - headWidth;
    const left = scrolled.current;
    if (x < left || x > left + view - 24) body.scrollLeft = scrolled.current = Math.max(0, x - 48);
  }, [x, bodyRef, headWidth, bodyWidth]);
  return (
    <div className="playhead" data-testid="playhead" data-frame={Math.floor(frame)} style={{ left: x }}>
      <div className="playhead-handle" />
    </div>
  );
}

/** Issue #118: the skimmer, a faint line at the frame under the pointer (the one the preview shows while skimming). */
export function Skimmer({ ppf }: { ppf: number }) {
  const hover = usePlayback((s) => s.hover);
  if (hover === undefined) return null;
  return <div className="skimmer" data-testid="skimmer" data-frame={hover} style={{ left: hover * ppf }} />;
}
