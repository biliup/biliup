// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type DuckKey, pwlAt, SILENCE_DB } from "@/app/lib/editor/core";
import type { GainModel } from "./types.ts";
import { n } from "./num.ts";

// ---- gain ----

const clamp01 = (p: number) => Math.min(1, Math.max(0, p));

/** core keyDbAt, on keys already sorted. */
const keyDb = (keys: GainModel["keys"], local: number) => {
  if (!keys.length) return 0;
  if (local <= keys[0]!.frame) return keys[0]!.db;
  for (let i = 1; i < keys.length; i++) {
    const a = keys[i - 1]!;
    const b = keys[i]!;
    if (local <= b.frame) return b.frame === a.frame ? b.db : a.db + ((b.db - a.db) * (local - a.frame)) / (b.frame - a.frame);
  }
  return keys.at(-1)!.db;
};

/** core duckDbAt on keys local to the item, held at the first / last key (whose dB is 0 at the curve's real ends): core pwlAt. */
const duckDb = (keys: ReadonlyArray<DuckKey>, local: number) => pwlAt(keys, local);

/** The model's gain at `local` frames from the item's start: core itemGain, restated so it can be turned into an expression. */
export function gainAt(m: GainModel, local: number): number {
  const db = m.baseDb + keyDb(m.keys, Math.min(m.duration, Math.max(0, local))) + (m.duck?.length ? duckDb(m.duck, local) : 0);
  let g = db <= SILENCE_DB ? 0 : 10 ** (db / 20);
  if (!(m.half > 0) && m.fadeIn > 0) g *= clamp01(local / m.fadeIn);
  if (!(m.nextHalf > 0) && m.fadeOut > 0) g *= clamp01((m.duration - local) / m.fadeOut);
  if (m.half > 0) g *= clamp01((local + m.half) / (2 * m.half));
  if (m.nextHalf > 0) g *= clamp01((m.duration + m.nextHalf - local) / (2 * m.nextHalf));
  return g;
}

/** The gain when it never changes over the item (no keys, fades or transitions), else undefined. */
export function constantGain(m: GainModel): number | undefined {
  if (m.keys.length > 1 || m.half > 0 || m.nextHalf > 0 || m.fadeIn > 0 || m.fadeOut > 0 || m.duck?.length) return undefined;
  const db = m.baseDb + (m.keys[0]?.db ?? 0);
  return db <= SILENCE_DB ? 0 : 10 ** (db / 20);
}

/** Samples per block the gain is evaluated for (10 ms at 48 kHz). */
export const AUDIO_BLOCK = 480;

/**
 * ffmpeg volume expression (eval=frame) for the model: the gain at the middle of each audio
 * block (AUDIO_BLOCK samples at 48 kHz; volume's nb_samples is NaN per frame, so the block is
 * fixed by asetnsamples), `t` being seconds since the layer's start (`from`). `localStart` is its
 * actual item-local frame, including preroll clipped at project zero. Variables: 0 = local frames,
 * 1 = local clamped to the item, 2 = combined dB.
 */
export function gainExpr(m: GainModel, fps: number, localStart: number): string {
  const L = "ld(1)";
  let keys = m.keys.length ? n(m.keys.at(-1)!.db) : "0";
  for (let i = m.keys.length - 1; i >= 1; i--) {
    const a = m.keys[i - 1]!;
    const b = m.keys[i]!;
    const value = b.frame === a.frame ? n(b.db) : `(${n(a.db)}+(${n(b.db - a.db)})*(${L}-${n(a.frame)})/${n(b.frame - a.frame)})`;
    keys = `if(lte(${L},${n(b.frame)}),${value},${keys})`;
  }
  if (m.keys.length) keys = `if(lte(${L},${n(m.keys[0]!.frame)}),${n(m.keys[0]!.db)},${keys})`;
  const parts = [`if(lte(ld(2),${SILENCE_DB}),0,pow(10,ld(2)/20))`];
  if (!(m.half > 0) && m.fadeIn > 0) parts.push(`clip(ld(0)/${n(m.fadeIn)},0,1)`);
  if (!(m.nextHalf > 0) && m.fadeOut > 0) parts.push(`clip((${n(m.duration)}-ld(0))/${n(m.fadeOut)},0,1)`);
  if (m.half > 0) parts.push(`clip((ld(0)+${n(m.half)})/${n(2 * m.half)},0,1)`);
  if (m.nextHalf > 0) parts.push(`clip((${n(m.duration + m.nextHalf)}-ld(0))/${n(2 * m.nextHalf)},0,1)`);
  // The duck curve (D85) on the unclamped local frame, as a balanced tree so a long curve stays shallow.
  const duck = m.duck?.length ? `+${pwlExpr(m.duck, "ld(0)")}` : "";
  return [
    `st(0,(t+${n(AUDIO_BLOCK / 2 / 48000)})*${fps}+${n(localStart)})`,
    `st(1,clip(ld(0),0,${n(m.duration)}))`,
    `st(2,${n(m.baseDb)}+${keys}${duck})`,
    parts.join("*"),
  ].join(";");
}

/**
 * A piecewise-linear curve (keys sorted by frame, held at both ends; a step where two keys share a
 * frame, the later one winning from that frame on, as core duckDbAt) as an ffmpeg expression of `x`,
 * nested as a balanced tree of comparisons (depth log2 of the keys).
 */
export function pwlExpr(keys: ReadonlyArray<DuckKey>, x: string): string {
  if (!keys.length) return "0";
  if (keys.length === 1) return n(keys[0]!.db);
  const seg = (i: number) => {
    const a = keys[i]!;
    const b = keys[i + 1]!;
    return b.frame === a.frame || b.db === a.db ? n(b.db) : `(${n(a.db)}+(${n(b.db - a.db)})*(${x}-${n(a.frame)})/${n(b.frame - a.frame)})`;
  };
  const tree = (lo: number, hi: number): string => {
    if (lo === hi) return seg(lo);
    const mid = (lo + hi) >> 1;
    return `if(lt(${x},${n(keys[mid + 1]!.frame)}),${tree(lo, mid)},${tree(mid + 1, hi)})`;
  };
  return `if(lt(${x},${n(keys[0]!.frame)}),${n(keys[0]!.db)},if(gte(${x},${n(keys.at(-1)!.frame)}),${n(keys.at(-1)!.db)},${tree(0, keys.length - 2)}))`;
}
