// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Filmstrips on timeline clips (issue #53): the server's sprite of a video's frames (one every
// `interval` s, shared through the bounded preview cache), drawn as cells along the clip,
// each showing the frame at its own time (in point, speed, loop; zoom changes how many there are).
// Lazy: a clip's strip is asked for only once the clip is on screen, and only the cells in view
// are drawn. Until it's there the poster stands in, repeated, as before.
import { useFilmstrip } from "@/app/ui/editor/preview/resources";
import { useEffect, useRef, useState } from "react";

import type { Item, MediaItem } from "@/app/lib/editor/state/project";
import { filmstripUrl } from "@/app/lib/editor/state/store";
import { visibleCells } from "@/app/ui/editor/timeline/filmstripTiles";

/** The part of `el` (px from its left edge) inside its scrolling timeline body, or undefined when none of it is. */
function shownSpan(el: HTMLElement): { left: number; right: number; height: number } | undefined {
  const r = el.getBoundingClientRect();
  const body = el.closest(".bottom-body")?.getBoundingClientRect();
  const left = Math.max(r.left, body?.left ?? 0, 0);
  const right = Math.min(r.right, body?.right ?? window.innerWidth, window.innerWidth);
  const top = Math.max(r.top, body?.top ?? 0);
  const bottom = Math.min(r.bottom, body?.bottom ?? window.innerHeight);
  if (right <= left || bottom <= top) return undefined;
  return { left: left - r.left, right: right - r.left, height: r.height };
}

/**
 * The picture of a video clip `width` px wide (its `duration` frames at `ppf`): a filmstrip once
 * it's made, the poster (`poster`, repeated) until then.
 */
export function ClipFilmstrip({ item, media, fps, duration, ppf, poster }: { item: Item; media: MediaItem; fps: number; duration: number; ppf: number; poster: string | undefined }) {
  const ref = useRef<HTMLDivElement>(null);
  const [span, setSpan] = useState<{ left: number; right: number; height: number } | undefined>(undefined);
  const strip = useFilmstrip(filmstripUrl(media.id, media.path), span !== undefined);
  // Where the clip is on screen: on scroll, resize and whenever it's laid out again (zoom, trims).
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    let frame = 0;
    const measure = () => {
      frame = 0;
      const next = shownSpan(el);
      setSpan((prev) => {
        if (!next) return undefined;
        // Only a change of a few px is worth a render (the cells overlap the edges by one).
        if (prev && Math.abs(prev.left - next.left) < 4 && Math.abs(prev.right - next.right) < 4 && prev.height === next.height) return prev;
        return next;
      });
    };
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(measure);
    };
    measure();
    const body = el.closest(".bottom-body");
    body?.addEventListener("scroll", schedule, { passive: true });
    window.addEventListener("resize", schedule);
    const io = new IntersectionObserver(schedule, { threshold: [0, 1] });
    io.observe(el);
    const ro = new ResizeObserver(schedule);
    ro.observe(el);
    return () => {
      if (frame) cancelAnimationFrame(frame);
      body?.removeEventListener("scroll", schedule);
      window.removeEventListener("resize", schedule);
      io.disconnect();
      ro.disconnect();
    };
  }, []);
  // The clip moved without its own box changing size (the timeline zoomed around it): measure again.
  // Issue #118: in the next animation frame, all clips together, when the browser has laid the
  // timeline out anyway; measured right after the render, each clip made it lay out the whole
  // timeline before it was drawn (most of a zoom step's time on a long edit).
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const frame = requestAnimationFrame(() => {
      const next = shownSpan(el);
      setSpan((prev) => (!next ? undefined : prev && prev.left === next.left && prev.right === next.right && prev.height === next.height ? prev : next));
    });
    return () => cancelAnimationFrame(frame);
  }, [duration, ppf]);

  const width = duration * ppf;
  if (!strip || !span || span.height <= 0) {
    return <div ref={ref} className="strip filmstrip" data-testid="filmstrip" data-media={media.id} data-ready="false" style={poster ? { backgroundImage: `url("${poster}")` } : undefined} />;
  }
  const { layout, src } = strip;
  const { width: cell, cells } = visibleCells(item, fps, layout, width, span.height, span.left, span.right);
  const scale = span.height / layout.tileHeight;
  const size = `${layout.cols * layout.tileWidth * scale}px ${layout.rows * layout.tileHeight * scale}px`;
  return (
    <div ref={ref} className="strip filmstrip" data-testid="filmstrip" data-media={media.id} data-ready="true" data-interval={layout.interval} data-tiles={cells.map((c) => c.tile).join(",")}>
      {cells.map((c) => (
        <div
          key={c.index}
          className="film-cell"
          data-tile={c.tile}
          style={{
            left: c.x,
            width: cell,
            backgroundImage: `url("${src}")`,
            backgroundSize: size,
            backgroundPosition: `${-(c.tile % layout.cols) * layout.tileWidth * scale}px ${-Math.floor(c.tile / layout.cols) * layout.tileHeight * scale}px`,
          }}
        />
      ))}
    </div>
  );
}
