// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// D221 scrub cache (scrub.ts): one source file's index, decoder and decode queue.
import { type Mp4Index, readMp4Index, sampleAt, keyframeOf } from "@/app/lib/editor/core/media/mp4index";
import type { ScrubCache } from "@/app/ui/editor/preview/scrub/cache";
import { decoderColour } from "@/app/ui/editor/preview/scrub/colour";
import { cachedSize, windowRanks } from "@/app/ui/editor/preview/scrub/frames";

/** Largest byte range read at once while decoding a long GOP. */
const READ_SPAN = 2 * 1024 * 1024;

/** Whether a range response still describes the file the index was made from. */
export function sameFile(known: { size: number; modified: string | undefined }, contentRange: string | null, modified: string | null) {
  const total = Number(/\/(\d+)\s*$/.exec(contentRange ?? "")?.[1]);
  if (Number.isFinite(total) && total !== known.size) return false;
  return !(known.modified && modified && modified !== known.modified);
}

/** What one source's decode is doing. */
interface Job {
  readonly gen: number;
  /** The keyframe it decodes from and the last sample (decode order) it feeds. */
  readonly from: number;
  readonly upTo: number;
  /** Samples it keeps (the rest are decoded only as references). */
  readonly wanted: Set<number>;
  /** The furthest sample fed so far. */
  fed: number;
}

let sourceSeq = 0;

/** One file's index and decoder. */
export class ScrubSource {
  readonly id = ++sourceSeq;
  state: "loading" | "ready" | "broken" = "loading";
  /** The decoded frames' colour space, as WebCodecs reports it (for diagnosis). */
  colour = "";
  /** What the stream says and what the decoder was configured with (for diagnosis). */
  colourIn = "";
  /** Why it isn't used (for diagnosis). */
  reason = "";
  index: Mp4Index | undefined;
  private file = { size: 0, modified: undefined as string | undefined };
  private decoder: VideoDecoder | undefined;
  private config: VideoDecoderConfig | undefined;
  private job: Job | undefined;
  private gen = 0;
  private target: { sample: number; dir: -1 | 0 | 1 } | undefined;
  private readonly byTimestamp = new Map<number, number>();
  // A target that wasn't cached when asked: when, for the miss timings in stats().
  private missAt: { sample: number; at: number } | undefined;
  private scratch: OffscreenCanvas | undefined;
  private size = { w: 0, h: 0 };
  // Samples tried and not output (a leading picture that needs the GOP before): decoded from an earlier keyframe next time.
  private readonly earlier = new Set<number>();

  readonly url: string;
  private readonly cache: ScrubCache;
  // Samples that never came out even from the keyframe before: left to the <video>.
  private readonly hopeless = new Set<number>();

  constructor(url: string, cache: ScrubCache) {
    this.url = url;
    this.cache = cache;
    void this.init();
  }

  private key(sample: number) {
    return `${this.id}|${sample}`;
  }

  private fail(reason: string) {
    this.state = "broken";
    this.reason = reason;
    this.close();
    this.cache.changed();
  }

  /** Bytes [offset, offset + length) of the file, checked against the file the index came from. */
  private async read(offset: number, length: number): Promise<Uint8Array> {
    const r = await fetch(this.url, { headers: { Range: `bytes=${offset}-${offset + length - 1}` } });
    if (r.status !== 206 && r.status !== 200) throw new Error(`HTTP ${r.status}`);
    if (this.state !== "loading" && !sameFile(this.file, r.headers.get("content-range"), r.headers.get("last-modified"))) {
      this.cache.forget(this);
      throw new Error("the file changed");
    }
    const bytes = new Uint8Array(await r.arrayBuffer());
    return r.status === 200 ? bytes.subarray(offset, offset + length) : bytes;
  }

  private async init() {
    try {
      if (typeof VideoDecoder === "undefined") return this.fail("no WebCodecs");
      const head = await fetch(this.url, { headers: { Range: "bytes=0-15" } });
      const size = Number(/\/(\d+)\s*$/.exec(head.headers.get("content-range") ?? "")?.[1] ?? head.headers.get("content-length"));
      await head.arrayBuffer().catch(() => {});
      if (!head.ok || !Number.isFinite(size) || size <= 0) return this.fail(`no size (HTTP ${head.status})`);
      this.file = { size, modified: head.headers.get("last-modified") ?? undefined };
      const index = await readMp4Index((o, l) => this.read(o, l), size);
      if (!index) return this.fail("not an indexable MP4/MOV with H.264 or HEVC");
      const colorSpace = decoderColour(index.colour, index.codedHeight);
      this.colourIn = JSON.stringify({ stream: index.colour ?? "unknown", config: colorSpace ?? "webcodecs" });
      const config: VideoDecoderConfig = { codec: index.codec, description: index.description, codedWidth: index.codedWidth, codedHeight: index.codedHeight, optimizeForLatency: true, ...(colorSpace ? { colorSpace } : {}) };
      const support = await VideoDecoder.isConfigSupported(config).catch(() => undefined);
      if (!support?.supported) return this.fail(`${index.codec} isn't decodable here`);
      if (this.state === "broken") return;
      this.index = index;
      this.config = config;
      for (let i = 0; i < index.pts.length; i++) this.byTimestamp.set(Math.round(index.pts[i]! * 1e6), i);
      this.state = "ready";
      this.cache.changed();
      this.pump();
    } catch (error) {
      this.fail(String((error as Error).message ?? error));
    }
  }

  /** Light enough (≤ 1080p) that a <video> seek may race the cache on a miss. */
  get light() {
    return !!this.index && this.index.codedWidth * this.index.codedHeight <= 1920 * 1088;
  }

  /** The sample shown at t (seconds into the file), if indexed. */
  sampleAt(t: number) {
    return this.index ? sampleAt(this.index, t) : undefined;
  }

  /** The cached frame shown at t, if there is one. */
  frame(t: number): { bitmap: ImageBitmap; sample: number; pts: number } | undefined {
    if (this.state !== "ready" || !this.index) return undefined;
    const sample = sampleAt(this.index, t);
    const bitmap = this.cache.lru.get(this.key(sample));
    return bitmap ? { bitmap, sample, pts: this.index.pts[sample]! } : undefined;
  }

  /** Have the frame at t (and the window around it, in the direction of travel) decoded. */
  want(t: number, dir: -1 | 0 | 1) {
    if (this.state !== "ready" || !this.index) return;
    const sample = sampleAt(this.index, t);
    if (this.target?.sample === sample && this.target.dir === dir) return;
    this.target = { sample, dir };
    if (!this.cache.lru.has(this.key(sample))) this.missAt = { sample, at: performance.now() };
    this.pump();
  }

  /** How many frames around the playhead to keep, from the frame size and the budget. */
  private span() {
    const guess = cachedSize(this.index!.codedWidth, this.index!.codedHeight);
    const bytes = Math.max(1, (this.size.w * this.size.h || guess.w * guess.h) * 4);
    return Math.max(2, Math.min(30, Math.floor((this.cache.lru.budget * 0.5) / Math.max(1, this.cache.active()) / bytes)));
  }

  /** Starts (or keeps) the decode that brings the next missing frame of the window. */
  private pump() {
    const index = this.index;
    if (this.state !== "ready" || !index || !this.target) return;
    const { sample, dir } = this.target;
    const ranks = windowRanks(index.rank[sample]!, index.order.length, dir, this.span());
    const window = ranks.map((r) => index.order[r]!);
    const missing = window.filter((s) => !this.hopeless.has(s) && !this.cache.lru.has(this.key(s)));
    const job = this.job;
    if (!missing.length) return;
    const first = missing[0]!;
    const gopOf = (s: number) => (this.earlier.has(s) ? keyframeOf(index, Math.max(0, keyframeOf(index, s) - 1)) : keyframeOf(index, s));
    if (job) {
      // The running decode still brings the first missing frame: let it.
      if (job.wanted.has(first) && gopOf(first) === job.from && first <= job.upTo) return;
      // Otherwise, start over where it's needed (dropping what's queued).
      this.abort();
    }
    const from = gopOf(first);
    let nextKey = keyframeOf(index, first) + 1;
    while (nextKey < index.key.length && !index.key[nextKey]) nextKey++;
    // The playhead's own frame first and alone (converting the window's other frames would delay it); the window next.
    const inGop = first === sample ? [first] : missing.filter((s) => s >= from && s < nextKey);
    const upTo = Math.max(...inGop);
    void this.decode({ gen: ++this.gen, from, upTo, wanted: new Set(inGop), fed: from - 1 });
  }

  private abort() {
    this.job = undefined;
    this.gen++;
    if (this.decoder && this.decoder.state === "configured") this.decoder.reset();
  }

  private ensureDecoder(): VideoDecoder {
    if (this.decoder && this.decoder.state !== "closed") {
      if (this.decoder.state === "unconfigured") this.decoder.configure(this.config!);
      return this.decoder;
    }
    const decoder = new VideoDecoder({
      output: (frame) => this.output(frame),
      error: (e) => {
        // A decode error on this file: the <video> path takes over for it.
        if (this.decoder === decoder) this.fail(`decoder: ${e.message}`);
      },
    });
    decoder.configure(this.config!);
    this.decoder = decoder;
    return decoder;
  }

  private async decode(job: Job) {
    const index = this.index!;
    this.job = job;
    try {
      const decoder = this.ensureDecoder();
      // Batches of samples read with one range request each; the next batch is read while this one decodes.
      const batches: Array<{ first: number; last: number; start: number; end: number }> = [];
      for (let s = job.from; s <= job.upTo; ) {
        const start = index.offsets[s]!;
        let end = start + index.sizes[s]!;
        let last = s;
        while (last + 1 <= job.upTo) {
          const lo = Math.min(start, index.offsets[last + 1]!);
          const hi = Math.max(end, index.offsets[last + 1]! + index.sizes[last + 1]!);
          if (hi - lo > READ_SPAN || lo < start) break;
          end = hi;
          last++;
        }
        batches.push({ first: s, last, start, end });
        s = last + 1;
      }
      const readBatch = (b: (typeof batches)[number]) => this.read(b.start, b.end - b.start);
      let next = readBatch(batches[0]!);
      for (let b = 0; b < batches.length; b++) {
        const batch = batches[b]!;
        const bytes = await next;
        if (job.gen !== this.gen) return;
        if (b + 1 < batches.length) {
          next = readBatch(batches[b + 1]!);
          next.catch(() => {});
        }
        for (let i = batch.first; i <= batch.last; i++) {
          while (decoder.decodeQueueSize > 16) {
            await new Promise((r) => decoder.addEventListener("dequeue", r, { once: true }));
            if (job.gen !== this.gen) return;
          }
          const at = index.offsets[i]! - batch.start;
          decoder.decode(new EncodedVideoChunk({ type: index.key[i] ? "key" : "delta", timestamp: Math.round(index.pts[i]! * 1e6), duration: Math.round(index.durations[i]! * 1e6), data: bytes.subarray(at, at + index.sizes[i]!) }));
          job.fed = i;
        }
      }
      await decoder.flush();
      if (job.gen !== this.gen) return;
      // A wanted frame that never came out (a leading picture of an open GOP): next time from the keyframe before.
      for (const w of job.wanted) {
        if (this.cache.lru.has(this.key(w))) continue;
        if (!this.earlier.has(w) && job.from > 0) this.earlier.add(w);
        else this.hopeless.add(w);
      }
      this.job = undefined;
      this.pump();
    } catch (error) {
      if (job.gen !== this.gen) return; // aborted (reset rejects the flush)
      this.job = undefined;
      if ((error as Error).name === "AbortError") return;
      this.fail(String((error as Error).message ?? error));
    }
  }

  private output(frame: VideoFrame) {
    const sample = this.byTimestamp.get(frame.timestamp);
    const job = this.job;
    try {
      if (sample === undefined || !job || !job.wanted.has(sample)) return;
      // HDR footage would be tone-mapped differently from the <video>: leave it to the <video>.
      const transfer = frame.colorSpace?.transfer as string | null | undefined;
      if (transfer === "pq" || transfer === "hlg") {
        queueMicrotask(() => this.fail(`HDR (${transfer})`));
        return;
      }
      if (!this.colour) this.colour = JSON.stringify(frame.colorSpace?.toJSON?.() ?? {});
      const turn = this.index!.rotation;
      const dw = turn % 180 ? frame.displayHeight : frame.displayWidth;
      const dh = turn % 180 ? frame.displayWidth : frame.displayHeight;
      const { w, h } = cachedSize(dw, dh);
      this.size = { w, h };
      if (!this.scratch) this.scratch = new OffscreenCanvas(w, h);
      if (this.scratch.width !== w) this.scratch.width = w;
      if (this.scratch.height !== h) this.scratch.height = h;
      const ctx = this.scratch.getContext("2d")!;
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      if (turn) {
        ctx.translate(w / 2, h / 2);
        ctx.rotate((turn * Math.PI) / 180);
        const [rw, rh] = turn % 180 ? [h, w] : [w, h];
        ctx.drawImage(frame, -rw / 2, -rh / 2, rw, rh);
      } else ctx.drawImage(frame, 0, 0, w, h);
      const bitmap = this.scratch.transferToImageBitmap();
      this.cache.lru.put(this.key(sample), bitmap, w * h * 4, () => bitmap.close());
      // The playhead's frame is drawn now, not on the next animation frame.
      if (this.target?.sample === sample) this.cache.changed(true);
      else this.cache.changed();
      if (this.missAt?.sample === sample) {
        this.cache.misses.push(performance.now() - this.missAt.at);
        if (this.cache.misses.length > 100) this.cache.misses.shift();
        this.missAt = undefined;
      }
    } finally {
      frame.close();
    }
  }

  close() {
    this.gen++;
    this.job = undefined;
    if (this.decoder && this.decoder.state !== "closed") this.decoder.close();
    this.decoder = undefined;
  }

  /** Its frames out of the cache. */
  drop() {
    this.close();
    const prefix = `${this.id}|`;
    this.cache.lru.dropWhere((k) => k.startsWith(prefix));
  }
}
