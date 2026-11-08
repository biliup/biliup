import type { Item } from './types';

/** One pass through a source file, bounded by the item and an optional local edit window. */
export interface SourceSpan {
  /** Absolute pass number in the unwrapped source time; splitting an item preserves it. */
  readonly cycle: number;
  readonly offset: number;
  /** Source frames within the file, before rounding to timeline frames. */
  readonly from: number;
  readonly to: number;
  /** Local timeline frames, [start, end). */
  readonly start: number;
  readonly end: number;
}

/** Inverse of playback source time. A held picture has no repeated speech after the file ends. */
export function* sourceSpans(item: Pick<Item, 'in' | 'speed' | 'duration' | 'loop'>, length?: number, window?: { start: number; end: number }): Generator<SourceSpan> {
  const speed = item.speed ?? 1;
  const inF = item.in ?? 0;
  const start = Math.max(0, window?.start ?? 0);
  const end = Math.min(item.duration, window?.end ?? item.duration);
  if (end <= start) return;
  const firstFrame = inF + start * speed;
  const lastFrame = inF + end * speed;
  const size = length !== undefined && length > 0 ? length : undefined;
  const loop = item.loop === true && size !== undefined;
  const first = loop ? Math.floor(firstFrame / size!) : 0;
  const last = loop ? Math.ceil(lastFrame / size! - 1e-9) : 1;
  for (let cycle = first; cycle < last; cycle++) {
    const offset = loop ? cycle * size! : 0;
    const from = Math.max(0, firstFrame - offset);
    const to = Math.min(size ?? lastFrame, lastFrame - offset);
    const a = Math.max(start, Math.ceil((offset - inF) / speed - 1e-9));
    const b = Math.min(end, Math.ceil((offset + (size ?? lastFrame) - inF) / speed - 1e-9));
    if (to > from) yield { cycle, offset, from, to, start: a, end: b };
  }
}

/** Timeline frames of this pass that play the requested source interval. */
export function sourceRange(item: Pick<Item, 'in' | 'speed'>, span: SourceSpan, from: number, to: number): [number, number] {
  const inF = item.in ?? 0;
  const speed = item.speed ?? 1;
  return [
    Math.max(span.start, Math.ceil((from + span.offset - inF) / speed - 1e-9)),
    Math.min(span.end, Math.ceil((to + span.offset - inF) / speed - 1e-9)),
  ];
}

/** Map chronologically ordered source events (seconds) into each played pass, keeping subframes. */
export function* sourcePoints(item: Pick<Item, 'in' | 'speed' | 'duration' | 'loop'>, length: number | undefined, points: ReadonlyArray<number>, fps: number, window?: { start: number; end: number }): Generator<{ local: number; cycle: number; index: number }> {
  if (!points.length) return;
  const inF = item.in ?? 0;
  const speed = item.speed ?? 1;
  for (const span of sourceSpans(item, length, window)) {
    const from = span.from / fps;
    const to = span.to / fps;
    let lo = 0;
    let hi = points.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (points[mid]! < from) lo = mid + 1;
      else hi = mid;
    }
    for (let index = lo; index < points.length && points[index]! < to; index++) {
      yield { local: (points[index]! * fps + span.offset - inF) / speed, cycle: span.cycle, index };
    }
  }
}
