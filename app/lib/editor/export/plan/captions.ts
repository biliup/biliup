// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type Project, type Item, type Track, captionSegments, isPlainCue, textHtml } from "@/app/lib/editor/core";
import type { VisualLayer } from "./types.ts";

/** A captions layer between cards: nothing drawn (a transparent PNG). */
export const CAPTION_GAP_HTML = "<div></div>";
/** At most this many layers (ffmpeg inputs) per captions item: runs of cards closest together are merged. */
export const MAX_CAPTION_LAYERS = 16;
/** Past this many looks, the export says captions add time (each look is one PNG Chromium draws). */
export const CAPTION_LOOKS_WARN = 1500;
/** Roughly how many caption looks the rasteriser draws a second (for that warning's estimate). */
const CAPTION_LOOKS_PER_S = 25;

/**
 * Captions (D82) as raster layers: core captionSegments gives each stretch with one look (a card, or
 * a card with one speaking word), so textHtml runs once per look, not per frame. Touching segments
 * form a run (one layer: a still when it has one look, else a sequence of its looks); the gaps
 * between runs draw nothing and cost nothing. Past MAX_CAPTION_LAYERS runs, those with the shortest
 * gaps between them are merged (the gap is an empty look). Each card has at most its words + 1
 * looks.
 */
export function captionLayers(project: Project, item: Item, track: Track, duration: number, warnings: Array<string>, from = 0): Array<VisualLayer> {
  return segmentLayers(project, item, track, captionSegments(project, item), duration, warnings, from);
}

/**
 * A Subtitles track's plain text items (D105: an imported .srt / .vtt, one item per cue) as the
 * same few layers as captions: each cue a still look over its frames, touching cues one run.
 * Drawn exactly as each text item alone would be (no motion, keys or opacity of their own), without
 * an ffmpeg input per cue.
 */
export function subtitleLayers(project: Project, track: Track, items: ReadonlyArray<Item>, starts: ReadonlyMap<string, number>, duration: number, warnings: Array<string>, from = 0): Array<VisualLayer> {
  if (!items.length) return [];
  const segs = items.map((it) => ({ from: starts.get(it.id) ?? it.start ?? 0, to: (starts.get(it.id) ?? it.start ?? 0) + it.duration, text: it })).sort((a, b) => a.from - b.from);
  return segmentLayers(project, { ...items[0]!, name: track.name }, track, segs, duration, warnings, from);
}

/**
 * A Subtitles track's cues that can share layers: plain ones that overlap no other item on the track.
 * Overlapping cues (a user dragged one onto another) each take the normal text path, so both draw, as in the preview.
 */
export function mergeableCues(track: Track, starts: ReadonlyMap<string, number>): Array<Item> {
  const spans = track.items.map((it) => ({ it, from: starts.get(it.id) ?? it.start ?? 0 })).map((x) => ({ ...x, to: x.from + x.it.duration })).sort((a, b) => a.from - b.from);
  const overlapping = new Set<Item>();
  let last: (typeof spans)[number] | undefined;
  for (const x of spans) {
    if (last && x.from < last.to) {
      overlapping.add(x.it);
      overlapping.add(last.it);
    }
    if (!last || x.to > last.to) last = x;
  }
  return track.items.filter((it) => isPlainCue(it) && !overlapping.has(it));
}

/** Stretches of one look each (a card, a speaking word, a cue) as raster layers: touching stretches one run, at most MAX_CAPTION_LAYERS runs. */
function segmentLayers(project: Project, item: Item, track: Track, all: ReadonlyArray<{ readonly from: number; readonly to: number; readonly text: Item }>, duration: number, warnings: Array<string>, from: number): Array<VisualLayer> {
  const segs = all
    .map((s) => ({ from: Math.max(from, s.from), to: Math.min(duration, s.to), text: s.text }))
    .filter((s) => s.to > s.from);
  if (!segs.length) return [];
  const runs: Array<Array<(typeof segs)[number]>> = [];
  for (const s of segs) {
    const run = runs.at(-1);
    if (run && run.at(-1)!.to === s.from) run.push(s);
    else runs.push([s]);
  }
  // Merge across the shortest gaps until few enough layers.
  while (runs.length > MAX_CAPTION_LAYERS) {
    let best = 0;
    for (let i = 1; i < runs.length - 1; i++) if (runs[i + 1]![0]!.from - runs[i]!.at(-1)!.to < runs[best + 1]![0]!.from - runs[best]!.at(-1)!.to) best = i;
    runs.splice(best, 2, [...runs[best]!, ...runs[best + 1]!]);
  }
  const html = new Map<Item, string>();
  let looks = 0;
  const layers = runs.map((run): VisualLayer => {
    const f0 = run[0]!.from;
    const f1 = run.at(-1)!.to;
    const states: Array<string> = [];
    const index = new Map<string, number>();
    const at = (h: string) => index.get(h) ?? (index.set(h, states.push(h) - 1), states.length - 1);
    const frames = new Array<number>(f1 - f0);
    let f = f0;
    for (const s of run) {
      if (s.from > f) frames.fill(at(CAPTION_GAP_HTML), f - f0, s.from - f0);
      let h = html.get(s.text);
      if (h === undefined) html.set(s.text, (h = textHtml(s.text, project)));
      frames.fill(at(h), s.from - f0, s.to - f0);
      f = s.to;
    }
    looks += states.length;
    return { item, track, f0, f1, ramps: [], kind: "raster", states, frames };
  });
  if (looks > CAPTION_LOOKS_WARN) warnings.push(`"${item.name ?? item.id}" has ${looks} caption looks to draw, which adds about ${Math.max(1, Math.round(looks / CAPTION_LOOKS_PER_S / 60))} min to the export.`);
  return layers;
}
