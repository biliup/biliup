// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Waveforms on timeline items (D54): the server's peaks for a media item (fetched once per file and
// shared through the preview cache), drawn into a canvas for the part of the media an item plays. Audio items
// get a full-height waveform; clips with sound a compact one along their bottom edge. The drawing
// follows the shared item/track gain, fades, transitions and ducking before mixer effects.
import { useWaveform, usePreviewVisible, type Peaks } from "@/app/ui/editor/preview/resources";
import { itemGain } from "@/app/lib/editor/core/audio/audio";
import { duckDbAt, type DuckKey } from '@/app/lib/editor/core/audio/ducking';
import { peakBetween } from '@/app/lib/editor/core/audio/peaks';
import { sourceSpans } from '@/app/lib/editor/core/timeline/sourceSpans';
import { useEffect, useRef, useState } from "react";

import type { Item, MediaItem, ProjectModel } from "@/app/lib/editor/state/project";
import { waveformUrl } from "@/app/lib/editor/state/store";

export type { Peaks } from '@/app/ui/editor/preview/resources';

/** The same registered file version is shared by every timeline item using this sound. */
export function usePeaks(media: MediaItem | undefined): Peaks | undefined {
  return useWaveform(media && media.kind !== 'image' && media.probe?.hasAudio !== false ? waveformUrl(media.id, media.path) : undefined);
}
/** The canvas never gets wider than this (Chromium's limit is 32767); wider items stretch it. */
const MAX_CANVAS_PX = 8192;

/**
 * The waveform of `item` (its media from `in`, for its length at its speed) in a box `width` px
 * wide. `compact` draws a short strip from the bottom (clips); otherwise mirrored about the middle.
 */
export function ItemWaveform({ project, item, media, start, duck, width, compact }: { project: ProjectModel; item: Item; media: MediaItem; start: number; duck: ReadonlyArray<DuckKey> | undefined; width: number; compact: boolean }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const visible = usePreviewVisible(ref);
  const peaks = usePeaks(visible ? media : undefined);
  // Issue #118: the canvas's height as a ResizeObserver reports it. Read from clientHeight on every
  // redraw (each zoom step), it made the browser lay out the whole timeline once per waveform.
  const [cssHeight, setCssHeight] = useState(0);
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ro = new ResizeObserver(([entry]) => entry && setCssHeight(entry.contentRect.height));
    ro.observe(canvas);
    return () => ro.disconnect();
  }, []);
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    canvas.dataset.ready = 'false';
    canvas.dataset.drawn = '0';
    if (!peaks || cssHeight <= 0) {
      canvas.getContext('2d')?.clearRect(0, 0, canvas.width, canvas.height);
      return;
    }
    const dpr = window.devicePixelRatio || 1;
    const w = Math.max(1, Math.min(MAX_CANVAS_PX, Math.round(width * dpr)));
    const h = Math.max(1, Math.round(cssHeight * dpr));
    if (canvas.width !== w) canvas.width = w;
    if (canvas.height !== h) canvas.height = h;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = compact ? "rgba(140, 220, 180, 0.75)" : "rgba(120, 230, 170, 0.85)";
    const fps = project.canvas.fps;
    const track = project.tracks.find(track => track.items.some(candidate => candidate.id === item.id));
    if (!track) return;
    const index = track.items.findIndex(candidate => candidate.id === item.id);
    const sourceLength = media.probe?.frames;
    const sourceWidth = item.duration * (item.speed ?? 1) / w;
    // A zoomed-out column can cover whole loops. Its maximum is the full source's maximum.
    const fullLoop = item.loop && sourceLength && sourceWidth >= sourceLength
      ? peakBetween(peaks.data, 0, sourceLength / fps * peaks.rate) : undefined;
    const gainAt = (local: number) => itemGain(project, track, index, local, duckDbAt(duck, start + local));
    let drawn = 0;
    // Issue #118: one path filled once, not a fillRect per column (thousands per item on every zoom step).
    ctx.beginPath();
    for (let x = 0; x < w; x++) {
      const a = x / w * item.duration;
      const b = (x + 1) / w * item.duration;
      let peak = fullLoop ?? 0;
      if (fullLoop === undefined) for (const span of sourceSpans(item, sourceLength, { start: a, end: b })) {
        peak = Math.max(peak, peakBetween(peaks.data, span.from / fps * peaks.rate, span.to / fps * peaks.rate));
      }
      const scale = Math.min(2, Math.max(gainAt(a), gainAt((a + b) / 2), gainAt(b)));
      const v = Math.min(1, (peak / 255) * scale);
      if (v <= 0) continue;
      drawn++;
      const bar = Math.max(1, v * (compact ? h : h - 2));
      if (compact) ctx.rect(x, h - bar, 1, bar);
      else ctx.rect(x, (h - bar) / 2, 1, bar);
    }
    ctx.fill();
    canvas.dataset.ready = "true";
    canvas.dataset.drawn = String(drawn);
  }, [peaks, width, project, item, media, start, duck, compact, cssHeight]);
  return <canvas ref={ref} className={`wave ${compact ? "compact" : ""}`} data-testid="waveform" data-media={media.id} />;
}
