import { audioSpan, heldPast, itemGain, sourceAt } from './audio';
import type { Project, Track } from '../timeline/types';
import type { SpeechEnvelope } from '../text/transcript/speech';

/** Samples on the project's envelope clock, including transition handles and source loops. */
export function* itemEnvelope(project: Pick<Project, 'tracks' | 'media' | 'canvas'>, track: Track, index: number, start: number, envelope: SpeechEnvelope): Generator<{ from: number; to: number; sourceDb: number; gain: number }> {
  const item = track.items[index]!;
  const fps = project.canvas.fps;
  const rate = envelope.rate;
  const span = audioSpan(project, track, index, start);
  const from = Math.max(0, span.from);
  const first = Math.ceil(from / fps * rate - 1e-9);
  const last = Math.ceil(span.to / fps * rate - 1e-9);
  for (let k = first; k < last; k++) {
    const frame = k / rate * fps;
    const local = frame - start;
    if (heldPast(project, item, local)) continue;
    const gain = itemGain(project, track, index, local);
    if (gain <= 0) continue;
    const sourceDb = envelope.db[Math.floor(sourceAt(project, item, local) * rate + 1e-9)];
    if (sourceDb === undefined) continue;
    yield { from: frame, to: Math.min(span.to, (k + 1) / rate * fps), sourceDb, gain };
  }
}
