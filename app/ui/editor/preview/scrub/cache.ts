// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// D221 scrub cache (scrub.ts): the cache over every source, its budget and the shared LRU.
import { SCRUB_BUDGET, FrameLru } from "@/app/ui/editor/preview/scrub/frames";
import { ScrubSource } from "@/app/ui/editor/preview/scrub/source";

const BUDGET_KEY = "biliup.preview.scrubCacheMB";
const budgetSetting = () => {
  try {
    const mb = Number(globalThis.localStorage?.getItem(BUDGET_KEY));
    return mb > 0 ? mb * 1024 * 1024 : SCRUB_BUDGET;
  } catch {
    return SCRUB_BUDGET;
  }
};

/** Every source the preview scrubs, and the one LRU their frames share. */
export class ScrubCache {
  readonly lru = new FrameLru<ImageBitmap>(budgetSetting());
  private readonly sources = new Map<string, ScrubSource>();
  /** The last decode times of frames that weren't cached when asked (ms, from want() to the frame). */
  readonly misses: Array<number> = [];
  /** Tests: keep the canvas up at rest (the <video> otherwise takes over once it has caught up). */
  keep = false;
  private used = new Set<string>();
  private scheduled = 0;

  private readonly onFrame: () => void;

  constructor(onFrame: () => void) {
    this.onFrame = onFrame;
    (globalThis as { __biliupScrub?: unknown }).__biliupScrub = { stats: () => this.stats(), setBudget: (bytes: number) => this.lru.setBudget(bytes), keep: (on: boolean) => {
        this.keep = on;
        this.changed();
      },
    };
  }

  stats() {
    return {
      bytes: this.lru.bytes,
      budget: this.lru.budget,
      frames: this.lru.size,
      misses: [...this.misses],
      sources: [...this.sources.values()].map((s) => ({ url: s.url.replace(/[?&]token=[^&]*/, ""), state: s.state, reason: s.reason, colour: s.colour, colourIn: s.colourIn })),
    };
  }

  /** Sources being scrubbed now (they share the window budget). */
  active() {
    return Math.max(1, this.used.size);
  }

  /** The source of a URL if there is one, without asking for it. */
  peek(url: string): ScrubSource | undefined {
    return this.sources.get(url);
  }

  /** The source of a URL (made on first use; a broken one stays broken: the <video> plays it). */
  source(url: string): ScrubSource {
    this.used.add(url);
    let s = this.sources.get(url);
    if (!s) {
      s = new ScrubSource(url, this);
      this.sources.set(url, s);
    }
    return s;
  }

  /** A source whose file changed in place: its frames go and the next use starts it over. */
  forget(s: ScrubSource) {
    if (this.sources.get(s.url) === s) this.sources.delete(s.url);
    s.drop();
    this.changed();
  }

  /**
   * After a render: sources no clip used stop decoding; a media item now played from another URL
   * (relinked, a new proxy, proxies turned on or off) drops the old one's frames.
   */
  sweep() {
    const usedIds = new Map<string, string>();
    for (const u of this.used) usedIds.set(mediaIdOf(u), u);
    for (const [url, s] of [...this.sources]) {
      if (this.used.has(url)) continue;
      const now = usedIds.get(mediaIdOf(url));
      if (now !== undefined && now !== url) {
        this.sources.delete(url);
        s.drop();
      } else s.close();
    }
    this.used = new Set();
  }

  /** A new frame (or a source's state): one render on the next animation frame, or `now` for the playhead's frame. */
  changed(now = false) {
    if (now) {
      cancelAnimationFrame(this.scheduled);
      this.scheduled = 0;
      // After the decoder's callback returns (the render may start the next decode).
      queueMicrotask(this.onFrame);
      return;
    }
    if (this.scheduled) return;
    this.scheduled = requestAnimationFrame(() => {
      this.scheduled = 0;
      this.onFrame();
    });
  }

  /** Every source and frame dropped (the setting turned off). */
  clear() {
    for (const s of this.sources.values()) s.drop();
    this.sources.clear();
    this.lru.clear();
  }

  destroy() {
    cancelAnimationFrame(this.scheduled);
    for (const s of this.sources.values()) s.drop();
    this.sources.clear();
    this.lru.clear();
  }
}

/** The media route and id of a media URL (/media/file/<id> or /media/proxy/<id>), its tag and token left out. */
export function mediaIdOf(url: string) {
  try {
    const u = new URL(url);
    return decodeURIComponent(u.pathname.split("/").pop() ?? "");
  } catch {
    return url;
  }
}
