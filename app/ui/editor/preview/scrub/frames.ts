// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// D221 scrub cache (scrub.ts): its size limits, the byte-bounded LRU and the decode window's order.
/** Longest side of a cached frame (a 4K original is kept at 1920 wide; proxies are 1280). */
export const SCRUB_MAX_SIDE = 1920;
/** Default byte budget of the whole cache. */
export const SCRUB_BUDGET = 300 * 1024 * 1024;

/** An LRU of closable values with a byte budget (pure: unit-tested in node). */
export class FrameLru<V> {
  private readonly entries = new Map<string, { value: V; bytes: number; close: () => void }>();
  bytes = 0;
  budget: number;
  constructor(budget: number) {
    this.budget = budget;
  }

  get size() {
    return this.entries.size;
  }
  has(key: string) {
    return this.entries.has(key);
  }
  /** The value, made the most recently used. */
  get(key: string): V | undefined {
    const e = this.entries.get(key);
    if (!e) return undefined;
    this.entries.delete(key);
    this.entries.set(key, e);
    return e.value;
  }
  /** Adds a value; the oldest go (closed) until the budget holds. One bigger than the budget is closed at once. */
  put(key: string, value: V, bytes: number, close: () => void): boolean {
    const old = this.entries.get(key);
    if (old) {
      this.entries.delete(key);
      this.bytes -= old.bytes;
      old.close();
    }
    if (bytes > this.budget) {
      close();
      return false;
    }
    this.entries.set(key, { value, bytes, close });
    this.bytes += bytes;
    this.trim();
    return true;
  }
  setBudget(budget: number) {
    this.budget = budget;
    this.trim();
  }
  private trim() {
    for (const [k, e] of this.entries) {
      if (this.bytes <= this.budget) break;
      this.entries.delete(k);
      this.bytes -= e.bytes;
      e.close();
    }
  }
  /** Drops (and closes) every entry whose key matches. */
  dropWhere(match: (key: string) => boolean) {
    for (const [k, e] of [...this.entries]) {
      if (!match(k)) continue;
      this.entries.delete(k);
      this.bytes -= e.bytes;
      e.close();
    }
  }
  clear() {
    this.dropWhere(() => true);
  }
}

/** The frame size a cache entry is kept at: the display size, the long side capped. */
export function cachedSize(width: number, height: number, max = SCRUB_MAX_SIDE) {
  const k = Math.min(1, max / Math.max(width, height, 1));
  return { w: Math.max(1, Math.round(width * k)), h: Math.max(1, Math.round(height * k)) };
}

/**
 * The samples worth having around presentation rank `r` (of `count`), nearest first in the
 * direction of travel: ahead of a forward scrub, behind a backward one, both ways when still.
 */
export function windowRanks(r: number, count: number, dir: -1 | 0 | 1, span: number): Array<number> {
  const out: Array<number> = [r];
  const ahead = dir === 0 ? Math.ceil(span / 2) : span;
  const behind = dir === 0 ? Math.ceil(span / 2) : 2;
  const fwd = dir >= 0 ? ahead : behind;
  const back = dir >= 0 ? behind : ahead;
  for (let i = 1; i <= Math.max(fwd, back); i++) {
    if (dir >= 0) {
      if (i <= fwd && r + i < count) out.push(r + i);
      if (i <= back && r - i >= 0) out.push(r - i);
    } else {
      if (i <= back && r - i >= 0) out.push(r - i);
      if (i <= fwd && r + i < count) out.push(r + i);
    }
  }
  return out;
}
